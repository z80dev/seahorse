//! Multisig tests: Seahorse implementation behavior verification
//!
//! These tests verify the Seahorse multisig wallet implementation produces
//! consistent behavior for:
//! - Multisig initialization with correct state
//! - Transaction proposal by owners
//! - Approval counting and threshold enforcement
//! - Double-approval prevention via approval record PDA
//! - Transaction execution after threshold is met
//! - Deterministic PDA derivation
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile:
//! - target/deploy/multisig.so (Seahorse)

use seahorse_test_common::*;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_sha256_hasher::hash;
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::str::FromStr;

/// Multisig program ID
fn program_id() -> Pubkey {
    Pubkey::from_str("5h7CmLSs5q2ZfWFns5yUjN6BTYVxVxTNddJ2X1yHnWCd").unwrap()
}

/// Derive multisig PDA
fn derive_multisig_pda(multisig_id: u64, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"multisig", multisig_id.to_le_bytes().as_ref()],
        program_id,
    )
}

/// Derive transaction PDA
fn derive_transaction_pda(multisig_id: u64, tx_id: u64, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[
            b"transaction",
            multisig_id.to_le_bytes().as_ref(),
            tx_id.to_le_bytes().as_ref(),
        ],
        program_id,
    )
}

/// Derive approval PDA
fn derive_approval_pda(
    multisig_id: u64,
    tx_id: u64,
    approver: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    find_pda(
        &[
            b"approval",
            multisig_id.to_le_bytes().as_ref(),
            tx_id.to_le_bytes().as_ref(),
            approver.as_ref(),
        ],
        program_id,
    )
}

/// Compute instruction discriminator (first 8 bytes of SHA256 of "global:<name>")
fn instruction_disc(name: &str) -> [u8; 8] {
    let preimage = format!("global:{}", name);
    let hash_result = hash(preimage.as_bytes());
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash_result.as_ref()[..8]);
    disc
}

/// Build create_multisig instruction data
fn create_multisig_data(
    multisig_id: u64,
    threshold: u8,
    owner_count: u8,
    owner_1: Pubkey,
    owner_2: Pubkey,
    owner_3: Pubkey,
) -> Vec<u8> {
    let mut data = instruction_disc("create_multisig").to_vec();
    data.extend_from_slice(&multisig_id.to_le_bytes());
    data.push(threshold);
    data.push(owner_count);
    data.extend_from_slice(owner_1.as_ref());
    data.extend_from_slice(owner_2.as_ref());
    data.extend_from_slice(owner_3.as_ref());
    data
}

/// Build propose_transaction instruction data
fn propose_transaction_data(tx_id: u64, recipient: Pubkey, amount: u64) -> Vec<u8> {
    let mut data = instruction_disc("propose_transaction").to_vec();
    data.extend_from_slice(&tx_id.to_le_bytes());
    data.extend_from_slice(recipient.as_ref());
    data.extend_from_slice(&amount.to_le_bytes());
    data
}

/// Build approve instruction data
fn approve_data(tx_id: u64) -> Vec<u8> {
    let mut data = instruction_disc("approve").to_vec();
    data.extend_from_slice(&tx_id.to_le_bytes());
    data
}

/// Build execute instruction data
fn execute_data(tx_id: u64) -> Vec<u8> {
    let mut data = instruction_disc("execute").to_vec();
    data.extend_from_slice(&tx_id.to_le_bytes());
    data
}

/// Read multisig fields from account data
fn read_multisig_threshold(data: &[u8]) -> u8 {
    data[16]
}

fn read_multisig_owner_count(data: &[u8]) -> u8 {
    data[17]
}

