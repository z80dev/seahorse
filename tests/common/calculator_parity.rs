//! Behavior parity tests: Seahorse Calculator vs Anchor Calculator
//!
//! This test verifies that the Seahorse-compiled calculator program produces
//! identical behavior to the reference Anchor implementation.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile both:
//! - target/deploy/calculator.so (Seahorse)
//! - target/deploy/calculator_anchor.so (Anchor reference)

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
    Pubkey::from_str("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS").unwrap()
}

/// Operation enum values matching both Seahorse and Anchor
#[repr(u8)]
enum Operation {
    Add = 0,
    Sub = 1,
    Mul = 2,
    Div = 3,
}

/// Create instruction data for do_operation: op (1 byte) + num (8 bytes as i64)
fn operation_data(op: Operation, num: i64) -> Vec<u8> {
    let mut data = Vec::with_capacity(9);
    data.push(op as u8);
    data.extend_from_slice(&num.to_le_bytes());
    data
}

/// Read display value from calculator account data
fn read_display_value(data: &[u8]) -> i64 {
    // Skip discriminator (8) and owner (32), read display (8)
    let display_bytes: [u8; 8] = data[40..48].try_into().unwrap();
    i64::from_le_bytes(display_bytes)
}

/// Run a complete workflow on a program and return the final display value
fn run_calculator_workflow(
    svm: &mut litesvm::LiteSVM,
    program_id: &Pubkey,
    owner: &Keypair,
    program_type: ProgramType,
) -> Result<i64, String> {
    let (calculator_pda, _bump) = find_pda(&[b"Calculator", owner.pubkey().as_ref()], program_id);

    // 1. Initialize - Seahorse requires rent sysvar, Anchor doesn't
    let init_ix = match program_type {
        ProgramType::Seahorse => InstructionBuilder::new("init_calculator")
            .with_signer(&owner.pubkey())
            .with_writable(&calculator_pda)
            .with_readonly(&rent_sysvar::id())
            .with_readonly(&system_program::id())
            .build(*program_id),
        ProgramType::Anchor => InstructionBuilder::new("init_calculator")
            .with_signer(&owner.pubkey())
            .with_writable(&calculator_pda)
            .with_readonly(&system_program::id())
            .build(*program_id),
    };

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[init_ix], Some(&owner.pubkey()));
    let tx = solana_transaction::Transaction::new(&[owner], message, blockhash);
    let result = svm.send_transaction(tx);
    if result.is_err() {
        return Err(format!("Initialize failed: {:?}", result));
    }

    // 2. Add 50
    svm.expire_blockhash();
    let add_ix = InstructionBuilder::new("do_operation")
        .with_signer(&owner.pubkey())
        .with_writable(&calculator_pda)
        .with_data(operation_data(Operation::Add, 50))
        .build(*program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[add_ix], Some(&owner.pubkey()));
    let tx = solana_transaction::Transaction::new(&[owner], message, blockhash);
    let result = svm.send_transaction(tx);
    if result.is_err() {
        return Err(format!("Add failed: {:?}", result));
    }

    // 3. Multiply by 3 (50 * 3 = 150)
    svm.expire_blockhash();
    let mul_ix = InstructionBuilder::new("do_operation")
        .with_signer(&owner.pubkey())
        .with_writable(&calculator_pda)
        .with_data(operation_data(Operation::Mul, 3))
        .build(*program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[mul_ix], Some(&owner.pubkey()));
    let tx = solana_transaction::Transaction::new(&[owner], message, blockhash);
    let result = svm.send_transaction(tx);
    if result.is_err() {
        return Err(format!("Multiply failed: {:?}", result));
    }

    // 4. Subtract 8 (150 - 8 = 142)
    svm.expire_blockhash();
    let sub_ix = InstructionBuilder::new("do_operation")
        .with_signer(&owner.pubkey())
        .with_writable(&calculator_pda)
        .with_data(operation_data(Operation::Sub, 8))
        .build(*program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[sub_ix], Some(&owner.pubkey()));
    let tx = solana_transaction::Transaction::new(&[owner], message, blockhash);
    let result = svm.send_transaction(tx);
    if result.is_err() {
        return Err(format!("Subtract failed: {:?}", result));
    }

    // 5. Divide by 2 (142 / 2 = 71)
    svm.expire_blockhash();
    let div_ix = InstructionBuilder::new("do_operation")
        .with_signer(&owner.pubkey())
        .with_writable(&calculator_pda)
        .with_data(operation_data(Operation::Div, 2))
        .build(*program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[div_ix], Some(&owner.pubkey()));
    let tx = solana_transaction::Transaction::new(&[owner], message, blockhash);
    let result = svm.send_transaction(tx);
    if result.is_err() {
        return Err(format!("Divide failed: {:?}", result));
    }

    // Read final value
    let account = svm.get_account(&calculator_pda).ok_or("Account not found")?;
    Ok(read_display_value(&account.data))
}

