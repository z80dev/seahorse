use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("AUCTnhLbEfvJxTt9NQekQfpjsVq4xDv5aX1VL6h9pLwD");

/// Token-based auction program with time-based state transitions
///
/// Features:
/// - Seller creates auction with starting price (in SPL tokens) and end slot
/// - Bidders place bids in tokens, previous highest bidder refunded
/// - After end time, auction can be finalized
/// - Winner claims prize (tokens transferred to seller)
#[program]
pub mod auction_anchor {
    use super::*;

    /// Create a new auction
    ///
    /// Args:
    /// - auction_id: Unique identifier for this auction
    /// - starting_price: Minimum bid in tokens
    /// - duration_slots: Number of slots auction runs for
    pub fn create_auction(
        ctx: Context<CreateAuction>,
        auction_id: u64,
        starting_price: u64,
        duration_slots: u64,
    ) -> Result<()> {
        require!(starting_price > 0, ErrorCode::InvalidStartingPrice);
        require!(duration_slots > 0, ErrorCode::InvalidDuration);

        let clock = Clock::get()?;
        let current_slot = clock.slot;

        let auction = &mut ctx.accounts.auction;
        auction.seller = ctx.accounts.seller.key();
        auction.auction_id = auction_id;
        auction.bid_mint = ctx.accounts.bid_mint.key();
        auction.starting_price = starting_price;
        auction.current_bid = 0;
        auction.highest_bidder = Pubkey::default(); // No bidder initially
        auction.start_slot = current_slot;
        auction.end_slot = current_slot
            .checked_add(duration_slots)
            .ok_or(ErrorCode::Overflow)?;
        auction.is_ended = false;
        auction.is_claimed = false;
        auction.bump = ctx.bumps.auction;

        msg!("Auction created: id={}, starting_price={}, end_slot={}",
             auction_id, starting_price, auction.end_slot);

        Ok(())
    }

    /// Place a bid on an auction
    ///
    /// Bid amount must exceed current bid (or starting price if no bids)
    /// Previous highest bidder is automatically refunded
    pub fn place_bid(ctx: Context<PlaceBid>, bid_amount: u64) -> Result<()> {
        let auction = &ctx.accounts.auction;

        // Auction must be active
        require!(!auction.is_ended, ErrorCode::AuctionEnded);

        let clock = Clock::get()?;
        require!(clock.slot < auction.end_slot, ErrorCode::AuctionExpired);

        // Determine minimum bid
        let min_bid = if auction.current_bid > 0 {
            auction.current_bid
        } else {
            auction.starting_price
        };

        // Bid must exceed minimum
        require!(bid_amount > min_bid, ErrorCode::BidTooLow);

        // Refund previous highest bidder if exists
        let previous_bid = auction.current_bid;

        if previous_bid > 0 {
            // Transfer from escrow back to previous bidder using PDA signer
            let auction_id_bytes = auction.auction_id.to_le_bytes();
            let bump = auction.bump;
            let signer_seeds: &[&[&[u8]]] = &[&[
                b"auction",
                auction_id_bytes.as_ref(),
                &[bump],
            ]];

            let cpi_accounts = Transfer {
                from: ctx.accounts.bid_escrow.to_account_info(),
                to: ctx.accounts.previous_bidder_token_account.to_account_info(),
                authority: ctx.accounts.auction.to_account_info(),
            };
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                signer_seeds,
            );
            token::transfer(cpi_ctx, previous_bid)?;
        }

        // Transfer new bid to escrow
        let cpi_accounts = Transfer {
            from: ctx.accounts.bidder_token_account.to_account_info(),
            to: ctx.accounts.bid_escrow.to_account_info(),
            authority: ctx.accounts.bidder.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        );
        token::transfer(cpi_ctx, bid_amount)?;

        // Update auction state
        let auction = &mut ctx.accounts.auction;
        auction.highest_bidder = ctx.accounts.bidder.key();
        auction.current_bid = bid_amount;

        msg!("Bid placed: bidder={}, amount={}", ctx.accounts.bidder.key(), bid_amount);

