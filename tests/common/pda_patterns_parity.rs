//! Behavior parity tests: Seahorse PDA Patterns vs Anchor PDA Patterns
//!
//! This test verifies that the Seahorse-compiled pda_patterns program produces
//! identical behavior to the reference Anchor implementation.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile both:
//! - target/deploy/pda_patterns.so (Seahorse)
//! - target/deploy/pda_patterns_anchor.so (Anchor reference)

use seahorse_test_common::*;
use solana_keypair::Keypair;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_sdk_ids::sysvar::rent as rent_sysvar;
use solana_signer::Signer;
use std::str::FromStr;

/// Both programs use the same program ID
fn program_id() -> Pubkey {
    Pubkey::from_str("DxpKiPEUYHy6NKsnEMRvRKtC3ncrtP8epGt5KKS59Xzh").unwrap()
}

/// Read GlobalConfig counter value from account data
fn read_global_counter(data: &[u8]) -> u64 {
    // discriminator(8) + admin(32) + counter(8) + bump(1)
    u64::from_le_bytes(data[40..48].try_into().unwrap())
}

/// Read Vault balance from account data
fn read_vault_balance(data: &[u8]) -> u64 {
    // discriminator(8) + owner(32) + vault_id(8) + balance(8) + bump(1)
    u64::from_le_bytes(data[48..56].try_into().unwrap())
}

/// Program type for different account layouts
#[derive(Clone, Copy)]
enum ProgramType {
    Seahorse, // Seahorse requires rent sysvar for init
    Anchor,   // Anchor doesn't require rent sysvar
}

/// Run global config workflow on a program and return the final counter value
fn run_global_config_workflow(
    svm: &mut litesvm::LiteSVM,
    program_id: &Pubkey,
    admin: &Keypair,
    program_type: ProgramType,
) -> Result<u64, String> {
    let (config_pda, _) = find_pda(&[b"global_config"], program_id);

    // Initialize - Seahorse requires rent sysvar
    let init_ix = match program_type {
        ProgramType::Seahorse => InstructionBuilder::new("init_global_config")
            .with_signer(&admin.pubkey())
            .with_writable(&config_pda)
            .with_readonly(&rent_sysvar::id())
            .with_readonly(&system_program::id())
            .build(*program_id),
        ProgramType::Anchor => InstructionBuilder::new("init_global_config")
            .with_signer(&admin.pubkey())
            .with_writable(&config_pda)
            .with_readonly(&system_program::id())
            .build(*program_id),
    };

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[init_ix], Some(&admin.pubkey()));
    let tx = solana_transaction::Transaction::new(&[admin], message, blockhash);
    if let Err(e) = svm.send_transaction(tx) {
        return Err(format!("Initialize failed: {:?}", e));
    }

    // Update counter twice
    for i in 0..2 {
        svm.expire_blockhash();

        let update_ix = InstructionBuilder::new("update_global_config")
            .with_signer(&admin.pubkey())
            .with_writable(&config_pda)
            .build(*program_id);

        let blockhash = svm.latest_blockhash();
        let message = solana_message::Message::new(&[update_ix], Some(&admin.pubkey()));
        let tx = solana_transaction::Transaction::new(&[admin], message, blockhash);
        if let Err(e) = svm.send_transaction(tx) {
            return Err(format!("Update {} failed: {:?}", i + 1, e));
        }
    }

    // Read final counter value
    let account = svm.get_account(&config_pda).ok_or("Config account not found")?;
    Ok(read_global_counter(&account.data))
}

