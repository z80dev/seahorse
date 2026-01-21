//! LiteSVM integration tests for the Seahorse Counter program
//!
//! These tests verify complete transaction flows including initialization.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile counter.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// Counter program ID (from declare_id!)
fn counter_program_id() -> Pubkey {
    Pubkey::from_str("CntrQd1yLLfEMvj47u3qHxq5xW3jcfC2E51h4jRmpump").unwrap()
}

/// Counter account size: discriminator (8) + count (8) + authority (32)
const COUNTER_SIZE: usize = 8 + 8 + 32;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the counter program into LiteSVM
fn load_counter_program() -> (litesvm::LiteSVM, Keypair) {
    let program_keypair = Keypair::new();
    let mut svm = litesvm_with_program(&program_keypair, "../../target/deploy/counter.so");

    // Create a funded authority
    let authority = funded_keypair_10_sol(&mut svm);

    // Override program ID to match declared ID
    let program_id = counter_program_id();
    let program_bytes = std::fs::read("../../target/deploy/counter.so").unwrap();
    svm.add_program(program_id, &program_bytes);

    (svm, authority)
}

/// Read counter value from account data
fn read_counter_value(data: &[u8]) -> u64 {
    // Skip discriminator (8 bytes), read count (8 bytes)
    let count_bytes: [u8; 8] = data[8..16].try_into().unwrap();
    u64::from_le_bytes(count_bytes)
}

/// Read authority from account data
fn read_counter_authority(data: &[u8]) -> Pubkey {
    // Skip discriminator (8) and count (8), read authority (32)
    Pubkey::new_from_array(data[16..48].try_into().unwrap())
}

// =============================================================================
// INITIALIZE TESTS
// =============================================================================

