//! Token-Weighted Governance tests: Seahorse implementation behavior verification
//!
//! These tests verify the Seahorse governance implementation produces
//! consistent behavior for:
//! - Governance config initialization
//! - Proposal creation with token threshold
//! - Weighted vote tallying (voting power = token balance)
//! - Quorum enforcement
//! - Finalization mechanics
//! - Deterministic PDA derivation
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile:
//! - target/deploy/governance.so (Seahorse)

use seahorse_test_common::*;
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use solana_sdk_ids::system_program;
use solana_sha256_hasher::hash;
use solana_signer::Signer;
use solana_transaction::Transaction;
use spl_token::solana_program::program_option::COption;
use spl_token::solana_program::program_pack::Pack;
use std::str::FromStr;

/// SPL Token Mint account size
const MINT_SIZE: usize = 82;
/// SPL Token Account size
const TOKEN_ACCOUNT_SIZE: usize = 165;

/// Governance program ID
fn program_id() -> Pubkey {
    Pubkey::from_str("GovToken11111111111111111111111111111111111").unwrap()
}

/// Create a mint account for testing
fn create_mint_account(authority: &Pubkey, decimals: u8) -> Account {
    let rent = Rent::default();

    let mint = spl_token::state::Mint {
        mint_authority: COption::Some(*authority),
        supply: 0,
        decimals,
        is_initialized: true,
        freeze_authority: COption::None,
    };

    let mut data = vec![0u8; MINT_SIZE];
    spl_token::state::Mint::pack(mint, &mut data).unwrap();

    Account {
        lamports: rent.minimum_balance(MINT_SIZE),
        data,
        owner: spl_token::id(),
        executable: false,
        rent_epoch: 0,
    }
}

/// Create a token account for testing
fn create_token_account(owner: &Pubkey, mint: &Pubkey, amount: u64) -> Account {
    let rent = Rent::default();

    let token_account = spl_token::state::Account {
        mint: *mint,
        owner: *owner,
        amount,
        delegate: COption::None,
        state: spl_token::state::AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };

    let mut data = vec![0u8; TOKEN_ACCOUNT_SIZE];
    spl_token::state::Account::pack(token_account, &mut data).unwrap();

    Account {
        lamports: rent.minimum_balance(TOKEN_ACCOUNT_SIZE),
        data,
        owner: spl_token::id(),
        executable: false,
        rent_epoch: 0,
    }
}

/// Derive governance config PDA
fn derive_governance_config_pda(mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"governance", mint.as_ref()],
        program_id,
    )
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

/// Build initialize_governance instruction data
fn initialize_governance_data(proposal_threshold: u64, default_voting_duration: u64) -> Vec<u8> {
    let mut data = instruction_disc("initialize_governance").to_vec();
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
    let mut data = instruction_disc("create_proposal").to_vec();
    data.extend_from_slice(&proposal_id.to_le_bytes());
    data.extend_from_slice(&(title.len() as u32).to_le_bytes());
    data.extend_from_slice(title.as_bytes());
    data.extend_from_slice(&(description.len() as u32).to_le_bytes());
    data.extend_from_slice(description.as_bytes());
    data.extend_from_slice(&quorum.to_le_bytes());
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

/// Read governance config fields
fn read_config_proposal_threshold(data: &[u8]) -> u64 {
    u64::from_le_bytes(data[72..80].try_into().unwrap())
}

fn read_config_next_proposal_id(data: &[u8]) -> u64 {
    u64::from_le_bytes(data[88..96].try_into().unwrap())
}

/// Get offset to votes section in proposal data (after variable-length strings)
fn get_votes_offset(data: &[u8]) -> usize {
    let title_len = u32::from_le_bytes(data[80..84].try_into().unwrap()) as usize;
    let desc_offset = 84 + title_len;
    let desc_len = u32::from_le_bytes(data[desc_offset..desc_offset + 4].try_into().unwrap()) as usize;
    desc_offset + 4 + desc_len
}

/// Read proposal yes_votes from account data
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

fn read_voter_record_voting_power(data: &[u8]) -> u64 {
    u64::from_le_bytes(data[73..81].try_into().unwrap())
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
fn test_parity_governance_initialization() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/governance.so")
        .expect("Failed to read governance.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup authority
    let authority = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    // Create mint
    let mint = Keypair::new();
    svm.set_account(mint.pubkey(), create_mint_account(&authority.pubkey(), 9)).unwrap();

    let (config_pda, _) = derive_governance_config_pda(&mint.pubkey(), &program_id);

    let proposal_threshold: u64 = 1000;
    let default_voting_duration: u64 = 100;

    // Initialize governance
    let ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint.pubkey()),
            writable_meta(config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: initialize_governance_data(proposal_threshold, default_voting_duration),
    };

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "initialize_governance should succeed: {:?}", result);

    // Verify config state
    let config_account = svm.get_account(&config_pda).unwrap();
    assert_eq!(read_config_proposal_threshold(&config_account.data), proposal_threshold, "threshold should match");
    assert_eq!(read_config_next_proposal_id(&config_account.data), 1, "next_proposal_id should start at 1");
}

