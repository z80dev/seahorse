//! Behavior parity tests for Oracle Consumer program
//!
//! These tests verify that Seahorse oracle_consumer produces consistent behavior
//! for:
//! - Price config initialization state
//! - Staleness validation
//! - Price recording and update counting
//! - Threshold triggering logic
//! - PDA derivation
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile:
//! - target/deploy/oracle_consumer.so (Seahorse)

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

/// Oracle Consumer program ID
fn program_id() -> Pubkey {
    Pubkey::from_str("3ePvSJdgkK1r5CrxyEjJoMnSCJJjCMoRKNdYLi6Ws8g7").unwrap()
}

/// Derive price config PDA
fn derive_price_config_pda(config_id: u64, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"price_config", config_id.to_le_bytes().as_ref()],
        program_id,
    )
}

/// Derive price threshold PDA
fn derive_price_threshold_pda(
    config_id: u64,
    threshold_id: u64,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    find_pda(
        &[
            b"threshold",
            config_id.to_le_bytes().as_ref(),
            threshold_id.to_le_bytes().as_ref(),
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

/// Build create_price_config instruction data
fn create_price_config_data(
    config_id: u64,
    feed_name: &str,
    oracle_address: &Pubkey,
    max_staleness_slots: u64,
) -> Vec<u8> {
    let mut data = instruction_disc("create_price_config").to_vec();
    data.extend_from_slice(&config_id.to_le_bytes());
    data.extend_from_slice(&(feed_name.len() as u32).to_le_bytes());
    data.extend_from_slice(feed_name.as_bytes());
    data.extend_from_slice(oracle_address.as_ref());
    data.extend_from_slice(&max_staleness_slots.to_le_bytes());
    data
}

/// Build record_price instruction data
fn record_price_data(
    oracle_price: i64,
    oracle_confidence: u64,
    oracle_exponent: i32,
    oracle_publish_slot: u64,
) -> Vec<u8> {
    let mut data = instruction_disc("record_price").to_vec();
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
    let mut data = instruction_disc("create_price_threshold").to_vec();
    data.extend_from_slice(&config_id.to_le_bytes());
    data.extend_from_slice(&threshold_id.to_le_bytes());
    data.push(if trigger_above { 1 } else { 0 });
    data.extend_from_slice(&target_price_scaled.to_le_bytes());
    data
}

/// Build check_threshold instruction data
fn check_threshold_data() -> Vec<u8> {
    instruction_disc("check_threshold").to_vec()
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

fn read_price_config_update_count(data: &[u8]) -> u64 {
    let feed_name_len = u32::from_le_bytes(data[48..52].try_into().unwrap()) as usize;
    // offset: 52 + feed_name_len + oracle_address(32) + max_staleness(8) + last_price(8) + last_confidence(8) + last_exponent(4) + last_record_slot(8)
    let offset = 52 + feed_name_len + 32 + 8 + 8 + 8 + 4 + 8;
    u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap())
}

/// Read PriceThreshold fields from account data
fn read_price_threshold_is_triggered(data: &[u8]) -> bool {
    // After discriminator(8) + config(32) + threshold_id(8) + trigger_above(1) + target_price_scaled(8)
    data[57] != 0
}

fn read_price_threshold_triggered_at_slot(data: &[u8]) -> u64 {
    u64::from_le_bytes(data[58..66].try_into().unwrap())
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
fn test_parity_price_config_initialization() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/oracle_consumer.so")
        .expect("Failed to read oracle_consumer.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup authority
    let authority = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let oracle_address = Pubkey::new_unique();
    let config_id = 1u64;
    let feed_name = "SOL/USD";
    let max_staleness_slots = 100u64;

    let (price_config_pda, _) = derive_price_config_pda(config_id, &program_id);

    // Create price config
    let ix = Instruction {
        program_id,
        accounts: vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: create_price_config_data(config_id, feed_name, &oracle_address, max_staleness_slots),
    };

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(
        result.is_ok(),
        "create_price_config should succeed: {:?}",
        result
    );

    // Verify initialization state
    let account = svm.get_account(&price_config_pda).unwrap();
    let data = &account.data;

    assert_eq!(read_price_config_authority(data), authority.pubkey());
    assert_eq!(read_price_config_id(data), config_id);
    assert_eq!(read_price_config_feed_name(data), feed_name);
    assert_eq!(read_price_config_oracle_address(data), oracle_address);
    assert_eq!(read_price_config_max_staleness(data), max_staleness_slots);
    assert_eq!(read_price_config_last_price(data), 0); // Initial price is 0
    assert_eq!(read_price_config_update_count(data), 0); // Initial count is 0
}

#[test]
fn test_parity_staleness_validation() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/oracle_consumer.so")
        .expect("Failed to read oracle_consumer.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    let authority = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let oracle_address = Pubkey::new_unique();
    let config_id = 1u64;
    let max_staleness = 50u64; // Only allow prices 50 slots old

    let (price_config_pda, _) = derive_price_config_pda(config_id, &program_id);

    // Create config with small staleness window
    let ix1 = Instruction {
        program_id,
        accounts: vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: create_price_config_data(config_id, "SOL/USD", &oracle_address, max_staleness),
    };

    execute_tx(&mut svm, ix1, &authority, &[&authority]).unwrap();

    // Warp to slot 100
    svm.warp_to_slot(100);
    svm.expire_blockhash();

    // Fresh price (publish_slot=60, age=40 < max_staleness=50) should succeed
    let ix_fresh = Instruction {
        program_id,
        accounts: vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
        data: record_price_data(15000000000i64, 100000u64, -8i32, 60u64),
    };

    let fresh_result = execute_tx(&mut svm, ix_fresh, &authority, &[&authority]);
    assert!(fresh_result.is_ok(), "Fresh price should be accepted");

    // Stale price (publish_slot=10, age=90 > max_staleness=50) should fail
    svm.expire_blockhash();
    let ix_stale = Instruction {
        program_id,
        accounts: vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
        data: record_price_data(15000000000i64, 100000u64, -8i32, 10u64),
    };

    let stale_result = execute_tx(&mut svm, ix_stale, &authority, &[&authority]);
    assert!(stale_result.is_err(), "Stale price should be rejected");
}

#[test]
fn test_parity_update_counting() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/oracle_consumer.so")
        .expect("Failed to read oracle_consumer.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    let authority = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let oracle_address = Pubkey::new_unique();
    let config_id = 1u64;

    let (price_config_pda, _) = derive_price_config_pda(config_id, &program_id);

    // Create config
    let ix1 = Instruction {
        program_id,
        accounts: vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: create_price_config_data(config_id, "SOL/USD", &oracle_address, 100),
    };

    execute_tx(&mut svm, ix1, &authority, &[&authority]).unwrap();

    // Record 3 prices
    for i in 0..3 {
        svm.warp_to_slot((i + 1) * 10);
        svm.expire_blockhash();

        let ix = Instruction {
            program_id,
            accounts: vec![
                signer_meta(authority.pubkey()),
                writable_meta(price_config_pda),
                readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            ],
            data: record_price_data(
                15000000000i64 + (i as i64 * 1000000000), // Price increases each update
                100000u64,
                -8i32,
                (i * 10 + 5) as u64, // Fresh publish slot
            ),
        };

        execute_tx(&mut svm, ix, &authority, &[&authority]).unwrap();
    }

    // Verify update count
    let account = svm.get_account(&price_config_pda).unwrap();
    let data = &account.data;
    assert_eq!(read_price_config_update_count(data), 3);

    // Verify last price is the most recent
    assert_eq!(read_price_config_last_price(data), 17000000000i64);
}

#[test]
fn test_parity_threshold_triggering() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/oracle_consumer.so")
        .expect("Failed to read oracle_consumer.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    let authority = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let oracle_address = Pubkey::new_unique();
    let config_id = 1u64;
    let threshold_id = 1u64;

    let (price_config_pda, _) = derive_price_config_pda(config_id, &program_id);
    let (price_threshold_pda, _) = derive_price_threshold_pda(config_id, threshold_id, &program_id);

    // Create config
    let ix1 = Instruction {
        program_id,
        accounts: vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: create_price_config_data(config_id, "SOL/USD", &oracle_address, 100),
    };

    execute_tx(&mut svm, ix1, &authority, &[&authority]).unwrap();

    // Create threshold: trigger when price >= $150
    let target_price = 15000000000u64;
    let ix2 = Instruction {
        program_id,
        accounts: vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            writable_meta(price_threshold_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
        data: create_price_threshold_data(config_id, threshold_id, true, target_price),
    };

    svm.expire_blockhash();
    execute_tx(&mut svm, ix2, &authority, &[&authority]).unwrap();

    // Record price at $140 (below threshold)
    svm.warp_to_slot(10);
    svm.expire_blockhash();

    let ix3 = Instruction {
        program_id,
        accounts: vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
        data: record_price_data(14000000000i64, 100000u64, -8i32, 5u64),
    };

    execute_tx(&mut svm, ix3, &authority, &[&authority]).unwrap();

    // Check threshold - should not trigger
    svm.expire_blockhash();
    let ix4 = Instruction {
        program_id,
        accounts: vec![
            writable_meta(price_config_pda),
            writable_meta(price_threshold_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
        data: check_threshold_data(),
    };

    execute_tx(&mut svm, ix4, &authority, &[&authority]).unwrap();

    // Verify not triggered
    let account1 = svm.get_account(&price_threshold_pda).unwrap();
    assert_eq!(read_price_threshold_is_triggered(&account1.data), false);

    // Record price at $160 (above threshold)
    svm.warp_to_slot(20);
    svm.expire_blockhash();

    let ix5 = Instruction {
        program_id,
        accounts: vec![
            signer_meta(authority.pubkey()),
            writable_meta(price_config_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
        data: record_price_data(16000000000i64, 100000u64, -8i32, 15u64),
    };

    execute_tx(&mut svm, ix5, &authority, &[&authority]).unwrap();

    // Check threshold - should trigger
    svm.expire_blockhash();
    let ix6 = Instruction {
        program_id,
        accounts: vec![
            writable_meta(price_config_pda),
            writable_meta(price_threshold_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
        data: check_threshold_data(),
    };

    execute_tx(&mut svm, ix6, &authority, &[&authority]).unwrap();

    // Verify triggered
    let account2 = svm.get_account(&price_threshold_pda).unwrap();
    assert_eq!(read_price_threshold_is_triggered(&account2.data), true);
    assert!(read_price_threshold_triggered_at_slot(&account2.data) > 0);
}

#[test]
fn test_parity_deterministic_pda_derivation() {
    let program_id = program_id();
    let config_id = 42u64;
    let threshold_id = 7u64;

    // Config PDA
    let (pda1, bump1) = derive_price_config_pda(config_id, &program_id);
    let (pda2, bump2) = derive_price_config_pda(config_id, &program_id);
    assert_eq!(pda1, pda2, "Config PDA should be deterministic");
    assert_eq!(bump1, bump2, "Config bump should be deterministic");

    // Threshold PDA
    let (t_pda1, t_bump1) = derive_price_threshold_pda(config_id, threshold_id, &program_id);
    let (t_pda2, t_bump2) = derive_price_threshold_pda(config_id, threshold_id, &program_id);
    assert_eq!(t_pda1, t_pda2, "Threshold PDA should be deterministic");
    assert_eq!(t_bump1, t_bump2, "Threshold bump should be deterministic");

    // Different IDs produce different PDAs
    let (pda3, _) = derive_price_config_pda(config_id + 1, &program_id);
    assert_ne!(
        pda1, pda3,
        "Different config_id should produce different PDA"
    );

    let (t_pda3, _) = derive_price_threshold_pda(config_id, threshold_id + 1, &program_id);
    assert_ne!(
        t_pda1, t_pda3,
        "Different threshold_id should produce different PDA"
    );
}
