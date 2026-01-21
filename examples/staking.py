# staking
# Built with Seahorse v0.1.0
#
# Demonstrates token staking with time-based rewards.
# Features:
# - Initialize staking pool with configurable reward rate
# - Stake tokens into pool
# - Unstake tokens and claim rewards
# - Claim rewards without unstaking
# - Time-based reward calculation using slot numbers
#
# Design note: Pool PDA uses stake_mint only (not authority) so any user can
# call unstake/claim without needing to pass authority as UncheckedAccount.
# Authority key is still stored in pool for validation purposes.

from seahorse.prelude import *

declare_id('HFQhM2FYhiP1mbpVFnpHZ7hJf5xFzekJAjqoxexvKA5Q')

# Scale factor for reward calculations (avoids floating point)
REWARD_SCALE = 1000000


class Pool(Account):
    # Authority who created the pool
    authority: Pubkey
    # Mint of the token being staked
    stake_mint: Pubkey
    # Mint of the reward token (pool is mint authority)
    reward_mint: Pubkey
    # Token account holding staked tokens
    stake_vault: Pubkey
    # Reward tokens per staked token per slot (scaled by REWARD_SCALE)
    reward_rate: u64
    # Total tokens staked in pool
    total_staked: u64
    # Last slot when pool was updated
    last_update_slot: u64
    # Bump seed for pool PDA
    bump: u8


class UserStake(Account):
    # Owner of this stake account
    owner: Pubkey
    # Pool this stake belongs to
    pool: Pubkey
    # Amount of tokens staked
    staked_amount: u64
    # Pending rewards accumulated
    pending_rewards: u64
    # Slot when user last staked/claimed
    last_stake_slot: u64
    # Bump seed for user stake PDA
    bump: u8


@instruction
def initialize_pool(
    authority: Signer,
    stake_mint: TokenMint,
    reward_mint: TokenMint,
    pool: Empty[Pool],
    stake_vault: Empty[TokenAccount],
    clock: Clock,
    reward_rate: u64
):
    # Get bump before init
    pool_bump = pool.bump()

    # Initialize pool state PDA - uses stake_mint only for simpler PDA signer seeds
    pool = pool.init(
        payer=authority,
        seeds=['pool', stake_mint]
    )

    # Initialize stake vault PDA with pool as authority
    stake_vault.init(
        payer=authority,
        seeds=['stake_vault', stake_mint],
        mint=stake_mint,
        authority=pool
    )

    # Get current slot from clock sysvar
    current_slot = clock.slot()

    # Store pool configuration
    pool.authority = authority.key()
    pool.stake_mint = stake_mint.key()
    pool.reward_mint = reward_mint.key()
    pool.stake_vault = stake_vault.key()
    pool.reward_rate = reward_rate
    pool.total_staked = 0
    pool.last_update_slot = current_slot
    pool.bump = pool_bump


@instruction
def create_user_stake(
    user: Signer,
    pool: Pool,
    user_stake: Empty[UserStake],
    stake_mint: TokenMint,
    clock: Clock
):
    # Get bump before init
    stake_bump = user_stake.bump()

    # Initialize user stake account PDA - use stake_mint instead of pool for seeds
    user_stake = user_stake.init(
        payer=user,
        seeds=['user_stake', stake_mint, user]
    )

    # Get current slot from clock sysvar
    current_slot = clock.slot()

    # Initialize user stake fields
    user_stake.owner = user.key()
    user_stake.pool = pool.key()
    user_stake.staked_amount = 0
    user_stake.pending_rewards = 0
    user_stake.last_stake_slot = current_slot
    user_stake.bump = stake_bump


