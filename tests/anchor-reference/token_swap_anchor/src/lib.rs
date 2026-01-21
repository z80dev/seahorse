use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Burn, Token, TokenAccount, Transfer};

declare_id!("SwAP5VzVZYxLYtDmJq5DEmUKDEcPnP2zGxqHKbv2AmM");

/// Minimum liquidity locked in pool on first deposit to prevent manipulation
pub const MINIMUM_LIQUIDITY: u64 = 100;

/// Fee in basis points (e.g., 30 = 0.3%)
pub const FEE_BPS: u64 = 30;

#[program]
pub mod token_swap_anchor {
    use super::*;

    /// Initialize a new liquidity pool
    /// Creates pool state, LP token mint, and pool token accounts
    pub fn initialize_pool(ctx: Context<InitializePool>, fee_bps: u16) -> Result<()> {
        require!(fee_bps < 10000, ErrorCode::InvalidFee);

        let pool = &mut ctx.accounts.pool;
        pool.mint_a = ctx.accounts.mint_a.key();
        pool.mint_b = ctx.accounts.mint_b.key();
        pool.lp_mint = ctx.accounts.lp_mint.key();
        pool.pool_token_a = ctx.accounts.pool_token_a.key();
        pool.pool_token_b = ctx.accounts.pool_token_b.key();
        pool.fee_bps = fee_bps;
        pool.bump = ctx.bumps.pool;
        pool.lp_mint_bump = ctx.bumps.lp_mint;

        msg!("Pool initialized with fee: {} bps", fee_bps);
        Ok(())
    }

    /// Add liquidity to the pool
    /// For first deposit: mints sqrt(amount_a * amount_b) - MINIMUM_LIQUIDITY LP tokens
    /// For subsequent deposits: mints proportional LP tokens, excess tokens of one type are not deposited
    pub fn add_liquidity(
        ctx: Context<AddLiquidity>,
        amount_a: u64,
        amount_b: u64,
        min_lp_tokens: u64,
    ) -> Result<()> {
        require!(amount_a > 0 && amount_b > 0, ErrorCode::InvalidAmount);

        let pool_a_balance = ctx.accounts.pool_token_a.amount;
        let pool_b_balance = ctx.accounts.pool_token_b.amount;
        let lp_supply = ctx.accounts.lp_mint.supply;

        let (deposit_a, deposit_b, lp_tokens) = if lp_supply == 0 {
            // First deposit: use geometric mean and lock minimum liquidity
            let lp_tokens = integer_sqrt(amount_a as u128 * amount_b as u128);
            require!(lp_tokens > MINIMUM_LIQUIDITY as u128, ErrorCode::DepositTooSmall);
            let lp_tokens = (lp_tokens - MINIMUM_LIQUIDITY as u128) as u64;
            (amount_a, amount_b, lp_tokens)
        } else {
            // Calculate proportional deposits based on existing pool ratio
            // Use the smaller ratio to determine actual deposits
            let ratio_a = (amount_a as u128 * lp_supply as u128) / pool_a_balance as u128;
            let ratio_b = (amount_b as u128 * lp_supply as u128) / pool_b_balance as u128;

            let (deposit_a, deposit_b, lp_tokens) = if ratio_a <= ratio_b {
                let deposit_a = amount_a;
                let deposit_b = (amount_a as u128 * pool_b_balance as u128 / pool_a_balance as u128) as u64;
                let lp_tokens = (deposit_a as u128 * lp_supply as u128 / pool_a_balance as u128) as u64;
                (deposit_a, deposit_b, lp_tokens)
            } else {
                let deposit_b = amount_b;
                let deposit_a = (amount_b as u128 * pool_a_balance as u128 / pool_b_balance as u128) as u64;
                let lp_tokens = (deposit_b as u128 * lp_supply as u128 / pool_b_balance as u128) as u64;
                (deposit_a, deposit_b, lp_tokens)
            };

            (deposit_a, deposit_b, lp_tokens)
        };

        require!(lp_tokens >= min_lp_tokens, ErrorCode::SlippageExceeded);

        // Transfer token A to pool
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_a.to_account_info(),
                to: ctx.accounts.pool_token_a.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, deposit_a)?;

