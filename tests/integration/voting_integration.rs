//! LiteSVM integration tests for the Seahorse Voting program
//!
//! These tests verify basic voting mechanics:
//! - create_proposal: Creates a new proposal with voting period
//! - cast_vote: Records votes with double-vote prevention via voter record PDA
//! - finalize_proposal: Finalizes proposal after voting period ends
//!
//! Uses LiteSVM's warp_to_slot for time-based voting tests.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile voting.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// Voting program ID (from declare_id!)
fn voting_program_id() -> Pubkey {
    Pubkey::from_str("VoteSimp1e111111111111111111111111111111111").unwrap()
}

/// Proposal account size: discriminator (8) + creator (32) + proposal_id (8) + title (4+64) + description (4+256)
///                        + yes_votes (8) + no_votes (8) + start_slot (8) + end_slot (8) + is_finalized (1) + bump (1)
/// Using 400 padding to match Seahorse
const PROPOSAL_SIZE: usize = 8 + 32 + 8 + (4 + 64) + (4 + 256) + 8 + 8 + 8 + 8 + 1 + 1 + 400;

/// VoterRecord account size: discriminator (8) + voter (32) + proposal (32) + vote_yes (1) + voted_at_slot (8) + bump (1)
const VOTER_RECORD_SIZE: usize = 8 + 32 + 32 + 1 + 8 + 1;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the voting program into LiteSVM
fn load_voting_program() -> litesvm::LiteSVM {
    let program_id = voting_program_id();
    let program_bytes = std::fs::read("../../target/deploy/voting.so")
        .expect("Failed to read voting.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);
    svm
}

/// Derive proposal PDA
fn derive_proposal_pda(proposal_id: u64) -> (Pubkey, u8) {
    find_pda(
        &[b"proposal", proposal_id.to_le_bytes().as_ref()],
        &voting_program_id(),
    )
}

/// Derive voter record PDA
fn derive_voter_record_pda(proposal_id: u64, voter: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"voter_record", proposal_id.to_le_bytes().as_ref(), voter.as_ref()],
        &voting_program_id(),
    )
}

/// Build create_proposal instruction data
fn create_proposal_data(
    proposal_id: u64,
    title: &str,
    description: &str,
    voting_duration_slots: u64,
) -> Vec<u8> {
    let mut data = Vec::new();
    // proposal_id: u64
    data.extend_from_slice(&proposal_id.to_le_bytes());
    // title: String (length prefix + bytes)
    data.extend_from_slice(&(title.len() as u32).to_le_bytes());
    data.extend_from_slice(title.as_bytes());
    // description: String (length prefix + bytes)
    data.extend_from_slice(&(description.len() as u32).to_le_bytes());
    data.extend_from_slice(description.as_bytes());
    // voting_duration_slots: u64
    data.extend_from_slice(&voting_duration_slots.to_le_bytes());
    data
}

/// Build cast_vote instruction data
fn cast_vote_data(proposal_id: u64, vote_yes: bool) -> Vec<u8> {
    let mut data = Vec::new();
    // proposal_id: u64
    data.extend_from_slice(&proposal_id.to_le_bytes());
    // vote_yes: bool
    data.push(if vote_yes { 1 } else { 0 });
    data
}

