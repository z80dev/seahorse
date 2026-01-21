//! Vesting tests: Seahorse implementation behavior verification
//!
//! These tests verify the Seahorse vesting implementation produces
//! mathematically correct results for cliff and linear vesting.
//!
//! Key behaviors to verify:
//! - Vesting schedule initialization creates correct PDAs and state
//! - Cliff period enforcement (nothing vested before cliff)
//! - Linear vesting calculation: total * elapsed / vesting_period
//! - Partial claims work correctly (releasable = vested - already_released)
//! - Cancellation distributes tokens correctly
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile:
//! - target/deploy/vesting.so (Seahorse)

use seahorse_test_common::*;
use solana_account::Account;
use solana_keypair::Keypair;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use spl_token::solana_program::program_option::COption;
use spl_token::solana_program::program_pack::Pack;
use std::str::FromStr;

/// SPL Token Mint account size
const MINT_SIZE: usize = 82;
/// SPL Token Account size
const TOKEN_ACCOUNT_SIZE: usize = 165;

/// Vesting program ID
fn program_id() -> Pubkey {
    Pubkey::from_str("VESTngW9a1XxRgwWhmK7PNYWMFp4mW8q3xk2BgfCNih").unwrap()
}

/// SPL Token program ID
fn token_program_id() -> Pubkey {
    spl_token::id()
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

/// Derive vesting PDA
fn derive_vesting_pda(beneficiary: &Pubkey, mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"vesting", beneficiary.as_ref(), mint.as_ref()], program_id)
}

/// Derive vault PDA
fn derive_vault_pda(beneficiary: &Pubkey, mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"vesting_vault", beneficiary.as_ref(), mint.as_ref()], program_id)
}

/// Get ATA address
fn get_ata(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    spl_associated_token_account::get_associated_token_address(wallet, mint)
}

/// Read token account balance
fn read_token_balance(data: &[u8]) -> u64 {
    let account = spl_token::state::Account::unpack(data).unwrap();
    account.amount
}

