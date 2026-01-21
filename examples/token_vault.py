# token_vault
# Built with Seahorse v0.1.0
#
# Demonstrates SPL Token integration with deposit/withdraw functionality.
# Features:
# - PDA-owned token account for secure custody
# - Deposit tokens from user to vault
# - Withdraw tokens from vault to owner (with PDA signer)

from seahorse.prelude import *

declare_id('3JwSuBw6X2q2FknhbhfvXnuFhdeJN9KCTtpGk6Qx9mLL')


class Vault(Account):
    # Owner who can withdraw from the vault
    owner: Pubkey
    # Mint of the tokens this vault holds
    mint: Pubkey
    # Total tokens deposited (tracked in vault state)
    total_deposits: u64
    # Bump for vault PDA (used for signing withdrawals)
    bump: u8


@instruction
def initialize_vault(
    owner: Signer,
    vault: Empty[Vault],
    vault_token_account: Empty[TokenAccount],
    mint: TokenMint
):
    # Get bump before init
    bump = vault.bump()

    # Initialize vault state PDA
    vault = vault.init(
        payer=owner,
        seeds=['vault', owner, mint]
    )

    # Initialize vault's token account PDA with vault as authority
    vault_token_account.init(
        payer=owner,
        seeds=['vault_token', owner, mint],
        mint=mint,
        authority=vault
    )

    vault.owner = owner.key()
    vault.mint = mint.key()
    vault.total_deposits = 0
    vault.bump = bump


@instruction
def deposit(
    user: Signer,
    vault: Vault,
    user_token_account: TokenAccount,
    vault_token_account: TokenAccount,
    mint: TokenMint,
    amount: u64
):
    # Validate amount
    assert amount > 0, 'Amount must be greater than zero'

    # Transfer tokens from user to vault
    # User is signer so no special signer seeds needed
    user_token_account.transfer(
        authority=user,
        to=vault_token_account,
        amount=amount
    )

    # Update vault state
    vault.total_deposits += amount


@instruction
def withdraw(
    owner: Signer,
    vault: Vault,
    owner_token_account: TokenAccount,
    vault_token_account: TokenAccount,
    mint: TokenMint,
    amount: u64
):
    # Check authorization
    assert owner.key() == vault.owner, 'Unauthorized'

    # Validate amount
    assert amount > 0, 'Amount must be greater than zero'

    # Check sufficient balance
    assert vault_token_account.amount() >= amount, 'Insufficient funds in vault'

    # Transfer tokens from vault to owner using PDA signer
    # vault is the authority of vault_token_account
    bump = vault.bump
    vault_token_account.transfer(
        authority=vault,
        to=owner_token_account,
        amount=amount,
        signer=['vault', owner, mint, bump]
    )

    # Update vault state
    vault.total_deposits -= amount