#[test]
fn test_parity_weighted_vote_calculation() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/governance.so")
        .expect("Failed to read governance.so");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup
    let authority = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let mint = Keypair::new();
    svm.set_account(mint.pubkey(), create_mint_account(&authority.pubkey(), 9)).unwrap();

    let (config_pda, _) = derive_governance_config_pda(&mint.pubkey(), &program_id);

    // Initialize governance
    let init_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint.pubkey()),
            writable_meta(config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: initialize_governance_data(100, 100),
    };
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Create token account for creator with 1000 tokens
    let creator_token_account = Keypair::new();
    svm.set_account(creator_token_account.pubkey(), create_token_account(&authority.pubkey(), &mint.pubkey(), 1000)).unwrap();

    // Create proposal
    svm.expire_blockhash();
    let proposal_id: u64 = 1;
    let (proposal_pda, _) = derive_proposal_pda(proposal_id, &program_id);
    let create_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(authority.pubkey()),
            writable_meta(creator_token_account.pubkey()), // must be mut
            writable_meta(config_pda),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: create_proposal_data(proposal_id, "Test", "Test", 500, 50),
    };
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Cast votes with different token balances
    // Voter 1: 300 tokens, votes yes
    // Voter 2: 200 tokens, votes yes
    // Voter 3: 400 tokens, votes no
    let voters_config = vec![
        (300u64, true),
        (200u64, true),
        (400u64, false),
    ];

    for (token_amount, vote_yes) in voters_config {
        let voter = {
            let kp = Keypair::new();
            svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
            kp
        };
        let voter_token_account = Keypair::new();
        svm.set_account(voter_token_account.pubkey(), create_token_account(&voter.pubkey(), &mint.pubkey(), token_amount)).unwrap();

        svm.expire_blockhash();
        let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey(), &program_id);
        let vote_ix = Instruction {
            program_id,
            accounts: vec![
                signer_meta(voter.pubkey()),
                writable_meta(voter_token_account.pubkey()), // must be mut
                writable_meta(config_pda),                    // must be mut
                writable_meta(proposal_pda),
                writable_meta(voter_record_pda),
                readonly_meta(solana_sdk_ids::sysvar::clock::id()),
                readonly_meta(solana_sdk_ids::sysvar::rent::id()),
                readonly_meta(system_program::id()),
            ],
            data: cast_vote_data(proposal_id, vote_yes),
        };
        execute_tx(&mut svm, vote_ix, &voter, &[&voter]).unwrap();

        // Verify voting power recorded
        let voter_record = svm.get_account(&voter_record_pda).unwrap();
        assert_eq!(
            read_voter_record_voting_power(&voter_record.data),
            token_amount,
            "voting power should equal token balance"
        );
    }

    // Verify weighted totals: yes=500 (300+200), no=400
    let proposal_account = svm.get_account(&proposal_pda).unwrap();
    assert_eq!(read_proposal_yes_votes(&proposal_account.data), 500, "weighted yes_votes should be 500");
    assert_eq!(read_proposal_no_votes(&proposal_account.data), 400, "weighted no_votes should be 400");
}

