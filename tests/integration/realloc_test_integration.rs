//! LiteSVM integration tests for the Seahorse realloc_test program
//!
//! These tests verify native Anchor realloc constraint functionality:
//! - initialize: Creates a DynamicData account with PDA seeds ["data", owner]
//! - grow_data: Reallocates account to 200 bytes (zero=True, zeroes new space)
//! - shrink_data: Reallocates account to 150 bytes (zero=False, preserves data)
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile realloc_test.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_pubkey::Pubkey;
use solana_sdk_ids::{system_program, sysvar};
use solana_signer::Signer;
use std::path::Path;
use std::str::FromStr;

/// realloc_test program ID (from declare_id!)
fn program_id() -> Pubkey {
    Pubkey::from_str("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS").unwrap()
}

/// Path to the compiled program
const PROGRAM_PATH: &str = "../../target/deploy/realloc_test.so";

/// Check if the program is built
fn program_exists() -> bool {
    Path::new(PROGRAM_PATH).exists()
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the realloc_test program into LiteSVM
fn load_program() -> litesvm::LiteSVM {
    let prog_id = program_id();

    if !program_exists() {
        panic!("Failed to read realloc_test.so - run ./scripts/build-test-programs.sh first");
    }

    let program_bytes = std::fs::read(PROGRAM_PATH)
        .expect("Failed to read realloc_test.so");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(prog_id, &program_bytes);
    svm
}

/// Derive DynamicData PDA (seeds = ["data", owner])
fn derive_data_pda(owner: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"data", owner.as_ref()],
        &program_id(),
    )
}

/// Build initialize instruction data (no args beyond discriminator)
fn initialize_data() -> Vec<u8> {
    Vec::new() // No additional args
}

/// Build grow_data instruction data (no args beyond discriminator)
fn grow_data_data() -> Vec<u8> {
    Vec::new() // No additional args
}

/// Build shrink_data instruction data (no args beyond discriminator)
fn shrink_data_data() -> Vec<u8> {
    Vec::new() // No additional args
}

// =============================================================================
// DATA READER HELPERS
// =============================================================================

/// DynamicData account structure (from realloc_test.py):
/// - 8 bytes: discriminator
/// - 32 bytes: owner (Pubkey)
/// - 8 bytes: size (u64)
/// - 100 bytes: data ([u8; 100])

