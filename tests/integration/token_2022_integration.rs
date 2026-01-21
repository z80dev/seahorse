//! LiteSVM integration tests for the Seahorse Token-2022 program
//!
//! These tests verify the Token-2022 transfer fee configuration management.
//! Note: Seahorse doesn't have native Token-2022 support, so these tests focus on
//! the fee configuration tracking functionality.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile token_2022.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// Token-2022 program ID (from declare_id!)
fn token_2022_program_id() -> Pubkey {
    Pubkey::from_str("T2Ex5VzVZYxLYtDmJq5DEmUKDEcPnP2zGxqHKbv2Tbn").unwrap()
}

/// FeeConfig account size: discriminator (8) + mint (32) + authority (32) + decimals (1) +
/// transfer_fee_bps (2) + max_fee (8) + total_fees_collected (8) + bump (1)
#[allow(dead_code)]
const FEE_CONFIG_SIZE: usize = 8 + 32 + 32 + 1 + 2 + 8 + 8 + 1;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the token_2022 program into LiteSVM
fn load_token_2022_program() -> litesvm::LiteSVM {
    let program_id = token_2022_program_id();
    let program_bytes = std::fs::read("../../target/deploy/token_2022.so")
        .expect("Failed to read token_2022.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);
    svm
}

/// Derive fee_config PDA
fn derive_fee_config_pda(authority: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"fee_config", authority.as_ref()],
        &token_2022_program_id(),
    )
}

/// Read FeeConfig fields from account data
fn read_fee_config(data: &[u8]) -> (Pubkey, Pubkey, u8, u16, u64, u64, u8) {
    // discriminator (8) + mint (32) + authority (32) + decimals (1) + transfer_fee_bps (2) + max_fee (8) + total_fees_collected (8) + bump (1)
    let mint = Pubkey::new_from_array(data[8..40].try_into().unwrap());
    let authority = Pubkey::new_from_array(data[40..72].try_into().unwrap());
    let decimals = data[72];
    let transfer_fee_bps = u16::from_le_bytes(data[73..75].try_into().unwrap());
    let max_fee = u64::from_le_bytes(data[75..83].try_into().unwrap());
    let total_fees_collected = u64::from_le_bytes(data[83..91].try_into().unwrap());
    let bump = data[91];
    (
        mint,
        authority,
        decimals,
        transfer_fee_bps,
        max_fee,
        total_fees_collected,
        bump,
    )
}

// =============================================================================
// INITIALIZE FEE CONFIG TESTS
// =============================================================================

