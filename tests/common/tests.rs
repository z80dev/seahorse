//! Tests for the behavior comparison framework
//!
//! These tests verify that the comparison framework itself works correctly.
//! They don't require actual Seahorse/Anchor programs - they test the
//! comparison logic and utilities.

use seahorse_test_common::compare::*;
use solana_account::Account;
use solana_pubkey::Pubkey;

#[test]
fn test_instruction_builder_basic() {
    let program_id = Pubkey::new_unique();
    let user = Pubkey::new_unique();

    let builder = InstructionBuilder::new("initialize").with_signer(&user);

    let ix = builder.build(program_id);

    assert_eq!(ix.program_id, program_id);
    assert_eq!(ix.accounts.len(), 1);
    assert!(ix.accounts[0].is_signer);
    assert!(ix.accounts[0].is_writable);
    assert_eq!(ix.data.len(), 8); // Just discriminator
}

#[test]
fn test_instruction_builder_with_data() {
    let program_id = Pubkey::new_unique();

    let builder = InstructionBuilder::new("set_value").with_data(vec![42, 0, 0, 0]); // u32 = 42

    let ix = builder.build(program_id);

    assert_eq!(ix.data.len(), 8 + 4); // Discriminator + u32
    assert_eq!(&ix.data[8..], &[42, 0, 0, 0]);
}

#[test]
fn test_instruction_builder_multiple_accounts() {
    let program_id = Pubkey::new_unique();
    let user = Pubkey::new_unique();
    let counter = Pubkey::new_unique();
    let system = Pubkey::new_unique();

    let builder = InstructionBuilder::new("initialize")
        .with_signer(&user)
        .with_writable(&counter)
        .with_readonly(&system);

    let ix = builder.build(program_id);

    assert_eq!(ix.accounts.len(), 3);
    assert!(ix.accounts[0].is_signer && ix.accounts[0].is_writable);
    assert!(!ix.accounts[1].is_signer && ix.accounts[1].is_writable);
    assert!(!ix.accounts[2].is_signer && !ix.accounts[2].is_writable);
}

#[test]
fn test_compare_accounts_identical() {
    let pubkey = Pubkey::new_unique();
    let owner = Pubkey::new_unique();

    let account = Account {
        lamports: 1000,
        data: vec![1, 2, 3, 4, 5],
        owner,
        executable: false,
        rent_epoch: 0,
    };

    let diff = compare_accounts(pubkey, &account, &account.clone());
    assert!(diff.is_none(), "Identical accounts should have no diff");
}

#[test]
fn test_compare_accounts_lamports_difference() {
    let pubkey = Pubkey::new_unique();
    let owner = Pubkey::new_unique();

    let account1 = Account {
        lamports: 1000,
        data: vec![1, 2, 3],
        owner,
        executable: false,
        rent_epoch: 0,
    };

    let account2 = Account {
        lamports: 2000,
        ..account1.clone()
    };

    let diff = compare_accounts(pubkey, &account1, &account2);
    assert!(diff.is_some());

    match diff.unwrap().diff_type {
        DiffType::LamportsDiff { seahorse, anchor } => {
            assert_eq!(seahorse, 1000);
            assert_eq!(anchor, 2000);
        }
        _ => panic!("Expected LamportsDiff"),
    }
}

#[test]
fn test_compare_accounts_data_difference() {
    let pubkey = Pubkey::new_unique();
    let owner = Pubkey::new_unique();

    let account1 = Account {
        lamports: 1000,
        data: vec![1, 2, 3, 4, 5],
        owner,
        executable: false,
        rent_epoch: 0,
    };

    let account2 = Account {
        data: vec![1, 2, 99, 4, 5], // Byte 2 differs
        ..account1.clone()
    };

    let diff = compare_accounts(pubkey, &account1, &account2);
    assert!(diff.is_some());

    match diff.unwrap().diff_type {
        DiffType::DataDiff { byte_positions, .. } => {
            assert_eq!(byte_positions, vec![2]);
        }
        _ => panic!("Expected DataDiff"),
    }
}

#[test]
fn test_compare_accounts_size_difference() {
    let pubkey = Pubkey::new_unique();
    let owner = Pubkey::new_unique();

    let account1 = Account {
        lamports: 1000,
        data: vec![1, 2, 3],
        owner,
        executable: false,
        rent_epoch: 0,
    };

    let account2 = Account {
        data: vec![1, 2, 3, 4, 5], // Different size
        ..account1.clone()
    };

    let diff = compare_accounts(pubkey, &account1, &account2);
    assert!(diff.is_some());

    match diff.unwrap().diff_type {
        DiffType::DataSizeDiff { seahorse, anchor } => {
            assert_eq!(seahorse, 3);
            assert_eq!(anchor, 5);
        }
        _ => panic!("Expected DataSizeDiff"),
    }
}

