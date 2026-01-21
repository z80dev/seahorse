//! LiteSVM integration tests for the Seahorse NFT Staking program
//!
//! These tests verify NFT staking with time-based rewards:
//! - initialize_pool: Creates NFT staking pool with reward rate
//! - stake_nft: Stakes an NFT (transfers to PDA-owned vault)
//! - unstake_nft: Returns NFT and claims rewards
//! - claim_rewards: Claims pending rewards without unstaking
//!
//! Uses LiteSVM's warp_to_slot for time-based reward testing.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile nft_staking.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// NFT Staking program ID (from declare_id!)
fn nft_staking_program_id() -> Pubkey {
    Pubkey::from_str("8DtVbTgYbcM8b2igVzb7RjbVKLLLDQ2bzXriSFp8fxhb").unwrap()
}

/// Pool account size: discriminator (8) + authority (32) + reward_mint (32)
///                    + reward_rate (8) + total_staked (8) + last_update_slot (8) + bump (1)
const POOL_SIZE: usize = 8 + 32 + 32 + 8 + 8 + 8 + 1;

/// StakeRecord account size: discriminator (8) + owner (32) + pool (32) + nft_mint (32) + nft_vault (32)
///                           + staked_at_slot (8) + last_claim_slot (8) + pending_rewards (8) + is_staked (1) + bump (1)
const STAKE_RECORD_SIZE: usize = 8 + 32 + 32 + 32 + 32 + 8 + 8 + 8 + 1 + 1;

/// Scale factor for reward calculations (same as Seahorse program)
const REWARD_SCALE: u64 = 1_000_000;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the NFT staking program into LiteSVM
fn load_nft_staking_program() -> litesvm::LiteSVM {
    let program_id = nft_staking_program_id();
    let program_bytes = std::fs::read("../../target/deploy/nft_staking.so")
        .expect("Failed to read nft_staking.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);
    svm
}

/// Setup a reward mint where pool PDA will be the mint authority
/// Returns (reward_mint_keypair, pool_pda)
fn setup_reward_mint_and_pool(svm: &mut litesvm::LiteSVM, decimals: u8) -> (Keypair, Pubkey) {
    let reward_mint = Keypair::new();

    // Derive pool PDA from the reward_mint pubkey
    let (pool_pda, _) = derive_pool_pda(&reward_mint.pubkey());

    // Create mint with pool_pda as authority
    let mint_account = create_mint_account(&pool_pda, decimals);
    svm.set_account(reward_mint.pubkey(), mint_account).unwrap();

    (reward_mint, pool_pda)
}

/// Setup an NFT mint (supply = 1, decimals = 0) - returns mint keypair
fn setup_nft_mint(svm: &mut litesvm::LiteSVM, mint_authority: &Pubkey) -> Keypair {
    let nft_mint = Keypair::new();

    // NFT mints have decimals = 0
    let mint_account = create_mint_account(mint_authority, 0);
    svm.set_account(nft_mint.pubkey(), mint_account).unwrap();

    nft_mint
}

/// Setup a user with an NFT (token account with amount = 1)
fn setup_user_with_nft(
    svm: &mut litesvm::LiteSVM,
    nft_mint: &Pubkey,
) -> (Keypair, Pubkey) {
    let user = funded_keypair_10_sol(svm);
    let user_nft_token = get_ata(&user.pubkey(), nft_mint);

    // Create token account with 1 NFT
    let token_account = create_token_account(&user.pubkey(), nft_mint, 1);
    svm.set_account(user_nft_token, token_account).unwrap();

    (user, user_nft_token)
}

/// Setup a user with empty token account
fn setup_user_with_empty_token_account(
    svm: &mut litesvm::LiteSVM,
    mint: &Pubkey,
) -> (Keypair, Pubkey) {
    let user = funded_keypair_10_sol(svm);
    let user_token = get_ata(&user.pubkey(), mint);

    let token_account = create_token_account(&user.pubkey(), mint, 0);
    svm.set_account(user_token, token_account).unwrap();

    (user, user_token)
}

/// Derive pool PDA (seeds: ['pool', reward_mint])
fn derive_pool_pda(reward_mint: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"pool", reward_mint.as_ref()],
        &nft_staking_program_id(),
    )
}