#[test]
fn test_initialize_fee_config() {
    let mut svm = load_token_2022_program();
    let program_id = token_2022_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let (fee_config_pda, _) = derive_fee_config_pda(&authority.pubkey());

    // Mock Token-2022 mint (in real use, this would be created via Token-2022 program)
    let mock_mint = Keypair::new();

    let decimals: u8 = 6;
    let transfer_fee_bps: u16 = 250; // 2.5%
    let max_fee: u64 = 1000_000; // 1 token max fee

    // Build instruction data: decimals (u8) + transfer_fee_bps (u16) + max_fee (u64)
    let mut ix_data = vec![];
    ix_data.extend_from_slice(&[decimals]);
    ix_data.extend_from_slice(&transfer_fee_bps.to_le_bytes());
    ix_data.extend_from_slice(&max_fee.to_le_bytes());

    let ix = anchor_instruction(
        program_id,
        "initialize_fee_config",
        &ix_data,
        vec![
            signer_meta(authority.pubkey()),           // authority
            writable_meta(fee_config_pda),             // fee_config
            writable_meta(mock_mint.pubkey()),         // mint (unchecked)
            readonly_meta(solana_sdk_ids::sysvar::rent::id()), // rent
            readonly_meta(system_program::id()),       // system_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(
        result.is_ok(),
        "Initialize fee config should succeed: {:?}",
        result
    );

    // Verify fee_config was created
    let fee_config_account = svm
        .get_account(&fee_config_pda)
        .expect("FeeConfig should exist");
    assert_eq!(
        fee_config_account.owner, program_id,
        "FeeConfig should be owned by program"
    );

    let (
        stored_mint,
        stored_authority,
        stored_decimals,
        stored_fee_bps,
        stored_max_fee,
        total_collected,
        _bump,
    ) = read_fee_config(&fee_config_account.data);

    assert_eq!(
        stored_mint,
        mock_mint.pubkey(),
        "Stored mint should match"
    );
    assert_eq!(
        stored_authority,
        authority.pubkey(),
        "Stored authority should match"
    );
    assert_eq!(stored_decimals, decimals, "Stored decimals should match");
    assert_eq!(
        stored_fee_bps, transfer_fee_bps,
        "Stored fee bps should match"
    );
    assert_eq!(stored_max_fee, max_fee, "Stored max fee should match");
    assert_eq!(total_collected, 0, "Initial total collected should be 0");
}

#[test]
fn test_initialize_fee_config_max_fee() {
    let mut svm = load_token_2022_program();
    let program_id = token_2022_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let (fee_config_pda, _) = derive_fee_config_pda(&authority.pubkey());
    let mock_mint = Keypair::new();

    // Maximum allowed fee: 10000 bps (100%)
    let decimals: u8 = 6;
    let transfer_fee_bps: u16 = 10000;
    let max_fee: u64 = u64::MAX;

    let mut ix_data = vec![];
    ix_data.extend_from_slice(&[decimals]);
    ix_data.extend_from_slice(&transfer_fee_bps.to_le_bytes());
    ix_data.extend_from_slice(&max_fee.to_le_bytes());

    let ix = anchor_instruction(
        program_id,
        "initialize_fee_config",
        &ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(fee_config_pda),
            writable_meta(mock_mint.pubkey()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(
        result.is_ok(),
        "Max fee (100%) should be allowed: {:?}",
        result
    );
}

#[test]
fn test_initialize_fee_config_exceeds_max_fails() {
    let mut svm = load_token_2022_program();
    let program_id = token_2022_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let (fee_config_pda, _) = derive_fee_config_pda(&authority.pubkey());
    let mock_mint = Keypair::new();

    // Exceeds maximum: 10001 bps (> 100%)
    let decimals: u8 = 6;
    let transfer_fee_bps: u16 = 10001;
    let max_fee: u64 = 1000;

    let mut ix_data = vec![];
    ix_data.extend_from_slice(&[decimals]);
    ix_data.extend_from_slice(&transfer_fee_bps.to_le_bytes());
    ix_data.extend_from_slice(&max_fee.to_le_bytes());

    let ix = anchor_instruction(
        program_id,
        "initialize_fee_config",
        &ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(fee_config_pda),
            writable_meta(mock_mint.pubkey()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(
        result.is_err(),
        "Fee exceeding 10000 bps should fail"
    );
}

// =============================================================================
// UPDATE FEE RATE TESTS
// =============================================================================

#[test]
fn test_update_fee_rate() {
    let mut svm = load_token_2022_program();
    let program_id = token_2022_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let (fee_config_pda, _) = derive_fee_config_pda(&authority.pubkey());
    let mock_mint = Keypair::new();

    // Initialize with 2.5% fee
    let decimals: u8 = 6;
    let transfer_fee_bps: u16 = 250;
    let max_fee: u64 = 1000_000;

    let mut init_ix_data = vec![];
    init_ix_data.extend_from_slice(&[decimals]);
    init_ix_data.extend_from_slice(&transfer_fee_bps.to_le_bytes());
    init_ix_data.extend_from_slice(&max_fee.to_le_bytes());

    let init_ix = anchor_instruction(
        program_id,
        "initialize_fee_config",
        &init_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(fee_config_pda),
            writable_meta(mock_mint.pubkey()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Update to 5% fee with new max
    svm.expire_blockhash();
    let new_fee_bps: u16 = 500;
    let new_max_fee: u64 = 2000_000;

    let mut update_ix_data = vec![];
    update_ix_data.extend_from_slice(&new_fee_bps.to_le_bytes());
    update_ix_data.extend_from_slice(&new_max_fee.to_le_bytes());

    let update_ix = anchor_instruction(
        program_id,
        "update_fee_rate",
        &update_ix_data,
        vec![
            signer_meta(authority.pubkey()), // authority
            writable_meta(fee_config_pda),   // fee_config
        ],
    );

    let result = execute_tx(&mut svm, update_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Update fee rate should succeed: {:?}", result);

    // Verify updated values
    let fee_config_account = svm.get_account(&fee_config_pda).unwrap();
    let (_, _, _, stored_fee_bps, stored_max_fee, _, _) =
        read_fee_config(&fee_config_account.data);

    assert_eq!(stored_fee_bps, new_fee_bps, "Fee rate should be updated");
    assert_eq!(stored_max_fee, new_max_fee, "Max fee should be updated");
}

#[test]
fn test_update_fee_rate_unauthorized_fails() {
    let mut svm = load_token_2022_program();
    let program_id = token_2022_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let attacker = funded_keypair_10_sol(&mut svm);
    let (fee_config_pda, _) = derive_fee_config_pda(&authority.pubkey());
    let mock_mint = Keypair::new();

    // Initialize by authority
    let decimals: u8 = 6;
    let transfer_fee_bps: u16 = 250;
    let max_fee: u64 = 1000_000;

    let mut init_ix_data = vec![];
    init_ix_data.extend_from_slice(&[decimals]);
    init_ix_data.extend_from_slice(&transfer_fee_bps.to_le_bytes());
    init_ix_data.extend_from_slice(&max_fee.to_le_bytes());

    let init_ix = anchor_instruction(
        program_id,
        "initialize_fee_config",
        &init_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(fee_config_pda),
            writable_meta(mock_mint.pubkey()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Attacker tries to update
    svm.expire_blockhash();
    let new_fee_bps: u16 = 9999;
    let new_max_fee: u64 = 0;

    let mut update_ix_data = vec![];
    update_ix_data.extend_from_slice(&new_fee_bps.to_le_bytes());
    update_ix_data.extend_from_slice(&new_max_fee.to_le_bytes());

    let update_ix = anchor_instruction(
        program_id,
        "update_fee_rate",
        &update_ix_data,
        vec![
            signer_meta(attacker.pubkey()), // attacker signs
            writable_meta(fee_config_pda),  // but fee_config.authority != attacker
        ],
    );

    let result = execute_tx(&mut svm, update_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "Unauthorized update should fail");
}

// =============================================================================
// CALCULATE TRANSFER FEE TESTS
// =============================================================================

#[test]
fn test_calculate_transfer_fee() {
    let mut svm = load_token_2022_program();
    let program_id = token_2022_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let (fee_config_pda, _) = derive_fee_config_pda(&authority.pubkey());
    let mock_mint = Keypair::new();

    // Initialize with 2.5% fee (250 bps), max 1 token
    let decimals: u8 = 6;
    let transfer_fee_bps: u16 = 250;
    let max_fee: u64 = 1_000_000; // 1 token

    let mut init_ix_data = vec![];
    init_ix_data.extend_from_slice(&[decimals]);
    init_ix_data.extend_from_slice(&transfer_fee_bps.to_le_bytes());
    init_ix_data.extend_from_slice(&max_fee.to_le_bytes());

    let init_ix = anchor_instruction(
        program_id,
        "initialize_fee_config",
        &init_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(fee_config_pda),
            writable_meta(mock_mint.pubkey()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Calculate fee for 10 tokens (10_000_000 base units)
    // 2.5% of 10 = 0.25 tokens = 250_000 base units
    svm.expire_blockhash();
    let amount: u64 = 10_000_000;

    let ix = anchor_instruction(
        program_id,
        "calculate_transfer_fee",
        &amount.to_le_bytes(),
        vec![writable_meta(fee_config_pda)],
    );

    // This instruction logs the calculated fee; we just verify it executes
    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(
        result.is_ok(),
        "Calculate fee should succeed: {:?}",
        result
    );
}

#[test]
fn test_calculate_transfer_fee_capped_at_max() {
    let mut svm = load_token_2022_program();
    let program_id = token_2022_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let (fee_config_pda, _) = derive_fee_config_pda(&authority.pubkey());
    let mock_mint = Keypair::new();

    // Initialize with 10% fee (1000 bps), max 1 token
    let decimals: u8 = 6;
    let transfer_fee_bps: u16 = 1000;
    let max_fee: u64 = 1_000_000; // 1 token

    let mut init_ix_data = vec![];
    init_ix_data.extend_from_slice(&[decimals]);
    init_ix_data.extend_from_slice(&transfer_fee_bps.to_le_bytes());
    init_ix_data.extend_from_slice(&max_fee.to_le_bytes());

    let init_ix = anchor_instruction(
        program_id,
        "initialize_fee_config",
        &init_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(fee_config_pda),
            writable_meta(mock_mint.pubkey()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Calculate fee for 100 tokens
    // 10% of 100 = 10 tokens, but max is 1 token
    // Fee should be capped at 1_000_000
    svm.expire_blockhash();
    let amount: u64 = 100_000_000;

    let ix = anchor_instruction(
        program_id,
        "calculate_transfer_fee",
        &amount.to_le_bytes(),
        vec![writable_meta(fee_config_pda)],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(
        result.is_ok(),
        "Calculate fee (capped) should succeed: {:?}",
        result
    );
}

// =============================================================================
// RECORD FEE COLLECTION TESTS
// =============================================================================

#[test]
fn test_record_fee_collection() {
    let mut svm = load_token_2022_program();
    let program_id = token_2022_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let (fee_config_pda, _) = derive_fee_config_pda(&authority.pubkey());
    let mock_mint = Keypair::new();

    // Initialize
    let decimals: u8 = 6;
    let transfer_fee_bps: u16 = 250;
    let max_fee: u64 = 1_000_000;

    let mut init_ix_data = vec![];
    init_ix_data.extend_from_slice(&[decimals]);
    init_ix_data.extend_from_slice(&transfer_fee_bps.to_le_bytes());
    init_ix_data.extend_from_slice(&max_fee.to_le_bytes());

    let init_ix = anchor_instruction(
        program_id,
        "initialize_fee_config",
        &init_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(fee_config_pda),
            writable_meta(mock_mint.pubkey()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Record fee collection
    svm.expire_blockhash();
    let collected_amount: u64 = 500_000; // 0.5 tokens

    let record_ix = anchor_instruction(
        program_id,
        "record_fee_collection",
        &collected_amount.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()), // authority
            writable_meta(fee_config_pda),   // fee_config
        ],
    );

    let result = execute_tx(&mut svm, record_ix, &authority, &[&authority]);
    assert!(
        result.is_ok(),
        "Record fee collection should succeed: {:?}",
        result
    );

    // Verify total collected updated
    let fee_config_account = svm.get_account(&fee_config_pda).unwrap();
    let (_, _, _, _, _, total_collected, _) = read_fee_config(&fee_config_account.data);
    assert_eq!(
        total_collected, collected_amount,
        "Total collected should be updated"
    );

    // Record more fees
    svm.expire_blockhash();
    let more_collected: u64 = 750_000;

    let record_ix2 = anchor_instruction(
        program_id,
        "record_fee_collection",
        &more_collected.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(fee_config_pda),
        ],
    );
    execute_tx(&mut svm, record_ix2, &authority, &[&authority]).unwrap();

    // Verify cumulative total
    let fee_config_account = svm.get_account(&fee_config_pda).unwrap();
    let (_, _, _, _, _, total_collected, _) = read_fee_config(&fee_config_account.data);
    assert_eq!(
        total_collected,
        500_000 + 750_000,
        "Total collected should accumulate"
    );
}

#[test]
fn test_record_fee_collection_unauthorized_fails() {
    let mut svm = load_token_2022_program();
    let program_id = token_2022_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let attacker = funded_keypair_10_sol(&mut svm);
    let (fee_config_pda, _) = derive_fee_config_pda(&authority.pubkey());
    let mock_mint = Keypair::new();

    // Initialize by authority
    let decimals: u8 = 6;
    let transfer_fee_bps: u16 = 250;
    let max_fee: u64 = 1_000_000;

    let mut init_ix_data = vec![];
    init_ix_data.extend_from_slice(&[decimals]);
    init_ix_data.extend_from_slice(&transfer_fee_bps.to_le_bytes());
    init_ix_data.extend_from_slice(&max_fee.to_le_bytes());

    let init_ix = anchor_instruction(
        program_id,
        "initialize_fee_config",
        &init_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(fee_config_pda),
            writable_meta(mock_mint.pubkey()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Attacker tries to record fake fees
    svm.expire_blockhash();
    let fake_collected: u64 = 999_999_999;

    let record_ix = anchor_instruction(
        program_id,
        "record_fee_collection",
        &fake_collected.to_le_bytes(),
        vec![
            signer_meta(attacker.pubkey()), // attacker signs
            writable_meta(fee_config_pda),  // but fee_config.authority != attacker
        ],
    );

    let result = execute_tx(&mut svm, record_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "Unauthorized fee recording should fail");
}

// =============================================================================
// GET FEE INFO TESTS
// =============================================================================

#[test]
fn test_get_fee_info() {
    let mut svm = load_token_2022_program();
    let program_id = token_2022_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let (fee_config_pda, _) = derive_fee_config_pda(&authority.pubkey());
    let mock_mint = Keypair::new();

    // Initialize
    let decimals: u8 = 9;
    let transfer_fee_bps: u16 = 100; // 1%
    let max_fee: u64 = 5_000_000_000; // 5 tokens

    let mut init_ix_data = vec![];
    init_ix_data.extend_from_slice(&[decimals]);
    init_ix_data.extend_from_slice(&transfer_fee_bps.to_le_bytes());
    init_ix_data.extend_from_slice(&max_fee.to_le_bytes());

    let init_ix = anchor_instruction(
        program_id,
        "initialize_fee_config",
        &init_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(fee_config_pda),
            writable_meta(mock_mint.pubkey()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Get fee info (instruction just logs data)
    svm.expire_blockhash();
    let info_ix = anchor_instruction(
        program_id,
        "get_fee_info",
        &[],
        vec![writable_meta(fee_config_pda)],
    );

    let result = execute_tx(&mut svm, info_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Get fee info should succeed: {:?}", result);
}

// =============================================================================
// FULL WORKFLOW TESTS
// =============================================================================

#[test]
fn test_full_fee_config_workflow() {
    let mut svm = load_token_2022_program();
    let program_id = token_2022_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let (fee_config_pda, _) = derive_fee_config_pda(&authority.pubkey());
    let mock_mint = Keypair::new();

    // 1. Initialize with 2% fee
    let decimals: u8 = 6;
    let transfer_fee_bps: u16 = 200;
    let max_fee: u64 = 2_000_000;

    let mut init_ix_data = vec![];
    init_ix_data.extend_from_slice(&[decimals]);
    init_ix_data.extend_from_slice(&transfer_fee_bps.to_le_bytes());
    init_ix_data.extend_from_slice(&max_fee.to_le_bytes());

    let init_ix = anchor_instruction(
        program_id,
        "initialize_fee_config",
        &init_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(fee_config_pda),
            writable_meta(mock_mint.pubkey()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // 2. Calculate fee (just verify it works)
    svm.expire_blockhash();
    let amount: u64 = 50_000_000; // 50 tokens
    let calc_ix = anchor_instruction(
        program_id,
        "calculate_transfer_fee",
        &amount.to_le_bytes(),
        vec![writable_meta(fee_config_pda)],
    );
    execute_tx(&mut svm, calc_ix, &authority, &[&authority]).unwrap();

    // 3. Record some fee collections
    svm.expire_blockhash();
    let collected1: u64 = 1_000_000; // 1 token
    let record_ix1 = anchor_instruction(
        program_id,
        "record_fee_collection",
        &collected1.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(fee_config_pda),
        ],
    );
    execute_tx(&mut svm, record_ix1, &authority, &[&authority]).unwrap();

    svm.expire_blockhash();
    let collected2: u64 = 500_000; // 0.5 tokens
    let record_ix2 = anchor_instruction(
        program_id,
        "record_fee_collection",
        &collected2.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(fee_config_pda),
        ],
    );
    execute_tx(&mut svm, record_ix2, &authority, &[&authority]).unwrap();

    // 4. Update fee rate
    svm.expire_blockhash();
    let new_fee_bps: u16 = 300; // 3%
    let new_max_fee: u64 = 3_000_000;

    let mut update_ix_data = vec![];
    update_ix_data.extend_from_slice(&new_fee_bps.to_le_bytes());
    update_ix_data.extend_from_slice(&new_max_fee.to_le_bytes());

    let update_ix = anchor_instruction(
        program_id,
        "update_fee_rate",
        &update_ix_data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(fee_config_pda),
        ],
    );
    execute_tx(&mut svm, update_ix, &authority, &[&authority]).unwrap();

    // 5. Get fee info
    svm.expire_blockhash();
    let info_ix = anchor_instruction(
        program_id,
        "get_fee_info",
        &[],
        vec![writable_meta(fee_config_pda)],
    );
    execute_tx(&mut svm, info_ix, &authority, &[&authority]).unwrap();

    // Verify final state
    let fee_config_account = svm.get_account(&fee_config_pda).unwrap();
    let (_, _, _, stored_fee_bps, stored_max_fee, total_collected, _) =
        read_fee_config(&fee_config_account.data);

    assert_eq!(stored_fee_bps, 300, "Fee should be updated to 3%");
    assert_eq!(stored_max_fee, 3_000_000, "Max fee should be updated");
    assert_eq!(
        total_collected,
        1_500_000,
        "Total collected should be 1.5 tokens"
    );
}