#[test]
fn test_calculator_behavior_parity() {
    let program_id = program_id();

    // Load Seahorse version
    let seahorse_bytes = std::fs::read("../../target/deploy/calculator.so")
        .expect("Failed to read calculator.so - run ./scripts/build-test-programs.sh first");

    // Load Anchor reference version
    let anchor_bytes = std::fs::read("../../target/deploy/calculator_anchor.so")
        .expect("Failed to read calculator_anchor.so - run ./scripts/build-test-programs.sh first");

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

    // Run workflows
    let seahorse_result = run_calculator_workflow(&mut seahorse_svm, &program_id, &seahorse_owner, ProgramType::Seahorse);
    let anchor_result = run_calculator_workflow(&mut anchor_svm, &program_id, &anchor_owner, ProgramType::Anchor);

    // Both should succeed
    assert!(
        seahorse_result.is_ok(),
        "Seahorse workflow failed: {:?}",
        seahorse_result
    );
    assert!(
        anchor_result.is_ok(),
        "Anchor workflow failed: {:?}",
        anchor_result
    );

    // Both should produce same final value
    let seahorse_value = seahorse_result.unwrap();
    let anchor_value = anchor_result.unwrap();

    assert_eq!(
        seahorse_value, anchor_value,
        "Behavior parity failed: Seahorse={}, Anchor={}",
        seahorse_value, anchor_value
    );

    // Both should be 71 (50 * 3 - 8 / 2 = 150 - 8 / 2 = 142 / 2 = 71)
    assert_eq!(seahorse_value, 71, "Expected final value of 71");
}

