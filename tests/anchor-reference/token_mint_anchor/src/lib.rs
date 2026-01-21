use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, MintTo, Token, TokenAccount};

declare_id!("MiNT5VzVZYxLYtDmJq5DEmUKDEcPnP2zGxqHKbv2PgZ");

#[program]
pub mod token_mint_anchor {
    use super::*;

    /// Create a new token mint with mint_config PDA as the mint authority
    /// The program controls all minting through the mint_config PDA
    pub fn create_mint(ctx: Context<CreateMint>, decimals: u8) -> Result<()> {
        let mint_config = &mut ctx.accounts.mint_config;
        mint_config.mint = ctx.accounts.mint.key();
        mint_config.authority = ctx.accounts.authority.key();
        mint_config.decimals = decimals;
        mint_config.total_minted = 0;
        mint_config.total_burned = 0;
        mint_config.bump = ctx.bumps.mint_config;

        msg!("Created mint: {:?}", ctx.accounts.mint.key());
        msg!("Decimals: {}", decimals);

        Ok(())
    }

    /// Mint new tokens to a destination token account
    /// Only the authority can mint tokens
    pub fn mint_tokens(ctx: Context<MintTokens>, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);

        let mint_config = &ctx.accounts.mint_config;

        // PDA signer seeds for mint_config (the mint authority)
        let authority_key = ctx.accounts.authority.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"mint_config",
            authority_key.as_ref(),
            &[mint_config.bump],
        ]];

        // CPI to mint tokens using mint_config PDA as authority
        let cpi_accounts = MintTo {
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.destination.to_account_info(),
            authority: ctx.accounts.mint_config.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        token::mint_to(cpi_ctx, amount)?;

        // Update config
        let mint_config = &mut ctx.accounts.mint_config;
        mint_config.total_minted = mint_config
            .total_minted
            .checked_add(amount)
            .ok_or(ErrorCode::Overflow)?;

        msg!("Minted {} tokens to {:?}", amount, ctx.accounts.destination.key());

        Ok(())
    }

    /// Burn tokens from a token account
    /// The token account owner must sign
    pub fn burn_tokens(ctx: Context<BurnTokens>, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);
        require!(
            ctx.accounts.source.amount >= amount,
            ErrorCode::InsufficientFunds
        );

        // CPI to burn tokens
        let cpi_accounts = Burn {
            mint: ctx.accounts.mint.to_account_info(),
            from: ctx.accounts.source.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::burn(cpi_ctx, amount)?;

        // Update config
        let mint_config = &mut ctx.accounts.mint_config;
        mint_config.total_burned = mint_config
            .total_burned
            .checked_add(amount)
            .ok_or(ErrorCode::Overflow)?;

        msg!("Burned {} tokens from {:?}", amount, ctx.accounts.source.key());

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(decimals: u8)]
pub struct CreateMint<'info> {
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

    #[account(
        init,
        payer = authority,
        mint::decimals = decimals,
        mint::authority = mint_config,
        seeds = [b"mint", authority.key().as_ref()],
        bump
    )]
    pub mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintTokens<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"mint_config", authority.key().as_ref()],
        bump = mint_config.bump,
        has_one = authority @ ErrorCode::Unauthorized
    )]
    pub mint_config: Account<'info, MintConfig>,

    #[account(
        mut,
        seeds = [b"mint", authority.key().as_ref()],
        bump
    )]
    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        token::mint = mint
    )]
    pub destination: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct BurnTokens<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: Authority is only used to derive mint_config PDA
    pub authority: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"mint_config", authority.key().as_ref()],
        bump = mint_config.bump
    )]
    pub mint_config: Account<'info, MintConfig>,

    #[account(
        mut,
        seeds = [b"mint", authority.key().as_ref()],
        bump
    )]
    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = owner
    )]
    pub source: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(InitSpace)]
pub struct MintConfig {
    /// The mint PDA
    pub mint: Pubkey,
    /// The authority who can mint tokens
    pub authority: Pubkey,
    /// Token decimals
    pub decimals: u8,
    /// Total tokens minted
    pub total_minted: u64,
    /// Total tokens burned
    pub total_burned: u64,
    /// Bump for mint_config PDA
    pub bump: u8,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("Insufficient funds")]
    InsufficientFunds,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Unauthorized access")]
    Unauthorized,
}