#[test]
fn test_parity_quorum_enforcement() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/governance.so")
        .expect("Failed to read governance.so");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup
    let authority = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let mint = Keypair::new();
    svm.set_account(mint.pubkey(), create_mint_account(&authority.pubkey(), 9)).unwrap();

    let (config_pda, _) = derive_governance_config_pda(&mint.pubkey(), &program_id);

    // Initialize governance
    let init_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint.pubkey()),
            writable_meta(config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: initialize_governance_data(100, 100),
    };
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Create token account for creator with 1000 tokens
    let creator_token_account = Keypair::new();
    svm.set_account(creator_token_account.pubkey(), create_token_account(&authority.pubkey(), &mint.pubkey(), 1000)).unwrap();

    // Create proposal with quorum of 500
    svm.expire_blockhash();
    let proposal_id: u64 = 1;
    let quorum: u64 = 500;
    let (proposal_pda, _) = derive_proposal_pda(proposal_id, &program_id);
    let create_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(authority.pubkey()),
            writable_meta(creator_token_account.pubkey()), // must be mut
            writable_meta(config_pda),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: create_proposal_data(proposal_id, "Test", "Test", quorum, 10),
    };
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Cast only 100 tokens (below quorum of 500)
    let voter = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let voter_token_account = Keypair::new();
    svm.set_account(voter_token_account.pubkey(), create_token_account(&voter.pubkey(), &mint.pubkey(), 100)).unwrap();

    svm.expire_blockhash();
    let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey(), &program_id);
    let vote_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(voter.pubkey()),
            writable_meta(voter_token_account.pubkey()), // must be mut
            writable_meta(config_pda),                    // must be mut
            writable_meta(proposal_pda),
            writable_meta(voter_record_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: cast_vote_data(proposal_id, true),
    };
    execute_tx(&mut svm, vote_ix, &voter, &[&voter]).unwrap();

    // Verify quorum stored
    let proposal_account = svm.get_account(&proposal_pda).unwrap();
    assert_eq!(read_proposal_quorum(&proposal_account.data), quorum, "quorum should be stored");

    // Warp and finalize
    svm.warp_to_slot(15);
    svm.expire_blockhash();

    let finalizer = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let finalize_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(finalizer.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
        data: finalize_proposal_data(),
    };
    execute_tx(&mut svm, finalize_ix, &finalizer, &[&finalizer]).unwrap();

    // Verify quorum not met (result = 3)
    let proposal_account = svm.get_account(&proposal_pda).unwrap();
    assert_eq!(read_proposal_is_finalized(&proposal_account.data), true, "should be finalized");
    assert_eq!(read_proposal_result(&proposal_account.data), 3, "result should be QUORUM_NOT_MET (3)");
}

