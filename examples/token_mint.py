# token_mint
# Built with Seahorse v0.1.0
#
# Demonstrates token minting and burning with PDA as mint authority.
# Features:
# - Create a new SPL Token mint with mint_config PDA as authority
# - Mint new tokens to any token account (authority only)
# - Burn tokens from token accounts (token owner)

from seahorse.prelude import *

declare_id('MiNT5VzVZYxLYtDmJq5DEmUKDEcPnP2zGxqHKbv2PgZ')


class MintConfig(Account):
    # The mint PDA address
    mint: Pubkey
    # Authority who can mint tokens
    authority: Pubkey
    # Token decimals
    decimals: u8
    # Total tokens minted (for tracking)
    total_minted: u64
    # Total tokens burned (for tracking)
    total_burned: u64
    # Bump for mint_config PDA
    bump: u8


@instruction
def create_mint(
    authority: Signer,
    mint_config: Empty[MintConfig],
    mint: Empty[TokenMint],
    decimals: u8
):
    # Get bump before init
    config_bump = mint_config.bump()

    # Initialize mint config PDA first
    mint_config = mint_config.init(
        payer=authority,
        seeds=['mint_config', authority]
    )

    # Initialize the token mint PDA
    # mint_config is the mint authority (program-controlled)
    mint.init(
        payer=authority,
        seeds=['mint', authority],
        decimals=decimals,
        authority=mint_config
    )

    # Store configuration
    mint_config.mint = mint.key()
    mint_config.authority = authority.key()
    mint_config.decimals = decimals
    mint_config.total_minted = 0
    mint_config.total_burned = 0
    mint_config.bump = config_bump

    print('Created mint:', mint.key())
    print('Decimals:', decimals)


@instruction
def mint_tokens(
    authority: Signer,
    mint_config: MintConfig,
    mint: TokenMint,
    destination: TokenAccount,
    amount: u64
):
    # Validate authority
    assert authority.key() == mint_config.authority, 'Unauthorized'

    # Validate amount
    assert amount > 0, 'Amount must be greater than zero'

    # Mint tokens using mint_config PDA as signer
    # mint_config is the mint authority
    bump = mint_config.bump
    mint.mint(
        authority=mint_config,
        to=destination,
        amount=amount,
        signer=['mint_config', authority, bump]
    )

    # Update tracking
    mint_config.total_minted += amount

    print('Minted', amount, 'tokens')


@instruction
def burn_tokens(
    owner: Signer,
    authority: UncheckedAccount,
    mint_config: MintConfig,
    mint: TokenMint,
    source: TokenAccount,
    amount: u64
):
    # Validate amount
    assert amount > 0, 'Amount must be greater than zero'

    # Validate sufficient balance
    assert source.amount() >= amount, 'Insufficient funds'

    # Burn tokens using TokenMint.burn()
    # owner must be the token account authority (holder)
    mint.burn(
        authority=owner,
        holder=source,
        amount=amount
    )

    # Update tracking
    mint_config.total_burned += amount

    print('Burned', amount, 'tokens')
