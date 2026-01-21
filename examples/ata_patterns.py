# ata_patterns
# Built with Seahorse v0.1.0
#
# Demonstrates Token Account patterns including:
# - PDA-owned token accounts
# - Token transfers between accounts
# - User registration with token accounts
# - Treasury patterns with PDA authorities
#
# Note: This example uses PDA-derived token accounts rather than true ATAs
# (Associated Token Accounts) because Seahorse's `associated=True` parameter
# has a known bug. These PDA token accounts demonstrate similar patterns
# with program-controlled addresses.

from seahorse.prelude import *

declare_id('ATAPtrns1111111111111111111111111111111111AA')


class UserAccount(Account):
    # Owner of this user account
    owner: Pubkey
    # Mint for the user's registered token
    mint: Pubkey
    # User's token account address
    token_account: Pubkey
    # Bump for user account PDA
    bump: u8
    # Whether user is registered
    is_registered: bool


class Treasury(Account):
    # Admin who controls the treasury
    admin: Pubkey
    # Mint for the treasury
    mint: Pubkey
    # Treasury's token account address
    treasury_token_account: Pubkey
    # Bump for treasury PDA
    bump: u8


@instruction
def create_user_token_account(
    user: Signer,
    mint: TokenMint,
    user_token_account: Empty[TokenAccount]
):
    """
    Create a PDA token account for a user.
    Uses seeds to derive a deterministic address.
    """
    user_token_account.init(
        payer=user,
        seeds=['user_token', user, mint],
        mint=mint,
        authority=user
    )
    print('Created token account for user:', user.key())
    print('Token account address:', user_token_account.key())


@instruction
def transfer_tokens(
    sender: Signer,
    mint: TokenMint,
    sender_token_account: TokenAccount,
    recipient_token_account: TokenAccount,
    amount: u64
):
    """
    Transfer tokens from sender's account to recipient's account.
    Both accounts must already exist.
    """
    assert amount > 0, 'Amount must be greater than zero'

    sender_token_account.transfer(
        authority=sender,
        to=recipient_token_account,
        amount=amount
    )
    print('Transferred', amount, 'tokens')


@instruction
def register_user(
    user: Signer,
    mint: TokenMint,
    user_account: Empty[UserAccount],
    user_token_account: Empty[TokenAccount]
):
    """
    Register a user in the system and create their token account.
    Demonstrates combining custom state accounts with token account creation.
    """
    # Get bump before init
    bump = user_account.bump()

    # Initialize user account PDA
    user_account = user_account.init(
        payer=user,
        seeds=['user_account', user, mint]
    )

    # Initialize user's token account
    user_token_account = user_token_account.init(
        payer=user,
        seeds=['user_token', user, mint],
        mint=mint,
        authority=user
    )

    user_account.owner = user.key()
    user_account.mint = mint.key()
    user_account.token_account = user_token_account.key()
    user_account.bump = bump
    user_account.is_registered = True

    print('Registered user:', user.key())
    print('User token account:', user_token_account.key())


@instruction
def initialize_treasury(
    admin: Signer,
    mint: TokenMint,
    treasury: Empty[Treasury],
    treasury_token_account: Empty[TokenAccount]
):
    """
    Initialize a treasury with its token account.
    Treasury PDA owns the token account.
    """
    # Get bump before init
    bump = treasury.bump()

    # Initialize treasury PDA
    treasury = treasury.init(
        payer=admin,
        seeds=['treasury', admin, mint]
    )

    # Initialize treasury's token account with treasury PDA as authority
    treasury_token_account = treasury_token_account.init(
        payer=admin,
        seeds=['treasury_token', admin, mint],
        mint=mint,
        authority=treasury
    )

    treasury.admin = admin.key()
    treasury.mint = mint.key()
    treasury.treasury_token_account = treasury_token_account.key()
    treasury.bump = bump

    print('Initialized treasury with token account:', treasury_token_account.key())


@instruction
def airdrop_to_user(
    admin: Signer,
    mint: TokenMint,
    treasury: Treasury,
    treasury_token_account: TokenAccount,
    user_account: UserAccount,
    user_token_account: TokenAccount,
    amount: u64
):
    """
    Airdrop tokens to a registered user's token account.
    PDA authority signs the transfer.
    """
    assert amount > 0, 'Amount must be greater than zero'
    assert user_account.is_registered, 'User is not registered'

    # Transfer from treasury to user's token account using PDA signer
    bump = treasury.bump
    treasury_token_account.transfer(
        authority=treasury,
        to=user_token_account,
        amount=amount,
        signer=['treasury', admin, mint, bump]
    )

    print('Airdropped', amount, 'tokens to user')
