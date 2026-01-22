//! LiteSVM integration tests for the Seahorse Events program
//!
//! These tests verify event emission patterns:
//! - initialize: Creates an event counter account
//! - emit_simple: Emits a SimpleEvent with u64 and String
//! - emit_user_action: Emits a UserActionEvent with Pubkey, u8, i64
//! - emit_numeric: Emits a NumericEvent with various integer types
//! - emit_transfer: Emits a TransferEvent with from, to, amount, memo
//! - emit_state_change: Emits a StateChangeEvent with old/new values
//! - emit_multiple: Emits multiple SimpleEvents in one transaction
//! - get_counter_info: Logs counter state (read-only)
//!
//! NOTE: Events are emitted to transaction logs. These tests verify:
//! 1. Instructions execute successfully
//! 2. Counter state is updated correctly
//! 3. Events don't cause execution failures
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile events.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// Events program ID (from declare_id!)
fn program_id() -> Pubkey {
    Pubkey::from_str("YqBL3cHjsojPxJuyLF6bcQSYJ59p9X5qePjhWkXCEGR").unwrap()
}

/// Path to the compiled program
const PROGRAM_PATH: &str = "../../target/deploy/events.so";

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the events program into LiteSVM
fn load_program() -> litesvm::LiteSVM {
    let prog_id = program_id();

    let program_bytes = std::fs::read(PROGRAM_PATH)
        .expect("Failed to read events.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(prog_id, &program_bytes);
    svm
}

/// Derive counter PDA
fn derive_counter_pda(authority: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"counter", authority.as_ref()],
        &program_id(),
    )
}

/// Build empty instruction data
fn empty_data() -> Vec<u8> {
    Vec::new()
}

/// Build emit_simple instruction data
fn emit_simple_data(value: u64, label: &str) -> Vec<u8> {
    let mut data = Vec::new();
    // value: u64
    data.extend_from_slice(&value.to_le_bytes());
    // label: String (length prefix + bytes)
    data.extend_from_slice(&(label.len() as u32).to_le_bytes());
    data.extend_from_slice(label.as_bytes());
    data
}

/// Build emit_user_action instruction data
fn emit_user_action_data(action_type: u8, timestamp: i64) -> Vec<u8> {
    let mut data = Vec::new();
    // action_type: u8
    data.push(action_type);
    // timestamp: i64
    data.extend_from_slice(&timestamp.to_le_bytes());
    data
}

/// Build emit_numeric instruction data
fn emit_numeric_data(unsigned_small: u8, unsigned_medium: u32, unsigned_large: u64, signed_value: i64) -> Vec<u8> {
    let mut data = Vec::new();
    data.push(unsigned_small);
    data.extend_from_slice(&unsigned_medium.to_le_bytes());
    data.extend_from_slice(&unsigned_large.to_le_bytes());
    data.extend_from_slice(&signed_value.to_le_bytes());
    data
}

/// Build emit_transfer instruction data
fn emit_transfer_data(to_addr: &Pubkey, amount: u64, memo: &str) -> Vec<u8> {
    let mut data = Vec::new();
    // to_addr: Pubkey
    data.extend_from_slice(to_addr.as_ref());
    // amount: u64
    data.extend_from_slice(&amount.to_le_bytes());
    // memo: String (length prefix + bytes)
    data.extend_from_slice(&(memo.len() as u32).to_le_bytes());
    data.extend_from_slice(memo.as_bytes());
    data
}

/// Build emit_state_change instruction data
fn emit_state_change_data(old_value: u64, new_value: u64, change_type: &str) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&old_value.to_le_bytes());
    data.extend_from_slice(&new_value.to_le_bytes());
    data.extend_from_slice(&(change_type.len() as u32).to_le_bytes());
    data.extend_from_slice(change_type.as_bytes());
    data
}

/// Build emit_multiple instruction data
fn emit_multiple_data(count: u8) -> Vec<u8> {
    vec![count]
}

