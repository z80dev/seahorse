//! Example Mollusk unit tests for Seahorse programs
//!
//! These tests demonstrate how to use Mollusk to test Seahorse-compiled programs.
//!
//! Note: Before running these tests, you must:
//! 1. Compile Seahorse programs with `seahorse compile`
//! 2. Build program binaries with `anchor build` (or `cargo build-sbf`)
//!
//! The tests use Mollusk to execute instructions in isolation without
//! requiring a full Solana validator.

use mollusk_svm::{result::Check, Mollusk};
use seahorse_unit_tests::helpers::*;
use seahorse_unit_tests::*;
use solana_sdk::{
    account::Account,
    account::ReadableAccount,
    pubkey::Pubkey,
};

/// Create a system account
fn system_account(lamports: u64) -> Account {
    Account {
        lamports,
        data: vec![],
        owner: solana_sdk::system_program::id(),
        executable: false,
        rent_epoch: 0,
    }
}

/// Example test using Mollusk with the System Program
///
/// This demonstrates the basic Mollusk workflow without requiring
/// a compiled Seahorse program.
#[test]
fn test_mollusk_system_transfer() {
    // Create Mollusk instance for system program tests
    let mollusk = Mollusk::default();

    // Set up accounts
    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    let sender_starting_lamports = 1_000_000_000; // 1 SOL
    let transfer_amount = 100_000_000; // 0.1 SOL

    // Create accounts with initial balances
    let accounts: Vec<(Pubkey, Account)> = vec![
        (sender, system_account(sender_starting_lamports)),
        (recipient, system_account(0)),
    ];

    // Create transfer instruction
    let instruction = solana_sdk::system_instruction::transfer(
        &sender,
        &recipient,
        transfer_amount,
    );

    // Process the instruction
    let result = mollusk.process_instruction(&instruction, &accounts);

    // Verify success via raw_result
    assert!(
        result.raw_result.is_ok(),
        "Transfer should succeed: {:?}",
        result.raw_result
    );

    // Verify account balances changed correctly
    let sender_account = result
        .get_account(&sender)
        .expect("Sender account should exist");
    let recipient_account = result
        .get_account(&recipient)
        .expect("Recipient account should exist");

    assert_eq!(
        sender_account.lamports(),
        sender_starting_lamports - transfer_amount,
        "Sender should have sent lamports"
    );
    assert_eq!(
        recipient_account.lamports(),
        transfer_amount,
        "Recipient should have received lamports"
    );
}

/// Example test using process_and_validate_instruction
///
/// This demonstrates using Mollusk's validation checks for cleaner test assertions.
#[test]
fn test_mollusk_with_checks() {
    let mollusk = Mollusk::default();

    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    let starting_lamports = 500_000_000;
    let transfer_amount = 42_000;

    let accounts: Vec<(Pubkey, Account)> = vec![
        (sender, system_account(starting_lamports)),
        (recipient, system_account(starting_lamports)),
    ];

    let instruction = solana_sdk::system_instruction::transfer(
        &sender,
        &recipient,
        transfer_amount,
    );

    // Define checks to validate the result
    let checks = vec![
        Check::success(),
        Check::account(&sender)
            .lamports(starting_lamports - transfer_amount)
            .build(),
        Check::account(&recipient)
            .lamports(starting_lamports + transfer_amount)
            .build(),
    ];

    // Process and validate - panics if any check fails
    mollusk.process_and_validate_instruction(&instruction, &accounts, &checks);
}

/// Example test demonstrating instruction chain
///
/// Multiple instructions can be chained together, with state persisting
/// between them.
#[test]
fn test_mollusk_instruction_chain() {
    let mollusk = Mollusk::default();

    let alice = Pubkey::new_unique();
    let bob = Pubkey::new_unique();
    let carol = Pubkey::new_unique();

    let starting_lamports = 1_000_000_000;

    let accounts: Vec<(Pubkey, Account)> = vec![
        (alice, system_account(starting_lamports)),
        (bob, system_account(starting_lamports)),
        (carol, system_account(starting_lamports)),
    ];

    // Chain: Alice -> Bob -> Carol
    let instructions = vec![
        solana_sdk::system_instruction::transfer(&alice, &bob, 100_000_000),
        solana_sdk::system_instruction::transfer(&bob, &carol, 50_000_000),
    ];

    let result = mollusk.process_instruction_chain(&instructions, &accounts);

    // Verify final state
    assert!(result.raw_result.is_ok());

    let alice_account = result.get_account(&alice).unwrap();
    let bob_account = result.get_account(&bob).unwrap();
    let carol_account = result.get_account(&carol).unwrap();

    // Alice: 1B - 100M = 900M
    assert_eq!(alice_account.lamports(), 900_000_000);
    // Bob: 1B + 100M - 50M = 1.05B
    assert_eq!(bob_account.lamports(), 1_050_000_000);
    // Carol: 1B + 50M = 1.05B
    assert_eq!(carol_account.lamports(), 1_050_000_000);
}