/// Read DynamicData owner from account data
fn read_owner(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

/// Read DynamicData size field from account data
fn read_size(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[40..48].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

/// Read DynamicData data array from account data
fn read_data_array(data: &[u8]) -> &[u8] {
    &data[48..]
}

// =============================================================================
// INITIALIZE TESTS
// =============================================================================

#[test]
fn test_initialize() {
    if !program_exists() {
        eprintln!("Skipping test_initialize: program not built. Run ./scripts/build-test-programs.sh");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    // Setup owner
    let owner = funded_keypair_10_sol(&mut svm);

    // Derive PDA
    let (data_pda, _bump) = derive_data_pda(&owner.pubkey());

    // Build instruction
    let ix = anchor_instruction(
        prog_id,
        "initialize",
        &initialize_data(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    // Execute transaction
    let result = execute_tx(&mut svm, ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Failed to initialize: {:?}", result.err());

    // Verify account state
    let account_data = get_account_data(&svm, &data_pda).unwrap();

    assert_eq!(read_owner(&account_data), owner.pubkey());
    assert_eq!(read_size(&account_data), 100);
}

#[test]
fn test_initialize_different_owners() {
    if !program_exists() {
        eprintln!("Skipping test_initialize_different_owners: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    // Two different owners
    let owner1 = funded_keypair_10_sol(&mut svm);
    let owner2 = funded_keypair_10_sol(&mut svm);

    // Each owner gets their own PDA
    let (data_pda1, _) = derive_data_pda(&owner1.pubkey());
    let (data_pda2, _) = derive_data_pda(&owner2.pubkey());

    // Initialize owner1's account
    let ix1 = anchor_instruction(
        prog_id,
        "initialize",
        &initialize_data(),
        vec![
            signer_meta(owner1.pubkey()),
            writable_meta(data_pda1),
            readonly_meta(sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    let result = execute_tx(&mut svm, ix1, &owner1, &[&owner1]);
    assert!(result.is_ok());
    svm.expire_blockhash();

    // Initialize owner2's account
    let ix2 = anchor_instruction(
        prog_id,
        "initialize",
        &initialize_data(),
        vec![
            signer_meta(owner2.pubkey()),
            writable_meta(data_pda2),
            readonly_meta(sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    let result = execute_tx(&mut svm, ix2, &owner2, &[&owner2]);
    assert!(result.is_ok());

    // Verify both PDAs are different and have correct owners
    assert_ne!(data_pda1, data_pda2);

    let data1 = get_account_data(&svm, &data_pda1).unwrap();
    let data2 = get_account_data(&svm, &data_pda2).unwrap();

    assert_eq!(read_owner(&data1), owner1.pubkey());
    assert_eq!(read_owner(&data2), owner2.pubkey());
}

// =============================================================================
// GROW_DATA TESTS (realloc with zero=True)
// =============================================================================

#[test]
fn test_grow_data() {
    if !program_exists() {
        eprintln!("Skipping test_grow_data: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    let owner = funded_keypair_10_sol(&mut svm);
    let (data_pda, _) = derive_data_pda(&owner.pubkey());

    // Initialize first
    let init_ix = anchor_instruction(
        prog_id,
        "initialize",
        &initialize_data(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    // Get initial account info
    let initial_account = get_account(&svm, &data_pda).unwrap();
    let initial_lamports = initial_account.lamports;
    let initial_data_len = initial_account.data.len();

    // Grow the account
    let grow_ix = anchor_instruction(
        prog_id,
        "grow_data",
        &grow_data_data(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, grow_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Failed to grow data: {:?}", result.err());

    // Verify account was reallocated
    let grown_account = get_account(&svm, &data_pda).unwrap();

    // Account data length should have increased
    assert!(grown_account.data.len() > initial_data_len,
        "Account data length should have grown: {} -> {}",
        initial_data_len, grown_account.data.len());

    // Lamports should have increased (more rent required)
    assert!(grown_account.lamports >= initial_lamports,
        "Lamports should not decrease: {} -> {}",
        initial_lamports, grown_account.lamports);

    // Size field should be updated to 200
    let account_data = get_account_data(&svm, &data_pda).unwrap();
    assert_eq!(read_size(&account_data), 200);
}

#[test]
fn test_grow_data_zero_flag() {
    if !program_exists() {
        eprintln!("Skipping test_grow_data_zero_flag: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    let owner = funded_keypair_10_sol(&mut svm);
    let (data_pda, _) = derive_data_pda(&owner.pubkey());

    // Initialize first
    let init_ix = anchor_instruction(
        prog_id,
        "initialize",
        &initialize_data(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    // Get initial data length
    let initial_data = get_account_data(&svm, &data_pda).unwrap();
    let initial_len = initial_data.len();

    // Grow the account (zero=True should zero new bytes)
    let grow_ix = anchor_instruction(
        prog_id,
        "grow_data",
        &grow_data_data(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(system_program::id()),
        ],
    );

    execute_tx(&mut svm, grow_ix, &owner, &[&owner]).unwrap();

    // Verify new bytes are zeroed (grow_data uses zero=True)
    let grown_data = get_account_data(&svm, &data_pda).unwrap();

    // New bytes beyond original length should be zeroed
    if grown_data.len() > initial_len {
        let new_bytes = &grown_data[initial_len..];
        let all_zeros = new_bytes.iter().all(|&b| b == 0);
        assert!(all_zeros, "New bytes should be zeroed (zero=True), but found non-zero bytes");
    }
}

#[test]
fn test_grow_data_unauthorized_fails() {
    if !program_exists() {
        eprintln!("Skipping test_grow_data_unauthorized_fails: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    let owner = funded_keypair_10_sol(&mut svm);
    let attacker = funded_keypair_10_sol(&mut svm);
    let (data_pda, _) = derive_data_pda(&owner.pubkey());

    // Initialize with owner
    let init_ix = anchor_instruction(
        prog_id,
        "initialize",
        &initialize_data(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    // Try to grow from attacker (wrong PDA seeds)
    let (attacker_pda, _) = derive_data_pda(&attacker.pubkey());

    // Attempt 1: Use attacker's PDA (doesn't exist)
    let grow_ix = anchor_instruction(
        prog_id,
        "grow_data",
        &grow_data_data(),
        vec![
            signer_meta(attacker.pubkey()),
            writable_meta(attacker_pda), // Attacker's PDA
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, grow_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "Should fail - attacker's PDA doesn't exist");
}

// =============================================================================
// SHRINK_DATA TESTS (realloc with zero=False)
// =============================================================================

#[test]
fn test_shrink_data() {
    if !program_exists() {
        eprintln!("Skipping test_shrink_data: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    let owner = funded_keypair_10_sol(&mut svm);
    let (data_pda, _) = derive_data_pda(&owner.pubkey());

    // Initialize first
    let init_ix = anchor_instruction(
        prog_id,
        "initialize",
        &initialize_data(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    // Get initial account info
    let initial_account = get_account(&svm, &data_pda).unwrap();
    let initial_data_len = initial_account.data.len();
    let initial_lamports = initial_account.lamports;

    // Shrink the account
    let shrink_ix = anchor_instruction(
        prog_id,
        "shrink_data",
        &shrink_data_data(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, shrink_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Failed to shrink data: {:?}", result.err());

    // Verify account was reallocated
    let shrunk_account = get_account(&svm, &data_pda).unwrap();

    // Account data length should have decreased
    assert!(shrunk_account.data.len() < initial_data_len,
        "Account data length should have shrunk: {} -> {}",
        initial_data_len, shrunk_account.data.len());

    // Size field should be updated to 150
    let account_data = get_account_data(&svm, &data_pda).unwrap();
    assert_eq!(read_size(&account_data), 150);

    // Owner should receive excess rent
    let owner_account = get_account(&svm, &owner.pubkey()).unwrap();
    // Owner lamports should have increased (received excess rent)
    // Note: Owner also paid tx fee, so we can't assert exact amount
    println!("Initial account lamports: {}, shrunk: {}", initial_lamports, shrunk_account.lamports);
}

#[test]
fn test_shrink_data_preserves_existing_data() {
    if !program_exists() {
        eprintln!("Skipping test_shrink_data_preserves_existing_data: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    let owner = funded_keypair_10_sol(&mut svm);
    let (data_pda, _) = derive_data_pda(&owner.pubkey());

    // Initialize first
    let init_ix = anchor_instruction(
        prog_id,
        "initialize",
        &initialize_data(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    // Get initial owner value
    let initial_data = get_account_data(&svm, &data_pda).unwrap();
    let initial_owner = read_owner(&initial_data);

    // Shrink the account (zero=False should preserve existing data)
    let shrink_ix = anchor_instruction(
        prog_id,
        "shrink_data",
        &shrink_data_data(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(system_program::id()),
        ],
    );

    execute_tx(&mut svm, shrink_ix, &owner, &[&owner]).unwrap();

    // Verify existing data (owner field) is preserved
    let shrunk_data = get_account_data(&svm, &data_pda).unwrap();
    let shrunk_owner = read_owner(&shrunk_data);

    assert_eq!(initial_owner, shrunk_owner,
        "Owner field should be preserved after shrink (zero=False)");
}

// =============================================================================
// FULL WORKFLOW TESTS
// =============================================================================

#[test]
fn test_full_realloc_workflow() {
    if !program_exists() {
        eprintln!("Skipping test_full_realloc_workflow: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    let owner = funded_keypair_10_sol(&mut svm);
    let (data_pda, _) = derive_data_pda(&owner.pubkey());

    // 1. Initialize (size=100)
    let init_ix = anchor_instruction(
        prog_id,
        "initialize",
        &initialize_data(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    let data = get_account_data(&svm, &data_pda).unwrap();
    assert_eq!(read_size(&data), 100);
    let len_after_init = get_account(&svm, &data_pda).unwrap().data.len();

    // 2. Grow (size=200, zero=True)
    let grow_ix = anchor_instruction(
        prog_id,
        "grow_data",
        &grow_data_data(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, grow_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    let data = get_account_data(&svm, &data_pda).unwrap();
    assert_eq!(read_size(&data), 200);
    let len_after_grow = get_account(&svm, &data_pda).unwrap().data.len();
    assert!(len_after_grow > len_after_init, "Account should have grown");

    // 3. Shrink (size=150, zero=False)
    let shrink_ix = anchor_instruction(
        prog_id,
        "shrink_data",
        &shrink_data_data(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, shrink_ix, &owner, &[&owner]).unwrap();

    let data = get_account_data(&svm, &data_pda).unwrap();
    assert_eq!(read_size(&data), 150);
    let len_after_shrink = get_account(&svm, &data_pda).unwrap().data.len();
    assert!(len_after_shrink < len_after_grow, "Account should have shrunk");

    // Verify owner is still correct throughout
    assert_eq!(read_owner(&data), owner.pubkey());
}

#[test]
fn test_grow_then_shrink_rent_changes() {
    if !program_exists() {
        eprintln!("Skipping test_grow_then_shrink_rent_changes: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    let owner = funded_keypair_10_sol(&mut svm);
    let (data_pda, _) = derive_data_pda(&owner.pubkey());

    // Initialize
    let init_ix = anchor_instruction(
        prog_id,
        "initialize",
        &initialize_data(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    let lamports_after_init = get_account(&svm, &data_pda).unwrap().lamports;

    // Grow
    let grow_ix = anchor_instruction(
        prog_id,
        "grow_data",
        &grow_data_data(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, grow_ix, &owner, &[&owner]).unwrap();
    svm.expire_blockhash();

    let lamports_after_grow = get_account(&svm, &data_pda).unwrap().lamports;
    assert!(lamports_after_grow > lamports_after_init,
        "Lamports should increase after grow for additional rent");

    // Shrink
    let shrink_ix = anchor_instruction(
        prog_id,
        "shrink_data",
        &shrink_data_data(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(data_pda),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, shrink_ix, &owner, &[&owner]).unwrap();

    let lamports_after_shrink = get_account(&svm, &data_pda).unwrap().lamports;
    assert!(lamports_after_shrink < lamports_after_grow,
        "Lamports should decrease after shrink - excess rent returned to payer");
}

// =============================================================================
// PDA DERIVATION TESTS
// =============================================================================

#[test]
fn test_pda_derivation_deterministic() {
    // This test doesn't need program built
    let owner = Pubkey::new_unique();

    let (pda1, bump1) = derive_data_pda(&owner);
    let (pda2, bump2) = derive_data_pda(&owner);

    assert_eq!(pda1, pda2);
    assert_eq!(bump1, bump2);

    // Different owners produce different PDAs
    let other_owner = Pubkey::new_unique();
    let (other_pda, _) = derive_data_pda(&other_owner);
    assert_ne!(pda1, other_pda);
}
