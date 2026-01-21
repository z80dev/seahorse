use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    metadata::{
        create_master_edition_v3, create_metadata_accounts_v3, set_and_verify_collection,
        unverify_collection, CreateMasterEditionV3, CreateMetadataAccountsV3, Metadata,
        SetAndVerifyCollection, UnverifyCollection,
    },
    token::{mint_to, Mint, MintTo, Token, TokenAccount},
};
use mpl_token_metadata::types::{Collection as MplCollection, DataV2};

declare_id!("NFTCoL1ect1on111111111111111111111111111111");

#[program]
pub mod nft_collection_anchor {
    use super::*;

    /// Create a new collection configuration
    /// This is the parity-compatible instruction that matches Seahorse
    pub fn create_collection(
        ctx: Context<CreateCollection>,
        collection_id: u64,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        require!(name.len() <= 32, ErrorCode::NameTooLong);
        require!(symbol.len() <= 10, ErrorCode::SymbolTooLong);
        require!(uri.len() <= 200, ErrorCode::UriTooLong);

        let collection = &mut ctx.accounts.collection;
        collection.authority = ctx.accounts.authority.key();
        collection.collection_id = collection_id;
        collection.collection_mint = ctx.accounts.authority.key(); // Placeholder
        collection.name = name.clone();
        collection.symbol = symbol.clone();
        collection.uri = uri.clone();
        collection.total_nfts = 0;
        collection.verified_nfts = 0;
        collection.is_finalized = false;
        collection.bump = ctx.bumps.collection;

        msg!("Collection created with id: {}", collection_id);
        msg!("Name: {}, Symbol: {}", name, symbol);

        Ok(())
    }

    /// Update collection metadata (before finalization)
    pub fn update_collection(
        ctx: Context<UpdateCollection>,
        name: Option<String>,
        symbol: Option<String>,
        uri: Option<String>,
    ) -> Result<()> {
        let collection = &mut ctx.accounts.collection;

        require!(!collection.is_finalized, ErrorCode::CollectionFinalized);

        if let Some(name) = name {
            require!(name.len() <= 32, ErrorCode::NameTooLong);
            collection.name = name;
        }

        if let Some(symbol) = symbol {
            require!(symbol.len() <= 10, ErrorCode::SymbolTooLong);
            collection.symbol = symbol;
        }

        if let Some(uri) = uri {
            require!(uri.len() <= 200, ErrorCode::UriTooLong);
            collection.uri = uri;
        }

        msg!("Collection updated");

        Ok(())
    }

    /// Finalize the collection, preventing further metadata changes
    pub fn finalize_collection(ctx: Context<FinalizeCollection>) -> Result<()> {
        let collection = &mut ctx.accounts.collection;

        require!(!collection.is_finalized, ErrorCode::CollectionFinalized);

        collection.is_finalized = true;

        msg!("Collection finalized");

        Ok(())
    }

    /// Set the collection mint address
    pub fn set_collection_mint(ctx: Context<SetCollectionMint>) -> Result<()> {
        let collection = &mut ctx.accounts.collection;
        collection.collection_mint = ctx.accounts.collection_mint.key();

        msg!("Collection mint set to: {}", ctx.accounts.collection_mint.key());

        Ok(())
    }

    /// Add an NFT to the collection (register as member)
    pub fn add_nft_to_collection(ctx: Context<AddNftToCollection>) -> Result<()> {
        let member = &mut ctx.accounts.member;
        member.collection = ctx.accounts.collection.key();
        member.nft_mint = ctx.accounts.nft_mint.key();
        member.is_verified = false;
        member.bump = ctx.bumps.member;

        let collection = &mut ctx.accounts.collection;
        collection.total_nfts = collection.total_nfts.checked_add(1).unwrap();

        msg!("NFT added to collection: {}", ctx.accounts.nft_mint.key());

        Ok(())
    }

    /// Verify an NFT's membership in the collection
    pub fn verify_nft(ctx: Context<VerifyNft>) -> Result<()> {
        let member = &mut ctx.accounts.member;

        require!(
            member.collection == ctx.accounts.collection.key(),
            ErrorCode::NftNotInCollection
        );
        require!(!member.is_verified, ErrorCode::NftAlreadyVerified);

        member.is_verified = true;

        let collection = &mut ctx.accounts.collection;
        collection.verified_nfts = collection.verified_nfts.checked_add(1).unwrap();

        msg!("NFT verified in collection");

        Ok(())
    }

