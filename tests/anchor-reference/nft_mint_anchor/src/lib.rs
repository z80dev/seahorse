use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    metadata::{
        create_master_edition_v3, create_metadata_accounts_v3, CreateMasterEditionV3,
        CreateMetadataAccountsV3, Metadata,
    },
    token::{mint_to, Mint, MintTo, Token, TokenAccount},
};
use mpl_token_metadata::types::DataV2;

declare_id!("NFTm1nt111111111111111111111111111111111111");

#[program]
pub mod nft_mint_anchor {
    use super::*;

    /// Mint a new NFT with metadata using Metaplex Token Metadata program
    /// Creates mint, metadata account, and master edition
    pub fn mint_nft(
        ctx: Context<MintNft>,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        require!(name.len() <= 32, ErrorCode::NameTooLong);
        require!(symbol.len() <= 10, ErrorCode::SymbolTooLong);
        require!(uri.len() <= 200, ErrorCode::UriTooLong);

        // Initialize mint config state
        let nft_config = &mut ctx.accounts.nft_config;
        nft_config.authority = ctx.accounts.authority.key();
        nft_config.mint = ctx.accounts.mint.key();
        nft_config.name = name.clone();
        nft_config.symbol = symbol.clone();
        nft_config.uri = uri.clone();
        nft_config.is_minted = false;
        nft_config.bump = ctx.bumps.nft_config;

        msg!("NFT config created for mint: {}", ctx.accounts.mint.key());
        msg!("Name: {}, Symbol: {}", name, symbol);

        // Mint 1 token to the token account
        let cpi_accounts = MintTo {
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.token_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        mint_to(cpi_ctx, 1)?;

        // Create metadata account
        let data = DataV2 {
            name,
            symbol,
            uri,
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        };

        let cpi_accounts = CreateMetadataAccountsV3 {
            metadata: ctx.accounts.metadata.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            mint_authority: ctx.accounts.authority.to_account_info(),
            payer: ctx.accounts.authority.to_account_info(),
            update_authority: ctx.accounts.authority.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        };
        let cpi_program = ctx.accounts.metadata_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        create_metadata_accounts_v3(cpi_ctx, data, true, true, None)?;

        // Create master edition account (makes it a true NFT with supply of 1)
        let cpi_accounts = CreateMasterEditionV3 {
            edition: ctx.accounts.master_edition.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            update_authority: ctx.accounts.authority.to_account_info(),
            mint_authority: ctx.accounts.authority.to_account_info(),
            payer: ctx.accounts.authority.to_account_info(),
            metadata: ctx.accounts.metadata.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        };
        let cpi_program = ctx.accounts.metadata_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        create_master_edition_v3(cpi_ctx, Some(0))?; // max_supply = 0 means unique 1/1 NFT

        // Update config to mark as minted
        let nft_config = &mut ctx.accounts.nft_config;
        nft_config.is_minted = true;

        msg!("NFT minted successfully!");

        Ok(())
    }

    /// Create NFT config without actually minting (for Seahorse parity testing)
    /// This creates just the config account that tracks NFT metadata
    pub fn create_nft_config(
        ctx: Context<CreateNftConfig>,
        nft_id: u64,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        require!(name.len() <= 32, ErrorCode::NameTooLong);
        require!(symbol.len() <= 10, ErrorCode::SymbolTooLong);
        require!(uri.len() <= 200, ErrorCode::UriTooLong);

        let nft_config = &mut ctx.accounts.nft_config;
        nft_config.authority = ctx.accounts.authority.key();
        nft_config.nft_id = nft_id;
        // Mint will be set when mark_as_minted is called
        nft_config.mint = ctx.accounts.authority.key(); // Placeholder
        nft_config.name = name.clone();
        nft_config.symbol = symbol.clone();
        nft_config.uri = uri.clone();
        nft_config.is_minted = false;
        nft_config.bump = ctx.bumps.nft_config;

        msg!("NFT config created with id: {}", nft_id);
        msg!("Name: {}, Symbol: {}", name, symbol);

        Ok(())
    }

    /// Update NFT metadata (before minting)
    pub fn update_nft_config(
        ctx: Context<UpdateNftConfig>,
        name: Option<String>,
        symbol: Option<String>,
        uri: Option<String>,
    ) -> Result<()> {
        let nft_config = &mut ctx.accounts.nft_config;

        require!(!nft_config.is_minted, ErrorCode::AlreadyMinted);

        if let Some(name) = name {
            require!(name.len() <= 32, ErrorCode::NameTooLong);
            nft_config.name = name;
        }

        if let Some(symbol) = symbol {
            require!(symbol.len() <= 10, ErrorCode::SymbolTooLong);
            nft_config.symbol = symbol;
        }

        if let Some(uri) = uri {
            require!(uri.len() <= 200, ErrorCode::UriTooLong);
            nft_config.uri = uri;
        }

        msg!("NFT config updated");

        Ok(())
    }

    /// Mark NFT as minted and record the mint address
    /// For parity with Seahorse which doesn't have Metaplex CPI
    pub fn mark_as_minted(ctx: Context<MarkAsMinted>) -> Result<()> {
        let nft_config = &mut ctx.accounts.nft_config;

        require!(!nft_config.is_minted, ErrorCode::AlreadyMinted);

        nft_config.mint = ctx.accounts.mint.key();
        nft_config.is_minted = true;

        msg!("NFT marked as minted with mint: {}", ctx.accounts.mint.key());

        Ok(())
    }

    /// Get NFT info (read-only demonstration)
    pub fn get_nft_info(ctx: Context<GetNftInfo>) -> Result<()> {
        let nft_config = &ctx.accounts.nft_config;

        msg!("NFT Config Info:");
        msg!("  Authority: {}", nft_config.authority);
        msg!("  NFT ID: {}", nft_config.nft_id);
        msg!("  Mint: {}", nft_config.mint);
        msg!("  Name: {}", nft_config.name);
        msg!("  Symbol: {}", nft_config.symbol);
        msg!("  URI: {}", nft_config.uri);
        msg!("  Is Minted: {}", nft_config.is_minted);

        Ok(())
    }
}

#[derive(Accounts)]
pub struct MintNft<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + NftConfig::INIT_SPACE,
        seeds = [b"nft_config", mint.key().as_ref()],
        bump
    )]
    pub nft_config: Account<'info, NftConfig>,

    #[account(
        init,
        payer = authority,
        mint::decimals = 0,
        mint::authority = authority,
        mint::freeze_authority = authority,
    )]
    pub mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        associated_token::mint = mint,
        associated_token::authority = authority,
    )]
    pub token_account: Account<'info, TokenAccount>,

    /// CHECK: Validated by Metaplex
    #[account(mut)]
    pub metadata: UncheckedAccount<'info>,

    /// CHECK: Validated by Metaplex
    #[account(mut)]
    pub master_edition: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub metadata_program: Program<'info, Metadata>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(nft_id: u64)]