fn read_multisig_next_tx_id(data: &[u8]) -> u64 {
    // After 10 owners: 18 + 10*32 = 338
    let bytes: [u8; 8] = data[338..346].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

/// Read transaction fields from account data
fn read_transaction_approval_count(data: &[u8]) -> u8 {
    data[120]
}

fn read_transaction_is_executed(data: &[u8]) -> bool {
    data[121] != 0
}

/// Build signer account meta
fn signer_meta(pubkey: Pubkey) -> AccountMeta {
    AccountMeta::new(pubkey, true)
}

/// Build writable account meta
fn writable_meta(pubkey: Pubkey) -> AccountMeta {
    AccountMeta::new(pubkey, false)
}

/// Build readonly account meta
fn readonly_meta(pubkey: Pubkey) -> AccountMeta {
    AccountMeta::new_readonly(pubkey, false)
}

/// Execute transaction
fn execute_tx(
    svm: &mut litesvm::LiteSVM,
    ix: Instruction,
    payer: &Keypair,
    signers: &[&Keypair],
) -> Result<(), String> {
    let blockhash = svm.latest_blockhash();
    let message = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let tx = Transaction::new(signers, message, blockhash);
    svm.send_transaction(tx)
        .map(|_| ())
        .map_err(|e| format!("{:?}", e))
}

// =============================================================================
// PARITY TESTS
// =============================================================================

#[test]
fn test_parity_multisig_initialization() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/multisig.so")
        .expect("Failed to read multisig.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup creator and owners
    let creator = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let owner_1 = Keypair::new();
    let owner_2 = Keypair::new();
    let owner_3 = Keypair::new();

    let multisig_id: u64 = 1;
    let threshold: u8 = 2;
    let owner_count: u8 = 3;

    let (multisig_pda, _) = derive_multisig_pda(multisig_id, &program_id);

    // Create multisig
    let ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(creator.pubkey()),
            writable_meta(multisig_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: create_multisig_data(
            multisig_id,
            threshold,
            owner_count,
            owner_1.pubkey(),
            owner_2.pubkey(),
            owner_3.pubkey(),
        ),
    };

    let result = execute_tx(&mut svm, ix, &creator, &[&creator]);
    assert!(result.is_ok(), "create_multisig should succeed: {:?}", result);

    // Verify initial state
    let multisig_account = svm.get_account(&multisig_pda).unwrap();

    assert_eq!(
        read_multisig_threshold(&multisig_account.data),
        threshold,
        "threshold should match"
    );
    assert_eq!(
        read_multisig_owner_count(&multisig_account.data),
        owner_count,
        "owner_count should match"
    );
    assert_eq!(
        read_multisig_next_tx_id(&multisig_account.data),
        1,
        "next_tx_id should be 1"
    );
}

#[test]
fn test_parity_approval_counting() {
    let program_id = program_id();

    let seahorse_bytes =
        std::fs::read("../../target/deploy/multisig.so").expect("Failed to read multisig.so");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup
    let creator = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let owner_1 = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let owner_2 = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let owner_3 = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let multisig_id: u64 = 1;
    let (multisig_pda, _) = derive_multisig_pda(multisig_id, &program_id);

    // Create 2-of-3 multisig
    let create_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(creator.pubkey()),
            writable_meta(multisig_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: create_multisig_data(
            multisig_id,
            2,
            3,
            owner_1.pubkey(),
            owner_2.pubkey(),
            owner_3.pubkey(),
        ),
    };
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // Propose transaction
    let tx_id: u64 = 1;
    let (transaction_pda, _) = derive_transaction_pda(multisig_id, tx_id, &program_id);
    let recipient = Pubkey::new_unique();

    svm.expire_blockhash();
    let propose_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(owner_1.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: propose_transaction_data(tx_id, recipient, 1_000_000_000),
    };
    execute_tx(&mut svm, propose_ix, &owner_1, &[&owner_1]).unwrap();

    // First approval
    let (approval_pda_1, _) = derive_approval_pda(multisig_id, tx_id, &owner_1.pubkey(), &program_id);
    svm.expire_blockhash();
    let approve_ix_1 = Instruction {
        program_id,
        accounts: vec![
            signer_meta(owner_1.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(approval_pda_1),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: approve_data(tx_id),
    };
    execute_tx(&mut svm, approve_ix_1, &owner_1, &[&owner_1]).unwrap();

    // Verify 1 approval
    let tx_account = svm.get_account(&transaction_pda).unwrap();
    assert_eq!(
        read_transaction_approval_count(&tx_account.data),
        1,
        "approval_count should be 1"
    );

    // Second approval
    let (approval_pda_2, _) = derive_approval_pda(multisig_id, tx_id, &owner_2.pubkey(), &program_id);
    svm.expire_blockhash();
    let approve_ix_2 = Instruction {
        program_id,
        accounts: vec![
            signer_meta(owner_2.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(approval_pda_2),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: approve_data(tx_id),
    };
    execute_tx(&mut svm, approve_ix_2, &owner_2, &[&owner_2]).unwrap();

    // Verify 2 approvals
    let tx_account = svm.get_account(&transaction_pda).unwrap();
    assert_eq!(
        read_transaction_approval_count(&tx_account.data),
        2,
        "approval_count should be 2"
    );
}

#[test]
fn test_parity_double_approval_prevention() {
    let program_id = program_id();

    let seahorse_bytes =
        std::fs::read("../../target/deploy/multisig.so").expect("Failed to read multisig.so");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup
    let creator = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let owner = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let multisig_id: u64 = 1;
    let (multisig_pda, _) = derive_multisig_pda(multisig_id, &program_id);

    // Create 1-of-1 multisig
    let create_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(creator.pubkey()),
            writable_meta(multisig_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: create_multisig_data(
            multisig_id,
            2, // Need 2 approvals to test double-approval
            2,
            owner.pubkey(),
            owner.pubkey(),
            owner.pubkey(),
        ),
    };
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // Propose transaction
    let tx_id: u64 = 1;
    let (transaction_pda, _) = derive_transaction_pda(multisig_id, tx_id, &program_id);
    let recipient = Pubkey::new_unique();

    svm.expire_blockhash();
    let propose_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(owner.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: propose_transaction_data(tx_id, recipient, 1_000_000_000),
    };
    execute_tx(&mut svm, propose_ix, &owner, &[&owner]).unwrap();

    // First approval
    let (approval_pda, _) = derive_approval_pda(multisig_id, tx_id, &owner.pubkey(), &program_id);
    svm.expire_blockhash();
    let approve_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(owner.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(approval_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: approve_data(tx_id),
    };
    let first_approval = execute_tx(&mut svm, approve_ix, &owner, &[&owner]);
    assert!(first_approval.is_ok(), "first approval should succeed");

    // Second approval from same owner should fail (PDA already exists)
    svm.expire_blockhash();
    let approve_ix_2 = Instruction {
        program_id,
        accounts: vec![
            signer_meta(owner.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(approval_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: approve_data(tx_id),
    };
    let second_approval = execute_tx(&mut svm, approve_ix_2, &owner, &[&owner]);
    assert!(
        second_approval.is_err(),
        "second approval from same owner should fail"
    );

    // Approval count should remain 1
    let tx_account = svm.get_account(&transaction_pda).unwrap();
    assert_eq!(
        read_transaction_approval_count(&tx_account.data),
        1,
        "approval_count should still be 1"
    );
}

#[test]
fn test_parity_threshold_enforcement() {
    let program_id = program_id();

    let seahorse_bytes =
        std::fs::read("../../target/deploy/multisig.so").expect("Failed to read multisig.so");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup
    let creator = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let owner_1 = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let owner_2 = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let owner_3 = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let recipient = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 1 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let multisig_id: u64 = 1;
    let (multisig_pda, _) = derive_multisig_pda(multisig_id, &program_id);

    // Create 2-of-3 multisig
    let create_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(creator.pubkey()),
            writable_meta(multisig_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: create_multisig_data(
            multisig_id,
            2,
            3,
            owner_1.pubkey(),
            owner_2.pubkey(),
            owner_3.pubkey(),
        ),
    };
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // Fund multisig
    let multisig_account = svm.get_account(&multisig_pda).unwrap();
    let mut funded_account = multisig_account.clone();
    funded_account.lamports += 5_000_000_000;
    svm.set_account(multisig_pda, funded_account).unwrap();

    // Propose transaction
    let tx_id: u64 = 1;
    let transfer_amount: u64 = 1_000_000_000;
    let (transaction_pda, _) = derive_transaction_pda(multisig_id, tx_id, &program_id);

    svm.expire_blockhash();
    let propose_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(owner_1.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: propose_transaction_data(tx_id, recipient.pubkey(), transfer_amount),
    };
    execute_tx(&mut svm, propose_ix, &owner_1, &[&owner_1]).unwrap();

    // Only 1 approval
    let (approval_pda_1, _) = derive_approval_pda(multisig_id, tx_id, &owner_1.pubkey(), &program_id);
    svm.expire_blockhash();
    let approve_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(owner_1.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(approval_pda_1),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: approve_data(tx_id),
    };
    execute_tx(&mut svm, approve_ix, &owner_1, &[&owner_1]).unwrap();

    // Try to execute with only 1 approval (need 2) - should fail
    svm.expire_blockhash();
    let execute_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(owner_1.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(recipient.pubkey()),
        ],
        data: execute_data(tx_id),
    };
    let early_execute = execute_tx(&mut svm, execute_ix, &owner_1, &[&owner_1]);
    assert!(
        early_execute.is_err(),
        "execute with insufficient approvals should fail"
    );

    // Verify not executed
    let tx_account = svm.get_account(&transaction_pda).unwrap();
    assert_eq!(
        read_transaction_is_executed(&tx_account.data),
        false,
        "should not be executed"
    );

    // Second approval
    let (approval_pda_2, _) = derive_approval_pda(multisig_id, tx_id, &owner_2.pubkey(), &program_id);
    svm.expire_blockhash();
    let approve_ix_2 = Instruction {
        program_id,
        accounts: vec![
            signer_meta(owner_2.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(approval_pda_2),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: approve_data(tx_id),
    };
    execute_tx(&mut svm, approve_ix_2, &owner_2, &[&owner_2]).unwrap();

    // Now execute should succeed
    let recipient_balance_before = svm.get_account(&recipient.pubkey()).unwrap().lamports;

    svm.expire_blockhash();
    let execute_ix_2 = Instruction {
        program_id,
        accounts: vec![
            signer_meta(owner_1.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(recipient.pubkey()),
        ],
        data: execute_data(tx_id),
    };
    let execute_result = execute_tx(&mut svm, execute_ix_2, &owner_1, &[&owner_1]);
    assert!(execute_result.is_ok(), "execute should succeed after threshold met");

    // Verify executed
    let tx_account = svm.get_account(&transaction_pda).unwrap();
    assert_eq!(
        read_transaction_is_executed(&tx_account.data),
        true,
        "should be executed"
    );

    // Verify transfer
    let recipient_balance_after = svm.get_account(&recipient.pubkey()).unwrap().lamports;
    assert_eq!(
        recipient_balance_after,
        recipient_balance_before + transfer_amount,
        "recipient should receive funds"
    );
}

#[test]
fn test_parity_deterministic_pda_derivation() {
    let program_id = program_id();

    // Test that PDA derivation is deterministic across multiple calls
    let multisig_id: u64 = 42;
    let tx_id: u64 = 99;
    let approver = Pubkey::new_unique();

    // Derive multiple times
    let (multisig_pda1, bump1) = derive_multisig_pda(multisig_id, &program_id);
    let (multisig_pda2, bump2) = derive_multisig_pda(multisig_id, &program_id);

    assert_eq!(multisig_pda1, multisig_pda2, "multisig PDA should be deterministic");
    assert_eq!(bump1, bump2, "multisig bump should be deterministic");

    let (tx_pda1, tbump1) = derive_transaction_pda(multisig_id, tx_id, &program_id);
    let (tx_pda2, tbump2) = derive_transaction_pda(multisig_id, tx_id, &program_id);

    assert_eq!(tx_pda1, tx_pda2, "transaction PDA should be deterministic");
    assert_eq!(tbump1, tbump2, "transaction bump should be deterministic");

    let (approval_pda1, abump1) = derive_approval_pda(multisig_id, tx_id, &approver, &program_id);
    let (approval_pda2, abump2) = derive_approval_pda(multisig_id, tx_id, &approver, &program_id);

    assert_eq!(approval_pda1, approval_pda2, "approval PDA should be deterministic");
    assert_eq!(abump1, abump2, "approval bump should be deterministic");

    // Different inputs should produce different PDAs
    let (other_multisig_pda, _) = derive_multisig_pda(123, &program_id);
    assert_ne!(
        multisig_pda1, other_multisig_pda,
        "different multisig_id should produce different PDA"
    );

    let (other_tx_pda, _) = derive_transaction_pda(multisig_id, 456, &program_id);
    assert_ne!(tx_pda1, other_tx_pda, "different tx_id should produce different PDA");

    let other_approver = Pubkey::new_unique();
    let (other_approval_pda, _) =
        derive_approval_pda(multisig_id, tx_id, &other_approver, &program_id);
    assert_ne!(
        approval_pda1, other_approval_pda,
        "different approver should produce different PDA"
    );
}
