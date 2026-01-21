//! LiteSVM integration tests for the Seahorse Token-Weighted Governance program
//!
//! These tests verify token-weighted governance mechanics:
//! - initialize_governance: Creates governance config for a token
//! - create_proposal: Creates proposals with quorum requirements
//! - cast_vote: Records votes weighted by token balance
//! - finalize_proposal: Finalizes with quorum check
//!
//! Uses LiteSVM's warp_to_slot for time-based voting tests.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile governance.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// Governance program ID (from declare_id!)
fn governance_program_id() -> Pubkey {
    Pubkey::from_str("GovToken11111111111111111111111111111111111").unwrap()
}

/// Result codes
const RESULT_PENDING: u8 = 0;
const RESULT_PASSED: u8 = 1;
const RESULT_REJECTED: u8 = 2;
const RESULT_QUORUM_NOT_MET: u8 = 3;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the governance program into LiteSVM
fn load_governance_program() -> litesvm::LiteSVM {
    let program_id = governance_program_id();
    let program_bytes = std::fs::read("../../target/deploy/governance.so")
        .expect("Failed to read governance.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);
    svm
}

/// Setup a mint for testing
fn setup_mint(svm: &mut litesvm::LiteSVM, authority: &Pubkey) -> Pubkey {
    let mint = Keypair::new();
    let mint_account = create_mint_account(authority, 9);
    svm.set_account(mint.pubkey(), mint_account).unwrap();
    mint.pubkey()
}

/// Setup a user with a token account containing tokens
fn setup_user_with_tokens(
    svm: &mut litesvm::LiteSVM,
    mint: &Pubkey,
    amount: u64,
) -> (Keypair, Pubkey) {
    let user = funded_keypair_10_sol(svm);
    let user_ata = get_ata(&user.pubkey(), mint);
    let token_account = create_token_account(&user.pubkey(), mint, amount);
    svm.set_account(user_ata, token_account).unwrap();
    (user, user_ata)
}

/// Derive governance config PDA
fn derive_governance_config_pda(mint: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"governance", mint.as_ref()],
        &governance_program_id(),
    )
}

/// Derive proposal PDA
fn derive_proposal_pda(proposal_id: u64) -> (Pubkey, u8) {
    find_pda(
        &[b"proposal", proposal_id.to_le_bytes().as_ref()],
        &governance_program_id(),
    )
}

/// Derive voter record PDA
fn derive_voter_record_pda(proposal_id: u64, voter: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"voter_record", proposal_id.to_le_bytes().as_ref(), voter.as_ref()],
        &governance_program_id(),
    )
}

/// Build initialize_governance instruction data
fn initialize_governance_data(proposal_threshold: u64, default_voting_duration: u64) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&proposal_threshold.to_le_bytes());
    data.extend_from_slice(&default_voting_duration.to_le_bytes());
    data
}

/// Build create_proposal instruction data
fn create_proposal_data(
    proposal_id: u64,
    title: &str,
    description: &str,
    quorum: u64,
    voting_duration_slots: u64,
) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&proposal_id.to_le_bytes());
    // title: String (length prefix + bytes)
    data.extend_from_slice(&(title.len() as u32).to_le_bytes());
    data.extend_from_slice(title.as_bytes());
    // description: String (length prefix + bytes)
    data.extend_from_slice(&(description.len() as u32).to_le_bytes());
    data.extend_from_slice(description.as_bytes());
    data.extend_from_slice(&quorum.to_le_bytes());
    data.extend_from_slice(&voting_duration_slots.to_le_bytes());
    data
}

/// Build cast_vote instruction data
fn cast_vote_data(proposal_id: u64, vote_yes: bool) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&proposal_id.to_le_bytes());
    data.push(if vote_yes { 1 } else { 0 });
    data
}

/// Read governance config fields
fn read_config_authority(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

fn read_config_mint(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[40..72].try_into().unwrap())
}

