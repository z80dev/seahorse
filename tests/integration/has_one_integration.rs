//! LiteSVM integration tests for the has_one constraint
//!
//! Tests the has_one constraint which enforces that an account field
//! matches a signer's pubkey. This is used for authority checks.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile has_one_test.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// has_one_test program ID (from declare_id!)
fn has_one_program_id() -> Pubkey {
    Pubkey::from_str("HAS1111111111111111111111111111111111111111").unwrap()
}

/// Vault account size: discriminator (8) + owner (32) + authority (32) + data (8)
const VAULT_SIZE: usize = 8 + 32 + 32 + 8;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the has_one_test program into LiteSVM
fn load_has_one_program() -> (litesvm::LiteSVM, Keypair) {
    let program_id = has_one_program_id();
    let program_bytes = std::fs::read("../../target/deploy/has_one_test.so")
        .expect("Failed to read has_one_test.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);

    // Create a funded owner
    let owner = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    (svm, owner)
}

/// Read vault data field from account data
fn read_vault_data(data: &[u8]) -> u64 {
    // Skip discriminator (8) + owner (32) + authority (32), read data (8)
    let data_bytes: [u8; 8] = data[72..80].try_into().unwrap();
    u64::from_le_bytes(data_bytes)
}

/// Read vault authority from account data
fn read_vault_authority(data: &[u8]) -> Pubkey {
    // Skip discriminator (8) + owner (32), read authority (32)
    Pubkey::new_from_array(data[40..72].try_into().unwrap())
}

// =============================================================================
// CREATE VAULT TESTS
// =============================================================================

#[test]
fn test_create_vault() {
    let (mut svm, owner) = load_has_one_program();
    let program_id = has_one_program_id();

    // Derive vault PDA
    let (vault_pda, _bump) = find_pda(&[b"vault", owner.pubkey().as_ref()], &program_id);

    // Create vault instruction
    let ix = anchor_instruction(
        program_id,
        "create_vault",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    // Execute transaction
    let result = execute_tx(&mut svm, ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Create vault should succeed: {:?}", result);

    // Verify account state
    let account = svm.get_account(&vault_pda).expect("Vault should exist");
    assert_eq!(account.data.len(), VAULT_SIZE, "Account size should match");

    let data = read_vault_data(&account.data);
    assert_eq!(data, 0, "Data should be initialized to 0");

    let authority = read_vault_authority(&account.data);
    assert_eq!(authority, owner.pubkey(), "Authority should be set to owner");
}

// =============================================================================
// UPDATE VAULT TESTS (has_one constraint)
// =============================================================================

#[test]
fn test_update_vault_with_correct_authority() {
    let (mut svm, owner) = load_has_one_program();
    let program_id = has_one_program_id();
    let (vault_pda, _) = find_pda(&[b"vault", owner.pubkey().as_ref()], &program_id);

    // Create vault
    let create_ix = anchor_instruction(
        program_id,
        "create_vault",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &owner, &[&owner]).unwrap();

    // Update vault with correct authority (owner is the authority)
    let update_ix = anchor_instruction(
        program_id,
        "update_vault",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
        ],
    );
    let result = execute_tx(&mut svm, update_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Update with correct authority should succeed: {:?}", result);

    // Verify data was incremented
    let account = svm.get_account(&vault_pda).unwrap();
    assert_eq!(read_vault_data(&account.data), 1, "Data should be incremented to 1");
}

#[test]
fn test_update_vault_with_wrong_authority_fails() {
    let program_id = has_one_program_id();
    let program_bytes = std::fs::read("../../target/deploy/has_one_test.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);

    let owner = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let attacker = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    let (vault_pda, _) = find_pda(&[b"vault", owner.pubkey().as_ref()], &program_id);

    // Create vault (authority = owner)
    let create_ix = anchor_instruction(
        program_id,
        "create_vault",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &owner, &[&owner]).unwrap();

    // Attacker tries to update vault - should fail due to has_one constraint
    let update_ix = anchor_instruction(
        program_id,
        "update_vault",
        &[],
        vec![
            signer_meta(attacker.pubkey()),
            writable_meta(vault_pda),
        ],
    );
    let result = execute_tx(&mut svm, update_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "Update with wrong authority should fail due to has_one constraint");

    // Verify data was not changed
    let account = svm.get_account(&vault_pda).unwrap();
    assert_eq!(read_vault_data(&account.data), 0, "Data should remain 0");
}

#[test]
fn test_update_vault_multiple_times() {
    let (mut svm, owner) = load_has_one_program();
    let program_id = has_one_program_id();
    let (vault_pda, _) = find_pda(&[b"vault", owner.pubkey().as_ref()], &program_id);

    // Create vault
    let create_ix = anchor_instruction(
        program_id,
        "create_vault",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &owner, &[&owner]).unwrap();

    // Update vault 3 times
    for i in 1..=3 {
        svm.expire_blockhash();
        let update_ix = anchor_instruction(
            program_id,
            "update_vault",
            &[],
            vec![
                signer_meta(owner.pubkey()),
                writable_meta(vault_pda),
            ],
        );
        let result = execute_tx(&mut svm, update_ix, &owner, &[&owner]);
        assert!(result.is_ok(), "Update {} should succeed", i);
    }

    // Verify data was incremented 3 times
    let account = svm.get_account(&vault_pda).unwrap();
    assert_eq!(read_vault_data(&account.data), 3, "Data should be 3 after 3 updates");
}

// =============================================================================
// TRANSFER AUTHORITY TESTS (has_one constraint)
// =============================================================================

#[test]
fn test_transfer_authority_with_correct_authority() {
    let program_id = has_one_program_id();
    let program_bytes = std::fs::read("../../target/deploy/has_one_test.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);

    let owner = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let new_authority = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    let (vault_pda, _) = find_pda(&[b"vault", owner.pubkey().as_ref()], &program_id);

    // Create vault (authority = owner)
    let create_ix = anchor_instruction(
        program_id,
        "create_vault",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &owner, &[&owner]).unwrap();

    // Transfer authority from owner to new_authority
    let transfer_ix = anchor_instruction(
        program_id,
        "transfer_authority",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            signer_meta(new_authority.pubkey()),
            writable_meta(vault_pda),
        ],
    );
    let result = execute_tx(&mut svm, transfer_ix, &owner, &[&owner, &new_authority]);
    assert!(result.is_ok(), "Transfer authority should succeed: {:?}", result);

    // Verify authority was transferred
    let account = svm.get_account(&vault_pda).unwrap();
    assert_eq!(
        read_vault_authority(&account.data),
        new_authority.pubkey(),
        "Authority should be transferred to new_authority"
    );
}

#[test]
fn test_transfer_authority_with_wrong_authority_fails() {
    let program_id = has_one_program_id();
    let program_bytes = std::fs::read("../../target/deploy/has_one_test.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);

    let owner = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let attacker = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let new_authority = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    let (vault_pda, _) = find_pda(&[b"vault", owner.pubkey().as_ref()], &program_id);

    // Create vault (authority = owner)
    let create_ix = anchor_instruction(
        program_id,
        "create_vault",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &owner, &[&owner]).unwrap();

    // Attacker tries to transfer authority - should fail
    let transfer_ix = anchor_instruction(
        program_id,
        "transfer_authority",
        &[],
        vec![
            signer_meta(attacker.pubkey()),
            signer_meta(new_authority.pubkey()),
            writable_meta(vault_pda),
        ],
    );
    let result = execute_tx(&mut svm, transfer_ix, &attacker, &[&attacker, &new_authority]);
    assert!(result.is_err(), "Transfer with wrong authority should fail due to has_one constraint");

    // Verify authority was not changed
    let account = svm.get_account(&vault_pda).unwrap();
    assert_eq!(
        read_vault_authority(&account.data),
        owner.pubkey(),
        "Authority should remain unchanged"
    );
}

#[test]
fn test_new_authority_can_update_after_transfer() {
    let program_id = has_one_program_id();
    let program_bytes = std::fs::read("../../target/deploy/has_one_test.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);

    let owner = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let new_authority = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    let (vault_pda, _) = find_pda(&[b"vault", owner.pubkey().as_ref()], &program_id);

    // Create vault
    let create_ix = anchor_instruction(
        program_id,
        "create_vault",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &owner, &[&owner]).unwrap();

    // Transfer authority
    let transfer_ix = anchor_instruction(
        program_id,
        "transfer_authority",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            signer_meta(new_authority.pubkey()),
            writable_meta(vault_pda),
        ],
    );
    execute_tx(&mut svm, transfer_ix, &owner, &[&owner, &new_authority]).unwrap();

    // New authority can update the vault
    svm.expire_blockhash();
    let update_ix = anchor_instruction(
        program_id,
        "update_vault",
        &[],
        vec![
            signer_meta(new_authority.pubkey()),
            writable_meta(vault_pda),
        ],
    );
    let result = execute_tx(&mut svm, update_ix, &new_authority, &[&new_authority]);
    assert!(result.is_ok(), "New authority should be able to update: {:?}", result);

    // Old authority can no longer update
    svm.expire_blockhash();
    let old_update_ix = anchor_instruction(
        program_id,
        "update_vault",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
        ],
    );
    let result = execute_tx(&mut svm, old_update_ix, &owner, &[&owner]);
    assert!(result.is_err(), "Old authority should no longer be able to update");
}
