# vesting
# Built with Seahorse v0.1.0
#
# Demonstrates token vesting with cliff and linear release schedule.
# Features:
# - Create vesting schedule with cliff period and linear vesting
# - Claim vested tokens after cliff period
# - Partial claims of vested tokens
# - Cancel vesting (authority only, returns unvested tokens)
# - Time-based calculation using slot numbers
#
# Design note: Vesting PDA uses beneficiary + mint for unique per-user per-token
# vesting schedules. Authority can create multiple vesting schedules for different
# beneficiaries.

from seahorse.prelude import *

declare_id('VESTngW9a1XxRgwWhmK7PNYWMFp4mW8q3xk2BgfCNih')


class VestingAccount(Account):
    # Authority who can cancel the vesting
    authority: Pubkey
    # Beneficiary who receives vested tokens
    beneficiary: Pubkey
    # Mint of the token being vested
    mint: Pubkey
    # Vault holding tokens
    vault: Pubkey
    # Total tokens to be vested
    total_amount: u64
    # Tokens already released
    released_amount: u64
    # Vesting start slot
    start_slot: u64
    # Cliff end slot (tokens unlock at this point)
    cliff_slot: u64
    # Full vesting end slot
    end_slot: u64
    # Is vesting cancelled
    is_cancelled: bool
    # Bump seed for PDA
    bump: u8


@instruction
def create_vesting(
    authority: Signer,
    beneficiary: Pubkey,
    mint: TokenMint,
    authority_token: TokenAccount,
    vesting: Empty[VestingAccount],
    vault: Empty[TokenAccount],
    clock: Clock,
    amount: u64,
    cliff_slots: u64,
    vesting_duration_slots: u64
):
    """
    Create a new vesting schedule.

    Args:
        authority: Creator and cancellation authority
        beneficiary: Recipient of vested tokens
        mint: Token mint
        authority_token: Authority's token account (source of tokens)
        vesting: Empty vesting account to initialize
        vault: Empty token account for vault
        clock: Clock sysvar for time
        amount: Total tokens to vest
        cliff_slots: Slots until cliff (0 for no cliff)
        vesting_duration_slots: Slots for linear vesting after cliff
    """
    # Validate inputs
    assert amount > 0, 'Amount must be greater than zero'
    assert vesting_duration_slots > 0, 'Vesting duration must be greater than zero'

    # Get bump before init
    vesting_bump = vesting.bump()

    # Initialize vesting account PDA
    vesting = vesting.init(
        payer=authority,
        seeds=['vesting', beneficiary, mint]
    )

    # Initialize vault PDA with vesting as authority
    vault = vault.init(
        payer=authority,
        seeds=['vesting_vault', beneficiary, mint],
        mint=mint,
        authority=vesting
    )

    # Get current slot
    current_slot = clock.slot()

    # Transfer tokens from authority to vault
    authority_token.transfer(
        authority=authority,
        to=vault,
        amount=amount
    )

    # Store vesting configuration
    vesting.authority = authority.key()
    vesting.beneficiary = beneficiary
    vesting.mint = mint.key()
    vesting.vault = vault.key()
    vesting.total_amount = amount
    vesting.released_amount = 0
    vesting.start_slot = current_slot
    vesting.cliff_slot = current_slot + cliff_slots
    vesting.end_slot = current_slot + cliff_slots + vesting_duration_slots
    vesting.is_cancelled = False
    vesting.bump = vesting_bump


@instruction
def claim_vested(
    beneficiary: Signer,
    vesting: VestingAccount,
    vault: TokenAccount,
    beneficiary_token: TokenAccount,
    mint: TokenMint,
    clock: Clock
):
    """
    Claim vested tokens.

    Calculates the amount of tokens that have vested and haven't been
    released yet, then transfers them to the beneficiary.
    """
    # Validate beneficiary
    assert beneficiary.key() == vesting.beneficiary, 'Unauthorized'

    # Cannot claim from cancelled vesting
    assert not vesting.is_cancelled, 'Vesting has been cancelled'

    # Get current slot
    current_slot = clock.slot()

    # Calculate vested amount
    vested_amount: u64 = 0

    if current_slot < vesting.cliff_slot:
        # Before cliff: nothing vested
        vested_amount = 0
    elif current_slot >= vesting.end_slot:
        # After end: everything vested
        vested_amount = vesting.total_amount
    else:
        # During vesting period: linear vesting
        # vested = total * (current - cliff) / (end - cliff)
        vesting_period = vesting.end_slot - vesting.cliff_slot
        elapsed = current_slot - vesting.cliff_slot
        vested_amount = vesting.total_amount * elapsed // vesting_period

    # Calculate releasable amount
    releasable = vested_amount - vesting.released_amount

    # Validate there are tokens to release
    assert releasable > 0, 'No tokens available to claim'

    # Get vesting bump for PDA signer
    vesting_bump = vesting.bump

    # Transfer tokens from vault to beneficiary using PDA signer
    vault.transfer(
        authority=vesting,
        to=beneficiary_token,
        amount=releasable,
        signer=['vesting', beneficiary, mint, vesting_bump]
    )

    # Update released amount
    vesting.released_amount += releasable


@instruction
def cancel_vesting(
    authority: Signer,
    vesting: VestingAccount,
    vault: TokenAccount,
    authority_token: TokenAccount,
    beneficiary_token: TokenAccount,
    mint: TokenMint,
    clock: Clock
):
    """
    Cancel vesting schedule.

    Authority can cancel vesting at any time. Vested tokens go to beneficiary,
    unvested tokens return to authority.
    """
    # Validate authority
    assert authority.key() == vesting.authority, 'Unauthorized'

    # Cannot cancel already cancelled vesting
    assert not vesting.is_cancelled, 'Vesting already cancelled'

    # Get current slot
    current_slot = clock.slot()

    # Calculate vested amount (same logic as claim)
    vested_amount: u64 = 0

    if current_slot < vesting.cliff_slot:
        vested_amount = 0
    elif current_slot >= vesting.end_slot:
        vested_amount = vesting.total_amount
    else:
        vesting_period = vesting.end_slot - vesting.cliff_slot
        elapsed = current_slot - vesting.cliff_slot
        vested_amount = vesting.total_amount * elapsed // vesting_period

    # Calculate amounts
    unreleased_vested = vested_amount - vesting.released_amount
    unvested = vesting.total_amount - vested_amount

    # Get vesting bump for PDA signer
    vesting_bump = vesting.bump
    beneficiary = vesting.beneficiary

    # Transfer vested tokens to beneficiary (if any)
    if unreleased_vested > 0:
        vault.transfer(
            authority=vesting,
            to=beneficiary_token,
            amount=unreleased_vested,
            signer=['vesting', beneficiary, mint, vesting_bump]
        )

    # Transfer unvested tokens back to authority (if any)
    if unvested > 0:
        vault.transfer(
            authority=vesting,
            to=authority_token,
            amount=unvested,
            signer=['vesting', beneficiary, mint, vesting_bump]
        )

    # Mark as cancelled
    vesting.is_cancelled = True
    vesting.released_amount = vested_amount
