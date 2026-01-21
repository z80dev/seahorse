//! LiteSVM integration tests for the Seahorse Calculator program
//!
//! These tests verify complete transaction flows including initialization.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile calculator.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// Calculator program ID (from declare_id!)
fn calculator_program_id() -> Pubkey {
    Pubkey::from_str("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS").unwrap()
}

/// Calculator account size: discriminator (8) + owner (32) + display (8)
const CALCULATOR_SIZE: usize = 8 + 32 + 8;

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

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the calculator program into LiteSVM
fn load_calculator_program() -> (litesvm::LiteSVM, Keypair) {
    let program_keypair = Keypair::new();
    let mut svm = litesvm_with_program(&program_keypair, "../../target/deploy/calculator.so");

    // Create a funded owner
    let owner = funded_keypair_10_sol(&mut svm);

    // Override program ID to match declared ID
    let program_id = calculator_program_id();
    let program_bytes = std::fs::read("../../target/deploy/calculator.so").unwrap();
    svm.add_program(program_id, &program_bytes);

    (svm, owner)
}

/// Read display value from calculator account data
fn read_display_value(data: &[u8]) -> i64 {
    // Skip discriminator (8) and owner (32), read display (8)
    let display_bytes: [u8; 8] = data[40..48].try_into().unwrap();
    i64::from_le_bytes(display_bytes)
}

/// Read owner from calculator account data
fn read_calculator_owner(data: &[u8]) -> Pubkey {
    // Skip discriminator (8), read owner (32)
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

// =============================================================================
// INITIALIZE TESTS
// =============================================================================

#[test]
fn test_calculator_initialize() {
    let (mut svm, owner) = load_calculator_program();
    let program_id = calculator_program_id();

    // Derive calculator PDA
    let (calculator_pda, _bump) = find_pda(&[b"Calculator", owner.pubkey().as_ref()], &program_id);

    // Create init_calculator instruction - Seahorse requires rent sysvar
    let ix = anchor_instruction(
        program_id,
        "init_calculator",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    // Execute transaction
    let result = execute_tx(&mut svm, ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Initialize should succeed: {:?}", result);

    // Verify account state
    let account = svm.get_account(&calculator_pda).expect("Calculator should exist");
    assert_eq!(account.data.len(), CALCULATOR_SIZE, "Account size should match");

    let display = read_display_value(&account.data);
    assert_eq!(display, 0, "Display should be initialized to 0");

    let stored_owner = read_calculator_owner(&account.data);
    assert_eq!(stored_owner, owner.pubkey(), "Owner should be set");
}

#[test]
fn test_calculator_initialize_different_users() {
    let program_id = calculator_program_id();
    let program_bytes = std::fs::read("../../target/deploy/calculator.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);

    // Create two different users
    let user1 = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user2 = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    // Derive PDAs for each user
    let (calc1_pda, _) = find_pda(&[b"Calculator", user1.pubkey().as_ref()], &program_id);
    let (calc2_pda, _) = find_pda(&[b"Calculator", user2.pubkey().as_ref()], &program_id);

    // PDAs should be different
    assert_ne!(calc1_pda, calc2_pda);

    // Initialize user1's calculator
    let ix1 = anchor_instruction(
        program_id,
        "init_calculator",
        &[],
        vec![
            signer_meta(user1.pubkey()),
            writable_meta(calc1_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    let result1 = execute_tx(&mut svm, ix1, &user1, &[&user1]);
    assert!(result1.is_ok(), "User1 initialize should succeed");

    // Initialize user2's calculator
    let ix2 = anchor_instruction(
        program_id,
        "init_calculator",
        &[],
        vec![
            signer_meta(user2.pubkey()),
            writable_meta(calc2_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    let result2 = execute_tx(&mut svm, ix2, &user2, &[&user2]);
    assert!(result2.is_ok(), "User2 initialize should succeed");

    // Both calculators should exist independently
    let account1 = svm.get_account(&calc1_pda).unwrap();
    let account2 = svm.get_account(&calc2_pda).unwrap();

    assert_eq!(read_calculator_owner(&account1.data), user1.pubkey());
    assert_eq!(read_calculator_owner(&account2.data), user2.pubkey());
}

// =============================================================================
// FULL WORKFLOW TESTS
// =============================================================================

#[test]
fn test_calculator_full_workflow() {
    let (mut svm, owner) = load_calculator_program();
    let program_id = calculator_program_id();

    let (calculator_pda, _bump) = find_pda(&[b"Calculator", owner.pubkey().as_ref()], &program_id);

    // 1. Initialize
    let init_ix = anchor_instruction(
        program_id,
        "init_calculator",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    let result = execute_tx(&mut svm, init_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Initialize should succeed");

    // Verify initial state
    let account = svm.get_account(&calculator_pda).unwrap();
    assert_eq!(read_display_value(&account.data), 0);

    // 2. Add 10
    svm.expire_blockhash();
    let add_ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Add, 10),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
        ],
    );
    let result = execute_tx(&mut svm, add_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Add should succeed");

    let account = svm.get_account(&calculator_pda).unwrap();
    assert_eq!(read_display_value(&account.data), 10, "Display should be 10 after adding 10");

    // 3. Multiply by 5
    svm.expire_blockhash();
    let mul_ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Mul, 5),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
        ],
    );
    let result = execute_tx(&mut svm, mul_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Multiply should succeed");

    let account = svm.get_account(&calculator_pda).unwrap();
    assert_eq!(read_display_value(&account.data), 50, "Display should be 50 (10 * 5)");

    // 4. Subtract 8
    svm.expire_blockhash();
    let sub_ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Sub, 8),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
        ],
    );
    let result = execute_tx(&mut svm, sub_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Subtract should succeed");

    let account = svm.get_account(&calculator_pda).unwrap();
    assert_eq!(read_display_value(&account.data), 42, "Display should be 42 (50 - 8)");

    // 5. Divide by 6
    svm.expire_blockhash();
    let div_ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Div, 6),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
        ],
    );
    let result = execute_tx(&mut svm, div_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Divide should succeed");

    let account = svm.get_account(&calculator_pda).unwrap();
    assert_eq!(read_display_value(&account.data), 7, "Display should be 7 (42 / 6)");

    // 6. Reset
    svm.expire_blockhash();
    let reset_ix = anchor_instruction(
        program_id,
        "reset_calculator",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
        ],
    );
    let result = execute_tx(&mut svm, reset_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Reset should succeed");

    let account = svm.get_account(&calculator_pda).unwrap();
    assert_eq!(read_display_value(&account.data), 0, "Display should be 0 after reset");
}

// =============================================================================
// OPERATION TESTS
// =============================================================================

#[test]
fn test_calculator_add() {
    let (mut svm, owner) = load_calculator_program();
    let program_id = calculator_program_id();
    let (calculator_pda, _) = find_pda(&[b"Calculator", owner.pubkey().as_ref()], &program_id);

    // Initialize
    let init_ix = anchor_instruction(
        program_id,
        "init_calculator",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // Add 42
    let add_ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Add, 42),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
        ],
    );
    let result = execute_tx(&mut svm, add_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Add should succeed");

    let account = svm.get_account(&calculator_pda).unwrap();
    assert_eq!(read_display_value(&account.data), 42);
}

#[test]
fn test_calculator_sub() {
    let (mut svm, owner) = load_calculator_program();
    let program_id = calculator_program_id();
    let (calculator_pda, _) = find_pda(&[b"Calculator", owner.pubkey().as_ref()], &program_id);

    // Initialize and add 100
    let init_ix = anchor_instruction(
        program_id,
        "init_calculator",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    let add_ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Add, 100),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
        ],
    );
    execute_tx(&mut svm, add_ix, &owner, &[&owner]).unwrap();

    // Subtract 58
    svm.expire_blockhash();
    let sub_ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Sub, 58),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
        ],
    );
    let result = execute_tx(&mut svm, sub_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Sub should succeed");

    let account = svm.get_account(&calculator_pda).unwrap();
    assert_eq!(read_display_value(&account.data), 42);
}