#[test]
fn test_counter_initialize() {
    let (mut svm, authority) = load_counter_program();
    let program_id = counter_program_id();

    // Derive counter PDA
    let (counter_pda, _bump) = find_pda(&[b"counter", authority.pubkey().as_ref()], &program_id);

    // Create initialize instruction
    let ix = anchor_instruction(
        program_id,
        "initialize",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    // Execute transaction
    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Initialize should succeed: {:?}", result);

    // Verify account state
    let account = svm.get_account(&counter_pda).expect("Counter should exist");
    assert_eq!(account.data.len(), COUNTER_SIZE, "Account size should match");

    let count = read_counter_value(&account.data);
    assert_eq!(count, 0, "Counter should be initialized to 0");

    let stored_authority = read_counter_authority(&account.data);
    assert_eq!(stored_authority, authority.pubkey(), "Authority should be set");
}

#[test]
fn test_counter_initialize_different_users() {
    let program_id = counter_program_id();
    let program_bytes = std::fs::read("../../target/deploy/counter.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);

    // Create two different users
    let user1 = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user2 = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    // Derive PDAs for each user
    let (counter1_pda, _) = find_pda(&[b"counter", user1.pubkey().as_ref()], &program_id);
    let (counter2_pda, _) = find_pda(&[b"counter", user2.pubkey().as_ref()], &program_id);

    // PDAs should be different
    assert_ne!(counter1_pda, counter2_pda);

    // Initialize user1's counter
    let ix1 = anchor_instruction(
        program_id,
        "initialize",
        &[],
        vec![
            signer_meta(user1.pubkey()),
            writable_meta(counter1_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    let result1 = execute_tx(&mut svm, ix1, &user1, &[&user1]);
    assert!(result1.is_ok(), "User1 initialize should succeed");

    // Initialize user2's counter
    let ix2 = anchor_instruction(
        program_id,
        "initialize",
        &[],
        vec![
            signer_meta(user2.pubkey()),
            writable_meta(counter2_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    let result2 = execute_tx(&mut svm, ix2, &user2, &[&user2]);
    assert!(result2.is_ok(), "User2 initialize should succeed");

    // Both counters should exist independently
    let account1 = svm.get_account(&counter1_pda).unwrap();
    let account2 = svm.get_account(&counter2_pda).unwrap();

    assert_eq!(read_counter_authority(&account1.data), user1.pubkey());
    assert_eq!(read_counter_authority(&account2.data), user2.pubkey());
}

// =============================================================================
// FULL WORKFLOW TESTS
// =============================================================================

#[test]
fn test_counter_full_workflow() {
    let (mut svm, authority) = load_counter_program();
    let program_id = counter_program_id();

    let (counter_pda, _bump) = find_pda(&[b"counter", authority.pubkey().as_ref()], &program_id);

    // 1. Initialize
    let init_ix = anchor_instruction(
        program_id,
        "initialize",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    let result = execute_tx(&mut svm, init_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Initialize should succeed");

    // Verify initial state
    let account = svm.get_account(&counter_pda).unwrap();
    assert_eq!(read_counter_value(&account.data), 0);

    // 2. Increment 3 times
    for _i in 0..3 {
        // Expire blockhash to get fresh blockhash (prevents AlreadyProcessed error)
        svm.expire_blockhash();

        let inc_ix = anchor_instruction(
            program_id,
            "increment",
            &[],
            vec![
                signer_meta(authority.pubkey()),
                writable_meta(counter_pda),
            ],
        );
        let result = execute_tx(&mut svm, inc_ix, &authority, &[&authority]);
        assert!(result.is_ok(), "Increment should succeed");

        let account = svm.get_account(&counter_pda).unwrap();
        let _value = read_counter_value(&account.data);
        // Note: Each increment increases the counter by 1
    }

    // Verify final increment result
    let account = svm.get_account(&counter_pda).unwrap();
    assert_eq!(read_counter_value(&account.data), 3, "Counter should be 3 after 3 increments");

    // 3. Decrement once
    svm.expire_blockhash();
    let dec_ix = anchor_instruction(
        program_id,
        "decrement",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
        ],
    );
    let result = execute_tx(&mut svm, dec_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Decrement should succeed");

    let account = svm.get_account(&counter_pda).unwrap();
    assert_eq!(read_counter_value(&account.data), 2, "Counter should be 2 after decrement");

    // 4. Set value to 100
    svm.expire_blockhash();
    let set_ix = anchor_instruction(
        program_id,
        "set_value",
        &100u64.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
        ],
    );
    let result = execute_tx(&mut svm, set_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Set value should succeed");

    let account = svm.get_account(&counter_pda).unwrap();
    assert_eq!(read_counter_value(&account.data), 100, "Counter should be 100 after set_value");
}

// =============================================================================
// INCREMENT TESTS
// =============================================================================

#[test]
fn test_counter_increment() {
    let (mut svm, authority) = load_counter_program();
    let program_id = counter_program_id();
    let (counter_pda, _) = find_pda(&[b"counter", authority.pubkey().as_ref()], &program_id);

    // Initialize
    let init_ix = anchor_instruction(
        program_id,
        "initialize",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Increment
    let inc_ix = anchor_instruction(
        program_id,
        "increment",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
        ],
    );
    let result = execute_tx(&mut svm, inc_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Increment should succeed");

    let account = svm.get_account(&counter_pda).unwrap();
    assert_eq!(read_counter_value(&account.data), 1);
}

// =============================================================================
// DECREMENT TESTS
// =============================================================================

#[test]
fn test_counter_decrement() {
    let (mut svm, authority) = load_counter_program();
    let program_id = counter_program_id();
    let (counter_pda, _) = find_pda(&[b"counter", authority.pubkey().as_ref()], &program_id);

    // Initialize
    let init_ix = anchor_instruction(
        program_id,
        "initialize",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Set to 5
    let set_ix = anchor_instruction(
        program_id,
        "set_value",
        &5u64.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
        ],
    );
    execute_tx(&mut svm, set_ix, &authority, &[&authority]).unwrap();

    // Decrement
    let dec_ix = anchor_instruction(
        program_id,
        "decrement",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
        ],
    );
    let result = execute_tx(&mut svm, dec_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Decrement should succeed");

    let account = svm.get_account(&counter_pda).unwrap();
    assert_eq!(read_counter_value(&account.data), 4);
}

#[test]
fn test_counter_decrement_underflow_fails() {
    let (mut svm, authority) = load_counter_program();
    let program_id = counter_program_id();
    let (counter_pda, _) = find_pda(&[b"counter", authority.pubkey().as_ref()], &program_id);

    // Initialize (count = 0)
    let init_ix = anchor_instruction(
        program_id,
        "initialize",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Attempt to decrement from 0 - should fail
    let dec_ix = anchor_instruction(
        program_id,
        "decrement",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
        ],
    );
    let result = execute_tx(&mut svm, dec_ix, &authority, &[&authority]);
    assert!(result.is_err(), "Decrement from 0 should fail");
}

// =============================================================================
// SET_VALUE TESTS
// =============================================================================

#[test]
fn test_counter_set_value() {
    let (mut svm, authority) = load_counter_program();
    let program_id = counter_program_id();
    let (counter_pda, _) = find_pda(&[b"counter", authority.pubkey().as_ref()], &program_id);

    // Initialize
    let init_ix = anchor_instruction(
        program_id,
        "initialize",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Set value to 42
    let set_ix = anchor_instruction(
        program_id,
        "set_value",
        &42u64.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
        ],
    );
    let result = execute_tx(&mut svm, set_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Set value should succeed");

    let account = svm.get_account(&counter_pda).unwrap();
    assert_eq!(read_counter_value(&account.data), 42);
}

// =============================================================================
// AUTHORIZATION TESTS
// =============================================================================

#[test]
fn test_counter_unauthorized_increment_fails() {
    let program_id = counter_program_id();
    let program_bytes = std::fs::read("../../target/deploy/counter.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);

    let owner = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let attacker = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    // Owner initializes their counter
    let (counter_pda, _) = find_pda(&[b"counter", owner.pubkey().as_ref()], &program_id);

    let init_ix = anchor_instruction(
        program_id,
        "initialize",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // Attacker tries to increment owner's counter - should fail
    let inc_ix = anchor_instruction(
        program_id,
        "increment",
        &[],
        vec![
            signer_meta(attacker.pubkey()),
            writable_meta(counter_pda),
        ],
    );
    let result = execute_tx(&mut svm, inc_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "Unauthorized increment should fail");
}

#[test]
fn test_counter_unauthorized_decrement_fails() {
    let program_id = counter_program_id();
    let program_bytes = std::fs::read("../../target/deploy/counter.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);

    let owner = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let attacker = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    let (counter_pda, _) = find_pda(&[b"counter", owner.pubkey().as_ref()], &program_id);

    // Owner initializes and sets value
    let init_ix = anchor_instruction(
        program_id,
        "initialize",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    let set_ix = anchor_instruction(
        program_id,
        "set_value",
        &10u64.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(counter_pda),
        ],
    );
    execute_tx(&mut svm, set_ix, &owner, &[&owner]).unwrap();

    // Attacker tries to decrement - should fail
    let dec_ix = anchor_instruction(
        program_id,
        "decrement",
        &[],
        vec![
            signer_meta(attacker.pubkey()),
            writable_meta(counter_pda),
        ],
    );
    let result = execute_tx(&mut svm, dec_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "Unauthorized decrement should fail");
}

#[test]
fn test_counter_unauthorized_set_value_fails() {
    let program_id = counter_program_id();
    let program_bytes = std::fs::read("../../target/deploy/counter.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);

    let owner = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let attacker = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    let (counter_pda, _) = find_pda(&[b"counter", owner.pubkey().as_ref()], &program_id);

    // Owner initializes
    let init_ix = anchor_instruction(
        program_id,
        "initialize",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // Attacker tries to set value - should fail
    let set_ix = anchor_instruction(
        program_id,
        "set_value",
        &999u64.to_le_bytes(),
        vec![
            signer_meta(attacker.pubkey()),
            writable_meta(counter_pda),
        ],
    );
    let result = execute_tx(&mut svm, set_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "Unauthorized set_value should fail");

    // Verify value unchanged
    let account = svm.get_account(&counter_pda).unwrap();
    assert_eq!(read_counter_value(&account.data), 0, "Value should remain 0");
}