/// Read event_count from account data
/// Account layout: discriminator (8) + authority (32) + event_count (8) + last_event_type (1) + bump (1)
fn read_event_count(data: &[u8]) -> u64 {
    let count_bytes: [u8; 8] = data[40..48].try_into().unwrap();
    u64::from_le_bytes(count_bytes)
}

/// Read last_event_type from account data
fn read_last_event_type(data: &[u8]) -> u8 {
    data[48]
}

/// Read authority from account data
fn read_authority(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

// =============================================================================
// INITIALIZE TESTS
// =============================================================================

#[test]
fn test_events_initialize() {
    let mut svm = load_program();
    let authority = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let (counter_pda, _bump) = derive_counter_pda(&authority.pubkey());

    let ix = anchor_instruction(
        prog_id,
        "initialize",
        &empty_data(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Initialize should succeed: {:?}", result);

    // Verify account state
    let account = svm.get_account(&counter_pda).expect("Counter should exist");

    let stored_authority = read_authority(&account.data);
    assert_eq!(stored_authority, authority.pubkey(), "Authority should be set");

    let event_count = read_event_count(&account.data);
    assert_eq!(event_count, 0, "Event count should be 0");

    let last_event_type = read_last_event_type(&account.data);
    assert_eq!(last_event_type, 0, "Last event type should be 0");
}

#[test]
fn test_events_initialize_different_users() {
    let mut svm = load_program();
    let prog_id = program_id();

    // Create two different users
    let user1 = funded_keypair_10_sol(&mut svm);
    let user2 = funded_keypair_10_sol(&mut svm);

    let (counter_pda1, _) = derive_counter_pda(&user1.pubkey());
    let (counter_pda2, _) = derive_counter_pda(&user2.pubkey());

    // Initialize for user1
    let ix1 = anchor_instruction(
        prog_id,
        "initialize",
        &empty_data(),
        vec![
            signer_meta(user1.pubkey()),
            writable_meta(counter_pda1),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    let result1 = execute_tx(&mut svm, ix1, &user1, &[&user1]);
    assert!(result1.is_ok(), "Initialize user1 should succeed");

    // Initialize for user2
    let ix2 = anchor_instruction(
        prog_id,
        "initialize",
        &empty_data(),
        vec![
            signer_meta(user2.pubkey()),
            writable_meta(counter_pda2),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    let result2 = execute_tx(&mut svm, ix2, &user2, &[&user2]);
    assert!(result2.is_ok(), "Initialize user2 should succeed");

    // Verify both have separate counters
    assert_ne!(counter_pda1, counter_pda2, "Users should have different PDAs");
}

// =============================================================================
// EMIT_SIMPLE TESTS
// =============================================================================

#[test]
fn test_emit_simple_event() {
    let mut svm = load_program();
    let authority = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let (counter_pda, _) = derive_counter_pda(&authority.pubkey());

    // Initialize first
    let init_ix = anchor_instruction(
        prog_id,
        "initialize",
        &empty_data(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Emit simple event
    let emit_ix = anchor_instruction(
        prog_id,
        "emit_simple",
        &emit_simple_data(42, "test_label"),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
        ],
    );

    let result = execute_tx(&mut svm, emit_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "emit_simple should succeed: {:?}", result);

    // Verify counter updated
    let account = svm.get_account(&counter_pda).unwrap();
    let event_count = read_event_count(&account.data);
    assert_eq!(event_count, 1, "Event count should be 1");

    let last_event_type = read_last_event_type(&account.data);
    assert_eq!(last_event_type, 1, "Last event type should be 1 (SimpleEvent)");
}

#[test]
fn test_emit_simple_multiple_times() {
    let mut svm = load_program();
    let authority = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let (counter_pda, _) = derive_counter_pda(&authority.pubkey());

    // Initialize
    let init_ix = anchor_instruction(
        prog_id,
        "initialize",
        &empty_data(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Emit 3 simple events
    for i in 0..3 {
        let emit_ix = anchor_instruction(
            prog_id,
            "emit_simple",
            &emit_simple_data(i, &format!("event_{}", i)),
            vec![
                signer_meta(authority.pubkey()),
                writable_meta(counter_pda),
            ],
        );
        let result = execute_tx(&mut svm, emit_ix, &authority, &[&authority]);
        assert!(result.is_ok(), "emit_simple {} should succeed", i);
    }

    // Verify count is 3
    let account = svm.get_account(&counter_pda).unwrap();
    let event_count = read_event_count(&account.data);
    assert_eq!(event_count, 3, "Event count should be 3");
}

// =============================================================================
// EMIT_USER_ACTION TESTS
// =============================================================================

#[test]
fn test_emit_user_action_event() {
    let mut svm = load_program();
    let authority = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let (counter_pda, _) = derive_counter_pda(&authority.pubkey());

    // Initialize
    let init_ix = anchor_instruction(
        prog_id,
        "initialize",
        &empty_data(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Emit user action event
    let emit_ix = anchor_instruction(
        prog_id,
        "emit_user_action",
        &emit_user_action_data(5, 1700000000), // action_type=5, timestamp
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
        ],
    );

    let result = execute_tx(&mut svm, emit_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "emit_user_action should succeed: {:?}", result);

    // Verify counter updated
    let account = svm.get_account(&counter_pda).unwrap();
    let last_event_type = read_last_event_type(&account.data);
    assert_eq!(last_event_type, 2, "Last event type should be 2 (UserActionEvent)");
}

// =============================================================================
// EMIT_NUMERIC TESTS
// =============================================================================

#[test]
fn test_emit_numeric_event() {
    let mut svm = load_program();
    let authority = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let (counter_pda, _) = derive_counter_pda(&authority.pubkey());

    // Initialize
    let init_ix = anchor_instruction(
        prog_id,
        "initialize",
        &empty_data(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Emit numeric event with various types
    let emit_ix = anchor_instruction(
        prog_id,
        "emit_numeric",
        &emit_numeric_data(255, 1_000_000, 1_000_000_000_000, -500),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
        ],
    );

    let result = execute_tx(&mut svm, emit_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "emit_numeric should succeed: {:?}", result);

    // Verify counter updated
    let account = svm.get_account(&counter_pda).unwrap();
    let last_event_type = read_last_event_type(&account.data);
    assert_eq!(last_event_type, 3, "Last event type should be 3 (NumericEvent)");
}

// =============================================================================
// EMIT_TRANSFER TESTS
// =============================================================================

#[test]
fn test_emit_transfer_event() {
    let mut svm = load_program();
    let authority = funded_keypair_10_sol(&mut svm);
    let recipient = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let (counter_pda, _) = derive_counter_pda(&authority.pubkey());

    // Initialize
    let init_ix = anchor_instruction(
        prog_id,
        "initialize",
        &empty_data(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Emit transfer event
    let emit_ix = anchor_instruction(
        prog_id,
        "emit_transfer",
        &emit_transfer_data(&recipient.pubkey(), 1_000_000, "payment for services"),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
        ],
    );

    let result = execute_tx(&mut svm, emit_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "emit_transfer should succeed: {:?}", result);

    // Verify counter updated
    let account = svm.get_account(&counter_pda).unwrap();
    let last_event_type = read_last_event_type(&account.data);
    assert_eq!(last_event_type, 4, "Last event type should be 4 (TransferEvent)");
}

// =============================================================================
// EMIT_STATE_CHANGE TESTS
// =============================================================================

#[test]
fn test_emit_state_change_event() {
    let mut svm = load_program();
    let authority = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let (counter_pda, _) = derive_counter_pda(&authority.pubkey());

    // Initialize
    let init_ix = anchor_instruction(
        prog_id,
        "initialize",
        &empty_data(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Emit state change event
    let emit_ix = anchor_instruction(
        prog_id,
        "emit_state_change",
        &emit_state_change_data(100, 150, "increment"),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
        ],
    );

    let result = execute_tx(&mut svm, emit_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "emit_state_change should succeed: {:?}", result);

    // Verify counter updated
    let account = svm.get_account(&counter_pda).unwrap();
    let last_event_type = read_last_event_type(&account.data);
    assert_eq!(last_event_type, 5, "Last event type should be 5 (StateChangeEvent)");
}

// =============================================================================
// EMIT_MULTIPLE TESTS
// =============================================================================

#[test]
fn test_emit_multiple_events() {
    let mut svm = load_program();
    let authority = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let (counter_pda, _) = derive_counter_pda(&authority.pubkey());

    // Initialize
    let init_ix = anchor_instruction(
        prog_id,
        "initialize",
        &empty_data(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Emit 5 events in single transaction
    let emit_ix = anchor_instruction(
        prog_id,
        "emit_multiple",
        &emit_multiple_data(5),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
        ],
    );

    let result = execute_tx(&mut svm, emit_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "emit_multiple should succeed: {:?}", result);

    // Verify counter shows 5 events
    let account = svm.get_account(&counter_pda).unwrap();
    let event_count = read_event_count(&account.data);
    assert_eq!(event_count, 5, "Event count should be 5");

    let last_event_type = read_last_event_type(&account.data);
    assert_eq!(last_event_type, 6, "Last event type should be 6 (Multiple)");
}

#[test]
fn test_emit_multiple_max_limit() {
    let mut svm = load_program();
    let authority = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let (counter_pda, _) = derive_counter_pda(&authority.pubkey());

    // Initialize
    let init_ix = anchor_instruction(
        prog_id,
        "initialize",
        &empty_data(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Emit max 10 events
    let emit_ix = anchor_instruction(
        prog_id,
        "emit_multiple",
        &emit_multiple_data(10),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
        ],
    );

    let result = execute_tx(&mut svm, emit_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "emit_multiple(10) should succeed");

    let account = svm.get_account(&counter_pda).unwrap();
    let event_count = read_event_count(&account.data);
    assert_eq!(event_count, 10, "Event count should be 10");
}

#[test]
fn test_emit_multiple_exceeds_limit() {
    let mut svm = load_program();
    let authority = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let (counter_pda, _) = derive_counter_pda(&authority.pubkey());

    // Initialize
    let init_ix = anchor_instruction(
        prog_id,
        "initialize",
        &empty_data(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Try to emit 11 events (should fail)
    let emit_ix = anchor_instruction(
        prog_id,
        "emit_multiple",
        &emit_multiple_data(11),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
        ],
    );

    let result = execute_tx(&mut svm, emit_ix, &authority, &[&authority]);
    assert!(result.is_err(), "emit_multiple(11) should fail - exceeds limit");
}

// =============================================================================
// AUTHORIZATION TESTS
// =============================================================================

#[test]
fn test_emit_unauthorized_fails() {
    let mut svm = load_program();
    let authority = funded_keypair_10_sol(&mut svm);
    let other_user = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let (counter_pda, _) = derive_counter_pda(&authority.pubkey());

    // Initialize with authority
    let init_ix = anchor_instruction(
        prog_id,
        "initialize",
        &empty_data(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Try to emit as different user (should fail due to PDA constraint)
    let emit_ix = anchor_instruction(
        prog_id,
        "emit_simple",
        &emit_simple_data(42, "unauthorized"),
        vec![
            signer_meta(other_user.pubkey()),
            writable_meta(counter_pda),
        ],
    );

    let result = execute_tx(&mut svm, emit_ix, &other_user, &[&other_user]);
    assert!(result.is_err(), "Unauthorized emit should fail");
}

// =============================================================================
// GET_COUNTER_INFO TESTS
// =============================================================================

#[test]
fn test_get_counter_info() {
    let mut svm = load_program();
    let authority = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let (counter_pda, _) = derive_counter_pda(&authority.pubkey());

    // Initialize
    let init_ix = anchor_instruction(
        prog_id,
        "initialize",
        &empty_data(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Emit some events first
    let emit_ix = anchor_instruction(
        prog_id,
        "emit_simple",
        &emit_simple_data(100, "test"),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
        ],
    );
    execute_tx(&mut svm, emit_ix, &authority, &[&authority]).unwrap();

    // Get counter info (read-only, just logs)
    let info_ix = anchor_instruction(
        prog_id,
        "get_counter_info",
        &empty_data(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
        ],
    );

    let result = execute_tx(&mut svm, info_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "get_counter_info should succeed: {:?}", result);
}

// =============================================================================
// FULL WORKFLOW TESTS
// =============================================================================

#[test]
fn test_full_event_workflow() {
    let mut svm = load_program();
    let authority = funded_keypair_10_sol(&mut svm);
    let recipient = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    let (counter_pda, _) = derive_counter_pda(&authority.pubkey());

    // 1. Initialize
    let init_ix = anchor_instruction(
        prog_id,
        "initialize",
        &empty_data(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(counter_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // 2. Emit various event types
    // SimpleEvent
    let ix1 = anchor_instruction(
        prog_id,
        "emit_simple",
        &emit_simple_data(1, "first"),
        vec![signer_meta(authority.pubkey()), writable_meta(counter_pda)],
    );
    execute_tx(&mut svm, ix1, &authority, &[&authority]).unwrap();

    // UserActionEvent
    let ix2 = anchor_instruction(
        prog_id,
        "emit_user_action",
        &emit_user_action_data(1, 1700000000),
        vec![signer_meta(authority.pubkey()), writable_meta(counter_pda)],
    );
    execute_tx(&mut svm, ix2, &authority, &[&authority]).unwrap();

    // NumericEvent
    let ix3 = anchor_instruction(
        prog_id,
        "emit_numeric",
        &emit_numeric_data(100, 50000, 9999999999, -1234),
        vec![signer_meta(authority.pubkey()), writable_meta(counter_pda)],
    );
    execute_tx(&mut svm, ix3, &authority, &[&authority]).unwrap();

    // TransferEvent
    let ix4 = anchor_instruction(
        prog_id,
        "emit_transfer",
        &emit_transfer_data(&recipient.pubkey(), 1000000, "test transfer"),
        vec![signer_meta(authority.pubkey()), writable_meta(counter_pda)],
    );
    execute_tx(&mut svm, ix4, &authority, &[&authority]).unwrap();

    // StateChangeEvent
    let ix5 = anchor_instruction(
        prog_id,
        "emit_state_change",
        &emit_state_change_data(0, 100, "set"),
        vec![signer_meta(authority.pubkey()), writable_meta(counter_pda)],
    );
    execute_tx(&mut svm, ix5, &authority, &[&authority]).unwrap();

    // 3. Verify final state
    let account = svm.get_account(&counter_pda).unwrap();
    let event_count = read_event_count(&account.data);
    assert_eq!(event_count, 5, "Should have emitted 5 events");

    let last_event_type = read_last_event_type(&account.data);
    assert_eq!(last_event_type, 5, "Last event should be StateChangeEvent (type 5)");
}

#[test]
fn test_pda_derivation_deterministic() {
    let mut svm = load_program();
    let authority = funded_keypair_10_sol(&mut svm);
    let prog_id = program_id();

    // Derive PDA twice - should be identical
    let (pda1, bump1) = derive_counter_pda(&authority.pubkey());
    let (pda2, bump2) = derive_counter_pda(&authority.pubkey());

    assert_eq!(pda1, pda2, "PDA should be deterministic");
    assert_eq!(bump1, bump2, "Bump should be deterministic");

    // Verify it matches what program uses
    let init_ix = anchor_instruction(
        prog_id,
        "initialize",
        &empty_data(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(pda1),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, init_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "PDA derivation should match program");
}