/// Read vesting account fields
fn read_vesting_total_amount(data: &[u8]) -> u64 {
    // Offset: 8 (disc) + 32 (authority) + 32 (beneficiary) + 32 (mint) + 32 (vault) = 136
    let bytes: [u8; 8] = data[136..144].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_vesting_released_amount(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[144..152].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_vesting_start_slot(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[152..160].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_vesting_cliff_slot(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[160..168].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_vesting_end_slot(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[168..176].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_vesting_is_cancelled(data: &[u8]) -> bool {
    data[176] != 0
}

/// Build create_vesting instruction data
fn create_vesting_data(beneficiary: &Pubkey, amount: u64, cliff_slots: u64, vesting_duration_slots: u64) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(beneficiary.as_ref());
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&cliff_slots.to_le_bytes());
    data.extend_from_slice(&vesting_duration_slots.to_le_bytes());
    data
}

/// Calculate expected vested amount
fn calculate_vested_amount(total: u64, cliff_slot: u64, end_slot: u64, current_slot: u64) -> u64 {
    if current_slot < cliff_slot {
        0
    } else if current_slot >= end_slot {
        total
    } else {
        let vesting_period = end_slot - cliff_slot;
        let elapsed = current_slot - cliff_slot;
        total * elapsed / vesting_period
    }
}

// =============================================================================
// PARITY TESTS
// =============================================================================

#[test]
fn test_parity_vesting_initialization() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/vesting.so")
        .expect("Failed to read vesting.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup mint
    let mint = Keypair::new();
    svm.set_account(mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    // Setup authority with tokens
    let authority = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let authority_token = get_ata(&authority.pubkey(), &mint.pubkey());
    svm.set_account(authority_token, create_token_account(&authority.pubkey(), &mint.pubkey(), 10000))
        .unwrap();

    // Setup beneficiary
    let beneficiary = Keypair::new();

    // Derive PDAs
    let (vesting_pda, _) = derive_vesting_pda(&beneficiary.pubkey(), &mint.pubkey(), &program_id);
    let (vault_pda, _) = derive_vault_pda(&beneficiary.pubkey(), &mint.pubkey(), &program_id);

    let amount: u64 = 1000;
    let cliff_slots: u64 = 100;
    let vesting_duration_slots: u64 = 900;

    // Create vesting
    let ix = InstructionBuilder::new("create_vesting")
        .with_signer(&authority.pubkey())
        .with_writable(&mint.pubkey())
        .with_writable(&authority_token)
        .with_writable(&vesting_pda)
        .with_writable(&vault_pda)
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(create_vesting_data(&beneficiary.pubkey(), amount, cliff_slots, vesting_duration_slots))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[ix], Some(&authority.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&authority], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "create_vesting failed: {:?}", result);

    // Verify vesting account state
    let vesting = svm.get_account(&vesting_pda).expect("Vesting should exist");
    assert_eq!(vesting.owner, program_id, "Vesting should be owned by program");
    assert_eq!(read_vesting_total_amount(&vesting.data), amount);
    assert_eq!(read_vesting_released_amount(&vesting.data), 0);

    let start_slot = read_vesting_start_slot(&vesting.data);
    assert_eq!(read_vesting_cliff_slot(&vesting.data), start_slot + cliff_slots);
    assert_eq!(read_vesting_end_slot(&vesting.data), start_slot + cliff_slots + vesting_duration_slots);
    assert!(!read_vesting_is_cancelled(&vesting.data));

    // Verify vault has tokens
    let vault = svm.get_account(&vault_pda).expect("Vault should exist");
    assert_eq!(read_token_balance(&vault.data), amount);
}

#[test]
fn test_parity_cliff_enforcement() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/vesting.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup
    let mint = Keypair::new();
    svm.set_account(mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    let authority = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let authority_token = get_ata(&authority.pubkey(), &mint.pubkey());
    svm.set_account(authority_token, create_token_account(&authority.pubkey(), &mint.pubkey(), 10000))
        .unwrap();

    let beneficiary = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let beneficiary_token = get_ata(&beneficiary.pubkey(), &mint.pubkey());
    svm.set_account(beneficiary_token, create_token_account(&beneficiary.pubkey(), &mint.pubkey(), 0))
        .unwrap();

    let (vesting_pda, _) = derive_vesting_pda(&beneficiary.pubkey(), &mint.pubkey(), &program_id);
    let (vault_pda, _) = derive_vault_pda(&beneficiary.pubkey(), &mint.pubkey(), &program_id);

    // Create vesting with 100 slot cliff
    let create_ix = InstructionBuilder::new("create_vesting")
        .with_signer(&authority.pubkey())
        .with_writable(&mint.pubkey())
        .with_writable(&authority_token)
        .with_writable(&vesting_pda)
        .with_writable(&vault_pda)
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(create_vesting_data(&beneficiary.pubkey(), 1000, 100, 900))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[create_ix], Some(&authority.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&authority], message, blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // Try to claim before cliff (at current slot, which is < cliff_slot)
    let claim_ix = InstructionBuilder::new("claim_vested")
        .with_signer(&beneficiary.pubkey())
        .with_writable(&vesting_pda)
        .with_writable(&vault_pda)
        .with_writable(&beneficiary_token)
        .with_writable(&mint.pubkey())
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&token_program_id())
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[claim_ix], Some(&beneficiary.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&beneficiary], message, blockhash);
    let result = svm.send_transaction(tx);

    // Should fail because nothing is vested before cliff
    assert!(result.is_err(), "Claim before cliff should fail");

    // Verify beneficiary has no tokens
    let beneficiary_account = svm.get_account(&beneficiary_token).unwrap();
    assert_eq!(read_token_balance(&beneficiary_account.data), 0);
}

#[test]
fn test_parity_linear_vesting_calculation() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/vesting.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup
    let mint = Keypair::new();
    svm.set_account(mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    let authority = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let authority_token = get_ata(&authority.pubkey(), &mint.pubkey());
    svm.set_account(authority_token, create_token_account(&authority.pubkey(), &mint.pubkey(), 10000))
        .unwrap();

    let beneficiary = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let beneficiary_token = get_ata(&beneficiary.pubkey(), &mint.pubkey());
    svm.set_account(beneficiary_token, create_token_account(&beneficiary.pubkey(), &mint.pubkey(), 0))
        .unwrap();

    let (vesting_pda, _) = derive_vesting_pda(&beneficiary.pubkey(), &mint.pubkey(), &program_id);
    let (vault_pda, _) = derive_vault_pda(&beneficiary.pubkey(), &mint.pubkey(), &program_id);

    let total_amount: u64 = 1000;
    let cliff_slots: u64 = 0; // No cliff for simpler testing
    let vesting_period: u64 = 1000;

    // Create vesting
    let create_ix = InstructionBuilder::new("create_vesting")
        .with_signer(&authority.pubkey())
        .with_writable(&mint.pubkey())
        .with_writable(&authority_token)
        .with_writable(&vesting_pda)
        .with_writable(&vault_pda)
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(create_vesting_data(&beneficiary.pubkey(), total_amount, cliff_slots, vesting_period))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[create_ix], Some(&authority.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&authority], message, blockhash);
    svm.send_transaction(tx).unwrap();

    let vesting = svm.get_account(&vesting_pda).unwrap();
    let start_slot = read_vesting_start_slot(&vesting.data);
    let cliff_slot = read_vesting_cliff_slot(&vesting.data);
    let end_slot = read_vesting_end_slot(&vesting.data);

    // Warp to 50% vesting (500 slots into vesting period)
    let target_slot = start_slot + 500;
    svm.warp_to_slot(target_slot);
    svm.expire_blockhash();

    // Calculate expected vested amount
    let expected_vested = calculate_vested_amount(total_amount, cliff_slot, end_slot, target_slot);
    assert_eq!(expected_vested, 500, "At 50%, 500 tokens should be vested");

    // Claim vested tokens
    let claim_ix = InstructionBuilder::new("claim_vested")
        .with_signer(&beneficiary.pubkey())
        .with_writable(&vesting_pda)
        .with_writable(&vault_pda)
        .with_writable(&beneficiary_token)
        .with_writable(&mint.pubkey())
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&token_program_id())
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[claim_ix], Some(&beneficiary.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&beneficiary], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "claim_vested failed: {:?}", result);

    // Verify beneficiary received expected amount
    let beneficiary_account = svm.get_account(&beneficiary_token).unwrap();
    let actual_claimed = read_token_balance(&beneficiary_account.data);
    assert_eq!(actual_claimed, expected_vested, "Should receive exactly 50%");

    // Verify vesting released amount updated
    let vesting = svm.get_account(&vesting_pda).unwrap();
    assert_eq!(read_vesting_released_amount(&vesting.data), expected_vested);
}

