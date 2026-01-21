//! Behavior parity tests: Seahorse Init Patterns vs Anchor Init Patterns
//!
//! This test verifies that the Seahorse-compiled init_patterns program produces
//! comparable behavior to the reference Anchor implementation.
//!
//! Key focus areas:
//! - Space calculation for various field types
//! - Account initialization produces valid account state
//! - Update operations work correctly
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile both:
//! - target/deploy/init_patterns.so (Seahorse)
//! - target/deploy/init_patterns_anchor.so (Anchor reference)

use seahorse_test_common::*;
use solana_keypair::Keypair;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_sdk_ids::sysvar::rent as rent_sysvar;
use solana_signer::Signer;
use std::str::FromStr;

/// Whether to use Seahorse account layout (includes rent sysvar)
#[derive(Clone, Copy)]
enum ProgramType {
    Seahorse,
    Anchor,
}

/// Both programs use the same program ID for testing
fn program_id() -> Pubkey {
    Pubkey::from_str("InitPtrn1111111111111111111111111111111111").unwrap()
}

/// Read SimpleData values from account data
fn read_simple_data(data: &[u8]) -> (Pubkey, u8, u16, u32, u64, i64) {
    // discriminator(8) + owner(32) + u8(1) + u16(2) + u32(4) + u64(8) + i64(8) + bump(1)
    let owner = Pubkey::new_from_array(data[8..40].try_into().unwrap());
    let value_u8 = data[40];
    let value_u16 = u16::from_le_bytes(data[41..43].try_into().unwrap());
    let value_u32 = u32::from_le_bytes(data[43..47].try_into().unwrap());
    let value_u64 = u64::from_le_bytes(data[47..55].try_into().unwrap());
    let value_i64 = i64::from_le_bytes(data[55..63].try_into().unwrap());
    (owner, value_u8, value_u16, value_u32, value_u64, value_i64)
}

/// Read ComplexData values from account data
fn read_complex_data(data: &[u8]) -> (Pubkey, u64, u64, u64, bool) {
    // discriminator(8) + owner(32) + Stats(count:8 + total:8 + average:8) + is_active(1) + bump(1)
    let owner = Pubkey::new_from_array(data[8..40].try_into().unwrap());
    let count = u64::from_le_bytes(data[40..48].try_into().unwrap());
    let total = u64::from_le_bytes(data[48..56].try_into().unwrap());
    let average = u64::from_le_bytes(data[56..64].try_into().unwrap());
    let is_active = data[64] != 0;
    (owner, count, total, average, is_active)
}

/// Run init_simple on a program and return the account data
fn run_init_simple(
    svm: &mut litesvm::LiteSVM,
    program_id: &Pubkey,
    owner: &Keypair,
    program_type: ProgramType,
    value_u8: u8,
    value_u64: u64,
) -> Result<Vec<u8>, String> {
    let (data_pda, _bump) = find_pda(&[b"simple", owner.pubkey().as_ref()], program_id);

    // Create init instruction data
    let mut ix_data = Vec::new();
    ix_data.push(value_u8);
    ix_data.extend_from_slice(&value_u64.to_le_bytes());

    // Seahorse requires rent sysvar, Anchor doesn't
    let init_ix = match program_type {
        ProgramType::Seahorse => InstructionBuilder::new("init_simple")
            .with_signer(&owner.pubkey())
            .with_writable(&data_pda)
            .with_readonly(&rent_sysvar::id())
            .with_readonly(&system_program::id())
            .with_data(ix_data)
            .build(*program_id),
        ProgramType::Anchor => InstructionBuilder::new("init_simple")
            .with_signer(&owner.pubkey())
            .with_writable(&data_pda)
            .with_readonly(&system_program::id())
            .with_data(ix_data)
            .build(*program_id),
    };

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[init_ix], Some(&owner.pubkey()));
    let tx = solana_transaction::Transaction::new(&[owner], message, blockhash);
    let result = svm.send_transaction(tx);
    if result.is_err() {
        return Err(format!("Initialize failed: {:?}", result));
    }

    // Read account data
    let account = svm.get_account(&data_pda).ok_or("Account not found")?;
    Ok(account.data.clone())
}

