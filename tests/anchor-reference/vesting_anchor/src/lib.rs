use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("VESTngW9a1XxRgwWhmK7PNYWMFp4mW8q3xk2BgfCNih");

#[program]
pub mod vesting_anchor {
    use super::*;

    /// Create a new vesting schedule
    ///
    /// Args:
    /// - amount: Total tokens to vest
    /// - cliff_slots: Slots until cliff (0 for no cliff)
    /// - vesting_duration_slots: Slots for linear vesting after cliff
    pub fn create_vesting(
        ctx: Context<CreateVesting>,
        beneficiary: Pubkey,
        amount: u64,
        cliff_slots: u64,
        vesting_duration_slots: u64,
    ) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);
        require!(vesting_duration_slots > 0, ErrorCode::InvalidDuration);

        let clock = Clock::get()?;
        let current_slot = clock.slot;

        // Transfer tokens from authority to vault
        let cpi_accounts = Transfer {
            from: ctx.accounts.authority_token.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        );
        token::transfer(cpi_ctx, amount)?;

        // Initialize vesting account
        let vesting = &mut ctx.accounts.vesting;
        vesting.authority = ctx.accounts.authority.key();
        vesting.beneficiary = beneficiary;
        vesting.mint = ctx.accounts.mint.key();
        vesting.vault = ctx.accounts.vault.key();
        vesting.total_amount = amount;
        vesting.released_amount = 0;
        vesting.start_slot = current_slot;
        vesting.cliff_slot = current_slot.checked_add(cliff_slots).ok_or(ErrorCode::Overflow)?;
        vesting.end_slot = current_slot
            .checked_add(cliff_slots)
            .ok_or(ErrorCode::Overflow)?
            .checked_add(vesting_duration_slots)
            .ok_or(ErrorCode::Overflow)?;
        vesting.is_cancelled = false;
        vesting.bump = ctx.bumps.vesting;

        Ok(())
    }

    /// Claim vested tokens
    pub fn claim_vested(ctx: Context<ClaimVested>) -> Result<()> {
        let vesting = &ctx.accounts.vesting;

        require!(!vesting.is_cancelled, ErrorCode::VestingCancelled);

        let clock = Clock::get()?;
        let current_slot = clock.slot;

        // Calculate vested amount
        let vested_amount = calculate_vested_amount(
            vesting.total_amount,
            vesting.cliff_slot,
            vesting.end_slot,
            current_slot,
        )?;

        // Calculate releasable amount
        let releasable = vested_amount
            .checked_sub(vesting.released_amount)
            .ok_or(ErrorCode::Underflow)?;

        require!(releasable > 0, ErrorCode::NoTokensToRelease);

        // Transfer tokens using PDA signer
        let beneficiary_key = vesting.beneficiary;
        let mint_key = vesting.mint;
        let bump = vesting.bump;
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"vesting",
            beneficiary_key.as_ref(),
            mint_key.as_ref(),
            &[bump],
        ]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.beneficiary_token.to_account_info(),
            authority: ctx.accounts.vesting.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token::transfer(cpi_ctx, releasable)?;

        // Update released amount
        let vesting = &mut ctx.accounts.vesting;
        vesting.released_amount = vesting
            .released_amount
            .checked_add(releasable)
            .ok_or(ErrorCode::Overflow)?;

        Ok(())
    }

    /// Cancel vesting schedule
    /// Vested tokens go to beneficiary, unvested tokens return to authority
    pub fn cancel_vesting(ctx: Context<CancelVesting>) -> Result<()> {
        let vesting = &ctx.accounts.vesting;

        require!(!vesting.is_cancelled, ErrorCode::VestingAlreadyCancelled);

        let clock = Clock::get()?;
        let current_slot = clock.slot;

        // Calculate vested amount
        let vested_amount = calculate_vested_amount(
            vesting.total_amount,
            vesting.cliff_slot,
            vesting.end_slot,
            current_slot,
        )?;

        // Calculate amounts
        let unreleased_vested = vested_amount
            .checked_sub(vesting.released_amount)
            .ok_or(ErrorCode::Underflow)?;
        let unvested = vesting
            .total_amount
            .checked_sub(vested_amount)
            .ok_or(ErrorCode::Underflow)?;

        // Prepare signer seeds
        let beneficiary_key = vesting.beneficiary;
        let mint_key = vesting.mint;
        let bump = vesting.bump;
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"vesting",
            beneficiary_key.as_ref(),
            mint_key.as_ref(),
            &[bump],
        ]];

        // Transfer vested tokens to beneficiary (if any)
        if unreleased_vested > 0 {
            let cpi_accounts = Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.beneficiary_token.to_account_info(),
                authority: ctx.accounts.vesting.to_account_info(),
            };
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                signer_seeds,
            );
            token::transfer(cpi_ctx, unreleased_vested)?;
        }

        // Transfer unvested tokens back to authority (if any)
        if unvested > 0 {
            let cpi_accounts = Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.authority_token.to_account_info(),
                authority: ctx.accounts.vesting.to_account_info(),
            };
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                signer_seeds,
            );
            token::transfer(cpi_ctx, unvested)?;
        }

        // Mark as cancelled
        let vesting = &mut ctx.accounts.vesting;
        vesting.is_cancelled = true;
        vesting.released_amount = vested_amount;

        Ok(())
    }
}

