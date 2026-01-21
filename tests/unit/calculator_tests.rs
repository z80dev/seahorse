//! Mollusk unit tests for the Seahorse Calculator program
//!
//! These tests verify each operation (ADD, SUB, MUL, DIV) in isolation using Mollusk.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile calculator.so

use mollusk_svm::Mollusk;
use seahorse_unit_tests::helpers::*;
use seahorse_unit_tests::*;
use solana_sdk::{
    account::Account,
    account::ReadableAccount,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    rent::Rent,
};
use std::str::FromStr;

/// Calculator program ID (from declare_id!)
fn calculator_program_id() -> Pubkey {
    Pubkey::from_str("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS").unwrap()
}

/// Create a Mollusk instance for the calculator program
fn calculator_mollusk() -> Mollusk {
    mollusk_for_program(&calculator_program_id(), "../../target/deploy/calculator")
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

/// Calculator account layout:
/// - discriminator: 8 bytes
/// - owner: 32 bytes (Pubkey)
/// - display: 8 bytes (i64)
const CALCULATOR_DATA_SIZE: usize = 8 + 32 + 8;

/// Create an initialized calculator account
fn initialized_calculator_account(display: i64, owner: &Pubkey) -> Account {
    let mut data = Vec::with_capacity(CALCULATOR_DATA_SIZE);

    // Discriminator for "Calculator"
    let disc = anchor_discriminator("Calculator");
    data.extend_from_slice(&disc);

    // owner: Pubkey
    data.extend_from_slice(owner.as_ref());

    // display: i64
    data.extend_from_slice(&display.to_le_bytes());

    let rent = Rent::default();
    Account {
        lamports: rent.minimum_balance(CALCULATOR_DATA_SIZE),
        data,
        owner: calculator_program_id(),
        executable: false,
        rent_epoch: 0,
    }
}

/// Operation enum values matching Seahorse-generated code
#[repr(u8)]
enum Operation {
    Add = 0,
    Sub = 1,
    Mul = 2,
    Div = 3,
}

/// Create instruction data for do_operation: op (1 byte) + num (8 bytes as i64)
fn operation_data(op: Operation, num: i64) -> Vec<u8> {
    let mut data = Vec::with_capacity(9);
    data.push(op as u8);
    data.extend_from_slice(&num.to_le_bytes());
    data
}

/// Read display value from calculator account data
fn read_display_value(data: &[u8]) -> i64 {
    // Skip discriminator (8) and owner (32), read display (8)
    let display_bytes: [u8; 8] = data[40..48].try_into().unwrap();
    i64::from_le_bytes(display_bytes)
}

// =============================================================================
// ADD OPERATION TESTS
// =============================================================================

#[test]
fn test_calculator_add_positive() {
    let mollusk = calculator_mollusk();
    let program_id = calculator_program_id();

    let owner = Pubkey::new_unique();
    let (calculator_pda, _bump) = find_pda(&[b"Calculator", owner.as_ref()], &program_id);

    // Start with display = 10
    let accounts: Vec<(Pubkey, Account)> = vec![
        (owner, system_account(10 * LAMPORTS_PER_SOL)),
        (calculator_pda, initialized_calculator_account(10, &owner)),
    ];

    // Add 5
    let ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Add, 5),
        vec![signer_meta(owner), writable_meta(calculator_pda)],
    );

    let result = mollusk.process_instruction(&ix, &accounts);

    assert!(
        result.raw_result.is_ok(),
        "Add should succeed: {:?}",
        result.raw_result
    );

    let calculator_account = result.get_account(&calculator_pda).expect("Calculator account should exist");
    let display = read_display_value(calculator_account.data());
    assert_eq!(display, 15, "10 + 5 = 15");
}

