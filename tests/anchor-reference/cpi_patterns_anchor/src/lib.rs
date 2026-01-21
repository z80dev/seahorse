use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer, MintTo, Burn};

declare_id!("Cpi1Pattrn111111111111111111111111111111111");

/// CPI Patterns - Anchor Reference Program
/// Demonstrates various Cross-Program Invocation patterns:
/// 1. Basic CPI - Token transfer with user as signer
/// 2. CPI with PDA signer - PDA-signed token operations
/// 3. Multiple CPIs - Chained operations in sequence
/// 4. Error handling across CPI boundary

#[program]
pub mod cpi_patterns_anchor {
    use super::*;

    /// Initialize a CPI vault that can hold tokens and perform CPI operations
    /// Pattern: Account initialization with PDA
    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.authority = ctx.accounts.authority.key();
        vault.mint = ctx.accounts.mint.key();
        vault.bump = ctx.bumps.vault;
        vault.total_deposited = 0;
        vault.total_withdrawn = 0;
        vault.transfer_count = 0;

        msg!("Vault initialized: authority={}", vault.authority);
        Ok(())
    }

    /// Basic CPI - Transfer tokens from user to vault
    /// Pattern: User signs the transfer, basic CPI to token program
    pub fn basic_transfer_to_vault(ctx: Context<BasicTransfer>, amount: u64) -> Result<()> {
        require!(amount > 0, CpiPatternsError::AmountMustBePositive);

        // CPI to Token Program - user is the authority (signer)
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.vault_token_account.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, amount)?;

        // Update vault state
        let vault = &mut ctx.accounts.vault;
        vault.total_deposited += amount;
        vault.transfer_count += 1;

        msg!("Basic transfer: {} tokens deposited, total_deposited={}", amount, vault.total_deposited);
        Ok(())
    }

    /// CPI with PDA signer - Transfer tokens from vault back to user
    /// Pattern: PDA signs the transfer using signer seeds
    pub fn pda_signed_transfer_from_vault(ctx: Context<PdaSignedTransfer>, amount: u64) -> Result<()> {
        require!(amount > 0, CpiPatternsError::AmountMustBePositive);
        require!(
            ctx.accounts.vault.authority == ctx.accounts.authority.key(),
            CpiPatternsError::Unauthorized
        );
        require!(
            ctx.accounts.vault_token_account.amount >= amount,
            CpiPatternsError::InsufficientFunds
        );

        // Build signer seeds for vault PDA
        let authority_key = ctx.accounts.authority.key();
        let mint_key = ctx.accounts.mint.key();
        let bump = ctx.accounts.vault.bump;
        let signer_seeds: &[&[&[u8]]] = &[
            &[b"vault", authority_key.as_ref(), mint_key.as_ref(), &[bump]]
        ];

        // CPI to Token Program with PDA signer
        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault_token_account.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.vault.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(transfer_ctx, amount)?;

        // Update vault state
        let vault = &mut ctx.accounts.vault;
        vault.total_withdrawn += amount;
        vault.transfer_count += 1;

        msg!("PDA-signed transfer: {} tokens withdrawn, total_withdrawn={}", amount, vault.total_withdrawn);
        Ok(())
    }

    /// Multiple CPIs - Mint tokens and immediately transfer to another account
    /// Pattern: Chained CPI operations in sequence
    pub fn chained_mint_and_transfer(
        ctx: Context<ChainedOperations>,
        mint_amount: u64,
        transfer_amount: u64,
    ) -> Result<()> {
        require!(mint_amount > 0, CpiPatternsError::AmountMustBePositive);
        require!(transfer_amount <= mint_amount, CpiPatternsError::TransferExceedsMint);

        // Build signer seeds for mint authority PDA
        let authority_key = ctx.accounts.authority.key();
        let bump = ctx.accounts.mint_config.bump;
        let signer_seeds: &[&[&[u8]]] = &[
            &[b"mint_config", authority_key.as_ref(), &[bump]]
        ];

        // First CPI: Mint tokens to intermediate account
        let mint_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.intermediate_account.to_account_info(),
                authority: ctx.accounts.mint_config.to_account_info(),
            },
            signer_seeds,
        );
        token::mint_to(mint_ctx, mint_amount)?;

        // Second CPI: Transfer some tokens to final destination
        // Note: For this CPI, user is authority of intermediate_account
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.intermediate_account.to_account_info(),
                to: ctx.accounts.destination_account.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, transfer_amount)?;

        // Update state
        let mint_config = &mut ctx.accounts.mint_config;
        mint_config.total_minted += mint_amount;
        mint_config.operation_count += 1;

        msg!("Chained CPIs: minted {} tokens, transferred {} to destination", mint_amount, transfer_amount);
        Ok(())
    }

    /// Error handling across CPI - Demonstrate handling CPI errors gracefully
    /// Pattern: Validate conditions before CPI to provide better error messages
    pub fn validated_burn(ctx: Context<ValidatedBurn>, amount: u64) -> Result<()> {
        require!(amount > 0, CpiPatternsError::AmountMustBePositive);

        // Pre-validation before CPI (better UX than letting CPI fail)
        let available_balance = ctx.accounts.source_account.amount;
        require!(
            available_balance >= amount,
            CpiPatternsError::InsufficientBalance
        );

        // CPI to burn tokens
        let burn_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.mint.to_account_info(),
                from: ctx.accounts.source_account.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        );
        token::burn(burn_ctx, amount)?;

        msg!("Validated burn: {} tokens burned, remaining={}", amount, available_balance - amount);
        Ok(())
    }

    /// Initialize mint config - Create a mint authority PDA
    pub fn initialize_mint_config(ctx: Context<InitializeMintConfig>) -> Result<()> {
        let config = &mut ctx.accounts.mint_config;
        config.authority = ctx.accounts.authority.key();
        config.mint = ctx.accounts.mint.key();
        config.bump = ctx.bumps.mint_config;
        config.total_minted = 0;
        config.operation_count = 0;

        msg!("MintConfig initialized: authority={}", config.authority);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + CpiVault::INIT_SPACE,
        seeds = [b"vault", authority.key().as_ref(), mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, CpiVault>,

    #[account(
        init,
        payer = authority,
        token::mint = mint,
        token::authority = vault,
        seeds = [b"vault_token", authority.key().as_ref(), mint.key().as_ref()],
        bump
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BasicTransfer<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"vault", vault.authority.as_ref(), vault.mint.as_ref()],
        bump = vault.bump
    )]
    pub vault: Account<'info, CpiVault>,

    #[account(
        mut,
        constraint = user_token_account.owner == user.key() @ CpiPatternsError::TokenOwnerMismatch,
        constraint = user_token_account.mint == vault.mint @ CpiPatternsError::MintMismatch
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"vault_token", vault.authority.as_ref(), vault.mint.as_ref()],
        bump
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct PdaSignedTransfer<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"vault", authority.key().as_ref(), mint.key().as_ref()],
        bump = vault.bump
    )]
    pub vault: Account<'info, CpiVault>,

    #[account(
        mut,
        constraint = user_token_account.mint == vault.mint @ CpiPatternsError::MintMismatch
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"vault_token", authority.key().as_ref(), mint.key().as_ref()],
        bump
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct InitializeMintConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + MintConfig::INIT_SPACE,
        seeds = [b"mint_config", authority.key().as_ref()],
        bump
    )]
    pub mint_config: Account<'info, MintConfig>,

    #[account(mut)]
    pub mint: Account<'info, Mint>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ChainedOperations<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"mint_config", authority.key().as_ref()],
        bump = mint_config.bump,
        constraint = mint_config.authority == authority.key() @ CpiPatternsError::Unauthorized
    )]
    pub mint_config: Account<'info, MintConfig>,

    #[account(
        mut,
        constraint = mint.key() == mint_config.mint @ CpiPatternsError::MintMismatch
    )]
    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        constraint = intermediate_account.mint == mint.key() @ CpiPatternsError::MintMismatch,
        constraint = intermediate_account.owner == authority.key() @ CpiPatternsError::TokenOwnerMismatch
    )]
    pub intermediate_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = destination_account.mint == mint.key() @ CpiPatternsError::MintMismatch
    )]
    pub destination_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ValidatedBurn<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        constraint = source_account.owner == owner.key() @ CpiPatternsError::TokenOwnerMismatch,
        constraint = source_account.mint == mint.key() @ CpiPatternsError::MintMismatch
    )]
    pub source_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(InitSpace)]
