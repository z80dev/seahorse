//! LiteSVM integration tests for the Seahorse Vesting program
//!
//! These tests verify token vesting with cliff and linear release:
//! - create_vesting: Creates vesting schedule with cliff and linear period
//! - claim_vested: Claims vested tokens after cliff/during linear period
//! - cancel_vesting: Authority cancels vesting, returns unvested tokens
//!
//! Uses LiteSVM's warp_to_slot for time-based vesting testing.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile vesting.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// Vesting program ID (from declare_id!)
fn vesting_program_id() -> Pubkey {
    Pubkey::from_str("VESTngW9a1XxRgwWhmK7PNYWMFp4mW8q3xk2BgfCNih").unwrap()
}

/// VestingAccount size:
/// discriminator (8) + authority (32) + beneficiary (32) + mint (32) + vault (32)
/// + total_amount (8) + released_amount (8) + start_slot (8) + cliff_slot (8)
/// + end_slot (8) + is_cancelled (1) + bump (1) = 178 bytes
const VESTING_ACCOUNT_SIZE: usize = 8 + 32 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 8 + 1 + 1;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the vesting program into LiteSVM
fn load_vesting_program() -> litesvm::LiteSVM {
    let program_id = vesting_program_id();
    let program_bytes = std::fs::read("../../target/deploy/vesting.so")
        .expect("Failed to read vesting.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);
    svm
}

/// Setup a mint and mint authority for testing
fn setup_mint(svm: &mut litesvm::LiteSVM, decimals: u8) -> (Keypair, Pubkey) {
    let mint_authority = funded_keypair_10_sol(svm);
    let mint = Keypair::new();

    let mint_account = create_mint_account(&mint_authority.pubkey(), decimals);
    svm.set_account(mint.pubkey(), mint_account).unwrap();

    (mint_authority, mint.pubkey())
}

/// Setup a user with tokens
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

/// Setup a user with empty token account
fn setup_user_with_empty_token_account(
    svm: &mut litesvm::LiteSVM,
    mint: &Pubkey,
) -> (Keypair, Pubkey) {
    setup_user_with_tokens(svm, mint, 0)
}

/// Derive vesting PDA
fn derive_vesting_pda(beneficiary: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"vesting", beneficiary.as_ref(), mint.as_ref()],
        &vesting_program_id(),
    )
}

/// Derive vault PDA
fn derive_vault_pda(beneficiary: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"vesting_vault", beneficiary.as_ref(), mint.as_ref()],
        &vesting_program_id(),
    )
}

/// Read vesting account fields
fn read_vesting_authority(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

fn read_vesting_beneficiary(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[40..72].try_into().unwrap())
}

fn read_vesting_mint(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[72..104].try_into().unwrap())
}

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

// =============================================================================
// CREATE VESTING TESTS
// =============================================================================

