//! LiteSVM integration tests for the Seahorse Multisig wallet program
//!
//! These tests verify multisig wallet mechanics:
//! - create_multisig: Creates a multisig wallet with N owners and threshold M
//! - propose_transaction: Owner proposes a SOL transfer transaction
//! - approve: Owner approves a proposed transaction
//! - execute: Executes transaction once threshold is met
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile multisig.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// Multisig program ID (from declare_id!)
fn multisig_program_id() -> Pubkey {
    Pubkey::from_str("5h7CmLSs5q2ZfWFns5yUjN6BTYVxVxTNddJ2X1yHnWCd").unwrap()
}

/// Multisig account size: discriminator (8) + multisig_id (8) + threshold (1) + owner_count (1)
///                        + 10 * owner pubkeys (10 * 32) + next_tx_id (8) + bump (1)
const MULTISIG_SIZE: usize = 8 + 8 + 1 + 1 + (10 * 32) + 8 + 1;

/// Transaction account size: discriminator (8) + multisig (32) + tx_id (8) + proposer (32)
///                           + recipient (32) + amount (8) + approval_count (1) + is_executed (1) + bump (1)
const TRANSACTION_SIZE: usize = 8 + 32 + 8 + 32 + 32 + 8 + 1 + 1 + 1;

/// Approval account size: discriminator (8) + approver (32) + transaction (32) + approved_at_slot (8) + bump (1)
const APPROVAL_SIZE: usize = 8 + 32 + 32 + 8 + 1;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the multisig program into LiteSVM
fn load_multisig_program() -> litesvm::LiteSVM {
    let program_id = multisig_program_id();
    let program_bytes = std::fs::read("../../target/deploy/multisig.so")
        .expect("Failed to read multisig.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);
    svm
}

/// Derive multisig PDA
fn derive_multisig_pda(multisig_id: u64) -> (Pubkey, u8) {
    find_pda(
        &[b"multisig", multisig_id.to_le_bytes().as_ref()],
        &multisig_program_id(),
    )
}

/// Derive transaction PDA
fn derive_transaction_pda(multisig_id: u64, tx_id: u64) -> (Pubkey, u8) {
    find_pda(
        &[
            b"transaction",
            multisig_id.to_le_bytes().as_ref(),
            tx_id.to_le_bytes().as_ref(),
        ],
        &multisig_program_id(),
    )
}

/// Derive approval PDA
fn derive_approval_pda(multisig_id: u64, tx_id: u64, approver: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[
            b"approval",
            multisig_id.to_le_bytes().as_ref(),
            tx_id.to_le_bytes().as_ref(),
            approver.as_ref(),
        ],
        &multisig_program_id(),
    )
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
    let mut data = Vec::new();
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
    let mut data = Vec::new();
    data.extend_from_slice(&tx_id.to_le_bytes());
    data.extend_from_slice(recipient.as_ref());
    data.extend_from_slice(&amount.to_le_bytes());
    data
}

/// Build approve instruction data
fn approve_data(tx_id: u64) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&tx_id.to_le_bytes());
    data
}

/// Build execute instruction data
fn execute_data(tx_id: u64) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&tx_id.to_le_bytes());
    data
}

/// Read multisig fields from account data
fn read_multisig_id(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[8..16].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_multisig_threshold(data: &[u8]) -> u8 {
    data[16]
}

fn read_multisig_owner_count(data: &[u8]) -> u8 {
    data[17]
}

fn read_multisig_owner_1(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[18..50].try_into().unwrap())
}

fn read_multisig_owner_2(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[50..82].try_into().unwrap())
}

fn read_multisig_owner_3(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[82..114].try_into().unwrap())
}