        Ok(())
    }

    /// End the auction
    ///
    /// Can only be called after end_slot has passed
    /// Marks the auction as ended, enabling prize claim
    pub fn end_auction(ctx: Context<EndAuction>) -> Result<()> {
        let auction = &ctx.accounts.auction;

        require!(!auction.is_ended, ErrorCode::AuctionAlreadyEnded);

        let clock = Clock::get()?;
        require!(clock.slot >= auction.end_slot, ErrorCode::AuctionStillActive);

        let auction = &mut ctx.accounts.auction;
        auction.is_ended = true;

        msg!("Auction ended: id={}, winning_bid={}",
             auction.auction_id, auction.current_bid);

        Ok(())
    }

    /// Claim auction prize
    ///
    /// Winner receives confirmation, seller receives payment
    /// Can only be called after auction is ended
    pub fn claim_prize(ctx: Context<ClaimPrize>) -> Result<()> {
        let auction = &ctx.accounts.auction;

        require!(auction.is_ended, ErrorCode::AuctionNotEnded);
        require!(!auction.is_claimed, ErrorCode::PrizeAlreadyClaimed);
        require!(auction.current_bid > 0, ErrorCode::NoBids);
        require!(
            auction.highest_bidder == ctx.accounts.winner.key(),
            ErrorCode::NotTheWinner
        );

        let payment_amount = auction.current_bid;

        // Transfer payment from escrow to seller using PDA signer
        let auction_id_bytes = auction.auction_id.to_le_bytes();
        let bump = auction.bump;
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"auction",
            auction_id_bytes.as_ref(),
            &[bump],
        ]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.bid_escrow.to_account_info(),
            to: ctx.accounts.seller_token_account.to_account_info(),
            authority: ctx.accounts.auction.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token::transfer(cpi_ctx, payment_amount)?;

        // Mark as claimed
        let auction = &mut ctx.accounts.auction;
        auction.is_claimed = true;

        msg!("Prize claimed: winner={}, payment_to_seller={}",
             ctx.accounts.winner.key(), payment_amount);

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(auction_id: u64)]
pub struct CreateAuction<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,

    pub bid_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = seller,
        space = 8 + Auction::INIT_SPACE,
        seeds = [b"auction", auction_id.to_le_bytes().as_ref()],
        bump
    )]
    pub auction: Account<'info, Auction>,

    /// Bid escrow token account
    #[account(
        init,
        payer = seller,
        token::mint = bid_mint,
        token::authority = auction,
        seeds = [b"bid_escrow", auction_id.to_le_bytes().as_ref()],
        bump
    )]
    pub bid_escrow: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PlaceBid<'info> {
    #[account(mut)]
    pub bidder: Signer<'info>,

    #[account(
        mut,
        seeds = [b"auction", auction.auction_id.to_le_bytes().as_ref()],
        bump = auction.bump
    )]
    pub auction: Account<'info, Auction>,

    /// Bidder's token account
    #[account(
        mut,
        constraint = bidder_token_account.mint == auction.bid_mint,
        constraint = bidder_token_account.owner == bidder.key()
    )]
    pub bidder_token_account: Account<'info, TokenAccount>,

    /// Previous highest bidder's token account for refund
    /// CHECK: Validated by mint constraint
    #[account(
        mut,
        constraint = previous_bidder_token_account.mint == auction.bid_mint
    )]
    pub previous_bidder_token_account: Account<'info, TokenAccount>,

    /// Bid escrow token account
    #[account(
        mut,
        seeds = [b"bid_escrow", auction.auction_id.to_le_bytes().as_ref()],
        bump
    )]
    pub bid_escrow: Account<'info, TokenAccount>,

    pub bid_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct EndAuction<'info> {
    pub caller: Signer<'info>,

    #[account(
        mut,
        seeds = [b"auction", auction.auction_id.to_le_bytes().as_ref()],
        bump = auction.bump
    )]
    pub auction: Account<'info, Auction>,
}

#[derive(Accounts)]
pub struct ClaimPrize<'info> {
    pub winner: Signer<'info>,

    #[account(
        mut,
        seeds = [b"auction", auction.auction_id.to_le_bytes().as_ref()],
        bump = auction.bump
    )]
    pub auction: Account<'info, Auction>,

    /// Seller's token account to receive payment
    #[account(
        mut,
        constraint = seller_token_account.mint == auction.bid_mint,
        constraint = seller_token_account.owner == auction.seller @ ErrorCode::InvalidSeller
    )]
    pub seller_token_account: Account<'info, TokenAccount>,

    /// Bid escrow token account
    #[account(
        mut,
        seeds = [b"bid_escrow", auction.auction_id.to_le_bytes().as_ref()],
        bump
    )]
    pub bid_escrow: Account<'info, TokenAccount>,

    pub bid_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(InitSpace)]
pub struct Auction {
    /// Seller who created the auction
    pub seller: Pubkey,
    /// Unique auction identifier
    pub auction_id: u64,
    /// Token mint for bids
    pub bid_mint: Pubkey,
    /// Starting price in tokens
    pub starting_price: u64,
    /// Current highest bid (0 if no bids)
    pub current_bid: u64,
    /// Current highest bidder (Pubkey::default if no bids)
    pub highest_bidder: Pubkey,
    /// Auction start slot
    pub start_slot: u64,
    /// Auction end slot
    pub end_slot: u64,
    /// Whether auction has ended
    pub is_ended: bool,
    /// Whether prize has been claimed
    pub is_claimed: bool,
    /// PDA bump seed
    pub bump: u8,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Starting price must be greater than zero")]
    InvalidStartingPrice,
    #[msg("Duration must be greater than zero")]
    InvalidDuration,
    #[msg("Bid amount is too low")]
    BidTooLow,
    #[msg("Auction has already ended")]
    AuctionEnded,
    #[msg("Auction time has expired")]
    AuctionExpired,
    #[msg("Auction is still active")]
    AuctionStillActive,
    #[msg("Auction has already been ended")]
    AuctionAlreadyEnded,
    #[msg("Auction has not ended yet")]
    AuctionNotEnded,
    #[msg("Prize has already been claimed")]
    PrizeAlreadyClaimed,
    #[msg("No bids were placed")]
    NoBids,
    #[msg("Caller is not the winner")]
    NotTheWinner,
    #[msg("Invalid seller account")]
    InvalidSeller,
    #[msg("Arithmetic overflow")]
    Overflow,
}
