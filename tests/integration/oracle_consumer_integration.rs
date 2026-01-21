//! LiteSVM integration tests for the Seahorse Oracle Consumer program
//!
//! These tests verify:
//! - create_price_config: Creates price feed configuration PDA
//! - update_price_config: Updates configuration (authority only)
//! - record_price: Records price with staleness validation
//! - create_price_threshold: Creates price threshold monitoring
//! - check_threshold: Checks if threshold is crossed
//! - reset_threshold: Resets triggered threshold
//! - get_price_info: Displays price and staleness status
//!
//! Uses LiteSVM's warp_to_slot for staleness testing.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile oracle_consumer.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// Oracle Consumer program ID (from declare_id!)
fn oracle_consumer_program_id() -> Pubkey {
    Pubkey::from_str("3ePvSJdgkK1r5CrxyEjJoMnSCJJjCMoRKNdYLi6Ws8g7").unwrap()
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the oracle consumer program into LiteSVM
fn load_oracle_consumer_program() -> litesvm::LiteSVM {
    let program_id = oracle_consumer_program_id();
    let program_bytes = std::fs::read("../../target/deploy/oracle_consumer.so")
        .expect("Failed to read oracle_consumer.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);
    svm
}

/// Derive price config PDA
fn derive_price_config_pda(config_id: u64) -> (Pubkey, u8) {
    find_pda(
        &[b"price_config", config_id.to_le_bytes().as_ref()],
        &oracle_consumer_program_id(),
    )
}

/// Derive price threshold PDA
fn derive_price_threshold_pda(config_id: u64, threshold_id: u64) -> (Pubkey, u8) {
    find_pda(
        &[
            b"threshold",
            config_id.to_le_bytes().as_ref(),
            threshold_id.to_le_bytes().as_ref(),
        ],
        &oracle_consumer_program_id(),
    )
}

/// Build create_price_config instruction data
fn create_price_config_data(
    config_id: u64,
    feed_name: &str,
    oracle_address: &Pubkey,
    max_staleness_slots: u64,
) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&config_id.to_le_bytes());
    data.extend_from_slice(&(feed_name.len() as u32).to_le_bytes());
    data.extend_from_slice(feed_name.as_bytes());
    data.extend_from_slice(oracle_address.as_ref());
    data.extend_from_slice(&max_staleness_slots.to_le_bytes());
    data
}

/// Build update_price_config instruction data
fn update_price_config_data(new_oracle_address: &Pubkey, new_max_staleness_slots: u64) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(new_oracle_address.as_ref());
    data.extend_from_slice(&new_max_staleness_slots.to_le_bytes());
    data
}

/// Build record_price instruction data
fn record_price_data(
    oracle_price: i64,
    oracle_confidence: u64,
    oracle_exponent: i32,
    oracle_publish_slot: u64,
) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&oracle_price.to_le_bytes());
    data.extend_from_slice(&oracle_confidence.to_le_bytes());
    data.extend_from_slice(&oracle_exponent.to_le_bytes());
    data.extend_from_slice(&oracle_publish_slot.to_le_bytes());
    data
}

/// Build create_price_threshold instruction data
fn create_price_threshold_data(
    config_id: u64,
    threshold_id: u64,
    trigger_above: bool,
    target_price_scaled: u64,
) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&config_id.to_le_bytes());
    data.extend_from_slice(&threshold_id.to_le_bytes());
    data.push(if trigger_above { 1 } else { 0 });
    data.extend_from_slice(&target_price_scaled.to_le_bytes());
    data
}

/// Read PriceConfig fields from account data
fn read_price_config_authority(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

fn read_price_config_id(data: &[u8]) -> u64 {
    u64::from_le_bytes(data[40..48].try_into().unwrap())
}

fn read_price_config_feed_name(data: &[u8]) -> String {
    let len = u32::from_le_bytes(data[48..52].try_into().unwrap()) as usize;
    String::from_utf8(data[52..52 + len].to_vec()).unwrap()
}

fn read_price_config_oracle_address(data: &[u8]) -> Pubkey {
    let feed_name_len = u32::from_le_bytes(data[48..52].try_into().unwrap()) as usize;
    let offset = 52 + feed_name_len;
    Pubkey::new_from_array(data[offset..offset + 32].try_into().unwrap())
}

fn read_price_config_max_staleness(data: &[u8]) -> u64 {
    let feed_name_len = u32::from_le_bytes(data[48..52].try_into().unwrap()) as usize;
    let offset = 52 + feed_name_len + 32;
    u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap())
}

