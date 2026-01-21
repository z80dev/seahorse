//! LiteSVM integration tests for the Seahorse PDA Patterns program
//!
//! These tests verify various PDA derivation patterns:
//! - Single seed: global config
//! - User-derived: user profile with owner key
//! - Multiple seeds: data record with category + id
//! - Multiple seeds: vault with owner + id
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile pda_patterns.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// PDA Patterns program ID (from declare_id!)
fn pda_program_id() -> Pubkey {
    Pubkey::from_str("DxpKiPEUYHy6NKsnEMRvRKtC3ncrtP8epGt5KKS59Xzh").unwrap()
}

// Account layout reference:
// GlobalConfig: discriminator(8) + admin(32) + counter(8) + bump(1) = 49 bytes
// UserProfile: discriminator(8) + owner(32) + name_len(4) + name(var) + visits(8) + bump(1)
// Vault: discriminator(8) + owner(32) + vault_id(8) + balance(8) + bump(1) = 57 bytes

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the PDA patterns program into LiteSVM
fn load_pda_program() -> (litesvm::LiteSVM, Keypair) {
    let program_id = pda_program_id();
    let program_bytes = std::fs::read("../../target/deploy/pda_patterns.so")
        .expect("pda_patterns.so not found - run ./scripts/build-test-programs.sh");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);

    // Create a funded user
    let user = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    (svm, user)
}

/// Read GlobalConfig from account data
fn read_global_config(data: &[u8]) -> (Pubkey, u64, u8) {
    // discriminator(8) + admin(32) + counter(8) + bump(1)
    let admin = Pubkey::new_from_array(data[8..40].try_into().unwrap());
    let counter = u64::from_le_bytes(data[40..48].try_into().unwrap());
    let bump = data[48];
    (admin, counter, bump)
}

/// Read Vault from account data
fn read_vault(data: &[u8]) -> (Pubkey, u64, u64, u8) {
    // discriminator(8) + owner(32) + vault_id(8) + balance(8) + bump(1)
    let owner = Pubkey::new_from_array(data[8..40].try_into().unwrap());
    let vault_id = u64::from_le_bytes(data[40..48].try_into().unwrap());
    let balance = u64::from_le_bytes(data[48..56].try_into().unwrap());
    let bump = data[56];
    (owner, vault_id, balance, bump)
}

/// Read UserProfile from account data (simplified - just reads owner and visits)
fn read_user_profile(data: &[u8]) -> (Pubkey, u64, u8) {
    // discriminator(8) + owner(32) + name_len(4) + name(var) + visits(8) + bump(1)
    let owner = Pubkey::new_from_array(data[8..40].try_into().unwrap());
    // Read name length to find visits offset
    let name_len = u32::from_le_bytes(data[40..44].try_into().unwrap()) as usize;
    let visits_offset = 44 + name_len;
    let visits = u64::from_le_bytes(data[visits_offset..visits_offset + 8].try_into().unwrap());
    let bump = data[visits_offset + 8];
    (owner, visits, bump)
}

/// Serialize a string for Anchor (4-byte length prefix + UTF-8 bytes)
fn serialize_string(s: &str) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&(s.len() as u32).to_le_bytes());
    data.extend_from_slice(s.as_bytes());
    data
}

// =============================================================================
// SINGLE SEED TESTS: GLOBAL CONFIG
// =============================================================================

