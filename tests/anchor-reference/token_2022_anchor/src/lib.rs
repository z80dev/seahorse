//! Token-2022 program demonstrating transfer fee extension
//!
//! This Anchor program creates a Token-2022 mint with the transfer fee extension,
//! which allows charging a percentage fee on every token transfer.
//!
//! Features:
//! - Create a Token-2022 mint with transfer fee extension
//! - Mint tokens using Token-2022 program
//! - Transfer tokens (fees are automatically collected)
//! - Withdraw collected fees

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::{invoke, invoke_signed};
use anchor_spl::token_interface::{Mint, Token2022, TokenAccount};
use spl_token_2022::{
    extension::{
        transfer_fee::{instruction as transfer_fee_instruction, TransferFeeConfig},
        BaseStateWithExtensions, StateWithExtensions,
    },
    instruction as token_2022_instruction,
    state::Mint as SplMint,
};

declare_id!("T2Ex5VzVZYxLYtDmJq5DEmUKDEcPnP2zGxqHKbv2Tbn");

/// Maximum basis points (100%)
const MAX_FEE_BASIS_POINTS: u16 = 10_000;

#[program]
pub mod token_2022_anchor {
    use super::*;

    /// Initialize a Token-2022 mint with transfer fee extension
    /// transfer_fee_bps: Fee in basis points (1% = 100 bps)
    /// max_fee: Maximum fee to charge in token units
    pub fn create_transfer_fee_mint(
        ctx: Context<CreateTransferFeeMint>,
        decimals: u8,
        transfer_fee_bps: u16,
        max_fee: u64,
    ) -> Result<()> {
        require!(
            transfer_fee_bps <= MAX_FEE_BASIS_POINTS,
            ErrorCode::InvalidFee
        );

        // Calculate space needed for mint with transfer fee extension
        let space =
            spl_token_2022::extension::ExtensionType::try_calculate_account_len::<SplMint>(&[
                spl_token_2022::extension::ExtensionType::TransferFeeConfig,
            ])?;

        // Get rent for the mint account
        let rent = Rent::get()?;
        let lamports = rent.minimum_balance(space);

        // Create the account
        invoke(
            &anchor_lang::solana_program::system_instruction::create_account(
                ctx.accounts.authority.key,
                ctx.accounts.mint.key,
                lamports,
                space as u64,
                &spl_token_2022::ID,
            ),
            &[
                ctx.accounts.authority.to_account_info(),
                ctx.accounts.mint.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        // Initialize transfer fee config extension
        invoke(
            &transfer_fee_instruction::initialize_transfer_fee_config(
                &spl_token_2022::ID,
                ctx.accounts.mint.key,
                Some(&ctx.accounts.fee_config.key()), // Transfer fee config authority
                Some(&ctx.accounts.fee_config.key()), // Withdraw withheld authority
                transfer_fee_bps,
                max_fee,
            )?,
            &[ctx.accounts.mint.to_account_info()],
        )?;

        // Initialize mint
        invoke(
            &token_2022_instruction::initialize_mint(
                &spl_token_2022::ID,
                ctx.accounts.mint.key,
                &ctx.accounts.fee_config.key(),
                None, // No freeze authority
                decimals,
            )?,
            &[ctx.accounts.mint.to_account_info()],
        )?;

        // Store config
        let fee_config = &mut ctx.accounts.fee_config;
        fee_config.mint = ctx.accounts.mint.key();
        fee_config.authority = ctx.accounts.authority.key();
        fee_config.decimals = decimals;
        fee_config.transfer_fee_bps = transfer_fee_bps;
        fee_config.max_fee = max_fee;
        fee_config.bump = ctx.bumps.fee_config;

        msg!(
            "Created Token-2022 mint with transfer fee: {:?}",
            ctx.accounts.mint.key()
        );
        msg!("Transfer fee: {} bps, max: {}", transfer_fee_bps, max_fee);

        Ok(())
    }

    /// Mint tokens to a token account
    pub fn mint_tokens(ctx: Context<MintTokens>, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);

        let fee_config = &ctx.accounts.fee_config;
        let authority_key = ctx.accounts.authority.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"fee_config",
            authority_key.as_ref(),
            &[fee_config.bump],
        ]];

        anchor_spl::token_interface::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token_interface::MintTo {
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.destination.to_account_info(),
                    authority: ctx.accounts.fee_config.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )?;