fn read_config_proposal_threshold(data: &[u8]) -> u64 {
    u64::from_le_bytes(data[72..80].try_into().unwrap())
}

fn read_config_default_voting_duration(data: &[u8]) -> u64 {
    u64::from_le_bytes(data[80..88].try_into().unwrap())
}

fn read_config_next_proposal_id(data: &[u8]) -> u64 {
    u64::from_le_bytes(data[88..96].try_into().unwrap())
}

/// Read proposal fields - governance(32), creator(32), proposal_id(8), then variable-length strings
fn read_proposal_governance(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

fn read_proposal_creator(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[40..72].try_into().unwrap())
}

fn read_proposal_id(data: &[u8]) -> u64 {
    u64::from_le_bytes(data[72..80].try_into().unwrap())
}

fn read_proposal_title(data: &[u8]) -> String {
    let len = u32::from_le_bytes(data[80..84].try_into().unwrap()) as usize;
    String::from_utf8(data[84..84 + len].to_vec()).unwrap()
}

/// Get offset after title string
fn get_description_offset(data: &[u8]) -> usize {
    let title_len = u32::from_le_bytes(data[80..84].try_into().unwrap()) as usize;
    84 + title_len
}

fn read_proposal_description(data: &[u8]) -> String {
    let offset = get_description_offset(data);
    let len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
    String::from_utf8(data[offset + 4..offset + 4 + len].to_vec()).unwrap()
}

/// Get offset after description string
fn get_votes_offset(data: &[u8]) -> usize {
    let desc_offset = get_description_offset(data);
    let desc_len = u32::from_le_bytes(data[desc_offset..desc_offset + 4].try_into().unwrap()) as usize;
    desc_offset + 4 + desc_len
}

fn read_proposal_yes_votes(data: &[u8]) -> u64 {
    let offset = get_votes_offset(data);
    u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap())
}

fn read_proposal_no_votes(data: &[u8]) -> u64 {
    let offset = get_votes_offset(data) + 8;
    u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap())
}

fn read_proposal_quorum(data: &[u8]) -> u64 {
    let offset = get_votes_offset(data) + 16;
    u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap())
}

fn read_proposal_is_finalized(data: &[u8]) -> bool {
    let offset = get_votes_offset(data) + 40;
    data[offset] != 0
}

fn read_proposal_result(data: &[u8]) -> u8 {
    let offset = get_votes_offset(data) + 41;
    data[offset]
}

