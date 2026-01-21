//! Voting tests: Seahorse implementation behavior verification
//!
//! These tests verify the Seahorse voting implementation produces
//! consistent behavior for:
//! - Proposal initialization with correct state
//! - Vote tallying (yes/no counts)
//! - Double-vote prevention via voter record PDA
//! - Finalization mechanics (time-based)
//! - Deterministic PDA derivation
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile:
//! - target/deploy/voting.so (Seahorse)

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

/// Voting program ID
fn program_id() -> Pubkey {
    Pubkey::from_str("VoteSimp1e111111111111111111111111111111111").unwrap()
}

/// Derive proposal PDA
fn derive_proposal_pda(proposal_id: u64, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"proposal", proposal_id.to_le_bytes().as_ref()],
        program_id,
    )
}

/// Derive voter record PDA
fn derive_voter_record_pda(proposal_id: u64, voter: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"voter_record", proposal_id.to_le_bytes().as_ref(), voter.as_ref()],
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

/// Build create_proposal instruction data
fn create_proposal_data(
    proposal_id: u64,
    title: &str,
    description: &str,
    voting_duration_slots: u64,
) -> Vec<u8> {
    let mut data = instruction_disc("create_proposal").to_vec();
    data.extend_from_slice(&proposal_id.to_le_bytes());
    data.extend_from_slice(&(title.len() as u32).to_le_bytes());
    data.extend_from_slice(title.as_bytes());
    data.extend_from_slice(&(description.len() as u32).to_le_bytes());
    data.extend_from_slice(description.as_bytes());
    data.extend_from_slice(&voting_duration_slots.to_le_bytes());
    data
}

/// Build cast_vote instruction data
fn cast_vote_data(proposal_id: u64, vote_yes: bool) -> Vec<u8> {
    let mut data = instruction_disc("cast_vote").to_vec();
    data.extend_from_slice(&proposal_id.to_le_bytes());
    data.push(if vote_yes { 1 } else { 0 });
    data
}

/// Build finalize_proposal instruction data
fn finalize_proposal_data() -> Vec<u8> {
    instruction_disc("finalize_proposal").to_vec()
}