#[test]
fn test_compare_accounts_owner_difference() {
    let pubkey = Pubkey::new_unique();

    let account1 = Account {
        lamports: 1000,
        data: vec![1, 2, 3],
        owner: Pubkey::new_unique(),
        executable: false,
        rent_epoch: 0,
    };

    let account2 = Account {
        owner: Pubkey::new_unique(),
        ..account1.clone()
    };

    let diff = compare_accounts(pubkey, &account1, &account2);
    assert!(diff.is_some());

    assert!(matches!(diff.unwrap().diff_type, DiffType::OwnerDiff { .. }));
}

#[test]
fn test_compare_result_equivalent() {
    let result = CompareResult {
        equivalent: true,
        seahorse_outcome: ExecutionOutcome::Success {
            compute_units: 1500,
            logs: vec!["Program log: Hello".to_string()],
        },
        anchor_outcome: ExecutionOutcome::Success {
            compute_units: 1500,
            logs: vec!["Program log: Hello".to_string()],
        },
        account_diffs: vec![],
        summary: String::new(),
    };

    assert!(result.is_equivalent());

    let report = result.report();
    assert!(report.contains("EQUIVALENT"));
    assert!(!report.contains("DIVERGENT"));
}

#[test]
fn test_compare_result_divergent() {
    let result = CompareResult {
        equivalent: false,
        seahorse_outcome: ExecutionOutcome::Success {
            compute_units: 1500,
            logs: vec![],
        },
        anchor_outcome: ExecutionOutcome::Failure {
            error: "Account not initialized".to_string(),
            logs: vec![],
        },
        account_diffs: vec![],
        summary: "Seahorse succeeded but Anchor failed".to_string(),
    };

    assert!(!result.is_equivalent());

    let report = result.report();
    assert!(report.contains("DIVERGENT"));
    assert!(report.contains("Seahorse succeeded but Anchor failed"));
}

#[test]
fn test_execution_outcome_display() {
    let success = ExecutionOutcome::Success {
        compute_units: 2000,
        logs: vec!["Program log: Success".to_string()],
    };

    let display = format!("{}", success);
    assert!(display.contains("Success"));
    assert!(display.contains("2000"));

    let failure = ExecutionOutcome::Failure {
        error: "Custom program error: 0x1".to_string(),
        logs: vec![],
    };

    let display = format!("{}", failure);
    assert!(display.contains("Failure"));
    assert!(display.contains("Custom program error"));
}

#[test]
fn test_discriminator_consistency() {
    // Verify discriminators are deterministic
    let disc1 = instruction_discriminator("initialize");
    let disc2 = instruction_discriminator("initialize");
    assert_eq!(disc1, disc2);

    // Verify different names give different discriminators
    let disc3 = instruction_discriminator("transfer");
    assert_ne!(disc1, disc3);
}

#[test]
fn test_account_discriminator_format() {
    // Verify account discriminator uses "account:" prefix
    let disc = account_discriminator("Counter");

    // Should be 8 bytes
    assert_eq!(disc.len(), 8);

    // Different from instruction discriminator for same name
    let ix_disc = instruction_discriminator("Counter");
    assert_ne!(disc, ix_disc);
}

#[test]
fn test_find_pda() {
    let program_id = Pubkey::new_unique();
    let user = Pubkey::new_unique();

    let (pda1, bump1) = find_pda(&[b"counter", user.as_ref()], &program_id);
    let (pda2, bump2) = find_pda(&[b"counter", user.as_ref()], &program_id);

    // Same seeds should give same PDA
    assert_eq!(pda1, pda2);
    assert_eq!(bump1, bump2);

    // Different seeds should give different PDA
    let (pda3, _) = find_pda(&[b"other", user.as_ref()], &program_id);
    assert_ne!(pda1, pda3);
}

#[test]
fn test_program_account_creation() {
    let owner = Pubkey::new_unique();
    let account = program_account(32, &owner);

    // Should have discriminator + data space
    assert_eq!(account.data.len(), DISCRIMINATOR_SIZE + 32);
    assert_eq!(account.owner, owner);
    assert!(!account.executable);
    // Should be rent exempt
    assert!(account.lamports > 0);
}

#[test]
fn test_account_diff_display() {
    let pubkey = Pubkey::new_unique();

    let diff = AccountDiff {
        pubkey,
        diff_type: DiffType::LamportsDiff {
            seahorse: 1000,
            anchor: 2000,
        },
    };

    let display = format!("{}", diff);
    assert!(display.contains("LAMPORTS"));
    assert!(display.contains("1000"));
    assert!(display.contains("2000"));
}

#[test]
fn test_instruction_builder_clone() {
    let user = Pubkey::new_unique();

    let builder = InstructionBuilder::new("test")
        .with_signer(&user)
        .with_data(vec![1, 2, 3]);

    let cloned = builder.clone();
    let program_id = Pubkey::new_unique();

    let ix1 = builder.build(program_id);
    let ix2 = cloned.build(program_id);

    assert_eq!(ix1.data, ix2.data);
    assert_eq!(ix1.accounts, ix2.accounts);
}