#[test]
fn test_calculator_add_negative() {
    let mollusk = calculator_mollusk();
    let program_id = calculator_program_id();

    let owner = Pubkey::new_unique();
    let (calculator_pda, _bump) = find_pda(&[b"Calculator", owner.as_ref()], &program_id);

    // Start with display = 10
    let accounts: Vec<(Pubkey, Account)> = vec![
        (owner, system_account(10 * LAMPORTS_PER_SOL)),
        (calculator_pda, initialized_calculator_account(10, &owner)),
    ];

    // Add -3 (effectively subtract 3)
    let ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Add, -3),
        vec![signer_meta(owner), writable_meta(calculator_pda)],
    );

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(result.raw_result.is_ok());

    let calculator_account = result.get_account(&calculator_pda).unwrap();
    let display = read_display_value(calculator_account.data());
    assert_eq!(display, 7, "10 + (-3) = 7");
}

#[test]
fn test_calculator_add_zero() {
    let mollusk = calculator_mollusk();
    let program_id = calculator_program_id();

    let owner = Pubkey::new_unique();
    let (calculator_pda, _bump) = find_pda(&[b"Calculator", owner.as_ref()], &program_id);

    let accounts: Vec<(Pubkey, Account)> = vec![
        (owner, system_account(10 * LAMPORTS_PER_SOL)),
        (calculator_pda, initialized_calculator_account(42, &owner)),
    ];

    // Add 0
    let ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Add, 0),
        vec![signer_meta(owner), writable_meta(calculator_pda)],
    );

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(result.raw_result.is_ok());

    let calculator_account = result.get_account(&calculator_pda).unwrap();
    let display = read_display_value(calculator_account.data());
    assert_eq!(display, 42, "42 + 0 = 42");
}

// =============================================================================
// SUB OPERATION TESTS
// =============================================================================

#[test]
fn test_calculator_sub_positive() {
    let mollusk = calculator_mollusk();
    let program_id = calculator_program_id();

    let owner = Pubkey::new_unique();
    let (calculator_pda, _bump) = find_pda(&[b"Calculator", owner.as_ref()], &program_id);

    // Start with display = 20
    let accounts: Vec<(Pubkey, Account)> = vec![
        (owner, system_account(10 * LAMPORTS_PER_SOL)),
        (calculator_pda, initialized_calculator_account(20, &owner)),
    ];

    // Subtract 8
    let ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Sub, 8),
        vec![signer_meta(owner), writable_meta(calculator_pda)],
    );

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(result.raw_result.is_ok(), "Sub should succeed: {:?}", result.raw_result);

    let calculator_account = result.get_account(&calculator_pda).unwrap();
    let display = read_display_value(calculator_account.data());
    assert_eq!(display, 12, "20 - 8 = 12");
}

#[test]
fn test_calculator_sub_to_negative() {
    let mollusk = calculator_mollusk();
    let program_id = calculator_program_id();

    let owner = Pubkey::new_unique();
    let (calculator_pda, _bump) = find_pda(&[b"Calculator", owner.as_ref()], &program_id);

    // Start with display = 5
    let accounts: Vec<(Pubkey, Account)> = vec![
        (owner, system_account(10 * LAMPORTS_PER_SOL)),
        (calculator_pda, initialized_calculator_account(5, &owner)),
    ];

    // Subtract 10 (result is -5)
    let ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Sub, 10),
        vec![signer_meta(owner), writable_meta(calculator_pda)],
    );

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(result.raw_result.is_ok());

    let calculator_account = result.get_account(&calculator_pda).unwrap();
    let display = read_display_value(calculator_account.data());
    assert_eq!(display, -5, "5 - 10 = -5");
}

// =============================================================================
// MUL OPERATION TESTS
// =============================================================================

#[test]
fn test_calculator_mul_positive() {
    let mollusk = calculator_mollusk();
    let program_id = calculator_program_id();

    let owner = Pubkey::new_unique();
    let (calculator_pda, _bump) = find_pda(&[b"Calculator", owner.as_ref()], &program_id);

    // Start with display = 7
    let accounts: Vec<(Pubkey, Account)> = vec![
        (owner, system_account(10 * LAMPORTS_PER_SOL)),
        (calculator_pda, initialized_calculator_account(7, &owner)),
    ];

    // Multiply by 6
    let ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Mul, 6),
        vec![signer_meta(owner), writable_meta(calculator_pda)],
    );

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(result.raw_result.is_ok(), "Mul should succeed: {:?}", result.raw_result);

    let calculator_account = result.get_account(&calculator_pda).unwrap();
    let display = read_display_value(calculator_account.data());
    assert_eq!(display, 42, "7 * 6 = 42");
}

