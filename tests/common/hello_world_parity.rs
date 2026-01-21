//! Behavior parity tests: Seahorse Hello World vs Anchor Hello World
//!
//! This test verifies that the Seahorse-compiled hello_world program produces
//! identical behavior to the reference Anchor implementation.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile both:
//! - target/deploy/hello_world.so (Seahorse)
//! - target/deploy/hello_world_anchor.so (Anchor reference)

use seahorse_test_common::*;
use solana_keypair::Keypair;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use std::str::FromStr;

/// Hello World program ID (from declare_id!)
fn program_id() -> Pubkey {
    Pubkey::from_str("He11oWor1d111111111111111111111111111111111").unwrap()
}

/// Run say_hello instruction and return success/failure + logs
fn run_say_hello(
    svm: &mut litesvm::LiteSVM,
    program_id: &Pubkey,
    signer: &Keypair,
) -> Result<Vec<String>, String> {
    let ix = InstructionBuilder::new("say_hello")
        .with_signer(&signer.pubkey())
        .build(*program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[ix], Some(&signer.pubkey()));
    let tx = solana_transaction::Transaction::new(&[signer], message, blockhash);

    match svm.send_transaction(tx) {
        Ok(metadata) => Ok(metadata.logs),
        Err(e) => Err(format!("Transaction failed: {:?}", e)),
    }
}

#[test]
fn test_hello_world_behavior_parity() {
    let program_id = program_id();

    // Load Seahorse version
    let seahorse_bytes = std::fs::read("../../target/deploy/hello_world.so")
        .expect("Failed to read hello_world.so - run ./scripts/build-test-programs.sh first");

    // Load Anchor reference version
    let anchor_bytes = std::fs::read("../../target/deploy/hello_world_anchor.so")
        .expect("Failed to read hello_world_anchor.so - run ./scripts/build-test-programs.sh first");

    // Create SVMs for each program
    let mut seahorse_svm = litesvm::LiteSVM::new();
    seahorse_svm.add_program(program_id, &seahorse_bytes);
    let seahorse_signer = {
        let kp = Keypair::new();
        seahorse_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let mut anchor_svm = litesvm::LiteSVM::new();
    anchor_svm.add_program(program_id, &anchor_bytes);
    let anchor_signer = {
        let kp = Keypair::new();
        anchor_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    // Run say_hello on both programs
    let seahorse_result = run_say_hello(&mut seahorse_svm, &program_id, &seahorse_signer);
    let anchor_result = run_say_hello(&mut anchor_svm, &program_id, &anchor_signer);

    // Both should succeed
    assert!(
        seahorse_result.is_ok(),
        "Seahorse say_hello failed: {:?}",
        seahorse_result
    );
    assert!(
        anchor_result.is_ok(),
        "Anchor say_hello failed: {:?}",
        anchor_result
    );

    // Both should log the greeting message
    let seahorse_logs = seahorse_result.unwrap().join("\n");
    let anchor_logs = anchor_result.unwrap().join("\n");

    assert!(
        seahorse_logs.contains("Hello world"),
        "Seahorse logs should contain 'Hello world'. Logs:\n{}",
        seahorse_logs
    );
    assert!(
        anchor_logs.contains("Hello world"),
        "Anchor logs should contain 'Hello world'. Logs:\n{}",
        anchor_logs
    );
}

#[test]
fn test_hello_world_multiple_calls_parity() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/hello_world.so").unwrap();
    let anchor_bytes = std::fs::read("../../target/deploy/hello_world_anchor.so").unwrap();

    // Create SVMs
    let mut seahorse_svm = litesvm::LiteSVM::new();
    seahorse_svm.add_program(program_id, &seahorse_bytes);
    let seahorse_signer = {
        let kp = Keypair::new();
        seahorse_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let mut anchor_svm = litesvm::LiteSVM::new();
    anchor_svm.add_program(program_id, &anchor_bytes);
    let anchor_signer = {
        let kp = Keypair::new();
        anchor_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    // Run multiple calls on each
    let mut seahorse_success_count = 0;
    let mut anchor_success_count = 0;

    for i in 0..3 {
        if i > 0 {
            seahorse_svm.expire_blockhash();
            anchor_svm.expire_blockhash();
        }

        if run_say_hello(&mut seahorse_svm, &program_id, &seahorse_signer).is_ok() {
            seahorse_success_count += 1;
        }

        if run_say_hello(&mut anchor_svm, &program_id, &anchor_signer).is_ok() {
            anchor_success_count += 1;
        }
    }

    // Both should have the same success count (all 3 should succeed)
    assert_eq!(
        seahorse_success_count, anchor_success_count,
        "Behavior parity: Seahorse succeeded {} times, Anchor succeeded {} times",
        seahorse_success_count, anchor_success_count
    );
    assert_eq!(
        seahorse_success_count, 3,
        "All calls should succeed, but only {} did",
        seahorse_success_count
    );
}
