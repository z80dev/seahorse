//! LiteSVM integration tests for the Seahorse Staking program
//!
//! These tests verify token staking with time-based rewards:
//! - initialize_pool: Creates staking pool with reward rate
//! - create_user_stake: Creates user stake account
//! - stake: Deposits tokens into pool
//! - unstake: Withdraws tokens and claims rewards
//! - claim_rewards: Claims pending rewards without unstaking
//!
//! Uses LiteSVM's warp_to_slot for time-based reward testing.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile staking.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// Staking program ID (from declare_id!)
fn staking_program_id() -> Pubkey {
    Pubkey::from_str("HFQhM2FYhiP1mbpVFnpHZ7hJf5xFzekJAjqoxexvKA5Q").unwrap()
}

/// Pool account size: discriminator (8) + authority (32) + stake_mint (32) + reward_mint (32)
///                    + stake_vault (32) + reward_rate (8) + total_staked (8) + last_update_slot (8) + bump (1)
const POOL_SIZE: usize = 8 + 32 + 32 + 32 + 32 + 8 + 8 + 8 + 1;

/// UserStake account size: discriminator (8) + owner (32) + pool (32) + staked_amount (8)
///                         + pending_rewards (8) + last_stake_slot (8) + bump (1)
const USER_STAKE_SIZE: usize = 8 + 32 + 32 + 8 + 8 + 8 + 1;

/// Scale factor for reward calculations (same as Seahorse program)
const REWARD_SCALE: u64 = 1_000_000;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the staking program into LiteSVM
fn load_staking_program() -> litesvm::LiteSVM {
    let program_id = staking_program_id();
    let program_bytes = std::fs::read("../../target/deploy/staking.so")
        .expect("Failed to read staking.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);
    svm
}

/// Setup a mint and mint authority for testing
fn setup_mint(svm: &mut litesvm::LiteSVM, decimals: u8) -> (Keypair, Pubkey) {
    let mint_authority = funded_keypair_10_sol(svm);
    let mint = Keypair::new();

    let mint_account = create_mint_account(&mint_authority.pubkey(), decimals);
    svm.set_account(mint.pubkey(), mint_account).unwrap();

    (mint_authority, mint.pubkey())
}

/// Setup a mint with a specific authority (PDA) - used for reward mint
fn setup_mint_with_authority(svm: &mut litesvm::LiteSVM, authority: &Pubkey, decimals: u8) -> Pubkey {
    let mint = Keypair::new();

    let mint_account = create_mint_account(authority, decimals);
    svm.set_account(mint.pubkey(), mint_account).unwrap();

    mint.pubkey()
}

/// Setup a user with an ATA containing tokens
fn setup_user_with_tokens(
    svm: &mut litesvm::LiteSVM,
    mint: &Pubkey,
    amount: u64,
) -> (Keypair, Pubkey) {
    let user = funded_keypair_10_sol(svm);
    let user_ata = get_ata(&user.pubkey(), mint);

    let token_account = create_token_account(&user.pubkey(), mint, amount);
    svm.set_account(user_ata, token_account).unwrap();

    (user, user_ata)
}

/// Setup a user with empty token account
fn setup_user_with_empty_token_account(
    svm: &mut litesvm::LiteSVM,
    mint: &Pubkey,
) -> (Keypair, Pubkey) {
    setup_user_with_tokens(svm, mint, 0)
}

/// Derive pool PDA
fn derive_pool_pda(stake_mint: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"pool", stake_mint.as_ref()],
        &staking_program_id(),
    )
}

/// Derive stake vault PDA
fn derive_stake_vault_pda(stake_mint: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"stake_vault", stake_mint.as_ref()],
        &staking_program_id(),
    )
}

/// Derive user stake PDA
fn derive_user_stake_pda(stake_mint: &Pubkey, user: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"user_stake", stake_mint.as_ref(), user.as_ref()],
        &staking_program_id(),
    )
}

/// Read pool fields from account data
fn read_pool_authority(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

fn read_pool_stake_mint(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[40..72].try_into().unwrap())
}

fn read_pool_reward_mint(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[72..104].try_into().unwrap())
}