/// Read proposal yes_votes from account data (dynamic offset due to variable-length strings)
fn read_proposal_yes_votes(data: &[u8]) -> u64 {
    let title_len = u32::from_le_bytes(data[48..52].try_into().unwrap()) as usize;
    let desc_offset = 52 + title_len;
    let desc_len = u32::from_le_bytes(data[desc_offset..desc_offset + 4].try_into().unwrap()) as usize;
    let yes_votes_offset = desc_offset + 4 + desc_len;
    let bytes: [u8; 8] = data[yes_votes_offset..yes_votes_offset + 8].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_proposal_no_votes(data: &[u8]) -> u64 {
    let title_len = u32::from_le_bytes(data[48..52].try_into().unwrap()) as usize;
    let desc_offset = 52 + title_len;
    let desc_len = u32::from_le_bytes(data[desc_offset..desc_offset + 4].try_into().unwrap()) as usize;
    let no_votes_offset = desc_offset + 4 + desc_len + 8;
    let bytes: [u8; 8] = data[no_votes_offset..no_votes_offset + 8].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_proposal_is_finalized(data: &[u8]) -> bool {
    let title_len = u32::from_le_bytes(data[48..52].try_into().unwrap()) as usize;
    let desc_offset = 52 + title_len;
    let desc_len = u32::from_le_bytes(data[desc_offset..desc_offset + 4].try_into().unwrap()) as usize;
    let is_finalized_offset = desc_offset + 4 + desc_len + 32;
    data[is_finalized_offset] != 0
}

fn read_voter_record_vote_yes(data: &[u8]) -> bool {
    data[72] != 0
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
    svm.send_transaction(tx).map(|_| ()).map_err(|e| format!("{:?}", e))
}

// =============================================================================
// PARITY TESTS
// =============================================================================

#[test]
fn test_parity_proposal_initialization() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/voting.so")
        .expect("Failed to read voting.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup creator
    let creator = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let proposal_id: u64 = 1;
    let title = "Test Proposal";
    let description = "Test description";
    let voting_duration: u64 = 100;

    let (proposal_pda, _) = derive_proposal_pda(proposal_id, &program_id);

    // Create proposal
    let ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(creator.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: create_proposal_data(proposal_id, title, description, voting_duration),
    };

    let result = execute_tx(&mut svm, ix, &creator, &[&creator]);
    assert!(result.is_ok(), "create_proposal should succeed: {:?}", result);

    // Verify initial state
    let proposal_account = svm.get_account(&proposal_pda).unwrap();

    // Proposal should have zero votes initially
    assert_eq!(read_proposal_yes_votes(&proposal_account.data), 0, "yes_votes should be 0");
    assert_eq!(read_proposal_no_votes(&proposal_account.data), 0, "no_votes should be 0");
    assert_eq!(read_proposal_is_finalized(&proposal_account.data), false, "should not be finalized");
}

#[test]
fn test_parity_vote_tallying() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/voting.so")
        .expect("Failed to read voting.so");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    let creator = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let proposal_id: u64 = 1;
    let (proposal_pda, _) = derive_proposal_pda(proposal_id, &program_id);

    // Create proposal
    let create_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(creator.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: create_proposal_data(proposal_id, "Test", "Test", 100),
    };
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // Cast votes: 3 yes, 2 no
    let votes = vec![true, false, true, true, false];
    for vote_yes in votes {
        let voter = {
            let kp = Keypair::new();
            svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
            kp
        };
        let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey(), &program_id);

        svm.expire_blockhash();
        let vote_ix = Instruction {
            program_id,
            accounts: vec![
                signer_meta(voter.pubkey()),
                writable_meta(proposal_pda),
                writable_meta(voter_record_pda),
                readonly_meta(solana_sdk_ids::sysvar::clock::id()),
                readonly_meta(solana_sdk_ids::sysvar::rent::id()),
                readonly_meta(system_program::id()),
            ],
            data: cast_vote_data(proposal_id, vote_yes),
        };
        execute_tx(&mut svm, vote_ix, &voter, &[&voter]).unwrap();
    }

    // Verify vote counts
    let proposal_account = svm.get_account(&proposal_pda).unwrap();
    assert_eq!(read_proposal_yes_votes(&proposal_account.data), 3, "yes_votes should be 3");
    assert_eq!(read_proposal_no_votes(&proposal_account.data), 2, "no_votes should be 2");
}

