//! Mollusk unit tests for the Seahorse Counter program
//!
//! These tests verify each instruction in isolation using Mollusk.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile counter.so

use mollusk_svm::{result::Check, Mollusk};
use seahorse_unit_tests::helpers::*;
use seahorse_unit_tests::*;
use solana_sdk::{
    account::Account,
    account::ReadableAccount,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    rent::Rent,
    sysvar,
};
use std::str::FromStr;

/// Counter program ID (from declare_id!)
fn counter_program_id() -> Pubkey {
    Pubkey::from_str("CntrQd1yLLfEMvj47u3qHxq5xW3jcfC2E51h4jRmpump").unwrap()
}

/// Create a Mollusk instance for the counter program
fn counter_mollusk() -> Mollusk {
    // Path relative to project root (tests run from tests/unit/)
    mollusk_for_program(&counter_program_id(), "../../target/deploy/counter")
}

/// Create a system account with given lamports
fn system_account(lamports: u64) -> Account {
    Account {
        lamports,
        data: vec![],
        owner: solana_sdk::system_program::id(),
        executable: false,
        rent_epoch: 0,
    }
}

/// Create an uninitialized counter account for init
fn uninitialized_counter_account(lamports: u64) -> Account {
    Account {
        lamports,
        data: vec![0u8; COUNTER_SIZE],
        owner: counter_program_id(),
        executable: false,
        rent_epoch: 0,
    }
}

/// Create an initialized counter account
fn initialized_counter_account(count: u64, authority: &Pubkey) -> Account {
    let mut data = Vec::with_capacity(COUNTER_SIZE);

    // Discriminator for "Counter"
    let disc = anchor_discriminator("Counter");
    data.extend_from_slice(&disc);

    // count: u64
    data.extend_from_slice(&count.to_le_bytes());

    // authority: Pubkey
    data.extend_from_slice(authority.as_ref());

    let rent = Rent::default();
    Account {
        lamports: rent.minimum_balance(COUNTER_SIZE),
        data,
        owner: counter_program_id(),
        executable: false,
        rent_epoch: 0,
    }
}

/// Create a rent sysvar account
fn rent_sysvar_account() -> Account {
    let rent = Rent::default();
    let data = bincode::serialize(&rent).unwrap();
    Account {
        lamports: 1,
        data,
        owner: sysvar::id(),
        executable: false,
        rent_epoch: 0,
    }
}

// =============================================================================
// INITIALIZE TESTS
// =============================================================================
//
// Note: The initialize instruction requires CPI to the system program to
// create and allocate the account. Mollusk has limited CPI support, so we
// test initialization through the LiteSVM integration tests instead.
//
// The Mollusk tests focus on instruction behavior after initialization.

// =============================================================================
// INCREMENT TESTS
// =============================================================================

#[test]
fn test_counter_increment() {
    let mollusk = counter_mollusk();
    let program_id = counter_program_id();

    let authority = Pubkey::new_unique();
    let (counter_pda, _bump) = find_pda(&[b"counter", authority.as_ref()], &program_id);

    // Set up accounts with initialized counter at 0
    let accounts: Vec<(Pubkey, Account)> = vec![
        (authority, system_account(10 * LAMPORTS_PER_SOL)),
        (counter_pda, initialized_counter_account(0, &authority)),
    ];

    // Create increment instruction
    let ix = anchor_instruction(
        program_id,
        "increment",
        &[],
        vec![
            signer_meta(authority),
            writable_meta(counter_pda),
        ],
    );

    // Process instruction
    let result = mollusk.process_instruction(&ix, &accounts);

    assert!(
        result.raw_result.is_ok(),
        "Increment should succeed: {:?}",
        result.raw_result
    );

    // Verify count increased
    let counter_account = result.get_account(&counter_pda).expect("Counter account should exist");
    let data = counter_account.data();
    let count_bytes: [u8; 8] = data[8..16].try_into().unwrap();
    let count = u64::from_le_bytes(count_bytes);
    assert_eq!(count, 1, "Counter should be 1 after increment");
}

#[test]
fn test_counter_increment_multiple() {
    let mollusk = counter_mollusk();
    let program_id = counter_program_id();

    let authority = Pubkey::new_unique();
    let (counter_pda, _bump) = find_pda(&[b"counter", authority.as_ref()], &program_id);

    // Start with count = 5
    let accounts: Vec<(Pubkey, Account)> = vec![
        (authority, system_account(10 * LAMPORTS_PER_SOL)),
        (counter_pda, initialized_counter_account(5, &authority)),
    ];

    let ix = anchor_instruction(
        program_id,
        "increment",
        &[],
        vec![
            signer_meta(authority),
            writable_meta(counter_pda),
        ],
    );

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(result.raw_result.is_ok());

    let counter_account = result.get_account(&counter_pda).unwrap();
    let data = counter_account.data();
    let count_bytes: [u8; 8] = data[8..16].try_into().unwrap();
    let count = u64::from_le_bytes(count_bytes);
    assert_eq!(count, 6, "Counter should be 6 after incrementing from 5");
}