fn read_pool_total_staked(data: &[u8]) -> u64 {
    // Offset: 8 (disc) + 32 (authority) + 32 (stake_mint) + 32 (reward_mint) + 32 (stake_vault) + 8 (reward_rate) = 144
    let bytes: [u8; 8] = data[144..152].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_pool_reward_rate(data: &[u8]) -> u64 {
    // Offset: 8 (disc) + 32 (authority) + 32 (stake_mint) + 32 (reward_mint) + 32 (stake_vault) = 136
    let bytes: [u8; 8] = data[136..144].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

/// Read user stake fields from account data
fn read_user_stake_owner(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

fn read_user_stake_staked_amount(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[72..80].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_user_stake_pending_rewards(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[80..88].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_user_stake_last_slot(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[88..96].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

/// Build initialize_pool instruction data
fn initialize_pool_data(reward_rate: u64) -> Vec<u8> {
    reward_rate.to_le_bytes().to_vec()
}

/// Build stake instruction data
fn stake_data(amount: u64) -> Vec<u8> {
    amount.to_le_bytes().to_vec()
}

/// Build unstake instruction data
fn unstake_data(amount: u64) -> Vec<u8> {
    amount.to_le_bytes().to_vec()
}

// =============================================================================
// INITIALIZE POOL TESTS
// =============================================================================

#[test]
fn test_initialize_pool() {
    let mut svm = load_staking_program();
    let program_id = staking_program_id();

    // Setup stake mint
    let (_, stake_mint) = setup_mint(&mut svm, 6);

    // Setup pool authority
    let authority = funded_keypair_10_sol(&mut svm);

    // Derive PDAs
    let (pool_pda, _) = derive_pool_pda(&stake_mint);
    let (stake_vault_pda, _) = derive_stake_vault_pda(&stake_mint);

    // Setup reward mint with pool as authority
    let reward_mint = setup_mint_with_authority(&mut svm, &pool_pda, 6);

    let reward_rate: u64 = 1000; // 1000 reward tokens per staked token per slot / REWARD_SCALE

    // Build initialize_pool instruction (Seahorse account order)
    let ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(reward_rate),
        vec![
            signer_meta(authority.pubkey()),      // authority
            writable_meta(stake_mint),            // stake_mint
            writable_meta(reward_mint),           // reward_mint
            writable_meta(pool_pda),              // pool
            writable_meta(stake_vault_pda),       // stake_vault
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(solana_sdk_ids::sysvar::rent::id()), // rent
            readonly_meta(system_program::id()), // system_program
            readonly_meta(token_program_id()),   // token_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "initialize_pool should succeed: {:?}", result);

    // Verify pool state
    let pool_account = svm.get_account(&pool_pda).expect("Pool should exist");
    assert_eq!(pool_account.owner, program_id, "Pool should be owned by program");

    assert_eq!(read_pool_authority(&pool_account.data), authority.pubkey());
    assert_eq!(read_pool_stake_mint(&pool_account.data), stake_mint);
    assert_eq!(read_pool_reward_mint(&pool_account.data), reward_mint);
    assert_eq!(read_pool_total_staked(&pool_account.data), 0);
    assert_eq!(read_pool_reward_rate(&pool_account.data), reward_rate);

    // Verify stake vault exists
    let vault_account = svm.get_account(&stake_vault_pda).expect("Stake vault should exist");
    assert_eq!(vault_account.owner, token_program_id());
}

// =============================================================================
// STAKE TESTS
// =============================================================================

#[test]
fn test_stake() {
    let mut svm = load_staking_program();
    let program_id = staking_program_id();

    // Setup
    let (_, stake_mint) = setup_mint(&mut svm, 6);
    let authority = funded_keypair_10_sol(&mut svm);
    let (pool_pda, _) = derive_pool_pda(&stake_mint);
    let (stake_vault_pda, _) = derive_stake_vault_pda(&stake_mint);
    let reward_mint = setup_mint_with_authority(&mut svm, &pool_pda, 6);

    // Initialize pool
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(1000),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(stake_mint),
            writable_meta(reward_mint),
            writable_meta(pool_pda),
            writable_meta(stake_vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Setup user with stake tokens
    let (user, user_stake_token) = setup_user_with_tokens(&mut svm, &stake_mint, 1000);
    let (user_stake_pda, _) = derive_user_stake_pda(&stake_mint, &user.pubkey());

    // Create user stake account
    svm.expire_blockhash();
    let create_stake_ix = anchor_instruction(
        program_id,
        "create_user_stake",
        &[],
        vec![
            signer_meta(user.pubkey()),           // user
            writable_meta(pool_pda),              // pool
            writable_meta(user_stake_pda),        // user_stake
            writable_meta(stake_mint),            // stake_mint
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),  // rent
            readonly_meta(system_program::id()), // system_program
        ],
    );
    execute_tx(&mut svm, create_stake_ix, &user, &[&user]).unwrap();

    // Stake tokens
    svm.expire_blockhash();
    let stake_amount: u64 = 500;
    let stake_ix = anchor_instruction(
        program_id,
        "stake",
        &stake_data(stake_amount),
        vec![
            signer_meta(user.pubkey()),           // user
            writable_meta(pool_pda),              // pool
            writable_meta(user_stake_pda),        // user_stake
            writable_meta(user_stake_token),      // user_stake_token
            writable_meta(stake_vault_pda),       // stake_vault
            writable_meta(stake_mint),            // stake_mint
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(token_program_id()),   // token_program
        ],
    );

    let result = execute_tx(&mut svm, stake_ix, &user, &[&user]);
    assert!(result.is_ok(), "stake should succeed: {:?}", result);

    // Verify user stake account
    let user_stake_account = svm.get_account(&user_stake_pda).expect("User stake should exist");
    assert_eq!(read_user_stake_owner(&user_stake_account.data), user.pubkey());
    assert_eq!(read_user_stake_staked_amount(&user_stake_account.data), stake_amount);

    // Verify pool total staked increased
    let pool_account = svm.get_account(&pool_pda).unwrap();
    assert_eq!(read_pool_total_staked(&pool_account.data), stake_amount);

    // Verify user's token account decreased
    let user_token_account = svm.get_account(&user_stake_token).unwrap();
    assert_eq!(read_token_balance(&user_token_account.data), 500);

    // Verify stake vault received tokens
    let vault_account = svm.get_account(&stake_vault_pda).unwrap();
    assert_eq!(read_token_balance(&vault_account.data), stake_amount);
}

// =============================================================================
// UNSTAKE WITH REWARDS TESTS
// =============================================================================

#[test]
fn test_unstake_with_rewards() {
    let mut svm = load_staking_program();
    let program_id = staking_program_id();

    // Setup
    let (_, stake_mint) = setup_mint(&mut svm, 6);
    let authority = funded_keypair_10_sol(&mut svm);
    let (pool_pda, _) = derive_pool_pda(&stake_mint);
    let (stake_vault_pda, _) = derive_stake_vault_pda(&stake_mint);
    let reward_mint = setup_mint_with_authority(&mut svm, &pool_pda, 6);

    let reward_rate: u64 = 100_000; // 0.1 reward token per staked token per slot

    // Initialize pool
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(reward_rate),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(stake_mint),
            writable_meta(reward_mint),
            writable_meta(pool_pda),
            writable_meta(stake_vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Setup user
    let (user, user_stake_token) = setup_user_with_tokens(&mut svm, &stake_mint, 1000);
    let user_reward_token = get_ata(&user.pubkey(), &reward_mint);
    svm.set_account(user_reward_token, create_token_account(&user.pubkey(), &reward_mint, 0)).unwrap();

    let (user_stake_pda, _) = derive_user_stake_pda(&stake_mint, &user.pubkey());

    // Create user stake
    svm.expire_blockhash();
    let create_stake_ix = anchor_instruction(
        program_id,
        "create_user_stake",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_stake_pda),
            writable_meta(stake_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_stake_ix, &user, &[&user]).unwrap();

    // Stake 1000 tokens
    svm.expire_blockhash();
    let stake_amount: u64 = 1000;
    let stake_ix = anchor_instruction(
        program_id,
        "stake",
        &stake_data(stake_amount),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_stake_pda),
            writable_meta(user_stake_token),
            writable_meta(stake_vault_pda),
            writable_meta(stake_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, stake_ix, &user, &[&user]).unwrap();

    // Warp forward 100 slots to accumulate rewards
    // Expected rewards: 1000 staked * 100_000 reward_rate * 100 slots / 1_000_000 = 10_000
    svm.warp_to_slot(100);
    svm.expire_blockhash();

    // Unstake all tokens
    let unstake_ix = anchor_instruction(
        program_id,
        "unstake",
        &unstake_data(stake_amount),
        vec![
            signer_meta(user.pubkey()),           // user
            writable_meta(pool_pda),              // pool
            writable_meta(reward_mint),           // reward_mint
            writable_meta(user_stake_pda),        // user_stake
            writable_meta(user_stake_token),      // user_stake_token
            writable_meta(user_reward_token),     // user_reward_token
            writable_meta(stake_vault_pda),       // stake_vault
            writable_meta(stake_mint),            // stake_mint
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(token_program_id()),   // token_program
        ],
    );

    let result = execute_tx(&mut svm, unstake_ix, &user, &[&user]);
    assert!(result.is_ok(), "unstake should succeed: {:?}", result);

    // Verify user got stake tokens back
    let user_token_account = svm.get_account(&user_stake_token).unwrap();
    assert_eq!(read_token_balance(&user_token_account.data), 1000);

    // Verify user received rewards
    let user_reward_account = svm.get_account(&user_reward_token).unwrap();
    let reward_balance = read_token_balance(&user_reward_account.data);
    assert!(reward_balance > 0, "User should have received rewards");

    // Verify pool is empty
    let pool_account = svm.get_account(&pool_pda).unwrap();
    assert_eq!(read_pool_total_staked(&pool_account.data), 0);
}

// =============================================================================
// CLAIM REWARDS TESTS
// =============================================================================

#[test]
fn test_claim_rewards_without_unstaking() {
    let mut svm = load_staking_program();
    let program_id = staking_program_id();

    // Setup
    let (_, stake_mint) = setup_mint(&mut svm, 6);
    let authority = funded_keypair_10_sol(&mut svm);
    let (pool_pda, _) = derive_pool_pda(&stake_mint);
    let (stake_vault_pda, _) = derive_stake_vault_pda(&stake_mint);
    let reward_mint = setup_mint_with_authority(&mut svm, &pool_pda, 6);

    let reward_rate: u64 = 100_000;

    // Initialize pool
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(reward_rate),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(stake_mint),
            writable_meta(reward_mint),
            writable_meta(pool_pda),
            writable_meta(stake_vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Setup user
    let (user, user_stake_token) = setup_user_with_tokens(&mut svm, &stake_mint, 1000);
    let user_reward_token = get_ata(&user.pubkey(), &reward_mint);
    svm.set_account(user_reward_token, create_token_account(&user.pubkey(), &reward_mint, 0)).unwrap();

    let (user_stake_pda, _) = derive_user_stake_pda(&stake_mint, &user.pubkey());

    // Create user stake and stake tokens
    svm.expire_blockhash();
    let create_stake_ix = anchor_instruction(
        program_id,
        "create_user_stake",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_stake_pda),
            writable_meta(stake_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_stake_ix, &user, &[&user]).unwrap();

    svm.expire_blockhash();
    let stake_ix = anchor_instruction(
        program_id,
        "stake",
        &stake_data(1000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_stake_pda),
            writable_meta(user_stake_token),
            writable_meta(stake_vault_pda),
            writable_meta(stake_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, stake_ix, &user, &[&user]).unwrap();

    // Warp forward to accumulate rewards
    svm.warp_to_slot(50);
    svm.expire_blockhash();

    // Claim rewards (should keep stake)
    let claim_ix = anchor_instruction(
        program_id,
        "claim_rewards",
        &[],
        vec![
            signer_meta(user.pubkey()),           // user
            writable_meta(pool_pda),              // pool
            writable_meta(reward_mint),           // reward_mint
            writable_meta(user_stake_pda),        // user_stake
            writable_meta(user_reward_token),     // user_reward_token
            writable_meta(stake_mint),            // stake_mint
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(token_program_id()),   // token_program
        ],
    );

    let result = execute_tx(&mut svm, claim_ix, &user, &[&user]);
    assert!(result.is_ok(), "claim_rewards should succeed: {:?}", result);

    // Verify user received rewards
    let user_reward_account = svm.get_account(&user_reward_token).unwrap();
    let reward_balance = read_token_balance(&user_reward_account.data);
    assert!(reward_balance > 0, "User should have received rewards");

    // Verify user stake amount unchanged
    let user_stake_account = svm.get_account(&user_stake_pda).unwrap();
    assert_eq!(read_user_stake_staked_amount(&user_stake_account.data), 1000, "Stake should remain");

    // Verify pool total unchanged
    let pool_account = svm.get_account(&pool_pda).unwrap();
    assert_eq!(read_pool_total_staked(&pool_account.data), 1000, "Pool total should remain");
}

// =============================================================================
// ERROR CASE TESTS
// =============================================================================

#[test]
fn test_stake_zero_amount_fails() {
    let mut svm = load_staking_program();
    let program_id = staking_program_id();

    // Setup
    let (_, stake_mint) = setup_mint(&mut svm, 6);
    let authority = funded_keypair_10_sol(&mut svm);
    let (pool_pda, _) = derive_pool_pda(&stake_mint);
    let (stake_vault_pda, _) = derive_stake_vault_pda(&stake_mint);
    let reward_mint = setup_mint_with_authority(&mut svm, &pool_pda, 6);

    // Initialize pool
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(1000),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(stake_mint),
            writable_meta(reward_mint),
            writable_meta(pool_pda),
            writable_meta(stake_vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Setup user
    let (user, user_stake_token) = setup_user_with_tokens(&mut svm, &stake_mint, 1000);
    let (user_stake_pda, _) = derive_user_stake_pda(&stake_mint, &user.pubkey());

    // Create user stake
    svm.expire_blockhash();
    let create_stake_ix = anchor_instruction(
        program_id,
        "create_user_stake",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_stake_pda),
            writable_meta(stake_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_stake_ix, &user, &[&user]).unwrap();

    // Try to stake zero
    svm.expire_blockhash();
    let stake_ix = anchor_instruction(
        program_id,
        "stake",
        &stake_data(0),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_stake_pda),
            writable_meta(user_stake_token),
            writable_meta(stake_vault_pda),
            writable_meta(stake_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, stake_ix, &user, &[&user]);
    assert!(result.is_err(), "stake with zero amount should fail");
}

#[test]
fn test_unstake_more_than_staked_fails() {
    let mut svm = load_staking_program();
    let program_id = staking_program_id();

    // Setup
    let (_, stake_mint) = setup_mint(&mut svm, 6);
    let authority = funded_keypair_10_sol(&mut svm);
    let (pool_pda, _) = derive_pool_pda(&stake_mint);
    let (stake_vault_pda, _) = derive_stake_vault_pda(&stake_mint);
    let reward_mint = setup_mint_with_authority(&mut svm, &pool_pda, 6);

    // Initialize pool
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(1000),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(stake_mint),
            writable_meta(reward_mint),
            writable_meta(pool_pda),
            writable_meta(stake_vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Setup user and stake 500 tokens
    let (user, user_stake_token) = setup_user_with_tokens(&mut svm, &stake_mint, 1000);
    let user_reward_token = get_ata(&user.pubkey(), &reward_mint);
    svm.set_account(user_reward_token, create_token_account(&user.pubkey(), &reward_mint, 0)).unwrap();

    let (user_stake_pda, _) = derive_user_stake_pda(&stake_mint, &user.pubkey());

    // Create user stake
    svm.expire_blockhash();
    let create_stake_ix = anchor_instruction(
        program_id,
        "create_user_stake",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_stake_pda),
            writable_meta(stake_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_stake_ix, &user, &[&user]).unwrap();

    // Stake 500
    svm.expire_blockhash();
    let stake_ix = anchor_instruction(
        program_id,
        "stake",
        &stake_data(500),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_stake_pda),
            writable_meta(user_stake_token),
            writable_meta(stake_vault_pda),
            writable_meta(stake_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, stake_ix, &user, &[&user]).unwrap();

    // Try to unstake 1000 (more than staked)
    svm.expire_blockhash();
    let unstake_ix = anchor_instruction(
        program_id,
        "unstake",
        &unstake_data(1000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(reward_mint),
            writable_meta(user_stake_pda),
            writable_meta(user_stake_token),
            writable_meta(user_reward_token),
            writable_meta(stake_vault_pda),
            writable_meta(stake_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, unstake_ix, &user, &[&user]);
    assert!(result.is_err(), "unstake more than staked should fail");
}

#[test]
fn test_claim_rewards_with_no_stake_fails() {
    let mut svm = load_staking_program();
    let program_id = staking_program_id();

    // Setup
    let (_, stake_mint) = setup_mint(&mut svm, 6);
    let authority = funded_keypair_10_sol(&mut svm);
    let (pool_pda, _) = derive_pool_pda(&stake_mint);
    let (stake_vault_pda, _) = derive_stake_vault_pda(&stake_mint);
    let reward_mint = setup_mint_with_authority(&mut svm, &pool_pda, 6);

    // Initialize pool
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(1000),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(stake_mint),
            writable_meta(reward_mint),
            writable_meta(pool_pda),
            writable_meta(stake_vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Setup user (no staking)
    let user = funded_keypair_10_sol(&mut svm);
    let user_reward_token = get_ata(&user.pubkey(), &reward_mint);
    svm.set_account(user_reward_token, create_token_account(&user.pubkey(), &reward_mint, 0)).unwrap();

    let (user_stake_pda, _) = derive_user_stake_pda(&stake_mint, &user.pubkey());

    // Create user stake but don't stake any tokens
    svm.expire_blockhash();
    let create_stake_ix = anchor_instruction(
        program_id,
        "create_user_stake",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_stake_pda),
            writable_meta(stake_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_stake_ix, &user, &[&user]).unwrap();

    // Try to claim rewards with zero stake
    svm.expire_blockhash();
    let claim_ix = anchor_instruction(
        program_id,
        "claim_rewards",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(reward_mint),
            writable_meta(user_stake_pda),
            writable_meta(user_reward_token),
            writable_meta(stake_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, claim_ix, &user, &[&user]);
    assert!(result.is_err(), "claim_rewards with no stake should fail");
}