@instruction
def stake(
    user: Signer,
    pool: Pool,
    user_stake: UserStake,
    user_stake_token: TokenAccount,
    stake_vault: TokenAccount,
    stake_mint: TokenMint,
    clock: Clock,
    amount: u64
):
    # Validate amount
    assert amount > 0, 'Amount must be greater than zero'

    # Validate ownership
    assert user.key() == user_stake.owner, 'Unauthorized'

    # Get current slot from clock sysvar
    current_slot = clock.slot()

    # If user has existing stake, calculate pending rewards
    if user_stake.staked_amount > 0:
        slots_staked = current_slot - user_stake.last_stake_slot
        pending = user_stake.staked_amount * pool.reward_rate * slots_staked // REWARD_SCALE
        user_stake.pending_rewards += pending

    # Transfer stake tokens from user to vault
    user_stake_token.transfer(
        authority=user,
        to=stake_vault,
        amount=amount
    )

    # Update user stake
    user_stake.staked_amount += amount
    user_stake.last_stake_slot = current_slot

    # Update pool total
    pool.total_staked += amount
    pool.last_update_slot = current_slot


@instruction
def unstake(
    user: Signer,
    pool: Pool,
    reward_mint: TokenMint,
    user_stake: UserStake,
    user_stake_token: TokenAccount,
    user_reward_token: TokenAccount,
    stake_vault: TokenAccount,
    stake_mint: TokenMint,
    clock: Clock,
    amount: u64
):
    # Validate amount
    assert amount > 0, 'Amount must be greater than zero'

    # Validate ownership
    assert user.key() == user_stake.owner, 'Unauthorized'

    # Validate sufficient stake
    assert user_stake.staked_amount >= amount, 'Insufficient staked amount'

    # Get current slot from clock sysvar
    current_slot = clock.slot()

    # Calculate pending rewards
    slots_staked = current_slot - user_stake.last_stake_slot
    pending = user_stake.staked_amount * pool.reward_rate * slots_staked // REWARD_SCALE
    total_rewards = user_stake.pending_rewards + pending

    # Get pool bump for signer - using stake_mint from pool's stored value
    pool_bump = pool.bump

    # Transfer staked tokens back to user using PDA signer
    # Signer seeds match pool PDA: ['pool', stake_mint, bump]
    stake_vault.transfer(
        authority=pool,
        to=user_stake_token,
        amount=amount,
        signer=['pool', stake_mint, pool_bump]
    )

    # Mint reward tokens to user if any
    if total_rewards > 0:
        reward_mint.mint(
            authority=pool,
            to=user_reward_token,
            amount=total_rewards,
            signer=['pool', stake_mint, pool_bump]
        )

    # Update user stake
    user_stake.staked_amount -= amount
    user_stake.pending_rewards = 0
    user_stake.last_stake_slot = current_slot

    # Update pool total
    pool.total_staked -= amount
    pool.last_update_slot = current_slot


@instruction
def claim_rewards(
    user: Signer,
    pool: Pool,
    reward_mint: TokenMint,
    user_stake: UserStake,
    user_reward_token: TokenAccount,
    stake_mint: TokenMint,
    clock: Clock
):
    # Validate ownership
    assert user.key() == user_stake.owner, 'Unauthorized'

    # Validate user has stake
    assert user_stake.staked_amount > 0, 'No tokens staked'

    # Get current slot from clock sysvar
    current_slot = clock.slot()

    # Calculate pending rewards
    slots_staked = current_slot - user_stake.last_stake_slot
    pending = user_stake.staked_amount * pool.reward_rate * slots_staked // REWARD_SCALE
    total_rewards = user_stake.pending_rewards + pending

    # Validate rewards available
    assert total_rewards > 0, 'No rewards to claim'

    # Get pool bump for signer
    pool_bump = pool.bump

    # Mint reward tokens to user using PDA signer
    # Signer seeds match pool PDA: ['pool', stake_mint, bump]
    reward_mint.mint(
        authority=pool,
        to=user_reward_token,
        amount=total_rewards,
        signer=['pool', stake_mint, pool_bump]
    )

    # Update user stake
    user_stake.pending_rewards = 0
    user_stake.last_stake_slot = current_slot