        // Transfer token B to pool
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_b.to_account_info(),
                to: ctx.accounts.pool_token_b.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, deposit_b)?;

        // Mint LP tokens to user
        let pool = &ctx.accounts.pool;
        let mint_a_key = pool.mint_a;
        let mint_b_key = pool.mint_b;
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"pool",
            mint_a_key.as_ref(),
            mint_b_key.as_ref(),
            &[pool.bump],
        ]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.lp_mint.to_account_info(),
                to: ctx.accounts.user_lp_token.to_account_info(),
                authority: ctx.accounts.pool.to_account_info(),
            },
            signer_seeds,
        );
        token::mint_to(cpi_ctx, lp_tokens)?;

        msg!("Added liquidity: {} A, {} B, minted {} LP", deposit_a, deposit_b, lp_tokens);
        Ok(())
    }

    /// Remove liquidity from the pool
    /// Burns LP tokens and returns proportional amounts of both tokens
    pub fn remove_liquidity(
        ctx: Context<RemoveLiquidity>,
        lp_amount: u64,
        min_amount_a: u64,
        min_amount_b: u64,
    ) -> Result<()> {
        require!(lp_amount > 0, ErrorCode::InvalidAmount);

        let pool_a_balance = ctx.accounts.pool_token_a.amount;
        let pool_b_balance = ctx.accounts.pool_token_b.amount;
        let lp_supply = ctx.accounts.lp_mint.supply;

        // Calculate proportional withdrawals
        let amount_a = (lp_amount as u128 * pool_a_balance as u128 / lp_supply as u128) as u64;
        let amount_b = (lp_amount as u128 * pool_b_balance as u128 / lp_supply as u128) as u64;

        require!(amount_a >= min_amount_a && amount_b >= min_amount_b, ErrorCode::SlippageExceeded);

        // Burn LP tokens
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.lp_mint.to_account_info(),
                from: ctx.accounts.user_lp_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::burn(cpi_ctx, lp_amount)?;

        // Transfer tokens to user
        let pool = &ctx.accounts.pool;
        let mint_a_key = pool.mint_a;
        let mint_b_key = pool.mint_b;
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"pool",
            mint_a_key.as_ref(),
            mint_b_key.as_ref(),
            &[pool.bump],
        ]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.pool_token_a.to_account_info(),
                to: ctx.accounts.user_token_a.to_account_info(),
                authority: ctx.accounts.pool.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(cpi_ctx, amount_a)?;

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.pool_token_b.to_account_info(),
                to: ctx.accounts.user_token_b.to_account_info(),
                authority: ctx.accounts.pool.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(cpi_ctx, amount_b)?;

        msg!("Removed liquidity: {} LP, received {} A, {} B", lp_amount, amount_a, amount_b);
        Ok(())
    }

    /// Swap tokens using constant product formula (x * y = k)
    /// swap_a_to_b: true = swap A for B, false = swap B for A
    pub fn swap(
        ctx: Context<Swap>,
        amount_in: u64,
        min_amount_out: u64,
        swap_a_to_b: bool,
    ) -> Result<()> {
        require!(amount_in > 0, ErrorCode::InvalidAmount);

        let pool = &ctx.accounts.pool;
        let pool_a_balance = ctx.accounts.pool_token_a.amount;
        let pool_b_balance = ctx.accounts.pool_token_b.amount;

        // Calculate output using constant product formula with fee
        // output = (input * (10000 - fee_bps) * reserve_out) / (reserve_in * 10000 + input * (10000 - fee_bps))
        let fee_bps = pool.fee_bps as u128;
        let amount_in_with_fee = amount_in as u128 * (10000 - fee_bps);

        let (reserve_in, reserve_out) = if swap_a_to_b {
            (pool_a_balance as u128, pool_b_balance as u128)
        } else {
            (pool_b_balance as u128, pool_a_balance as u128)
        };

        let numerator = amount_in_with_fee * reserve_out;
        let denominator = reserve_in * 10000 + amount_in_with_fee;
        let amount_out = (numerator / denominator) as u64;

        require!(amount_out >= min_amount_out, ErrorCode::SlippageExceeded);

        let mint_a_key = pool.mint_a;
        let mint_b_key = pool.mint_b;
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"pool",
            mint_a_key.as_ref(),
            mint_b_key.as_ref(),
            &[pool.bump],
        ]];

        if swap_a_to_b {
            // Transfer A from user to pool
            let cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_a.to_account_info(),
                    to: ctx.accounts.pool_token_a.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            );
            token::transfer(cpi_ctx, amount_in)?;

            // Transfer B from pool to user
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.pool_token_b.to_account_info(),
                    to: ctx.accounts.user_token_b.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                signer_seeds,
            );
            token::transfer(cpi_ctx, amount_out)?;
        } else {
            // Transfer B from user to pool
            let cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_b.to_account_info(),
                    to: ctx.accounts.pool_token_b.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            );
            token::transfer(cpi_ctx, amount_in)?;

            // Transfer A from pool to user
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.pool_token_a.to_account_info(),
                    to: ctx.accounts.user_token_a.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                signer_seeds,
            );
            token::transfer(cpi_ctx, amount_out)?;
        }

        msg!("Swapped {} in for {} out (a_to_b: {})", amount_in, amount_out, swap_a_to_b);
        Ok(())
    }
}