/// Test helper function utilities
#[test]
fn test_anchor_discriminator_format() {
    // Verify discriminator calculation produces consistent 8-byte values
    let disc1 = anchor_discriminator("Counter");
    let disc2 = anchor_discriminator("Counter");

    assert_eq!(disc1.len(), 8);
    assert_eq!(disc1, disc2);

    // Different names produce different discriminators
    let disc3 = anchor_discriminator("Calculator");
    assert_ne!(disc1, disc3);
}

/// Test instruction discriminator helper
#[test]
fn test_instruction_discriminator_format() {
    let disc = instruction_discriminator("initialize");

    assert_eq!(disc.len(), 8);

    // Should be deterministic
    assert_eq!(disc, instruction_discriminator("initialize"));

    // Different instructions have different discriminators
    assert_ne!(disc, instruction_discriminator("increment"));
}

/// Test PDA derivation helper
#[test]
fn test_pda_helper() {
    let program_id = Pubkey::new_unique();
    let user = Pubkey::new_unique();

    let (pda1, bump1) = find_pda(&[b"counter", user.as_ref()], &program_id);
    let (pda2, bump2) = find_pda(&[b"counter", user.as_ref()], &program_id);

    // Same seeds produce same PDA
    assert_eq!(pda1, pda2);
    assert_eq!(bump1, bump2);

    // Different seeds produce different PDA
    let (pda3, _) = find_pda(&[b"other", user.as_ref()], &program_id);
    assert_ne!(pda1, pda3);
}

/// Test initialized Anchor account creation
#[test]
fn test_initialized_anchor_account() {
    let program_id = Pubkey::new_unique();

    // Create a simple counter account (just a u64)
    let count: u64 = 42;
    let data = count.to_le_bytes();

    let account = initialized_anchor_account("Counter", &data, &program_id);

    // Account should have discriminator + data
    assert_eq!(account.data.len(), 8 + 8);
    assert_eq!(account.owner, program_id);

    // Verify discriminator is set
    let disc = anchor_discriminator("Counter");
    assert_eq!(&account.data[..8], &disc);

    // Verify data is set
    assert_eq!(&account.data[8..], &data);
}

// =============================================================================
// SEAHORSE PROGRAM TESTS
// =============================================================================
// The tests below are templates for testing actual Seahorse programs.
// They require compiled program binaries to run.
//
// To enable these tests:
// 1. Compile the Seahorse examples: `seahorse compile examples/calculator.py`
// 2. Build the program: `anchor build` (or similar)
// 3. Uncomment the tests and update program IDs
// =============================================================================

/*
/// Example: Testing the Calculator program with Mollusk
///
/// This shows how to test a Seahorse-compiled program.
#[test]
fn test_calculator_init() {
    // Program ID (from declare_id! in the compiled program)
    let program_id = Pubkey::from_str("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS").unwrap();

    // Create Mollusk instance for the program
    let mollusk = mollusk_for_program(&program_id, "calculator");

    // Set up accounts
    let owner = Pubkey::new_unique();
    let (calculator_pda, _bump) = find_pda(
        &[b"calculator", owner.as_ref()],
        &program_id,
    );

    let accounts: Vec<(Pubkey, AccountSharedData)> = vec![
        (owner, system_account_shared(1_000_000_000)),
        (calculator_pda, AccountSharedData::new(
            rent_exempt_lamports(CALCULATOR_SIZE),
            CALCULATOR_SIZE,
            &program_id,
        )),
        (solana_sdk::system_program::id(), AccountSharedData::default()),
    ];

    // Create init_calculator instruction
    let instruction = anchor_instruction(
        program_id,
        "init_calculator",
        &[],
        vec![
            signer_meta(owner),
            writable_meta(calculator_pda),
            readonly_meta(solana_sdk::system_program::id()),
        ],
    );

    // Process and validate
    mollusk.process_and_validate_instruction(
        &instruction,
        &accounts,
        &[Check::success()],
    );
}
*/