#[test]
fn test_calculator_div_by_zero_parity() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/calculator.so").unwrap();
    let anchor_bytes = std::fs::read("../../target/deploy/calculator_anchor.so").unwrap();

    // Test Seahorse division by zero behavior
    let mut seahorse_svm = litesvm::LiteSVM::new();
    seahorse_svm.add_program(program_id, &seahorse_bytes);
    let seahorse_owner = {
        let kp = Keypair::new();
        seahorse_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let (calculator_pda, _) = find_pda(&[b"Calculator", seahorse_owner.pubkey().as_ref()], &program_id);

    // Initialize (Seahorse needs rent sysvar)
    let init_ix = InstructionBuilder::new("init_calculator")
        .with_signer(&seahorse_owner.pubkey())
        .with_writable(&calculator_pda)
        .with_readonly(&rent_sysvar::id())
        .with_readonly(&system_program::id())
        .build(program_id);

    let blockhash = seahorse_svm.latest_blockhash();
    let message = solana_message::Message::new(&[init_ix], Some(&seahorse_owner.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seahorse_owner], message, blockhash);
    seahorse_svm.send_transaction(tx).unwrap();

    // Add some value
    seahorse_svm.expire_blockhash();
    let add_ix = InstructionBuilder::new("do_operation")
        .with_signer(&seahorse_owner.pubkey())
        .with_writable(&calculator_pda)
        .with_data(operation_data(Operation::Add, 42))
        .build(program_id);

    let blockhash = seahorse_svm.latest_blockhash();
    let message = solana_message::Message::new(&[add_ix], Some(&seahorse_owner.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seahorse_owner], message, blockhash);
    seahorse_svm.send_transaction(tx).unwrap();

    // Try to divide by 0 - should fail
    seahorse_svm.expire_blockhash();
    let div_ix = InstructionBuilder::new("do_operation")
        .with_signer(&seahorse_owner.pubkey())
        .with_writable(&calculator_pda)
        .with_data(operation_data(Operation::Div, 0))
        .build(program_id);

    let blockhash = seahorse_svm.latest_blockhash();
    let message = solana_message::Message::new(&[div_ix], Some(&seahorse_owner.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seahorse_owner], message, blockhash);
    let seahorse_div_by_zero = seahorse_svm.send_transaction(tx);

    // Test Anchor division by zero behavior
    let mut anchor_svm = litesvm::LiteSVM::new();
    anchor_svm.add_program(program_id, &anchor_bytes);
    let anchor_owner = {
        let kp = Keypair::new();
        anchor_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let (calculator_pda, _) = find_pda(&[b"Calculator", anchor_owner.pubkey().as_ref()], &program_id);

    // Initialize (Anchor doesn't need rent sysvar)
    let init_ix = InstructionBuilder::new("init_calculator")
        .with_signer(&anchor_owner.pubkey())
        .with_writable(&calculator_pda)
        .with_readonly(&system_program::id())
        .build(program_id);

    let blockhash = anchor_svm.latest_blockhash();
    let message = solana_message::Message::new(&[init_ix], Some(&anchor_owner.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&anchor_owner], message, blockhash);
    anchor_svm.send_transaction(tx).unwrap();

    // Add some value
    anchor_svm.expire_blockhash();
    let add_ix = InstructionBuilder::new("do_operation")
        .with_signer(&anchor_owner.pubkey())
        .with_writable(&calculator_pda)
        .with_data(operation_data(Operation::Add, 42))
        .build(program_id);

    let blockhash = anchor_svm.latest_blockhash();
    let message = solana_message::Message::new(&[add_ix], Some(&anchor_owner.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&anchor_owner], message, blockhash);
    anchor_svm.send_transaction(tx).unwrap();

    // Try to divide by 0 - should fail
    anchor_svm.expire_blockhash();
    let div_ix = InstructionBuilder::new("do_operation")
        .with_signer(&anchor_owner.pubkey())
        .with_writable(&calculator_pda)
        .with_data(operation_data(Operation::Div, 0))
        .build(program_id);

    let blockhash = anchor_svm.latest_blockhash();
    let message = solana_message::Message::new(&[div_ix], Some(&anchor_owner.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&anchor_owner], message, blockhash);
    let anchor_div_by_zero = anchor_svm.send_transaction(tx);

    // Both should fail
    assert!(
        seahorse_div_by_zero.is_err(),
        "Seahorse should fail on division by zero"
    );
    assert!(
        anchor_div_by_zero.is_err(),
        "Anchor should fail on division by zero"
    );
}

#[test]
fn test_calculator_unauthorized_parity() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/calculator.so").unwrap();
    let anchor_bytes = std::fs::read("../../target/deploy/calculator_anchor.so").unwrap();

    // Test Seahorse unauthorized access
    let mut seahorse_svm = litesvm::LiteSVM::new();
    seahorse_svm.add_program(program_id, &seahorse_bytes);
    let seahorse_owner = {
        let kp = Keypair::new();
        seahorse_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let seahorse_attacker = {
        let kp = Keypair::new();
        seahorse_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let (calculator_pda, _) = find_pda(&[b"Calculator", seahorse_owner.pubkey().as_ref()], &program_id);

    // Initialize (owner)
    let init_ix = InstructionBuilder::new("init_calculator")
        .with_signer(&seahorse_owner.pubkey())
        .with_writable(&calculator_pda)
        .with_readonly(&rent_sysvar::id())
        .with_readonly(&system_program::id())
        .build(program_id);

    let blockhash = seahorse_svm.latest_blockhash();
    let message = solana_message::Message::new(&[init_ix], Some(&seahorse_owner.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seahorse_owner], message, blockhash);
    seahorse_svm.send_transaction(tx).unwrap();

    // Attacker tries to operate on owner's calculator
    seahorse_svm.expire_blockhash();
    let add_ix = InstructionBuilder::new("do_operation")
        .with_signer(&seahorse_attacker.pubkey())
        .with_writable(&calculator_pda)
        .with_data(operation_data(Operation::Add, 100))
        .build(program_id);

    let blockhash = seahorse_svm.latest_blockhash();
    let message = solana_message::Message::new(&[add_ix], Some(&seahorse_attacker.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seahorse_attacker], message, blockhash);
    let seahorse_unauthorized = seahorse_svm.send_transaction(tx);

    // Test Anchor unauthorized access
    let mut anchor_svm = litesvm::LiteSVM::new();
    anchor_svm.add_program(program_id, &anchor_bytes);
    let anchor_owner = {
        let kp = Keypair::new();
        anchor_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let anchor_attacker = {
        let kp = Keypair::new();
        anchor_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let (calculator_pda, _) = find_pda(&[b"Calculator", anchor_owner.pubkey().as_ref()], &program_id);

    // Initialize (owner)
    let init_ix = InstructionBuilder::new("init_calculator")
        .with_signer(&anchor_owner.pubkey())
        .with_writable(&calculator_pda)
        .with_readonly(&system_program::id())
        .build(program_id);

    let blockhash = anchor_svm.latest_blockhash();
    let message = solana_message::Message::new(&[init_ix], Some(&anchor_owner.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&anchor_owner], message, blockhash);
    anchor_svm.send_transaction(tx).unwrap();

    // Attacker tries to operate on owner's calculator
    anchor_svm.expire_blockhash();
    let add_ix = InstructionBuilder::new("do_operation")
        .with_signer(&anchor_attacker.pubkey())
        .with_writable(&calculator_pda)
        .with_data(operation_data(Operation::Add, 100))
        .build(program_id);

    let blockhash = anchor_svm.latest_blockhash();
    let message = solana_message::Message::new(&[add_ix], Some(&anchor_attacker.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&anchor_attacker], message, blockhash);
    let anchor_unauthorized = anchor_svm.send_transaction(tx);

    // Both should fail
    assert!(
        seahorse_unauthorized.is_err(),
        "Seahorse should fail on unauthorized access"
    );
    assert!(
        anchor_unauthorized.is_err(),
        "Anchor should fail on unauthorized access"
    );
}