    /// Unverify an NFT from the collection
    pub fn unverify_nft(ctx: Context<UnverifyNft>) -> Result<()> {
        let member = &mut ctx.accounts.member;

        require!(
            member.collection == ctx.accounts.collection.key(),
            ErrorCode::NftNotInCollection
        );
        require!(member.is_verified, ErrorCode::NftNotVerified);

        member.is_verified = false;

        let collection = &mut ctx.accounts.collection;
        collection.verified_nfts = collection.verified_nfts.checked_sub(1).unwrap();

        msg!("NFT unverified from collection");

        Ok(())
    }

    /// Get collection info (read-only)
    pub fn get_collection_info(ctx: Context<GetCollectionInfo>) -> Result<()> {
        let collection = &ctx.accounts.collection;

        msg!("Collection Info:");
        msg!("  Authority: {}", collection.authority);
        msg!("  Collection Mint: {}", collection.collection_mint);
        msg!("  Name: {}", collection.name);
        msg!("  Symbol: {}", collection.symbol);
        msg!("  URI: {}", collection.uri);
        msg!("  Total NFTs: {}", collection.total_nfts);
        msg!("  Verified NFTs: {}", collection.verified_nfts);
        msg!("  Is Finalized: {}", collection.is_finalized);

        Ok(())
    }

    /// Get member info (read-only)
    pub fn get_member_info(ctx: Context<GetMemberInfo>) -> Result<()> {
        let member = &ctx.accounts.member;

        msg!("Member Info:");
        msg!("  Collection: {}", member.collection);
        msg!("  NFT Mint: {}", member.nft_mint);
        msg!("  Is Verified: {}", member.is_verified);

        Ok(())
    }

    // =========================================================================
    // Full Metaplex Implementation (for actual NFT collection functionality)
    // =========================================================================

    /// Create a collection NFT with Metaplex metadata
    /// This creates an actual NFT that serves as the collection identifier
    pub fn create_collection_nft(
        ctx: Context<CreateCollectionNft>,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        require!(name.len() <= 32, ErrorCode::NameTooLong);
        require!(symbol.len() <= 10, ErrorCode::SymbolTooLong);
        require!(uri.len() <= 200, ErrorCode::UriTooLong);

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
            name: name.clone(),
            symbol: symbol.clone(),
            uri: uri.clone(),
            seller_fee_basis_points: 0,
            creators: None,
            collection: None, // Collection NFT has no parent collection
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

        // Create master edition (makes it a true NFT)
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
        create_master_edition_v3(cpi_ctx, Some(0))?;

        msg!("Collection NFT created: {}", ctx.accounts.mint.key());
        msg!("Name: {}, Symbol: {}", name, symbol);

        Ok(())
    }