/// Run init_complex on a program and return the account data
fn run_init_complex(
    svm: &mut litesvm::LiteSVM,
    program_id: &Pubkey,
    owner: &Keypair,
    program_type: ProgramType,
    initial_count: u64,
) -> Result<Vec<u8>, String> {
    let (data_pda, _bump) = find_pda(&[b"complex", owner.pubkey().as_ref()], program_id);

    // Seahorse requires rent sysvar, Anchor doesn't
    let init_ix = match program_type {
        ProgramType::Seahorse => InstructionBuilder::new("init_complex")
            .with_signer(&owner.pubkey())
            .with_writable(&data_pda)
            .with_readonly(&rent_sysvar::id())
            .with_readonly(&system_program::id())
            .with_data(initial_count.to_le_bytes().to_vec())
            .build(*program_id),
        ProgramType::Anchor => InstructionBuilder::new("init_complex")
            .with_signer(&owner.pubkey())
            .with_writable(&data_pda)
            .with_readonly(&system_program::id())
            .with_data(initial_count.to_le_bytes().to_vec())
            .build(*program_id),
    };

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[init_ix], Some(&owner.pubkey()));
    let tx = solana_transaction::Transaction::new(&[owner], message, blockhash);
    let result = svm.send_transaction(tx);
    if result.is_err() {
        return Err(format!("Initialize failed: {:?}", result));
    }

    // Read account data
    let account = svm.get_account(&data_pda).ok_or("Account not found")?;
    Ok(account.data.clone())
}