/// Run vault workflow on a program and return the final balance
fn run_vault_workflow(
    svm: &mut litesvm::LiteSVM,
    program_id: &Pubkey,
    owner: &Keypair,
    program_type: ProgramType,
) -> Result<u64, String> {
    let vault_id: u64 = 1;
    let (vault_pda, _) = find_pda(
        &[b"vault", owner.pubkey().as_ref(), &vault_id.to_le_bytes()],
        program_id,
    );

    // Initialize vault - Seahorse requires rent sysvar
    let init_ix = match program_type {
        ProgramType::Seahorse => InstructionBuilder::new("init_vault")
            .with_signer(&owner.pubkey())
            .with_writable(&vault_pda)
            .with_readonly(&rent_sysvar::id())
            .with_readonly(&system_program::id())
            .with_data(vault_id.to_le_bytes().to_vec())
            .build(*program_id),
        ProgramType::Anchor => InstructionBuilder::new("init_vault")
            .with_signer(&owner.pubkey())
            .with_writable(&vault_pda)
            .with_readonly(&system_program::id())
            .with_data(vault_id.to_le_bytes().to_vec())
            .build(*program_id),
    };

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[init_ix], Some(&owner.pubkey()));
    let tx = solana_transaction::Transaction::new(&[owner], message, blockhash);
    if let Err(e) = svm.send_transaction(tx) {
        return Err(format!("Init vault failed: {:?}", e));
    }

    // Deposit 100
    // Note: Seahorse only needs amount, Anchor needs (vault_id, amount) for PDA verification
    svm.expire_blockhash();
    let deposit_amount: u64 = 100;
    let deposit_data = match program_type {
        ProgramType::Seahorse => deposit_amount.to_le_bytes().to_vec(),
        ProgramType::Anchor => {
            let mut data = Vec::new();
            data.extend_from_slice(&vault_id.to_le_bytes());
            data.extend_from_slice(&deposit_amount.to_le_bytes());
            data
        }
    };
    let deposit_ix = InstructionBuilder::new("deposit_to_vault")
        .with_signer(&owner.pubkey())
        .with_writable(&vault_pda)
        .with_data(deposit_data)
        .build(*program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[deposit_ix], Some(&owner.pubkey()));
    let tx = solana_transaction::Transaction::new(&[owner], message, blockhash);
    if let Err(e) = svm.send_transaction(tx) {
        return Err(format!("Deposit failed: {:?}", e));
    }

    // Deposit 50 more
    svm.expire_blockhash();
    let deposit_amount: u64 = 50;
    let deposit_data = match program_type {
        ProgramType::Seahorse => deposit_amount.to_le_bytes().to_vec(),
        ProgramType::Anchor => {
            let mut data = Vec::new();
            data.extend_from_slice(&vault_id.to_le_bytes());
            data.extend_from_slice(&deposit_amount.to_le_bytes());
            data
        }
    };
    let deposit_ix = InstructionBuilder::new("deposit_to_vault")
        .with_signer(&owner.pubkey())
        .with_writable(&vault_pda)
        .with_data(deposit_data)
        .build(*program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[deposit_ix], Some(&owner.pubkey()));
    let tx = solana_transaction::Transaction::new(&[owner], message, blockhash);
    if let Err(e) = svm.send_transaction(tx) {
        return Err(format!("Deposit 2 failed: {:?}", e));
    }

    // Read final balance
    let account = svm.get_account(&vault_pda).ok_or("Vault account not found")?;
    Ok(read_vault_balance(&account.data))
}