fn read_multisig_next_tx_id(data: &[u8]) -> u64 {
    // After 10 owners: 18 + 10*32 = 338
    let bytes: [u8; 8] = data[338..346].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

/// Read transaction fields from account data
fn read_transaction_multisig(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

fn read_transaction_tx_id(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[40..48].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_transaction_proposer(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[48..80].try_into().unwrap())
}

fn read_transaction_recipient(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[80..112].try_into().unwrap())
}

fn read_transaction_amount(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[112..120].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_transaction_approval_count(data: &[u8]) -> u8 {
    data[120]
}

fn read_transaction_is_executed(data: &[u8]) -> bool {
    data[121] != 0
}

/// Read approval fields from account data
fn read_approval_approver(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

fn read_approval_transaction(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[40..72].try_into().unwrap())
}

fn read_approval_slot(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[72..80].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

// =============================================================================
// CREATE MULTISIG TESTS
// =============================================================================

#[test]
fn test_create_multisig() {
    let mut svm = load_multisig_program();
    let program_id = multisig_program_id();

    // Setup creators/owners
    let creator = funded_keypair_10_sol(&mut svm);
    let owner_1 = funded_keypair_10_sol(&mut svm);
    let owner_2 = funded_keypair_10_sol(&mut svm);
    let owner_3 = funded_keypair_10_sol(&mut svm);

    // Multisig parameters: 2-of-3
    let multisig_id: u64 = 1;
    let threshold: u8 = 2;
    let owner_count: u8 = 3;

    // Derive PDA
    let (multisig_pda, _) = derive_multisig_pda(multisig_id);

    // Build create_multisig instruction
    let ix = anchor_instruction(
        program_id,
        "create_multisig",
        &create_multisig_data(
            multisig_id,
            threshold,
            owner_count,
            owner_1.pubkey(),
            owner_2.pubkey(),
            owner_3.pubkey(),
        ),
        vec![
            signer_meta(creator.pubkey()),       // creator
            writable_meta(multisig_pda),         // multisig
            readonly_meta(solana_sdk_ids::sysvar::rent::id()), // rent
            readonly_meta(system_program::id()), // system_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &creator, &[&creator]);
    assert!(result.is_ok(), "create_multisig should succeed: {:?}", result);

    // Verify multisig state
    let multisig_account = svm.get_account(&multisig_pda).expect("Multisig should exist");
    assert_eq!(multisig_account.owner, program_id);

    assert_eq!(read_multisig_id(&multisig_account.data), multisig_id);
    assert_eq!(read_multisig_threshold(&multisig_account.data), threshold);
    assert_eq!(read_multisig_owner_count(&multisig_account.data), owner_count);
    assert_eq!(read_multisig_owner_1(&multisig_account.data), owner_1.pubkey());
    assert_eq!(read_multisig_owner_2(&multisig_account.data), owner_2.pubkey());
    assert_eq!(read_multisig_owner_3(&multisig_account.data), owner_3.pubkey());
    assert_eq!(read_multisig_next_tx_id(&multisig_account.data), 1);
}

#[test]
fn test_create_multisig_different_ids() {
    let mut svm = load_multisig_program();
    let program_id = multisig_program_id();

    let creator = funded_keypair_10_sol(&mut svm);
    let owner = funded_keypair_10_sol(&mut svm);

    // Create multiple multisigs
    for multisig_id in 1..=3 {
        let (multisig_pda, _) = derive_multisig_pda(multisig_id);

        let ix = anchor_instruction(
            program_id,
            "create_multisig",
            &create_multisig_data(multisig_id, 1, 1, owner.pubkey(), owner.pubkey(), owner.pubkey()),
            vec![
                signer_meta(creator.pubkey()),
                writable_meta(multisig_pda),
                readonly_meta(solana_sdk_ids::sysvar::rent::id()),
                readonly_meta(system_program::id()),
            ],
        );

        svm.expire_blockhash();
        let result = execute_tx(&mut svm, ix, &creator, &[&creator]);
        assert!(result.is_ok(), "create_multisig {} should succeed", multisig_id);

        let multisig_account = svm.get_account(&multisig_pda).expect("Multisig should exist");
        assert_eq!(read_multisig_id(&multisig_account.data), multisig_id);
    }
}

#[test]
fn test_create_multisig_invalid_threshold_fails() {
    let mut svm = load_multisig_program();
    let program_id = multisig_program_id();

    let creator = funded_keypair_10_sol(&mut svm);
    let owner = funded_keypair_10_sol(&mut svm);
    let multisig_id: u64 = 1;
    let (multisig_pda, _) = derive_multisig_pda(multisig_id);

    // threshold = 0 should fail
    let ix = anchor_instruction(
        program_id,
        "create_multisig",
        &create_multisig_data(multisig_id, 0, 1, owner.pubkey(), owner.pubkey(), owner.pubkey()),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(multisig_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &creator, &[&creator]);
    assert!(result.is_err(), "create_multisig with threshold=0 should fail");
}

#[test]
fn test_create_multisig_threshold_exceeds_owners_fails() {
    let mut svm = load_multisig_program();
    let program_id = multisig_program_id();

    let creator = funded_keypair_10_sol(&mut svm);
    let owner = funded_keypair_10_sol(&mut svm);
    let multisig_id: u64 = 1;
    let (multisig_pda, _) = derive_multisig_pda(multisig_id);

    // threshold = 3, owner_count = 2 should fail
    let ix = anchor_instruction(
        program_id,
        "create_multisig",
        &create_multisig_data(multisig_id, 3, 2, owner.pubkey(), owner.pubkey(), owner.pubkey()),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(multisig_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &creator, &[&creator]);
    assert!(result.is_err(), "create_multisig with threshold > owners should fail");
}

// =============================================================================
// PROPOSE TRANSACTION TESTS
// =============================================================================

#[test]
fn test_propose_transaction() {
    let mut svm = load_multisig_program();
    let program_id = multisig_program_id();

    // Setup multisig
    let creator = funded_keypair_10_sol(&mut svm);
    let owner_1 = funded_keypair_10_sol(&mut svm);
    let owner_2 = funded_keypair_10_sol(&mut svm);
    let multisig_id: u64 = 1;
    let (multisig_pda, _) = derive_multisig_pda(multisig_id);

    let create_ix = anchor_instruction(
        program_id,
        "create_multisig",
        &create_multisig_data(multisig_id, 2, 2, owner_1.pubkey(), owner_2.pubkey(), owner_2.pubkey()),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(multisig_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // Propose transaction
    let tx_id: u64 = 1;
    let recipient = Pubkey::new_unique();
    let amount: u64 = 1_000_000_000; // 1 SOL

    let (transaction_pda, _) = derive_transaction_pda(multisig_id, tx_id);

    svm.expire_blockhash();
    let propose_ix = anchor_instruction(
        program_id,
        "propose_transaction",
        &propose_transaction_data(tx_id, recipient, amount),
        vec![
            signer_meta(owner_1.pubkey()),       // proposer
            writable_meta(multisig_pda),         // multisig
            writable_meta(transaction_pda),      // transaction
            readonly_meta(solana_sdk_ids::sysvar::rent::id()), // rent
            readonly_meta(system_program::id()), // system_program
        ],
    );

    let result = execute_tx(&mut svm, propose_ix, &owner_1, &[&owner_1]);
    assert!(result.is_ok(), "propose_transaction should succeed: {:?}", result);

    // Verify transaction state
    let tx_account = svm.get_account(&transaction_pda).expect("Transaction should exist");
    assert_eq!(read_transaction_multisig(&tx_account.data), multisig_pda);
    assert_eq!(read_transaction_tx_id(&tx_account.data), tx_id);
    assert_eq!(read_transaction_proposer(&tx_account.data), owner_1.pubkey());
    assert_eq!(read_transaction_recipient(&tx_account.data), recipient);
    assert_eq!(read_transaction_amount(&tx_account.data), amount);
    assert_eq!(read_transaction_approval_count(&tx_account.data), 0);
    assert_eq!(read_transaction_is_executed(&tx_account.data), false);

    // Verify next_tx_id incremented
    let multisig_account = svm.get_account(&multisig_pda).unwrap();
    assert_eq!(read_multisig_next_tx_id(&multisig_account.data), 2);
}

#[test]
fn test_propose_transaction_non_owner_fails() {
    let mut svm = load_multisig_program();
    let program_id = multisig_program_id();

    // Setup multisig
    let creator = funded_keypair_10_sol(&mut svm);
    let owner_1 = funded_keypair_10_sol(&mut svm);
    let owner_2 = funded_keypair_10_sol(&mut svm);
    let non_owner = funded_keypair_10_sol(&mut svm);
    let multisig_id: u64 = 1;
    let (multisig_pda, _) = derive_multisig_pda(multisig_id);

    let create_ix = anchor_instruction(
        program_id,
        "create_multisig",
        &create_multisig_data(multisig_id, 2, 2, owner_1.pubkey(), owner_2.pubkey(), owner_2.pubkey()),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(multisig_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // Non-owner tries to propose
    let tx_id: u64 = 1;
    let (transaction_pda, _) = derive_transaction_pda(multisig_id, tx_id);

    svm.expire_blockhash();
    let propose_ix = anchor_instruction(
        program_id,
        "propose_transaction",
        &propose_transaction_data(tx_id, Pubkey::new_unique(), 1_000_000_000),
        vec![
            signer_meta(non_owner.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, propose_ix, &non_owner, &[&non_owner]);
    assert!(result.is_err(), "propose_transaction by non-owner should fail");
}

#[test]
fn test_propose_transaction_zero_amount_fails() {
    let mut svm = load_multisig_program();
    let program_id = multisig_program_id();

    // Setup multisig
    let creator = funded_keypair_10_sol(&mut svm);
    let owner = funded_keypair_10_sol(&mut svm);
    let multisig_id: u64 = 1;
    let (multisig_pda, _) = derive_multisig_pda(multisig_id);

    let create_ix = anchor_instruction(
        program_id,
        "create_multisig",
        &create_multisig_data(multisig_id, 1, 1, owner.pubkey(), owner.pubkey(), owner.pubkey()),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(multisig_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // Propose with zero amount
    let tx_id: u64 = 1;
    let (transaction_pda, _) = derive_transaction_pda(multisig_id, tx_id);

    svm.expire_blockhash();
    let propose_ix = anchor_instruction(
        program_id,
        "propose_transaction",
        &propose_transaction_data(tx_id, Pubkey::new_unique(), 0),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, propose_ix, &owner, &[&owner]);
    assert!(result.is_err(), "propose_transaction with zero amount should fail");
}

// =============================================================================
// APPROVE TESTS
// =============================================================================

#[test]
fn test_approve() {
    let mut svm = load_multisig_program();
    let program_id = multisig_program_id();

    // Setup multisig (2-of-3)
    let creator = funded_keypair_10_sol(&mut svm);
    let owner_1 = funded_keypair_10_sol(&mut svm);
    let owner_2 = funded_keypair_10_sol(&mut svm);
    let owner_3 = funded_keypair_10_sol(&mut svm);
    let multisig_id: u64 = 1;
    let (multisig_pda, _) = derive_multisig_pda(multisig_id);

    let create_ix = anchor_instruction(
        program_id,
        "create_multisig",
        &create_multisig_data(multisig_id, 2, 3, owner_1.pubkey(), owner_2.pubkey(), owner_3.pubkey()),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(multisig_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // Propose transaction
    let tx_id: u64 = 1;
    let (transaction_pda, _) = derive_transaction_pda(multisig_id, tx_id);

    svm.expire_blockhash();
    let propose_ix = anchor_instruction(
        program_id,
        "propose_transaction",
        &propose_transaction_data(tx_id, Pubkey::new_unique(), 1_000_000_000),
        vec![
            signer_meta(owner_1.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, propose_ix, &owner_1, &[&owner_1]).unwrap();

    // First approval
    let (approval_pda, _) = derive_approval_pda(multisig_id, tx_id, &owner_1.pubkey());

    svm.expire_blockhash();
    let approve_ix = anchor_instruction(
        program_id,
        "approve",
        &approve_data(tx_id),
        vec![
            signer_meta(owner_1.pubkey()),       // approver
            writable_meta(multisig_pda),         // multisig (read for owner check)
            writable_meta(transaction_pda),      // transaction
            writable_meta(approval_pda),         // approval
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(solana_sdk_ids::sysvar::rent::id()), // rent
            readonly_meta(system_program::id()), // system_program
        ],
    );

    let result = execute_tx(&mut svm, approve_ix, &owner_1, &[&owner_1]);
    assert!(result.is_ok(), "approve should succeed: {:?}", result);

    // Verify approval record
    let approval_account = svm.get_account(&approval_pda).expect("Approval should exist");
    assert_eq!(read_approval_approver(&approval_account.data), owner_1.pubkey());
    assert_eq!(read_approval_transaction(&approval_account.data), transaction_pda);

    // Verify transaction approval count
    let tx_account = svm.get_account(&transaction_pda).unwrap();
    assert_eq!(read_transaction_approval_count(&tx_account.data), 1);
}

#[test]
fn test_double_approval_fails() {
    let mut svm = load_multisig_program();
    let program_id = multisig_program_id();

    // Setup multisig
    let creator = funded_keypair_10_sol(&mut svm);
    let owner = funded_keypair_10_sol(&mut svm);
    let multisig_id: u64 = 1;
    let (multisig_pda, _) = derive_multisig_pda(multisig_id);

    let create_ix = anchor_instruction(
        program_id,
        "create_multisig",
        &create_multisig_data(multisig_id, 2, 2, owner.pubkey(), owner.pubkey(), owner.pubkey()),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(multisig_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // Propose
    let tx_id: u64 = 1;
    let (transaction_pda, _) = derive_transaction_pda(multisig_id, tx_id);

    svm.expire_blockhash();
    let propose_ix = anchor_instruction(
        program_id,
        "propose_transaction",
        &propose_transaction_data(tx_id, Pubkey::new_unique(), 1_000_000_000),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, propose_ix, &owner, &[&owner]).unwrap();

    // First approval
    let (approval_pda, _) = derive_approval_pda(multisig_id, tx_id, &owner.pubkey());

    svm.expire_blockhash();
    let approve_ix = anchor_instruction(
        program_id,
        "approve",
        &approve_data(tx_id),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(approval_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, approve_ix, &owner, &[&owner]).unwrap();

    // Second approval from same owner should fail (PDA already exists)
    svm.expire_blockhash();
    let approve_ix2 = anchor_instruction(
        program_id,
        "approve",
        &approve_data(tx_id),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(approval_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, approve_ix2, &owner, &[&owner]);
    assert!(result.is_err(), "double approval should fail");
}

#[test]
fn test_approve_non_owner_fails() {
    let mut svm = load_multisig_program();
    let program_id = multisig_program_id();

    // Setup multisig
    let creator = funded_keypair_10_sol(&mut svm);
    let owner = funded_keypair_10_sol(&mut svm);
    let non_owner = funded_keypair_10_sol(&mut svm);
    let multisig_id: u64 = 1;
    let (multisig_pda, _) = derive_multisig_pda(multisig_id);

    let create_ix = anchor_instruction(
        program_id,
        "create_multisig",
        &create_multisig_data(multisig_id, 1, 1, owner.pubkey(), owner.pubkey(), owner.pubkey()),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(multisig_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // Propose
    let tx_id: u64 = 1;
    let (transaction_pda, _) = derive_transaction_pda(multisig_id, tx_id);

    svm.expire_blockhash();
    let propose_ix = anchor_instruction(
        program_id,
        "propose_transaction",
        &propose_transaction_data(tx_id, Pubkey::new_unique(), 1_000_000_000),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, propose_ix, &owner, &[&owner]).unwrap();

    // Non-owner tries to approve
    let (approval_pda, _) = derive_approval_pda(multisig_id, tx_id, &non_owner.pubkey());

    svm.expire_blockhash();
    let approve_ix = anchor_instruction(
        program_id,
        "approve",
        &approve_data(tx_id),
        vec![
            signer_meta(non_owner.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(approval_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, approve_ix, &non_owner, &[&non_owner]);
    assert!(result.is_err(), "approve by non-owner should fail");
}

// =============================================================================
// EXECUTE TESTS
// =============================================================================

#[test]
fn test_execute_threshold_met() {
    let mut svm = load_multisig_program();
    let program_id = multisig_program_id();

    // Setup multisig (2-of-3)
    let creator = funded_keypair_10_sol(&mut svm);
    let owner_1 = funded_keypair_10_sol(&mut svm);
    let owner_2 = funded_keypair_10_sol(&mut svm);
    let owner_3 = funded_keypair_10_sol(&mut svm);
    let recipient = funded_keypair_10_sol(&mut svm);
    let multisig_id: u64 = 1;
    let (multisig_pda, _) = derive_multisig_pda(multisig_id);

    let create_ix = anchor_instruction(
        program_id,
        "create_multisig",
        &create_multisig_data(multisig_id, 2, 3, owner_1.pubkey(), owner_2.pubkey(), owner_3.pubkey()),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(multisig_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // Fund multisig PDA
    let transfer_amount: u64 = 1_000_000_000; // 1 SOL
    let multisig_account = svm.get_account(&multisig_pda).unwrap();
    let initial_lamports = multisig_account.lamports;

    // Fund the multisig account with extra SOL
    let mut funded_account = multisig_account.clone();
    funded_account.lamports = initial_lamports + transfer_amount + 10_000_000; // Extra for rent
    svm.set_account(multisig_pda, funded_account).unwrap();

    // Propose transaction
    let tx_id: u64 = 1;
    let (transaction_pda, _) = derive_transaction_pda(multisig_id, tx_id);

    svm.expire_blockhash();
    let propose_ix = anchor_instruction(
        program_id,
        "propose_transaction",
        &propose_transaction_data(tx_id, recipient.pubkey(), transfer_amount),
        vec![
            signer_meta(owner_1.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, propose_ix, &owner_1, &[&owner_1]).unwrap();

    // Approval from owner_1
    let (approval_pda_1, _) = derive_approval_pda(multisig_id, tx_id, &owner_1.pubkey());
    svm.expire_blockhash();
    let approve_ix_1 = anchor_instruction(
        program_id,
        "approve",
        &approve_data(tx_id),
        vec![
            signer_meta(owner_1.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(approval_pda_1),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, approve_ix_1, &owner_1, &[&owner_1]).unwrap();

    // Approval from owner_2
    let (approval_pda_2, _) = derive_approval_pda(multisig_id, tx_id, &owner_2.pubkey());
    svm.expire_blockhash();
    let approve_ix_2 = anchor_instruction(
        program_id,
        "approve",
        &approve_data(tx_id),
        vec![
            signer_meta(owner_2.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(approval_pda_2),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, approve_ix_2, &owner_2, &[&owner_2]).unwrap();

    // Verify 2 approvals
    let tx_account = svm.get_account(&transaction_pda).unwrap();
    assert_eq!(read_transaction_approval_count(&tx_account.data), 2);

    // Record recipient balance before execute
    let recipient_balance_before = svm.get_account(&recipient.pubkey()).unwrap().lamports;

    // Execute transaction
    svm.expire_blockhash();
    let execute_ix = anchor_instruction(
        program_id,
        "execute",
        &execute_data(tx_id),
        vec![
            signer_meta(owner_3.pubkey()),  // executor (any owner can execute)
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(recipient.pubkey()),
        ],
    );

    let result = execute_tx(&mut svm, execute_ix, &owner_3, &[&owner_3]);
    assert!(result.is_ok(), "execute should succeed: {:?}", result);

    // Verify transaction marked as executed
    let tx_account = svm.get_account(&transaction_pda).unwrap();
    assert_eq!(read_transaction_is_executed(&tx_account.data), true);

    // Verify lamports transferred
    let recipient_balance_after = svm.get_account(&recipient.pubkey()).unwrap().lamports;
    assert_eq!(
        recipient_balance_after,
        recipient_balance_before + transfer_amount
    );
}

#[test]
fn test_execute_threshold_not_met_fails() {
    let mut svm = load_multisig_program();
    let program_id = multisig_program_id();

    // Setup multisig (2-of-3)
    let creator = funded_keypair_10_sol(&mut svm);
    let owner_1 = funded_keypair_10_sol(&mut svm);
    let owner_2 = funded_keypair_10_sol(&mut svm);
    let owner_3 = funded_keypair_10_sol(&mut svm);
    let recipient = funded_keypair_10_sol(&mut svm);
    let multisig_id: u64 = 1;
    let (multisig_pda, _) = derive_multisig_pda(multisig_id);

    let create_ix = anchor_instruction(
        program_id,
        "create_multisig",
        &create_multisig_data(multisig_id, 2, 3, owner_1.pubkey(), owner_2.pubkey(), owner_3.pubkey()),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(multisig_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // Fund multisig
    let multisig_account = svm.get_account(&multisig_pda).unwrap();
    let mut funded_account = multisig_account.clone();
    funded_account.lamports += 2_000_000_000;
    svm.set_account(multisig_pda, funded_account).unwrap();

    // Propose transaction
    let tx_id: u64 = 1;
    let (transaction_pda, _) = derive_transaction_pda(multisig_id, tx_id);

    svm.expire_blockhash();
    let propose_ix = anchor_instruction(
        program_id,
        "propose_transaction",
        &propose_transaction_data(tx_id, recipient.pubkey(), 1_000_000_000),
        vec![
            signer_meta(owner_1.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, propose_ix, &owner_1, &[&owner_1]).unwrap();

    // Only 1 approval (need 2)
    let (approval_pda_1, _) = derive_approval_pda(multisig_id, tx_id, &owner_1.pubkey());
    svm.expire_blockhash();
    let approve_ix_1 = anchor_instruction(
        program_id,
        "approve",
        &approve_data(tx_id),
        vec![
            signer_meta(owner_1.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(approval_pda_1),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, approve_ix_1, &owner_1, &[&owner_1]).unwrap();

    // Try to execute with only 1 approval
    svm.expire_blockhash();
    let execute_ix = anchor_instruction(
        program_id,
        "execute",
        &execute_data(tx_id),
        vec![
            signer_meta(owner_1.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(recipient.pubkey()),
        ],
    );

    let result = execute_tx(&mut svm, execute_ix, &owner_1, &[&owner_1]);
    assert!(result.is_err(), "execute with insufficient approvals should fail");
}

#[test]
fn test_execute_already_executed_fails() {
    let mut svm = load_multisig_program();
    let program_id = multisig_program_id();

    // Setup 1-of-1 multisig for simplicity
    let creator = funded_keypair_10_sol(&mut svm);
    let owner = funded_keypair_10_sol(&mut svm);
    let recipient = funded_keypair_10_sol(&mut svm);
    let multisig_id: u64 = 1;
    let (multisig_pda, _) = derive_multisig_pda(multisig_id);

    let create_ix = anchor_instruction(
        program_id,
        "create_multisig",
        &create_multisig_data(multisig_id, 1, 1, owner.pubkey(), owner.pubkey(), owner.pubkey()),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(multisig_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // Fund multisig
    let multisig_account = svm.get_account(&multisig_pda).unwrap();
    let mut funded_account = multisig_account.clone();
    funded_account.lamports += 3_000_000_000;
    svm.set_account(multisig_pda, funded_account).unwrap();

    // Propose and approve
    let tx_id: u64 = 1;
    let (transaction_pda, _) = derive_transaction_pda(multisig_id, tx_id);
    let (approval_pda, _) = derive_approval_pda(multisig_id, tx_id, &owner.pubkey());

    svm.expire_blockhash();
    let propose_ix = anchor_instruction(
        program_id,
        "propose_transaction",
        &propose_transaction_data(tx_id, recipient.pubkey(), 1_000_000_000),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, propose_ix, &owner, &[&owner]).unwrap();

    svm.expire_blockhash();
    let approve_ix = anchor_instruction(
        program_id,
        "approve",
        &approve_data(tx_id),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(approval_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, approve_ix, &owner, &[&owner]).unwrap();

    // First execute
    svm.expire_blockhash();
    let execute_ix = anchor_instruction(
        program_id,
        "execute",
        &execute_data(tx_id),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(recipient.pubkey()),
        ],
    );
    execute_tx(&mut svm, execute_ix, &owner, &[&owner]).unwrap();

    // Second execute should fail
    svm.expire_blockhash();
    let execute_ix2 = anchor_instruction(
        program_id,
        "execute",
        &execute_data(tx_id),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(recipient.pubkey()),
        ],
    );

    let result = execute_tx(&mut svm, execute_ix2, &owner, &[&owner]);
    assert!(result.is_err(), "execute already executed should fail");
}

// =============================================================================
// FULL WORKFLOW TESTS
// =============================================================================

#[test]
fn test_full_multisig_workflow_2_of_3() {
    let mut svm = load_multisig_program();
    let program_id = multisig_program_id();

    // Setup 2-of-3 multisig
    let creator = funded_keypair_10_sol(&mut svm);
    let owner_1 = funded_keypair_10_sol(&mut svm);
    let owner_2 = funded_keypair_10_sol(&mut svm);
    let owner_3 = funded_keypair_10_sol(&mut svm);
    let recipient = funded_keypair_10_sol(&mut svm);
    let multisig_id: u64 = 1;
    let (multisig_pda, _) = derive_multisig_pda(multisig_id);

    // 1. Create multisig
    let create_ix = anchor_instruction(
        program_id,
        "create_multisig",
        &create_multisig_data(multisig_id, 2, 3, owner_1.pubkey(), owner_2.pubkey(), owner_3.pubkey()),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(multisig_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // 2. Fund multisig
    let transfer_amount: u64 = 5_000_000_000; // 5 SOL
    let multisig_account = svm.get_account(&multisig_pda).unwrap();
    let mut funded_account = multisig_account.clone();
    funded_account.lamports += transfer_amount + 10_000_000;
    svm.set_account(multisig_pda, funded_account).unwrap();

    // 3. Propose transaction (owner_1)
    let tx_id: u64 = 1;
    let (transaction_pda, _) = derive_transaction_pda(multisig_id, tx_id);

    svm.expire_blockhash();
    let propose_ix = anchor_instruction(
        program_id,
        "propose_transaction",
        &propose_transaction_data(tx_id, recipient.pubkey(), transfer_amount),
        vec![
            signer_meta(owner_1.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, propose_ix, &owner_1, &[&owner_1]).unwrap();

    // 4. Approve by owner_2
    let (approval_pda_2, _) = derive_approval_pda(multisig_id, tx_id, &owner_2.pubkey());
    svm.expire_blockhash();
    let approve_ix_2 = anchor_instruction(
        program_id,
        "approve",
        &approve_data(tx_id),
        vec![
            signer_meta(owner_2.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(approval_pda_2),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, approve_ix_2, &owner_2, &[&owner_2]).unwrap();

    // Verify 1 approval
    let tx_account = svm.get_account(&transaction_pda).unwrap();
    assert_eq!(read_transaction_approval_count(&tx_account.data), 1);

    // 5. Try to execute (should fail - only 1 approval)
    svm.expire_blockhash();
    let execute_ix = anchor_instruction(
        program_id,
        "execute",
        &execute_data(tx_id),
        vec![
            signer_meta(owner_1.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(recipient.pubkey()),
        ],
    );
    let result = execute_tx(&mut svm, execute_ix, &owner_1, &[&owner_1]);
    assert!(result.is_err(), "execute with 1 approval should fail");

    // 6. Approve by owner_3
    let (approval_pda_3, _) = derive_approval_pda(multisig_id, tx_id, &owner_3.pubkey());
    svm.expire_blockhash();
    let approve_ix_3 = anchor_instruction(
        program_id,
        "approve",
        &approve_data(tx_id),
        vec![
            signer_meta(owner_3.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(approval_pda_3),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, approve_ix_3, &owner_3, &[&owner_3]).unwrap();

    // Verify 2 approvals
    let tx_account = svm.get_account(&transaction_pda).unwrap();
    assert_eq!(read_transaction_approval_count(&tx_account.data), 2);

    // 7. Execute (should succeed now)
    let recipient_balance_before = svm.get_account(&recipient.pubkey()).unwrap().lamports;

    svm.expire_blockhash();
    let execute_ix = anchor_instruction(
        program_id,
        "execute",
        &execute_data(tx_id),
        vec![
            signer_meta(owner_1.pubkey()),
            writable_meta(multisig_pda),
            writable_meta(transaction_pda),
            writable_meta(recipient.pubkey()),
        ],
    );
    execute_tx(&mut svm, execute_ix, &owner_1, &[&owner_1]).unwrap();

    // 8. Verify final state
    let tx_account = svm.get_account(&transaction_pda).unwrap();
    assert_eq!(read_transaction_is_executed(&tx_account.data), true);

    let recipient_balance_after = svm.get_account(&recipient.pubkey()).unwrap().lamports;
    assert_eq!(recipient_balance_after, recipient_balance_before + transfer_amount);
}

#[test]
fn test_pda_derivation_deterministic() {
    let multisig_id: u64 = 123;
    let tx_id: u64 = 456;
    let approver = Pubkey::new_unique();

    // Derive multiple times
    let (pda1, bump1) = derive_multisig_pda(multisig_id);
    let (pda2, bump2) = derive_multisig_pda(multisig_id);
    let (tx_pda1, tx_bump1) = derive_transaction_pda(multisig_id, tx_id);
    let (tx_pda2, tx_bump2) = derive_transaction_pda(multisig_id, tx_id);
    let (approval_pda1, approval_bump1) = derive_approval_pda(multisig_id, tx_id, &approver);
    let (approval_pda2, approval_bump2) = derive_approval_pda(multisig_id, tx_id, &approver);

    // Should be identical
    assert_eq!(pda1, pda2);
    assert_eq!(bump1, bump2);
    assert_eq!(tx_pda1, tx_pda2);
    assert_eq!(tx_bump1, tx_bump2);
    assert_eq!(approval_pda1, approval_pda2);
    assert_eq!(approval_bump1, approval_bump2);

    // Different seeds should produce different PDAs
    let (other_pda, _) = derive_multisig_pda(789);
    assert_ne!(pda1, other_pda);

    let (other_tx_pda, _) = derive_transaction_pda(multisig_id, 999);
    assert_ne!(tx_pda1, other_tx_pda);

    let other_approver = Pubkey::new_unique();
    let (other_approval_pda, _) = derive_approval_pda(multisig_id, tx_id, &other_approver);
    assert_ne!(approval_pda1, other_approval_pda);
}