fn read_price_config_last_price(data: &[u8]) -> i64 {
    let feed_name_len = u32::from_le_bytes(data[48..52].try_into().unwrap()) as usize;
    let offset = 52 + feed_name_len + 32 + 8;
    i64::from_le_bytes(data[offset..offset + 8].try_into().unwrap())
}

fn read_price_config_last_confidence(data: &[u8]) -> u64 {
    let feed_name_len = u32::from_le_bytes(data[48..52].try_into().unwrap()) as usize;
    let offset = 52 + feed_name_len + 32 + 8 + 8;
    u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap())
}

fn read_price_config_last_exponent(data: &[u8]) -> i32 {
    let feed_name_len = u32::from_le_bytes(data[48..52].try_into().unwrap()) as usize;
    let offset = 52 + feed_name_len + 32 + 8 + 8 + 8;
    i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn read_price_config_update_count(data: &[u8]) -> u64 {
    let feed_name_len = u32::from_le_bytes(data[48..52].try_into().unwrap()) as usize;
    let offset = 52 + feed_name_len + 32 + 8 + 8 + 8 + 4 + 8;
    u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap())
}

/// Read PriceThreshold fields from account data
fn read_price_threshold_config(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

fn read_price_threshold_id(data: &[u8]) -> u64 {
    u64::from_le_bytes(data[40..48].try_into().unwrap())
}

fn read_price_threshold_trigger_above(data: &[u8]) -> bool {
    data[48] != 0
}

fn read_price_threshold_target_price(data: &[u8]) -> u64 {
    u64::from_le_bytes(data[49..57].try_into().unwrap())
}

fn read_price_threshold_is_triggered(data: &[u8]) -> bool {
    data[57] != 0
}

fn read_price_threshold_triggered_at_slot(data: &[u8]) -> u64 {
    u64::from_le_bytes(data[58..66].try_into().unwrap())
}

// =============================================================================
// TESTS
// =============================================================================

#[test]
fn test_create_price_config() {
    let mut svm = load_oracle_consumer_program();
    let program_id = oracle_consumer_program_id();
    let authority = funded_keypair_10_sol(&mut svm);
    let oracle_address = Pubkey::new_unique();
    let config_id = 1u64;
    let feed_name = "SOL/USD";
    let max_staleness_slots = 100u64;

    let (price_config_pda, _) = derive_price_config_pda(config_id);

    let ix = anchor_instruction(
        program_id,
        "create_price_config",
        &create_price_config_data(config_id, feed_name, &oracle_address, max_staleness_slots),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "create_price_config failed: {:?}", result);

    let account = svm.get_account(&price_config_pda).expect("Price config not found");

    assert_eq!(read_price_config_authority(&account.data), authority.pubkey());
    assert_eq!(read_price_config_id(&account.data), config_id);
    assert_eq!(read_price_config_feed_name(&account.data), feed_name);
    assert_eq!(read_price_config_oracle_address(&account.data), oracle_address);
    assert_eq!(read_price_config_max_staleness(&account.data), max_staleness_slots);
    assert_eq!(read_price_config_last_price(&account.data), 0);
    assert_eq!(read_price_config_update_count(&account.data), 0);
}

#[test]
fn test_create_price_config_feed_name_too_long() {
    let mut svm = load_oracle_consumer_program();
    let program_id = oracle_consumer_program_id();
    let authority = funded_keypair_10_sol(&mut svm);
    let oracle_address = Pubkey::new_unique();
    let config_id = 1u64;
    let long_name = "A".repeat(33);

    let (price_config_pda, _) = derive_price_config_pda(config_id);

    let ix = anchor_instruction(
        program_id,
        "create_price_config",
        &create_price_config_data(config_id, &long_name, &oracle_address, 100),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "Should fail with name too long");
}

#[test]
fn test_update_price_config() {
    let mut svm = load_oracle_consumer_program();
    let program_id = oracle_consumer_program_id();
    let authority = funded_keypair_10_sol(&mut svm);
    let oracle_address1 = Pubkey::new_unique();
    let oracle_address2 = Pubkey::new_unique();
    let config_id = 1u64;

    let (price_config_pda, _) = derive_price_config_pda(config_id);

    // Create config
    let ix1 = anchor_instruction(
        program_id,
        "create_price_config",
        &create_price_config_data(config_id, "SOL/USD", &oracle_address1, 100),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix1, &authority, &[&authority]).unwrap();

    // Update config
    let ix2 = anchor_instruction(
        program_id,
        "update_price_config",
        &update_price_config_data(&oracle_address2, 200),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
        ],
    );
    svm.expire_blockhash();
    execute_tx(&mut svm, ix2, &authority, &[&authority]).unwrap();

    let account = svm.get_account(&price_config_pda).unwrap();
    assert_eq!(read_price_config_oracle_address(&account.data), oracle_address2);
    assert_eq!(read_price_config_max_staleness(&account.data), 200);
}