#[test]
fn test_create_vesting_with_cliff() {
    let mut svm = load_vesting_program();
    let program_id = vesting_program_id();

    // Setup mint
    let (_, mint) = setup_mint(&mut svm, 6);

    // Setup authority with tokens
    let (authority, authority_token) = setup_user_with_tokens(&mut svm, &mint, 10000);

    // Setup beneficiary (will receive vested tokens)
    let beneficiary = Keypair::new();

    // Derive PDAs
    let (vesting_pda, _) = derive_vesting_pda(&beneficiary.pubkey(), &mint);
    let (vault_pda, _) = derive_vault_pda(&beneficiary.pubkey(), &mint);

    let amount: u64 = 1000;
    let cliff_slots: u64 = 100;
    let vesting_duration_slots: u64 = 900;

    // Build create_vesting instruction (Seahorse account order)
    let ix = anchor_instruction(
        program_id,
        "create_vesting",
        &create_vesting_data(&beneficiary.pubkey(), amount, cliff_slots, vesting_duration_slots),
        vec![
            signer_meta(authority.pubkey()),      // authority
            writable_meta(mint),                  // mint
            writable_meta(authority_token),       // authority_token
            writable_meta(vesting_pda),           // vesting
            writable_meta(vault_pda),             // vault
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),  // rent
            readonly_meta(system_program::id()), // system_program
            readonly_meta(token_program_id()),   // token_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "create_vesting should succeed: {:?}", result);

    // Verify vesting account state
    let vesting_account = svm.get_account(&vesting_pda).expect("Vesting should exist");
    assert_eq!(vesting_account.owner, program_id);

    assert_eq!(read_vesting_authority(&vesting_account.data), authority.pubkey());
    assert_eq!(read_vesting_beneficiary(&vesting_account.data), beneficiary.pubkey());
    assert_eq!(read_vesting_mint(&vesting_account.data), mint);
    assert_eq!(read_vesting_total_amount(&vesting_account.data), amount);
    assert_eq!(read_vesting_released_amount(&vesting_account.data), 0);

    let start_slot = read_vesting_start_slot(&vesting_account.data);
    assert_eq!(read_vesting_cliff_slot(&vesting_account.data), start_slot + cliff_slots);
    assert_eq!(read_vesting_end_slot(&vesting_account.data), start_slot + cliff_slots + vesting_duration_slots);
    assert!(!read_vesting_is_cancelled(&vesting_account.data));

    // Verify vault has tokens
    let vault_account = svm.get_account(&vault_pda).expect("Vault should exist");
    assert_eq!(read_token_balance(&vault_account.data), amount);

    // Verify authority's tokens decreased
    let authority_token_account = svm.get_account(&authority_token).unwrap();
    assert_eq!(read_token_balance(&authority_token_account.data), 9000);
}

#[test]
fn test_create_vesting_no_cliff() {
    let mut svm = load_vesting_program();
    let program_id = vesting_program_id();

    // Setup
    let (_, mint) = setup_mint(&mut svm, 6);
    let (authority, authority_token) = setup_user_with_tokens(&mut svm, &mint, 10000);
    let beneficiary = Keypair::new();

    let (vesting_pda, _) = derive_vesting_pda(&beneficiary.pubkey(), &mint);
    let (vault_pda, _) = derive_vault_pda(&beneficiary.pubkey(), &mint);

    // No cliff - tokens vest immediately from start
    let amount: u64 = 500;
    let cliff_slots: u64 = 0;
    let vesting_duration_slots: u64 = 100;

    let ix = anchor_instruction(
        program_id,
        "create_vesting",
        &create_vesting_data(&beneficiary.pubkey(), amount, cliff_slots, vesting_duration_slots),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint),
            writable_meta(authority_token),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "create_vesting without cliff should succeed");

    // Verify cliff_slot == start_slot (no cliff)
    let vesting_account = svm.get_account(&vesting_pda).unwrap();
    let start_slot = read_vesting_start_slot(&vesting_account.data);
    assert_eq!(read_vesting_cliff_slot(&vesting_account.data), start_slot);
}

#[test]
fn test_create_vesting_zero_amount_fails() {
    let mut svm = load_vesting_program();
    let program_id = vesting_program_id();

    let (_, mint) = setup_mint(&mut svm, 6);
    let (authority, authority_token) = setup_user_with_tokens(&mut svm, &mint, 10000);
    let beneficiary = Keypair::new();

    let (vesting_pda, _) = derive_vesting_pda(&beneficiary.pubkey(), &mint);
    let (vault_pda, _) = derive_vault_pda(&beneficiary.pubkey(), &mint);

    let ix = anchor_instruction(
        program_id,
        "create_vesting",
        &create_vesting_data(&beneficiary.pubkey(), 0, 100, 900),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint),
            writable_meta(authority_token),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "create_vesting with zero amount should fail");
}

#[test]
fn test_create_vesting_zero_duration_fails() {
    let mut svm = load_vesting_program();
    let program_id = vesting_program_id();

    let (_, mint) = setup_mint(&mut svm, 6);
    let (authority, authority_token) = setup_user_with_tokens(&mut svm, &mint, 10000);
    let beneficiary = Keypair::new();

    let (vesting_pda, _) = derive_vesting_pda(&beneficiary.pubkey(), &mint);
    let (vault_pda, _) = derive_vault_pda(&beneficiary.pubkey(), &mint);

    let ix = anchor_instruction(
        program_id,
        "create_vesting",
        &create_vesting_data(&beneficiary.pubkey(), 1000, 100, 0),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint),
            writable_meta(authority_token),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "create_vesting with zero duration should fail");
}

