//! LiteSVM integration tests for the Seahorse Hello World program
//!
//! These tests verify that the hello_world program executes without error
//! and logs the expected message.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile hello_world.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use std::str::FromStr;

/// Hello World program ID (from declare_id!)
fn hello_world_program_id() -> Pubkey {
    Pubkey::from_str("He11oWor1d111111111111111111111111111111111").unwrap()
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the hello_world program into LiteSVM
fn load_hello_world_program() -> (litesvm::LiteSVM, Keypair) {
    let program_id = hello_world_program_id();
    let program_bytes = std::fs::read("../../target/deploy/hello_world.so")
        .expect("Failed to read hello_world.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);

    // Create a funded signer
    let signer = funded_keypair_10_sol(&mut svm);

    (svm, signer)
}

// =============================================================================
// SAY_HELLO TESTS
// =============================================================================

#[test]
fn test_hello_world_say_hello() {
    let (mut svm, signer) = load_hello_world_program();
    let program_id = hello_world_program_id();

    // Create say_hello instruction
    let ix = anchor_instruction(
        program_id,
        "say_hello",
        &[],
        vec![signer_meta(signer.pubkey())],
    );

    // Execute transaction
    let result = execute_tx(&mut svm, ix, &signer, &[&signer]);
    assert!(result.is_ok(), "say_hello should succeed: {:?}", result);

    // Verify logs contain the expected message
    let metadata = result.unwrap();
    let logs = metadata.logs.join("\n");
    assert!(
        logs.contains("Hello world, from Solana smart contract"),
        "Logs should contain greeting message. Actual logs:\n{}",
        logs
    );
}

#[test]
fn test_hello_world_multiple_calls() {
    let (mut svm, signer) = load_hello_world_program();
    let program_id = hello_world_program_id();

    // Call say_hello multiple times
    for i in 0..3 {
        // Expire blockhash between transactions to avoid AlreadyProcessed error
        if i > 0 {
            svm.expire_blockhash();
        }

        let ix = anchor_instruction(
            program_id,
            "say_hello",
            &[],
            vec![signer_meta(signer.pubkey())],
        );

        let result = execute_tx(&mut svm, ix, &signer, &[&signer]);
        assert!(result.is_ok(), "Call {} should succeed: {:?}", i + 1, result);
    }
}

#[test]
fn test_hello_world_different_signers() {
    let program_id = hello_world_program_id();
    let program_bytes = std::fs::read("../../target/deploy/hello_world.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);

    // Create multiple signers
    let signer1 = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let signer2 = funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    // Both should be able to call say_hello
    let ix1 = anchor_instruction(
        program_id,
        "say_hello",
        &[],
        vec![signer_meta(signer1.pubkey())],
    );
    let result1 = execute_tx(&mut svm, ix1, &signer1, &[&signer1]);
    assert!(result1.is_ok(), "Signer1 call should succeed");

    svm.expire_blockhash();

    let ix2 = anchor_instruction(
        program_id,
        "say_hello",
        &[],
        vec![signer_meta(signer2.pubkey())],
    );
    let result2 = execute_tx(&mut svm, ix2, &signer2, &[&signer2]);
    assert!(result2.is_ok(), "Signer2 call should succeed");
}

// =============================================================================
// COMPUTE UNIT TESTS
// =============================================================================

#[test]
fn test_hello_world_compute_units() {
    let (mut svm, signer) = load_hello_world_program();
    let program_id = hello_world_program_id();

    let ix = anchor_instruction(
        program_id,
        "say_hello",
        &[],
        vec![signer_meta(signer.pubkey())],
    );

    let result = execute_tx(&mut svm, ix, &signer, &[&signer]);
    assert!(result.is_ok(), "Transaction should succeed");

    let metadata = result.unwrap();
    // Hello world should use minimal compute units
    assert!(
        metadata.compute_units_consumed < 10_000,
        "Hello world should use fewer than 10,000 CUs, used: {}",
        metadata.compute_units_consumed
    );
}