/// Derive stake record PDA (seeds: ['stake_record', nft_mint])
fn derive_stake_record_pda(nft_mint: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"stake_record", nft_mint.as_ref()],
        &nft_staking_program_id(),
    )
}

/// Derive NFT vault PDA (seeds: ['nft_vault', nft_mint])
fn derive_nft_vault_pda(nft_mint: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"nft_vault", nft_mint.as_ref()],
        &nft_staking_program_id(),
    )
}

/// Read pool fields from account data
fn read_pool_authority(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

fn read_pool_reward_mint(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[40..72].try_into().unwrap())
}

fn read_pool_reward_rate(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[72..80].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_pool_total_staked(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[80..88].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_pool_last_update_slot(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[88..96].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

/// Read stake record fields from account data
fn read_stake_record_owner(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

fn read_stake_record_pool(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[40..72].try_into().unwrap())
}

fn read_stake_record_nft_mint(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[72..104].try_into().unwrap())
}

fn read_stake_record_nft_vault(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[104..136].try_into().unwrap())
}

fn read_stake_record_staked_at_slot(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[136..144].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_stake_record_last_claim_slot(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[144..152].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_stake_record_pending_rewards(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[152..160].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_stake_record_is_staked(data: &[u8]) -> bool {
    data[160] != 0
}

/// Build initialize_pool instruction data
fn initialize_pool_data(reward_rate: u64) -> Vec<u8> {
    reward_rate.to_le_bytes().to_vec()
}

// =============================================================================
// INITIALIZE POOL TESTS
// =============================================================================

#[test]
fn test_initialize_pool() {
    let mut svm = load_nft_staking_program();
    let program_id = nft_staking_program_id();

    // Setup pool authority
    let authority = funded_keypair_10_sol(&mut svm);

    // Setup reward mint with pool PDA as authority
    let (reward_mint, pool_pda) = setup_reward_mint_and_pool(&mut svm, 6);

    let reward_rate: u64 = 100_000; // 0.1 reward token per NFT per slot

    // Build initialize_pool instruction (Seahorse account order)
    let ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(reward_rate),
        vec![
            signer_meta(authority.pubkey()),      // authority
            writable_meta(reward_mint.pubkey()),  // reward_mint
            writable_meta(pool_pda),              // pool
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(solana_sdk_ids::sysvar::rent::id()), // rent
            readonly_meta(system_program::id()), // system_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "initialize_pool should succeed: {:?}", result);

    // Verify pool state
    let pool_account = svm.get_account(&pool_pda).expect("Pool should exist");
    assert_eq!(pool_account.owner, program_id, "Pool should be owned by program");

    assert_eq!(read_pool_authority(&pool_account.data), authority.pubkey());
    assert_eq!(read_pool_reward_mint(&pool_account.data), reward_mint.pubkey());
    assert_eq!(read_pool_reward_rate(&pool_account.data), reward_rate);
    assert_eq!(read_pool_total_staked(&pool_account.data), 0);
}

// =============================================================================
// STAKE NFT TESTS
// =============================================================================

#[test]
fn test_stake_nft() {
    let mut svm = load_nft_staking_program();
    let program_id = nft_staking_program_id();

    // Setup authority and pool
    let authority = funded_keypair_10_sol(&mut svm);
    let (reward_mint, pool_pda) = setup_reward_mint_and_pool(&mut svm, 6);

    // Initialize pool
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(100_000),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(reward_mint.pubkey()),
            writable_meta(pool_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Setup NFT and user
    let nft_mint = setup_nft_mint(&mut svm, &authority.pubkey());
    let (user, user_nft_token) = setup_user_with_nft(&mut svm, &nft_mint.pubkey());

    // Derive PDAs
    let (stake_record_pda, _) = derive_stake_record_pda(&nft_mint.pubkey());
    let (nft_vault_pda, _) = derive_nft_vault_pda(&nft_mint.pubkey());

    // Stake NFT
    svm.expire_blockhash();
    let stake_ix = anchor_instruction(
        program_id,
        "stake_nft",
        &[],
        vec![
            signer_meta(user.pubkey()),           // user
            writable_meta(pool_pda),              // pool
            writable_meta(nft_mint.pubkey()),     // nft_mint
            writable_meta(user_nft_token),        // user_nft_token
            writable_meta(nft_vault_pda),         // nft_vault
            writable_meta(stake_record_pda),      // stake_record
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),  // rent
            readonly_meta(system_program::id()), // system_program
            readonly_meta(token_program_id()),   // token_program
        ],
    );

    let result = execute_tx(&mut svm, stake_ix, &user, &[&user]);
    assert!(result.is_ok(), "stake_nft should succeed: {:?}", result);

    // Verify stake record
    let stake_record = svm.get_account(&stake_record_pda).expect("Stake record should exist");
    assert_eq!(read_stake_record_owner(&stake_record.data), user.pubkey());
    assert_eq!(read_stake_record_pool(&stake_record.data), pool_pda);
    assert_eq!(read_stake_record_nft_mint(&stake_record.data), nft_mint.pubkey());
    assert_eq!(read_stake_record_nft_vault(&stake_record.data), nft_vault_pda);
    assert!(read_stake_record_is_staked(&stake_record.data));

    // Verify NFT transferred to vault
    let vault_account = svm.get_account(&nft_vault_pda).expect("NFT vault should exist");
    assert_eq!(read_token_balance(&vault_account.data), 1);

    // Verify user's token account is empty
    let user_token_account = svm.get_account(&user_nft_token).unwrap();
    assert_eq!(read_token_balance(&user_token_account.data), 0);

    // Verify pool total staked
    let pool_account = svm.get_account(&pool_pda).unwrap();
    assert_eq!(read_pool_total_staked(&pool_account.data), 1);
}

#[test]
fn test_stake_nft_user_does_not_own_fails() {
    let mut svm = load_nft_staking_program();
    let program_id = nft_staking_program_id();

    // Setup authority and pool
    let authority = funded_keypair_10_sol(&mut svm);
    let (reward_mint, pool_pda) = setup_reward_mint_and_pool(&mut svm, 6);

    // Initialize pool
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(100_000),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(reward_mint.pubkey()),
            writable_meta(pool_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Setup NFT but user has empty token account (amount = 0)
    let nft_mint = setup_nft_mint(&mut svm, &authority.pubkey());
    let (user, user_nft_token) = setup_user_with_empty_token_account(&mut svm, &nft_mint.pubkey());

    let (stake_record_pda, _) = derive_stake_record_pda(&nft_mint.pubkey());
    let (nft_vault_pda, _) = derive_nft_vault_pda(&nft_mint.pubkey());

    // Try to stake NFT user doesn't own
    svm.expire_blockhash();
    let stake_ix = anchor_instruction(
        program_id,
        "stake_nft",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(nft_mint.pubkey()),
            writable_meta(user_nft_token),
            writable_meta(nft_vault_pda),
            writable_meta(stake_record_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, stake_ix, &user, &[&user]);
    assert!(result.is_err(), "stake_nft should fail when user doesn't own NFT");
}

// =============================================================================
// UNSTAKE NFT TESTS
// =============================================================================

#[test]
fn test_unstake_nft_with_rewards() {
    let mut svm = load_nft_staking_program();
    let program_id = nft_staking_program_id();

    // Setup authority and pool
    let authority = funded_keypair_10_sol(&mut svm);
    let (reward_mint, pool_pda) = setup_reward_mint_and_pool(&mut svm, 6);

    let reward_rate: u64 = 100_000; // 0.1 reward token per slot

    // Initialize pool
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(reward_rate),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(reward_mint.pubkey()),
            writable_meta(pool_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Setup NFT and user
    let nft_mint = setup_nft_mint(&mut svm, &authority.pubkey());
    let (user, user_nft_token) = setup_user_with_nft(&mut svm, &nft_mint.pubkey());

    // Setup reward token account for user
    let user_reward_token = get_ata(&user.pubkey(), &reward_mint.pubkey());
    svm.set_account(user_reward_token, create_token_account(&user.pubkey(), &reward_mint.pubkey(), 0)).unwrap();

    let (stake_record_pda, _) = derive_stake_record_pda(&nft_mint.pubkey());
    let (nft_vault_pda, _) = derive_nft_vault_pda(&nft_mint.pubkey());

    // Stake NFT
    svm.expire_blockhash();
    let stake_ix = anchor_instruction(
        program_id,
        "stake_nft",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(nft_mint.pubkey()),
            writable_meta(user_nft_token),
            writable_meta(nft_vault_pda),
            writable_meta(stake_record_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, stake_ix, &user, &[&user]).unwrap();

    // Warp forward 100 slots to accumulate rewards
    // Expected rewards: 100_000 * 100 / 1_000_000 = 10 reward tokens
    svm.warp_to_slot(100);
    svm.expire_blockhash();

    // Unstake NFT
    let unstake_ix = anchor_instruction(
        program_id,
        "unstake_nft",
        &[],
        vec![
            signer_meta(user.pubkey()),           // user
            writable_meta(pool_pda),              // pool
            writable_meta(reward_mint.pubkey()),  // reward_mint
            writable_meta(nft_mint.pubkey()),     // nft_mint
            writable_meta(stake_record_pda),      // stake_record
            writable_meta(nft_vault_pda),         // nft_vault
            writable_meta(user_nft_token),        // user_nft_token
            writable_meta(user_reward_token),     // user_reward_token
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(token_program_id()),   // token_program
        ],
    );

    let result = execute_tx(&mut svm, unstake_ix, &user, &[&user]);
    assert!(result.is_ok(), "unstake_nft should succeed: {:?}", result);

    // Verify NFT returned to user
    let user_token_account = svm.get_account(&user_nft_token).unwrap();
    assert_eq!(read_token_balance(&user_token_account.data), 1);

    // Verify user received rewards
    let user_reward_account = svm.get_account(&user_reward_token).unwrap();
    let reward_balance = read_token_balance(&user_reward_account.data);
    assert!(reward_balance > 0, "User should have received rewards");
    // Expected: 100_000 * 100 / 1_000_000 = 10
    assert_eq!(reward_balance, 10, "Expected 10 reward tokens");

    // Verify stake record is_staked is false
    let stake_record = svm.get_account(&stake_record_pda).unwrap();
    assert!(!read_stake_record_is_staked(&stake_record.data));

    // Verify pool total staked is 0
    let pool_account = svm.get_account(&pool_pda).unwrap();
    assert_eq!(read_pool_total_staked(&pool_account.data), 0);
}

#[test]
fn test_unstake_nft_not_staked_fails() {
    let mut svm = load_nft_staking_program();
    let program_id = nft_staking_program_id();

    // Setup authority and pool
    let authority = funded_keypair_10_sol(&mut svm);
    let (reward_mint, pool_pda) = setup_reward_mint_and_pool(&mut svm, 6);

    // Initialize pool
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(100_000),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(reward_mint.pubkey()),
            writable_meta(pool_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Setup NFT, user stakes and unstakes
    let nft_mint = setup_nft_mint(&mut svm, &authority.pubkey());
    let (user, user_nft_token) = setup_user_with_nft(&mut svm, &nft_mint.pubkey());
    let user_reward_token = get_ata(&user.pubkey(), &reward_mint.pubkey());
    svm.set_account(user_reward_token, create_token_account(&user.pubkey(), &reward_mint.pubkey(), 0)).unwrap();

    let (stake_record_pda, _) = derive_stake_record_pda(&nft_mint.pubkey());
    let (nft_vault_pda, _) = derive_nft_vault_pda(&nft_mint.pubkey());

    // Stake NFT
    svm.expire_blockhash();
    let stake_ix = anchor_instruction(
        program_id,
        "stake_nft",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(nft_mint.pubkey()),
            writable_meta(user_nft_token),
            writable_meta(nft_vault_pda),
            writable_meta(stake_record_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, stake_ix, &user, &[&user]).unwrap();

    // Unstake NFT
    svm.expire_blockhash();
    let unstake_ix = anchor_instruction(
        program_id,
        "unstake_nft",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(reward_mint.pubkey()),
            writable_meta(nft_mint.pubkey()),
            writable_meta(stake_record_pda),
            writable_meta(nft_vault_pda),
            writable_meta(user_nft_token),
            writable_meta(user_reward_token),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, unstake_ix, &user, &[&user]).unwrap();

    // Try to unstake again (already unstaked)
    svm.expire_blockhash();
    let unstake_again_ix = anchor_instruction(
        program_id,
        "unstake_nft",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(reward_mint.pubkey()),
            writable_meta(nft_mint.pubkey()),
            writable_meta(stake_record_pda),
            writable_meta(nft_vault_pda),
            writable_meta(user_nft_token),
            writable_meta(user_reward_token),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, unstake_again_ix, &user, &[&user]);
    assert!(result.is_err(), "unstake when not staked should fail");
}

// =============================================================================
// CLAIM REWARDS TESTS
// =============================================================================

#[test]
fn test_claim_rewards_without_unstaking() {
    let mut svm = load_nft_staking_program();
    let program_id = nft_staking_program_id();

    // Setup authority and pool
    let authority = funded_keypair_10_sol(&mut svm);
    let (reward_mint, pool_pda) = setup_reward_mint_and_pool(&mut svm, 6);

    let reward_rate: u64 = 100_000; // 0.1 reward token per slot

    // Initialize pool
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(reward_rate),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(reward_mint.pubkey()),
            writable_meta(pool_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Setup NFT and user
    let nft_mint = setup_nft_mint(&mut svm, &authority.pubkey());
    let (user, user_nft_token) = setup_user_with_nft(&mut svm, &nft_mint.pubkey());
    let user_reward_token = get_ata(&user.pubkey(), &reward_mint.pubkey());
    svm.set_account(user_reward_token, create_token_account(&user.pubkey(), &reward_mint.pubkey(), 0)).unwrap();

    let (stake_record_pda, _) = derive_stake_record_pda(&nft_mint.pubkey());
    let (nft_vault_pda, _) = derive_nft_vault_pda(&nft_mint.pubkey());

    // Stake NFT
    svm.expire_blockhash();
    let stake_ix = anchor_instruction(
        program_id,
        "stake_nft",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(nft_mint.pubkey()),
            writable_meta(user_nft_token),
            writable_meta(nft_vault_pda),
            writable_meta(stake_record_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, stake_ix, &user, &[&user]).unwrap();

    // Warp forward to accumulate rewards
    svm.warp_to_slot(50);
    svm.expire_blockhash();

    // Claim rewards
    let claim_ix = anchor_instruction(
        program_id,
        "claim_rewards",
        &[],
        vec![
            signer_meta(user.pubkey()),           // user
            writable_meta(pool_pda),              // pool
            writable_meta(reward_mint.pubkey()),  // reward_mint
            writable_meta(nft_mint.pubkey()),     // nft_mint
            writable_meta(stake_record_pda),      // stake_record
            writable_meta(user_reward_token),     // user_reward_token
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
    // Expected: 100_000 * 50 / 1_000_000 = 5
    assert_eq!(reward_balance, 5);

    // Verify NFT still in vault
    let vault_account = svm.get_account(&nft_vault_pda).unwrap();
    assert_eq!(read_token_balance(&vault_account.data), 1);

    // Verify stake record still staked
    let stake_record = svm.get_account(&stake_record_pda).unwrap();
    assert!(read_stake_record_is_staked(&stake_record.data));

    // Verify pool total unchanged
    let pool_account = svm.get_account(&pool_pda).unwrap();
    assert_eq!(read_pool_total_staked(&pool_account.data), 1);
}

#[test]
fn test_claim_rewards_not_staked_fails() {
    let mut svm = load_nft_staking_program();
    let program_id = nft_staking_program_id();

    // Setup authority and pool
    let authority = funded_keypair_10_sol(&mut svm);
    let (reward_mint, pool_pda) = setup_reward_mint_and_pool(&mut svm, 6);

    // Initialize pool
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(100_000),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(reward_mint.pubkey()),
            writable_meta(pool_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Setup NFT and user, stake then unstake
    let nft_mint = setup_nft_mint(&mut svm, &authority.pubkey());
    let (user, user_nft_token) = setup_user_with_nft(&mut svm, &nft_mint.pubkey());
    let user_reward_token = get_ata(&user.pubkey(), &reward_mint.pubkey());
    svm.set_account(user_reward_token, create_token_account(&user.pubkey(), &reward_mint.pubkey(), 0)).unwrap();

    let (stake_record_pda, _) = derive_stake_record_pda(&nft_mint.pubkey());
    let (nft_vault_pda, _) = derive_nft_vault_pda(&nft_mint.pubkey());

    // Stake then unstake
    svm.expire_blockhash();
    let stake_ix = anchor_instruction(
        program_id,
        "stake_nft",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(nft_mint.pubkey()),
            writable_meta(user_nft_token),
            writable_meta(nft_vault_pda),
            writable_meta(stake_record_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, stake_ix, &user, &[&user]).unwrap();

    svm.expire_blockhash();
    let unstake_ix = anchor_instruction(
        program_id,
        "unstake_nft",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(reward_mint.pubkey()),
            writable_meta(nft_mint.pubkey()),
            writable_meta(stake_record_pda),
            writable_meta(nft_vault_pda),
            writable_meta(user_nft_token),
            writable_meta(user_reward_token),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, unstake_ix, &user, &[&user]).unwrap();

    // Try to claim rewards when not staked
    svm.warp_to_slot(100);
    svm.expire_blockhash();
    let claim_ix = anchor_instruction(
        program_id,
        "claim_rewards",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(reward_mint.pubkey()),
            writable_meta(nft_mint.pubkey()),
            writable_meta(stake_record_pda),
            writable_meta(user_reward_token),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, claim_ix, &user, &[&user]);
    assert!(result.is_err(), "claim_rewards when not staked should fail");
}

// =============================================================================
// FULL WORKFLOW TESTS
// =============================================================================

#[test]
fn test_full_nft_staking_workflow() {
    let mut svm = load_nft_staking_program();
    let program_id = nft_staking_program_id();

    // Setup authority and pool
    let authority = funded_keypair_10_sol(&mut svm);
    let (reward_mint, pool_pda) = setup_reward_mint_and_pool(&mut svm, 6);

    let reward_rate: u64 = 100_000;

    // 1. Initialize pool
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(reward_rate),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(reward_mint.pubkey()),
            writable_meta(pool_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Setup NFT and user
    let nft_mint = setup_nft_mint(&mut svm, &authority.pubkey());
    let (user, user_nft_token) = setup_user_with_nft(&mut svm, &nft_mint.pubkey());
    let user_reward_token = get_ata(&user.pubkey(), &reward_mint.pubkey());
    svm.set_account(user_reward_token, create_token_account(&user.pubkey(), &reward_mint.pubkey(), 0)).unwrap();

    let (stake_record_pda, _) = derive_stake_record_pda(&nft_mint.pubkey());
    let (nft_vault_pda, _) = derive_nft_vault_pda(&nft_mint.pubkey());

    // 2. Stake NFT
    svm.expire_blockhash();
    let stake_ix = anchor_instruction(
        program_id,
        "stake_nft",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(nft_mint.pubkey()),
            writable_meta(user_nft_token),
            writable_meta(nft_vault_pda),
            writable_meta(stake_record_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, stake_ix, &user, &[&user]).unwrap();

    // Verify initial state
    assert_eq!(read_token_balance(&svm.get_account(&user_nft_token).unwrap().data), 0);
    assert_eq!(read_token_balance(&svm.get_account(&nft_vault_pda).unwrap().data), 1);
    assert_eq!(read_pool_total_staked(&svm.get_account(&pool_pda).unwrap().data), 1);

    // 3. Advance time and claim rewards
    svm.warp_to_slot(50);
    svm.expire_blockhash();

    let claim_ix = anchor_instruction(
        program_id,
        "claim_rewards",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(reward_mint.pubkey()),
            writable_meta(nft_mint.pubkey()),
            writable_meta(stake_record_pda),
            writable_meta(user_reward_token),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, claim_ix, &user, &[&user]).unwrap();

    // Verify claim worked - 100_000 * 50 / 1_000_000 = 5
    let first_claim_rewards = read_token_balance(&svm.get_account(&user_reward_token).unwrap().data);
    assert_eq!(first_claim_rewards, 5);

    // 4. Advance more time and unstake
    svm.warp_to_slot(100);
    svm.expire_blockhash();

    let unstake_ix = anchor_instruction(
        program_id,
        "unstake_nft",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(reward_mint.pubkey()),
            writable_meta(nft_mint.pubkey()),
            writable_meta(stake_record_pda),
            writable_meta(nft_vault_pda),
            writable_meta(user_nft_token),
            writable_meta(user_reward_token),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, unstake_ix, &user, &[&user]).unwrap();

    // Verify final state
    // NFT returned
    assert_eq!(read_token_balance(&svm.get_account(&user_nft_token).unwrap().data), 1);

    // Pool empty
    assert_eq!(read_pool_total_staked(&svm.get_account(&pool_pda).unwrap().data), 0);

    // Total rewards: 5 (first claim) + 5 (remaining slots 50-100) = 10
    let final_rewards = read_token_balance(&svm.get_account(&user_reward_token).unwrap().data);
    assert_eq!(final_rewards, 10);

    // Stake record marked as not staked
    assert!(!read_stake_record_is_staked(&svm.get_account(&stake_record_pda).unwrap().data));
}

#[test]
fn test_pda_derivation_deterministic() {
    let reward_mint = Pubkey::new_unique();
    let nft_mint = Pubkey::new_unique();

    // Pool PDA should be consistent
    let (pool_pda_1, bump_1) = derive_pool_pda(&reward_mint);
    let (pool_pda_2, bump_2) = derive_pool_pda(&reward_mint);
    assert_eq!(pool_pda_1, pool_pda_2);
    assert_eq!(bump_1, bump_2);

    // Stake record PDA should be consistent
    let (stake_record_1, sr_bump_1) = derive_stake_record_pda(&nft_mint);
    let (stake_record_2, sr_bump_2) = derive_stake_record_pda(&nft_mint);
    assert_eq!(stake_record_1, stake_record_2);
    assert_eq!(sr_bump_1, sr_bump_2);

    // NFT vault PDA should be consistent
    let (vault_1, v_bump_1) = derive_nft_vault_pda(&nft_mint);
    let (vault_2, v_bump_2) = derive_nft_vault_pda(&nft_mint);
    assert_eq!(vault_1, vault_2);
    assert_eq!(v_bump_1, v_bump_2);
}