/// Integer square root using Newton's method
fn integer_sqrt(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub mint_a: Account<'info, Mint>,
    pub mint_b: Account<'info, Mint>,

    #[account(
        init,
        payer = payer,
        space = 8 + Pool::INIT_SPACE,
        seeds = [b"pool", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump
    )]
    pub pool: Account<'info, Pool>,

    /// LP token mint - pool is the mint authority
    #[account(
        init,
        payer = payer,
        mint::decimals = 6,
        mint::authority = pool,
        seeds = [b"lp_mint", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump
    )]
    pub lp_mint: Account<'info, Mint>,

    /// Pool's token A account - pool is the authority
    #[account(
        init,
        payer = payer,
        token::mint = mint_a,
        token::authority = pool,
        seeds = [b"pool_token_a", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump
    )]
    pub pool_token_a: Account<'info, TokenAccount>,

    /// Pool's token B account - pool is the authority
    #[account(
        init,
        payer = payer,
        token::mint = mint_b,
        token::authority = pool,
        seeds = [b"pool_token_b", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump
    )]
    pub pool_token_b: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddLiquidity<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub mint_a: Account<'info, Mint>,
    pub mint_b: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [b"pool", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        seeds = [b"lp_mint", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump = pool.lp_mint_bump
    )]
    pub lp_mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [b"pool_token_a", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump
    )]
    pub pool_token_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"pool_token_b", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump
    )]
    pub pool_token_b: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_token_a.owner == user.key()
    )]
    pub user_token_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_token_b.owner == user.key()
    )]
    pub user_token_b: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_lp_token.owner == user.key()
    )]
    pub user_lp_token: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct RemoveLiquidity<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub mint_a: Account<'info, Mint>,
    pub mint_b: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [b"pool", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        seeds = [b"lp_mint", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump = pool.lp_mint_bump
    )]
    pub lp_mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [b"pool_token_a", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump
    )]
    pub pool_token_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"pool_token_b", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump
    )]
    pub pool_token_b: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_token_a.owner == user.key()
    )]
    pub user_token_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_token_b.owner == user.key()
    )]
    pub user_token_b: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_lp_token.owner == user.key()
    )]
    pub user_lp_token: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub mint_a: Account<'info, Mint>,
    pub mint_b: Account<'info, Mint>,

    #[account(
        seeds = [b"pool", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        seeds = [b"pool_token_a", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump
    )]
    pub pool_token_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"pool_token_b", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump
    )]
    pub pool_token_b: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_token_a.owner == user.key()
    )]
    pub user_token_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_token_b.owner == user.key()
    )]
    pub user_token_b: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(InitSpace)]
pub struct Pool {
    /// Mint of token A
    pub mint_a: Pubkey,
    /// Mint of token B
    pub mint_b: Pubkey,
    /// LP token mint address
    pub lp_mint: Pubkey,
    /// Pool's token A account
    pub pool_token_a: Pubkey,
    /// Pool's token B account
    pub pool_token_b: Pubkey,
    /// Fee in basis points (e.g., 30 = 0.3%)
    pub fee_bps: u16,
    /// Bump seed for pool PDA
    pub bump: u8,
    /// Bump seed for LP mint PDA
    pub lp_mint_bump: u8,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid fee: must be less than 10000 basis points")]
    InvalidFee,
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("Initial deposit too small")]
    DepositTooSmall,
    #[msg("Slippage tolerance exceeded")]
    SlippageExceeded,
}