pub struct CreateNftConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + NftConfig::INIT_SPACE,
        seeds = [b"nft_config", nft_id.to_le_bytes().as_ref()],
        bump
    )]
    pub nft_config: Account<'info, NftConfig>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateNftConfig<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"nft_config", nft_config.nft_id.to_le_bytes().as_ref()],
        bump = nft_config.bump,
        has_one = authority @ ErrorCode::Unauthorized
    )]
    pub nft_config: Account<'info, NftConfig>,
}

#[derive(Accounts)]
pub struct MarkAsMinted<'info> {
    pub authority: Signer<'info>,

    /// CHECK: The mint account to record (can be any existing mint)
    pub mint: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"nft_config", nft_config.nft_id.to_le_bytes().as_ref()],
        bump = nft_config.bump,
        has_one = authority @ ErrorCode::Unauthorized
    )]
    pub nft_config: Account<'info, NftConfig>,
}

#[derive(Accounts)]
pub struct GetNftInfo<'info> {
    #[account(
        seeds = [b"nft_config", nft_config.nft_id.to_le_bytes().as_ref()],
        bump = nft_config.bump
    )]
    pub nft_config: Account<'info, NftConfig>,
}

#[account]
#[derive(InitSpace)]
pub struct NftConfig {
    /// The authority who can update/mint
    pub authority: Pubkey,
    /// Unique NFT identifier (used for PDA derivation)
    pub nft_id: u64,
    /// The mint pubkey for this NFT
    pub mint: Pubkey,
    /// NFT name (max 32 chars)
    #[max_len(32)]
    pub name: String,
    /// NFT symbol (max 10 chars)
    #[max_len(10)]
    pub symbol: String,
    /// Metadata URI (max 200 chars)
    #[max_len(200)]
    pub uri: String,
    /// Whether the NFT has been minted
    pub is_minted: bool,
    /// Bump for PDA
    pub bump: u8,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Name too long (max 32 characters)")]
    NameTooLong,
    #[msg("Symbol too long (max 10 characters)")]
    SymbolTooLong,
    #[msg("URI too long (max 200 characters)")]
    UriTooLong,
    #[msg("NFT already minted")]
    AlreadyMinted,
    #[msg("Unauthorized")]
    Unauthorized,
}
