//! LiteSVM integration tests for the Seahorse Errors program
//!
//! These tests verify error handling patterns:
//! - Custom error messages via assert statements
//! - Authorization checks with error messages
//! - Numeric validation (range, overflow, underflow)
//! - String length validation
//! - State transition validation
//! - Role-based access control
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile errors.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::path::Path;
use std::str::FromStr;

/// Errors program ID (from declare_id!)
fn program_id() -> Pubkey {
    Pubkey::from_str("ErrDemo111111111111111111111111111111111111").unwrap()
}

/// Path to the compiled program
const PROGRAM_PATH: &str = "../../target/deploy/errors.so";

/// Check if the program is built
fn program_exists() -> bool {
    Path::new(PROGRAM_PATH).exists()
}

/// Macro to skip tests if program doesn't exist
macro_rules! skip_if_not_built {
    () => {
        if !program_exists() {
            eprintln!("SKIPPED: errors.so not found - run ./scripts/build-test-programs.sh first");
            return;
        }
    };
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the errors program into LiteSVM
fn load_program() -> litesvm::LiteSVM {
    let prog_id = program_id();

    let program_bytes =
        std::fs::read(PROGRAM_PATH).expect("Failed to read errors.so");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(prog_id, &program_bytes);
    svm
}

/// Derive error_demo PDA
fn derive_error_demo_pda(authority: &Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"error_demo", authority.as_ref()], &program_id())
}

/// Derive role_registry PDA
fn derive_role_registry_pda(registry_id: u64) -> (Pubkey, u8) {
    find_pda(
        &[b"role_registry", &registry_id.to_le_bytes()],
        &program_id(),
    )
}

/// Build initialize instruction data
fn initialize_data(max_value: u64, name: &str) -> Vec<u8> {
    let mut data = Vec::new();
    // max_value: u64
    data.extend_from_slice(&max_value.to_le_bytes());
    // name: String (length prefix + bytes)
    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());
    data
}

/// Build set_value instruction data
fn set_value_data(new_value: u64) -> Vec<u8> {
    new_value.to_le_bytes().to_vec()
}

/// Build increment_value instruction data
fn increment_data(amount: u64) -> Vec<u8> {
    amount.to_le_bytes().to_vec()
}

/// Build decrement_value instruction data
fn decrement_data(amount: u64) -> Vec<u8> {
    amount.to_le_bytes().to_vec()
}

/// Build update_name instruction data
fn update_name_data(new_name: &str) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&(new_name.len() as u32).to_le_bytes());
    data.extend_from_slice(new_name.as_bytes());
    data
}

/// Build initialize_role_registry instruction data
fn init_registry_data(registry_id: u64) -> Vec<u8> {
    registry_id.to_le_bytes().to_vec()
}

/// Build set_operator instruction data
fn set_operator_data(new_operator: &Pubkey) -> Vec<u8> {
    new_operator.as_ref().to_vec()
}

/// Build complex_validation instruction data
fn complex_validation_data(new_value: u64, new_name: &str, require_high_value: bool) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&new_value.to_le_bytes());
    data.extend_from_slice(&(new_name.len() as u32).to_le_bytes());
    data.extend_from_slice(new_name.as_bytes());
    data.push(if require_high_value { 1 } else { 0 });
    data
}

/// Read value from ErrorDemo account data
/// Account layout: discriminator (8) + authority (32) + value (8) + operation_count (8) + is_active (1) + name (4+len) + max_value (8) + bump (1)
fn read_value(data: &[u8]) -> u64 {
    let value_bytes: [u8; 8] = data[40..48].try_into().unwrap();
    u64::from_le_bytes(value_bytes)
}

/// Read operation_count from ErrorDemo account data
fn read_operation_count(data: &[u8]) -> u64 {
    let count_bytes: [u8; 8] = data[48..56].try_into().unwrap();
    u64::from_le_bytes(count_bytes)
}

