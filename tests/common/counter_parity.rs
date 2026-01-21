//! Behavior parity tests: Seahorse Counter vs Anchor Counter
//!
//! This test verifies that the Seahorse-compiled counter program produces
//! identical behavior to the reference Anchor implementation.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile both:
//! - target/deploy/counter.so (Seahorse)
//! - target/deploy/counter_anchor.so (Anchor reference)

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

/// Both programs use the same program ID
fn program_id() -> Pubkey {
    Pubkey::from_str("CntrQd1yLLfEMvj47u3qHxq5xW3jcfC2E51h4jRmpump").unwrap()
}

/// Read counter value from account data
fn read_counter_value(data: &[u8]) -> u64 {
    let count_bytes: [u8; 8] = data[8..16].try_into().unwrap();
    u64::from_le_bytes(count_bytes)
}

/// Run a complete workflow on a program and return the final counter value
fn run_counter_workflow(
    svm: &mut litesvm::LiteSVM,
    program_id: &Pubkey,
    authority: &Keypair,
    program_type: ProgramType,
) -> Result<u64, String> {
    let (counter_pda, _bump) = find_pda(&[b"counter", authority.pubkey().as_ref()], program_id);

    // 1. Initialize - Seahorse requires rent sysvar, Anchor doesn't
    let init_ix = match program_type {
        ProgramType::Seahorse => InstructionBuilder::new("initialize")
            .with_signer(&authority.pubkey())
            .with_writable(&counter_pda)
            .with_readonly(&rent_sysvar::id())
            .with_readonly(&system_program::id())
            .build(*program_id),
        ProgramType::Anchor => InstructionBuilder::new("initialize")
            .with_signer(&authority.pubkey())
            .with_writable(&counter_pda)
            .with_readonly(&system_program::id())
            .build(*program_id),
    };

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[init_ix], Some(&authority.pubkey()));
    let tx = solana_transaction::Transaction::new(&[authority], message, blockhash);
    let result = svm.send_transaction(tx);
    if result.is_err() {
        return Err(format!("Initialize failed: {:?}", result));
    }

    // 2. Increment twice
    for i in 0..2 {
        svm.expire_blockhash();

        let inc_ix = InstructionBuilder::new("increment")
            .with_signer(&authority.pubkey())
            .with_writable(&counter_pda)
            .build(*program_id);

        let blockhash = svm.latest_blockhash();
        let message = solana_message::Message::new(&[inc_ix], Some(&authority.pubkey()));
        let tx = solana_transaction::Transaction::new(&[authority], message, blockhash);
        let result = svm.send_transaction(tx);
        if result.is_err() {
            return Err(format!("Increment {} failed: {:?}", i + 1, result));
        }
    }

    // 3. Decrement once
    svm.expire_blockhash();
    let dec_ix = InstructionBuilder::new("decrement")
        .with_signer(&authority.pubkey())
        .with_writable(&counter_pda)
        .build(*program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[dec_ix], Some(&authority.pubkey()));
    let tx = solana_transaction::Transaction::new(&[authority], message, blockhash);
    let result = svm.send_transaction(tx);
    if result.is_err() {
        return Err(format!("Decrement failed: {:?}", result));
    }

    // 4. Set value to 42
    svm.expire_blockhash();
    let set_ix = InstructionBuilder::new("set_value")
        .with_signer(&authority.pubkey())
        .with_writable(&counter_pda)
        .with_data(42u64.to_le_bytes().to_vec())
        .build(*program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[set_ix], Some(&authority.pubkey()));
    let tx = solana_transaction::Transaction::new(&[authority], message, blockhash);
    let result = svm.send_transaction(tx);
    if result.is_err() {
        return Err(format!("Set value failed: {:?}", result));
    }

    // Read final value
    let account = svm.get_account(&counter_pda).ok_or("Account not found")?;
    Ok(read_counter_value(&account.data))
}

