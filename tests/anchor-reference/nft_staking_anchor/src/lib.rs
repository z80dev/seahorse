use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer, MintTo};

declare_id!("8DtVbTgYbcM8b2igVzb7RjbVKLLLDQ2bzXriSFp8fxhb");

// Scale factor for reward calculations (avoids floating point)
const REWARD_SCALE: u64 = 1_000_000;

#[program]
pub mod nft_staking_anchor {
    use super::*;

    /// Initialize a new NFT staking pool
    /// Creates pool state account
    /// Note: Pool PDA uses only reward_mint for simpler signer seeds
    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        reward_rate: u64, // Reward tokens per NFT per slot (scaled by REWARD_SCALE)
    ) -> Result<()> {
        let pool = &mut ctx.accounts.pool;
        pool.authority = ctx.accounts.authority.key();
        pool.reward_mint = ctx.accounts.reward_mint.key();
        pool.reward_rate = reward_rate;
        pool.total_staked = 0;
        pool.last_update_slot = Clock::get()?.slot;
        pool.bump = ctx.bumps.pool;
        Ok(())
    }

    /// Stake an NFT into the pool
    /// Transfers NFT from user to a PDA-owned vault
    /// Creates a stake record for the user's NFT
    pub fn stake_nft(ctx: Context<StakeNft>) -> Result<()> {
        let pool = &mut ctx.accounts.pool;
        let stake_record = &mut ctx.accounts.stake_record;
        let clock = Clock::get()?;

        // Initialize stake record
        stake_record.owner = ctx.accounts.user.key();
        stake_record.pool = pool.key();
        stake_record.nft_mint = ctx.accounts.nft_mint.key();
        stake_record.nft_vault = ctx.accounts.nft_vault.key();
        stake_record.staked_at_slot = clock.slot;
        stake_record.last_claim_slot = clock.slot;
        stake_record.pending_rewards = 0;
        stake_record.is_staked = true;
        stake_record.bump = ctx.bumps.stake_record;

        // Transfer NFT from user to vault
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_nft_token.to_account_info(),
            to: ctx.accounts.nft_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        );
        token::transfer(cpi_ctx, 1)?; // NFTs have amount = 1

        // Update pool total
        pool.total_staked = pool
            .total_staked
            .checked_add(1)
            .ok_or(ErrorCode::Overflow)?;
        pool.last_update_slot = clock.slot;

        Ok(())
    }

    /// Unstake an NFT from the pool
    /// Returns NFT to user and claims all pending rewards
    pub fn unstake_nft(ctx: Context<UnstakeNft>) -> Result<()> {
        let stake_record = &ctx.accounts.stake_record;
        require!(stake_record.is_staked, ErrorCode::NotStaked);

        let pool = &ctx.accounts.pool;
        let clock = Clock::get()?;

        // Calculate pending rewards
        let slots_staked = clock.slot.saturating_sub(stake_record.last_claim_slot);
        let pending = pool
            .reward_rate
            .checked_mul(slots_staked)
            .ok_or(ErrorCode::Overflow)?
            .checked_div(REWARD_SCALE)
            .unwrap_or(0);
        let total_rewards = stake_record
            .pending_rewards
            .checked_add(pending)
            .ok_or(ErrorCode::Overflow)?;

        // Transfer NFT back to user using PDA signer
        // Stake record PDA seeds: ["stake_record", nft_mint]
        let nft_mint_key = stake_record.nft_mint;
        let stake_bump = stake_record.bump;
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"stake_record",
            nft_mint_key.as_ref(),
            &[stake_bump],
        ]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.nft_vault.to_account_info(),
            to: ctx.accounts.user_nft_token.to_account_info(),
            authority: ctx.accounts.stake_record.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token::transfer(cpi_ctx, 1)?;

        // Mint reward tokens to user if any
        if total_rewards > 0 {
            let pool_bump = pool.bump;
            let reward_mint_key = pool.reward_mint;
            let pool_signer_seeds: &[&[&[u8]]] = &[&[
                b"pool",
                reward_mint_key.as_ref(),
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
                pool_signer_seeds,
            );
            token::mint_to(mint_ctx, total_rewards)?;
        }

        // Update stake record
        let stake_record = &mut ctx.accounts.stake_record;
        stake_record.is_staked = false;
        stake_record.pending_rewards = 0;
        stake_record.last_claim_slot = clock.slot;

        // Update pool total
        let pool = &mut ctx.accounts.pool;
        pool.total_staked = pool
            .total_staked
            .checked_sub(1)
            .ok_or(ErrorCode::Underflow)?;
        pool.last_update_slot = clock.slot;

        Ok(())
    }

    /// Claim pending rewards without unstaking
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        let stake_record = &ctx.accounts.stake_record;
        require!(stake_record.is_staked, ErrorCode::NotStaked);

        let pool = &ctx.accounts.pool;
        let clock = Clock::get()?;

        // Calculate pending rewards
        let slots_staked = clock.slot.saturating_sub(stake_record.last_claim_slot);
        let pending = pool
            .reward_rate
            .checked_mul(slots_staked)
            .ok_or(ErrorCode::Overflow)?
            .checked_div(REWARD_SCALE)
            .unwrap_or(0);
        let total_rewards = stake_record
            .pending_rewards
            .checked_add(pending)
            .ok_or(ErrorCode::Overflow)?;

        require!(total_rewards > 0, ErrorCode::NoRewards);

        // Mint reward tokens to user using pool PDA signer
        let pool_bump = pool.bump;
        let reward_mint_key = pool.reward_mint;
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"pool",
            reward_mint_key.as_ref(),
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

        // Update stake record
        let stake_record = &mut ctx.accounts.stake_record;
        stake_record.pending_rewards = 0;
        stake_record.last_claim_slot = clock.slot;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Mint of the reward token (pool PDA must be mint authority)
    #[account(mut)]
    pub reward_mint: Account<'info, Mint>,

    /// Pool state account - PDA uses reward_mint
    #[account(
        init,
        payer = authority,
        space = 8 + Pool::INIT_SPACE,
        seeds = [b"pool", reward_mint.key().as_ref()],
        bump
    )]
    pub pool: Account<'info, Pool>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct StakeNft<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"pool", pool.reward_mint.as_ref()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,

    /// The NFT mint (supply should be 1)
    pub nft_mint: Account<'info, Mint>,

    /// User's NFT token account
    #[account(
        mut,
        constraint = user_nft_token.mint == nft_mint.key(),
        constraint = user_nft_token.owner == user.key(),
        constraint = user_nft_token.amount == 1 @ ErrorCode::NotOwned
    )]
    pub user_nft_token: Account<'info, TokenAccount>,

    /// Vault to hold the staked NFT - PDA uses nft_mint
    /// Stake record is the authority (for transfer back)
    #[account(
        init,
        payer = user,
        token::mint = nft_mint,
        token::authority = stake_record,
        seeds = [b"nft_vault", nft_mint.key().as_ref()],
        bump
    )]
    pub nft_vault: Account<'info, TokenAccount>,

    /// Stake record for this NFT - PDA uses nft_mint
    #[account(
        init,
        payer = user,
        space = 8 + StakeRecord::INIT_SPACE,
        seeds = [b"stake_record", nft_mint.key().as_ref()],
        bump
    )]
    pub stake_record: Account<'info, StakeRecord>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UnstakeNft<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"pool", pool.reward_mint.as_ref()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,

    /// Reward mint (pool must be mint authority)
    #[account(
        mut,
        constraint = reward_mint.key() == pool.reward_mint
    )]
    pub reward_mint: Account<'info, Mint>,

    /// The NFT mint
    pub nft_mint: Account<'info, Mint>,

    /// Stake record for this NFT
    #[account(
        mut,
        seeds = [b"stake_record", nft_mint.key().as_ref()],
        bump = stake_record.bump,
        constraint = stake_record.owner == user.key() @ ErrorCode::Unauthorized,
        constraint = stake_record.pool == pool.key()
    )]
    pub stake_record: Account<'info, StakeRecord>,

    /// Vault holding the staked NFT
    #[account(
        mut,
        seeds = [b"nft_vault", nft_mint.key().as_ref()],
        bump,
        constraint = nft_vault.amount == 1
    )]
    pub nft_vault: Account<'info, TokenAccount>,

    /// User's NFT token account (to receive the NFT back)
    #[account(
        mut,
        constraint = user_nft_token.mint == nft_mint.key(),
        constraint = user_nft_token.owner == user.key()
    )]
    pub user_nft_token: Account<'info, TokenAccount>,

    /// User's reward token account
    #[account(
        mut,
        constraint = user_reward_token.mint == pool.reward_mint,
        constraint = user_reward_token.owner == user.key()
    )]
    pub user_reward_token: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [b"pool", pool.reward_mint.as_ref()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,

    /// Reward mint (pool must be mint authority)
    #[account(
        mut,
        constraint = reward_mint.key() == pool.reward_mint
    )]
    pub reward_mint: Account<'info, Mint>,

    /// The NFT mint
    pub nft_mint: Account<'info, Mint>,

    /// Stake record for this NFT
    #[account(
        mut,
        seeds = [b"stake_record", nft_mint.key().as_ref()],
        bump = stake_record.bump,
        constraint = stake_record.owner == user.key() @ ErrorCode::Unauthorized,
        constraint = stake_record.is_staked @ ErrorCode::NotStaked
    )]
    pub stake_record: Account<'info, StakeRecord>,

    /// User's reward token account
    #[account(
        mut,
        constraint = user_reward_token.mint == pool.reward_mint,
        constraint = user_reward_token.owner == user.key()
    )]
    pub user_reward_token: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(InitSpace)]