#[test]
fn test_calculator_mul_by_zero() {
    let mollusk = calculator_mollusk();
    let program_id = calculator_program_id();

    let owner = Pubkey::new_unique();
    let (calculator_pda, _bump) = find_pda(&[b"Calculator", owner.as_ref()], &program_id);

    let accounts: Vec<(Pubkey, Account)> = vec![
        (owner, system_account(10 * LAMPORTS_PER_SOL)),
        (calculator_pda, initialized_calculator_account(999, &owner)),
    ];

    // Multiply by 0
    let ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Mul, 0),
        vec![signer_meta(owner), writable_meta(calculator_pda)],
    );

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(result.raw_result.is_ok());

    let calculator_account = result.get_account(&calculator_pda).unwrap();
    let display = read_display_value(calculator_account.data());
    assert_eq!(display, 0, "999 * 0 = 0");
}

#[test]
fn test_calculator_mul_negative() {
    let mollusk = calculator_mollusk();
    let program_id = calculator_program_id();

    let owner = Pubkey::new_unique();
    let (calculator_pda, _bump) = find_pda(&[b"Calculator", owner.as_ref()], &program_id);

    let accounts: Vec<(Pubkey, Account)> = vec![
        (owner, system_account(10 * LAMPORTS_PER_SOL)),
        (calculator_pda, initialized_calculator_account(5, &owner)),
    ];

    // Multiply by -3
    let ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Mul, -3),
        vec![signer_meta(owner), writable_meta(calculator_pda)],
    );

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(result.raw_result.is_ok());

    let calculator_account = result.get_account(&calculator_pda).unwrap();
    let display = read_display_value(calculator_account.data());
    assert_eq!(display, -15, "5 * (-3) = -15");
}

// =============================================================================
// DIV OPERATION TESTS
// =============================================================================

#[test]
fn test_calculator_div_positive() {
    let mollusk = calculator_mollusk();
    let program_id = calculator_program_id();

    let owner = Pubkey::new_unique();
    let (calculator_pda, _bump) = find_pda(&[b"Calculator", owner.as_ref()], &program_id);

    // Start with display = 42
    let accounts: Vec<(Pubkey, Account)> = vec![
        (owner, system_account(10 * LAMPORTS_PER_SOL)),
        (calculator_pda, initialized_calculator_account(42, &owner)),
    ];

    // Divide by 6
    let ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Div, 6),
        vec![signer_meta(owner), writable_meta(calculator_pda)],
    );

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(result.raw_result.is_ok(), "Div should succeed: {:?}", result.raw_result);

    let calculator_account = result.get_account(&calculator_pda).unwrap();
    let display = read_display_value(calculator_account.data());
    assert_eq!(display, 7, "42 / 6 = 7");
}

#[test]
fn test_calculator_div_truncation() {
    let mollusk = calculator_mollusk();
    let program_id = calculator_program_id();

    let owner = Pubkey::new_unique();
    let (calculator_pda, _bump) = find_pda(&[b"Calculator", owner.as_ref()], &program_id);

    // Start with display = 10
    let accounts: Vec<(Pubkey, Account)> = vec![
        (owner, system_account(10 * LAMPORTS_PER_SOL)),
        (calculator_pda, initialized_calculator_account(10, &owner)),
    ];

    // Divide by 3 (truncates to 3, not 3.33)
    let ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Div, 3),
        vec![signer_meta(owner), writable_meta(calculator_pda)],
    );

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(result.raw_result.is_ok());

    let calculator_account = result.get_account(&calculator_pda).unwrap();
    let display = read_display_value(calculator_account.data());
    assert_eq!(display, 3, "10 / 3 = 3 (integer division)");
}

