//! LiteSVM integration tests for the Seahorse Init Patterns program
//!
//! These tests verify various account initialization patterns:
//! - Basic init with primitive types (u8, u16, u32, u64, i64)
//! - Init with dynamic strings requiring padding
//! - Init with fixed-size arrays
//! - Init with nested structs
//! - Space calculation verification
//! - Rent exemption handling
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile init_patterns.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// Init Patterns program ID (from declare_id!)
fn init_program_id() -> Pubkey {
    Pubkey::from_str("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS").unwrap()
}

// Account layout reference:
// SimpleData: discriminator(8) + owner(32) + u8(1) + u16(2) + u32(4) + u64(8) + i64(8) + bump(1) = 64 bytes
// StringData: discriminator(8) + owner(32) + name_len(4) + name(var) + description_len(4) + description(var) + bump(1)
// ArrayData: discriminator(8) + owner(32) + values([u64;4]=32) + flags([bool;8]=8) + bump(1) = 81 bytes
// ComplexData: discriminator(8) + owner(32) + Stats(24) + is_active(1) + bump(1) = 66 bytes

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the init patterns program into LiteSVM
fn load_init_program() -> (litesvm::LiteSVM, Keypair) {
    let program_id = init_program_id();
    let program_bytes = std::fs::read("../../target/deploy/init_patterns.so")
        .expect("init_patterns.so not found - run ./scripts/build-test-programs.sh");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);

    // Create a funded user
    let user = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    (svm, user)
}

/// Read SimpleData from account data
fn read_simple_data(data: &[u8]) -> (Pubkey, u8, u16, u32, u64, i64, u8) {
    // discriminator(8) + owner(32) + u8(1) + u16(2) + u32(4) + u64(8) + i64(8) + bump(1)
    let owner = Pubkey::new_from_array(data[8..40].try_into().unwrap());
    let value_u8 = data[40];
    let value_u16 = u16::from_le_bytes(data[41..43].try_into().unwrap());
    let value_u32 = u32::from_le_bytes(data[43..47].try_into().unwrap());
    let value_u64 = u64::from_le_bytes(data[47..55].try_into().unwrap());
    let value_i64 = i64::from_le_bytes(data[55..63].try_into().unwrap());
    let bump = data[63];
    (owner, value_u8, value_u16, value_u32, value_u64, value_i64, bump)
}

/// Read ArrayData from account data
fn read_array_data(data: &[u8]) -> (Pubkey, [u64; 4], [bool; 8], u8) {
    // discriminator(8) + owner(32) + values([u64;4]=32) + flags([bool;8]=8) + bump(1)
    let owner = Pubkey::new_from_array(data[8..40].try_into().unwrap());
    let mut values = [0u64; 4];
    for i in 0..4 {
        let offset = 40 + i * 8;
        values[i] = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
    }
    let mut flags = [false; 8];
    for i in 0..8 {
        flags[i] = data[72 + i] != 0;
    }
    let bump = data[80];
    (owner, values, flags, bump)
}

/// Read ComplexData from account data
fn read_complex_data(data: &[u8]) -> (Pubkey, u64, u64, u64, bool, u8) {
    // discriminator(8) + owner(32) + Stats(count:8 + total:8 + average:8) + is_active(1) + bump(1)
    let owner = Pubkey::new_from_array(data[8..40].try_into().unwrap());
    let count = u64::from_le_bytes(data[40..48].try_into().unwrap());
    let total = u64::from_le_bytes(data[48..56].try_into().unwrap());
    let average = u64::from_le_bytes(data[56..64].try_into().unwrap());
    let is_active = data[64] != 0;
    let bump = data[65];
    (owner, count, total, average, is_active, bump)
}

/// Serialize a string for Anchor (4-byte length prefix + UTF-8 bytes)
fn serialize_string(s: &str) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&(s.len() as u32).to_le_bytes());
    data.extend_from_slice(s.as_bytes());
    data
}

// =============================================================================
// SIMPLE DATA TESTS: PRIMITIVE TYPES
// =============================================================================

