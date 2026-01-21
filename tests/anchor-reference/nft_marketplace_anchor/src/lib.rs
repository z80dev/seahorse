use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("MKT1NftMarketP1ace111111111111111111111111");

/// NFT Marketplace program for listing and purchasing NFTs
///
/// Features:
/// - Seller lists NFT with price (in SPL tokens)
/// - NFT escrowed in PDA-owned vault during listing
/// - Buyer purchases NFT, payment goes to seller
/// - Seller can delist and recover NFT
///
/// Design note: Uses SPL tokens for payment (not SOL) because Seahorse
/// supports PDA-signed token transfers but not PDA-signed lamport transfers.
#[program]
pub mod nft_marketplace_anchor {
    use super::*;

    /// List an NFT for sale
    ///
    /// Args:
    /// - listing_id: Unique identifier for this listing
    /// - price: Price in payment tokens
    pub fn list_nft(
        ctx: Context<ListNft>,
        listing_id: u64,
        price: u64,
    ) -> Result<()> {
        require!(price > 0, ErrorCode::InvalidPrice);

        let listing = &mut ctx.accounts.listing;
        listing.seller = ctx.accounts.seller.key();
        listing.listing_id = listing_id;
        listing.nft_mint = ctx.accounts.nft_mint.key();
        listing.payment_mint = ctx.accounts.payment_mint.key();
        listing.price = price;
        listing.is_active = true;
        listing.bump = ctx.bumps.listing;

        // Transfer NFT from seller to escrow vault
        let cpi_accounts = Transfer {
            from: ctx.accounts.seller_nft_token.to_account_info(),
            to: ctx.accounts.nft_escrow.to_account_info(),
            authority: ctx.accounts.seller.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        );
        token::transfer(cpi_ctx, 1)?;

        msg!("NFT listed: id={}, price={}", listing_id, price);

        Ok(())
    }

    /// Buy a listed NFT
    ///
    /// Buyer pays the listed price, NFT transferred to buyer
    pub fn buy_nft(ctx: Context<BuyNft>) -> Result<()> {
        let listing = &ctx.accounts.listing;

        require!(listing.is_active, ErrorCode::ListingInactive);

        let price = listing.price;
        let listing_id = listing.listing_id;
        let bump = listing.bump;

        // Transfer payment from buyer to seller
        let cpi_accounts = Transfer {
            from: ctx.accounts.buyer_payment_token.to_account_info(),
            to: ctx.accounts.seller_payment_token.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        );
        token::transfer(cpi_ctx, price)?;

        // Transfer NFT from escrow to buyer using PDA signer
        let listing_id_bytes = listing_id.to_le_bytes();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"listing",
            listing_id_bytes.as_ref(),
            &[bump],
        ]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.nft_escrow.to_account_info(),
            to: ctx.accounts.buyer_nft_token.to_account_info(),
            authority: ctx.accounts.listing.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token::transfer(cpi_ctx, 1)?;

        // Mark listing as inactive
        let listing = &mut ctx.accounts.listing;
        listing.is_active = false;

        msg!("NFT purchased: listing_id={}, price={}", listing_id, price);

