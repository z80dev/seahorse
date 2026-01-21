use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer, MintTo};

declare_id!("HFQhM2FYhiP1mbpVFnpHZ7hJf5xFzekJAjqoxexvKA5Q");

// Scale factor for reward calculations (same as Seahorse)
const REWARD_SCALE: u64 = 1_000_000;

#[program]
pub mod staking_anchor {
    use super::*;

    /// Initialize a new staking pool
    /// Creates pool state and stake token vault
    /// Note: Pool PDA uses only stake_mint for simpler signer seeds
    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        reward_rate: u64, // Reward tokens per staked token per slot (scaled)
    ) -> Result<()> {
        let pool = &mut ctx.accounts.pool;
        pool.authority = ctx.accounts.authority.key();
        pool.stake_mint = ctx.accounts.stake_mint.key();
        pool.reward_mint = ctx.accounts.reward_mint.key();
        pool.stake_vault = ctx.accounts.stake_vault.key();
        pool.reward_rate = reward_rate;
        pool.total_staked = 0;
        pool.last_update_slot = Clock::get()?.slot;
        pool.bump = ctx.bumps.pool;
        Ok(())
    }

    /// Create a user stake account (separate from staking)
    pub fn create_user_stake(ctx: Context<CreateUserStake>) -> Result<()> {
        let user_stake = &mut ctx.accounts.user_stake;
        user_stake.owner = ctx.accounts.user.key();
        user_stake.pool = ctx.accounts.pool.key();
        user_stake.staked_amount = 0;
        user_stake.pending_rewards = 0;
        user_stake.last_stake_slot = Clock::get()?.slot;
        user_stake.bump = ctx.bumps.user_stake;
        Ok(())
    }

    /// Stake tokens into the pool
    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);

        let pool = &mut ctx.accounts.pool;
        let user_stake = &mut ctx.accounts.user_stake;
        let clock = Clock::get()?;

        // If user has existing stake, calculate pending rewards first
        if user_stake.staked_amount > 0 {
            let slots_staked = clock.slot.saturating_sub(user_stake.last_stake_slot);
            let pending = user_stake
                .staked_amount
                .checked_mul(pool.reward_rate)
                .ok_or(ErrorCode::Overflow)?
                .checked_mul(slots_staked)
                .ok_or(ErrorCode::Overflow)?
                .checked_div(REWARD_SCALE)
                .unwrap_or(0);
            user_stake.pending_rewards = user_stake
                .pending_rewards
                .checked_add(pending)
                .ok_or(ErrorCode::Overflow)?;
        }

        // Transfer stake tokens from user to vault
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_stake_token.to_account_info(),
            to: ctx.accounts.stake_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        );
        token::transfer(cpi_ctx, amount)?;

        // Update user stake
        user_stake.staked_amount = user_stake
            .staked_amount
            .checked_add(amount)
            .ok_or(ErrorCode::Overflow)?;
        user_stake.last_stake_slot = clock.slot;

        // Update pool total
        pool.total_staked = pool
            .total_staked
            .checked_add(amount)
            .ok_or(ErrorCode::Overflow)?;
        pool.last_update_slot = clock.slot;

        Ok(())
    }

    /// Unstake tokens from the pool
    /// Returns staked tokens and claims all pending rewards
    pub fn unstake(ctx: Context<Unstake>, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);

        let user_stake = &ctx.accounts.user_stake;
        require!(
            user_stake.staked_amount >= amount,
            ErrorCode::InsufficientStake
        );

        let pool = &ctx.accounts.pool;
        let clock = Clock::get()?;

        // Calculate pending rewards
        let slots_staked = clock.slot.saturating_sub(user_stake.last_stake_slot);
        let pending = user_stake
            .staked_amount
            .checked_mul(pool.reward_rate)
            .ok_or(ErrorCode::Overflow)?
            .checked_mul(slots_staked)
            .ok_or(ErrorCode::Overflow)?
            .checked_div(REWARD_SCALE)
            .unwrap_or(0);
        let total_rewards = user_stake
            .pending_rewards
            .checked_add(pending)
            .ok_or(ErrorCode::Overflow)?;

        // Transfer staked tokens back to user using PDA signer
        // Pool PDA seeds: ["pool", stake_mint]
        let stake_mint_key = pool.stake_mint;
        let pool_bump = pool.bump;
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"pool",
            stake_mint_key.as_ref(),
            &[pool_bump],
        ]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.stake_vault.to_account_info(),
            to: ctx.accounts.user_stake_token.to_account_info(),
            authority: ctx.accounts.pool.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token::transfer(cpi_ctx, amount)?;

        // Mint reward tokens to user if any
        if total_rewards > 0 {
            let mint_accounts = MintTo {
                mint: ctx.accounts.reward_mint.to_account_info(),
                to: ctx.accounts.user_reward_token.to_account_info(),
                authority: ctx.accounts.pool.to_account_info(),
            };
            let mint_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                mint_accounts,
                signer_seeds,
            );
            token::mint_to(mint_ctx, total_rewards)?;
        }

        // Update user stake
        let user_stake = &mut ctx.accounts.user_stake;
        user_stake.staked_amount = user_stake
            .staked_amount
            .checked_sub(amount)
            .ok_or(ErrorCode::Underflow)?;
        user_stake.pending_rewards = 0;
        user_stake.last_stake_slot = clock.slot;

        // Update pool total
        let pool = &mut ctx.accounts.pool;
        pool.total_staked = pool
            .total_staked
            .checked_sub(amount)
            .ok_or(ErrorCode::Underflow)?;
        pool.last_update_slot = clock.slot;

        Ok(())
    }

    /// Claim pending rewards without unstaking
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        let pool = &ctx.accounts.pool;
        let user_stake = &ctx.accounts.user_stake;
        let clock = Clock::get()?;

        require!(user_stake.staked_amount > 0, ErrorCode::NoStake);

        // Calculate pending rewards
        let slots_staked = clock.slot.saturating_sub(user_stake.last_stake_slot);
        let pending = user_stake
            .staked_amount
            .checked_mul(pool.reward_rate)
            .ok_or(ErrorCode::Overflow)?
            .checked_mul(slots_staked)
            .ok_or(ErrorCode::Overflow)?
            .checked_div(REWARD_SCALE)
            .unwrap_or(0);
        let total_rewards = user_stake
            .pending_rewards
            .checked_add(pending)
            .ok_or(ErrorCode::Overflow)?;

        require!(total_rewards > 0, ErrorCode::NoRewards);

        // Mint reward tokens to user using PDA signer
        // Pool PDA seeds: ["pool", stake_mint]
        let stake_mint_key = pool.stake_mint;
        let pool_bump = pool.bump;
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"pool",
            stake_mint_key.as_ref(),
            &[pool_bump],
        ]];

        let mint_accounts = MintTo {
            mint: ctx.accounts.reward_mint.to_account_info(),
            to: ctx.accounts.user_reward_token.to_account_info(),
            authority: ctx.accounts.pool.to_account_info(),
        };
        let mint_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            mint_accounts,
            signer_seeds,
        );
        token::mint_to(mint_ctx, total_rewards)?;

        // Update user stake
        let user_stake = &mut ctx.accounts.user_stake;
        user_stake.pending_rewards = 0;
        user_stake.last_stake_slot = clock.slot;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Mint of the token users will stake
    pub stake_mint: Account<'info, Mint>,

    /// Mint of the reward token (pool PDA must be mint authority)
    #[account(mut)]
    pub reward_mint: Account<'info, Mint>,

    /// Pool state account - PDA uses stake_mint only
    #[account(
        init,
        payer = authority,
        space = 8 + Pool::INIT_SPACE,
        seeds = [b"pool", stake_mint.key().as_ref()],
        bump
    )]
    pub pool: Account<'info, Pool>,

    /// Vault to hold staked tokens - PDA uses stake_mint only
    #[account(
        init,
        payer = authority,
        token::mint = stake_mint,
        token::authority = pool,
        seeds = [b"stake_vault", stake_mint.key().as_ref()],
        bump
    )]
    pub stake_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateUserStake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [b"pool", pool.stake_mint.as_ref()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,

    /// Stake mint is needed to derive user_stake PDA
    pub stake_mint: Account<'info, Mint>,

    /// User's stake account - PDA uses stake_mint + user
    #[account(
        init,
        payer = user,
        space = 8 + UserStake::INIT_SPACE,
        seeds = [b"user_stake", stake_mint.key().as_ref(), user.key().as_ref()],
        bump
    )]
    pub user_stake: Account<'info, UserStake>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"pool", pool.stake_mint.as_ref()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,

    /// User's stake account
    #[account(
        mut,
        seeds = [b"user_stake", pool.stake_mint.as_ref(), user.key().as_ref()],
        bump = user_stake.bump,
        constraint = user_stake.owner == user.key() @ ErrorCode::Unauthorized
    )]
    pub user_stake: Account<'info, UserStake>,

    /// User's token account for stake tokens
    #[account(
        mut,
        constraint = user_stake_token.mint == pool.stake_mint,
        constraint = user_stake_token.owner == user.key()
    )]
    pub user_stake_token: Account<'info, TokenAccount>,

    /// Pool's stake vault
    #[account(
        mut,
        seeds = [b"stake_vault", pool.stake_mint.as_ref()],
        bump
    )]
    pub stake_vault: Account<'info, TokenAccount>,

    /// Stake mint is needed for account validation
    pub stake_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"pool", pool.stake_mint.as_ref()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,

    /// Reward mint (pool must be mint authority)
    #[account(
        mut,
        constraint = reward_mint.key() == pool.reward_mint
    )]
    pub reward_mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [b"user_stake", pool.stake_mint.as_ref(), user.key().as_ref()],
        bump = user_stake.bump,
        constraint = user_stake.owner == user.key() @ ErrorCode::Unauthorized
    )]
    pub user_stake: Account<'info, UserStake>,

    /// User's token account for stake tokens
    #[account(
        mut,
        constraint = user_stake_token.mint == pool.stake_mint,
        constraint = user_stake_token.owner == user.key()
    )]
    pub user_stake_token: Account<'info, TokenAccount>,

    /// User's token account for reward tokens
    #[account(
        mut,
        constraint = user_reward_token.mint == pool.reward_mint,
        constraint = user_reward_token.owner == user.key()
    )]
    pub user_reward_token: Account<'info, TokenAccount>,

    /// Pool's stake vault
    #[account(
        mut,
        seeds = [b"stake_vault", pool.stake_mint.as_ref()],
        bump
    )]
    pub stake_vault: Account<'info, TokenAccount>,

    /// Stake mint is needed for account validation
    pub stake_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [b"pool", pool.stake_mint.as_ref()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,

    /// Reward mint (pool must be mint authority)
    #[account(
        mut,
        constraint = reward_mint.key() == pool.reward_mint
    )]
    pub reward_mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [b"user_stake", pool.stake_mint.as_ref(), user.key().as_ref()],
        bump = user_stake.bump,
        constraint = user_stake.owner == user.key() @ ErrorCode::Unauthorized
    )]
    pub user_stake: Account<'info, UserStake>,

    /// User's token account for reward tokens
    #[account(
        mut,
        constraint = user_reward_token.mint == pool.reward_mint,
        constraint = user_reward_token.owner == user.key()
    )]
    pub user_reward_token: Account<'info, TokenAccount>,

    /// Stake mint is needed for account validation
    pub stake_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(InitSpace)]