// =============================================================================
// DECREMENT TESTS
// =============================================================================

#[test]
fn test_counter_decrement() {
    let mollusk = counter_mollusk();
    let program_id = counter_program_id();

    let authority = Pubkey::new_unique();
    let (counter_pda, _bump) = find_pda(&[b"counter", authority.as_ref()], &program_id);

    // Start with count = 5
    let accounts: Vec<(Pubkey, Account)> = vec![
        (authority, system_account(10 * LAMPORTS_PER_SOL)),
        (counter_pda, initialized_counter_account(5, &authority)),
    ];

    let ix = anchor_instruction(
        program_id,
        "decrement",
        &[],
        vec![
            signer_meta(authority),
            writable_meta(counter_pda),
        ],
    );

    let result = mollusk.process_instruction(&ix, &accounts);

    assert!(
        result.raw_result.is_ok(),
        "Decrement should succeed: {:?}",
        result.raw_result
    );

    let counter_account = result.get_account(&counter_pda).unwrap();
    let data = counter_account.data();
    let count_bytes: [u8; 8] = data[8..16].try_into().unwrap();
    let count = u64::from_le_bytes(count_bytes);
    assert_eq!(count, 4, "Counter should be 4 after decrementing from 5");
}

#[test]
fn test_counter_decrement_to_zero() {
    let mollusk = counter_mollusk();
    let program_id = counter_program_id();

    let authority = Pubkey::new_unique();
    let (counter_pda, _bump) = find_pda(&[b"counter", authority.as_ref()], &program_id);

    // Start with count = 1
    let accounts: Vec<(Pubkey, Account)> = vec![
        (authority, system_account(10 * LAMPORTS_PER_SOL)),
        (counter_pda, initialized_counter_account(1, &authority)),
    ];

    let ix = anchor_instruction(
        program_id,
        "decrement",
        &[],
        vec![
            signer_meta(authority),
            writable_meta(counter_pda),
        ],
    );

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(result.raw_result.is_ok());

    let counter_account = result.get_account(&counter_pda).unwrap();
    let data = counter_account.data();
    let count_bytes: [u8; 8] = data[8..16].try_into().unwrap();
    let count = u64::from_le_bytes(count_bytes);
    assert_eq!(count, 0, "Counter should be 0 after decrementing from 1");
}

#[test]
fn test_counter_decrement_underflow() {
    let mollusk = counter_mollusk();
    let program_id = counter_program_id();

    let authority = Pubkey::new_unique();
    let (counter_pda, _bump) = find_pda(&[b"counter", authority.as_ref()], &program_id);

    // Start with count = 0 (underflow case)
    let accounts: Vec<(Pubkey, Account)> = vec![
        (authority, system_account(10 * LAMPORTS_PER_SOL)),
        (counter_pda, initialized_counter_account(0, &authority)),
    ];

    let ix = anchor_instruction(
        program_id,
        "decrement",
        &[],
        vec![
            signer_meta(authority),
            writable_meta(counter_pda),
        ],
    );

    let result = mollusk.process_instruction(&ix, &accounts);

    // Should fail due to underflow check
    assert!(
        result.raw_result.is_err(),
        "Decrement from 0 should fail"
    );
}

// =============================================================================
// SET_VALUE TESTS
// =============================================================================

#[test]
fn test_counter_set_value() {
    let mollusk = counter_mollusk();
    let program_id = counter_program_id();

    let authority = Pubkey::new_unique();
    let (counter_pda, _bump) = find_pda(&[b"counter", authority.as_ref()], &program_id);

    let accounts: Vec<(Pubkey, Account)> = vec![
        (authority, system_account(10 * LAMPORTS_PER_SOL)),
        (counter_pda, initialized_counter_account(5, &authority)),
    ];

    // Set value to 42
    let new_value: u64 = 42;
    let ix = anchor_instruction(
        program_id,
        "set_value",
        &new_value.to_le_bytes(),
        vec![
            signer_meta(authority),
            writable_meta(counter_pda),
        ],
    );

    let result = mollusk.process_instruction(&ix, &accounts);

    assert!(
        result.raw_result.is_ok(),
        "Set value should succeed: {:?}",
        result.raw_result
    );

    let counter_account = result.get_account(&counter_pda).unwrap();
    let data = counter_account.data();
    let count_bytes: [u8; 8] = data[8..16].try_into().unwrap();
    let count = u64::from_le_bytes(count_bytes);
    assert_eq!(count, 42, "Counter should be 42 after set_value");
}