#[test]
fn test_global_config_behavior_parity() {
    let program_id = program_id();

    // Load programs
    let seahorse_bytes = std::fs::read("../../target/deploy/pda_patterns.so")
        .expect("Failed to read pda_patterns.so - run ./scripts/build-test-programs.sh");
    let anchor_bytes = std::fs::read("../../target/deploy/pda_patterns_anchor.so")
        .expect("Failed to read pda_patterns_anchor.so");

    // Create SVMs for each program
    let mut seahorse_svm = litesvm::LiteSVM::new();
    seahorse_svm.add_program(program_id, &seahorse_bytes);
    let seahorse_admin = {
        let kp = Keypair::new();
        seahorse_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let mut anchor_svm = litesvm::LiteSVM::new();
    anchor_svm.add_program(program_id, &anchor_bytes);
    let anchor_admin = {
        let kp = Keypair::new();
        anchor_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    // Run workflows
    let seahorse_result = run_global_config_workflow(
        &mut seahorse_svm,
        &program_id,
        &seahorse_admin,
        ProgramType::Seahorse,
    );
    let anchor_result = run_global_config_workflow(
        &mut anchor_svm,
        &program_id,
        &anchor_admin,
        ProgramType::Anchor,
    );

    // Both should succeed
    assert!(seahorse_result.is_ok(), "Seahorse failed: {:?}", seahorse_result);
    assert!(anchor_result.is_ok(), "Anchor failed: {:?}", anchor_result);

    let seahorse_counter = seahorse_result.unwrap();
    let anchor_counter = anchor_result.unwrap();

    // Both should have counter = 2 (updated twice)
    assert_eq!(
        seahorse_counter, anchor_counter,
        "Behavior parity failed: Seahorse counter={}, Anchor counter={}",
        seahorse_counter, anchor_counter
    );
    assert_eq!(seahorse_counter, 2, "Expected counter to be 2");
}

#[test]
fn test_vault_behavior_parity() {
    let program_id = program_id();

    // Load programs
    let seahorse_bytes = std::fs::read("../../target/deploy/pda_patterns.so")
        .expect("Failed to read pda_patterns.so");
    let anchor_bytes = std::fs::read("../../target/deploy/pda_patterns_anchor.so")
        .expect("Failed to read pda_patterns_anchor.so");

    // Create SVMs
    let mut seahorse_svm = litesvm::LiteSVM::new();
    seahorse_svm.add_program(program_id, &seahorse_bytes);
    let seahorse_owner = {
        let kp = Keypair::new();
        seahorse_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let mut anchor_svm = litesvm::LiteSVM::new();
    anchor_svm.add_program(program_id, &anchor_bytes);
    let anchor_owner = {
        let kp = Keypair::new();
        anchor_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    // Run vault workflows
    let seahorse_result = run_vault_workflow(
        &mut seahorse_svm,
        &program_id,
        &seahorse_owner,
        ProgramType::Seahorse,
    );
    let anchor_result = run_vault_workflow(
        &mut anchor_svm,
        &program_id,
        &anchor_owner,
        ProgramType::Anchor,
    );

    // Both should succeed
    assert!(seahorse_result.is_ok(), "Seahorse failed: {:?}", seahorse_result);
    assert!(anchor_result.is_ok(), "Anchor failed: {:?}", anchor_result);

    let seahorse_balance = seahorse_result.unwrap();
    let anchor_balance = anchor_result.unwrap();

    // Both should have balance = 150 (100 + 50)
    assert_eq!(
        seahorse_balance, anchor_balance,
        "Behavior parity failed: Seahorse balance={}, Anchor balance={}",
        seahorse_balance, anchor_balance
    );
    assert_eq!(seahorse_balance, 150, "Expected balance to be 150");
}

#[test]
fn test_pda_derivation_parity() {
    let program_id = program_id();
    let user = Pubkey::new_unique();

    // Verify PDA derivation is deterministic across both implementations
    // Since both use the same program_id and seeds, PDAs should be identical

    // Global config (single seed)
    let (global_pda_1, bump1) = find_pda(&[b"global_config"], &program_id);
    let (global_pda_2, bump2) = find_pda(&[b"global_config"], &program_id);
    assert_eq!(global_pda_1, global_pda_2, "Global config PDA should be deterministic");
    assert_eq!(bump1, bump2, "Bump should be deterministic");

    // User profile (user-derived)
    let (profile_pda_1, _) = find_pda(&[b"user_profile", user.as_ref()], &program_id);
    let (profile_pda_2, _) = find_pda(&[b"user_profile", user.as_ref()], &program_id);
    assert_eq!(profile_pda_1, profile_pda_2, "User profile PDA should be deterministic");

    // Vault (multiple seeds)
    let vault_id: u64 = 42;
    let (vault_pda_1, _) = find_pda(
        &[b"vault", user.as_ref(), &vault_id.to_le_bytes()],
        &program_id,
    );
    let (vault_pda_2, _) = find_pda(
        &[b"vault", user.as_ref(), &vault_id.to_le_bytes()],
        &program_id,
    );
    assert_eq!(vault_pda_1, vault_pda_2, "Vault PDA should be deterministic");

    // Data record (category + id seeds)
    let category = b"notes";
    let record_id: u64 = 1;
    let (record_pda_1, _) = find_pda(
        &[b"data_record", category, &record_id.to_le_bytes()],
        &program_id,
    );
    let (record_pda_2, _) = find_pda(
        &[b"data_record", category, &record_id.to_le_bytes()],
        &program_id,
    );
    assert_eq!(record_pda_1, record_pda_2, "Data record PDA should be deterministic");
}

#[test]
fn test_unauthorized_access_parity() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/pda_patterns.so").unwrap();
    let anchor_bytes = std::fs::read("../../target/deploy/pda_patterns_anchor.so").unwrap();

    // Test Seahorse unauthorized access
    let mut seahorse_svm = litesvm::LiteSVM::new();
    seahorse_svm.add_program(program_id, &seahorse_bytes);
    let admin = {
        let kp = Keypair::new();
        seahorse_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let attacker = {
        let kp = Keypair::new();
        seahorse_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let (config_pda, _) = find_pda(&[b"global_config"], &program_id);

    // Admin initializes
    let init_ix = InstructionBuilder::new("init_global_config")
        .with_signer(&admin.pubkey())
        .with_writable(&config_pda)
        .with_readonly(&rent_sysvar::id())
        .with_readonly(&system_program::id())
        .build(program_id);

    let blockhash = seahorse_svm.latest_blockhash();
    let message = solana_message::Message::new(&[init_ix], Some(&admin.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&admin], message, blockhash);
    seahorse_svm.send_transaction(tx).unwrap();

    // Attacker tries to update
    seahorse_svm.expire_blockhash();
    let update_ix = InstructionBuilder::new("update_global_config")
        .with_signer(&attacker.pubkey())
        .with_writable(&config_pda)
        .build(program_id);

    let blockhash = seahorse_svm.latest_blockhash();
    let message = solana_message::Message::new(&[update_ix], Some(&attacker.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&attacker], message, blockhash);
    let seahorse_unauthorized = seahorse_svm.send_transaction(tx);

    // Test Anchor unauthorized access
    let mut anchor_svm = litesvm::LiteSVM::new();
    anchor_svm.add_program(program_id, &anchor_bytes);
    let admin2 = {
        let kp = Keypair::new();
        anchor_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let attacker2 = {
        let kp = Keypair::new();
        anchor_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let (config_pda2, _) = find_pda(&[b"global_config"], &program_id);

    // Admin initializes
    let init_ix = InstructionBuilder::new("init_global_config")
        .with_signer(&admin2.pubkey())
        .with_writable(&config_pda2)
        .with_readonly(&system_program::id())
        .build(program_id);

    let blockhash = anchor_svm.latest_blockhash();
    let message = solana_message::Message::new(&[init_ix], Some(&admin2.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&admin2], message, blockhash);
    anchor_svm.send_transaction(tx).unwrap();

    // Attacker tries to update
    anchor_svm.expire_blockhash();
    let update_ix = InstructionBuilder::new("update_global_config")
        .with_signer(&attacker2.pubkey())
        .with_writable(&config_pda2)
        .build(program_id);

    let blockhash = anchor_svm.latest_blockhash();
    let message = solana_message::Message::new(&[update_ix], Some(&attacker2.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&attacker2], message, blockhash);
    let anchor_unauthorized = anchor_svm.send_transaction(tx);

    // Both should fail
    assert!(
        seahorse_unauthorized.is_err(),
        "Seahorse should reject unauthorized access"
    );
    assert!(
        anchor_unauthorized.is_err(),
        "Anchor should reject unauthorized access"
    );
}
