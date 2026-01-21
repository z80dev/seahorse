use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer, MintTo, Burn};

declare_id!("6LEndKVBqeRVFT6UYm5NN1SqBSCqCWvPaoYdJzrgLMge");

// Collateral ratio: 150% (must deposit 1.5x the value of borrowed amount)
// Stored as basis points: 15000 = 150%
const COLLATERAL_RATIO_BPS: u64 = 15000;
const BPS_DENOMINATOR: u64 = 10000;

// Interest rate per slot (simplified)
// e.g., 100 = 0.01% per slot (scaled by 1,000,000)
const INTEREST_SCALE: u64 = 1_000_000;

#[program]
pub mod lending_anchor {
    use super::*;

    /// Initialize a new lending pool
    /// Creates pool state and token vaults for collateral and borrowable tokens
    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        interest_rate: u64, // Interest per slot (scaled by INTEREST_SCALE)
    ) -> Result<()> {
        let pool = &mut ctx.accounts.pool;
        pool.authority = ctx.accounts.authority.key();
        pool.collateral_mint = ctx.accounts.collateral_mint.key();
        pool.borrow_mint = ctx.accounts.borrow_mint.key();
        pool.collateral_vault = ctx.accounts.collateral_vault.key();
        pool.borrow_vault = ctx.accounts.borrow_vault.key();
        pool.interest_rate = interest_rate;
        pool.total_deposited = 0;
        pool.total_borrowed = 0;
        pool.last_update_slot = Clock::get()?.slot;
        pool.bump = ctx.bumps.pool;
        Ok(())
    }

    /// Create a user position account
    pub fn create_user_position(ctx: Context<CreateUserPosition>) -> Result<()> {
        let position = &mut ctx.accounts.user_position;
        position.owner = ctx.accounts.user.key();
        position.pool = ctx.accounts.pool.key();
        position.collateral_deposited = 0;
        position.borrowed_amount = 0;
        position.last_update_slot = Clock::get()?.slot;
        position.bump = ctx.bumps.user_position;
        Ok(())
    }

    /// Deposit collateral into the pool
    pub fn deposit_collateral(ctx: Context<DepositCollateral>, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);

        // Transfer collateral from user to vault
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_collateral_token.to_account_info(),
            to: ctx.accounts.collateral_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        );
        token::transfer(cpi_ctx, amount)?;

        // Update user position
        let position = &mut ctx.accounts.user_position;
        position.collateral_deposited = position
            .collateral_deposited
            .checked_add(amount)
            .ok_or(ErrorCode::Overflow)?;

        // Update pool
        let pool = &mut ctx.accounts.pool;
        pool.total_deposited = pool
            .total_deposited
            .checked_add(amount)
            .ok_or(ErrorCode::Overflow)?;

        Ok(())
    }

    /// Borrow tokens against deposited collateral
    /// Requires collateral ratio to be maintained
    pub fn borrow(ctx: Context<Borrow>, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);

        let position = &ctx.accounts.user_position;
        let pool = &ctx.accounts.pool;
        let clock = Clock::get()?;

        // Calculate accrued interest on existing debt
        let accrued_interest = calculate_interest(
            position.borrowed_amount,
            pool.interest_rate,
            clock.slot.saturating_sub(position.last_update_slot),
        )?;

        let new_debt = position
            .borrowed_amount
            .checked_add(accrued_interest)
            .ok_or(ErrorCode::Overflow)?
            .checked_add(amount)
            .ok_or(ErrorCode::Overflow)?;

        // Check collateral ratio
        // Required collateral = borrowed * COLLATERAL_RATIO_BPS / BPS_DENOMINATOR
        // Simplified: collateral * BPS_DENOMINATOR >= borrowed * COLLATERAL_RATIO_BPS
        let required_collateral = new_debt
            .checked_mul(COLLATERAL_RATIO_BPS)
            .ok_or(ErrorCode::Overflow)?
            .checked_div(BPS_DENOMINATOR)
            .unwrap_or(0);

        require!(
            position.collateral_deposited >= required_collateral,
            ErrorCode::InsufficientCollateral
        );

        // Check pool has enough liquidity
        let vault_balance = ctx.accounts.borrow_vault.amount;
        require!(vault_balance >= amount, ErrorCode::InsufficientLiquidity);

        // Transfer borrowed tokens to user using PDA signer
        let collateral_mint_key = pool.collateral_mint;
        let pool_bump = pool.bump;
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"pool",
            collateral_mint_key.as_ref(),
            &[pool_bump],
        ]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.borrow_vault.to_account_info(),
            to: ctx.accounts.user_borrow_token.to_account_info(),
            authority: ctx.accounts.pool.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token::transfer(cpi_ctx, amount)?;

        // Update user position
        let position = &mut ctx.accounts.user_position;
        position.borrowed_amount = new_debt;
        position.last_update_slot = clock.slot;

        // Update pool
        let pool = &mut ctx.accounts.pool;
        pool.total_borrowed = pool
            .total_borrowed
            .checked_add(amount)
            .ok_or(ErrorCode::Overflow)?;
        pool.last_update_slot = clock.slot;

        Ok(())
    }

    /// Repay borrowed tokens
    /// Reduces debt including any accrued interest
    pub fn repay(ctx: Context<Repay>, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);

        let position = &ctx.accounts.user_position;
        let pool = &ctx.accounts.pool;
        let clock = Clock::get()?;

        // Calculate accrued interest
        let accrued_interest = calculate_interest(
            position.borrowed_amount,
            pool.interest_rate,
            clock.slot.saturating_sub(position.last_update_slot),
        )?;

        let total_debt = position
            .borrowed_amount
            .checked_add(accrued_interest)
            .ok_or(ErrorCode::Overflow)?;

        // Cannot repay more than total debt
        let repay_amount = amount.min(total_debt);

        // Transfer tokens from user to vault
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_borrow_token.to_account_info(),
            to: ctx.accounts.borrow_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        );
        token::transfer(cpi_ctx, repay_amount)?;

        // Update user position
        let position = &mut ctx.accounts.user_position;
        position.borrowed_amount = total_debt
            .checked_sub(repay_amount)
            .ok_or(ErrorCode::Underflow)?;
        position.last_update_slot = clock.slot;

        // Update pool (reduce total_borrowed by the principal portion only for simplicity)
        let pool = &mut ctx.accounts.pool;
        let principal_repaid = repay_amount.min(pool.total_borrowed);
        pool.total_borrowed = pool
            .total_borrowed
            .saturating_sub(principal_repaid);
        pool.last_update_slot = clock.slot;

        Ok(())
    }

    /// Withdraw collateral
    /// Requires maintaining collateral ratio after withdrawal
    pub fn withdraw_collateral(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);

        let position = &ctx.accounts.user_position;
        let pool = &ctx.accounts.pool;
        let clock = Clock::get()?;

        require!(
            position.collateral_deposited >= amount,
            ErrorCode::InsufficientCollateral
        );

        // Calculate accrued interest
        let accrued_interest = calculate_interest(
            position.borrowed_amount,
            pool.interest_rate,
            clock.slot.saturating_sub(position.last_update_slot),
        )?;

        let total_debt = position
            .borrowed_amount
            .checked_add(accrued_interest)
            .ok_or(ErrorCode::Overflow)?;

        // Check remaining collateral maintains ratio
        let remaining_collateral = position
            .collateral_deposited
            .checked_sub(amount)
            .ok_or(ErrorCode::Underflow)?;

        if total_debt > 0 {
            let required_collateral = total_debt
                .checked_mul(COLLATERAL_RATIO_BPS)
                .ok_or(ErrorCode::Overflow)?
                .checked_div(BPS_DENOMINATOR)
                .unwrap_or(0);

            require!(
                remaining_collateral >= required_collateral,
                ErrorCode::InsufficientCollateral
            );
        }

        // Transfer collateral back to user using PDA signer
        let collateral_mint_key = pool.collateral_mint;
        let pool_bump = pool.bump;
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"pool",
            collateral_mint_key.as_ref(),
            &[pool_bump],
        ]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.collateral_vault.to_account_info(),
            to: ctx.accounts.user_collateral_token.to_account_info(),
            authority: ctx.accounts.pool.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token::transfer(cpi_ctx, amount)?;

        // Update user position
        let position = &mut ctx.accounts.user_position;
        position.collateral_deposited = remaining_collateral;
        position.borrowed_amount = total_debt;
        position.last_update_slot = clock.slot;

        // Update pool
        let pool = &mut ctx.accounts.pool;
        pool.total_deposited = pool
            .total_deposited
            .saturating_sub(amount);
        pool.last_update_slot = clock.slot;

        Ok(())
    }
}