#[test]
fn test_calculator_mul() {
    let (mut svm, owner) = load_calculator_program();
    let program_id = calculator_program_id();
    let (calculator_pda, _) = find_pda(&[b"Calculator", owner.pubkey().as_ref()], &program_id);

    // Initialize and add 7
    let init_ix = anchor_instruction(
        program_id,
        "init_calculator",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    let add_ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Add, 7),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
        ],
    );
    execute_tx(&mut svm, add_ix, &owner, &[&owner]).unwrap();

    // Multiply by 6
    svm.expire_blockhash();
    let mul_ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Mul, 6),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
        ],
    );
    let result = execute_tx(&mut svm, mul_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Mul should succeed");

    let account = svm.get_account(&calculator_pda).unwrap();
    assert_eq!(read_display_value(&account.data), 42);
}

#[test]
fn test_calculator_div() {
    let (mut svm, owner) = load_calculator_program();
    let program_id = calculator_program_id();
    let (calculator_pda, _) = find_pda(&[b"Calculator", owner.pubkey().as_ref()], &program_id);

    // Initialize and add 84
    let init_ix = anchor_instruction(
        program_id,
        "init_calculator",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    let add_ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Add, 84),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
        ],
    );
    execute_tx(&mut svm, add_ix, &owner, &[&owner]).unwrap();

    // Divide by 2
    svm.expire_blockhash();
    let div_ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Div, 2),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
        ],
    );
    let result = execute_tx(&mut svm, div_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Div should succeed");

    let account = svm.get_account(&calculator_pda).unwrap();
    assert_eq!(read_display_value(&account.data), 42);
}

