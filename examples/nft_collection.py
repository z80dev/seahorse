# nft_collection.py
# NFT collection configuration tracking program with verified membership
#
# NOTE: Seahorse doesn't have native Metaplex Token Metadata support.
# This program demonstrates collection management and verified membership tracking.
# The Anchor reference (nft_collection_anchor) implements actual Metaplex CPI
# for creating collection NFTs and verifying collection membership.
#
# Instructions:
#   - create_collection: Create a collection configuration
#   - update_collection: Update collection metadata (before finalization)
#   - finalize_collection: Lock the collection (no more metadata changes)
#   - add_nft_to_collection: Register an NFT to the collection
#   - verify_nft: Mark an NFT as verified in the collection
#   - unverify_nft: Remove verification status from an NFT
#   - get_collection_info: Display collection information

from seahorse.prelude import *

declare_id('NFTCoL1ect1on111111111111111111111111111111')

# Collection configuration account
# Stores collection metadata that would be used with Metaplex Token Metadata program
class Collection(Account):
    # The authority who can manage the collection
    authority: Pubkey
    # Unique collection identifier (used for PDA derivation)
    collection_id: u64
    # The collection mint pubkey (for actual Metaplex collection)
    collection_mint: Pubkey
    # Collection name (max 32 chars)
    name: str
    # Collection symbol (max 10 chars)
    symbol: str
    # Metadata URI for collection JSON
    uri: str
    # Total number of NFTs in this collection
    total_nfts: u64
    # Number of verified NFTs in this collection
    verified_nfts: u64
    # Whether the collection is finalized (no more changes)
    is_finalized: bool
    # Bump for PDA derivation
    bump: u8


# NFT membership in a collection
# Tracks individual NFT's collection association and verification status
class CollectionMember(Account):
    # The collection this NFT belongs to
    collection: Pubkey
    # The NFT mint pubkey
    nft_mint: Pubkey
    # Whether this NFT is verified as part of the collection
    is_verified: bool
    # Bump for PDA derivation
    bump: u8


@instruction
def create_collection(
    authority: Signer,
    collection: Empty[Collection],
    collection_id: u64,
    name: str,
    symbol: str,
    uri: str
):
    """
    Create a new collection configuration.

    In a full Metaplex implementation, this would also:
    - Create the collection NFT mint (0 decimals, supply 1)
    - Create metadata account for the collection NFT
    - Create master edition for the collection NFT

    Since Seahorse doesn't have native Metaplex support, this creates
    a configuration account that tracks the collection metadata.
    """
    # Validate string lengths
    assert len(name) <= 32, "Name too long (max 32 characters)"
    assert len(symbol) <= 10, "Symbol too long (max 10 characters)"
    assert len(uri) <= 200, "URI too long (max 200 characters)"

    # Store bump before init
    bump = collection.bump()

    # Initialize the collection account
    collection = collection.init(
        payer=authority,
        seeds=['collection', collection_id],
        padding=300  # Space for name(36) + symbol(14) + uri(204) + extra
    )

    # Print first (before moving the strings into the account)
    print("Collection created with id:", collection_id)

    collection.authority = authority.key()
    collection.collection_id = collection_id
    # Collection mint will be set when actual Metaplex mint is created externally
    collection.collection_mint = authority.key()  # Placeholder
    collection.name = name
    collection.symbol = symbol
    collection.uri = uri
    collection.total_nfts = u64(0)
    collection.verified_nfts = u64(0)
    collection.is_finalized = False
    collection.bump = bump


@instruction
def update_collection(
    authority: Signer,
    collection: Collection,
    name: str,
    symbol: str,
    uri: str
):
    """
    Update collection metadata before finalization.

    Once a collection is finalized, its metadata cannot be changed.
    """
    # Verify authority
    assert collection.authority == authority.key(), "Unauthorized"

    # Cannot update after finalization
    assert not collection.is_finalized, "Collection is finalized"

    # Validate and update name if not empty
    if len(name) > 0:
        assert len(name) <= 32, "Name too long (max 32 characters)"
        collection.name = name

    # Validate and update symbol if not empty
    if len(symbol) > 0:
        assert len(symbol) <= 10, "Symbol too long (max 10 characters)"
        collection.symbol = symbol

    # Validate and update URI if not empty
    if len(uri) > 0:
        assert len(uri) <= 200, "URI too long (max 200 characters)"
        collection.uri = uri

    print("Collection updated")


