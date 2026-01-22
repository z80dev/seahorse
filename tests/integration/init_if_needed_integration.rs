//! LiteSVM integration tests for the init_if_needed constraint
//!
//! Tests the init_if_needed constraint which initializes an account if it doesn't exist,
//! or uses the existing account if it does. This is useful for idempotent account creation.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile init_if_needed_test.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// init_if_needed_test program ID (from declare_id!)
fn init_if_needed_program_id() -> Pubkey {
    Pubkey::from_str("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS").unwrap()
}

/// Counter account size: discriminator (8) + owner (32) + count (8)
const COUNTER_SIZE: usize = 8 + 32 + 8;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the init_if_needed_test program into LiteSVM
fn load_init_if_needed_program() -> (litesvm::LiteSVM, Keypair) {
    let program_id = init_if_needed_program_id();
    let program_bytes = std::fs::read("../../target/deploy/init_if_needed_test.so")
        .expect("Failed to read init_if_needed_test.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);

    // Create a funded owner
    let owner = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    (svm, owner)
}

/// Read counter count from account data
fn read_counter_count(data: &[u8]) -> u64 {
    // Skip discriminator (8) + owner (32), read count (8)
    let count_bytes: [u8; 8] = data[40..48].try_into().unwrap();
    u64::from_le_bytes(count_bytes)
}

/// Read counter owner from account data
fn read_counter_owner(data: &[u8]) -> Pubkey {
    // Skip discriminator (8), read owner (32)
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

// =============================================================================
// INIT_IF_NEEDED TESTS - FIRST CALL (INITIALIZATION)
// =============================================================================

#[test]
fn test_init_if_needed_first_call_initializes() {
    let (mut svm, owner) = load_init_if_needed_program();
    let program_id = init_if_needed_program_id();

    // Derive counter PDA
    let (counter_pda, _bump) = find_pda(&[b"counter", owner.pubkey().as_ref()], &program_id);

    // First call - should initialize the account
    let ix = anchor_instruction(
        program_id,
        "create_or_update_counter",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &owner, &[&owner]);
    assert!(result.is_ok(), "First call should succeed (init): {:?}", result);

    // Verify account was created
    let account = svm.get_account(&counter_pda).expect("Counter should exist");
    assert_eq!(account.data.len(), COUNTER_SIZE, "Account size should match");

    // Count should be 1 (initialized to 0 then incremented)
    let count = read_counter_count(&account.data);
    assert_eq!(count, 1, "Counter should be 1 after first call");

    // Owner should be set
    let stored_owner = read_counter_owner(&account.data);
    assert_eq!(stored_owner, owner.pubkey(), "Owner should be set");
}

// =============================================================================
// INIT_IF_NEEDED TESTS - SECOND CALL (USES EXISTING)
// =============================================================================

#[test]
fn test_init_if_needed_second_call_uses_existing() {
    let (mut svm, owner) = load_init_if_needed_program();
    let program_id = init_if_needed_program_id();
    let (counter_pda, _) = find_pda(&[b"counter", owner.pubkey().as_ref()], &program_id);

    // First call - initializes
    let ix1 = anchor_instruction(
        program_id,
        "create_or_update_counter",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix1, &owner, &[&owner]).unwrap();

    // Verify count is 1
    let account = svm.get_account(&counter_pda).unwrap();
    assert_eq!(read_counter_count(&account.data), 1);

    // Second call - should use existing account and increment
    svm.expire_blockhash();
    let ix2 = anchor_instruction(
        program_id,
        "create_or_update_counter",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    let result = execute_tx(&mut svm, ix2, &owner, &[&owner]);
    assert!(result.is_ok(), "Second call should succeed (use existing): {:?}", result);

    // Count should be 2 now
    let account = svm.get_account(&counter_pda).unwrap();
    assert_eq!(read_counter_count(&account.data), 2, "Counter should be 2 after second call");
}

#[test]
fn test_init_if_needed_multiple_calls_accumulate() {
    let (mut svm, owner) = load_init_if_needed_program();
    let program_id = init_if_needed_program_id();
    let (counter_pda, _) = find_pda(&[b"counter", owner.pubkey().as_ref()], &program_id);

    // Call 5 times
    for i in 1..=5 {
        if i > 1 {
            svm.expire_blockhash();
        }

        let ix = anchor_instruction(
            program_id,
            "create_or_update_counter",
            &[],
            vec![
                signer_meta(owner.pubkey()),
                writable_meta(counter_pda),
                readonly_meta(solana_sdk_ids::sysvar::rent::id()),
                readonly_meta(system_program::id()),
            ],
        );
        let result = execute_tx(&mut svm, ix, &owner, &[&owner]);
        assert!(result.is_ok(), "Call {} should succeed: {:?}", i, result);
    }

    // Count should be 5
    let account = svm.get_account(&counter_pda).unwrap();
    assert_eq!(read_counter_count(&account.data), 5, "Counter should be 5 after 5 calls");
}

// =============================================================================
// INIT_IF_NEEDED TESTS - DIFFERENT USERS
// =============================================================================

#[test]
fn test_init_if_needed_different_users_get_separate_accounts() {
    let program_id = init_if_needed_program_id();
    let program_bytes = std::fs::read("../../target/deploy/init_if_needed_test.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);

    let user1 = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user2 = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    // Derive PDAs for each user
    let (counter1_pda, _) = find_pda(&[b"counter", user1.pubkey().as_ref()], &program_id);
    let (counter2_pda, _) = find_pda(&[b"counter", user2.pubkey().as_ref()], &program_id);

    // PDAs should be different
    assert_ne!(counter1_pda, counter2_pda, "Different users should have different PDAs");

    // User1 creates/updates their counter 3 times
    for i in 1..=3 {
        if i > 1 {
            svm.expire_blockhash();
        }
        let ix = anchor_instruction(
            program_id,
            "create_or_update_counter",
            &[],
            vec![
                signer_meta(user1.pubkey()),
                writable_meta(counter1_pda),
                readonly_meta(solana_sdk_ids::sysvar::rent::id()),
                readonly_meta(system_program::id()),
            ],
        );
        execute_tx(&mut svm, ix, &user1, &[&user1]).unwrap();
    }

    // User2 creates/updates their counter 1 time
    svm.expire_blockhash();
    let ix = anchor_instruction(
        program_id,
        "create_or_update_counter",
        &[],
        vec![
            signer_meta(user2.pubkey()),
            writable_meta(counter2_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix, &user2, &[&user2]).unwrap();

    // Verify each user's counter
    let account1 = svm.get_account(&counter1_pda).unwrap();
    let account2 = svm.get_account(&counter2_pda).unwrap();

    assert_eq!(read_counter_count(&account1.data), 3, "User1 counter should be 3");
    assert_eq!(read_counter_count(&account2.data), 1, "User2 counter should be 1");
    assert_eq!(read_counter_owner(&account1.data), user1.pubkey());
    assert_eq!(read_counter_owner(&account2.data), user2.pubkey());
}

// =============================================================================
// INIT_IF_NEEDED TESTS - IDEMPOTENCY
// =============================================================================

#[test]
fn test_init_if_needed_is_idempotent_for_creation() {
    let (mut svm, owner) = load_init_if_needed_program();
    let program_id = init_if_needed_program_id();
    let (counter_pda, _) = find_pda(&[b"counter", owner.pubkey().as_ref()], &program_id);

    // Call create_or_update_counter
    let ix = anchor_instruction(
        program_id,
        "create_or_update_counter",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix, &owner, &[&owner]).unwrap();

    // Get the account state after first call
    let account_after_first = svm.get_account(&counter_pda).unwrap();
    let lamports_after_first = account_after_first.lamports;
    let data_len_after_first = account_after_first.data.len();

    // Call again
    svm.expire_blockhash();
    let ix2 = anchor_instruction(
        program_id,
        "create_or_update_counter",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix2, &owner, &[&owner]).unwrap();

    // Verify account structure is the same (only count changed)
    let account_after_second = svm.get_account(&counter_pda).unwrap();
    assert_eq!(account_after_second.lamports, lamports_after_first, "Lamports should be unchanged");
    assert_eq!(account_after_second.data.len(), data_len_after_first, "Data length should be unchanged");
    assert_eq!(read_counter_count(&account_after_second.data), 2, "Only count should change");
}