#[test]
fn test_parity_full_governance_workflow() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/governance.so")
        .expect("Failed to read governance.so");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup
    let authority = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let mint = Keypair::new();
    svm.set_account(mint.pubkey(), create_mint_account(&authority.pubkey(), 9)).unwrap();

    let (config_pda, _) = derive_governance_config_pda(&mint.pubkey(), &program_id);

    // 1. Initialize governance
    let init_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint.pubkey()),
            writable_meta(config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: initialize_governance_data(100, 100),
    };
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // 2. Create proposal
    let creator_token_account = Keypair::new();
    svm.set_account(creator_token_account.pubkey(), create_token_account(&authority.pubkey(), &mint.pubkey(), 1000)).unwrap();

    svm.expire_blockhash();
    let proposal_id: u64 = 1;
    let (proposal_pda, _) = derive_proposal_pda(proposal_id, &program_id);
    let create_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(authority.pubkey()),
            writable_meta(creator_token_account.pubkey()), // must be mut
            writable_meta(config_pda),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: create_proposal_data(proposal_id, "Upgrade Protocol", "Description", 500, 10),
    };
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // 3. Cast votes exceeding quorum: 600 yes > 500 quorum
    let voter = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let voter_token_account = Keypair::new();
    svm.set_account(voter_token_account.pubkey(), create_token_account(&voter.pubkey(), &mint.pubkey(), 600)).unwrap();

    svm.expire_blockhash();
    let (voter_record_pda, _) = derive_voter_record_pda(proposal_id, &voter.pubkey(), &program_id);
    let vote_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(voter.pubkey()),
            writable_meta(voter_token_account.pubkey()), // must be mut
            writable_meta(config_pda),                    // must be mut
            writable_meta(proposal_pda),
            writable_meta(voter_record_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: cast_vote_data(proposal_id, true),
    };
    execute_tx(&mut svm, vote_ix, &voter, &[&voter]).unwrap();

    // 4. Finalize after voting period
    svm.warp_to_slot(15);
    svm.expire_blockhash();

    let finalizer = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let finalize_ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(finalizer.pubkey()),
            writable_meta(proposal_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
        data: finalize_proposal_data(),
    };
    execute_tx(&mut svm, finalize_ix, &finalizer, &[&finalizer]).unwrap();

    // 5. Verify final state
    let proposal_account = svm.get_account(&proposal_pda).unwrap();
    assert_eq!(read_proposal_is_finalized(&proposal_account.data), true, "should be finalized");
    assert_eq!(read_proposal_result(&proposal_account.data), 1, "result should be PASSED (1)");
    assert_eq!(read_proposal_yes_votes(&proposal_account.data), 600, "yes_votes should be 600");
    assert_eq!(read_proposal_no_votes(&proposal_account.data), 0, "no_votes should be 0");
}

#[test]
fn test_parity_deterministic_pda_derivation() {
    let program_id = program_id();

    let mint = Pubkey::new_unique();
    let proposal_id: u64 = 42;
    let voter = Pubkey::new_unique();

    // Derive multiple times
    let (config_pda1, bump1) = derive_governance_config_pda(&mint, &program_id);
    let (config_pda2, bump2) = derive_governance_config_pda(&mint, &program_id);

    assert_eq!(config_pda1, config_pda2, "config PDA should be deterministic");
    assert_eq!(bump1, bump2, "config bump should be deterministic");

    let (proposal_pda1, pbump1) = derive_proposal_pda(proposal_id, &program_id);
    let (proposal_pda2, pbump2) = derive_proposal_pda(proposal_id, &program_id);

    assert_eq!(proposal_pda1, proposal_pda2, "proposal PDA should be deterministic");
    assert_eq!(pbump1, pbump2, "proposal bump should be deterministic");

    let (voter_record_pda1, vbump1) = derive_voter_record_pda(proposal_id, &voter, &program_id);
    let (voter_record_pda2, vbump2) = derive_voter_record_pda(proposal_id, &voter, &program_id);

    assert_eq!(voter_record_pda1, voter_record_pda2, "voter record PDA should be deterministic");
    assert_eq!(vbump1, vbump2, "voter record bump should be deterministic");

    // Different inputs should produce different PDAs
    let other_mint = Pubkey::new_unique();
    let (other_config, _) = derive_governance_config_pda(&other_mint, &program_id);
    assert_ne!(config_pda1, other_config, "different mint should produce different config PDA");

    let (other_proposal, _) = derive_proposal_pda(99, &program_id);
    assert_ne!(proposal_pda1, other_proposal, "different proposal_id should produce different PDA");

    let other_voter = Pubkey::new_unique();
    let (other_voter_pda, _) = derive_voter_record_pda(proposal_id, &other_voter, &program_id);
    assert_ne!(voter_record_pda1, other_voter_pda, "different voter should produce different PDA");
}
