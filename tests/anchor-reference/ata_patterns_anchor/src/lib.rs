use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("ATAPtrns1111111111111111111111111111111111AA");

#[program]
pub mod ata_patterns_anchor {
    use super::*;

    /// Create a PDA token account for a user
    pub fn create_user_token_account(ctx: Context<CreateUserTokenAccount>) -> Result<()> {
        msg!("Created token account for user: {}", ctx.accounts.user.key());
        msg!("Token account address: {}", ctx.accounts.user_token_account.key());
        Ok(())
    }

    /// Transfer tokens from sender's account to recipient's account
    /// Both accounts must already exist
    pub fn transfer_tokens(ctx: Context<TransferTokens>, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);

        let cpi_accounts = Transfer {
            from: ctx.accounts.sender_token_account.to_account_info(),
            to: ctx.accounts.recipient_token_account.to_account_info(),
            authority: ctx.accounts.sender.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        msg!("Transferred {} tokens", amount);
        Ok(())
    }

    /// Register a user in the system and create their token account
    pub fn register_user(ctx: Context<RegisterUser>) -> Result<()> {
        let user_account = &mut ctx.accounts.user_account;
        user_account.owner = ctx.accounts.user.key();
        user_account.mint = ctx.accounts.mint.key();
        user_account.token_account = ctx.accounts.user_token_account.key();
        user_account.bump = ctx.bumps.user_account;
        user_account.is_registered = true;

        msg!("Registered user: {}", ctx.accounts.user.key());
        msg!("User token account: {}", ctx.accounts.user_token_account.key());
        Ok(())
    }

    /// Initialize a treasury with its token account
    pub fn initialize_treasury(ctx: Context<InitializeTreasury>) -> Result<()> {
        let treasury = &mut ctx.accounts.treasury;
        treasury.admin = ctx.accounts.admin.key();
        treasury.mint = ctx.accounts.mint.key();
        treasury.treasury_token_account = ctx.accounts.treasury_token_account.key();
        treasury.bump = ctx.bumps.treasury;

        msg!("Initialized treasury with token account: {}", ctx.accounts.treasury_token_account.key());
        Ok(())
    }

    /// Airdrop tokens to a registered user's token account
    /// PDA authority signs the transfer
    pub fn airdrop_to_user(ctx: Context<AirdropToUser>, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);
        require!(
            ctx.accounts.user_account.is_registered,
            ErrorCode::UserNotRegistered
        );

        // Transfer from treasury to user's token account using PDA signer
        let admin_key = ctx.accounts.admin.key();
        let mint_key = ctx.accounts.mint.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"treasury",
            admin_key.as_ref(),
            mint_key.as_ref(),
            &[ctx.accounts.treasury.bump],
        ]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.treasury_token_account.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.treasury.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        token::transfer(cpi_ctx, amount)?;

        msg!("Airdropped {} tokens to user", amount);
        Ok(())
    }
}

// Note: CreateAndTransfer struct removed - Seahorse doesn't support UncheckedAccount in seeds

#[derive(Accounts)]
pub struct CreateUserTokenAccount<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(
        init,
        payer = user,
        seeds = [b"user_token", user.key().as_ref(), mint.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = user
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TransferTokens<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [b"user_token", sender.key().as_ref(), mint.key().as_ref()],
        bump
    )]
    pub sender_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub recipient_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct RegisterUser<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(
        init,
        payer = user,
        space = 8 + UserAccount::INIT_SPACE,
        seeds = [b"user_account", user.key().as_ref(), mint.key().as_ref()],
        bump
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(
        init,
        payer = user,
        seeds = [b"user_token", user.key().as_ref(), mint.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = user
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeTreasury<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(
        init,
        payer = admin,
        space = 8 + Treasury::INIT_SPACE,
        seeds = [b"treasury", admin.key().as_ref(), mint.key().as_ref()],
        bump
    )]
    pub treasury: Account<'info, Treasury>,

    #[account(
        init,
        payer = admin,
        seeds = [b"treasury_token", admin.key().as_ref(), mint.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = treasury
    )]
    pub treasury_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AirdropToUser<'info> {
    pub admin: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(
        seeds = [b"treasury", admin.key().as_ref(), mint.key().as_ref()],
        bump = treasury.bump
    )]
    pub treasury: Account<'info, Treasury>,

    #[account(
        mut,
        seeds = [b"treasury_token", admin.key().as_ref(), mint.key().as_ref()],
        bump
    )]
    pub treasury_token_account: Account<'info, TokenAccount>,

    #[account(
        seeds = [b"user_account", user_account.owner.as_ref(), mint.key().as_ref()],
        bump = user_account.bump
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(
        mut,
        seeds = [b"user_token", user_account.owner.as_ref(), mint.key().as_ref()],
        bump
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(InitSpace)]
pub struct UserAccount {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub token_account: Pubkey,
    pub bump: u8,
    pub is_registered: bool,
}

#[account]
#[derive(InitSpace)]
pub struct Treasury {
    pub admin: Pubkey,
    pub mint: Pubkey,
    pub treasury_token_account: Pubkey,
    pub bump: u8,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("User is not registered")]
    UserNotRegistered,
}