#[test]
fn test_calculator_div_by_zero_fails() {
    let (mut svm, owner) = load_calculator_program();
    let program_id = calculator_program_id();
    let (calculator_pda, _) = find_pda(&[b"Calculator", owner.pubkey().as_ref()], &program_id);

    // Initialize
    let init_ix = anchor_instruction(
        program_id,
        "init_calculator",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // Add some value
    let add_ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Add, 42),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
        ],
    );
    execute_tx(&mut svm, add_ix, &owner, &[&owner]).unwrap();

    // Attempt to divide by 0 - should fail
    svm.expire_blockhash();
    let div_ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Div, 0),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
        ],
    );
    let result = execute_tx(&mut svm, div_ix, &owner, &[&owner]);
    assert!(result.is_err(), "Division by zero should fail");
}

// =============================================================================
// RESET TESTS
// =============================================================================

#[test]
fn test_calculator_reset() {
    let (mut svm, owner) = load_calculator_program();
    let program_id = calculator_program_id();
    let (calculator_pda, _) = find_pda(&[b"Calculator", owner.pubkey().as_ref()], &program_id);

    // Initialize and add value
    let init_ix = anchor_instruction(
        program_id,
        "init_calculator",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    let add_ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Add, 999),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
        ],
    );
    execute_tx(&mut svm, add_ix, &owner, &[&owner]).unwrap();

    // Reset
    svm.expire_blockhash();
    let reset_ix = anchor_instruction(
        program_id,
        "reset_calculator",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
        ],
    );
    let result = execute_tx(&mut svm, reset_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Reset should succeed");

    let account = svm.get_account(&calculator_pda).unwrap();
    assert_eq!(read_display_value(&account.data), 0);
}

// =============================================================================
// AUTHORIZATION TESTS
// =============================================================================

#[test]
fn test_calculator_unauthorized_operation_fails() {
    let program_id = calculator_program_id();
    let program_bytes = std::fs::read("../../target/deploy/calculator.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);

    let owner = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let attacker = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    // Owner initializes their calculator
    let (calculator_pda, _) = find_pda(&[b"Calculator", owner.pubkey().as_ref()], &program_id);

    let init_ix = anchor_instruction(
        program_id,
        "init_calculator",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // Attacker tries to operate on owner's calculator - should fail
    let add_ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Add, 100),
        vec![
            signer_meta(attacker.pubkey()),
            writable_meta(calculator_pda),
        ],
    );
    let result = execute_tx(&mut svm, add_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "Unauthorized operation should fail");
}

#[test]
fn test_calculator_unauthorized_reset_fails() {
    let program_id = calculator_program_id();
    let program_bytes = std::fs::read("../../target/deploy/calculator.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);

    let owner = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let attacker = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    let (calculator_pda, _) = find_pda(&[b"Calculator", owner.pubkey().as_ref()], &program_id);

    // Owner initializes and adds value
    let init_ix = anchor_instruction(
        program_id,
        "init_calculator",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    let add_ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Add, 999),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
        ],
    );
    execute_tx(&mut svm, add_ix, &owner, &[&owner]).unwrap();

    // Attacker tries to reset - should fail
    let reset_ix = anchor_instruction(
        program_id,
        "reset_calculator",
        &[],
        vec![
            signer_meta(attacker.pubkey()),
            writable_meta(calculator_pda),
        ],
    );
    let result = execute_tx(&mut svm, reset_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "Unauthorized reset should fail");

    // Verify value unchanged
    let account = svm.get_account(&calculator_pda).unwrap();
    assert_eq!(read_display_value(&account.data), 999, "Value should remain 999");
}

// =============================================================================
// NEGATIVE VALUE TESTS
// =============================================================================

#[test]
fn test_calculator_negative_values() {
    let (mut svm, owner) = load_calculator_program();
    let program_id = calculator_program_id();
    let (calculator_pda, _) = find_pda(&[b"Calculator", owner.pubkey().as_ref()], &program_id);

    // Initialize
    let init_ix = anchor_instruction(
        program_id,
        "init_calculator",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // Subtract 10 (from 0, result is -10)
    let sub_ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Sub, 10),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
        ],
    );
    let result = execute_tx(&mut svm, sub_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Subtract should succeed");

    let account = svm.get_account(&calculator_pda).unwrap();
    assert_eq!(read_display_value(&account.data), -10, "Display should be -10");

    // Multiply by -3
    svm.expire_blockhash();
    let mul_ix = anchor_instruction(
        program_id,
        "do_operation",
        &operation_data(Operation::Mul, -3),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(calculator_pda),
        ],
    );
    let result = execute_tx(&mut svm, mul_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Multiply should succeed");

    let account = svm.get_account(&calculator_pda).unwrap();
    assert_eq!(read_display_value(&account.data), 30, "Display should be 30 (-10 * -3)");
}
