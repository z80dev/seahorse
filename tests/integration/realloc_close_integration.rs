//! LiteSVM integration tests for the Seahorse Realloc/Close program
//!
//! These tests verify realloc and close patterns:
//! - initialize_data: Creates a dynamic data account with initial content
//! - grow_account: Simulates growing the account (tracks size increase)
//! - shrink_account: Simulates shrinking the account (tracks size decrease)
//! - close_data_account: Simulates closing the account (marks as closed)
//! - create_record: Creates a resize tracking record
//! - record_resize: Records resize events for analytics
//! - close_record: Closes the resize record
//!
//! NOTE: Seahorse doesn't support native realloc/close operations.
//! These tests verify the tracking pattern implementation.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile realloc_close.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::path::Path;
use std::str::FromStr;

/// Realloc/Close program ID (from declare_id!)
fn program_id() -> Pubkey {
    Pubkey::from_str("F4fSecp12t3QtQaXTUWrjxLec1wQXJ31iQzuE2WjPsoB").unwrap()
}

/// Path to the compiled program
const PROGRAM_PATH: &str = "../../target/deploy/realloc_close.so";

/// Check if the program is built
fn program_exists() -> bool {
    Path::new(PROGRAM_PATH).exists()
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the realloc_close program into LiteSVM
fn load_program() -> litesvm::LiteSVM {
    let prog_id = program_id();

    if !program_exists() {
        panic!("Failed to read realloc_close.so - run ./scripts/build-test-programs.sh first");
    }

    let program_bytes = std::fs::read(PROGRAM_PATH)
        .expect("Failed to read realloc_close.so");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(prog_id, &program_bytes);
    svm
}

/// Derive data PDA
fn derive_data_pda(data_id: u64) -> (Pubkey, u8) {
    find_pda(
        &[b"data", data_id.to_le_bytes().as_ref()],
        &program_id(),
    )
}

/// Derive record PDA
fn derive_record_pda(record_id: u64) -> (Pubkey, u8) {
    find_pda(
        &[b"record", record_id.to_le_bytes().as_ref()],
        &program_id(),
    )
}

/// Build initialize_data instruction data
fn initialize_data_data(data_id: u64, initial_content: &str, initial_allocated_size: u64) -> Vec<u8> {
    let mut data = Vec::new();
    // data_id: u64
    data.extend_from_slice(&data_id.to_le_bytes());
    // initial_content: String (length prefix + bytes)
    data.extend_from_slice(&(initial_content.len() as u32).to_le_bytes());
    data.extend_from_slice(initial_content.as_bytes());
    // initial_allocated_size: u64
    data.extend_from_slice(&initial_allocated_size.to_le_bytes());
    data
}

/// Build grow_account instruction data
fn grow_account_data(data_id: u64, new_content: &str, additional_space: u64) -> Vec<u8> {
    let mut data = Vec::new();
    // data_id: u64
    data.extend_from_slice(&data_id.to_le_bytes());
    // new_content: String (length prefix + bytes)
    data.extend_from_slice(&(new_content.len() as u32).to_le_bytes());
    data.extend_from_slice(new_content.as_bytes());
    // additional_space: u64
    data.extend_from_slice(&additional_space.to_le_bytes());
    data
}

/// Build shrink_account instruction data
fn shrink_account_data(data_id: u64, new_content: &str, new_size: u64) -> Vec<u8> {
    let mut data = Vec::new();
    // data_id: u64
    data.extend_from_slice(&data_id.to_le_bytes());
    // new_content: String (length prefix + bytes)
    data.extend_from_slice(&(new_content.len() as u32).to_le_bytes());
    data.extend_from_slice(new_content.as_bytes());
    // new_size: u64
    data.extend_from_slice(&new_size.to_le_bytes());
    data
}

/// Build close_data_account instruction data
fn close_data_account_data(data_id: u64) -> Vec<u8> {
    let mut data = Vec::new();
    // data_id: u64
    data.extend_from_slice(&data_id.to_le_bytes());
    data
}

/// Build create_record instruction data
fn create_record_data(record_id: u64) -> Vec<u8> {
    let mut data = Vec::new();
    // record_id: u64
    data.extend_from_slice(&record_id.to_le_bytes());
    data
}

/// Build record_resize instruction data
fn record_resize_data(record_id: u64, new_size: u64, is_grow: bool) -> Vec<u8> {
    let mut data = Vec::new();
    // record_id: u64
    data.extend_from_slice(&record_id.to_le_bytes());
    // new_size: u64
    data.extend_from_slice(&new_size.to_le_bytes());
    // is_grow: bool
    data.push(if is_grow { 1 } else { 0 });
    data
}

/// Build close_record instruction data
fn close_record_data(record_id: u64) -> Vec<u8> {
    let mut data = Vec::new();
    // record_id: u64
    data.extend_from_slice(&record_id.to_le_bytes());
    data
}

/// Build get_data_info instruction data
fn get_data_info_data(data_id: u64) -> Vec<u8> {
    let mut data = Vec::new();
    // data_id: u64
    data.extend_from_slice(&data_id.to_le_bytes());
    data
}

// =============================================================================
// DATA READER HELPERS
// =============================================================================

/// Read DynamicData owner from account data
fn read_data_owner(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

/// Read DynamicData data_id from account data
fn read_data_data_id(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[40..48].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

/// Read DynamicData content from account data (string with length prefix)
fn read_data_content(data: &[u8]) -> String {
    let len = u32::from_le_bytes(data[48..52].try_into().unwrap()) as usize;
    String::from_utf8(data[52..52 + len].to_vec()).unwrap_or_default()
}

/// Calculate offset after content string
fn content_end_offset(data: &[u8]) -> usize {
    let len = u32::from_le_bytes(data[48..52].try_into().unwrap()) as usize;
    52 + len
}

/// Read DynamicData content_len from account data
fn read_data_content_len(data: &[u8]) -> u64 {
    let offset = content_end_offset(data);
    let bytes: [u8; 8] = data[offset..offset + 8].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

/// Read DynamicData allocated_size from account data
fn read_data_allocated_size(data: &[u8]) -> u64 {
    let offset = content_end_offset(data) + 8;
    let bytes: [u8; 8] = data[offset..offset + 8].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

/// Read DynamicData version from account data
fn read_data_version(data: &[u8]) -> u64 {
    let offset = content_end_offset(data) + 16;
    let bytes: [u8; 8] = data[offset..offset + 8].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

/// Read DynamicData is_closed from account data
fn read_data_is_closed(data: &[u8]) -> bool {
    let offset = content_end_offset(data) + 24;
    data[offset] != 0
}

// ResizeRecord readers (fixed offsets)
fn read_record_owner(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

fn read_record_record_id(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[40..48].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_record_current_size(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[48..56].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_record_max_size_reached(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[56..64].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_record_resize_count(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[64..72].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_record_is_active(data: &[u8]) -> bool {
    data[72] != 0
}

// =============================================================================
// INITIALIZE DATA TESTS
// =============================================================================

#[test]
fn test_initialize_data() {
    if !program_exists() {
        eprintln!("Skipping test_initialize_data: program not built. Run ./scripts/build-test-programs.sh");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    // Setup owner
    let owner = funded_keypair_10_sol(&mut svm);

    // Data parameters
    let data_id: u64 = 1;
    let initial_content = "Hello, World!";
    let initial_allocated_size: u64 = 100;

    // Derive PDA
    let (data_pda, _bump) = derive_data_pda(data_id);

    // Build instruction
    let ix = anchor_instruction(
        prog_id,
        "initialize_data",
        &initialize_data_data(data_id, initial_content, initial_allocated_size),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(system_program::id()),
        ],
    );

    // Execute transaction
    let result = execute_tx(&mut svm, ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Failed to initialize data: {:?}", result.err());

    // Verify account state
    let account_data = get_account_data(&svm, &data_pda).unwrap();

    assert_eq!(read_data_owner(&account_data), owner.pubkey());
    assert_eq!(read_data_data_id(&account_data), data_id);
    assert_eq!(read_data_content(&account_data), initial_content);
    assert_eq!(read_data_content_len(&account_data), initial_content.len() as u64);
    assert_eq!(read_data_allocated_size(&account_data), initial_allocated_size);
    assert_eq!(read_data_version(&account_data), 1);
    assert!(!read_data_is_closed(&account_data));
}

#[test]
fn test_initialize_data_multiple_accounts() {
    if !program_exists() {
        eprintln!("Skipping test_initialize_data_multiple_accounts: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    let owner = funded_keypair_10_sol(&mut svm);

    // Create first data account
    let data_id_1: u64 = 1;
    let (data_pda_1, _) = derive_data_pda(data_id_1);

    let ix1 = anchor_instruction(
        prog_id,
        "initialize_data",
        &initialize_data_data(data_id_1, "First", 50),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda_1),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix1, &owner, &[&owner]);
    assert!(result.is_ok());
    svm.expire_blockhash();

    // Create second data account
    let data_id_2: u64 = 2;
    let (data_pda_2, _) = derive_data_pda(data_id_2);

    let ix2 = anchor_instruction(
        prog_id,
        "initialize_data",
        &initialize_data_data(data_id_2, "Second", 75),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda_2),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix2, &owner, &[&owner]);
    assert!(result.is_ok());

    // Verify both accounts exist with correct data
    let data_1 = get_account_data(&svm, &data_pda_1).unwrap();
    let data_2 = get_account_data(&svm, &data_pda_2).unwrap();

    assert_eq!(read_data_content(&data_1), "First");
    assert_eq!(read_data_allocated_size(&data_1), 50);
    assert_eq!(read_data_content(&data_2), "Second");
    assert_eq!(read_data_allocated_size(&data_2), 75);
}

// =============================================================================
// GROW ACCOUNT TESTS
// =============================================================================

#[test]
fn test_grow_account() {
    if !program_exists() {
        eprintln!("Skipping test_grow_account: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    let owner = funded_keypair_10_sol(&mut svm);

    // Initialize data
    let data_id: u64 = 1;
    let (data_pda, _) = derive_data_pda(data_id);

    let init_ix = anchor_instruction(
        prog_id,
        "initialize_data",
        &initialize_data_data(data_id, "Initial", 50),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(system_program::id()),
        ],
    );

    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    // Grow account
    let new_content = "This is much longer content that requires more space";
    let additional_space: u64 = 100;

    let grow_ix = anchor_instruction(
        prog_id,
        "grow_account",
        &grow_account_data(data_id, new_content, additional_space),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
        ],
    );

    let result = execute_tx(&mut svm, grow_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Failed to grow account: {:?}", result.err());

    // Verify account state
    let account_data = get_account_data(&svm, &data_pda).unwrap();

    assert_eq!(read_data_content(&account_data), new_content);
    assert_eq!(read_data_content_len(&account_data), new_content.len() as u64);
    assert_eq!(read_data_allocated_size(&account_data), 50 + additional_space);
    assert_eq!(read_data_version(&account_data), 2); // Version incremented
}

#[test]
fn test_grow_account_unauthorized() {
    if !program_exists() {
        eprintln!("Skipping test_grow_account_unauthorized: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    let owner = funded_keypair_10_sol(&mut svm);
    let attacker = funded_keypair_10_sol(&mut svm);

    // Initialize data
    let data_id: u64 = 1;
    let (data_pda, _) = derive_data_pda(data_id);

    let init_ix = anchor_instruction(
        prog_id,
        "initialize_data",
        &initialize_data_data(data_id, "Initial", 50),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(system_program::id()),
        ],
    );

    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    // Try to grow from wrong account
    let grow_ix = anchor_instruction(
        prog_id,
        "grow_account",
        &grow_account_data(data_id, "Hacked!", 100),
        vec![
            signer_meta(attacker.pubkey()),
            writable_meta(data_pda),
        ],
    );

    let result = execute_tx(&mut svm, grow_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "Should fail with unauthorized");
}

// =============================================================================
// SHRINK ACCOUNT TESTS
// =============================================================================

#[test]
fn test_shrink_account() {
    if !program_exists() {
        eprintln!("Skipping test_shrink_account: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    let owner = funded_keypair_10_sol(&mut svm);

    // Initialize data with large allocation
    let data_id: u64 = 1;
    let (data_pda, _) = derive_data_pda(data_id);

    let init_ix = anchor_instruction(
        prog_id,
        "initialize_data",
        &initialize_data_data(data_id, "This is some initial content", 150),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(system_program::id()),
        ],
    );

    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    // Shrink account
    let new_content = "Smaller";
    let new_size: u64 = 50;

    let shrink_ix = anchor_instruction(
        prog_id,
        "shrink_account",
        &shrink_account_data(data_id, new_content, new_size),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
        ],
    );

    let result = execute_tx(&mut svm, shrink_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Failed to shrink account: {:?}", result.err());

    // Verify account state
    let account_data = get_account_data(&svm, &data_pda).unwrap();

    assert_eq!(read_data_content(&account_data), new_content);
    assert_eq!(read_data_content_len(&account_data), new_content.len() as u64);
    assert_eq!(read_data_allocated_size(&account_data), new_size);
    assert_eq!(read_data_version(&account_data), 2);
}

#[test]
fn test_shrink_account_content_too_large() {
    if !program_exists() {
        eprintln!("Skipping test_shrink_account_content_too_large: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    let owner = funded_keypair_10_sol(&mut svm);

    // Initialize data
    let data_id: u64 = 1;
    let (data_pda, _) = derive_data_pda(data_id);

    let init_ix = anchor_instruction(
        prog_id,
        "initialize_data",
        &initialize_data_data(data_id, "Initial content", 100),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(system_program::id()),
        ],
    );

    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    // Try to shrink with content larger than new size
    let shrink_ix = anchor_instruction(
        prog_id,
        "shrink_account",
        &shrink_account_data(data_id, "This content is way too long", 5), // 5 < 28 chars
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
        ],
    );

    let result = execute_tx(&mut svm, shrink_ix, &owner, &[&owner]);
    assert!(result.is_err(), "Should fail when content too large for new size");
}

// =============================================================================
// CLOSE DATA ACCOUNT TESTS
// =============================================================================

#[test]
fn test_close_data_account() {
    if !program_exists() {
        eprintln!("Skipping test_close_data_account: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    let owner = funded_keypair_10_sol(&mut svm);

    // Initialize data
    let data_id: u64 = 1;
    let (data_pda, _) = derive_data_pda(data_id);

    let init_ix = anchor_instruction(
        prog_id,
        "initialize_data",
        &initialize_data_data(data_id, "Some content", 100),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(system_program::id()),
        ],
    );

    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    // Close account
    let close_ix = anchor_instruction(
        prog_id,
        "close_data_account",
        &close_data_account_data(data_id),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
        ],
    );

    let result = execute_tx(&mut svm, close_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Failed to close account: {:?}", result.err());

    // Verify account is marked as closed
    let account_data = get_account_data(&svm, &data_pda).unwrap();

    assert!(read_data_is_closed(&account_data));
    assert_eq!(read_data_allocated_size(&account_data), 0);
}

#[test]
fn test_close_data_account_unauthorized() {
    if !program_exists() {
        eprintln!("Skipping test_close_data_account_unauthorized: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    let owner = funded_keypair_10_sol(&mut svm);
    let attacker = funded_keypair_10_sol(&mut svm);

    // Initialize data
    let data_id: u64 = 1;
    let (data_pda, _) = derive_data_pda(data_id);

    let init_ix = anchor_instruction(
        prog_id,
        "initialize_data",
        &initialize_data_data(data_id, "Content", 50),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(system_program::id()),
        ],
    );

    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    // Try to close from wrong account
    let close_ix = anchor_instruction(
        prog_id,
        "close_data_account",
        &close_data_account_data(data_id),
        vec![
            signer_meta(attacker.pubkey()),
            writable_meta(data_pda),
        ],
    );

    let result = execute_tx(&mut svm, close_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "Should fail with unauthorized");
}

// =============================================================================
// RESIZE RECORD TESTS
// =============================================================================

#[test]
fn test_create_record() {
    if !program_exists() {
        eprintln!("Skipping test_create_record: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    let owner = funded_keypair_10_sol(&mut svm);

    // Create record
    let record_id: u64 = 1;
    let (record_pda, _) = derive_record_pda(record_id);

    let ix = anchor_instruction(
        prog_id,
        "create_record",
        &create_record_data(record_id),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(record_pda),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Failed to create record: {:?}", result.err());

    // Verify record state
    let record_data = get_account_data(&svm, &record_pda).unwrap();

    assert_eq!(read_record_owner(&record_data), owner.pubkey());
    assert_eq!(read_record_record_id(&record_data), record_id);
    assert_eq!(read_record_current_size(&record_data), 0);
    assert_eq!(read_record_max_size_reached(&record_data), 0);
    assert_eq!(read_record_resize_count(&record_data), 0);
    assert!(read_record_is_active(&record_data));
}

#[test]
fn test_record_resize_grow() {
    if !program_exists() {
        eprintln!("Skipping test_record_resize_grow: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    let owner = funded_keypair_10_sol(&mut svm);

    // Create record
    let record_id: u64 = 1;
    let (record_pda, _) = derive_record_pda(record_id);

    let create_ix = anchor_instruction(
        prog_id,
        "create_record",
        &create_record_data(record_id),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(record_pda),
            readonly_meta(system_program::id()),
        ],
    );

    execute_tx(&mut svm, create_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    // Record a grow resize
    let resize_ix = anchor_instruction(
        prog_id,
        "record_resize",
        &record_resize_data(record_id, 100, true),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(record_pda),
        ],
    );

    let result = execute_tx(&mut svm, resize_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Failed to record resize: {:?}", result.err());

    // Verify record state
    let record_data = get_account_data(&svm, &record_pda).unwrap();

    assert_eq!(read_record_current_size(&record_data), 100);
    assert_eq!(read_record_max_size_reached(&record_data), 100);
    assert_eq!(read_record_resize_count(&record_data), 1);
}

#[test]
fn test_record_resize_multiple() {
    if !program_exists() {
        eprintln!("Skipping test_record_resize_multiple: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    let owner = funded_keypair_10_sol(&mut svm);

    // Create record
    let record_id: u64 = 1;
    let (record_pda, _) = derive_record_pda(record_id);

    let create_ix = anchor_instruction(
        prog_id,
        "create_record",
        &create_record_data(record_id),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(record_pda),
            readonly_meta(system_program::id()),
        ],
    );

    execute_tx(&mut svm, create_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    // Grow to 100
    let resize1_ix = anchor_instruction(
        prog_id,
        "record_resize",
        &record_resize_data(record_id, 100, true),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(record_pda),
        ],
    );
    execute_tx(&mut svm, resize1_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    // Grow to 200
    let resize2_ix = anchor_instruction(
        prog_id,
        "record_resize",
        &record_resize_data(record_id, 200, true),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(record_pda),
        ],
    );
    execute_tx(&mut svm, resize2_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    // Shrink to 150
    let resize3_ix = anchor_instruction(
        prog_id,
        "record_resize",
        &record_resize_data(record_id, 150, false),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(record_pda),
        ],
    );
    execute_tx(&mut svm, resize3_ix, &owner, &[&owner]).unwrap();

    // Verify record state
    let record_data = get_account_data(&svm, &record_pda).unwrap();

    assert_eq!(read_record_current_size(&record_data), 150);
    assert_eq!(read_record_max_size_reached(&record_data), 200); // Max was 200
    assert_eq!(read_record_resize_count(&record_data), 3);
}

#[test]
fn test_close_record() {
    if !program_exists() {
        eprintln!("Skipping test_close_record: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    let owner = funded_keypair_10_sol(&mut svm);

    // Create record
    let record_id: u64 = 1;
    let (record_pda, _) = derive_record_pda(record_id);

    let create_ix = anchor_instruction(
        prog_id,
        "create_record",
        &create_record_data(record_id),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(record_pda),
            readonly_meta(system_program::id()),
        ],
    );

    execute_tx(&mut svm, create_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    // Record some resizes first
    let resize_ix = anchor_instruction(
        prog_id,
        "record_resize",
        &record_resize_data(record_id, 100, true),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(record_pda),
        ],
    );
    execute_tx(&mut svm, resize_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    // Close record
    let close_ix = anchor_instruction(
        prog_id,
        "close_record",
        &close_record_data(record_id),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(record_pda),
        ],
    );

    let result = execute_tx(&mut svm, close_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Failed to close record: {:?}", result.err());

    // Verify record is inactive
    let record_data = get_account_data(&svm, &record_pda).unwrap();
    assert!(!read_record_is_active(&record_data));
}

// =============================================================================
// FULL WORKFLOW TESTS
// =============================================================================

#[test]
fn test_full_realloc_close_workflow() {
    if !program_exists() {
        eprintln!("Skipping test_full_realloc_close_workflow: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    let owner = funded_keypair_10_sol(&mut svm);

    let data_id: u64 = 1;
    let (data_pda, _) = derive_data_pda(data_id);

    // 1. Initialize data
    let init_ix = anchor_instruction(
        prog_id,
        "initialize_data",
        &initialize_data_data(data_id, "Start", 50),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    // 2. Grow account
    let grow_ix = anchor_instruction(
        prog_id,
        "grow_account",
        &grow_account_data(data_id, "This is a longer content", 50),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
        ],
    );
    execute_tx(&mut svm, grow_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    let data = get_account_data(&svm, &data_pda).unwrap();
    assert_eq!(read_data_allocated_size(&data), 100);
    assert_eq!(read_data_version(&data), 2);

    // 3. Shrink account
    let shrink_ix = anchor_instruction(
        prog_id,
        "shrink_account",
        &shrink_account_data(data_id, "Shrunk", 75),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
        ],
    );
    execute_tx(&mut svm, shrink_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    let data = get_account_data(&svm, &data_pda).unwrap();
    assert_eq!(read_data_allocated_size(&data), 75);
    assert_eq!(read_data_version(&data), 3);

    // 4. Get info (just verify it doesn't error)
    let info_ix = anchor_instruction(
        prog_id,
        "get_data_info",
        &get_data_info_data(data_id),
        vec![
            signer_meta(owner.pubkey()),
            readonly_meta(data_pda),
        ],
    );
    let result = execute_tx(&mut svm, info_ix, &owner, &[&owner]);
    assert!(result.is_ok());
    svm.expire_blockhash();

    // 5. Close account
    let close_ix = anchor_instruction(
        prog_id,
        "close_data_account",
        &close_data_account_data(data_id),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
        ],
    );
    execute_tx(&mut svm, close_ix, &owner, &[&owner]).unwrap();

    // Verify closed
    let data = get_account_data(&svm, &data_pda).unwrap();
    assert!(read_data_is_closed(&data));
}

// =============================================================================
// PDA DERIVATION TESTS
// =============================================================================

#[test]
fn test_pda_derivation_deterministic() {
    // This test doesn't need program built - just verifies PDA derivation
    let data_id: u64 = 12345;
    let record_id: u64 = 67890;

    let (data_pda1, bump1) = derive_data_pda(data_id);
    let (data_pda2, bump2) = derive_data_pda(data_id);

    assert_eq!(data_pda1, data_pda2);
    assert_eq!(bump1, bump2);

    let (record_pda1, bump3) = derive_record_pda(record_id);
    let (record_pda2, bump4) = derive_record_pda(record_id);

    assert_eq!(record_pda1, record_pda2);
    assert_eq!(bump3, bump4);

    // Different IDs produce different PDAs
    let (other_data_pda, _) = derive_data_pda(99999);
    assert_ne!(data_pda1, other_data_pda);
}