// =============================================================================
// CLAIM VESTED TESTS
// =============================================================================

#[test]
fn test_claim_vested_before_cliff_fails() {
    let mut svm = load_vesting_program();
    let program_id = vesting_program_id();

    // Setup
    let (_, mint) = setup_mint(&mut svm, 6);
    let (authority, authority_token) = setup_user_with_tokens(&mut svm, &mint, 10000);
    let (beneficiary, beneficiary_token) = setup_user_with_empty_token_account(&mut svm, &mint);

    let (vesting_pda, _) = derive_vesting_pda(&beneficiary.pubkey(), &mint);
    let (vault_pda, _) = derive_vault_pda(&beneficiary.pubkey(), &mint);

    // Create vesting with 100 slot cliff
    let create_ix = anchor_instruction(
        program_id,
        "create_vesting",
        &create_vesting_data(&beneficiary.pubkey(), 1000, 100, 900),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint),
            writable_meta(authority_token),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Try to claim before cliff (still at slot 1)
    svm.expire_blockhash();
    let claim_ix = anchor_instruction(
        program_id,
        "claim_vested",
        &[],
        vec![
            signer_meta(beneficiary.pubkey()),    // beneficiary
            writable_meta(vesting_pda),           // vesting
            writable_meta(vault_pda),             // vault
            writable_meta(beneficiary_token),     // beneficiary_token
            writable_meta(mint),                  // mint
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(token_program_id()),   // token_program
        ],
    );

    let result = execute_tx(&mut svm, claim_ix, &beneficiary, &[&beneficiary]);
    assert!(result.is_err(), "claim_vested before cliff should fail");
}

