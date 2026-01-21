# nft_staking
# Built with Seahorse v0.1.0
#
# Demonstrates NFT staking with time-based reward tokens.
# Features:
# - Initialize NFT staking pool with configurable reward rate
# - Stake NFT (transfer to PDA-owned vault)
# - Unstake NFT (return NFT and claim rewards)
# - Claim rewards without unstaking
# - Time-based reward calculation using slot numbers
#
# Design notes:
# - Pool PDA uses reward_mint only for simpler signer seeds
# - StakeRecord PDA uses nft_mint only (each NFT has one stake record)
# - StakeRecord is the vault authority (for transfer back on unstake)
# - Pool is the reward mint authority (for minting rewards)

from seahorse.prelude import *

declare_id('8DtVbTgYbcM8b2igVzb7RjbVKLLLDQ2bzXriSFp8fxhb')

# Scale factor for reward calculations (avoids floating point)
REWARD_SCALE = 1000000


class Pool(Account):
    # Authority who created the pool
    authority: Pubkey
    # Mint of the reward token (pool is mint authority)
    reward_mint: Pubkey
    # Reward tokens per NFT per slot (scaled by REWARD_SCALE)
    reward_rate: u64
    # Total NFTs staked in pool
    total_staked: u64
    # Last slot when pool was updated
    last_update_slot: u64
    # Bump seed for pool PDA
    bump: u8


class StakeRecord(Account):
    # Owner who staked the NFT
    owner: Pubkey
    # Pool this stake belongs to
    pool: Pubkey
    # Mint of the staked NFT
    nft_mint: Pubkey
    # Vault holding the NFT
    nft_vault: Pubkey
    # Slot when NFT was staked
    staked_at_slot: u64
    # Slot when rewards were last claimed
    last_claim_slot: u64
    # Pending rewards accumulated
    pending_rewards: u64
    # Whether NFT is currently staked
    is_staked: bool
    # Bump seed for stake record PDA
    bump: u8


@instruction
def initialize_pool(
    authority: Signer,
    reward_mint: TokenMint,
    pool: Empty[Pool],
    clock: Clock,
    reward_rate: u64
):
    # Get bump before init
    pool_bump = pool.bump()

    # Initialize pool state PDA - uses reward_mint only for simpler signer seeds
    pool = pool.init(
        payer=authority,
        seeds=['pool', reward_mint]
    )

    # Get current slot from clock sysvar
    current_slot = clock.slot()

    # Store pool configuration
    pool.authority = authority.key()
    pool.reward_mint = reward_mint.key()
    pool.reward_rate = reward_rate
    pool.total_staked = 0
    pool.last_update_slot = current_slot
    pool.bump = pool_bump


@instruction
def stake_nft(
    user: Signer,
    pool: Pool,
    nft_mint: TokenMint,
    user_nft_token: TokenAccount,
    nft_vault: Empty[TokenAccount],
    stake_record: Empty[StakeRecord],
    clock: Clock
):
    # Validate user owns the NFT
    assert user_nft_token.amount() == 1, 'User does not own the NFT'

    # Get bump before init
    stake_bump = stake_record.bump()

    # Initialize stake record PDA - uses nft_mint only (each NFT has one record)
    stake_record = stake_record.init(
        payer=user,
        seeds=['stake_record', nft_mint]
    )

    # Initialize NFT vault PDA with stake_record as authority
    # This allows stake_record to sign the transfer back on unstake
    nft_vault = nft_vault.init(
        payer=user,
        seeds=['nft_vault', nft_mint],
        mint=nft_mint,
        authority=stake_record
    )

    # Get current slot from clock sysvar
    current_slot = clock.slot()

    # Initialize stake record fields
    stake_record.owner = user.key()
    stake_record.pool = pool.key()
    stake_record.nft_mint = nft_mint.key()
    stake_record.nft_vault = nft_vault.key()
    stake_record.staked_at_slot = current_slot
    stake_record.last_claim_slot = current_slot
    stake_record.pending_rewards = 0
    stake_record.is_staked = True
    stake_record.bump = stake_bump

    # Transfer NFT from user to vault
    user_nft_token.transfer(
        authority=user,
        to=nft_vault,
        amount=u64(1)
    )

    # Update pool total
    pool.total_staked += 1
    pool.last_update_slot = current_slot


@instruction
def unstake_nft(
    user: Signer,
    pool: Pool,
    reward_mint: TokenMint,
    nft_mint: TokenMint,
    stake_record: StakeRecord,
    nft_vault: TokenAccount,
    user_nft_token: TokenAccount,
    user_reward_token: TokenAccount,
    clock: Clock
):
    # Validate stake record
    assert stake_record.is_staked, 'NFT not staked'
    assert user.key() == stake_record.owner, 'Unauthorized'

    # Get current slot from clock sysvar
    current_slot = clock.slot()

    # Calculate pending rewards
    slots_staked = current_slot - stake_record.last_claim_slot
    pending = pool.reward_rate * slots_staked // REWARD_SCALE
    total_rewards = stake_record.pending_rewards + pending

    # Get stake record bump for signer
    stake_bump = stake_record.bump

    # Transfer NFT back to user using stake_record PDA signer
    # Signer seeds match stake_record PDA: ['stake_record', nft_mint, bump]
    nft_vault.transfer(
        authority=stake_record,
        to=user_nft_token,
        amount=u64(1),
        signer=['stake_record', nft_mint, stake_bump]
    )

    # Mint reward tokens to user if any
    if total_rewards > 0:
        # Get pool bump for signer - pool is reward mint authority
        pool_bump = pool.bump
        reward_mint.mint(
            authority=pool,
            to=user_reward_token,
            amount=total_rewards,
            signer=['pool', reward_mint, pool_bump]
        )

    # Update stake record
    stake_record.is_staked = False
    stake_record.pending_rewards = 0
    stake_record.last_claim_slot = current_slot

    # Update pool total
    pool.total_staked -= 1
    pool.last_update_slot = current_slot


@instruction
def claim_rewards(
    user: Signer,
    pool: Pool,
    reward_mint: TokenMint,
    nft_mint: TokenMint,
    stake_record: StakeRecord,
    user_reward_token: TokenAccount,
    clock: Clock
):
    # Validate stake record
    assert stake_record.is_staked, 'NFT not staked'
    assert user.key() == stake_record.owner, 'Unauthorized'

    # Get current slot from clock sysvar
    current_slot = clock.slot()

    # Calculate pending rewards
    slots_staked = current_slot - stake_record.last_claim_slot
    pending = pool.reward_rate * slots_staked // REWARD_SCALE
    total_rewards = stake_record.pending_rewards + pending

    # Validate rewards available
    assert total_rewards > 0, 'No rewards to claim'

    # Get pool bump for signer - pool is reward mint authority
    pool_bump = pool.bump

    # Mint reward tokens to user using pool PDA signer
    # Signer seeds match pool PDA: ['pool', reward_mint, bump]
    reward_mint.mint(
        authority=pool,
        to=user_reward_token,
        amount=total_rewards,
        signer=['pool', reward_mint, pool_bump]
    )

    # Update stake record
    stake_record.pending_rewards = 0
    stake_record.last_claim_slot = current_slot