pub struct Pool {
    /// Authority who created the pool
    pub authority: Pubkey,
    /// Mint of the reward token
    pub reward_mint: Pubkey,
    /// Reward tokens per NFT per slot (scaled by REWARD_SCALE)
    pub reward_rate: u64,
    /// Total NFTs staked in pool
    pub total_staked: u64,
    /// Last slot when pool was updated
    pub last_update_slot: u64,
    /// Bump seed for pool PDA
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct StakeRecord {
    /// Owner who staked the NFT
    pub owner: Pubkey,
    /// Pool this stake belongs to
    pub pool: Pubkey,
    /// Mint of the staked NFT
    pub nft_mint: Pubkey,
    /// Vault holding the NFT
    pub nft_vault: Pubkey,
    /// Slot when NFT was staked
    pub staked_at_slot: u64,
    /// Slot when rewards were last claimed
    pub last_claim_slot: u64,
    /// Pending rewards accumulated
    pub pending_rewards: u64,
    /// Whether NFT is currently staked
    pub is_staked: bool,
    /// Bump seed for stake record PDA
    pub bump: u8,
}

#[error_code]
pub enum ErrorCode {
    #[msg("NFT not staked")]
    NotStaked,
    #[msg("User does not own the NFT")]
    NotOwned,
    #[msg("No rewards to claim")]
    NoRewards,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Arithmetic underflow")]
    Underflow,
    #[msg("Unauthorized access")]
    Unauthorized,
}