/// Read proposal fields from account data
fn read_proposal_creator(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

fn read_proposal_id(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[40..48].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_proposal_title(data: &[u8]) -> String {
    let len = u32::from_le_bytes(data[48..52].try_into().unwrap()) as usize;
    String::from_utf8(data[52..52 + len].to_vec()).unwrap()
}

fn read_proposal_description(data: &[u8]) -> String {
    let title_len = u32::from_le_bytes(data[48..52].try_into().unwrap()) as usize;
    let desc_offset = 52 + title_len;
    let desc_len = u32::from_le_bytes(data[desc_offset..desc_offset + 4].try_into().unwrap()) as usize;
    String::from_utf8(data[desc_offset + 4..desc_offset + 4 + desc_len].to_vec()).unwrap()
}

fn read_proposal_yes_votes(data: &[u8]) -> u64 {
    // Dynamic offset based on title and description lengths
    let title_len = u32::from_le_bytes(data[48..52].try_into().unwrap()) as usize;
    let desc_offset = 52 + title_len;
    let desc_len = u32::from_le_bytes(data[desc_offset..desc_offset + 4].try_into().unwrap()) as usize;
    let yes_votes_offset = desc_offset + 4 + desc_len;
    let bytes: [u8; 8] = data[yes_votes_offset..yes_votes_offset + 8].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_proposal_no_votes(data: &[u8]) -> u64 {
    // Dynamic offset based on title and description lengths
    let title_len = u32::from_le_bytes(data[48..52].try_into().unwrap()) as usize;
    let desc_offset = 52 + title_len;
    let desc_len = u32::from_le_bytes(data[desc_offset..desc_offset + 4].try_into().unwrap()) as usize;
    let no_votes_offset = desc_offset + 4 + desc_len + 8;
    let bytes: [u8; 8] = data[no_votes_offset..no_votes_offset + 8].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_proposal_start_slot(data: &[u8]) -> u64 {
    let title_len = u32::from_le_bytes(data[48..52].try_into().unwrap()) as usize;
    let desc_offset = 52 + title_len;
    let desc_len = u32::from_le_bytes(data[desc_offset..desc_offset + 4].try_into().unwrap()) as usize;
    let start_slot_offset = desc_offset + 4 + desc_len + 16;
    let bytes: [u8; 8] = data[start_slot_offset..start_slot_offset + 8].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_proposal_end_slot(data: &[u8]) -> u64 {
    let title_len = u32::from_le_bytes(data[48..52].try_into().unwrap()) as usize;
    let desc_offset = 52 + title_len;
    let desc_len = u32::from_le_bytes(data[desc_offset..desc_offset + 4].try_into().unwrap()) as usize;
    let end_slot_offset = desc_offset + 4 + desc_len + 24;
    let bytes: [u8; 8] = data[end_slot_offset..end_slot_offset + 8].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_proposal_is_finalized(data: &[u8]) -> bool {
    let title_len = u32::from_le_bytes(data[48..52].try_into().unwrap()) as usize;
    let desc_offset = 52 + title_len;
    let desc_len = u32::from_le_bytes(data[desc_offset..desc_offset + 4].try_into().unwrap()) as usize;
    let is_finalized_offset = desc_offset + 4 + desc_len + 32;
    data[is_finalized_offset] != 0
}

/// Read voter record fields
fn read_voter_record_voter(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

fn read_voter_record_proposal(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[40..72].try_into().unwrap())
}

fn read_voter_record_vote_yes(data: &[u8]) -> bool {
    data[72] != 0
}

fn read_voter_record_voted_at_slot(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[73..81].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

// =============================================================================
// CREATE PROPOSAL TESTS
// =============================================================================

#[test]
fn test_create_proposal() {
    let mut svm = load_voting_program();
    let program_id = voting_program_id();

    // Setup creator
    let creator = funded_keypair_10_sol(&mut svm);

    // Proposal parameters
    let proposal_id: u64 = 1;
    let title = "Test Proposal";
    let description = "This is a test proposal for voting";
    let voting_duration_slots: u64 = 100;

    // Derive PDA
    let (proposal_pda, _) = derive_proposal_pda(proposal_id);

    // Build create_proposal instruction
    let ix = anchor_instruction(
        program_id,
        "create_proposal",
        &create_proposal_data(proposal_id, title, description, voting_duration_slots),
        vec![
            signer_meta(creator.pubkey()),     // creator
            writable_meta(proposal_pda),        // proposal
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(solana_sdk_ids::sysvar::rent::id()), // rent
            readonly_meta(system_program::id()), // system_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &creator, &[&creator]);
    assert!(result.is_ok(), "create_proposal should succeed: {:?}", result);

    // Verify proposal state
    let proposal_account = svm.get_account(&proposal_pda).expect("Proposal should exist");
    assert_eq!(proposal_account.owner, program_id);

    assert_eq!(read_proposal_creator(&proposal_account.data), creator.pubkey());
    assert_eq!(read_proposal_id(&proposal_account.data), proposal_id);
    assert_eq!(read_proposal_title(&proposal_account.data), title);
    assert_eq!(read_proposal_description(&proposal_account.data), description);
    assert_eq!(read_proposal_yes_votes(&proposal_account.data), 0);
    assert_eq!(read_proposal_no_votes(&proposal_account.data), 0);
    assert_eq!(read_proposal_is_finalized(&proposal_account.data), false);
}

#[test]
fn test_create_proposal_different_ids() {
    let mut svm = load_voting_program();
    let program_id = voting_program_id();

    let creator = funded_keypair_10_sol(&mut svm);

    // Create multiple proposals
    for proposal_id in 1..=3 {
        let (proposal_pda, _) = derive_proposal_pda(proposal_id);
        let title = format!("Proposal {}", proposal_id);
        let description = format!("Description for proposal {}", proposal_id);

        let ix = anchor_instruction(
            program_id,
            "create_proposal",
            &create_proposal_data(proposal_id, &title, &description, 100),
            vec![
                signer_meta(creator.pubkey()),
                writable_meta(proposal_pda),
                readonly_meta(solana_sdk_ids::sysvar::clock::id()),
                readonly_meta(solana_sdk_ids::sysvar::rent::id()),
                readonly_meta(system_program::id()),
            ],
        );

        svm.expire_blockhash();
        let result = execute_tx(&mut svm, ix, &creator, &[&creator]);
        assert!(result.is_ok(), "create_proposal {} should succeed", proposal_id);

        // Verify proposal exists
        let proposal_account = svm.get_account(&proposal_pda).expect("Proposal should exist");
        assert_eq!(read_proposal_id(&proposal_account.data), proposal_id);
    }
}

#[test]
fn test_create_proposal_title_too_long_fails() {
    let mut svm = load_voting_program();
    let program_id = voting_program_id();

    let creator = funded_keypair_10_sol(&mut svm);
    let proposal_id: u64 = 1;
    let title = "A".repeat(65); // 65 chars > 64 max
    let description = "Test description";

    let (proposal_pda, _) = derive_proposal_pda(proposal_id);

    let ix = anchor_instruction(
        program_id,
        "create_proposal",
        &create_proposal_data(proposal_id, &title, description, 100),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &creator, &[&creator]);
    assert!(result.is_err(), "create_proposal with title > 64 chars should fail");
}

#[test]
fn test_create_proposal_zero_duration_fails() {
    let mut svm = load_voting_program();
    let program_id = voting_program_id();

    let creator = funded_keypair_10_sol(&mut svm);
    let proposal_id: u64 = 1;

    let (proposal_pda, _) = derive_proposal_pda(proposal_id);

    let ix = anchor_instruction(
        program_id,
        "create_proposal",
        &create_proposal_data(proposal_id, "Test", "Test", 0), // zero duration
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &creator, &[&creator]);
    assert!(result.is_err(), "create_proposal with zero duration should fail");
}

// =============================================================================
// CAST VOTE TESTS
// =============================================================================

#[test]
fn test_cast_vote_yes() {
    let mut svm = load_voting_program();
    let program_id = voting_program_id();

    // Setup
    let creator = funded_keypair_10_sol(&mut svm);
    let voter = funded_keypair_10_sol(&mut svm);
    let proposal_id: u64 = 1;

    // Create proposal
    let (proposal_pda, _) = derive_proposal_pda(proposal_id);
    let create_ix = anchor_instruction(
        program_id,
        "create_proposal",
        &create_proposal_data(proposal_id, "Test", "Test", 100),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // Cast yes vote
    svm.expire_blockhash();
    let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey());
    let vote_ix = anchor_instruction(
        program_id,
        "cast_vote",
        &cast_vote_data(proposal_id, true),
        vec![
            signer_meta(voter.pubkey()),       // voter
            writable_meta(proposal_pda),        // proposal
            writable_meta(voter_record_pda),    // voter_record
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(solana_sdk_ids::sysvar::rent::id()), // rent
            readonly_meta(system_program::id()), // system_program
        ],
    );

    let result = execute_tx(&mut svm, vote_ix, &voter, &[&voter]);
    assert!(result.is_ok(), "cast_vote (yes) should succeed: {:?}", result);

    // Verify proposal vote count
    let proposal_account = svm.get_account(&proposal_pda).unwrap();
    assert_eq!(read_proposal_yes_votes(&proposal_account.data), 1);
    assert_eq!(read_proposal_no_votes(&proposal_account.data), 0);

    // Verify voter record
    let voter_record_account = svm.get_account(&voter_record_pda).expect("Voter record should exist");
    assert_eq!(read_voter_record_voter(&voter_record_account.data), voter.pubkey());
    assert_eq!(read_voter_record_proposal(&voter_record_account.data), proposal_pda);
    assert_eq!(read_voter_record_vote_yes(&voter_record_account.data), true);
}

#[test]
fn test_cast_vote_no() {
    let mut svm = load_voting_program();
    let program_id = voting_program_id();

    let creator = funded_keypair_10_sol(&mut svm);
    let voter = funded_keypair_10_sol(&mut svm);
    let proposal_id: u64 = 1;

    // Create proposal
    let (proposal_pda, _) = derive_proposal_pda(proposal_id);
    let create_ix = anchor_instruction(
        program_id,
        "create_proposal",
        &create_proposal_data(proposal_id, "Test", "Test", 100),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // Cast no vote
    svm.expire_blockhash();
    let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey());
    let vote_ix = anchor_instruction(
        program_id,
        "cast_vote",
        &cast_vote_data(proposal_id, false),
        vec![
            signer_meta(voter.pubkey()),
            writable_meta(proposal_pda),
            writable_meta(voter_record_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, vote_ix, &voter, &[&voter]);
    assert!(result.is_ok(), "cast_vote (no) should succeed: {:?}", result);

    // Verify proposal vote count
    let proposal_account = svm.get_account(&proposal_pda).unwrap();
    assert_eq!(read_proposal_yes_votes(&proposal_account.data), 0);
    assert_eq!(read_proposal_no_votes(&proposal_account.data), 1);

    // Verify voter record
    let voter_record_account = svm.get_account(&voter_record_pda).expect("Voter record should exist");
    assert_eq!(read_voter_record_vote_yes(&voter_record_account.data), false);
}

#[test]
fn test_cast_vote_multiple_voters() {
    let mut svm = load_voting_program();
    let program_id = voting_program_id();

    let creator = funded_keypair_10_sol(&mut svm);
    let proposal_id: u64 = 1;

    // Create proposal
    let (proposal_pda, _) = derive_proposal_pda(proposal_id);
    let create_ix = anchor_instruction(
        program_id,
        "create_proposal",
        &create_proposal_data(proposal_id, "Test", "Test", 100),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // Cast 3 yes votes, 2 no votes
    let votes = vec![true, true, false, true, false];
    for (i, vote_yes) in votes.iter().enumerate() {
        let voter = funded_keypair_10_sol(&mut svm);
        let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey());

        svm.expire_blockhash();
        let vote_ix = anchor_instruction(
            program_id,
            "cast_vote",
            &cast_vote_data(proposal_id, *vote_yes),
            vec![
                signer_meta(voter.pubkey()),
                writable_meta(proposal_pda),
                writable_meta(voter_record_pda),
                readonly_meta(solana_sdk_ids::sysvar::clock::id()),
                readonly_meta(solana_sdk_ids::sysvar::rent::id()),
                readonly_meta(system_program::id()),
            ],
        );

        let result = execute_tx(&mut svm, vote_ix, &voter, &[&voter]);
        assert!(result.is_ok(), "vote {} should succeed", i);
    }

    // Verify final vote counts
    let proposal_account = svm.get_account(&proposal_pda).unwrap();
    assert_eq!(read_proposal_yes_votes(&proposal_account.data), 3);
    assert_eq!(read_proposal_no_votes(&proposal_account.data), 2);
}

#[test]
fn test_double_vote_fails() {
    let mut svm = load_voting_program();
    let program_id = voting_program_id();

    let creator = funded_keypair_10_sol(&mut svm);
    let voter = funded_keypair_10_sol(&mut svm);
    let proposal_id: u64 = 1;

    // Create proposal
    let (proposal_pda, _) = derive_proposal_pda(proposal_id);
    let create_ix = anchor_instruction(
        program_id,
        "create_proposal",
        &create_proposal_data(proposal_id, "Test", "Test", 100),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // First vote
    let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey());
    svm.expire_blockhash();
    let vote_ix = anchor_instruction(
        program_id,
        "cast_vote",
        &cast_vote_data(proposal_id, true),
        vec![
            signer_meta(voter.pubkey()),
            writable_meta(proposal_pda),
            writable_meta(voter_record_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, vote_ix, &voter, &[&voter]).unwrap();

    // Second vote from same voter should fail (PDA already exists)
    svm.expire_blockhash();
    let vote_ix2 = anchor_instruction(
        program_id,
        "cast_vote",
        &cast_vote_data(proposal_id, false), // Try to change vote
        vec![
            signer_meta(voter.pubkey()),
            writable_meta(proposal_pda),
            writable_meta(voter_record_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, vote_ix2, &voter, &[&voter]);
    assert!(result.is_err(), "double voting should fail");
}

#[test]
fn test_vote_after_end_fails() {
    let mut svm = load_voting_program();
    let program_id = voting_program_id();

    let creator = funded_keypair_10_sol(&mut svm);
    let voter = funded_keypair_10_sol(&mut svm);
    let proposal_id: u64 = 1;
    let voting_duration: u64 = 10; // Short duration for testing

    // Create proposal
    let (proposal_pda, _) = derive_proposal_pda(proposal_id);
    let create_ix = anchor_instruction(
        program_id,
        "create_proposal",
        &create_proposal_data(proposal_id, "Test", "Test", voting_duration),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // Warp to after voting period ends
    svm.warp_to_slot(20); // Past end_slot
    svm.expire_blockhash();

    // Try to vote after end
    let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey());
    let vote_ix = anchor_instruction(
        program_id,
        "cast_vote",
        &cast_vote_data(proposal_id, true),
        vec![
            signer_meta(voter.pubkey()),
            writable_meta(proposal_pda),
            writable_meta(voter_record_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, vote_ix, &voter, &[&voter]);
    assert!(result.is_err(), "voting after end should fail");
}

// =============================================================================
// FINALIZE PROPOSAL TESTS
// =============================================================================

#[test]
fn test_finalize_proposal_passed() {
    let mut svm = load_voting_program();
    let program_id = voting_program_id();

    let creator = funded_keypair_10_sol(&mut svm);
    let proposal_id: u64 = 1;
    let voting_duration: u64 = 10;

    // Create proposal
    let (proposal_pda, _) = derive_proposal_pda(proposal_id);
    let create_ix = anchor_instruction(
        program_id,
        "create_proposal",
        &create_proposal_data(proposal_id, "Test", "Test", voting_duration),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // Cast 2 yes, 1 no votes
    for (i, vote_yes) in [true, true, false].iter().enumerate() {
        let voter = funded_keypair_10_sol(&mut svm);
        let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey());

        svm.expire_blockhash();
        let vote_ix = anchor_instruction(
            program_id,
            "cast_vote",
            &cast_vote_data(proposal_id, *vote_yes),
            vec![
                signer_meta(voter.pubkey()),
                writable_meta(proposal_pda),
                writable_meta(voter_record_pda),
                readonly_meta(solana_sdk_ids::sysvar::clock::id()),
                readonly_meta(solana_sdk_ids::sysvar::rent::id()),
                readonly_meta(system_program::id()),
            ],
        );
        execute_tx(&mut svm, vote_ix, &voter, &[&voter]).unwrap();
    }

    // Warp to after voting period
    svm.warp_to_slot(15);
    svm.expire_blockhash();

    // Finalize
    let finalizer = funded_keypair_10_sol(&mut svm);
    let finalize_ix = anchor_instruction(
        program_id,
        "finalize_proposal",
        &[],
        vec![
            signer_meta(finalizer.pubkey()), // finalizer
            writable_meta(proposal_pda),     // proposal
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
        ],
    );

    let result = execute_tx(&mut svm, finalize_ix, &finalizer, &[&finalizer]);
    assert!(result.is_ok(), "finalize_proposal should succeed: {:?}", result);

    // Verify proposal is finalized
    let proposal_account = svm.get_account(&proposal_pda).unwrap();
    assert_eq!(read_proposal_is_finalized(&proposal_account.data), true);
    assert_eq!(read_proposal_yes_votes(&proposal_account.data), 2);
    assert_eq!(read_proposal_no_votes(&proposal_account.data), 1);
}

#[test]
fn test_finalize_proposal_rejected() {
    let mut svm = load_voting_program();
    let program_id = voting_program_id();

    let creator = funded_keypair_10_sol(&mut svm);
    let proposal_id: u64 = 1;
    let voting_duration: u64 = 10;

    // Create proposal
    let (proposal_pda, _) = derive_proposal_pda(proposal_id);
    let create_ix = anchor_instruction(
        program_id,
        "create_proposal",
        &create_proposal_data(proposal_id, "Test", "Test", voting_duration),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // Cast 1 yes, 3 no votes
    for (i, vote_yes) in [true, false, false, false].iter().enumerate() {
        let voter = funded_keypair_10_sol(&mut svm);
        let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey());

        svm.expire_blockhash();
        let vote_ix = anchor_instruction(
            program_id,
            "cast_vote",
            &cast_vote_data(proposal_id, *vote_yes),
            vec![
                signer_meta(voter.pubkey()),
                writable_meta(proposal_pda),
                writable_meta(voter_record_pda),
                readonly_meta(solana_sdk_ids::sysvar::clock::id()),
                readonly_meta(solana_sdk_ids::sysvar::rent::id()),
                readonly_meta(system_program::id()),
            ],
        );
        execute_tx(&mut svm, vote_ix, &voter, &[&voter]).unwrap();
    }

    // Warp to after voting period
    svm.warp_to_slot(15);
    svm.expire_blockhash();

    // Finalize
    let finalizer = funded_keypair_10_sol(&mut svm);
    let finalize_ix = anchor_instruction(
        program_id,
        "finalize_proposal",
        &[],
        vec![
            signer_meta(finalizer.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
    );

    let result = execute_tx(&mut svm, finalize_ix, &finalizer, &[&finalizer]);
    assert!(result.is_ok(), "finalize_proposal should succeed");

    // Verify proposal is finalized with no > yes
    let proposal_account = svm.get_account(&proposal_pda).unwrap();
    assert_eq!(read_proposal_is_finalized(&proposal_account.data), true);
    assert_eq!(read_proposal_yes_votes(&proposal_account.data), 1);
    assert_eq!(read_proposal_no_votes(&proposal_account.data), 3);
}

#[test]
fn test_finalize_before_end_fails() {
    let mut svm = load_voting_program();
    let program_id = voting_program_id();

    let creator = funded_keypair_10_sol(&mut svm);
    let proposal_id: u64 = 1;

    // Create proposal with long voting period
    let (proposal_pda, _) = derive_proposal_pda(proposal_id);
    let create_ix = anchor_instruction(
        program_id,
        "create_proposal",
        &create_proposal_data(proposal_id, "Test", "Test", 1000),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // Try to finalize before voting ends
    svm.expire_blockhash();
    let finalizer = funded_keypair_10_sol(&mut svm);
    let finalize_ix = anchor_instruction(
        program_id,
        "finalize_proposal",
        &[],
        vec![
            signer_meta(finalizer.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
    );

    let result = execute_tx(&mut svm, finalize_ix, &finalizer, &[&finalizer]);
    assert!(result.is_err(), "finalize before end should fail");
}

#[test]
fn test_finalize_already_finalized_fails() {
    let mut svm = load_voting_program();
    let program_id = voting_program_id();

    let creator = funded_keypair_10_sol(&mut svm);
    let proposal_id: u64 = 1;

    // Create proposal
    let (proposal_pda, _) = derive_proposal_pda(proposal_id);
    let create_ix = anchor_instruction(
        program_id,
        "create_proposal",
        &create_proposal_data(proposal_id, "Test", "Test", 10),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // Warp to after voting period
    svm.warp_to_slot(15);
    svm.expire_blockhash();

    // First finalize
    let finalizer = funded_keypair_10_sol(&mut svm);
    let finalize_ix = anchor_instruction(
        program_id,
        "finalize_proposal",
        &[],
        vec![
            signer_meta(finalizer.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
    );
    execute_tx(&mut svm, finalize_ix, &finalizer, &[&finalizer]).unwrap();

    // Second finalize should fail
    svm.expire_blockhash();
    let finalize_ix2 = anchor_instruction(
        program_id,
        "finalize_proposal",
        &[],
        vec![
            signer_meta(finalizer.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
    );

    let result = execute_tx(&mut svm, finalize_ix2, &finalizer, &[&finalizer]);
    assert!(result.is_err(), "double finalize should fail");
}

// =============================================================================
// FULL WORKFLOW TESTS
// =============================================================================

#[test]
fn test_full_voting_workflow() {
    let mut svm = load_voting_program();
    let program_id = voting_program_id();

    let creator = funded_keypair_10_sol(&mut svm);
    let proposal_id: u64 = 42;

    // 1. Create proposal
    let (proposal_pda, _) = derive_proposal_pda(proposal_id);
    let create_ix = anchor_instruction(
        program_id,
        "create_proposal",
        &create_proposal_data(proposal_id, "Fund Development", "Allocate 1000 SOL to development team", 50),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // 2. Cast 5 yes votes, 3 no votes
    let votes = vec![true, true, true, false, true, false, true, false];
    for vote_yes in votes {
        let voter = funded_keypair_10_sol(&mut svm);
        let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey());

        svm.expire_blockhash();
        let vote_ix = anchor_instruction(
            program_id,
            "cast_vote",
            &cast_vote_data(proposal_id, vote_yes),
            vec![
                signer_meta(voter.pubkey()),
                writable_meta(proposal_pda),
                writable_meta(voter_record_pda),
                readonly_meta(solana_sdk_ids::sysvar::clock::id()),
                readonly_meta(solana_sdk_ids::sysvar::rent::id()),
                readonly_meta(system_program::id()),
            ],
        );
        execute_tx(&mut svm, vote_ix, &voter, &[&voter]).unwrap();
    }

    // 3. Verify vote counts before finalization
    let proposal_account = svm.get_account(&proposal_pda).unwrap();
    assert_eq!(read_proposal_yes_votes(&proposal_account.data), 5);
    assert_eq!(read_proposal_no_votes(&proposal_account.data), 3);
    assert_eq!(read_proposal_is_finalized(&proposal_account.data), false);

    // 4. Warp to after voting period and finalize
    svm.warp_to_slot(100);
    svm.expire_blockhash();

    let finalizer = funded_keypair_10_sol(&mut svm);
    let finalize_ix = anchor_instruction(
        program_id,
        "finalize_proposal",
        &[],
        vec![
            signer_meta(finalizer.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
    );
    execute_tx(&mut svm, finalize_ix, &finalizer, &[&finalizer]).unwrap();

    // 5. Verify final state
    let proposal_account = svm.get_account(&proposal_pda).unwrap();
    assert_eq!(read_proposal_is_finalized(&proposal_account.data), true);
    assert_eq!(read_proposal_yes_votes(&proposal_account.data), 5);
    assert_eq!(read_proposal_no_votes(&proposal_account.data), 3);
}

#[test]
fn test_pda_derivation_deterministic() {
    let proposal_id: u64 = 123;
    let voter = Pubkey::new_unique();

    // Derive multiple times
    let (pda1, bump1) = derive_proposal_pda(proposal_id);
    let (pda2, bump2) = derive_proposal_pda(proposal_id);
    let (voter_pda1, voter_bump1) = derive_voter_record_pda(proposal_id, &voter);
    let (voter_pda2, voter_bump2) = derive_voter_record_pda(proposal_id, &voter);

    // Should be identical
    assert_eq!(pda1, pda2);
    assert_eq!(bump1, bump2);
    assert_eq!(voter_pda1, voter_pda2);
    assert_eq!(voter_bump1, voter_bump2);

    // Different seeds should produce different PDAs
    let (other_pda, _) = derive_proposal_pda(456);
    assert_ne!(pda1, other_pda);

    let other_voter = Pubkey::new_unique();
    let (other_voter_pda, _) = derive_voter_record_pda(proposal_id, &other_voter);
    assert_ne!(voter_pda1, other_voter_pda);
}