    /// Mint an NFT and set it as part of a collection
    /// Creates an NFT with the collection field set to the collection mint
    pub fn mint_to_collection(
        ctx: Context<MintToCollection>,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        require!(name.len() <= 32, ErrorCode::NameTooLong);
        require!(symbol.len() <= 10, ErrorCode::SymbolTooLong);
        require!(uri.len() <= 200, ErrorCode::UriTooLong);

        // Mint 1 token to the token account
        let cpi_accounts = MintTo {
            mint: ctx.accounts.nft_mint.to_account_info(),
            to: ctx.accounts.token_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        mint_to(cpi_ctx, 1)?;

        // Create metadata with collection reference (unverified initially)
        let data = DataV2 {
            name: name.clone(),
            symbol: symbol.clone(),
            uri: uri.clone(),
            seller_fee_basis_points: 0,
            creators: None,
            collection: Some(MplCollection {
                verified: false,
                key: ctx.accounts.collection_mint.key(),
            }),
            uses: None,
        };

        let cpi_accounts = CreateMetadataAccountsV3 {
            metadata: ctx.accounts.nft_metadata.to_account_info(),
            mint: ctx.accounts.nft_mint.to_account_info(),
            mint_authority: ctx.accounts.authority.to_account_info(),
            payer: ctx.accounts.authority.to_account_info(),
            update_authority: ctx.accounts.authority.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        };
        let cpi_program = ctx.accounts.metadata_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        create_metadata_accounts_v3(cpi_ctx, data, true, true, None)?;

        // Create master edition
        let cpi_accounts = CreateMasterEditionV3 {
            edition: ctx.accounts.nft_master_edition.to_account_info(),
            mint: ctx.accounts.nft_mint.to_account_info(),
            update_authority: ctx.accounts.authority.to_account_info(),
            mint_authority: ctx.accounts.authority.to_account_info(),
            payer: ctx.accounts.authority.to_account_info(),
            metadata: ctx.accounts.nft_metadata.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        };
        let cpi_program = ctx.accounts.metadata_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        create_master_edition_v3(cpi_ctx, Some(0))?;

        msg!("NFT minted to collection: {}", ctx.accounts.nft_mint.key());
        msg!("Collection: {}", ctx.accounts.collection_mint.key());

        Ok(())
    }

    /// Verify an NFT's collection membership via Metaplex CPI
    /// Requires the collection authority to sign
    pub fn verify_collection_metaplex(ctx: Context<VerifyCollectionMetaplex>) -> Result<()> {
        let cpi_accounts = SetAndVerifyCollection {
            metadata: ctx.accounts.nft_metadata.to_account_info(),
            collection_authority: ctx.accounts.collection_authority.to_account_info(),
            payer: ctx.accounts.payer.to_account_info(),
            update_authority: ctx.accounts.update_authority.to_account_info(),
            collection_mint: ctx.accounts.collection_mint.to_account_info(),
            collection_metadata: ctx.accounts.collection_metadata.to_account_info(),
            collection_master_edition: ctx.accounts.collection_master_edition.to_account_info(),
        };
        let cpi_program = ctx.accounts.metadata_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        set_and_verify_collection(cpi_ctx, None)?;

        msg!("NFT collection verified via Metaplex");

        Ok(())
    }

    /// Unverify an NFT's collection membership via Metaplex CPI
    pub fn unverify_collection_metaplex(ctx: Context<UnverifyCollectionMetaplex>) -> Result<()> {
        let cpi_accounts = UnverifyCollection {
            metadata: ctx.accounts.nft_metadata.to_account_info(),
            collection_authority: ctx.accounts.collection_authority.to_account_info(),
            collection_mint: ctx.accounts.collection_mint.to_account_info(),
            collection: ctx.accounts.collection_metadata.to_account_info(),
            collection_master_edition_account: ctx.accounts.collection_master_edition.to_account_info(),
        };
        let cpi_program = ctx.accounts.metadata_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        unverify_collection(cpi_ctx, None)?;

        msg!("NFT collection unverified via Metaplex");

        Ok(())
    }
}

// =============================================================================
// ACCOUNT CONTEXTS
// =============================================================================

#[derive(Accounts)]
#[instruction(collection_id: u64)]
pub struct CreateCollection<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + Collection::INIT_SPACE,
        seeds = [b"collection", collection_id.to_le_bytes().as_ref()],
        bump
    )]
    pub collection: Account<'info, Collection>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateCollection<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"collection", collection.collection_id.to_le_bytes().as_ref()],
        bump = collection.bump,
        has_one = authority @ ErrorCode::Unauthorized
    )]
    pub collection: Account<'info, Collection>,
}

#[derive(Accounts)]
pub struct FinalizeCollection<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"collection", collection.collection_id.to_le_bytes().as_ref()],
        bump = collection.bump,
        has_one = authority @ ErrorCode::Unauthorized
    )]
    pub collection: Account<'info, Collection>,
}

#[derive(Accounts)]
pub struct SetCollectionMint<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"collection", collection.collection_id.to_le_bytes().as_ref()],
        bump = collection.bump,
        has_one = authority @ ErrorCode::Unauthorized
    )]
    pub collection: Account<'info, Collection>,

    pub collection_mint: Account<'info, Mint>,
}

#[derive(Accounts)]
pub struct AddNftToCollection<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"collection", collection.collection_id.to_le_bytes().as_ref()],
        bump = collection.bump,
        has_one = authority @ ErrorCode::Unauthorized
    )]
    pub collection: Account<'info, Collection>,

    pub nft_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        space = 8 + CollectionMember::INIT_SPACE,
        seeds = [b"member", collection.key().as_ref(), nft_mint.key().as_ref()],
        bump
    )]
    pub member: Account<'info, CollectionMember>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct VerifyNft<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"collection", collection.collection_id.to_le_bytes().as_ref()],
        bump = collection.bump,
        has_one = authority @ ErrorCode::Unauthorized
    )]
    pub collection: Account<'info, Collection>,

    #[account(
        mut,
        seeds = [b"member", collection.key().as_ref(), member.nft_mint.as_ref()],
        bump = member.bump
    )]
    pub member: Account<'info, CollectionMember>,
}

#[derive(Accounts)]
pub struct UnverifyNft<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"collection", collection.collection_id.to_le_bytes().as_ref()],
        bump = collection.bump,
        has_one = authority @ ErrorCode::Unauthorized
    )]
    pub collection: Account<'info, Collection>,

    #[account(
        mut,
        seeds = [b"member", collection.key().as_ref(), member.nft_mint.as_ref()],
        bump = member.bump
    )]
    pub member: Account<'info, CollectionMember>,
}