#[test]
fn test_record_price() {
    let mut svm = load_oracle_consumer_program();
    let program_id = oracle_consumer_program_id();
    let authority = funded_keypair_10_sol(&mut svm);
    let oracle_address = Pubkey::new_unique();
    let config_id = 1u64;

    let (price_config_pda, _) = derive_price_config_pda(config_id);

    // Create config
    let ix1 = anchor_instruction(
        program_id,
        "create_price_config",
        &create_price_config_data(config_id, "SOL/USD", &oracle_address, 100),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix1, &authority, &[&authority]).unwrap();

    // Warp to slot 10 so we have a valid publish slot
    svm.warp_to_slot(10);
    svm.expire_blockhash();

    // Record price (publish_slot=5, current=10, age=5 < max_staleness=100)
    let oracle_price = 15000000000i64; // $150.00 with 8 decimals
    let oracle_confidence = 100000u64;
    let oracle_exponent = -8i32;
    let oracle_publish_slot = 5u64;

    let ix2 = anchor_instruction(
        program_id,
        "record_price",
        &record_price_data(oracle_price, oracle_confidence, oracle_exponent, oracle_publish_slot),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
    );
    execute_tx(&mut svm, ix2, &authority, &[&authority]).unwrap();

    let account = svm.get_account(&price_config_pda).unwrap();
    assert_eq!(read_price_config_last_price(&account.data), oracle_price);
    assert_eq!(read_price_config_last_confidence(&account.data), oracle_confidence);
    assert_eq!(read_price_config_last_exponent(&account.data), oracle_exponent);
    assert_eq!(read_price_config_update_count(&account.data), 1);
}

#[test]
fn test_record_price_stale_fails() {
    let mut svm = load_oracle_consumer_program();
    let program_id = oracle_consumer_program_id();
    let authority = funded_keypair_10_sol(&mut svm);
    let oracle_address = Pubkey::new_unique();
    let config_id = 1u64;
    let max_staleness = 10u64;

    let (price_config_pda, _) = derive_price_config_pda(config_id);

    // Create config with small max_staleness
    let ix1 = anchor_instruction(
        program_id,
        "create_price_config",
        &create_price_config_data(config_id, "SOL/USD", &oracle_address, max_staleness),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix1, &authority, &[&authority]).unwrap();

    // Warp to slot 100
    svm.warp_to_slot(100);
    svm.expire_blockhash();

    // Try to record stale price (publish_slot=10, current=100, age=90 > max_staleness=10)
    let ix2 = anchor_instruction(
        program_id,
        "record_price",
        &record_price_data(15000000000i64, 100000u64, -8i32, 10u64),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
    );
    let result = execute_tx(&mut svm, ix2, &authority, &[&authority]);
    assert!(result.is_err(), "Should fail with stale price");
}