#[test]
fn test_init_simple() {
    let (mut svm, owner) = load_init_program();
    let program_id = init_program_id();

    // Derive PDA
    let (data_pda, _bump) = find_pda(&[b"simple", owner.pubkey().as_ref()], &program_id);

    // Create init instruction with value_u8 and value_u64
    let value_u8: u8 = 42;
    let value_u64: u64 = 1000000;
    let mut ix_data = Vec::new();
    ix_data.push(value_u8);
    ix_data.extend_from_slice(&value_u64.to_le_bytes());

    let ix = anchor_instruction(
        program_id,
        "init_simple",
        &ix_data,
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Init simple should succeed: {:?}", result);

    // Verify account state
    let account = svm.get_account(&data_pda).expect("Data should exist");
    let (stored_owner, v_u8, v_u16, v_u32, v_u64, v_i64, bump) = read_simple_data(&account.data);

    assert_eq!(stored_owner, owner.pubkey(), "Owner should be set");
    assert_eq!(v_u8, 42, "value_u8 should be 42");
    assert_eq!(v_u16, 0, "value_u16 should be 0");
    assert_eq!(v_u32, 0, "value_u32 should be 0");
    assert_eq!(v_u64, 1000000, "value_u64 should be 1000000");
    assert_eq!(v_i64, 0, "value_i64 should be 0");
    assert!(bump > 0, "Bump should be non-zero");
}

#[test]
fn test_update_simple() {
    let (mut svm, owner) = load_init_program();
    let program_id = init_program_id();

    let (data_pda, _) = find_pda(&[b"simple", owner.pubkey().as_ref()], &program_id);

    // Initialize
    let mut init_data = Vec::new();
    init_data.push(1u8);
    init_data.extend_from_slice(&100u64.to_le_bytes());

    let init_ix = anchor_instruction(
        program_id,
        "init_simple",
        &init_data,
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // Update with new values
    let value_u8: u8 = 255;
    let value_u16: u16 = 65535;
    let value_u32: u32 = 0xFFFFFFFF;
    let value_u64: u64 = 0xFFFFFFFFFFFFFFFF;
    let value_i64: i64 = -9223372036854775808; // i64::MIN

    let mut update_data = Vec::new();
    update_data.push(value_u8);
    update_data.extend_from_slice(&value_u16.to_le_bytes());
    update_data.extend_from_slice(&value_u32.to_le_bytes());
    update_data.extend_from_slice(&value_u64.to_le_bytes());
    update_data.extend_from_slice(&value_i64.to_le_bytes());

    let update_ix = anchor_instruction(
        program_id,
        "update_simple",
        &update_data,
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
        ],
    );
    let result = execute_tx(&mut svm, update_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Update should succeed");

    // Verify updated values
    let account = svm.get_account(&data_pda).unwrap();
    let (_, v_u8, v_u16, v_u32, v_u64, v_i64, _) = read_simple_data(&account.data);

    assert_eq!(v_u8, 255);
    assert_eq!(v_u16, 65535);
    assert_eq!(v_u32, 0xFFFFFFFF);
    assert_eq!(v_u64, 0xFFFFFFFFFFFFFFFF);
    assert_eq!(v_i64, i64::MIN);
}

#[test]
fn test_simple_unauthorized_update_fails() {
    let (mut svm, owner) = load_init_program();
    let program_id = init_program_id();
    let attacker = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    let (data_pda, _) = find_pda(&[b"simple", owner.pubkey().as_ref()], &program_id);

    // Owner initializes
    let mut init_data = Vec::new();
    init_data.push(1u8);
    init_data.extend_from_slice(&100u64.to_le_bytes());

    let init_ix = anchor_instruction(
        program_id,
        "init_simple",
        &init_data,
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // Attacker tries to update - should fail
    let mut update_data = Vec::new();
    update_data.push(99u8);
    update_data.extend_from_slice(&0u16.to_le_bytes());
    update_data.extend_from_slice(&0u32.to_le_bytes());
    update_data.extend_from_slice(&0u64.to_le_bytes());
    update_data.extend_from_slice(&0i64.to_le_bytes());

    let update_ix = anchor_instruction(
        program_id,
        "update_simple",
        &update_data,
        vec![
            signer_meta(attacker.pubkey()),
            writable_meta(data_pda),
        ],
    );
    let result = execute_tx(&mut svm, update_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "Unauthorized update should fail");
}

// =============================================================================
// STRING DATA TESTS: DYNAMIC SIZING
// =============================================================================

#[test]
fn test_init_with_string() {
    let (mut svm, owner) = load_init_program();
    let program_id = init_program_id();

    let (data_pda, _bump) = find_pda(&[b"string_data", owner.pubkey().as_ref()], &program_id);

    let name = "TestName";
    let description = "This is a test description";

    let mut ix_data = Vec::new();
    ix_data.extend_from_slice(&serialize_string(name));
    ix_data.extend_from_slice(&serialize_string(description));

    let ix = anchor_instruction(
        program_id,
        "init_with_string",
        &ix_data,
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Init with string should succeed: {:?}", result);

    // Verify account exists and owner is set
    let account = svm.get_account(&data_pda).expect("Data should exist");
    let stored_owner = Pubkey::new_from_array(account.data[8..40].try_into().unwrap());
    assert_eq!(stored_owner, owner.pubkey(), "Owner should be set");

    // Verify account has enough space (padding should be applied)
    assert!(account.data.len() >= 200, "Account should have adequate padding");
}

#[test]
fn test_update_string() {
    let (mut svm, owner) = load_init_program();
    let program_id = init_program_id();

    let (data_pda, _) = find_pda(&[b"string_data", owner.pubkey().as_ref()], &program_id);

    // Initialize
    let mut init_data = Vec::new();
    init_data.extend_from_slice(&serialize_string("Initial"));
    init_data.extend_from_slice(&serialize_string("First"));

    let init_ix = anchor_instruction(
        program_id,
        "init_with_string",
        &init_data,
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // Update with new strings
    let mut update_data = Vec::new();
    update_data.extend_from_slice(&serialize_string("Updated Name"));
    update_data.extend_from_slice(&serialize_string("This is a much longer description to test string updates"));

    let update_ix = anchor_instruction(
        program_id,
        "update_string",
        &update_data,
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
        ],
    );
    let result = execute_tx(&mut svm, update_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Update string should succeed");
}

// =============================================================================
// ARRAY DATA TESTS: FIXED-SIZE ARRAYS
// =============================================================================

#[test]
fn test_init_with_array() {
    let (mut svm, owner) = load_init_program();
    let program_id = init_program_id();

    let (data_pda, _bump) = find_pda(&[b"array_data", owner.pubkey().as_ref()], &program_id);

    let ix = anchor_instruction(
        program_id,
        "init_with_array",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Init with array should succeed: {:?}", result);

    // Verify account state
    let account = svm.get_account(&data_pda).expect("Data should exist");
    let (stored_owner, values, flags, bump) = read_array_data(&account.data);

    assert_eq!(stored_owner, owner.pubkey(), "Owner should be set");
    assert_eq!(values, [0, 0, 0, 0], "Values should be initialized to 0");
    assert_eq!(flags, [false; 8], "Flags should be initialized to false");
    assert!(bump > 0, "Bump should be non-zero");
}

#[test]
fn test_update_array() {
    let (mut svm, owner) = load_init_program();
    let program_id = init_program_id();

    let (data_pda, _) = find_pda(&[b"array_data", owner.pubkey().as_ref()], &program_id);

    // Initialize
    let init_ix = anchor_instruction(
        program_id,
        "init_with_array",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // Update arrays - passing individual values
    // v0, v1, v2, v3, f0, f1, f2, f3, f4, f5, f6, f7
    let values = [100u64, 200u64, 300u64, 400u64];
    let flags = [true, false, true, false, true, false, true, false];

    let mut update_data = Vec::new();
    for v in values.iter() {
        update_data.extend_from_slice(&v.to_le_bytes());
    }
    for f in flags.iter() {
        update_data.push(if *f { 1 } else { 0 });
    }

    let update_ix = anchor_instruction(
        program_id,
        "update_array",
        &update_data,
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
        ],
    );
    let result = execute_tx(&mut svm, update_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Update array should succeed");

    // Verify updated values
    let account = svm.get_account(&data_pda).unwrap();
    let (_, stored_values, stored_flags, _) = read_array_data(&account.data);

    assert_eq!(stored_values, values, "Values should be updated");
    assert_eq!(stored_flags, flags, "Flags should be updated");
}

// =============================================================================
// COMPLEX DATA TESTS: NESTED STRUCTS
// =============================================================================

#[test]
fn test_init_complex() {
    let (mut svm, owner) = load_init_program();
    let program_id = init_program_id();

    let (data_pda, _bump) = find_pda(&[b"complex", owner.pubkey().as_ref()], &program_id);

    let initial_count: u64 = 10;

    let ix = anchor_instruction(
        program_id,
        "init_complex",
        &initial_count.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Init complex should succeed: {:?}", result);

    // Verify account state
    let account = svm.get_account(&data_pda).expect("Data should exist");
    let (stored_owner, count, total, average, is_active, bump) = read_complex_data(&account.data);

    assert_eq!(stored_owner, owner.pubkey(), "Owner should be set");
    assert_eq!(count, 10, "Count should be 10");
    assert_eq!(total, 0, "Total should be 0");
    assert_eq!(average, 0, "Average should be 0");
    assert!(is_active, "is_active should be true");
    assert!(bump > 0, "Bump should be non-zero");
}

#[test]
fn test_update_complex() {
    let (mut svm, owner) = load_init_program();
    let program_id = init_program_id();

    let (data_pda, _) = find_pda(&[b"complex", owner.pubkey().as_ref()], &program_id);

    // Initialize
    let init_ix = anchor_instruction(
        program_id,
        "init_complex",
        &5u64.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // Update stats
    let count: u64 = 10;
    let total: u64 = 500;

    let mut update_data = Vec::new();
    update_data.extend_from_slice(&count.to_le_bytes());
    update_data.extend_from_slice(&total.to_le_bytes());

    let update_ix = anchor_instruction(
        program_id,
        "update_complex",
        &update_data,
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
        ],
    );
    let result = execute_tx(&mut svm, update_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Update complex should succeed");

    // Verify updated values
    let account = svm.get_account(&data_pda).unwrap();
    let (_, stored_count, stored_total, stored_average, _, _) = read_complex_data(&account.data);

    assert_eq!(stored_count, 10);
    assert_eq!(stored_total, 500);
    assert_eq!(stored_average, 50, "Average should be total/count = 500/10 = 50");
}

#[test]
fn test_toggle_active() {
    let (mut svm, owner) = load_init_program();
    let program_id = init_program_id();

    let (data_pda, _) = find_pda(&[b"complex", owner.pubkey().as_ref()], &program_id);

    // Initialize (is_active starts as true)
    let init_ix = anchor_instruction(
        program_id,
        "init_complex",
        &1u64.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // Verify initially active
    let account = svm.get_account(&data_pda).unwrap();
    let (_, _, _, _, is_active, _) = read_complex_data(&account.data);
    assert!(is_active, "Should start active");

    // Toggle to inactive
    let toggle_ix = anchor_instruction(
        program_id,
        "toggle_active",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
        ],
    );
    execute_tx(&mut svm, toggle_ix, &owner, &[&owner]).unwrap();

    let account = svm.get_account(&data_pda).unwrap();
    let (_, _, _, _, is_active, _) = read_complex_data(&account.data);
    assert!(!is_active, "Should be inactive after toggle");

    // Toggle back to active
    let toggle_ix2 = anchor_instruction(
        program_id,
        "toggle_active",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
        ],
    );
    svm.expire_blockhash();
    execute_tx(&mut svm, toggle_ix2, &owner, &[&owner]).unwrap();

    let account = svm.get_account(&data_pda).unwrap();
    let (_, _, _, _, is_active, _) = read_complex_data(&account.data);
    assert!(is_active, "Should be active after second toggle");
}

// =============================================================================
// SPACE CALCULATION VERIFICATION TESTS
// =============================================================================

#[test]
fn test_simple_data_space() {
    let (mut svm, owner) = load_init_program();
    let program_id = init_program_id();

    let (data_pda, _) = find_pda(&[b"simple", owner.pubkey().as_ref()], &program_id);

    let mut init_data = Vec::new();
    init_data.push(1u8);
    init_data.extend_from_slice(&100u64.to_le_bytes());

    let init_ix = anchor_instruction(
        program_id,
        "init_simple",
        &init_data,
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // Verify space calculation
    let account = svm.get_account(&data_pda).unwrap();
    // SimpleData: discriminator(8) + owner(32) + u8(1) + u16(2) + u32(4) + u64(8) + i64(8) + bump(1) = 64
    assert_eq!(account.data.len(), 64, "SimpleData should be 64 bytes");
}

#[test]
fn test_array_data_space() {
    let (mut svm, owner) = load_init_program();
    let program_id = init_program_id();

    let (data_pda, _) = find_pda(&[b"array_data", owner.pubkey().as_ref()], &program_id);

    let init_ix = anchor_instruction(
        program_id,
        "init_with_array",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // Verify space calculation
    let account = svm.get_account(&data_pda).unwrap();
    // ArrayData: discriminator(8) + owner(32) + [u64;4](32) + [bool;8](8) + bump(1)
    assert_eq!(account.data.len(), 88, "ArrayData should be 88 bytes");
}

#[test]
fn test_complex_data_space() {
    let (mut svm, owner) = load_init_program();
    let program_id = init_program_id();

    let (data_pda, _) = find_pda(&[b"complex", owner.pubkey().as_ref()], &program_id);

    let init_ix = anchor_instruction(
        program_id,
        "init_complex",
        &1u64.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // Verify space calculation
    let account = svm.get_account(&data_pda).unwrap();
    // ComplexData: discriminator(8) + owner(32) + Stats(24) + is_active(1) + bump(1)
    assert_eq!(account.data.len(), 72, "ComplexData should be 72 bytes");
}

// =============================================================================
// RENT EXEMPTION TESTS
// =============================================================================

#[test]
fn test_accounts_are_rent_exempt() {
    let (mut svm, owner) = load_init_program();
    let program_id = init_program_id();

    // Initialize all account types
    let (simple_pda, _) = find_pda(&[b"simple", owner.pubkey().as_ref()], &program_id);
    let (array_pda, _) = find_pda(&[b"array_data", owner.pubkey().as_ref()], &program_id);
    let (complex_pda, _) = find_pda(&[b"complex", owner.pubkey().as_ref()], &program_id);

    // Init simple
    let mut init_simple_data = Vec::new();
    init_simple_data.push(1u8);
    init_simple_data.extend_from_slice(&100u64.to_le_bytes());
    let init_simple_ix = anchor_instruction(
        program_id,
        "init_simple",
        &init_simple_data,
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(simple_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_simple_ix, &owner, &[&owner]).unwrap();

    // Init array
    let init_array_ix = anchor_instruction(
        program_id,
        "init_with_array",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(array_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_array_ix, &owner, &[&owner]).unwrap();

    // Init complex
    let init_complex_ix = anchor_instruction(
        program_id,
        "init_complex",
        &1u64.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(complex_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_complex_ix, &owner, &[&owner]).unwrap();

    // Verify all accounts have enough lamports for rent exemption
    let rent = solana_rent::Rent::default();

    let simple_account = svm.get_account(&simple_pda).unwrap();
    let min_balance_simple = rent.minimum_balance(simple_account.data.len());
    assert!(
        simple_account.lamports >= min_balance_simple,
        "SimpleData should be rent exempt"
    );

    let array_account = svm.get_account(&array_pda).unwrap();
    let min_balance_array = rent.minimum_balance(array_account.data.len());
    assert!(
        array_account.lamports >= min_balance_array,
        "ArrayData should be rent exempt"
    );

    let complex_account = svm.get_account(&complex_pda).unwrap();
    let min_balance_complex = rent.minimum_balance(complex_account.data.len());
    assert!(
        complex_account.lamports >= min_balance_complex,
        "ComplexData should be rent exempt"
    );
}