#[test]
fn test_counter_behavior_parity() {
    let program_id = program_id();

    // Load Seahorse version
    let seahorse_bytes = std::fs::read("../../target/deploy/counter.so")
        .expect("Failed to read counter.so - run ./scripts/build-test-programs.sh first");

    // Load Anchor reference version
    let anchor_bytes = std::fs::read("../../target/deploy/counter_anchor.so")
        .expect("Failed to read counter_anchor.so - run ./scripts/build-test-programs.sh first");

    // Create SVMs for each program
    let mut seahorse_svm = litesvm::LiteSVM::new();
    seahorse_svm.add_program(program_id, &seahorse_bytes);
    let seahorse_authority = {
        let kp = Keypair::new();
        seahorse_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let mut anchor_svm = litesvm::LiteSVM::new();
    anchor_svm.add_program(program_id, &anchor_bytes);
    let anchor_authority = {
        let kp = Keypair::new();
        anchor_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    // Run workflows
    let seahorse_result = run_counter_workflow(&mut seahorse_svm, &program_id, &seahorse_authority, ProgramType::Seahorse);
    let anchor_result = run_counter_workflow(&mut anchor_svm, &program_id, &anchor_authority, ProgramType::Anchor);

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

    // Both should be 42 (from set_value)
    assert_eq!(seahorse_value, 42, "Expected final value of 42");
}

#[test]
fn test_counter_underflow_parity() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/counter.so").unwrap();
    let anchor_bytes = std::fs::read("../../target/deploy/counter_anchor.so").unwrap();

    // Test Seahorse underflow behavior
    let mut seahorse_svm = litesvm::LiteSVM::new();
    seahorse_svm.add_program(program_id, &seahorse_bytes);
    let seahorse_authority = {
        let kp = Keypair::new();
        seahorse_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let (counter_pda, _) = find_pda(&[b"counter", seahorse_authority.pubkey().as_ref()], &program_id);

    // Initialize (Seahorse needs rent sysvar)
    let init_ix = InstructionBuilder::new("initialize")
        .with_signer(&seahorse_authority.pubkey())
        .with_writable(&counter_pda)
        .with_readonly(&rent_sysvar::id())
        .with_readonly(&system_program::id())
        .build(program_id);

    let blockhash = seahorse_svm.latest_blockhash();
    let message = solana_message::Message::new(&[init_ix], Some(&seahorse_authority.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seahorse_authority], message, blockhash);
    seahorse_svm.send_transaction(tx).unwrap();

    // Try to decrement from 0 - should fail
    seahorse_svm.expire_blockhash();
    let dec_ix = InstructionBuilder::new("decrement")
        .with_signer(&seahorse_authority.pubkey())
        .with_writable(&counter_pda)
        .build(program_id);

    let blockhash = seahorse_svm.latest_blockhash();
    let message = solana_message::Message::new(&[dec_ix], Some(&seahorse_authority.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seahorse_authority], message, blockhash);
    let seahorse_underflow = seahorse_svm.send_transaction(tx);

    // Test Anchor underflow behavior
    let mut anchor_svm = litesvm::LiteSVM::new();
    anchor_svm.add_program(program_id, &anchor_bytes);
    let anchor_authority = {
        let kp = Keypair::new();
        anchor_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let (counter_pda, _) = find_pda(&[b"counter", anchor_authority.pubkey().as_ref()], &program_id);

    // Initialize (Anchor doesn't need rent sysvar)
    let init_ix = InstructionBuilder::new("initialize")
        .with_signer(&anchor_authority.pubkey())
        .with_writable(&counter_pda)
        .with_readonly(&system_program::id())
        .build(program_id);

    let blockhash = anchor_svm.latest_blockhash();
    let message = solana_message::Message::new(&[init_ix], Some(&anchor_authority.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&anchor_authority], message, blockhash);
    anchor_svm.send_transaction(tx).unwrap();

    // Try to decrement from 0 - should fail
    anchor_svm.expire_blockhash();
    let dec_ix = InstructionBuilder::new("decrement")
        .with_signer(&anchor_authority.pubkey())
        .with_writable(&counter_pda)
        .build(program_id);

    let blockhash = anchor_svm.latest_blockhash();
    let message = solana_message::Message::new(&[dec_ix], Some(&anchor_authority.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&anchor_authority], message, blockhash);
    let anchor_underflow = anchor_svm.send_transaction(tx);

    // Both should fail
    assert!(
        seahorse_underflow.is_err(),
        "Seahorse should fail on underflow"
    );
    assert!(
        anchor_underflow.is_err(),
        "Anchor should fail on underflow"
    );
}