fn calculate_interest(principal: u64, rate: u64, slots: u64) -> Result<u64> {
    // interest = principal * rate * slots / INTEREST_SCALE
    let interest = (principal as u128)
        .checked_mul(rate as u128)
        .ok_or(ErrorCode::Overflow)?
        .checked_mul(slots as u128)
        .ok_or(ErrorCode::Overflow)?
        .checked_div(INTEREST_SCALE as u128)
        .unwrap_or(0);

    Ok(interest.min(u64::MAX as u128) as u64)
}

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Mint of the collateral token
    pub collateral_mint: Account<'info, Mint>,

    /// Mint of the borrowable token
    pub borrow_mint: Account<'info, Mint>,

    /// Pool state account - PDA uses collateral_mint only
    #[account(
        init,
        payer = authority,
        space = 8 + Pool::INIT_SPACE,
        seeds = [b"pool", collateral_mint.key().as_ref()],
        bump
    )]
    pub pool: Account<'info, Pool>,

    /// Vault for collateral tokens
    #[account(
        init,
        payer = authority,
        token::mint = collateral_mint,
        token::authority = pool,
        seeds = [b"collateral_vault", collateral_mint.key().as_ref()],
        bump
    )]
    pub collateral_vault: Account<'info, TokenAccount>,

    /// Vault for borrowable tokens
    #[account(
        init,
        payer = authority,
        token::mint = borrow_mint,
        token::authority = pool,
        seeds = [b"borrow_vault", collateral_mint.key().as_ref()],
        bump
    )]
    pub borrow_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateUserPosition<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [b"pool", pool.collateral_mint.as_ref()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,

    /// Collateral mint is needed to derive user_position PDA
    pub collateral_mint: Account<'info, Mint>,

    /// User's position account
    #[account(
        init,
        payer = user,
        space = 8 + UserPosition::INIT_SPACE,
        seeds = [b"user_position", collateral_mint.key().as_ref(), user.key().as_ref()],
        bump
    )]
    pub user_position: Account<'info, UserPosition>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositCollateral<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"pool", pool.collateral_mint.as_ref()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        seeds = [b"user_position", pool.collateral_mint.as_ref(), user.key().as_ref()],
        bump = user_position.bump,
        constraint = user_position.owner == user.key() @ ErrorCode::Unauthorized
    )]
    pub user_position: Account<'info, UserPosition>,

    /// User's collateral token account
    #[account(
        mut,
        constraint = user_collateral_token.mint == pool.collateral_mint,
        constraint = user_collateral_token.owner == user.key()
    )]
    pub user_collateral_token: Account<'info, TokenAccount>,

    /// Pool's collateral vault
    #[account(
        mut,
        seeds = [b"collateral_vault", pool.collateral_mint.as_ref()],
        bump
    )]
    pub collateral_vault: Account<'info, TokenAccount>,

    pub collateral_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Borrow<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"pool", pool.collateral_mint.as_ref()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        seeds = [b"user_position", pool.collateral_mint.as_ref(), user.key().as_ref()],
        bump = user_position.bump,
        constraint = user_position.owner == user.key() @ ErrorCode::Unauthorized
    )]
    pub user_position: Account<'info, UserPosition>,

    /// User's borrow token account
    #[account(
        mut,
        constraint = user_borrow_token.mint == pool.borrow_mint,
        constraint = user_borrow_token.owner == user.key()
    )]
    pub user_borrow_token: Account<'info, TokenAccount>,

    /// Pool's borrow vault
    #[account(
        mut,
        seeds = [b"borrow_vault", pool.collateral_mint.as_ref()],
        bump
    )]
    pub borrow_vault: Account<'info, TokenAccount>,

    pub collateral_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Repay<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"pool", pool.collateral_mint.as_ref()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        seeds = [b"user_position", pool.collateral_mint.as_ref(), user.key().as_ref()],
        bump = user_position.bump,
        constraint = user_position.owner == user.key() @ ErrorCode::Unauthorized
    )]
    pub user_position: Account<'info, UserPosition>,

    /// User's borrow token account
    #[account(
        mut,
        constraint = user_borrow_token.mint == pool.borrow_mint,
        constraint = user_borrow_token.owner == user.key()
    )]
    pub user_borrow_token: Account<'info, TokenAccount>,

    /// Pool's borrow vault
    #[account(
        mut,
        seeds = [b"borrow_vault", pool.collateral_mint.as_ref()],
        bump
    )]
    pub borrow_vault: Account<'info, TokenAccount>,

    pub collateral_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct WithdrawCollateral<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"pool", pool.collateral_mint.as_ref()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        seeds = [b"user_position", pool.collateral_mint.as_ref(), user.key().as_ref()],
        bump = user_position.bump,
        constraint = user_position.owner == user.key() @ ErrorCode::Unauthorized
    )]
    pub user_position: Account<'info, UserPosition>,

    /// User's collateral token account
    #[account(
        mut,
        constraint = user_collateral_token.mint == pool.collateral_mint,
        constraint = user_collateral_token.owner == user.key()
    )]
    pub user_collateral_token: Account<'info, TokenAccount>,

    /// Pool's collateral vault
    #[account(
        mut,
        seeds = [b"collateral_vault", pool.collateral_mint.as_ref()],
        bump
    )]
    pub collateral_vault: Account<'info, TokenAccount>,

    pub collateral_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(InitSpace)]
