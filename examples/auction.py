# auction
# Built with Seahorse v0.1.0
#
# Demonstrates an auction program with time-based state transitions.
# Features:
# - Create auction with starting price and end time
# - Place bids (SPL tokens), previous highest bidder refunded
# - End auction after time expires
# - Winner claims prize, seller receives payment
#
# Design note: Uses SPL tokens for bids (not native SOL) because Seahorse
# supports PDA-signed token transfers but not PDA-signed lamport transfers.
# Auction PDA is authority for bid escrow vault.

from seahorse.prelude import *

declare_id('AUCTnhLbEfvJxTt9NQekQfpjsVq4xDv5aX1VL6h9pLwD')


class Auction(Account):
    # Seller who created the auction
    seller: Pubkey
    # Unique auction identifier
    auction_id: u64
    # Token mint for bids
    bid_mint: Pubkey
    # Starting price in tokens
    starting_price: u64
    # Current highest bid (0 if no bids)
    current_bid: u64
    # Current highest bidder (default if no bids)
    highest_bidder: Pubkey
    # Auction start slot
    start_slot: u64
    # Auction end slot
    end_slot: u64
    # Whether auction has ended
    is_ended: bool
    # Whether prize has been claimed
    is_claimed: bool
    # Bump seed for PDA
    bump: u8


@instruction
def create_auction(
    seller: Signer,
    bid_mint: TokenMint,
    auction: Empty[Auction],
    bid_escrow: Empty[TokenAccount],
    clock: Clock,
    auction_id: u64,
    starting_price: u64,
    duration_slots: u64
):
    """
    Create a new auction.

    Args:
        seller: Auction creator who receives payment
        bid_mint: Token mint for bids
        auction: Empty auction account to initialize
        bid_escrow: Empty token account for bid escrow
        clock: Clock sysvar for time
        auction_id: Unique identifier for this auction
        starting_price: Minimum bid in tokens
        duration_slots: Number of slots auction runs for
    """
    # Validate inputs
    assert starting_price > 0, 'Starting price must be greater than zero'
    assert duration_slots > 0, 'Duration must be greater than zero'

    # Get current slot
    current_slot = clock.slot()

    # Get bump before init
    auction_bump = auction.bump()

    # Initialize auction state PDA
    auction = auction.init(
        payer=seller,
        seeds=['auction', auction_id]
    )

    # Initialize bid escrow token account PDA with auction as authority
    bid_escrow.init(
        payer=seller,
        seeds=['bid_escrow', auction_id],
        mint=bid_mint,
        authority=auction
    )

    # Store auction configuration
    auction.seller = seller.key()
    auction.auction_id = auction_id
    auction.bid_mint = bid_mint.key()
    auction.starting_price = starting_price
    auction.current_bid = 0
    # Use seller as placeholder for no bidder (current_bid=0 indicates no bids)
    auction.highest_bidder = seller.key()
    auction.start_slot = current_slot
    auction.end_slot = current_slot + duration_slots
    auction.is_ended = False
    auction.is_claimed = False
    auction.bump = auction_bump


@instruction
def place_bid(
    bidder: Signer,
    previous_bidder: UncheckedAccount,
    bid_mint: TokenMint,
    bidder_token_account: TokenAccount,
    previous_bidder_token_account: TokenAccount,
    auction: Auction,
    bid_escrow: TokenAccount,
    clock: Clock,
    bid_amount: u64
):
    """
    Place a bid on an auction.

    Bid amount must exceed current bid (or starting price if no bids).
    Previous highest bidder is automatically refunded.

    Args:
        bidder: The person placing the bid
        previous_bidder: Account of previous highest bidder (for refund)
        bid_mint: Token mint for bids
        bidder_token_account: Bidder's token account
        previous_bidder_token_account: Previous bidder's token account for refund
        auction: The auction being bid on
        bid_escrow: Token account holding bid escrow
        clock: Clock sysvar for time
        bid_amount: Amount of tokens to bid
    """
    # Auction must be active (not manually ended)
    assert not auction.is_ended, 'Auction has ended'

    # Get current slot
    current_slot = clock.slot()

    # Auction must not have expired
    assert current_slot < auction.end_slot, 'Auction time has expired'

    # Determine minimum bid
    min_bid: u64 = 0
    if auction.current_bid > 0:
        min_bid = auction.current_bid
    else:
        min_bid = auction.starting_price

    # Bid must exceed minimum
    assert bid_amount > min_bid, 'Bid amount is too low'

    # Get auction bump for PDA signer
    auction_bump = auction.bump
    auction_id = auction.auction_id

    # Refund previous highest bidder if exists
    if auction.current_bid > 0:
        # Transfer from escrow back to previous bidder using PDA signer
        bid_escrow.transfer(
            authority=auction,
            to=previous_bidder_token_account,
            amount=auction.current_bid,
            signer=['auction', auction_id, auction_bump]
        )

    # Transfer new bid to escrow
    bidder_token_account.transfer(
        authority=bidder,
        to=bid_escrow,
        amount=bid_amount
    )

    # Update auction state
    auction.highest_bidder = bidder.key()
    auction.current_bid = bid_amount


@instruction
def end_auction(
    caller: Signer,
    auction: Auction,
    clock: Clock
):
    """
    End the auction.

    Can only be called after end_slot has passed.
    Marks the auction as ended, enabling prize claim.

    Args:
        caller: Anyone can call this after time expires
        auction: The auction to end
        clock: Clock sysvar for time
    """
    # Cannot end already ended auction
    assert not auction.is_ended, 'Auction has already been ended'

    # Get current slot
    current_slot = clock.slot()

    # Must be after end time
    assert current_slot >= auction.end_slot, 'Auction is still active'

    # Mark as ended
    auction.is_ended = True


@instruction
def claim_prize(
    winner: Signer,
    seller: UncheckedAccount,
    bid_mint: TokenMint,
    seller_token_account: TokenAccount,
    auction: Auction,
    bid_escrow: TokenAccount
):
    """
    Claim auction prize.

    Winner confirms claim, seller receives payment from escrow.
    Can only be called after auction is ended.

    Args:
        winner: The auction winner (highest bidder)
        seller: The seller to receive payment
        bid_mint: Token mint for bids
        seller_token_account: Seller's token account to receive payment
        auction: The auction
        bid_escrow: Token account holding bid escrow
    """
    # Auction must be ended
    assert auction.is_ended, 'Auction has not ended yet'

    # Prize must not be already claimed
    assert not auction.is_claimed, 'Prize has already been claimed'

    # Must have bids
    assert auction.current_bid > 0, 'No bids were placed'

    # Caller must be the winner
    assert auction.highest_bidder == winner.key(), 'Not the winner'

    # Seller must match
    assert auction.seller == seller.key(), 'Invalid seller'

    # Get auction bump for PDA signer
    auction_bump = auction.bump
    auction_id = auction.auction_id

    # Transfer payment from escrow to seller using PDA signer
    bid_escrow.transfer(
        authority=auction,
        to=seller_token_account,
        amount=auction.current_bid,
        signer=['auction', auction_id, auction_bump]
    )

    # Mark as claimed
    auction.is_claimed = True