        Ok(())
    }

    /// Delist an NFT (cancel listing)
    ///
    /// Only seller can delist. NFT returned to seller.
    pub fn delist_nft(ctx: Context<DelistNft>) -> Result<()> {
        let listing = &ctx.accounts.listing;

        require!(listing.is_active, ErrorCode::ListingInactive);
        require!(
            listing.seller == ctx.accounts.seller.key(),
            ErrorCode::Unauthorized
        );

        let listing_id = listing.listing_id;
        let bump = listing.bump;

        // Transfer NFT from escrow back to seller using PDA signer
        let listing_id_bytes = listing_id.to_le_bytes();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"listing",
            listing_id_bytes.as_ref(),
            &[bump],
        ]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.nft_escrow.to_account_info(),
            to: ctx.accounts.seller_nft_token.to_account_info(),
            authority: ctx.accounts.listing.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token::transfer(cpi_ctx, 1)?;

        // Mark listing as inactive
        let listing = &mut ctx.accounts.listing;
        listing.is_active = false;

        msg!("NFT delisted: listing_id={}", listing_id);

        Ok(())
    }

    /// Update listing price
    ///
    /// Only seller can update price while listing is active
    pub fn update_price(ctx: Context<UpdatePrice>, new_price: u64) -> Result<()> {
        let listing = &mut ctx.accounts.listing;

        require!(listing.is_active, ErrorCode::ListingInactive);
        require!(
            listing.seller == ctx.accounts.seller.key(),
            ErrorCode::Unauthorized
        );
        require!(new_price > 0, ErrorCode::InvalidPrice);

        let old_price = listing.price;
        listing.price = new_price;

        msg!("Price updated: listing_id={}, {} -> {}",
             listing.listing_id, old_price, new_price);

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(listing_id: u64)]
pub struct ListNft<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,

    /// NFT mint (should have supply = 1, decimals = 0)
    pub nft_mint: Account<'info, Mint>,

    /// Payment token mint
    pub payment_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = seller,
        space = 8 + Listing::INIT_SPACE,
        seeds = [b"listing", listing_id.to_le_bytes().as_ref()],
        bump
    )]
    pub listing: Account<'info, Listing>,

    /// NFT escrow vault owned by listing PDA
    #[account(
        init,
        payer = seller,
        token::mint = nft_mint,
        token::authority = listing,
        seeds = [b"nft_escrow", listing_id.to_le_bytes().as_ref()],
        bump
    )]
    pub nft_escrow: Account<'info, TokenAccount>,

    /// Seller's NFT token account
    #[account(
        mut,
        constraint = seller_nft_token.mint == nft_mint.key(),
        constraint = seller_nft_token.owner == seller.key(),
        constraint = seller_nft_token.amount >= 1 @ ErrorCode::InsufficientNftBalance
    )]
    pub seller_nft_token: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BuyNft<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"listing", listing.listing_id.to_le_bytes().as_ref()],
        bump = listing.bump
    )]
    pub listing: Account<'info, Listing>,

    /// NFT escrow vault
    #[account(
        mut,
        seeds = [b"nft_escrow", listing.listing_id.to_le_bytes().as_ref()],
        bump
    )]
    pub nft_escrow: Account<'info, TokenAccount>,

    /// Buyer's NFT token account
    #[account(
        mut,
        constraint = buyer_nft_token.mint == listing.nft_mint
    )]
    pub buyer_nft_token: Account<'info, TokenAccount>,

    /// Buyer's payment token account
    #[account(
        mut,
        constraint = buyer_payment_token.mint == listing.payment_mint,
        constraint = buyer_payment_token.owner == buyer.key(),
        constraint = buyer_payment_token.amount >= listing.price @ ErrorCode::InsufficientPayment
    )]
    pub buyer_payment_token: Account<'info, TokenAccount>,

    /// Seller's payment token account
    #[account(
        mut,
        constraint = seller_payment_token.mint == listing.payment_mint,
        constraint = seller_payment_token.owner == listing.seller
    )]
    pub seller_payment_token: Account<'info, TokenAccount>,

    pub nft_mint: Account<'info, Mint>,
    pub payment_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct DelistNft<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,

    #[account(
        mut,
        seeds = [b"listing", listing.listing_id.to_le_bytes().as_ref()],
        bump = listing.bump
    )]
    pub listing: Account<'info, Listing>,

    /// NFT escrow vault
    #[account(
        mut,
        seeds = [b"nft_escrow", listing.listing_id.to_le_bytes().as_ref()],
        bump
    )]
    pub nft_escrow: Account<'info, TokenAccount>,

    /// Seller's NFT token account
    #[account(
        mut,
        constraint = seller_nft_token.mint == listing.nft_mint,
        constraint = seller_nft_token.owner == seller.key()
    )]
    pub seller_nft_token: Account<'info, TokenAccount>,

    pub nft_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct UpdatePrice<'info> {
    pub seller: Signer<'info>,

    #[account(
        mut,
        seeds = [b"listing", listing.listing_id.to_le_bytes().as_ref()],
        bump = listing.bump
    )]
    pub listing: Account<'info, Listing>,
}

#[account]
#[derive(InitSpace)]
pub struct Listing {
    /// Seller who listed the NFT
    pub seller: Pubkey,
    /// Unique listing identifier
    pub listing_id: u64,
    /// NFT mint address
    pub nft_mint: Pubkey,
    /// Payment token mint address
    pub payment_mint: Pubkey,
    /// Price in payment tokens
    pub price: u64,
    /// Whether the listing is active
    pub is_active: bool,
    /// PDA bump seed
    pub bump: u8,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Price must be greater than zero")]
    InvalidPrice,
    #[msg("Listing is not active")]
    ListingInactive,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Insufficient NFT balance")]
    InsufficientNftBalance,
    #[msg("Insufficient payment balance")]
    InsufficientPayment,
}
