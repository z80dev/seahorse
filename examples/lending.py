# lending
# Built with Seahorse v0.1.0
#
# Demonstrates a simplified lending protocol with collateralization.
# Features:
# - Deposit collateral to secure borrowing position
# - Borrow tokens against collateral (150% collateral ratio)
# - Repay borrowed amount with interest
# - Withdraw collateral (if collateralization requirements met)
# - Time-based interest calculation using slot numbers
#
# Design notes:
# - Pool PDA uses collateral_mint only for simpler PDA signer seeds
# - Interest is calculated per slot (simplified model)
# - Collateral ratio is 150% (must deposit 1.5x borrowed value)

from seahorse.prelude import *

declare_id('6LEndKVBqeRVFT6UYm5NN1SqBSCqCWvPaoYdJzrgLMge')

# Collateral ratio: 150% (must deposit 1.5x the value of borrowed amount)
# Stored as basis points: 15000 = 150%
COLLATERAL_RATIO_BPS = 15000
BPS_DENOMINATOR = 10000

# Interest rate scale factor (avoids floating point)
INTEREST_SCALE = 1000000


class Pool(Account):
    # Authority who created the pool
    authority: Pubkey
    # Mint of the collateral token
    collateral_mint: Pubkey
    # Mint of the borrowable token
    borrow_mint: Pubkey
    # Token account holding collateral
    collateral_vault: Pubkey
    # Token account holding borrowable tokens
    borrow_vault: Pubkey
    # Interest rate per slot (scaled by INTEREST_SCALE)
    interest_rate: u64
    # Total collateral deposited
    total_deposited: u64
    # Total amount borrowed
    total_borrowed: u64
    # Last slot when pool was updated
    last_update_slot: u64
    # Bump seed for pool PDA
    bump: u8


class UserPosition(Account):
    # Owner of this position
    owner: Pubkey
    # Pool this position belongs to
    pool: Pubkey
    # Collateral deposited by user
    collateral_deposited: u64
    # Amount borrowed by user (including accrued interest)
    borrowed_amount: u64
    # Last slot when position was updated
    last_update_slot: u64
    # Bump seed for position PDA
    bump: u8


@instruction
def initialize_pool(
    authority: Signer,
    collateral_mint: TokenMint,
    borrow_mint: TokenMint,
    pool: Empty[Pool],
    collateral_vault: Empty[TokenAccount],
    borrow_vault: Empty[TokenAccount],
    clock: Clock,
    interest_rate: u64
):
    # Get bump before init
    pool_bump = pool.bump()

    # Initialize pool state PDA - uses collateral_mint only for simpler PDA signer seeds
    pool = pool.init(
        payer=authority,
        seeds=['pool', collateral_mint]
    )

    # Initialize collateral vault PDA with pool as authority
    collateral_vault.init(
        payer=authority,
        seeds=['collateral_vault', collateral_mint],
        mint=collateral_mint,
        authority=pool
    )

    # Initialize borrow vault PDA with pool as authority
    borrow_vault.init(
        payer=authority,
        seeds=['borrow_vault', collateral_mint],
        mint=borrow_mint,
        authority=pool
    )

    # Get current slot from clock sysvar
    current_slot = clock.slot()

    # Store pool configuration
    pool.authority = authority.key()
    pool.collateral_mint = collateral_mint.key()
    pool.borrow_mint = borrow_mint.key()
    pool.collateral_vault = collateral_vault.key()
    pool.borrow_vault = borrow_vault.key()
    pool.interest_rate = interest_rate
    pool.total_deposited = 0
    pool.total_borrowed = 0
    pool.last_update_slot = current_slot
    pool.bump = pool_bump


@instruction
def create_user_position(
    user: Signer,
    pool: Pool,
    user_position: Empty[UserPosition],
    collateral_mint: TokenMint,
    clock: Clock
):
    # Get bump before init
    position_bump = user_position.bump()

    # Initialize user position PDA - uses collateral_mint + user for seeds
    user_position = user_position.init(
        payer=user,
        seeds=['user_position', collateral_mint, user]
    )

    # Get current slot from clock sysvar
    current_slot = clock.slot()

    # Initialize position fields
    user_position.owner = user.key()
    user_position.pool = pool.key()
    user_position.collateral_deposited = 0
    user_position.borrowed_amount = 0
    user_position.last_update_slot = current_slot
    user_position.bump = position_bump


@instruction
def deposit_collateral(
    user: Signer,
    pool: Pool,
    user_position: UserPosition,
    user_collateral_token: TokenAccount,
    collateral_vault: TokenAccount,
    collateral_mint: TokenMint,
    amount: u64
):
    # Validate amount
    assert amount > 0, 'Amount must be greater than zero'

    # Validate ownership
    assert user.key() == user_position.owner, 'Unauthorized'

    # Transfer collateral from user to vault
    user_collateral_token.transfer(
        authority=user,
        to=collateral_vault,
        amount=amount
    )

    # Update user position
    user_position.collateral_deposited += amount

    # Update pool total
    pool.total_deposited += amount