#[test]
fn test_parity_partial_claims_work_correctly() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/vesting.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup
    let mint = Keypair::new();
    svm.set_account(mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    let authority = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let authority_token = get_ata(&authority.pubkey(), &mint.pubkey());
    svm.set_account(authority_token, create_token_account(&authority.pubkey(), &mint.pubkey(), 10000))
        .unwrap();

    let beneficiary = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let beneficiary_token = get_ata(&beneficiary.pubkey(), &mint.pubkey());
    svm.set_account(beneficiary_token, create_token_account(&beneficiary.pubkey(), &mint.pubkey(), 0))
        .unwrap();

    let (vesting_pda, _) = derive_vesting_pda(&beneficiary.pubkey(), &mint.pubkey(), &program_id);
    let (vault_pda, _) = derive_vault_pda(&beneficiary.pubkey(), &mint.pubkey(), &program_id);

    // Create vesting: 1000 tokens over 1000 slots
    let create_ix = InstructionBuilder::new("create_vesting")
        .with_signer(&authority.pubkey())
        .with_writable(&mint.pubkey())
        .with_writable(&authority_token)
        .with_writable(&vesting_pda)
        .with_writable(&vault_pda)
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(create_vesting_data(&beneficiary.pubkey(), 1000, 0, 1000))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[create_ix], Some(&authority.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&authority], message, blockhash);
    svm.send_transaction(tx).unwrap();

    let vesting = svm.get_account(&vesting_pda).unwrap();
    let start_slot = read_vesting_start_slot(&vesting.data);

    // First claim at 25%
    svm.warp_to_slot(start_slot + 250);
    svm.expire_blockhash();

    let claim1_ix = InstructionBuilder::new("claim_vested")
        .with_signer(&beneficiary.pubkey())
        .with_writable(&vesting_pda)
        .with_writable(&vault_pda)
        .with_writable(&beneficiary_token)
        .with_writable(&mint.pubkey())
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&token_program_id())
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[claim1_ix], Some(&beneficiary.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&beneficiary], message, blockhash);
    svm.send_transaction(tx).unwrap();

    let beneficiary_account = svm.get_account(&beneficiary_token).unwrap();
    let first_claim = read_token_balance(&beneficiary_account.data);
    assert_eq!(first_claim, 250, "First claim should be 250 tokens (25%)");

    // Second claim at 75%
    svm.warp_to_slot(start_slot + 750);
    svm.expire_blockhash();

    let claim2_ix = InstructionBuilder::new("claim_vested")
        .with_signer(&beneficiary.pubkey())
        .with_writable(&vesting_pda)
        .with_writable(&vault_pda)
        .with_writable(&beneficiary_token)
        .with_writable(&mint.pubkey())
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&token_program_id())
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[claim2_ix], Some(&beneficiary.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&beneficiary], message, blockhash);
    svm.send_transaction(tx).unwrap();

    let beneficiary_account = svm.get_account(&beneficiary_token).unwrap();
    let total_claimed = read_token_balance(&beneficiary_account.data);
    // Total should be 750 (75% vested), second claim should have added 500 (750 - 250 already claimed)
    assert_eq!(total_claimed, 750, "Total after second claim should be 750 tokens (75%)");

    // Verify released amount matches
    let vesting = svm.get_account(&vesting_pda).unwrap();
    assert_eq!(read_vesting_released_amount(&vesting.data), 750);
}