pub struct Pool {
    /// Authority who created the pool
    pub authority: Pubkey,
    /// Mint of the token being staked
    pub stake_mint: Pubkey,
    /// Mint of the reward token
    pub reward_mint: Pubkey,
    /// Token account holding staked tokens
    pub stake_vault: Pubkey,
    /// Reward tokens per staked token per slot (scaled by REWARD_SCALE)
    pub reward_rate: u64,
    /// Total tokens staked in pool
    pub total_staked: u64,
    /// Last slot when pool was updated
    pub last_update_slot: u64,
    /// Bump seed for pool PDA
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct UserStake {
    /// Owner of this stake account
    pub owner: Pubkey,
    /// Pool this stake belongs to
    pub pool: Pubkey,
    /// Amount of tokens staked
    pub staked_amount: u64,
    /// Pending rewards accumulated
    pub pending_rewards: u64,
    /// Slot when user last staked/claimed
    pub last_stake_slot: u64,
    /// Bump seed for user stake PDA
    pub bump: u8,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("Insufficient staked amount")]
    InsufficientStake,
    #[msg("No tokens staked")]
    NoStake,
    #[msg("No rewards to claim")]
    NoRewards,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Arithmetic underflow")]
    Underflow,
    #[msg("Unauthorized access")]
    Unauthorized,
}