#[test]
fn test_claim_vested_at_cliff() {
    let mut svm = load_vesting_program();
    let program_id = vesting_program_id();

    // Setup
    let (_, mint) = setup_mint(&mut svm, 6);
    let (authority, authority_token) = setup_user_with_tokens(&mut svm, &mint, 10000);
    let (beneficiary, beneficiary_token) = setup_user_with_empty_token_account(&mut svm, &mint);

    let (vesting_pda, _) = derive_vesting_pda(&beneficiary.pubkey(), &mint);
    let (vault_pda, _) = derive_vault_pda(&beneficiary.pubkey(), &mint);

    // Create vesting: 1000 tokens, 100 slot cliff, 900 slot vesting period
    let create_ix = anchor_instruction(
        program_id,
        "create_vesting",
        &create_vesting_data(&beneficiary.pubkey(), 1000, 100, 900),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint),
            writable_meta(authority_token),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Get start slot to calculate cliff
    let vesting_account = svm.get_account(&vesting_pda).unwrap();
    let cliff_slot = read_vesting_cliff_slot(&vesting_account.data);

    // Warp to cliff (exactly at cliff, vested = 0 because elapsed = 0)
    // Actually need to go just past cliff to have some vested
    svm.warp_to_slot(cliff_slot + 100); // 100 slots into vesting period = 100/900 = ~11.1%
    svm.expire_blockhash();

    // Claim vested tokens
    let claim_ix = anchor_instruction(
        program_id,
        "claim_vested",
        &[],
        vec![
            signer_meta(beneficiary.pubkey()),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            writable_meta(beneficiary_token),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, claim_ix, &beneficiary, &[&beneficiary]);
    assert!(result.is_ok(), "claim_vested at cliff should succeed: {:?}", result);

    // Verify beneficiary received some tokens
    let beneficiary_account = svm.get_account(&beneficiary_token).unwrap();
    let claimed = read_token_balance(&beneficiary_account.data);
    assert!(claimed > 0, "Beneficiary should have received tokens");
    // Expected: 1000 * 100 / 900 = 111 tokens
    assert!(claimed >= 100 && claimed <= 120, "Should have claimed ~111 tokens, got {}", claimed);

    // Verify vesting released_amount updated
    let vesting_account = svm.get_account(&vesting_pda).unwrap();
    assert_eq!(read_vesting_released_amount(&vesting_account.data), claimed);
}

#[test]
fn test_claim_vested_partial_claims() {
    let mut svm = load_vesting_program();
    let program_id = vesting_program_id();

    // Setup
    let (_, mint) = setup_mint(&mut svm, 6);
    let (authority, authority_token) = setup_user_with_tokens(&mut svm, &mint, 10000);
    let (beneficiary, beneficiary_token) = setup_user_with_empty_token_account(&mut svm, &mint);

    let (vesting_pda, _) = derive_vesting_pda(&beneficiary.pubkey(), &mint);
    let (vault_pda, _) = derive_vault_pda(&beneficiary.pubkey(), &mint);

    // Create vesting: 1000 tokens, no cliff, 1000 slot vesting period
    let create_ix = anchor_instruction(
        program_id,
        "create_vesting",
        &create_vesting_data(&beneficiary.pubkey(), 1000, 0, 1000),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint),
            writable_meta(authority_token),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    let vesting_account = svm.get_account(&vesting_pda).unwrap();
    let start_slot = read_vesting_start_slot(&vesting_account.data);

    // First claim at 25% vesting (250 slots)
    svm.warp_to_slot(start_slot + 250);
    svm.expire_blockhash();

    let claim_ix = anchor_instruction(
        program_id,
        "claim_vested",
        &[],
        vec![
            signer_meta(beneficiary.pubkey()),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            writable_meta(beneficiary_token),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, claim_ix, &beneficiary, &[&beneficiary]).unwrap();

    let beneficiary_account = svm.get_account(&beneficiary_token).unwrap();
    let first_claim = read_token_balance(&beneficiary_account.data);
    assert_eq!(first_claim, 250, "First claim should be 250 tokens (25%)");

    // Second claim at 75% vesting (750 slots)
    svm.warp_to_slot(start_slot + 750);
    svm.expire_blockhash();

    let claim_ix2 = anchor_instruction(
        program_id,
        "claim_vested",
        &[],
        vec![
            signer_meta(beneficiary.pubkey()),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            writable_meta(beneficiary_token),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, claim_ix2, &beneficiary, &[&beneficiary]).unwrap();

    let beneficiary_account = svm.get_account(&beneficiary_token).unwrap();
    let total_claimed = read_token_balance(&beneficiary_account.data);
    assert_eq!(total_claimed, 750, "Total should be 750 tokens (75%)");

    // Final claim after end
    svm.warp_to_slot(start_slot + 1500);
    svm.expire_blockhash();

    let claim_ix3 = anchor_instruction(
        program_id,
        "claim_vested",
        &[],
        vec![
            signer_meta(beneficiary.pubkey()),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            writable_meta(beneficiary_token),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, claim_ix3, &beneficiary, &[&beneficiary]).unwrap();

    let beneficiary_account = svm.get_account(&beneficiary_token).unwrap();
    let final_total = read_token_balance(&beneficiary_account.data);
    assert_eq!(final_total, 1000, "Final total should be 1000 tokens (100%)");

    // Verify vesting is fully released
    let vesting_account = svm.get_account(&vesting_pda).unwrap();
    assert_eq!(read_vesting_released_amount(&vesting_account.data), 1000);
}

#[test]
fn test_claim_vested_unauthorized_fails() {
    let mut svm = load_vesting_program();
    let program_id = vesting_program_id();

    // Setup
    let (_, mint) = setup_mint(&mut svm, 6);
    let (authority, authority_token) = setup_user_with_tokens(&mut svm, &mint, 10000);
    let (beneficiary, _) = setup_user_with_empty_token_account(&mut svm, &mint);
    let (attacker, attacker_token) = setup_user_with_empty_token_account(&mut svm, &mint);

    let (vesting_pda, _) = derive_vesting_pda(&beneficiary.pubkey(), &mint);
    let (vault_pda, _) = derive_vault_pda(&beneficiary.pubkey(), &mint);

    // Create vesting for beneficiary
    let create_ix = anchor_instruction(
        program_id,
        "create_vesting",
        &create_vesting_data(&beneficiary.pubkey(), 1000, 0, 1000),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint),
            writable_meta(authority_token),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    let vesting_account = svm.get_account(&vesting_pda).unwrap();
    let start_slot = read_vesting_start_slot(&vesting_account.data);

    // Warp past cliff
    svm.warp_to_slot(start_slot + 500);
    svm.expire_blockhash();

    // Attacker tries to claim
    let claim_ix = anchor_instruction(
        program_id,
        "claim_vested",
        &[],
        vec![
            signer_meta(attacker.pubkey()),       // Wrong signer!
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            writable_meta(attacker_token),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, claim_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "Unauthorized claim should fail");
}

// =============================================================================
// CANCEL VESTING TESTS
// =============================================================================

#[test]
fn test_cancel_vesting_before_cliff() {
    let mut svm = load_vesting_program();
    let program_id = vesting_program_id();

    // Setup
    let (_, mint) = setup_mint(&mut svm, 6);
    let (authority, authority_token) = setup_user_with_tokens(&mut svm, &mint, 10000);
    let (beneficiary, beneficiary_token) = setup_user_with_empty_token_account(&mut svm, &mint);

    let (vesting_pda, _) = derive_vesting_pda(&beneficiary.pubkey(), &mint);
    let (vault_pda, _) = derive_vault_pda(&beneficiary.pubkey(), &mint);

    // Create vesting: 1000 tokens, 100 slot cliff
    let create_ix = anchor_instruction(
        program_id,
        "create_vesting",
        &create_vesting_data(&beneficiary.pubkey(), 1000, 100, 900),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint),
            writable_meta(authority_token),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Authority token balance after creating vesting
    let authority_account = svm.get_account(&authority_token).unwrap();
    assert_eq!(read_token_balance(&authority_account.data), 9000);

    // Cancel immediately (before cliff - nothing vested)
    svm.expire_blockhash();
    let cancel_ix = anchor_instruction(
        program_id,
        "cancel_vesting",
        &[],
        vec![
            signer_meta(authority.pubkey()),      // authority
            writable_meta(vesting_pda),           // vesting
            writable_meta(vault_pda),             // vault
            writable_meta(authority_token),       // authority_token
            writable_meta(beneficiary_token),     // beneficiary_token
            writable_meta(mint),                  // mint
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(token_program_id()),   // token_program
        ],
    );

    let result = execute_tx(&mut svm, cancel_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "cancel_vesting should succeed: {:?}", result);

    // Verify all tokens returned to authority (nothing vested yet)
    let authority_account = svm.get_account(&authority_token).unwrap();
    assert_eq!(read_token_balance(&authority_account.data), 10000, "All tokens should return to authority");

    // Verify beneficiary got nothing
    let beneficiary_account = svm.get_account(&beneficiary_token).unwrap();
    assert_eq!(read_token_balance(&beneficiary_account.data), 0);

    // Verify vesting is cancelled
    let vesting_account = svm.get_account(&vesting_pda).unwrap();
    assert!(read_vesting_is_cancelled(&vesting_account.data));
}

#[test]
fn test_cancel_vesting_mid_vesting() {
    let mut svm = load_vesting_program();
    let program_id = vesting_program_id();

    // Setup
    let (_, mint) = setup_mint(&mut svm, 6);
    let (authority, authority_token) = setup_user_with_tokens(&mut svm, &mint, 10000);
    let (beneficiary, beneficiary_token) = setup_user_with_empty_token_account(&mut svm, &mint);

    let (vesting_pda, _) = derive_vesting_pda(&beneficiary.pubkey(), &mint);
    let (vault_pda, _) = derive_vault_pda(&beneficiary.pubkey(), &mint);

    // Create vesting: 1000 tokens, no cliff, 1000 slot vesting
    let create_ix = anchor_instruction(
        program_id,
        "create_vesting",
        &create_vesting_data(&beneficiary.pubkey(), 1000, 0, 1000),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint),
            writable_meta(authority_token),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    let vesting_account = svm.get_account(&vesting_pda).unwrap();
    let start_slot = read_vesting_start_slot(&vesting_account.data);

    // Warp to 50% vesting
    svm.warp_to_slot(start_slot + 500);
    svm.expire_blockhash();

    // Cancel at 50%
    let cancel_ix = anchor_instruction(
        program_id,
        "cancel_vesting",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            writable_meta(authority_token),
            writable_meta(beneficiary_token),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, cancel_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "cancel_vesting should succeed: {:?}", result);

    // Verify beneficiary got 50% (vested amount)
    let beneficiary_account = svm.get_account(&beneficiary_token).unwrap();
    assert_eq!(read_token_balance(&beneficiary_account.data), 500, "Beneficiary should get 50%");

    // Verify authority got 50% back (unvested)
    let authority_account = svm.get_account(&authority_token).unwrap();
    assert_eq!(read_token_balance(&authority_account.data), 9500, "Authority should get unvested tokens back");
}

#[test]
fn test_cancel_vesting_unauthorized_fails() {
    let mut svm = load_vesting_program();
    let program_id = vesting_program_id();

    // Setup
    let (_, mint) = setup_mint(&mut svm, 6);
    let (authority, authority_token) = setup_user_with_tokens(&mut svm, &mint, 10000);
    let (beneficiary, beneficiary_token) = setup_user_with_empty_token_account(&mut svm, &mint);
    let (attacker, attacker_token) = setup_user_with_empty_token_account(&mut svm, &mint);

    let (vesting_pda, _) = derive_vesting_pda(&beneficiary.pubkey(), &mint);
    let (vault_pda, _) = derive_vault_pda(&beneficiary.pubkey(), &mint);

    // Create vesting
    let create_ix = anchor_instruction(
        program_id,
        "create_vesting",
        &create_vesting_data(&beneficiary.pubkey(), 1000, 0, 1000),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint),
            writable_meta(authority_token),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Attacker tries to cancel
    svm.expire_blockhash();
    let cancel_ix = anchor_instruction(
        program_id,
        "cancel_vesting",
        &[],
        vec![
            signer_meta(attacker.pubkey()),       // Wrong signer!
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            writable_meta(attacker_token),
            writable_meta(beneficiary_token),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, cancel_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "Unauthorized cancel should fail");
}

#[test]
fn test_cancel_vesting_already_cancelled_fails() {
    let mut svm = load_vesting_program();
    let program_id = vesting_program_id();

    // Setup
    let (_, mint) = setup_mint(&mut svm, 6);
    let (authority, authority_token) = setup_user_with_tokens(&mut svm, &mint, 10000);
    let (beneficiary, beneficiary_token) = setup_user_with_empty_token_account(&mut svm, &mint);

    let (vesting_pda, _) = derive_vesting_pda(&beneficiary.pubkey(), &mint);
    let (vault_pda, _) = derive_vault_pda(&beneficiary.pubkey(), &mint);

    // Create vesting
    let create_ix = anchor_instruction(
        program_id,
        "create_vesting",
        &create_vesting_data(&beneficiary.pubkey(), 1000, 0, 1000),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint),
            writable_meta(authority_token),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Cancel first time
    svm.expire_blockhash();
    let cancel_ix = anchor_instruction(
        program_id,
        "cancel_vesting",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            writable_meta(authority_token),
            writable_meta(beneficiary_token),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, cancel_ix, &authority, &[&authority]).unwrap();

    // Try to cancel again
    svm.expire_blockhash();
    let cancel_ix2 = anchor_instruction(
        program_id,
        "cancel_vesting",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            writable_meta(authority_token),
            writable_meta(beneficiary_token),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, cancel_ix2, &authority, &[&authority]);
    assert!(result.is_err(), "Cancel already cancelled vesting should fail");
}

// =============================================================================
// FULL WORKFLOW TESTS
// =============================================================================

#[test]
fn test_full_vesting_workflow() {
    let mut svm = load_vesting_program();
    let program_id = vesting_program_id();

    // Setup
    let (_, mint) = setup_mint(&mut svm, 6);
    let (authority, authority_token) = setup_user_with_tokens(&mut svm, &mint, 10000);
    let (beneficiary, beneficiary_token) = setup_user_with_empty_token_account(&mut svm, &mint);

    let (vesting_pda, _) = derive_vesting_pda(&beneficiary.pubkey(), &mint);
    let (vault_pda, _) = derive_vault_pda(&beneficiary.pubkey(), &mint);

    // Create vesting: 1000 tokens, 100 slot cliff, 900 slot vesting
    let create_ix = anchor_instruction(
        program_id,
        "create_vesting",
        &create_vesting_data(&beneficiary.pubkey(), 1000, 100, 900),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint),
            writable_meta(authority_token),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    let vesting_account = svm.get_account(&vesting_pda).unwrap();
    let cliff_slot = read_vesting_cliff_slot(&vesting_account.data);
    let end_slot = read_vesting_end_slot(&vesting_account.data);

    // Verify before cliff: nothing can be claimed
    let beneficiary_account = svm.get_account(&beneficiary_token).unwrap();
    assert_eq!(read_token_balance(&beneficiary_account.data), 0);

    // Warp past cliff + 450 slots (50% of vesting period)
    svm.warp_to_slot(cliff_slot + 450);
    svm.expire_blockhash();

    // Claim first batch
    let claim_ix = anchor_instruction(
        program_id,
        "claim_vested",
        &[],
        vec![
            signer_meta(beneficiary.pubkey()),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            writable_meta(beneficiary_token),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, claim_ix, &beneficiary, &[&beneficiary]).unwrap();

    let beneficiary_account = svm.get_account(&beneficiary_token).unwrap();
    let first_claim = read_token_balance(&beneficiary_account.data);
    assert_eq!(first_claim, 500, "Should claim 50%");

    // Warp past end
    svm.warp_to_slot(end_slot + 100);
    svm.expire_blockhash();

    // Claim remaining
    let claim_ix2 = anchor_instruction(
        program_id,
        "claim_vested",
        &[],
        vec![
            signer_meta(beneficiary.pubkey()),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            writable_meta(beneficiary_token),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, claim_ix2, &beneficiary, &[&beneficiary]).unwrap();

    let beneficiary_account = svm.get_account(&beneficiary_token).unwrap();
    let final_balance = read_token_balance(&beneficiary_account.data);
    assert_eq!(final_balance, 1000, "Should have all tokens");

    // Verify vault is empty
    let vault_account = svm.get_account(&vault_pda).unwrap();
    assert_eq!(read_token_balance(&vault_account.data), 0);

    // Verify vesting state
    let vesting_account = svm.get_account(&vesting_pda).unwrap();
    assert_eq!(read_vesting_released_amount(&vesting_account.data), 1000);
    assert!(!read_vesting_is_cancelled(&vesting_account.data));
}

#[test]
fn test_claim_from_cancelled_vesting_fails() {
    let mut svm = load_vesting_program();
    let program_id = vesting_program_id();

    // Setup
    let (_, mint) = setup_mint(&mut svm, 6);
    let (authority, authority_token) = setup_user_with_tokens(&mut svm, &mint, 10000);
    let (beneficiary, beneficiary_token) = setup_user_with_empty_token_account(&mut svm, &mint);

    let (vesting_pda, _) = derive_vesting_pda(&beneficiary.pubkey(), &mint);
    let (vault_pda, _) = derive_vault_pda(&beneficiary.pubkey(), &mint);

    // Create and cancel vesting
    let create_ix = anchor_instruction(
        program_id,
        "create_vesting",
        &create_vesting_data(&beneficiary.pubkey(), 1000, 0, 1000),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint),
            writable_meta(authority_token),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    svm.expire_blockhash();
    let cancel_ix = anchor_instruction(
        program_id,
        "cancel_vesting",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            writable_meta(authority_token),
            writable_meta(beneficiary_token),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, cancel_ix, &authority, &[&authority]).unwrap();

    // Try to claim from cancelled vesting
    svm.expire_blockhash();
    let claim_ix = anchor_instruction(
        program_id,
        "claim_vested",
        &[],
        vec![
            signer_meta(beneficiary.pubkey()),
            writable_meta(vesting_pda),
            writable_meta(vault_pda),
            writable_meta(beneficiary_token),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, claim_ix, &beneficiary, &[&beneficiary]);
    assert!(result.is_err(), "Claim from cancelled vesting should fail");
}