/// Read voter record fields
fn read_voter_record_voter(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

fn read_voter_record_vote_yes(data: &[u8]) -> bool {
    data[72] != 0
}

fn read_voter_record_voting_power(data: &[u8]) -> u64 {
    u64::from_le_bytes(data[73..81].try_into().unwrap())
}

// =============================================================================
// INITIALIZE GOVERNANCE TESTS
// =============================================================================

#[test]
fn test_initialize_governance() {
    let mut svm = load_governance_program();
    let program_id = governance_program_id();

    // Setup authority
    let authority = funded_keypair_10_sol(&mut svm);
    let mint_pubkey = setup_mint(&mut svm, &authority.pubkey());

    // Derive governance config PDA
    let (config_pda, _) = derive_governance_config_pda(&mint_pubkey);

    // Build initialize_governance instruction
    let proposal_threshold: u64 = 1000;
    let default_voting_duration: u64 = 100;

    let ix = anchor_instruction(
        program_id,
        "initialize_governance",
        &initialize_governance_data(proposal_threshold, default_voting_duration),
        vec![
            signer_meta(authority.pubkey()),     // authority
            writable_meta(mint_pubkey),           // governance_mint
            writable_meta(config_pda),            // config
            readonly_meta(solana_sdk_ids::sysvar::rent::id()), // rent
            readonly_meta(system_program::id()), // system_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "initialize_governance should succeed: {:?}", result);

    // Verify config state
    let config_account = svm.get_account(&config_pda).expect("Config should exist");
    assert_eq!(config_account.owner, program_id);
    assert_eq!(read_config_authority(&config_account.data), authority.pubkey());
    assert_eq!(read_config_mint(&config_account.data), mint_pubkey);
    assert_eq!(read_config_proposal_threshold(&config_account.data), proposal_threshold);
    assert_eq!(read_config_default_voting_duration(&config_account.data), default_voting_duration);
    assert_eq!(read_config_next_proposal_id(&config_account.data), 1);
}

#[test]
fn test_initialize_governance_zero_threshold_fails() {
    let mut svm = load_governance_program();
    let program_id = governance_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let mint_pubkey = setup_mint(&mut svm, &authority.pubkey());
    let (config_pda, _) = derive_governance_config_pda(&mint_pubkey);

    let ix = anchor_instruction(
        program_id,
        "initialize_governance",
        &initialize_governance_data(0, 100), // zero threshold
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint_pubkey),
            writable_meta(config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "initialize_governance with zero threshold should fail");
}

#[test]
fn test_initialize_governance_zero_duration_fails() {
    let mut svm = load_governance_program();
    let program_id = governance_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let mint_pubkey = setup_mint(&mut svm, &authority.pubkey());
    let (config_pda, _) = derive_governance_config_pda(&mint_pubkey);

    let ix = anchor_instruction(
        program_id,
        "initialize_governance",
        &initialize_governance_data(1000, 0), // zero duration
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint_pubkey),
            writable_meta(config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "initialize_governance with zero duration should fail");
}

// =============================================================================
// CREATE PROPOSAL TESTS
// =============================================================================

/// Helper to setup governance and return (authority, mint, config_pda)
fn setup_governance(svm: &mut litesvm::LiteSVM) -> (Keypair, Pubkey, Pubkey) {
    let program_id = governance_program_id();
    let authority = funded_keypair_10_sol(svm);
    let mint_pubkey = setup_mint(svm, &authority.pubkey());
    let (config_pda, _) = derive_governance_config_pda(&mint_pubkey);

    let ix = anchor_instruction(
        program_id,
        "initialize_governance",
        &initialize_governance_data(100, 100), // low threshold for testing
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint_pubkey),
            writable_meta(config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(svm, ix, &authority, &[&authority]).unwrap();

    (authority, mint_pubkey, config_pda)
}

#[test]
fn test_create_proposal() {
    let mut svm = load_governance_program();
    let program_id = governance_program_id();

    let (authority, mint_pubkey, config_pda) = setup_governance(&mut svm);

    // Setup creator with tokens (using ATA)
    let (creator, creator_ata) = setup_user_with_tokens(&mut svm, &mint_pubkey, 1000);

    // Create proposal
    svm.expire_blockhash();
    let proposal_id: u64 = 1;
    let title = "Test Proposal";
    let description = "A governance proposal for testing";
    let quorum: u64 = 500;
    let voting_duration: u64 = 50;

    let (proposal_pda, _) = derive_proposal_pda(proposal_id);
    let create_ix = anchor_instruction(
        program_id,
        "create_proposal",
        &create_proposal_data(proposal_id, title, description, quorum, voting_duration),
        vec![
            signer_meta(creator.pubkey()),         // creator
            writable_meta(creator_ata),            // creator_token_account (must be mut)
            writable_meta(config_pda),              // config
            writable_meta(proposal_pda),            // proposal
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),  // rent
            readonly_meta(system_program::id()),   // system_program
        ],
    );

    let result = execute_tx(&mut svm, create_ix, &creator, &[&creator]);
    assert!(result.is_ok(), "create_proposal should succeed: {:?}", result);

    // Verify proposal state
    let proposal_account = svm.get_account(&proposal_pda).expect("Proposal should exist");
    assert_eq!(read_proposal_governance(&proposal_account.data), config_pda);
    assert_eq!(read_proposal_creator(&proposal_account.data), creator.pubkey());
    assert_eq!(read_proposal_id(&proposal_account.data), proposal_id);
    assert_eq!(read_proposal_title(&proposal_account.data), title);
    assert_eq!(read_proposal_description(&proposal_account.data), description);
    assert_eq!(read_proposal_yes_votes(&proposal_account.data), 0);
    assert_eq!(read_proposal_no_votes(&proposal_account.data), 0);
    assert_eq!(read_proposal_quorum(&proposal_account.data), quorum);
    assert_eq!(read_proposal_is_finalized(&proposal_account.data), false);
    assert_eq!(read_proposal_result(&proposal_account.data), RESULT_PENDING);

    // Verify next_proposal_id incremented
    let config_account = svm.get_account(&config_pda).unwrap();
    assert_eq!(read_config_next_proposal_id(&config_account.data), 2);
}

#[test]
fn test_create_proposal_insufficient_tokens_fails() {
    let mut svm = load_governance_program();
    let program_id = governance_program_id();

    // Initialize with high threshold
    let authority = funded_keypair_10_sol(&mut svm);
    let mint_pubkey = setup_mint(&mut svm, &authority.pubkey());
    let (config_pda, _) = derive_governance_config_pda(&mint_pubkey);

    let init_ix = anchor_instruction(
        program_id,
        "initialize_governance",
        &initialize_governance_data(1000, 100), // threshold 1000
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint_pubkey),
            writable_meta(config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Creator with only 500 tokens (below threshold)
    let (creator, creator_ata) = setup_user_with_tokens(&mut svm, &mint_pubkey, 500);

    svm.expire_blockhash();
    let (proposal_pda, _) = derive_proposal_pda(1);
    let create_ix = anchor_instruction(
        program_id,
        "create_proposal",
        &create_proposal_data(1, "Test", "Test", 100, 50),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(creator_ata),  // must be mut
            writable_meta(config_pda),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, create_ix, &creator, &[&creator]);
    assert!(result.is_err(), "create_proposal with insufficient tokens should fail");
}

// =============================================================================
// CAST VOTE TESTS
// =============================================================================

/// Helper to setup governance and create a proposal
fn setup_governance_with_proposal(svm: &mut litesvm::LiteSVM) -> (Keypair, Pubkey, Pubkey, Pubkey, u64) {
    let program_id = governance_program_id();
    let (authority, mint_pubkey, config_pda) = setup_governance(svm);

    // Setup creator with tokens
    let (creator, creator_ata) = setup_user_with_tokens(svm, &mint_pubkey, 1000);

    // Create proposal
    svm.expire_blockhash();
    let proposal_id: u64 = 1;
    let (proposal_pda, _) = derive_proposal_pda(proposal_id);
    let create_ix = anchor_instruction(
        program_id,
        "create_proposal",
        &create_proposal_data(proposal_id, "Test Proposal", "For testing", 500, 50),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(creator_ata),  // must be mut
            writable_meta(config_pda),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(svm, create_ix, &creator, &[&creator]).unwrap();

    (authority, mint_pubkey, config_pda, proposal_pda, proposal_id)
}

#[test]
fn test_cast_vote_yes_weighted() {
    let mut svm = load_governance_program();
    let program_id = governance_program_id();

    let (_authority, mint_pubkey, config_pda, proposal_pda, proposal_id) = setup_governance_with_proposal(&mut svm);

    // Setup voter with 300 tokens
    let (voter, voter_ata) = setup_user_with_tokens(&mut svm, &mint_pubkey, 300);

    // Cast vote
    svm.expire_blockhash();
    let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey());
    let vote_ix = anchor_instruction(
        program_id,
        "cast_vote",
        &cast_vote_data(proposal_id, true),
        vec![
            signer_meta(voter.pubkey()),          // voter
            writable_meta(voter_ata),             // voter_token_account (must be mut)
            writable_meta(config_pda),             // config (must be mut)
            writable_meta(proposal_pda),           // proposal
            writable_meta(voter_record_pda),       // voter_record
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),  // rent
            readonly_meta(system_program::id()),  // system_program
        ],
    );

    let result = execute_tx(&mut svm, vote_ix, &voter, &[&voter]);
    assert!(result.is_ok(), "cast_vote should succeed: {:?}", result);

    // Verify weighted vote
    let proposal_account = svm.get_account(&proposal_pda).unwrap();
    assert_eq!(read_proposal_yes_votes(&proposal_account.data), 300); // Weighted by token balance
    assert_eq!(read_proposal_no_votes(&proposal_account.data), 0);

    // Verify voter record
    let voter_record = svm.get_account(&voter_record_pda).expect("Voter record should exist");
    assert_eq!(read_voter_record_voter(&voter_record.data), voter.pubkey());
    assert_eq!(read_voter_record_vote_yes(&voter_record.data), true);
    assert_eq!(read_voter_record_voting_power(&voter_record.data), 300);
}

#[test]
fn test_cast_vote_no_weighted() {
    let mut svm = load_governance_program();
    let program_id = governance_program_id();

    let (_authority, mint_pubkey, config_pda, proposal_pda, proposal_id) = setup_governance_with_proposal(&mut svm);

    // Setup voter with 200 tokens
    let (voter, voter_ata) = setup_user_with_tokens(&mut svm, &mint_pubkey, 200);

    // Cast no vote
    svm.expire_blockhash();
    let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey());
    let vote_ix = anchor_instruction(
        program_id,
        "cast_vote",
        &cast_vote_data(proposal_id, false),
        vec![
            signer_meta(voter.pubkey()),
            writable_meta(voter_ata),     // must be mut
            writable_meta(config_pda),     // must be mut
            writable_meta(proposal_pda),
            writable_meta(voter_record_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, vote_ix, &voter, &[&voter]);
    assert!(result.is_ok(), "cast_vote (no) should succeed: {:?}", result);

    let proposal_account = svm.get_account(&proposal_pda).unwrap();
    assert_eq!(read_proposal_yes_votes(&proposal_account.data), 0);
    assert_eq!(read_proposal_no_votes(&proposal_account.data), 200);
}

#[test]
fn test_cast_vote_multiple_voters_weighted() {
    let mut svm = load_governance_program();
    let program_id = governance_program_id();

    let (_authority, mint_pubkey, config_pda, proposal_pda, proposal_id) = setup_governance_with_proposal(&mut svm);

    // Setup multiple voters with different token amounts
    let voters_and_votes: Vec<(u64, bool)> = vec![
        (100, true),  // 100 tokens, yes
        (250, true),  // 250 tokens, yes
        (400, false), // 400 tokens, no
    ];

    for (token_amount, vote_yes) in &voters_and_votes {
        let (voter, voter_ata) = setup_user_with_tokens(&mut svm, &mint_pubkey, *token_amount);

        svm.expire_blockhash();
        let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey());
        let vote_ix = anchor_instruction(
            program_id,
            "cast_vote",
            &cast_vote_data(proposal_id, *vote_yes),
            vec![
                signer_meta(voter.pubkey()),
                writable_meta(voter_ata),     // must be mut
                writable_meta(config_pda),     // must be mut
                writable_meta(proposal_pda),
                writable_meta(voter_record_pda),
                readonly_meta(solana_sdk_ids::sysvar::clock::id()),
                readonly_meta(solana_sdk_ids::sysvar::rent::id()),
                readonly_meta(system_program::id()),
            ],
        );
        execute_tx(&mut svm, vote_ix, &voter, &[&voter]).unwrap();
    }

    // Verify weighted totals: yes=350 (100+250), no=400
    let proposal_account = svm.get_account(&proposal_pda).unwrap();
    assert_eq!(read_proposal_yes_votes(&proposal_account.data), 350);
    assert_eq!(read_proposal_no_votes(&proposal_account.data), 400);
}

#[test]
fn test_double_vote_fails() {
    let mut svm = load_governance_program();
    let program_id = governance_program_id();

    let (_authority, mint_pubkey, config_pda, proposal_pda, proposal_id) = setup_governance_with_proposal(&mut svm);

    // Setup voter
    let (voter, voter_ata) = setup_user_with_tokens(&mut svm, &mint_pubkey, 100);

    // First vote
    svm.expire_blockhash();
    let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey());
    let vote_ix = anchor_instruction(
        program_id,
        "cast_vote",
        &cast_vote_data(proposal_id, true),
        vec![
            signer_meta(voter.pubkey()),
            writable_meta(voter_ata),     // must be mut
            writable_meta(config_pda),     // must be mut
            writable_meta(proposal_pda),
            writable_meta(voter_record_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, vote_ix, &voter, &[&voter]).unwrap();

    // Second vote should fail
    svm.expire_blockhash();
    let vote_ix2 = anchor_instruction(
        program_id,
        "cast_vote",
        &cast_vote_data(proposal_id, false),
        vec![
            signer_meta(voter.pubkey()),
            writable_meta(voter_ata),     // must be mut
            writable_meta(config_pda),     // must be mut
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
fn test_vote_zero_balance_fails() {
    let mut svm = load_governance_program();
    let program_id = governance_program_id();

    let (_authority, mint_pubkey, config_pda, proposal_pda, proposal_id) = setup_governance_with_proposal(&mut svm);

    // Setup voter with zero tokens
    let (voter, voter_ata) = setup_user_with_tokens(&mut svm, &mint_pubkey, 0);

    svm.expire_blockhash();
    let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey());
    let vote_ix = anchor_instruction(
        program_id,
        "cast_vote",
        &cast_vote_data(proposal_id, true),
        vec![
            signer_meta(voter.pubkey()),
            writable_meta(voter_ata),     // must be mut
            writable_meta(config_pda),     // must be mut
            writable_meta(proposal_pda),
            writable_meta(voter_record_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, vote_ix, &voter, &[&voter]);
    assert!(result.is_err(), "voting with zero balance should fail");
}

#[test]
fn test_vote_after_end_fails() {
    let mut svm = load_governance_program();
    let program_id = governance_program_id();

    let (_authority, mint_pubkey, config_pda, proposal_pda, proposal_id) = setup_governance_with_proposal(&mut svm);

    // Warp past voting period
    svm.warp_to_slot(100);
    svm.expire_blockhash();

    // Setup voter
    let (voter, voter_ata) = setup_user_with_tokens(&mut svm, &mint_pubkey, 100);

    let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey());
    let vote_ix = anchor_instruction(
        program_id,
        "cast_vote",
        &cast_vote_data(proposal_id, true),
        vec![
            signer_meta(voter.pubkey()),
            writable_meta(voter_ata),     // must be mut
            writable_meta(config_pda),     // must be mut
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
fn test_finalize_proposal_passed_with_quorum() {
    let mut svm = load_governance_program();
    let program_id = governance_program_id();

    let (_authority, mint_pubkey, config_pda, proposal_pda, proposal_id) = setup_governance_with_proposal(&mut svm);

    // Cast votes exceeding quorum (500): 600 yes, 200 no
    for (amount, vote_yes) in [(300u64, true), (300u64, true), (200u64, false)] {
        let (voter, voter_ata) = setup_user_with_tokens(&mut svm, &mint_pubkey, amount);

        svm.expire_blockhash();
        let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey());
        let vote_ix = anchor_instruction(
            program_id,
            "cast_vote",
            &cast_vote_data(proposal_id, vote_yes),
            vec![
                signer_meta(voter.pubkey()),
                writable_meta(voter_ata),     // must be mut
                writable_meta(config_pda),     // must be mut
                writable_meta(proposal_pda),
                writable_meta(voter_record_pda),
                readonly_meta(solana_sdk_ids::sysvar::clock::id()),
                readonly_meta(solana_sdk_ids::sysvar::rent::id()),
                readonly_meta(system_program::id()),
            ],
        );
        execute_tx(&mut svm, vote_ix, &voter, &[&voter]).unwrap();
    }

    // Warp to after voting and finalize
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

    let result = execute_tx(&mut svm, finalize_ix, &finalizer, &[&finalizer]);
    assert!(result.is_ok(), "finalize_proposal should succeed: {:?}", result);

    // Verify result
    let proposal_account = svm.get_account(&proposal_pda).unwrap();
    assert_eq!(read_proposal_is_finalized(&proposal_account.data), true);
    assert_eq!(read_proposal_result(&proposal_account.data), RESULT_PASSED);
    assert_eq!(read_proposal_yes_votes(&proposal_account.data), 600);
    assert_eq!(read_proposal_no_votes(&proposal_account.data), 200);
}

#[test]
fn test_finalize_proposal_rejected() {
    let mut svm = load_governance_program();
    let program_id = governance_program_id();

    let (_authority, mint_pubkey, config_pda, proposal_pda, proposal_id) = setup_governance_with_proposal(&mut svm);

    // Cast votes: 200 yes, 400 no (total 600 > quorum 500)
    for (amount, vote_yes) in [(200u64, true), (400u64, false)] {
        let (voter, voter_ata) = setup_user_with_tokens(&mut svm, &mint_pubkey, amount);

        svm.expire_blockhash();
        let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey());
        let vote_ix = anchor_instruction(
            program_id,
            "cast_vote",
            &cast_vote_data(proposal_id, vote_yes),
            vec![
                signer_meta(voter.pubkey()),
                writable_meta(voter_ata),     // must be mut
                writable_meta(config_pda),     // must be mut
                writable_meta(proposal_pda),
                writable_meta(voter_record_pda),
                readonly_meta(solana_sdk_ids::sysvar::clock::id()),
                readonly_meta(solana_sdk_ids::sysvar::rent::id()),
                readonly_meta(system_program::id()),
            ],
        );
        execute_tx(&mut svm, vote_ix, &voter, &[&voter]).unwrap();
    }

    // Finalize
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

    let proposal_account = svm.get_account(&proposal_pda).unwrap();
    assert_eq!(read_proposal_is_finalized(&proposal_account.data), true);
    assert_eq!(read_proposal_result(&proposal_account.data), RESULT_REJECTED);
}

#[test]
fn test_finalize_proposal_quorum_not_met() {
    let mut svm = load_governance_program();
    let program_id = governance_program_id();

    let (_authority, mint_pubkey, config_pda, proposal_pda, proposal_id) = setup_governance_with_proposal(&mut svm);

    // Cast only 100 tokens (below quorum of 500)
    let (voter, voter_ata) = setup_user_with_tokens(&mut svm, &mint_pubkey, 100);

    svm.expire_blockhash();
    let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey());
    let vote_ix = anchor_instruction(
        program_id,
        "cast_vote",
        &cast_vote_data(proposal_id, true),
        vec![
            signer_meta(voter.pubkey()),
            writable_meta(voter_ata),     // must be mut
            writable_meta(config_pda),     // must be mut
            writable_meta(proposal_pda),
            writable_meta(voter_record_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, vote_ix, &voter, &[&voter]).unwrap();

    // Finalize
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

    let proposal_account = svm.get_account(&proposal_pda).unwrap();
    assert_eq!(read_proposal_is_finalized(&proposal_account.data), true);
    assert_eq!(read_proposal_result(&proposal_account.data), RESULT_QUORUM_NOT_MET);
}

#[test]
fn test_finalize_before_end_fails() {
    let mut svm = load_governance_program();
    let program_id = governance_program_id();

    let (_authority, _mint_pubkey, _config_pda, proposal_pda, _proposal_id) = setup_governance_with_proposal(&mut svm);

    // Don't warp, try to finalize immediately
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
    assert!(result.is_err(), "finalize before voting ends should fail");
}

#[test]
fn test_finalize_already_finalized_fails() {
    let mut svm = load_governance_program();
    let program_id = governance_program_id();

    let (_authority, _mint_pubkey, _config_pda, proposal_pda, _proposal_id) = setup_governance_with_proposal(&mut svm);

    // Warp and finalize
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

    // Try to finalize again
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
fn test_full_governance_workflow() {
    let mut svm = load_governance_program();
    let program_id = governance_program_id();

    // 1. Initialize governance
    let (authority, mint_pubkey, config_pda) = setup_governance(&mut svm);

    // 2. Create proposal
    let (creator, creator_ata) = setup_user_with_tokens(&mut svm, &mint_pubkey, 500);

    svm.expire_blockhash();
    let proposal_id: u64 = 1;
    let (proposal_pda, _) = derive_proposal_pda(proposal_id);
    let create_ix = anchor_instruction(
        program_id,
        "create_proposal",
        &create_proposal_data(proposal_id, "Upgrade Protocol", "Proposal to upgrade", 500, 50),
        vec![
            signer_meta(creator.pubkey()),
            writable_meta(creator_ata),  // must be mut
            writable_meta(config_pda),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &creator, &[&creator]).unwrap();

    // 3. Cast votes: 700 yes > 500 quorum
    let vote_configs: Vec<(u64, bool)> = vec![
        (200, true),
        (250, true),
        (250, true),
    ];

    for (amount, vote_yes) in vote_configs {
        let (voter, voter_ata) = setup_user_with_tokens(&mut svm, &mint_pubkey, amount);

        svm.expire_blockhash();
        let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey());
        let vote_ix = anchor_instruction(
            program_id,
            "cast_vote",
            &cast_vote_data(proposal_id, vote_yes),
            vec![
                signer_meta(voter.pubkey()),
                writable_meta(voter_ata),     // must be mut
                writable_meta(config_pda),     // must be mut
                writable_meta(proposal_pda),
                writable_meta(voter_record_pda),
                readonly_meta(solana_sdk_ids::sysvar::clock::id()),
                readonly_meta(solana_sdk_ids::sysvar::rent::id()),
                readonly_meta(system_program::id()),
            ],
        );
        execute_tx(&mut svm, vote_ix, &voter, &[&voter]).unwrap();
    }

    // 4. Verify intermediate state
    let proposal_account = svm.get_account(&proposal_pda).unwrap();
    assert_eq!(read_proposal_yes_votes(&proposal_account.data), 700);
    assert_eq!(read_proposal_no_votes(&proposal_account.data), 0);
    assert_eq!(read_proposal_is_finalized(&proposal_account.data), false);

    // 5. Warp and finalize
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

    // 6. Verify final state
    let proposal_account = svm.get_account(&proposal_pda).unwrap();
    assert_eq!(read_proposal_is_finalized(&proposal_account.data), true);
    assert_eq!(read_proposal_result(&proposal_account.data), RESULT_PASSED);
}

#[test]
fn test_pda_derivation_deterministic() {
    let mint = Pubkey::new_unique();
    let proposal_id: u64 = 42;
    let voter = Pubkey::new_unique();

    // Derive multiple times
    let (config_pda1, _) = derive_governance_config_pda(&mint);
    let (config_pda2, _) = derive_governance_config_pda(&mint);
    let (proposal_pda1, _) = derive_proposal_pda(proposal_id);
    let (proposal_pda2, _) = derive_proposal_pda(proposal_id);
    let (voter_pda1, _) = derive_voter_record_pda(proposal_id, &voter);
    let (voter_pda2, _) = derive_voter_record_pda(proposal_id, &voter);

    // Should be identical
    assert_eq!(config_pda1, config_pda2);
    assert_eq!(proposal_pda1, proposal_pda2);
    assert_eq!(voter_pda1, voter_pda2);

    // Different inputs should produce different PDAs
    let other_mint = Pubkey::new_unique();
    let (other_config, _) = derive_governance_config_pda(&other_mint);
    assert_ne!(config_pda1, other_config);
}
