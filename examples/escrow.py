# escrow
# Built with Seahorse v0.1.0
#
# Demonstrates a two-party token escrow for trustless swaps.
# Features:
# - Maker deposits Token A and specifies desired Token B amount
# - Taker sends Token B to maker and receives Token A
# - Maker can cancel to get Token A back before taker accepts

from seahorse.prelude import *

declare_id('EScroW111111111111111111111111111111111111111')


class Escrow(Account):
    # Unique escrow identifier (allows multiple escrows)
    escrow_id: u64
    # The maker who created the escrow
    maker: Pubkey
    # Mint of the token being offered (Token A)
    mint_a: Pubkey
    # Mint of the token wanted (Token B)
    mint_b: Pubkey
    # Amount of Token A deposited in vault
    token_a_amount: u64
    # Amount of Token B wanted from taker
    token_b_wanted_amount: u64
    # Bump seed for escrow PDA
    bump: u8


@instruction
def make_escrow(
    maker: Signer,
    mint_a: TokenMint,
    mint_b: TokenMint,
    maker_token_account_a: TokenAccount,
    escrow: Empty[Escrow],
    vault: Empty[TokenAccount],
    escrow_id: u64,
    token_a_amount: u64,
    token_b_wanted_amount: u64
):
    # Validate amounts
    assert token_a_amount > 0, 'Amount must be greater than zero'
    assert token_b_wanted_amount > 0, 'Amount must be greater than zero'

    # Get bump before init
    bump = escrow.bump()

    # Initialize escrow state PDA - uses escrow_id for uniqueness
    escrow = escrow.init(
        payer=maker,
        seeds=['escrow', escrow_id]
    )

    # Initialize vault token account PDA with escrow as authority
    # Seeds use escrow_id for uniqueness (same PDA space)
    vault = vault.init(
        payer=maker,
        seeds=['vault', escrow_id],
        mint=mint_a,
        authority=escrow
    )

    # Transfer maker's tokens to vault
    maker_token_account_a.transfer(
        authority=maker,
        to=vault,
        amount=token_a_amount
    )

    # Set escrow state
    escrow.escrow_id = escrow_id
    escrow.maker = maker.key()
    escrow.mint_a = mint_a.key()
    escrow.mint_b = mint_b.key()
    escrow.token_a_amount = token_a_amount
    escrow.token_b_wanted_amount = token_b_wanted_amount
    escrow.bump = bump

    print('Escrow created:', escrow_id)


@instruction
def take_escrow(
    taker: Signer,
    maker: UncheckedAccount,
    mint_a: TokenMint,
    mint_b: TokenMint,
    taker_token_account_a: TokenAccount,
    taker_token_account_b: TokenAccount,
    maker_token_account_b: TokenAccount,
    escrow: Escrow,
    vault: TokenAccount
):
    # Verify escrow maker matches
    assert escrow.maker == maker.key(), 'Invalid maker'

    # Transfer Token B from taker to maker
    taker_token_account_b.transfer(
        authority=taker,
        to=maker_token_account_b,
        amount=escrow.token_b_wanted_amount
    )

    # Transfer Token A from vault to taker using escrow PDA signer
    # Seeds match escrow PDA derivation: ['escrow', escrow_id]
    bump = escrow.bump
    vault.transfer(
        authority=escrow,
        to=taker_token_account_a,
        amount=escrow.token_a_amount,
        signer=['escrow', escrow.escrow_id, bump]
    )

    print('Escrow completed:', escrow.escrow_id)


@instruction
def cancel_escrow(
    maker: Signer,
    mint_a: TokenMint,
    maker_token_account_a: TokenAccount,
    escrow: Escrow,
    vault: TokenAccount
):
    # Verify escrow maker
    assert escrow.maker == maker.key(), 'Unauthorized'

    # Transfer Token A from vault back to maker using escrow PDA signer
    bump = escrow.bump
    vault.transfer(
        authority=escrow,
        to=maker_token_account_a,
        amount=escrow.token_a_amount,
        signer=['escrow', escrow.escrow_id, bump]
    )

    print('Escrow cancelled:', escrow.escrow_id)