#[test]
fn test_parity_double_vote_prevention() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/voting.so")
        .expect("Failed to read voting.so");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    let creator = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let voter = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let proposal_id: u64 = 1;
    let (proposal_pda, _) = derive_proposal_pda(proposal_id, &program_id);
    let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey(), &program_id);

    // Create proposal
    let create_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(creator.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: create_proposal_data(proposal_id, "Test", "Test", 100),
    };
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // First vote
    svm.expire_blockhash();
    let vote_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(voter.pubkey()),
            writable_meta(proposal_pda),
            writable_meta(voter_record_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: cast_vote_data(proposal_id, true),
    };
    let first_vote = execute_tx(&mut svm, vote_ix, &voter, &[&voter]);
    assert!(first_vote.is_ok(), "first vote should succeed");

    // Verify voter record stores the vote
    let voter_record = svm.get_account(&voter_record_pda).unwrap();
    assert_eq!(read_voter_record_vote_yes(&voter_record.data), true, "voter record should show yes vote");

    // Second vote should fail (PDA already exists)
    svm.expire_blockhash();
    let vote_ix2 = Instruction {
        program_id,
        accounts: vec![
            signer_meta(voter.pubkey()),
            writable_meta(proposal_pda),
            writable_meta(voter_record_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: cast_vote_data(proposal_id, false),
    };
    let second_vote = execute_tx(&mut svm, vote_ix2, &voter, &[&voter]);
    assert!(second_vote.is_err(), "second vote should fail - double voting prevented");

    // Vote count should remain unchanged
    let proposal_account = svm.get_account(&proposal_pda).unwrap();
    assert_eq!(read_proposal_yes_votes(&proposal_account.data), 1, "yes_votes should still be 1");
    assert_eq!(read_proposal_no_votes(&proposal_account.data), 0, "no_votes should still be 0");
}

#[test]
fn test_parity_finalization_mechanics() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/voting.so")
        .expect("Failed to read voting.so");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    let creator = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let proposal_id: u64 = 1;
    let voting_duration: u64 = 10;

    let (proposal_pda, _) = derive_proposal_pda(proposal_id, &program_id);

    // Create proposal
    let create_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(creator.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: create_proposal_data(proposal_id, "Test", "Test", voting_duration),
    };
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // Cast a vote
    let voter = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey(), &program_id);
    svm.expire_blockhash();
    let vote_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(voter.pubkey()),
            writable_meta(proposal_pda),
            writable_meta(voter_record_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: cast_vote_data(proposal_id, true),
    };
    execute_tx(&mut svm, vote_ix, &voter, &[&voter]).unwrap();

    // Try to finalize before end - should fail
    let finalizer = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    svm.expire_blockhash();
    let finalize_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(finalizer.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
        data: finalize_proposal_data(),
    };
    let early_finalize = execute_tx(&mut svm, finalize_ix, &finalizer, &[&finalizer]);
    assert!(early_finalize.is_err(), "finalize before end should fail");

    // Warp to after voting period
    svm.warp_to_slot(15);
    svm.expire_blockhash();

    // Now finalize should succeed
    let finalize_ix2 = Instruction {
        program_id,
        accounts: vec![
            signer_meta(finalizer.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
        data: finalize_proposal_data(),
    };
    let finalize_result = execute_tx(&mut svm, finalize_ix2, &finalizer, &[&finalizer]);
    assert!(finalize_result.is_ok(), "finalize after end should succeed");

    // Verify finalized state
    let proposal_account = svm.get_account(&proposal_pda).unwrap();
    assert_eq!(read_proposal_is_finalized(&proposal_account.data), true, "should be finalized");

    // Try to finalize again - should fail
    svm.expire_blockhash();
    let finalize_ix3 = Instruction {
        program_id,
        accounts: vec![
            signer_meta(finalizer.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
        data: finalize_proposal_data(),
    };
    let double_finalize = execute_tx(&mut svm, finalize_ix3, &finalizer, &[&finalizer]);
    assert!(double_finalize.is_err(), "double finalize should fail");
}

#[test]
fn test_parity_deterministic_pda_derivation() {
    let program_id = program_id();

    // Test that PDA derivation is deterministic across multiple calls
    let proposal_id: u64 = 42;
    let voter = Pubkey::new_unique();

    // Derive multiple times
    let (proposal_pda1, bump1) = derive_proposal_pda(proposal_id, &program_id);
    let (proposal_pda2, bump2) = derive_proposal_pda(proposal_id, &program_id);

    assert_eq!(proposal_pda1, proposal_pda2, "proposal PDA should be deterministic");
    assert_eq!(bump1, bump2, "proposal bump should be deterministic");

    let (voter_record_pda1, vbump1) = derive_voter_record_pda(proposal_id, &voter, &program_id);
    let (voter_record_pda2, vbump2) = derive_voter_record_pda(proposal_id, &voter, &program_id);

    assert_eq!(voter_record_pda1, voter_record_pda2, "voter record PDA should be deterministic");
    assert_eq!(vbump1, vbump2, "voter record bump should be deterministic");

    // Different proposal_id should produce different PDA
    let (other_pda, _) = derive_proposal_pda(99, &program_id);
    assert_ne!(proposal_pda1, other_pda, "different proposal_id should produce different PDA");

    // Different voter should produce different voter_record PDA
    let other_voter = Pubkey::new_unique();
    let (other_voter_pda, _) = derive_voter_record_pda(proposal_id, &other_voter, &program_id);
    assert_ne!(voter_record_pda1, other_voter_pda, "different voter should produce different PDA");
}