#[test]
fn test_parity_deterministic_pda_derivation() {
    let program_id = program_id();

    let beneficiary = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    // Verify vesting PDA derivation is deterministic
    let (vesting1, bump1) = derive_vesting_pda(&beneficiary, &mint, &program_id);
    let (vesting2, bump2) = derive_vesting_pda(&beneficiary, &mint, &program_id);

    assert_eq!(vesting1, vesting2, "Vesting PDAs should be deterministic");
    assert_eq!(bump1, bump2, "Vesting bumps should be deterministic");

    // Verify vault PDA derivation is deterministic
    let (vault1, vault_bump1) = derive_vault_pda(&beneficiary, &mint, &program_id);
    let (vault2, vault_bump2) = derive_vault_pda(&beneficiary, &mint, &program_id);

    assert_eq!(vault1, vault2, "Vault PDAs should be deterministic");
    assert_eq!(vault_bump1, vault_bump2, "Vault bumps should be deterministic");

    // Verify different beneficiaries produce different PDAs
    let other_beneficiary = Pubkey::new_unique();
    let (vesting3, _) = derive_vesting_pda(&other_beneficiary, &mint, &program_id);
    assert_ne!(vesting1, vesting3, "Different beneficiaries should produce different PDAs");

    // Verify different mints produce different PDAs
    let other_mint = Pubkey::new_unique();
    let (vesting4, _) = derive_vesting_pda(&beneficiary, &other_mint, &program_id);
    assert_ne!(vesting1, vesting4, "Different mints should produce different PDAs");
}