@instruction
def finalize_collection(
    authority: Signer,
    collection: Collection
):
    """
    Finalize the collection, preventing further metadata changes.

    This is similar to making the collection metadata immutable in Metaplex.
    NFTs can still be added and verified after finalization.
    """
    # Verify authority
    assert collection.authority == authority.key(), "Unauthorized"

    # Cannot finalize twice
    assert not collection.is_finalized, "Collection already finalized"

    collection.is_finalized = True

    print("Collection finalized")


@instruction
def set_collection_mint(
    authority: Signer,
    collection: Collection,
    collection_mint: TokenMint
):
    """
    Set the collection mint address after creating the actual Metaplex collection NFT.

    This allows linking the configuration to an externally-created collection NFT.
    """
    # Verify authority
    assert collection.authority == authority.key(), "Unauthorized"

    collection.collection_mint = collection_mint.key()

    print("Collection mint set to:", collection_mint.key())


@instruction
def add_nft_to_collection(
    authority: Signer,
    collection: Collection,
    nft_mint: TokenMint,
    member: Empty[CollectionMember]
):
    """
    Register an NFT as a member of the collection.

    In Metaplex, this would set the collection field on the NFT's metadata account.
    The NFT starts as unverified - call verify_nft to verify.
    """
    # Verify collection authority
    assert collection.authority == authority.key(), "Unauthorized"

    # Store bump before init
    bump = member.bump()

    # Initialize member account with seeds based on collection and nft_mint
    member = member.init(
        payer=authority,
        seeds=['member', collection.key(), nft_mint.key()]
    )

    member.collection = collection.key()
    member.nft_mint = nft_mint.key()
    member.is_verified = False
    member.bump = bump

    # Increment total NFTs in collection
    collection.total_nfts = collection.total_nfts + 1

    print("NFT added to collection:", nft_mint.key())


@instruction
def verify_nft(
    authority: Signer,
    collection: Collection,
    member: CollectionMember
):
    """
    Verify an NFT's membership in the collection.

    In Metaplex, this is done via the SetAndVerifyCollection or VerifyCollection
    instruction, which requires the collection authority to sign.

    Only the collection authority can verify NFTs.
    """
    # Verify collection authority
    assert collection.authority == authority.key(), "Unauthorized"

    # Verify member belongs to this collection
    assert member.collection == collection.key(), "NFT not in this collection"

    # Cannot verify twice
    assert not member.is_verified, "NFT already verified"

    member.is_verified = True

    # Increment verified count
    collection.verified_nfts = collection.verified_nfts + 1

    print("NFT verified in collection")


@instruction
def unverify_nft(
    authority: Signer,
    collection: Collection,
    member: CollectionMember
):
    """
    Remove verification status from an NFT.

    In Metaplex, this is done via the UnverifyCollection instruction.
    """
    # Verify collection authority
    assert collection.authority == authority.key(), "Unauthorized"

    # Verify member belongs to this collection
    assert member.collection == collection.key(), "NFT not in this collection"

    # Must be verified to unverify
    assert member.is_verified, "NFT not verified"

    member.is_verified = False

    # Decrement verified count
    collection.verified_nfts = collection.verified_nfts - 1

    print("NFT unverified from collection")


@instruction
def get_collection_info(collection: Collection):
    """
    Display collection information.

    This is a read-only instruction that logs the collection metadata.
    """
    print("Collection Info:")
    print("  Authority:", collection.authority)
    print("  Collection Mint:", collection.collection_mint)
    print("  Name:", collection.name)
    print("  Symbol:", collection.symbol)
    print("  URI:", collection.uri)
    print("  Total NFTs:", collection.total_nfts)
    print("  Verified NFTs:", collection.verified_nfts)
    print("  Is Finalized:", collection.is_finalized)


@instruction
def get_member_info(member: CollectionMember):
    """
    Display member information.

    This is a read-only instruction that logs the member status.
    """
    print("Member Info:")
    print("  Collection:", member.collection)
    print("  NFT Mint:", member.nft_mint)
    print("  Is Verified:", member.is_verified)