#[test]
fn test_counter_set_value_to_zero() {
    let mollusk = counter_mollusk();
    let program_id = counter_program_id();

    let authority = Pubkey::new_unique();
    let (counter_pda, _bump) = find_pda(&[b"counter", authority.as_ref()], &program_id);

    let accounts: Vec<(Pubkey, Account)> = vec![
        (authority, system_account(10 * LAMPORTS_PER_SOL)),
        (counter_pda, initialized_counter_account(100, &authority)),
    ];

    // Set value to 0
    let new_value: u64 = 0;
    let ix = anchor_instruction(
        program_id,
        "set_value",
        &new_value.to_le_bytes(),
        vec![
            signer_meta(authority),
            writable_meta(counter_pda),
        ],
    );

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(result.raw_result.is_ok());

    let counter_account = result.get_account(&counter_pda).unwrap();
    let data = counter_account.data();
    let count_bytes: [u8; 8] = data[8..16].try_into().unwrap();
    let count = u64::from_le_bytes(count_bytes);
    assert_eq!(count, 0, "Counter should be 0 after set_value(0)");
}

#[test]
fn test_counter_set_value_max() {
    let mollusk = counter_mollusk();
    let program_id = counter_program_id();

    let authority = Pubkey::new_unique();
    let (counter_pda, _bump) = find_pda(&[b"counter", authority.as_ref()], &program_id);

    let accounts: Vec<(Pubkey, Account)> = vec![
        (authority, system_account(10 * LAMPORTS_PER_SOL)),
        (counter_pda, initialized_counter_account(0, &authority)),
    ];

    // Set value to max u64
    let new_value: u64 = u64::MAX;
    let ix = anchor_instruction(
        program_id,
        "set_value",
        &new_value.to_le_bytes(),
        vec![
            signer_meta(authority),
            writable_meta(counter_pda),
        ],
    );

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(result.raw_result.is_ok());

    let counter_account = result.get_account(&counter_pda).unwrap();
    let data = counter_account.data();
    let count_bytes: [u8; 8] = data[8..16].try_into().unwrap();
    let count = u64::from_le_bytes(count_bytes);
    assert_eq!(count, u64::MAX, "Counter should be u64::MAX");
}

// =============================================================================
// AUTHORITY VALIDATION TESTS
// =============================================================================

#[test]
fn test_counter_unauthorized_increment() {
    let mollusk = counter_mollusk();
    let program_id = counter_program_id();

    let real_authority = Pubkey::new_unique();
    let fake_authority = Pubkey::new_unique();
    let (counter_pda, _bump) = find_pda(&[b"counter", real_authority.as_ref()], &program_id);

    // Counter owned by real_authority
    let accounts: Vec<(Pubkey, Account)> = vec![
        (fake_authority, system_account(10 * LAMPORTS_PER_SOL)),
        (counter_pda, initialized_counter_account(5, &real_authority)),
    ];

    // Try to increment with fake authority
    let ix = anchor_instruction(
        program_id,
        "increment",
        &[],
        vec![
            signer_meta(fake_authority),
            writable_meta(counter_pda),
        ],
    );

    let result = mollusk.process_instruction(&ix, &accounts);

    // Should fail - wrong authority
    assert!(
        result.raw_result.is_err(),
        "Increment with wrong authority should fail"
    );
}

#[test]
fn test_counter_unauthorized_decrement() {
    let mollusk = counter_mollusk();
    let program_id = counter_program_id();

    let real_authority = Pubkey::new_unique();
    let fake_authority = Pubkey::new_unique();
    let (counter_pda, _bump) = find_pda(&[b"counter", real_authority.as_ref()], &program_id);

    let accounts: Vec<(Pubkey, Account)> = vec![
        (fake_authority, system_account(10 * LAMPORTS_PER_SOL)),
        (counter_pda, initialized_counter_account(5, &real_authority)),
    ];

    let ix = anchor_instruction(
        program_id,
        "decrement",
        &[],
        vec![
            signer_meta(fake_authority),
            writable_meta(counter_pda),
        ],
    );

    let result = mollusk.process_instruction(&ix, &accounts);

    assert!(
        result.raw_result.is_err(),
        "Decrement with wrong authority should fail"
    );
}

#[test]
fn test_counter_unauthorized_set_value() {
    let mollusk = counter_mollusk();
    let program_id = counter_program_id();

    let real_authority = Pubkey::new_unique();
    let fake_authority = Pubkey::new_unique();
    let (counter_pda, _bump) = find_pda(&[b"counter", real_authority.as_ref()], &program_id);

    let accounts: Vec<(Pubkey, Account)> = vec![
        (fake_authority, system_account(10 * LAMPORTS_PER_SOL)),
        (counter_pda, initialized_counter_account(5, &real_authority)),
    ];

    let new_value: u64 = 999;
    let ix = anchor_instruction(
        program_id,
        "set_value",
        &new_value.to_le_bytes(),
        vec![
            signer_meta(fake_authority),
            writable_meta(counter_pda),
        ],
    );

    let result = mollusk.process_instruction(&ix, &accounts);

    assert!(
        result.raw_result.is_err(),
        "Set value with wrong authority should fail"
    );
}