pub struct CpiVault {
    /// Authority who controls this vault
    pub authority: Pubkey,
    /// Mint of tokens stored in this vault
    pub mint: Pubkey,
    /// Bump seed for PDA derivation
    pub bump: u8,
    /// Total tokens deposited into vault (cumulative)
    pub total_deposited: u64,
    /// Total tokens withdrawn from vault (cumulative)
    pub total_withdrawn: u64,
    /// Count of transfer operations
    pub transfer_count: u64,
}

#[account]
#[derive(InitSpace)]
pub struct MintConfig {
    /// Authority who controls minting
    pub authority: Pubkey,
    /// Mint this config controls
    pub mint: Pubkey,
    /// Bump seed for PDA derivation
    pub bump: u8,
    /// Total tokens minted
    pub total_minted: u64,
    /// Count of mint operations
    pub operation_count: u64,
}

#[error_code]
pub enum CpiPatternsError {
    #[msg("Amount must be greater than zero")]
    AmountMustBePositive,
    #[msg("Unauthorized: caller is not the authority")]
    Unauthorized,
    #[msg("Insufficient funds in account")]
    InsufficientFunds,
    #[msg("Insufficient balance for burn")]
    InsufficientBalance,
    #[msg("Transfer amount exceeds minted amount")]
    TransferExceedsMint,
    #[msg("Token account owner mismatch")]
    TokenOwnerMismatch,
    #[msg("Mint mismatch")]
    MintMismatch,
}