pub struct Pool {
    /// Authority who created the pool
    pub authority: Pubkey,
    /// Mint of the collateral token
    pub collateral_mint: Pubkey,
    /// Mint of the borrowable token
    pub borrow_mint: Pubkey,
    /// Token account holding collateral
    pub collateral_vault: Pubkey,
    /// Token account holding borrowable tokens
    pub borrow_vault: Pubkey,
    /// Interest rate per slot (scaled by INTEREST_SCALE)
    pub interest_rate: u64,
    /// Total collateral deposited
    pub total_deposited: u64,
    /// Total amount borrowed
    pub total_borrowed: u64,
    /// Last slot when pool was updated
    pub last_update_slot: u64,
    /// Bump seed for pool PDA
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct UserPosition {
    /// Owner of this position
    pub owner: Pubkey,
    /// Pool this position belongs to
    pub pool: Pubkey,
    /// Collateral deposited by user
    pub collateral_deposited: u64,
    /// Amount borrowed by user (including accrued interest)
    pub borrowed_amount: u64,
    /// Last slot when position was updated
    pub last_update_slot: u64,
    /// Bump seed for position PDA
    pub bump: u8,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("Insufficient collateral for this operation")]
    InsufficientCollateral,
    #[msg("Insufficient liquidity in pool")]
    InsufficientLiquidity,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Arithmetic underflow")]
    Underflow,
    #[msg("Unauthorized access")]
    Unauthorized,
}