@instruction
def borrow(
    user: Signer,
    pool: Pool,
    user_position: UserPosition,
    user_borrow_token: TokenAccount,
    borrow_vault: TokenAccount,
    collateral_mint: TokenMint,
    clock: Clock,
    amount: u64
):
    # Validate amount
    assert amount > 0, 'Amount must be greater than zero'

    # Validate ownership
    assert user.key() == user_position.owner, 'Unauthorized'

    # Get current slot from clock sysvar
    current_slot = clock.slot()

    # Calculate accrued interest on existing debt
    slots_elapsed = current_slot - user_position.last_update_slot
    accrued_interest = user_position.borrowed_amount * pool.interest_rate * slots_elapsed // INTEREST_SCALE

    # Calculate new total debt
    new_debt = user_position.borrowed_amount + accrued_interest + amount

    # Check collateral ratio
    # Required collateral = borrowed * COLLATERAL_RATIO_BPS / BPS_DENOMINATOR
    required_collateral = new_debt * COLLATERAL_RATIO_BPS // BPS_DENOMINATOR

    assert user_position.collateral_deposited >= required_collateral, 'Insufficient collateral'

    # Check pool has enough liquidity
    assert borrow_vault.amount() >= amount, 'Insufficient liquidity'

    # Get pool bump for signer
    pool_bump = pool.bump

    # Transfer borrowed tokens to user using PDA signer
    borrow_vault.transfer(
        authority=pool,
        to=user_borrow_token,
        amount=amount,
        signer=['pool', collateral_mint, pool_bump]
    )

    # Update user position
    user_position.borrowed_amount = new_debt
    user_position.last_update_slot = current_slot

    # Update pool
    pool.total_borrowed += amount
    pool.last_update_slot = current_slot


@instruction
def repay(
    user: Signer,
    pool: Pool,
    user_position: UserPosition,
    user_borrow_token: TokenAccount,
    borrow_vault: TokenAccount,
    collateral_mint: TokenMint,
    clock: Clock,
    amount: u64
):
    # Validate amount
    assert amount > 0, 'Amount must be greater than zero'

    # Validate ownership
    assert user.key() == user_position.owner, 'Unauthorized'

    # Get current slot from clock sysvar
    current_slot = clock.slot()

    # Calculate accrued interest
    slots_elapsed = current_slot - user_position.last_update_slot
    accrued_interest = user_position.borrowed_amount * pool.interest_rate * slots_elapsed // INTEREST_SCALE

    # Calculate total debt
    total_debt = user_position.borrowed_amount + accrued_interest

    # Cannot repay more than total debt
    repay_amount = amount
    if repay_amount > total_debt:
        repay_amount = total_debt

    # Transfer tokens from user to vault
    user_borrow_token.transfer(
        authority=user,
        to=borrow_vault,
        amount=repay_amount
    )

    # Update user position
    user_position.borrowed_amount = total_debt - repay_amount
    user_position.last_update_slot = current_slot

    # Update pool (reduce total_borrowed by the principal portion for simplicity)
    if repay_amount <= pool.total_borrowed:
        pool.total_borrowed -= repay_amount
    else:
        pool.total_borrowed = 0
    pool.last_update_slot = current_slot


@instruction
def withdraw_collateral(
    user: Signer,
    pool: Pool,
    user_position: UserPosition,
    user_collateral_token: TokenAccount,
    collateral_vault: TokenAccount,
    collateral_mint: TokenMint,
    clock: Clock,
    amount: u64
):
    # Validate amount
    assert amount > 0, 'Amount must be greater than zero'

    # Validate ownership
    assert user.key() == user_position.owner, 'Unauthorized'

    # Validate sufficient collateral
    assert user_position.collateral_deposited >= amount, 'Insufficient collateral'

    # Get current slot from clock sysvar
    current_slot = clock.slot()

    # Calculate accrued interest
    slots_elapsed = current_slot - user_position.last_update_slot
    accrued_interest = user_position.borrowed_amount * pool.interest_rate * slots_elapsed // INTEREST_SCALE

    # Calculate total debt
    total_debt = user_position.borrowed_amount + accrued_interest

    # Check remaining collateral maintains ratio
    remaining_collateral = user_position.collateral_deposited - amount

    if total_debt > 0:
        required_collateral = total_debt * COLLATERAL_RATIO_BPS // BPS_DENOMINATOR
        assert remaining_collateral >= required_collateral, 'Insufficient collateral after withdrawal'

    # Get pool bump for signer
    pool_bump = pool.bump

    # Transfer collateral back to user using PDA signer
    collateral_vault.transfer(
        authority=pool,
        to=user_collateral_token,
        amount=amount,
        signer=['pool', collateral_mint, pool_bump]
    )

    # Update user position
    user_position.collateral_deposited = remaining_collateral
    user_position.borrowed_amount = total_debt
    user_position.last_update_slot = current_slot

    # Update pool
    pool.total_deposited -= amount
    pool.last_update_slot = current_slot