#[test]
fn test_create_price_threshold() {
    let mut svm = load_oracle_consumer_program();
    let program_id = oracle_consumer_program_id();
    let authority = funded_keypair_10_sol(&mut svm);
    let oracle_address = Pubkey::new_unique();
    let config_id = 1u64;
    let threshold_id = 1u64;

    let (price_config_pda, _) = derive_price_config_pda(config_id);
    let (price_threshold_pda, _) = derive_price_threshold_pda(config_id, threshold_id);

    // Create config
    let ix1 = anchor_instruction(
        program_id,
        "create_price_config",
        &create_price_config_data(config_id, "SOL/USD", &oracle_address, 100),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix1, &authority, &[&authority]).unwrap();

    // Create threshold
    let target_price = 20000000000u64; // $200 target
    let ix2 = anchor_instruction(
        program_id,
        "create_price_threshold",
        &create_price_threshold_data(config_id, threshold_id, true, target_price),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            writable_meta(price_threshold_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    svm.expire_blockhash();
    execute_tx(&mut svm, ix2, &authority, &[&authority]).unwrap();

    let account = svm.get_account(&price_threshold_pda).unwrap();
    assert_eq!(read_price_threshold_config(&account.data), price_config_pda);
    assert_eq!(read_price_threshold_id(&account.data), threshold_id);
    assert_eq!(read_price_threshold_trigger_above(&account.data), true);
    assert_eq!(read_price_threshold_target_price(&account.data), target_price);
    assert_eq!(read_price_threshold_is_triggered(&account.data), false);
}

#[test]
fn test_check_threshold_trigger_above() {
    let mut svm = load_oracle_consumer_program();
    let program_id = oracle_consumer_program_id();
    let authority = funded_keypair_10_sol(&mut svm);
    let oracle_address = Pubkey::new_unique();
    let config_id = 1u64;
    let threshold_id = 1u64;

    let (price_config_pda, _) = derive_price_config_pda(config_id);
    let (price_threshold_pda, _) = derive_price_threshold_pda(config_id, threshold_id);

    // Create config
    let ix1 = anchor_instruction(
        program_id,
        "create_price_config",
        &create_price_config_data(config_id, "SOL/USD", &oracle_address, 100),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix1, &authority, &[&authority]).unwrap();

    // Create threshold at $150 (trigger when above)
    let target_price = 15000000000u64;
    let ix2 = anchor_instruction(
        program_id,
        "create_price_threshold",
        &create_price_threshold_data(config_id, threshold_id, true, target_price),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            writable_meta(price_threshold_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    svm.expire_blockhash();
    execute_tx(&mut svm, ix2, &authority, &[&authority]).unwrap();

    // Record price at $200 (above threshold)
    svm.warp_to_slot(10);
    svm.expire_blockhash();

    let ix3 = anchor_instruction(
        program_id,
        "record_price",
        &record_price_data(20000000000i64, 100000u64, -8i32, 5u64),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
    );
    execute_tx(&mut svm, ix3, &authority, &[&authority]).unwrap();

    // Check threshold
    svm.expire_blockhash();
    let ix4 = anchor_instruction(
        program_id,
        "check_threshold",
        &[],
        vec![
            writable_meta(price_config_pda),
            writable_meta(price_threshold_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
    );
    execute_tx(&mut svm, ix4, &authority, &[&authority]).unwrap();

    let account = svm.get_account(&price_threshold_pda).unwrap();
    assert_eq!(read_price_threshold_is_triggered(&account.data), true);
    assert!(read_price_threshold_triggered_at_slot(&account.data) > 0);
}

#[test]
fn test_reset_threshold() {
    let mut svm = load_oracle_consumer_program();
    let program_id = oracle_consumer_program_id();
    let authority = funded_keypair_10_sol(&mut svm);
    let oracle_address = Pubkey::new_unique();
    let config_id = 1u64;
    let threshold_id = 1u64;

    let (price_config_pda, _) = derive_price_config_pda(config_id);
    let (price_threshold_pda, _) = derive_price_threshold_pda(config_id, threshold_id);

    // Create config and threshold
    let ix1 = anchor_instruction(
        program_id,
        "create_price_config",
        &create_price_config_data(config_id, "SOL/USD", &oracle_address, 100),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix1, &authority, &[&authority]).unwrap();

    let ix2 = anchor_instruction(
        program_id,
        "create_price_threshold",
        &create_price_threshold_data(config_id, threshold_id, true, 10000000000u64),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            writable_meta(price_threshold_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    svm.expire_blockhash();
    execute_tx(&mut svm, ix2, &authority, &[&authority]).unwrap();

    // Record price to trigger threshold
    svm.warp_to_slot(10);
    svm.expire_blockhash();

    let ix3 = anchor_instruction(
        program_id,
        "record_price",
        &record_price_data(20000000000i64, 100000u64, -8i32, 5u64),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
    );
    execute_tx(&mut svm, ix3, &authority, &[&authority]).unwrap();

    // Check threshold
    svm.expire_blockhash();
    let ix4 = anchor_instruction(
        program_id,
        "check_threshold",
        &[],
        vec![
            writable_meta(price_config_pda),
            writable_meta(price_threshold_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
    );
    execute_tx(&mut svm, ix4, &authority, &[&authority]).unwrap();

    // Verify triggered
    let account = svm.get_account(&price_threshold_pda).unwrap();
    assert_eq!(read_price_threshold_is_triggered(&account.data), true);

    // Reset threshold
    svm.expire_blockhash();
    let ix5 = anchor_instruction(
        program_id,
        "reset_threshold",
        &[],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            writable_meta(price_threshold_pda),
        ],
    );
    execute_tx(&mut svm, ix5, &authority, &[&authority]).unwrap();

    // Verify reset
    let account = svm.get_account(&price_threshold_pda).unwrap();
    assert_eq!(read_price_threshold_is_triggered(&account.data), false);
    assert_eq!(read_price_threshold_triggered_at_slot(&account.data), 0);
}

#[test]
fn test_full_oracle_consumer_workflow() {
    let mut svm = load_oracle_consumer_program();
    let program_id = oracle_consumer_program_id();
    let authority = funded_keypair_10_sol(&mut svm);
    let oracle_address = Pubkey::new_unique();
    let config_id = 1u64;
    let threshold_id_high = 1u64;
    let threshold_id_low = 2u64;

    let (price_config_pda, _) = derive_price_config_pda(config_id);
    let (threshold_high_pda, _) = derive_price_threshold_pda(config_id, threshold_id_high);
    let (threshold_low_pda, _) = derive_price_threshold_pda(config_id, threshold_id_low);

    // 1. Create price config
    let ix1 = anchor_instruction(
        program_id,
        "create_price_config",
        &create_price_config_data(config_id, "SOL/USD", &oracle_address, 100),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, ix1, &authority, &[&authority]).unwrap();

    // 2. Create high threshold at $180
    let ix2 = anchor_instruction(
        program_id,
        "create_price_threshold",
        &create_price_threshold_data(config_id, threshold_id_high, true, 18000000000u64),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            writable_meta(threshold_high_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    svm.expire_blockhash();
    execute_tx(&mut svm, ix2, &authority, &[&authority]).unwrap();

    // 3. Create low threshold at $100
    let ix3 = anchor_instruction(
        program_id,
        "create_price_threshold",
        &create_price_threshold_data(config_id, threshold_id_low, false, 10000000000u64),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            writable_meta(threshold_low_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    svm.expire_blockhash();
    execute_tx(&mut svm, ix3, &authority, &[&authority]).unwrap();

    // 4. Record price at $150 (between thresholds)
    svm.warp_to_slot(10);
    svm.expire_blockhash();

    let ix4 = anchor_instruction(
        program_id,
        "record_price",
        &record_price_data(15000000000i64, 100000u64, -8i32, 5u64),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
    );
    execute_tx(&mut svm, ix4, &authority, &[&authority]).unwrap();

    // 5. Check high threshold (should not trigger)
    svm.expire_blockhash();
    let ix5 = anchor_instruction(
        program_id,
        "check_threshold",
        &[],
        vec![
            writable_meta(price_config_pda),
            writable_meta(threshold_high_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
    );
    execute_tx(&mut svm, ix5, &authority, &[&authority]).unwrap();

    // Verify high not triggered
    assert_eq!(
        read_price_threshold_is_triggered(&svm.get_account(&threshold_high_pda).unwrap().data),
        false
    );

    // 6. Record price at $200 (above high threshold)
    svm.warp_to_slot(20);
    svm.expire_blockhash();

    let ix7 = anchor_instruction(
        program_id,
        "record_price",
        &record_price_data(20000000000i64, 100000u64, -8i32, 15u64),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
    );
    execute_tx(&mut svm, ix7, &authority, &[&authority]).unwrap();

    // 7. Check high threshold (should trigger now)
    svm.expire_blockhash();
    let ix8 = anchor_instruction(
        program_id,
        "check_threshold",
        &[],
        vec![
            writable_meta(price_config_pda),
            writable_meta(threshold_high_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
    );
    execute_tx(&mut svm, ix8, &authority, &[&authority]).unwrap();

    // Verify high threshold triggered
    assert_eq!(
        read_price_threshold_is_triggered(&svm.get_account(&threshold_high_pda).unwrap().data),
        true
    );

    // Verify update count
    assert_eq!(
        read_price_config_update_count(&svm.get_account(&price_config_pda).unwrap().data),
        2
    );
}