/// Calculate vested amount based on current time
fn calculate_vested_amount(
    total_amount: u64,
    cliff_slot: u64,
    end_slot: u64,
    current_slot: u64,
) -> Result<u64> {
    if current_slot < cliff_slot {
        // Before cliff: nothing vested
        Ok(0)
    } else if current_slot >= end_slot {
        // After end: everything vested
        Ok(total_amount)
    } else {
        // During vesting period: linear vesting
        // vested = total * (current - cliff) / (end - cliff)
        let vesting_period = end_slot.checked_sub(cliff_slot).ok_or(ErrorCode::Underflow)?;
        let elapsed = current_slot.checked_sub(cliff_slot).ok_or(ErrorCode::Underflow)?;

        // Use u128 for intermediate calculation to prevent overflow
        let vested = (total_amount as u128)
            .checked_mul(elapsed as u128)
            .ok_or(ErrorCode::Overflow)?
            .checked_div(vesting_period as u128)
            .ok_or(ErrorCode::DivisionByZero)? as u64;

        Ok(vested)
    }
}

#[derive(Accounts)]
#[instruction(beneficiary: Pubkey)]
pub struct CreateVesting<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    pub mint: Account<'info, Mint>,

    /// Authority's token account (source of tokens)
    #[account(
        mut,
        constraint = authority_token.mint == mint.key(),
        constraint = authority_token.owner == authority.key()
    )]
    pub authority_token: Account<'info, TokenAccount>,

    /// Vesting account PDA
    #[account(
        init,
        payer = authority,
        space = 8 + VestingAccount::INIT_SPACE,
        seeds = [b"vesting", beneficiary.as_ref(), mint.key().as_ref()],
        bump
    )]
    pub vesting: Account<'info, VestingAccount>,

    /// Vault to hold vested tokens
    #[account(
        init,
        payer = authority,
        token::mint = mint,
        token::authority = vesting,
        seeds = [b"vesting_vault", beneficiary.as_ref(), mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimVested<'info> {
    pub beneficiary: Signer<'info>,

    #[account(
        mut,
        seeds = [b"vesting", vesting.beneficiary.as_ref(), vesting.mint.as_ref()],
        bump = vesting.bump,
        constraint = vesting.beneficiary == beneficiary.key() @ ErrorCode::Unauthorized
    )]
    pub vesting: Account<'info, VestingAccount>,

    /// Vault holding vested tokens
    #[account(
        mut,
        seeds = [b"vesting_vault", vesting.beneficiary.as_ref(), vesting.mint.as_ref()],
        bump
    )]
    pub vault: Account<'info, TokenAccount>,

    /// Beneficiary's token account
    #[account(
        mut,
        constraint = beneficiary_token.mint == vesting.mint,
        constraint = beneficiary_token.owner == beneficiary.key()
    )]
    pub beneficiary_token: Account<'info, TokenAccount>,

    pub mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CancelVesting<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"vesting", vesting.beneficiary.as_ref(), vesting.mint.as_ref()],
        bump = vesting.bump,
        constraint = vesting.authority == authority.key() @ ErrorCode::Unauthorized
    )]
    pub vesting: Account<'info, VestingAccount>,

    /// Vault holding vested tokens
    #[account(
        mut,
        seeds = [b"vesting_vault", vesting.beneficiary.as_ref(), vesting.mint.as_ref()],
        bump
    )]
    pub vault: Account<'info, TokenAccount>,

    /// Authority's token account (for receiving unvested tokens)
    #[account(
        mut,
        constraint = authority_token.mint == vesting.mint,
        constraint = authority_token.owner == authority.key()
    )]
    pub authority_token: Account<'info, TokenAccount>,

    /// Beneficiary's token account (for receiving vested tokens)
    #[account(
        mut,
        constraint = beneficiary_token.mint == vesting.mint
    )]
    pub beneficiary_token: Account<'info, TokenAccount>,

    pub mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(InitSpace)]
pub struct VestingAccount {
    /// Authority who can cancel the vesting
    pub authority: Pubkey,
    /// Beneficiary who receives vested tokens
    pub beneficiary: Pubkey,
    /// Mint of the token being vested
    pub mint: Pubkey,
    /// Vault holding tokens
    pub vault: Pubkey,
    /// Total tokens to be vested
    pub total_amount: u64,
    /// Tokens already released
    pub released_amount: u64,
    /// Vesting start slot
    pub start_slot: u64,
    /// Cliff end slot (tokens unlock at this point)
    pub cliff_slot: u64,
    /// Full vesting end slot
    pub end_slot: u64,
    /// Is vesting cancelled
    pub is_cancelled: bool,
    /// Bump seed for PDA
    pub bump: u8,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("Vesting duration must be greater than zero")]
    InvalidDuration,
    #[msg("Vesting has been cancelled")]
    VestingCancelled,
    #[msg("Vesting is already cancelled")]
    VestingAlreadyCancelled,
    #[msg("No tokens available to release")]
    NoTokensToRelease,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Arithmetic underflow")]
    Underflow,
    #[msg("Division by zero")]
    DivisionByZero,
    #[msg("Unauthorized access")]
    Unauthorized,
}