#[test]
fn test_simple_data_value_parity() {
    let program_id = program_id();

    // Load Seahorse version
    let seahorse_bytes = std::fs::read("../../target/deploy/init_patterns.so")
        .expect("Failed to read init_patterns.so - run ./scripts/build-test-programs.sh first");

    // Load Anchor reference version
    let anchor_bytes = std::fs::read("../../target/deploy/init_patterns_anchor.so")
        .expect("Failed to read init_patterns_anchor.so - run ./scripts/build-test-programs.sh first");

    // Create SVMs for each program
    let mut seahorse_svm = litesvm::LiteSVM::new();
    seahorse_svm.add_program(program_id, &seahorse_bytes);
    let seahorse_owner = {
        let kp = Keypair::new();
        seahorse_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let mut anchor_svm = litesvm::LiteSVM::new();
    anchor_svm.add_program(program_id, &anchor_bytes);
    let anchor_owner = {
        let kp = Keypair::new();
        anchor_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    // Run init_simple with same values
    let value_u8 = 42u8;
    let value_u64 = 1000000u64;

    let seahorse_data = run_init_simple(
        &mut seahorse_svm,
        &program_id,
        &seahorse_owner,
        ProgramType::Seahorse,
        value_u8,
        value_u64,
    );
    let anchor_data = run_init_simple(
        &mut anchor_svm,
        &program_id,
        &anchor_owner,
        ProgramType::Anchor,
        value_u8,
        value_u64,
    );

    // Both should succeed
    assert!(seahorse_data.is_ok(), "Seahorse init failed: {:?}", seahorse_data);
    assert!(anchor_data.is_ok(), "Anchor init failed: {:?}", anchor_data);

    let seahorse_data = seahorse_data.unwrap();
    let anchor_data = anchor_data.unwrap();

    // Parse account data
    let (_, s_u8, s_u16, s_u32, s_u64, s_i64) = read_simple_data(&seahorse_data);
    let (_, a_u8, a_u16, a_u32, a_u64, a_i64) = read_simple_data(&anchor_data);

    // Verify values match (excluding owner which is different between tests)
    assert_eq!(s_u8, a_u8, "value_u8 should match");
    assert_eq!(s_u16, a_u16, "value_u16 should match");
    assert_eq!(s_u32, a_u32, "value_u32 should match");
    assert_eq!(s_u64, a_u64, "value_u64 should match");
    assert_eq!(s_i64, a_i64, "value_i64 should match");

    // Verify expected values
    assert_eq!(s_u8, 42, "value_u8 should be 42");
    assert_eq!(s_u64, 1000000, "value_u64 should be 1000000");
    assert_eq!(s_u16, 0, "value_u16 should be initialized to 0");
    assert_eq!(s_u32, 0, "value_u32 should be initialized to 0");
    assert_eq!(s_i64, 0, "value_i64 should be initialized to 0");
}

#[test]
fn test_simple_data_space_parity() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/init_patterns.so").unwrap();
    let anchor_bytes = std::fs::read("../../target/deploy/init_patterns_anchor.so").unwrap();

    let mut seahorse_svm = litesvm::LiteSVM::new();
    seahorse_svm.add_program(program_id, &seahorse_bytes);
    let seahorse_owner = {
        let kp = Keypair::new();
        seahorse_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let mut anchor_svm = litesvm::LiteSVM::new();
    anchor_svm.add_program(program_id, &anchor_bytes);
    let anchor_owner = {
        let kp = Keypair::new();
        anchor_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let seahorse_data = run_init_simple(
        &mut seahorse_svm,
        &program_id,
        &seahorse_owner,
        ProgramType::Seahorse,
        1,
        100,
    ).unwrap();
    let anchor_data = run_init_simple(
        &mut anchor_svm,
        &program_id,
        &anchor_owner,
        ProgramType::Anchor,
        1,
        100,
    ).unwrap();

    // Account sizes should match
    assert_eq!(
        seahorse_data.len(),
        anchor_data.len(),
        "SimpleData account sizes should match: Seahorse={}, Anchor={}",
        seahorse_data.len(),
        anchor_data.len()
    );

    // Both should be 64 bytes: discriminator(8) + owner(32) + u8(1) + u16(2) + u32(4) + u64(8) + i64(8) + bump(1)
    assert_eq!(seahorse_data.len(), 64, "SimpleData should be 64 bytes");
}

#[test]
fn test_complex_data_parity() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/init_patterns.so").unwrap();
    let anchor_bytes = std::fs::read("../../target/deploy/init_patterns_anchor.so").unwrap();

    let mut seahorse_svm = litesvm::LiteSVM::new();
    seahorse_svm.add_program(program_id, &seahorse_bytes);
    let seahorse_owner = {
        let kp = Keypair::new();
        seahorse_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let mut anchor_svm = litesvm::LiteSVM::new();
    anchor_svm.add_program(program_id, &anchor_bytes);
    let anchor_owner = {
        let kp = Keypair::new();
        anchor_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let initial_count = 42u64;

    let seahorse_data = run_init_complex(
        &mut seahorse_svm,
        &program_id,
        &seahorse_owner,
        ProgramType::Seahorse,
        initial_count,
    );
    let anchor_data = run_init_complex(
        &mut anchor_svm,
        &program_id,
        &anchor_owner,
        ProgramType::Anchor,
        initial_count,
    );

    assert!(seahorse_data.is_ok(), "Seahorse init complex failed: {:?}", seahorse_data);
    assert!(anchor_data.is_ok(), "Anchor init complex failed: {:?}", anchor_data);

    let seahorse_data = seahorse_data.unwrap();
    let anchor_data = anchor_data.unwrap();

    // Parse account data
    let (_, s_count, s_total, s_average, s_active) = read_complex_data(&seahorse_data);
    let (_, a_count, a_total, a_average, a_active) = read_complex_data(&anchor_data);

    // Verify values match
    assert_eq!(s_count, a_count, "count should match");
    assert_eq!(s_total, a_total, "total should match");
    assert_eq!(s_average, a_average, "average should match");
    assert_eq!(s_active, a_active, "is_active should match");

    // Verify expected initial values
    assert_eq!(s_count, 42, "count should be 42");
    assert_eq!(s_total, 0, "total should be 0");
    assert_eq!(s_average, 0, "average should be 0");
    assert!(s_active, "is_active should be true");

    // Account sizes should match
    assert_eq!(
        seahorse_data.len(),
        anchor_data.len(),
        "ComplexData account sizes should match"
    );

    // Both should be 66 bytes: discriminator(8) + owner(32) + Stats(24) + is_active(1) + bump(1)
    assert_eq!(seahorse_data.len(), 66, "ComplexData should be 66 bytes");
}

#[test]
fn test_update_simple_parity() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/init_patterns.so").unwrap();
    let anchor_bytes = std::fs::read("../../target/deploy/init_patterns_anchor.so").unwrap();

    let mut seahorse_svm = litesvm::LiteSVM::new();
    seahorse_svm.add_program(program_id, &seahorse_bytes);
    let seahorse_owner = {
        let kp = Keypair::new();
        seahorse_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let mut anchor_svm = litesvm::LiteSVM::new();
    anchor_svm.add_program(program_id, &anchor_bytes);
    let anchor_owner = {
        let kp = Keypair::new();
        anchor_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    // Initialize both
    run_init_simple(
        &mut seahorse_svm,
        &program_id,
        &seahorse_owner,
        ProgramType::Seahorse,
        1,
        100,
    ).unwrap();
    run_init_simple(
        &mut anchor_svm,
        &program_id,
        &anchor_owner,
        ProgramType::Anchor,
        1,
        100,
    ).unwrap();

    // Update with extreme values
    let value_u8: u8 = 255;
    let value_u16: u16 = 65535;
    let value_u32: u32 = 0xFFFFFFFF;
    let value_u64: u64 = 0xFFFFFFFFFFFFFFFF;
    let value_i64: i64 = -9223372036854775808; // i64::MIN

    let mut update_data = Vec::new();
    update_data.push(value_u8);
    update_data.extend_from_slice(&value_u16.to_le_bytes());
    update_data.extend_from_slice(&value_u32.to_le_bytes());
    update_data.extend_from_slice(&value_u64.to_le_bytes());
    update_data.extend_from_slice(&value_i64.to_le_bytes());

    // Update Seahorse
    let (seahorse_pda, _) = find_pda(&[b"simple", seahorse_owner.pubkey().as_ref()], &program_id);
    seahorse_svm.expire_blockhash();
    let update_ix = InstructionBuilder::new("update_simple")
        .with_signer(&seahorse_owner.pubkey())
        .with_writable(&seahorse_pda)
        .with_data(update_data.clone())
        .build(program_id);

    let blockhash = seahorse_svm.latest_blockhash();
    let message = solana_message::Message::new(&[update_ix], Some(&seahorse_owner.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seahorse_owner], message, blockhash);
    let seahorse_result = seahorse_svm.send_transaction(tx);

    // Update Anchor
    let (anchor_pda, _) = find_pda(&[b"simple", anchor_owner.pubkey().as_ref()], &program_id);
    anchor_svm.expire_blockhash();
    let update_ix = InstructionBuilder::new("update_simple")
        .with_signer(&anchor_owner.pubkey())
        .with_writable(&anchor_pda)
        .with_data(update_data)
        .build(program_id);

    let blockhash = anchor_svm.latest_blockhash();
    let message = solana_message::Message::new(&[update_ix], Some(&anchor_owner.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&anchor_owner], message, blockhash);
    let anchor_result = anchor_svm.send_transaction(tx);

    // Both should succeed
    assert!(seahorse_result.is_ok(), "Seahorse update failed: {:?}", seahorse_result);
    assert!(anchor_result.is_ok(), "Anchor update failed: {:?}", anchor_result);

    // Read updated data
    let seahorse_account = seahorse_svm.get_account(&seahorse_pda).unwrap();
    let anchor_account = anchor_svm.get_account(&anchor_pda).unwrap();

    let (_, s_u8, s_u16, s_u32, s_u64, s_i64) = read_simple_data(&seahorse_account.data);
    let (_, a_u8, a_u16, a_u32, a_u64, a_i64) = read_simple_data(&anchor_account.data);

    // Values should match
    assert_eq!(s_u8, a_u8);
    assert_eq!(s_u16, a_u16);
    assert_eq!(s_u32, a_u32);
    assert_eq!(s_u64, a_u64);
    assert_eq!(s_i64, a_i64);

    // Verify extreme values handled correctly
    assert_eq!(s_u8, 255);
    assert_eq!(s_u16, 65535);
    assert_eq!(s_u32, 0xFFFFFFFF);
    assert_eq!(s_u64, 0xFFFFFFFFFFFFFFFF);
    assert_eq!(s_i64, i64::MIN);
}