#[derive(Accounts)]
pub struct GetCollectionInfo<'info> {
    #[account(
        seeds = [b"collection", collection.collection_id.to_le_bytes().as_ref()],
        bump = collection.bump
    )]
    pub collection: Account<'info, Collection>,
}

#[derive(Accounts)]
pub struct GetMemberInfo<'info> {
    #[account(
        seeds = [b"member", member.collection.as_ref(), member.nft_mint.as_ref()],
        bump = member.bump
    )]
    pub member: Account<'info, CollectionMember>,
}

// Metaplex-specific account contexts
#[derive(Accounts)]
pub struct CreateCollectionNft<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

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
pub struct MintToCollection<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    /// The collection mint (must have metadata)
    pub collection_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        mint::decimals = 0,
        mint::authority = authority,
        mint::freeze_authority = authority,
    )]
    pub nft_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        associated_token::mint = nft_mint,
        associated_token::authority = authority,
    )]
    pub token_account: Account<'info, TokenAccount>,

    /// CHECK: Validated by Metaplex
    #[account(mut)]
    pub nft_metadata: UncheckedAccount<'info>,

    /// CHECK: Validated by Metaplex
    #[account(mut)]
    pub nft_master_edition: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub metadata_program: Program<'info, Metadata>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct VerifyCollectionMetaplex<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub collection_authority: Signer<'info>,

    /// CHECK: Validated by Metaplex
    pub update_authority: UncheckedAccount<'info>,

    pub collection_mint: Account<'info, Mint>,

    /// CHECK: Validated by Metaplex
    #[account(mut)]
    pub nft_metadata: UncheckedAccount<'info>,

    /// CHECK: Validated by Metaplex
    pub collection_metadata: UncheckedAccount<'info>,

    /// CHECK: Validated by Metaplex
    pub collection_master_edition: UncheckedAccount<'info>,

    pub metadata_program: Program<'info, Metadata>,
}

#[derive(Accounts)]
pub struct UnverifyCollectionMetaplex<'info> {
    pub collection_authority: Signer<'info>,

    pub collection_mint: Account<'info, Mint>,

    /// CHECK: Validated by Metaplex
    #[account(mut)]
    pub nft_metadata: UncheckedAccount<'info>,

    /// CHECK: Validated by Metaplex
    pub collection_metadata: UncheckedAccount<'info>,

    /// CHECK: Validated by Metaplex
    pub collection_master_edition: UncheckedAccount<'info>,

    pub metadata_program: Program<'info, Metadata>,
}

// =============================================================================
// ACCOUNT STRUCTURES
// =============================================================================

#[account]
#[derive(InitSpace)]
pub struct Collection {
    /// The authority who can manage the collection
    pub authority: Pubkey,
    /// Unique collection identifier (used for PDA derivation)
    pub collection_id: u64,
    /// The collection mint pubkey
    pub collection_mint: Pubkey,
    /// Collection name (max 32 chars)
    #[max_len(32)]
    pub name: String,
    /// Collection symbol (max 10 chars)
    #[max_len(10)]
    pub symbol: String,
    /// Metadata URI (max 200 chars)
    #[max_len(200)]
    pub uri: String,
    /// Total number of NFTs in collection
    pub total_nfts: u64,
    /// Number of verified NFTs
    pub verified_nfts: u64,
    /// Whether the collection is finalized
    pub is_finalized: bool,
    /// Bump for PDA
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct CollectionMember {
    /// The collection this NFT belongs to
    pub collection: Pubkey,
    /// The NFT mint pubkey
    pub nft_mint: Pubkey,
    /// Whether this NFT is verified
    pub is_verified: bool,
    /// Bump for PDA
    pub bump: u8,
}

// =============================================================================
// ERROR CODES
// =============================================================================

#[error_code]
pub enum ErrorCode {
    #[msg("Name too long (max 32 characters)")]
    NameTooLong,
    #[msg("Symbol too long (max 10 characters)")]
    SymbolTooLong,
    #[msg("URI too long (max 200 characters)")]
    UriTooLong,
    #[msg("Collection is finalized")]
    CollectionFinalized,
    #[msg("NFT not in this collection")]
    NftNotInCollection,
    #[msg("NFT already verified")]
    NftAlreadyVerified,
    #[msg("NFT not verified")]
    NftNotVerified,
    #[msg("Unauthorized")]
    Unauthorized,
}