/// Read is_active from ErrorDemo account data
fn read_is_active(data: &[u8]) -> bool {
    data[56] != 0
}

/// Read max_value from ErrorDemo account data (after name string)
fn read_max_value(data: &[u8]) -> u64 {
    // First, read name length at offset 57
    let name_len_bytes: [u8; 4] = data[57..61].try_into().unwrap();
    let name_len = u32::from_le_bytes(name_len_bytes) as usize;
    // max_value is after the name string
    let max_value_offset = 61 + name_len;
    let max_value_bytes: [u8; 8] = data[max_value_offset..max_value_offset + 8]
        .try_into()
        .unwrap();
    u64::from_le_bytes(max_value_bytes)
}

/// Read authority from ErrorDemo account data
fn read_authority(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

// =============================================================================
// INITIALIZE TESTS
// =============================================================================

#[test]
fn test_errors_initialize_success() {
    skip_if_not_built!();
    let mut svm = load_program();
    let authority = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let (error_demo_pda, _) = derive_error_demo_pda(&authority.pubkey());

    let ix = anchor_instruction(
        prog_id,
        "initialize",
        &initialize_data(1000, "TestDemo"),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Initialize should succeed: {:?}", result);

    // Verify account state
    let account = svm
        .get_account(&error_demo_pda)
        .expect("ErrorDemo should exist");
    let stored_authority = read_authority(&account.data);
    assert_eq!(stored_authority, authority.pubkey());

    let value = read_value(&account.data);
    assert_eq!(value, 0, "Initial value should be 0");

    let is_active = read_is_active(&account.data);
    assert!(is_active, "Account should be active");
}

#[test]
fn test_errors_initialize_max_value_zero_fails() {
    skip_if_not_built!();
    let mut svm = load_program();
    let authority = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let (error_demo_pda, _) = derive_error_demo_pda(&authority.pubkey());

    let ix = anchor_instruction(
        prog_id,
        "initialize",
        &initialize_data(0, "TestDemo"), // max_value = 0
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(
        result.is_err(),
        "Initialize with max_value=0 should fail"
    );
}

#[test]
fn test_errors_initialize_max_value_exceeds_limit_fails() {
    skip_if_not_built!();
    let mut svm = load_program();
    let authority = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let (error_demo_pda, _) = derive_error_demo_pda(&authority.pubkey());

    let ix = anchor_instruction(
        prog_id,
        "initialize",
        &initialize_data(2_000_000, "TestDemo"), // max_value > 1,000,000
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(
        result.is_err(),
        "Initialize with max_value > 1,000,000 should fail"
    );
}

#[test]
fn test_errors_initialize_empty_name_fails() {
    skip_if_not_built!();
    let mut svm = load_program();
    let authority = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let (error_demo_pda, _) = derive_error_demo_pda(&authority.pubkey());

    let ix = anchor_instruction(
        prog_id,
        "initialize",
        &initialize_data(1000, ""), // empty name
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "Initialize with empty name should fail");
}

#[test]
fn test_errors_initialize_name_too_long_fails() {
    skip_if_not_built!();
    let mut svm = load_program();
    let authority = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let (error_demo_pda, _) = derive_error_demo_pda(&authority.pubkey());

    let long_name = "a".repeat(33); // 33 characters > 32 limit
    let ix = anchor_instruction(
        prog_id,
        "initialize",
        &initialize_data(1000, &long_name),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "Initialize with name > 32 chars should fail");
}

// =============================================================================
// SET_VALUE TESTS
// =============================================================================

fn setup_error_demo(svm: &mut litesvm::LiteSVM) -> (solana_keypair::Keypair, Pubkey) {
    let authority = funded_keypair_10_sol(svm);
    let prog_id = program_id();
    let (error_demo_pda, _) = derive_error_demo_pda(&authority.pubkey());

    let ix = anchor_instruction(
        prog_id,
        "initialize",
        &initialize_data(1000, "TestDemo"),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    execute_tx(svm, ix, &authority, &[&authority]).expect("Setup should succeed");

    (authority, error_demo_pda)
}

#[test]
fn test_errors_set_value_success() {
    skip_if_not_built!();
    let mut svm = load_program();
    let (authority, error_demo_pda) = setup_error_demo(&mut svm);
    let prog_id = program_id();

    let ix = anchor_instruction(
        prog_id,
        "set_value",
        &set_value_data(500),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "set_value should succeed: {:?}", result);

    let account = svm.get_account(&error_demo_pda).unwrap();
    let value = read_value(&account.data);
    assert_eq!(value, 500, "Value should be 500");
}

#[test]
fn test_errors_set_value_unauthorized_fails() {
    skip_if_not_built!();
    let mut svm = load_program();
    let (_, error_demo_pda) = setup_error_demo(&mut svm);
    let other_user = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let ix = anchor_instruction(
        prog_id,
        "set_value",
        &set_value_data(500),
        vec![
            signer_meta(other_user.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );

    let result = execute_tx(&mut svm, ix, &other_user, &[&other_user]);
    assert!(result.is_err(), "Unauthorized set_value should fail");
}

#[test]
fn test_errors_set_value_exceeds_max_fails() {
    skip_if_not_built!();
    let mut svm = load_program();
    let (authority, error_demo_pda) = setup_error_demo(&mut svm);
    let prog_id = program_id();

    let ix = anchor_instruction(
        prog_id,
        "set_value",
        &set_value_data(1001), // exceeds max_value of 1000
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "set_value exceeding max should fail");
}

// =============================================================================
// INCREMENT_VALUE TESTS
// =============================================================================

#[test]
fn test_errors_increment_value_success() {
    skip_if_not_built!();
    let mut svm = load_program();
    let (authority, error_demo_pda) = setup_error_demo(&mut svm);
    let prog_id = program_id();

    let ix = anchor_instruction(
        prog_id,
        "increment_value",
        &increment_data(100),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "increment_value should succeed: {:?}", result);

    let account = svm.get_account(&error_demo_pda).unwrap();
    let value = read_value(&account.data);
    assert_eq!(value, 100, "Value should be 100");
}

#[test]
fn test_errors_increment_value_zero_fails() {
    skip_if_not_built!();
    let mut svm = load_program();
    let (authority, error_demo_pda) = setup_error_demo(&mut svm);
    let prog_id = program_id();

    let ix = anchor_instruction(
        prog_id,
        "increment_value",
        &increment_data(0), // zero amount
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "increment_value with 0 should fail");
}

#[test]
fn test_errors_increment_value_exceeds_max_fails() {
    skip_if_not_built!();
    let mut svm = load_program();
    let (authority, error_demo_pda) = setup_error_demo(&mut svm);
    let prog_id = program_id();

    // First set value to 900
    let ix1 = anchor_instruction(
        prog_id,
        "set_value",
        &set_value_data(900),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );
    execute_tx(&mut svm, ix1, &authority, &[&authority]).unwrap();

    // Try to increment by 200 (would exceed max_value of 1000)
    let ix2 = anchor_instruction(
        prog_id,
        "increment_value",
        &increment_data(200),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );

    let result = execute_tx(&mut svm, ix2, &authority, &[&authority]);
    assert!(result.is_err(), "increment exceeding max should fail");
}

// =============================================================================
// DECREMENT_VALUE TESTS
// =============================================================================

#[test]
fn test_errors_decrement_value_success() {
    skip_if_not_built!();
    let mut svm = load_program();
    let (authority, error_demo_pda) = setup_error_demo(&mut svm);
    let prog_id = program_id();

    // First set value to 500
    let ix1 = anchor_instruction(
        prog_id,
        "set_value",
        &set_value_data(500),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );
    execute_tx(&mut svm, ix1, &authority, &[&authority]).unwrap();

    // Decrement by 200
    let ix2 = anchor_instruction(
        prog_id,
        "decrement_value",
        &decrement_data(200),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );

    let result = execute_tx(&mut svm, ix2, &authority, &[&authority]);
    assert!(result.is_ok(), "decrement_value should succeed: {:?}", result);

    let account = svm.get_account(&error_demo_pda).unwrap();
    let value = read_value(&account.data);
    assert_eq!(value, 300, "Value should be 300");
}

#[test]
fn test_errors_decrement_value_underflow_fails() {
    skip_if_not_built!();
    let mut svm = load_program();
    let (authority, error_demo_pda) = setup_error_demo(&mut svm);
    let prog_id = program_id();

    // Value is 0, try to decrement by 100
    let ix = anchor_instruction(
        prog_id,
        "decrement_value",
        &decrement_data(100),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "decrement causing underflow should fail");
}

#[test]
fn test_errors_decrement_value_zero_fails() {
    skip_if_not_built!();
    let mut svm = load_program();
    let (authority, error_demo_pda) = setup_error_demo(&mut svm);
    let prog_id = program_id();

    let ix = anchor_instruction(
        prog_id,
        "decrement_value",
        &decrement_data(0), // zero amount
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "decrement_value with 0 should fail");
}

// =============================================================================
// UPDATE_NAME TESTS
// =============================================================================

#[test]
fn test_errors_update_name_success() {
    skip_if_not_built!();
    let mut svm = load_program();
    let (authority, error_demo_pda) = setup_error_demo(&mut svm);
    let prog_id = program_id();

    let ix = anchor_instruction(
        prog_id,
        "update_name",
        &update_name_data("NewName"),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "update_name should succeed: {:?}", result);
}

#[test]
fn test_errors_update_name_empty_fails() {
    skip_if_not_built!();
    let mut svm = load_program();
    let (authority, error_demo_pda) = setup_error_demo(&mut svm);
    let prog_id = program_id();

    let ix = anchor_instruction(
        prog_id,
        "update_name",
        &update_name_data(""),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "update_name with empty name should fail");
}

#[test]
fn test_errors_update_name_too_long_fails() {
    skip_if_not_built!();
    let mut svm = load_program();
    let (authority, error_demo_pda) = setup_error_demo(&mut svm);
    let prog_id = program_id();

    let long_name = "x".repeat(33);
    let ix = anchor_instruction(
        prog_id,
        "update_name",
        &update_name_data(&long_name),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "update_name with name > 32 should fail");
}

// =============================================================================
// DEACTIVATE / REACTIVATE TESTS
// =============================================================================

#[test]
fn test_errors_deactivate_success() {
    skip_if_not_built!();
    let mut svm = load_program();
    let (authority, error_demo_pda) = setup_error_demo(&mut svm);
    let prog_id = program_id();

    let ix = anchor_instruction(
        prog_id,
        "deactivate",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "deactivate should succeed: {:?}", result);

    let account = svm.get_account(&error_demo_pda).unwrap();
    let is_active = read_is_active(&account.data);
    assert!(!is_active, "Account should be inactive");
}

#[test]
fn test_errors_deactivate_already_inactive_fails() {
    skip_if_not_built!();
    let mut svm = load_program();
    let (authority, error_demo_pda) = setup_error_demo(&mut svm);
    let prog_id = program_id();

    // Deactivate first time
    let ix1 = anchor_instruction(
        prog_id,
        "deactivate",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );
    execute_tx(&mut svm, ix1, &authority, &[&authority]).unwrap();

    // Try to deactivate again
    let ix2 = anchor_instruction(
        prog_id,
        "deactivate",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );

    let result = execute_tx(&mut svm, ix2, &authority, &[&authority]);
    assert!(result.is_err(), "deactivating already inactive should fail");
}

#[test]
fn test_errors_reactivate_success() {
    skip_if_not_built!();
    let mut svm = load_program();
    let (authority, error_demo_pda) = setup_error_demo(&mut svm);
    let prog_id = program_id();

    // Deactivate first
    let ix1 = anchor_instruction(
        prog_id,
        "deactivate",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );
    execute_tx(&mut svm, ix1, &authority, &[&authority]).unwrap();

    // Reactivate
    let ix2 = anchor_instruction(
        prog_id,
        "reactivate",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );

    let result = execute_tx(&mut svm, ix2, &authority, &[&authority]);
    assert!(result.is_ok(), "reactivate should succeed: {:?}", result);

    let account = svm.get_account(&error_demo_pda).unwrap();
    let is_active = read_is_active(&account.data);
    assert!(is_active, "Account should be active");
}

#[test]
fn test_errors_reactivate_already_active_fails() {
    skip_if_not_built!();
    let mut svm = load_program();
    let (authority, error_demo_pda) = setup_error_demo(&mut svm);
    let prog_id = program_id();

    // Try to reactivate when already active
    let ix = anchor_instruction(
        prog_id,
        "reactivate",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "reactivating already active should fail");
}

#[test]
fn test_errors_operation_on_inactive_fails() {
    skip_if_not_built!();
    let mut svm = load_program();
    let (authority, error_demo_pda) = setup_error_demo(&mut svm);
    let prog_id = program_id();

    // Deactivate first
    let ix1 = anchor_instruction(
        prog_id,
        "deactivate",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );
    execute_tx(&mut svm, ix1, &authority, &[&authority]).unwrap();

    // Try to set_value on inactive account
    let ix2 = anchor_instruction(
        prog_id,
        "set_value",
        &set_value_data(100),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );

    let result = execute_tx(&mut svm, ix2, &authority, &[&authority]);
    assert!(result.is_err(), "operations on inactive account should fail");
}

// =============================================================================
// ROLE REGISTRY TESTS
// =============================================================================

#[test]
fn test_errors_initialize_role_registry_success() {
    skip_if_not_built!();
    let mut svm = load_program();
    let admin = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let registry_id: u64 = 1;
    let (registry_pda, _) = derive_role_registry_pda(registry_id);

    let ix = anchor_instruction(
        prog_id,
        "initialize_role_registry",
        &init_registry_data(registry_id),
        vec![
            signer_meta(admin.pubkey()),
            writable_meta(registry_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()), // rent sysvar required by Seahorse
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &admin, &[&admin]);
    assert!(
        result.is_ok(),
        "initialize_role_registry should succeed: {:?}",
        result
    );
}

#[test]
fn test_errors_set_operator_success() {
    skip_if_not_built!();
    let mut svm = load_program();
    let admin = funded_keypair_10_sol(&mut svm);
    let operator = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let registry_id: u64 = 2;
    let (registry_pda, _) = derive_role_registry_pda(registry_id);

    // Initialize registry
    let ix1 = anchor_instruction(
        prog_id,
        "initialize_role_registry",
        &init_registry_data(registry_id),
        vec![
            signer_meta(admin.pubkey()),
            writable_meta(registry_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix1, &admin, &[&admin]).unwrap();

    // Set operator
    let ix2 = anchor_instruction(
        prog_id,
        "set_operator",
        &set_operator_data(&operator.pubkey()),
        vec![signer_meta(admin.pubkey()), writable_meta(registry_pda)],
    );

    let result = execute_tx(&mut svm, ix2, &admin, &[&admin]);
    assert!(result.is_ok(), "set_operator should succeed: {:?}", result);
}

#[test]
fn test_errors_set_operator_not_admin_fails() {
    skip_if_not_built!();
    let mut svm = load_program();
    let admin = funded_keypair_10_sol(&mut svm);
    let non_admin = funded_keypair_10_sol(&mut svm);
    let operator = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let registry_id: u64 = 3;
    let (registry_pda, _) = derive_role_registry_pda(registry_id);

    // Initialize registry
    let ix1 = anchor_instruction(
        prog_id,
        "initialize_role_registry",
        &init_registry_data(registry_id),
        vec![
            signer_meta(admin.pubkey()),
            writable_meta(registry_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix1, &admin, &[&admin]).unwrap();

    // Try to set operator as non-admin
    let ix2 = anchor_instruction(
        prog_id,
        "set_operator",
        &set_operator_data(&operator.pubkey()),
        vec![signer_meta(non_admin.pubkey()), writable_meta(registry_pda)],
    );

    let result = execute_tx(&mut svm, ix2, &non_admin, &[&non_admin]);
    assert!(result.is_err(), "set_operator by non-admin should fail");
}

#[test]
fn test_errors_admin_only_action_success() {
    skip_if_not_built!();
    let mut svm = load_program();
    let admin = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let registry_id: u64 = 4;
    let (registry_pda, _) = derive_role_registry_pda(registry_id);

    // Initialize registry
    let ix1 = anchor_instruction(
        prog_id,
        "initialize_role_registry",
        &init_registry_data(registry_id),
        vec![
            signer_meta(admin.pubkey()),
            writable_meta(registry_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix1, &admin, &[&admin]).unwrap();

    // Admin only action
    let ix2 = anchor_instruction(
        prog_id,
        "admin_only_action",
        &[],
        vec![signer_meta(admin.pubkey()), writable_meta(registry_pda)],
    );

    let result = execute_tx(&mut svm, ix2, &admin, &[&admin]);
    assert!(result.is_ok(), "admin_only_action should succeed: {:?}", result);
}

#[test]
fn test_errors_admin_only_action_not_admin_fails() {
    skip_if_not_built!();
    let mut svm = load_program();
    let admin = funded_keypair_10_sol(&mut svm);
    let non_admin = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let registry_id: u64 = 5;
    let (registry_pda, _) = derive_role_registry_pda(registry_id);

    // Initialize registry
    let ix1 = anchor_instruction(
        prog_id,
        "initialize_role_registry",
        &init_registry_data(registry_id),
        vec![
            signer_meta(admin.pubkey()),
            writable_meta(registry_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix1, &admin, &[&admin]).unwrap();

    // Try admin only action as non-admin
    let ix2 = anchor_instruction(
        prog_id,
        "admin_only_action",
        &[],
        vec![signer_meta(non_admin.pubkey()), writable_meta(registry_pda)],
    );

    let result = execute_tx(&mut svm, ix2, &non_admin, &[&non_admin]);
    assert!(result.is_err(), "admin_only_action by non-admin should fail");
}

#[test]
fn test_errors_operator_action_no_operator_set_fails() {
    skip_if_not_built!();
    let mut svm = load_program();
    let admin = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let registry_id: u64 = 6;
    let (registry_pda, _) = derive_role_registry_pda(registry_id);

    // Initialize registry (no operator set)
    let ix1 = anchor_instruction(
        prog_id,
        "initialize_role_registry",
        &init_registry_data(registry_id),
        vec![
            signer_meta(admin.pubkey()),
            writable_meta(registry_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix1, &admin, &[&admin]).unwrap();

    // Try operator action without operator set
    let ix2 = anchor_instruction(
        prog_id,
        "operator_action",
        &[],
        vec![signer_meta(admin.pubkey()), writable_meta(registry_pda)],
    );

    let result = execute_tx(&mut svm, ix2, &admin, &[&admin]);
    assert!(result.is_err(), "operator_action without operator set should fail");
}

#[test]
fn test_errors_operator_action_by_operator_success() {
    skip_if_not_built!();
    let mut svm = load_program();
    let admin = funded_keypair_10_sol(&mut svm);
    let operator = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let registry_id: u64 = 7;
    let (registry_pda, _) = derive_role_registry_pda(registry_id);

    // Initialize registry
    let ix1 = anchor_instruction(
        prog_id,
        "initialize_role_registry",
        &init_registry_data(registry_id),
        vec![
            signer_meta(admin.pubkey()),
            writable_meta(registry_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix1, &admin, &[&admin]).unwrap();

    // Set operator
    let ix2 = anchor_instruction(
        prog_id,
        "set_operator",
        &set_operator_data(&operator.pubkey()),
        vec![signer_meta(admin.pubkey()), writable_meta(registry_pda)],
    );
    execute_tx(&mut svm, ix2, &admin, &[&admin]).unwrap();

    // Operator action by operator
    let ix3 = anchor_instruction(
        prog_id,
        "operator_action",
        &[],
        vec![signer_meta(operator.pubkey()), writable_meta(registry_pda)],
    );

    let result = execute_tx(&mut svm, ix3, &operator, &[&operator]);
    assert!(result.is_ok(), "operator_action by operator should succeed: {:?}", result);
}

#[test]
fn test_errors_operator_action_by_admin_success() {
    skip_if_not_built!();
    let mut svm = load_program();
    let admin = funded_keypair_10_sol(&mut svm);
    let operator = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let registry_id: u64 = 8;
    let (registry_pda, _) = derive_role_registry_pda(registry_id);

    // Initialize registry
    let ix1 = anchor_instruction(
        prog_id,
        "initialize_role_registry",
        &init_registry_data(registry_id),
        vec![
            signer_meta(admin.pubkey()),
            writable_meta(registry_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix1, &admin, &[&admin]).unwrap();

    // Set operator
    let ix2 = anchor_instruction(
        prog_id,
        "set_operator",
        &set_operator_data(&operator.pubkey()),
        vec![signer_meta(admin.pubkey()), writable_meta(registry_pda)],
    );
    execute_tx(&mut svm, ix2, &admin, &[&admin]).unwrap();

    // Operator action by admin (admin also has operator privileges)
    let ix3 = anchor_instruction(
        prog_id,
        "operator_action",
        &[],
        vec![signer_meta(admin.pubkey()), writable_meta(registry_pda)],
    );

    let result = execute_tx(&mut svm, ix3, &admin, &[&admin]);
    assert!(result.is_ok(), "operator_action by admin should succeed: {:?}", result);
}

#[test]
fn test_errors_operator_action_unauthorized_fails() {
    skip_if_not_built!();
    let mut svm = load_program();
    let admin = funded_keypair_10_sol(&mut svm);
    let operator = funded_keypair_10_sol(&mut svm);
    let random_user = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let registry_id: u64 = 9;
    let (registry_pda, _) = derive_role_registry_pda(registry_id);

    // Initialize registry
    let ix1 = anchor_instruction(
        prog_id,
        "initialize_role_registry",
        &init_registry_data(registry_id),
        vec![
            signer_meta(admin.pubkey()),
            writable_meta(registry_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix1, &admin, &[&admin]).unwrap();

    // Set operator
    let ix2 = anchor_instruction(
        prog_id,
        "set_operator",
        &set_operator_data(&operator.pubkey()),
        vec![signer_meta(admin.pubkey()), writable_meta(registry_pda)],
    );
    execute_tx(&mut svm, ix2, &admin, &[&admin]).unwrap();

    // Try operator action by random user
    let ix3 = anchor_instruction(
        prog_id,
        "operator_action",
        &[],
        vec![signer_meta(random_user.pubkey()), writable_meta(registry_pda)],
    );

    let result = execute_tx(&mut svm, ix3, &random_user, &[&random_user]);
    assert!(result.is_err(), "operator_action by random user should fail");
}

// =============================================================================
// COMPLEX VALIDATION TESTS
// =============================================================================

#[test]
fn test_errors_complex_validation_success() {
    skip_if_not_built!();
    let mut svm = load_program();
    let (authority, error_demo_pda) = setup_error_demo(&mut svm);
    let prog_id = program_id();

    let ix = anchor_instruction(
        prog_id,
        "complex_validation",
        &complex_validation_data(500, "NewName", false),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "complex_validation should succeed: {:?}", result);
}

#[test]
fn test_errors_complex_validation_high_value_requirement_fails() {
    skip_if_not_built!();
    let mut svm = load_program();
    let (authority, error_demo_pda) = setup_error_demo(&mut svm);
    let prog_id = program_id();

    // With require_high_value=true, value must be >= max_value/2 (500)
    // Pass value=100, which is < 500
    let ix = anchor_instruction(
        prog_id,
        "complex_validation",
        &complex_validation_data(100, "NewName", true),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(
        result.is_err(),
        "complex_validation with low value when high required should fail"
    );
}

#[test]
fn test_errors_complex_validation_high_value_requirement_success() {
    skip_if_not_built!();
    let mut svm = load_program();
    let (authority, error_demo_pda) = setup_error_demo(&mut svm);
    let prog_id = program_id();

    // With require_high_value=true, value must be >= max_value/2 (500)
    // Pass value=600, which is >= 500
    let ix = anchor_instruction(
        prog_id,
        "complex_validation",
        &complex_validation_data(600, "HighValueName", true),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(
        result.is_ok(),
        "complex_validation with high value should succeed: {:?}",
        result
    );
}

// =============================================================================
// FULL WORKFLOW TESTS
// =============================================================================

#[test]
fn test_errors_full_workflow() {
    skip_if_not_built!();
    let mut svm = load_program();
    let (authority, error_demo_pda) = setup_error_demo(&mut svm);
    let prog_id = program_id();

    // 1. Set value
    let ix1 = anchor_instruction(
        prog_id,
        "set_value",
        &set_value_data(100),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );
    execute_tx(&mut svm, ix1, &authority, &[&authority]).unwrap();

    // 2. Increment
    let ix2 = anchor_instruction(
        prog_id,
        "increment_value",
        &increment_data(50),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );
    execute_tx(&mut svm, ix2, &authority, &[&authority]).unwrap();

    // 3. Verify value is 150
    let account = svm.get_account(&error_demo_pda).unwrap();
    assert_eq!(read_value(&account.data), 150);

    // 4. Decrement
    let ix3 = anchor_instruction(
        prog_id,
        "decrement_value",
        &decrement_data(25),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );
    execute_tx(&mut svm, ix3, &authority, &[&authority]).unwrap();

    // 5. Verify value is 125
    let account = svm.get_account(&error_demo_pda).unwrap();
    assert_eq!(read_value(&account.data), 125);

    // 6. Update name
    let ix4 = anchor_instruction(
        prog_id,
        "update_name",
        &update_name_data("UpdatedName"),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );
    execute_tx(&mut svm, ix4, &authority, &[&authority]).unwrap();

    // 7. Verify operation count is 5 (init + set + increment + decrement + update_name)
    let account = svm.get_account(&error_demo_pda).unwrap();
    assert_eq!(read_operation_count(&account.data), 4); // init doesn't count, so 4 operations

    // 8. Deactivate
    let ix5 = anchor_instruction(
        prog_id,
        "deactivate",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );
    execute_tx(&mut svm, ix5, &authority, &[&authority]).unwrap();

    // 9. Verify inactive
    let account = svm.get_account(&error_demo_pda).unwrap();
    assert!(!read_is_active(&account.data));

    // 10. Reactivate
    let ix6 = anchor_instruction(
        prog_id,
        "reactivate",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(error_demo_pda),
        ],
    );
    execute_tx(&mut svm, ix6, &authority, &[&authority]).unwrap();

    // 11. Verify active again
    let account = svm.get_account(&error_demo_pda).unwrap();
    assert!(read_is_active(&account.data));
}

#[test]
fn test_errors_pda_derivation_deterministic() {
    skip_if_not_built!();
    let authority = solana_keypair::Keypair::new();

    // Derive PDA twice - should be identical
    let (pda1, bump1) = derive_error_demo_pda(&authority.pubkey());
    let (pda2, bump2) = derive_error_demo_pda(&authority.pubkey());

    assert_eq!(pda1, pda2, "PDA should be deterministic");
    assert_eq!(bump1, bump2, "Bump should be deterministic");
}