        msg!("Minted {} tokens", amount);
        Ok(())
    }

    /// Read the transfer fee configuration from the mint
    pub fn get_transfer_fee_config(ctx: Context<GetTransferFeeConfig>) -> Result<()> {
        let mint_info = ctx.accounts.mint.to_account_info();
        let mint_data = mint_info.data.borrow();
        let mint_state = StateWithExtensions::<SplMint>::unpack(&mint_data)?;

        let extension = mint_state.get_extension::<TransferFeeConfig>()?;

        // Read current fee config
        let newer_fee = &extension.newer_transfer_fee;
        let fee_bps = u16::from(newer_fee.transfer_fee_basis_points);
        let max_fee = u64::from(newer_fee.maximum_fee);

        msg!("Transfer fee: {} basis points", fee_bps);
        msg!("Maximum fee: {}", max_fee);

        Ok(())
    }

    /// Withdraw collected fees from a token account to fee authority
    pub fn withdraw_withheld_fees(ctx: Context<WithdrawWithheldFees>) -> Result<()> {
        let fee_config = &ctx.accounts.fee_config;
        let authority_key = ctx.accounts.authority.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"fee_config",
            authority_key.as_ref(),
            &[fee_config.bump],
        ]];

        // Withdraw withheld tokens from the source account
        // Use empty signers array - the PDA authority is verified via invoke_signed seeds
        invoke_signed(
            &transfer_fee_instruction::withdraw_withheld_tokens_from_accounts(
                &spl_token_2022::ID,
                &ctx.accounts.mint.key(),
                &ctx.accounts.fee_destination.key(),
                &ctx.accounts.fee_config.key(),
                &[], // No multisig signers - authority is a PDA
                &[&ctx.accounts.source.key()],
            )?,
            &[
                ctx.accounts.mint.to_account_info(),
                ctx.accounts.fee_destination.to_account_info(),
                ctx.accounts.fee_config.to_account_info(),
                ctx.accounts.source.to_account_info(),
            ],
            signer_seeds,
        )?;

        msg!("Withdrew withheld fees");
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateTransferFeeMint<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + FeeConfig::INIT_SPACE,
        seeds = [b"fee_config", authority.key().as_ref()],
        bump
    )]
    pub fee_config: Account<'info, FeeConfig>,

    /// CHECK: Mint is initialized via CPI to Token-2022 program
    #[account(
        mut,
        seeds = [b"mint", authority.key().as_ref()],
        bump
    )]
    pub mint: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintTokens<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        seeds = [b"fee_config", authority.key().as_ref()],
        bump = fee_config.bump,
        has_one = authority @ ErrorCode::Unauthorized
    )]
    pub fee_config: Account<'info, FeeConfig>,

    #[account(
        mut,
        seeds = [b"mint", authority.key().as_ref()],
        bump
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        token::mint = mint,
        token::token_program = token_program
    )]
    pub destination: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token2022>,
}

#[derive(Accounts)]
pub struct GetTransferFeeConfig<'info> {
    pub authority: Signer<'info>,

    #[account(
        seeds = [b"fee_config", authority.key().as_ref()],
        bump = fee_config.bump
    )]
    pub fee_config: Account<'info, FeeConfig>,

    #[account(
        seeds = [b"mint", authority.key().as_ref()],
        bump
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    pub token_program: Program<'info, Token2022>,
}

#[derive(Accounts)]
pub struct WithdrawWithheldFees<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        seeds = [b"fee_config", authority.key().as_ref()],
        bump = fee_config.bump,
        has_one = authority @ ErrorCode::Unauthorized
    )]
    pub fee_config: Account<'info, FeeConfig>,

    #[account(
        mut,
        seeds = [b"mint", authority.key().as_ref()],
        bump
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    /// Token account with withheld fees
    #[account(
        mut,
        token::mint = mint,
        token::token_program = token_program
    )]
    pub source: InterfaceAccount<'info, TokenAccount>,

    /// Token account to receive withdrawn fees
    #[account(
        mut,
        token::mint = mint,
        token::token_program = token_program
    )]
    pub fee_destination: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token2022>,
}

#[account]
#[derive(InitSpace)]
pub struct FeeConfig {
    /// The Token-2022 mint address
    pub mint: Pubkey,
    /// The authority who controls the fee config
    pub authority: Pubkey,
    /// Token decimals
    pub decimals: u8,
    /// Transfer fee in basis points
    pub transfer_fee_bps: u16,
    /// Maximum transfer fee
    pub max_fee: u64,
    /// Bump for fee_config PDA
    pub bump: u8,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("Fee exceeds maximum allowed (10000 basis points)")]
    InvalidFee,
    #[msg("Unauthorized access")]
    Unauthorized,
}
