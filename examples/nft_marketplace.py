# nft_marketplace
# Built with Seahorse v0.1.0
#
# Demonstrates an NFT marketplace for listing and purchasing NFTs.
# Features:
# - List NFT with price (in SPL tokens)
# - NFT escrowed in PDA-owned vault during listing
# - Buy NFT (payment to seller, NFT to buyer)
# - Delist NFT (cancel listing, NFT returned to seller)
# - Update listing price
#
# Design note: Uses SPL tokens for payment (not native SOL) because Seahorse
# supports PDA-signed token transfers but not PDA-signed lamport transfers.
# Listing PDA is authority for NFT escrow vault.

from seahorse.prelude import *

declare_id('Coa8ZSxePf7ZCsDNHE9SRHyZJRWWnZgmanuKVf4ZBM7Q')


class Listing(Account):
    # Seller who listed the NFT
    seller: Pubkey
    # Unique listing identifier
    listing_id: u64
    # NFT mint address
    nft_mint: Pubkey
    # Payment token mint address
    payment_mint: Pubkey
    # Price in payment tokens
    price: u64
    # Whether the listing is active
    is_active: bool
    # PDA bump seed
    bump: u8


@instruction
def list_nft(
    seller: Signer,
    nft_mint: TokenMint,
    payment_mint: TokenMint,
    listing: Empty[Listing],
    nft_escrow: Empty[TokenAccount],
    seller_nft_token: TokenAccount,
    listing_id: u64,
    price: u64
):
    """
    List an NFT for sale on the marketplace.

    Args:
        seller: The NFT owner listing the NFT
        nft_mint: The NFT mint (should have supply=1, decimals=0)
        payment_mint: Token mint for payments
        listing: Empty listing account to initialize
        nft_escrow: Empty token account for NFT escrow
        seller_nft_token: Seller's token account holding the NFT
        listing_id: Unique identifier for this listing
        price: Price in payment tokens
    """
    # Validate price
    assert price > 0, 'Price must be greater than zero'

    # Validate seller owns the NFT
    assert seller_nft_token.amount() >= 1, 'Seller does not own the NFT'

    # Get bump before init
    listing_bump = listing.bump()

    # Initialize listing state PDA
    listing = listing.init(
        payer=seller,
        seeds=['listing', listing_id]
    )

    # Initialize NFT escrow token account with listing as authority
    nft_escrow = nft_escrow.init(
        payer=seller,
        seeds=['nft_escrow', listing_id],
        mint=nft_mint,
        authority=listing
    )

    # Store listing configuration
    listing.seller = seller.key()
    listing.listing_id = listing_id
    listing.nft_mint = nft_mint.key()
    listing.payment_mint = payment_mint.key()
    listing.price = price
    listing.is_active = True
    listing.bump = listing_bump

    # Transfer NFT from seller to escrow
    seller_nft_token.transfer(
        authority=seller,
        to=nft_escrow,
        amount=u64(1)
    )


@instruction
def buy_nft(
    buyer: Signer,
    seller: UncheckedAccount,
    nft_mint: TokenMint,
    payment_mint: TokenMint,
    listing: Listing,
    nft_escrow: TokenAccount,
    buyer_nft_token: TokenAccount,
    buyer_payment_token: TokenAccount,
    seller_payment_token: TokenAccount
):
    """
    Buy a listed NFT.

    Payment transferred to seller, NFT transferred to buyer.

    Args:
        buyer: The person buying the NFT
        seller: The seller's account (for validation)
        nft_mint: The NFT mint
        payment_mint: Token mint for payments
        listing: The listing being purchased
        nft_escrow: Token account holding escrowed NFT
        buyer_nft_token: Buyer's token account to receive NFT
        buyer_payment_token: Buyer's payment token account
        seller_payment_token: Seller's payment token account to receive payment
    """
    # Listing must be active
    assert listing.is_active, 'Listing is not active'

    # Validate seller matches
    assert listing.seller == seller.key(), 'Invalid seller'

    # Get listing info for signer
    listing_id = listing.listing_id
    listing_bump = listing.bump
    price = listing.price

    # Transfer payment from buyer to seller
    buyer_payment_token.transfer(
        authority=buyer,
        to=seller_payment_token,
        amount=price
    )

    # Transfer NFT from escrow to buyer using PDA signer
    nft_escrow.transfer(
        authority=listing,
        to=buyer_nft_token,
        amount=u64(1),
        signer=['listing', listing_id, listing_bump]
    )

    # Mark listing as inactive
    listing.is_active = False


@instruction
def delist_nft(
    seller: Signer,
    nft_mint: TokenMint,
    listing: Listing,
    nft_escrow: TokenAccount,
    seller_nft_token: TokenAccount
):
    """
    Delist an NFT (cancel listing).

    Only seller can delist. NFT returned to seller.

    Args:
        seller: The seller (must match listing seller)
        nft_mint: The NFT mint
        listing: The listing to cancel
        nft_escrow: Token account holding escrowed NFT
        seller_nft_token: Seller's token account to receive NFT back
    """
    # Listing must be active
    assert listing.is_active, 'Listing is not active'

    # Only seller can delist
    assert listing.seller == seller.key(), 'Unauthorized'

    # Get listing info for signer
    listing_id = listing.listing_id
    listing_bump = listing.bump

    # Transfer NFT from escrow back to seller using PDA signer
    nft_escrow.transfer(
        authority=listing,
        to=seller_nft_token,
        amount=u64(1),
        signer=['listing', listing_id, listing_bump]
    )

    # Mark listing as inactive
    listing.is_active = False


@instruction
def update_price(
    seller: Signer,
    listing: Listing,
    new_price: u64
):
    """
    Update listing price.

    Only seller can update price while listing is active.

    Args:
        seller: The seller (must match listing seller)
        listing: The listing to update
        new_price: New price in payment tokens
    """
    # Listing must be active
    assert listing.is_active, 'Listing is not active'

    # Only seller can update price
    assert listing.seller == seller.key(), 'Unauthorized'

    # Validate new price
    assert new_price > 0, 'Price must be greater than zero'

    # Update price
    listing.price = new_price
