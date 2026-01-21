# nft_mint.py
# NFT minting configuration tracking program
#
# NOTE: Seahorse doesn't have native Metaplex Token Metadata support.
# This program demonstrates NFT configuration tracking similar to how an NFT
# would be managed. The Anchor reference (nft_mint_anchor) implements actual
# Metaplex CPI for creating metadata and master edition accounts.
#
# Instructions:
#   - create_nft_config: Create NFT configuration with name, symbol, URI
#   - update_nft_config: Update NFT metadata (before minting)
#   - mark_as_minted: Mark the NFT as minted (simulates the minting step)
#   - get_nft_info: Display NFT configuration

from seahorse.prelude import *

declare_id('NFTm1nt111111111111111111111111111111111111')

# NFT Configuration account
# Stores metadata that would be used with Metaplex Token Metadata program
class NftConfig(Account):
    # The authority who can update/mint
    authority: Pubkey
    # Unique NFT identifier (used for PDA derivation)
    nft_id: u64
    # The mint pubkey for this NFT (stored after config creation)
    mint: Pubkey
    # NFT name (padded for storage)
    name: str
    # NFT symbol
    symbol: str
    # Metadata URI pointing to off-chain JSON
    uri: str
    # Whether the NFT has been minted
    is_minted: bool
    # Bump for PDA derivation
    bump: u8


@instruction
def create_nft_config(
    authority: Signer,
    nft_config: Empty[NftConfig],
    nft_id: u64,
    name: str,
    symbol: str,
    uri: str
):
    """
    Create a new NFT configuration.

    In a full implementation with Metaplex support, this would also:
    - Create the token mint (0 decimals)
    - Create the metadata account via CPI
    - Create the master edition account via CPI

    Since Seahorse doesn't have native Metaplex support, this creates
    a configuration account that tracks the NFT metadata.
    """
    # Validate string lengths
    assert len(name) <= 32, "Name too long (max 32 characters)"
    assert len(symbol) <= 10, "Symbol too long (max 10 characters)"
    assert len(uri) <= 200, "URI too long (max 200 characters)"

    # Store bump before init
    bump = nft_config.bump()

    # Initialize the NFT config account
    # Use nft_id for unique PDA derivation (similar to escrow pattern)
    # Use padding for variable-length strings
    nft_config = nft_config.init(
        payer=authority,
        seeds=['nft_config', nft_id],
        padding=300  # Space for name(36) + symbol(14) + uri(204) + extra
    )

    # Print first (before moving the strings into the account)
    print("NFT config created with id:", nft_id)

    nft_config.authority = authority.key()
    nft_config.nft_id = nft_id
    # Mint will be set when mark_as_minted is called with the actual mint
    nft_config.mint = authority.key()  # Placeholder - will be updated
    nft_config.name = name
    nft_config.symbol = symbol
    nft_config.uri = uri
    nft_config.is_minted = False
    nft_config.bump = bump


@instruction
def update_nft_config(
    authority: Signer,
    nft_config: NftConfig,
    name: str,
    symbol: str,
    uri: str
):
    """
    Update NFT configuration before minting.

    Once an NFT is minted, its on-chain metadata typically becomes immutable
    (depending on the is_mutable flag set during creation).
    """
    # Verify authority
    assert nft_config.authority == authority.key(), "Unauthorized"

    # Cannot update after minting
    assert not nft_config.is_minted, "NFT already minted"

    # Validate and update name if not empty
    if len(name) > 0:
        assert len(name) <= 32, "Name too long (max 32 characters)"
        nft_config.name = name

    # Validate and update symbol if not empty
    if len(symbol) > 0:
        assert len(symbol) <= 10, "Symbol too long (max 10 characters)"
        nft_config.symbol = symbol

    # Validate and update URI if not empty
    if len(uri) > 0:
        assert len(uri) <= 200, "URI too long (max 200 characters)"
        nft_config.uri = uri

    print("NFT config updated")


@instruction
def mark_as_minted(
    authority: Signer,
    mint: TokenMint,
    nft_config: NftConfig
):
    """
    Mark the NFT as minted and record the mint address.

    In a full Metaplex implementation, this step would:
    - Mint 1 token to the owner's token account
    - Create the metadata account
    - Create the master edition (supply = 0 for unique 1/1 NFT)

    This function simulates that step by:
    - Recording the mint address in the config
    - Setting the is_minted flag

    The actual token mint should be created externally (since Seahorse
    can only create mints via Empty[TokenMint].init, not Metaplex NFT mints).
    """
    # Verify authority
    assert nft_config.authority == authority.key(), "Unauthorized"

    # Cannot mint twice
    assert not nft_config.is_minted, "NFT already minted"

    # Store the mint address
    nft_config.mint = mint.key()
    nft_config.is_minted = True

    print("NFT marked as minted with mint:", mint.key())


@instruction
def get_nft_info(nft_config: NftConfig):
    """
    Display NFT configuration information.

    This is a read-only instruction that logs the NFT metadata.
    In practice, clients would read the account data directly.
    """
    print("NFT Config Info:")
    print("  Authority:", nft_config.authority)
    print("  Mint:", nft_config.mint)
    print("  Name:", nft_config.name)
    print("  Symbol:", nft_config.symbol)
    print("  URI:", nft_config.uri)
    print("  Is Minted:", nft_config.is_minted)