#[test]
fn test_init_global_config() {
    let (mut svm, admin) = load_pda_program();
    let program_id = pda_program_id();

    // Derive PDA with single seed
    let (config_pda, _bump) = find_pda(&[b"global_config"], &program_id);

    // Create initialize instruction
    let ix = anchor_instruction(
        program_id,
        "init_global_config",
        &[],
        vec![
            signer_meta(admin.pubkey()),
            writable_meta(config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    // Execute transaction
    let result = execute_tx(&mut svm, ix, &admin, &[&admin]);
    assert!(result.is_ok(), "Init global config should succeed: {:?}", result);

    // Verify account state
    let account = svm.get_account(&config_pda).expect("Config should exist");
    let (stored_admin, counter, bump) = read_global_config(&account.data);

    assert_eq!(stored_admin, admin.pubkey(), "Admin should be set");
    assert_eq!(counter, 0, "Counter should be initialized to 0");
    assert!(bump > 0, "Bump should be non-zero");
}

#[test]
fn test_update_global_config() {
    let (mut svm, admin) = load_pda_program();
    let program_id = pda_program_id();

    let (config_pda, _) = find_pda(&[b"global_config"], &program_id);

    // Initialize
    let init_ix = anchor_instruction(
        program_id,
        "init_global_config",
        &[],
        vec![
            signer_meta(admin.pubkey()),
            writable_meta(config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &admin, &[&admin]).unwrap();

    // Update (increment counter)
    let update_ix = anchor_instruction(
        program_id,
        "update_global_config",
        &[],
        vec![
            signer_meta(admin.pubkey()),
            writable_meta(config_pda),
        ],
    );
    let result = execute_tx(&mut svm, update_ix, &admin, &[&admin]);
    assert!(result.is_ok(), "Update should succeed");

    // Verify counter incremented
    let account = svm.get_account(&config_pda).unwrap();
    let (_, counter, _) = read_global_config(&account.data);
    assert_eq!(counter, 1, "Counter should be 1 after update");
}

#[test]
fn test_global_config_unauthorized_fails() {
    let (mut svm, admin) = load_pda_program();
    let program_id = pda_program_id();
    let attacker = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    let (config_pda, _) = find_pda(&[b"global_config"], &program_id);

    // Admin initializes
    let init_ix = anchor_instruction(
        program_id,
        "init_global_config",
        &[],
        vec![
            signer_meta(admin.pubkey()),
            writable_meta(config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &admin, &[&admin]).unwrap();

    // Attacker tries to update - should fail
    let update_ix = anchor_instruction(
        program_id,
        "update_global_config",
        &[],
        vec![
            signer_meta(attacker.pubkey()),
            writable_meta(config_pda),
        ],
    );
    let result = execute_tx(&mut svm, update_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "Unauthorized update should fail");
}

// =============================================================================
// USER-DERIVED PDA TESTS: USER PROFILE
// =============================================================================

#[test]
fn test_init_user_profile() {
    let (mut svm, user) = load_pda_program();
    let program_id = pda_program_id();

    // Derive user-specific PDA
    let (profile_pda, _bump) = find_pda(&[b"user_profile", user.pubkey().as_ref()], &program_id);

    // Serialize name argument
    let name = "TestUser";
    let mut data = Vec::new();
    data.extend_from_slice(&serialize_string(name));

    let ix = anchor_instruction(
        program_id,
        "init_user_profile",
        &data,
        vec![
            signer_meta(user.pubkey()),
            writable_meta(profile_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &user, &[&user]);
    assert!(result.is_ok(), "Init user profile should succeed: {:?}", result);

    // Verify account state
    let account = svm.get_account(&profile_pda).expect("Profile should exist");
    let (owner, visits, bump) = read_user_profile(&account.data);

    assert_eq!(owner, user.pubkey(), "Owner should be user");
    assert_eq!(visits, 0, "Visits should be 0");
    assert!(bump > 0, "Bump should be stored");
}

#[test]
fn test_different_users_different_profiles() {
    let (mut svm, user1) = load_pda_program();
    let program_id = pda_program_id();
    let user2 = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    // Derive PDAs for each user
    let (profile1_pda, _) = find_pda(&[b"user_profile", user1.pubkey().as_ref()], &program_id);
    let (profile2_pda, _) = find_pda(&[b"user_profile", user2.pubkey().as_ref()], &program_id);

    // PDAs should be different
    assert_ne!(profile1_pda, profile2_pda, "Different users should have different profile PDAs");

    // Initialize user1's profile
    let data1 = serialize_string("User One");
    let ix1 = anchor_instruction(
        program_id,
        "init_user_profile",
        &data1,
        vec![
            signer_meta(user1.pubkey()),
            writable_meta(profile1_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix1, &user1, &[&user1]).unwrap();

    // Initialize user2's profile
    let data2 = serialize_string("User Two");
    let ix2 = anchor_instruction(
        program_id,
        "init_user_profile",
        &data2,
        vec![
            signer_meta(user2.pubkey()),
            writable_meta(profile2_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix2, &user2, &[&user2]).unwrap();

    // Both profiles should exist independently
    let account1 = svm.get_account(&profile1_pda).unwrap();
    let account2 = svm.get_account(&profile2_pda).unwrap();

    let (owner1, _, _) = read_user_profile(&account1.data);
    let (owner2, _, _) = read_user_profile(&account2.data);

    assert_eq!(owner1, user1.pubkey());
    assert_eq!(owner2, user2.pubkey());
}

#[test]
fn test_visit_profile() {
    let (mut svm, user) = load_pda_program();
    let program_id = pda_program_id();

    let (profile_pda, _) = find_pda(&[b"user_profile", user.pubkey().as_ref()], &program_id);

    // Initialize profile
    let init_data = serialize_string("Visitor");
    let init_ix = anchor_instruction(
        program_id,
        "init_user_profile",
        &init_data,
        vec![
            signer_meta(user.pubkey()),
            writable_meta(profile_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &user, &[&user]).unwrap();

    // Visit profile
    let visit_ix = anchor_instruction(
        program_id,
        "visit_profile",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(profile_pda),
        ],
    );
    let result = execute_tx(&mut svm, visit_ix, &user, &[&user]);
    assert!(result.is_ok(), "Visit profile should succeed");

    // Verify visits incremented
    let account = svm.get_account(&profile_pda).unwrap();
    let (_, visits, _) = read_user_profile(&account.data);
    assert_eq!(visits, 1, "Visits should be 1 after visit");
}

// =============================================================================
// MULTIPLE SEEDS TESTS: VAULT
// =============================================================================

#[test]
fn test_init_vault() {
    let (mut svm, owner) = load_pda_program();
    let program_id = pda_program_id();

    let vault_id: u64 = 1;
    let (vault_pda, _bump) = find_pda(
        &[b"vault", owner.pubkey().as_ref(), &vault_id.to_le_bytes()],
        &program_id,
    );

    // Serialize vault_id argument
    let data = vault_id.to_le_bytes().to_vec();

    let ix = anchor_instruction(
        program_id,
        "init_vault",
        &data,
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Init vault should succeed: {:?}", result);

    // Verify account state
    let account = svm.get_account(&vault_pda).expect("Vault should exist");
    let (stored_owner, stored_id, balance, bump) = read_vault(&account.data);

    assert_eq!(stored_owner, owner.pubkey(), "Owner should be set");
    assert_eq!(stored_id, vault_id, "Vault ID should match");
    assert_eq!(balance, 0, "Balance should be 0");
    assert!(bump > 0, "Bump should be stored");
}

#[test]
fn test_multiple_vaults_same_owner() {
    let (mut svm, owner) = load_pda_program();
    let program_id = pda_program_id();

    // Create two vaults with different IDs
    let vault_id_1: u64 = 1;
    let vault_id_2: u64 = 2;

    let (vault1_pda, _) = find_pda(
        &[b"vault", owner.pubkey().as_ref(), &vault_id_1.to_le_bytes()],
        &program_id,
    );
    let (vault2_pda, _) = find_pda(
        &[b"vault", owner.pubkey().as_ref(), &vault_id_2.to_le_bytes()],
        &program_id,
    );

    assert_ne!(vault1_pda, vault2_pda, "Different vault IDs should produce different PDAs");

    // Initialize vault 1
    let ix1 = anchor_instruction(
        program_id,
        "init_vault",
        &vault_id_1.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault1_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix1, &owner, &[&owner]).unwrap();

    // Initialize vault 2
    let ix2 = anchor_instruction(
        program_id,
        "init_vault",
        &vault_id_2.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault2_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix2, &owner, &[&owner]).unwrap();

    // Both vaults should exist with correct IDs
    let account1 = svm.get_account(&vault1_pda).unwrap();
    let account2 = svm.get_account(&vault2_pda).unwrap();

    let (_, id1, _, _) = read_vault(&account1.data);
    let (_, id2, _, _) = read_vault(&account2.data);

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}

#[test]
fn test_deposit_to_vault() {
    let (mut svm, owner) = load_pda_program();
    let program_id = pda_program_id();

    let vault_id: u64 = 42;
    let (vault_pda, _) = find_pda(
        &[b"vault", owner.pubkey().as_ref(), &vault_id.to_le_bytes()],
        &program_id,
    );

    // Initialize vault
    let init_ix = anchor_instruction(
        program_id,
        "init_vault",
        &vault_id.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // Deposit to vault
    // deposit_to_vault(owner, vault, amount) - only amount in instruction data
    let amount: u64 = 1000;

    let deposit_ix = anchor_instruction(
        program_id,
        "deposit_to_vault",
        &amount.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
        ],
    );
    let result = execute_tx(&mut svm, deposit_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Deposit should succeed");

    // Verify balance
    let account = svm.get_account(&vault_pda).unwrap();
    let (_, _, balance, _) = read_vault(&account.data);
    assert_eq!(balance, 1000, "Balance should be 1000 after deposit");
}

#[test]
fn test_vault_unauthorized_deposit_fails() {
    let (mut svm, owner) = load_pda_program();
    let program_id = pda_program_id();
    let attacker = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    let vault_id: u64 = 99;
    let (vault_pda, _) = find_pda(
        &[b"vault", owner.pubkey().as_ref(), &vault_id.to_le_bytes()],
        &program_id,
    );

    // Owner initializes vault
    let init_ix = anchor_instruction(
        program_id,
        "init_vault",
        &vault_id.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // Attacker tries to deposit - should fail
    let amount: u64 = 999;

    let deposit_ix = anchor_instruction(
        program_id,
        "deposit_to_vault",
        &amount.to_le_bytes(),
        vec![
            signer_meta(attacker.pubkey()),
            writable_meta(vault_pda),
        ],
    );
    let result = execute_tx(&mut svm, deposit_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "Unauthorized deposit should fail");
}

// =============================================================================
// MULTIPLE SEEDS TESTS: DATA RECORD
// =============================================================================

#[test]
fn test_init_data_record() {
    let (mut svm, creator) = load_pda_program();
    let program_id = pda_program_id();

    let category = "notes";
    let record_id: u64 = 1;

    let (record_pda, _bump) = find_pda(
        &[b"data_record", category.as_bytes(), &record_id.to_le_bytes()],
        &program_id,
    );

    // Serialize arguments: category (string), record_id (u64), data (string)
    let mut data = Vec::new();
    data.extend_from_slice(&serialize_string(category));
    data.extend_from_slice(&record_id.to_le_bytes());
    data.extend_from_slice(&serialize_string("My first note"));

    let ix = anchor_instruction(
        program_id,
        "init_data_record",
        &data,
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(record_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &creator, &[&creator]);
    assert!(result.is_ok(), "Init data record should succeed: {:?}", result);

    // Verify account exists
    let account = svm.get_account(&record_pda).expect("Record should exist");
    // Verify creator pubkey is at offset 8 (after discriminator)
    let stored_creator = Pubkey::new_from_array(account.data[8..40].try_into().unwrap());
    assert_eq!(stored_creator, creator.pubkey(), "Creator should be set");
}

#[test]
fn test_multiple_data_records() {
    let (mut svm, creator) = load_pda_program();
    let program_id = pda_program_id();

    // Create records in different categories
    let records = vec![
        ("notes", 1u64, "Note 1"),
        ("notes", 2u64, "Note 2"),
        ("tasks", 1u64, "Task 1"),
    ];

    for (category, record_id, content) in &records {
        let (record_pda, _) = find_pda(
            &[b"data_record", category.as_bytes(), &record_id.to_le_bytes()],
            &program_id,
        );

        let mut data = Vec::new();
        data.extend_from_slice(&serialize_string(category));
        data.extend_from_slice(&record_id.to_le_bytes());
        data.extend_from_slice(&serialize_string(content));

        let ix = anchor_instruction(
            program_id,
            "init_data_record",
            &data,
            vec![
                signer_meta(creator.pubkey()),
                writable_meta(record_pda),
                readonly_meta(solana_sdk_ids::sysvar::rent::id()),
                readonly_meta(system_program::id()),
            ],
        );

        let result = execute_tx(&mut svm, ix, &creator, &[&creator]);
        assert!(result.is_ok(), "Init record ({}, {}) should succeed", category, record_id);
    }

    // Verify all records exist with different addresses
    let pda1 = find_pda(&[b"data_record", b"notes", &1u64.to_le_bytes()], &program_id).0;
    let pda2 = find_pda(&[b"data_record", b"notes", &2u64.to_le_bytes()], &program_id).0;
    let pda3 = find_pda(&[b"data_record", b"tasks", &1u64.to_le_bytes()], &program_id).0;

    assert_ne!(pda1, pda2, "Different IDs in same category should have different PDAs");
    assert_ne!(pda1, pda3, "Same ID in different categories should have different PDAs");

    // All accounts should exist
    assert!(svm.get_account(&pda1).is_some());
    assert!(svm.get_account(&pda2).is_some());
    assert!(svm.get_account(&pda3).is_some());
}

// =============================================================================
// BUMP PERSISTENCE TESTS
// =============================================================================

#[test]
fn test_bump_is_stored_and_consistent() {
    let (mut svm, user) = load_pda_program();
    let program_id = pda_program_id();

    // Calculate expected bump
    let (vault_pda, expected_bump) = find_pda(
        &[b"vault", user.pubkey().as_ref(), &1u64.to_le_bytes()],
        &program_id,
    );

    // Initialize vault
    let init_ix = anchor_instruction(
        program_id,
        "init_vault",
        &1u64.to_le_bytes(),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &user, &[&user]).unwrap();

    // Read stored bump
    let account = svm.get_account(&vault_pda).unwrap();
    let (_, _, _, stored_bump) = read_vault(&account.data);

    // Stored bump should match calculated bump
    assert_eq!(stored_bump, expected_bump, "Stored bump should match PDA derivation bump");
}

#[test]
fn test_pda_determinism() {
    let program_id = pda_program_id();
    let user = Pubkey::new_unique();

    // Derive same PDA multiple times
    let (pda1, bump1) = find_pda(&[b"vault", user.as_ref(), &1u64.to_le_bytes()], &program_id);
    let (pda2, bump2) = find_pda(&[b"vault", user.as_ref(), &1u64.to_le_bytes()], &program_id);
    let (pda3, bump3) = find_pda(&[b"vault", user.as_ref(), &1u64.to_le_bytes()], &program_id);

    // All should be identical
    assert_eq!(pda1, pda2);
    assert_eq!(pda2, pda3);
    assert_eq!(bump1, bump2);
    assert_eq!(bump2, bump3);
}