#[test]
fn test_calculator_div_by_zero_panics() {
    let mollusk = calculator_mollusk();
    let program_id = calculator_program_id();

    let owner = Pubkey::new_unique();
    let (calculator_pda, _bump) = find_pda(&[b"Calculator", owner.as_ref()], &program_id);

    let accounts: Vec<(Pubkey, Account)> = vec![
        (owner, system_account(10 * LAMPORTS_PER_SOL)),
        (calculator_pda, initialized_calculator_account(42, &owner)),
    ];

    // Divide by 0 - should fail
    let ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Div, 0),
        vec![signer_meta(owner), writable_meta(calculator_pda)],
    );

    let result = mollusk.process_instruction(&ix, &accounts);

    // Division by zero should fail
    assert!(
        result.raw_result.is_err(),
        "Division by zero should fail"
    );
}

// =============================================================================
// RESET TESTS
// =============================================================================

#[test]
fn test_calculator_reset() {
    let mollusk = calculator_mollusk();
    let program_id = calculator_program_id();

    let owner = Pubkey::new_unique();
    let (calculator_pda, _bump) = find_pda(&[b"Calculator", owner.as_ref()], &program_id);

    // Start with display = 999
    let accounts: Vec<(Pubkey, Account)> = vec![
        (owner, system_account(10 * LAMPORTS_PER_SOL)),
        (calculator_pda, initialized_calculator_account(999, &owner)),
    ];

    // Reset
    let ix = anchor_instruction(
        program_id,
        "reset_calculator",
        &[],
        vec![signer_meta(owner), writable_meta(calculator_pda)],
    );

    let result = mollusk.process_instruction(&ix, &accounts);
    assert!(result.raw_result.is_ok(), "Reset should succeed: {:?}", result.raw_result);

    let calculator_account = result.get_account(&calculator_pda).unwrap();
    let display = read_display_value(calculator_account.data());
    assert_eq!(display, 0, "Display should be 0 after reset");
}

// =============================================================================
// AUTHORIZATION TESTS
// =============================================================================

#[test]
fn test_calculator_unauthorized_operation() {
    let mollusk = calculator_mollusk();
    let program_id = calculator_program_id();

    let real_owner = Pubkey::new_unique();
    let fake_owner = Pubkey::new_unique();
    let (calculator_pda, _bump) = find_pda(&[b"Calculator", real_owner.as_ref()], &program_id);

    // Calculator owned by real_owner
    let accounts: Vec<(Pubkey, Account)> = vec![
        (fake_owner, system_account(10 * LAMPORTS_PER_SOL)),
        (calculator_pda, initialized_calculator_account(10, &real_owner)),
    ];

    // Try to add with fake_owner
    let ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Add, 5),
        vec![signer_meta(fake_owner), writable_meta(calculator_pda)],
    );

    let result = mollusk.process_instruction(&ix, &accounts);

    // Should fail - wrong owner
    assert!(
        result.raw_result.is_err(),
        "Operation with wrong owner should fail"
    );
}

#[test]
fn test_calculator_unauthorized_reset() {
    let mollusk = calculator_mollusk();
    let program_id = calculator_program_id();

    let real_owner = Pubkey::new_unique();
    let fake_owner = Pubkey::new_unique();
    let (calculator_pda, _bump) = find_pda(&[b"Calculator", real_owner.as_ref()], &program_id);

    let accounts: Vec<(Pubkey, Account)> = vec![
        (fake_owner, system_account(10 * LAMPORTS_PER_SOL)),
        (calculator_pda, initialized_calculator_account(999, &real_owner)),
    ];

    // Try to reset with fake_owner
    let ix = anchor_instruction(
        program_id,
        "reset_calculator",
        &[],
        vec![signer_meta(fake_owner), writable_meta(calculator_pda)],
    );

    let result = mollusk.process_instruction(&ix, &accounts);

    assert!(
        result.raw_result.is_err(),
        "Reset with wrong owner should fail"
    );
}
