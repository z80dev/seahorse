//! Integration tests for Seahorse-compiled programs using LiteSVM
//!
//! These tests demonstrate realistic transaction flows for Seahorse programs.

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;

/// Test that LiteSVM can be initialized and basic operations work
#[test]
fn test_litesvm_initialization() {
    let svm = create_litesvm();

    // Verify the SVM is functional
    let blockhash = svm.latest_blockhash();
    assert!(!blockhash.to_bytes().iter().all(|&b| b == 0));
}

/// Test airdrop functionality
#[test]
fn test_airdrop() {
    let mut svm = create_litesvm();
    let user = Keypair::new();

    // Airdrop some SOL
    svm.airdrop(&user.pubkey(), 5 * LAMPORTS_PER_SOL).unwrap();

    // Verify the balance
    let account = svm.get_account(&user.pubkey()).unwrap();
    assert_eq!(account.lamports, 5 * LAMPORTS_PER_SOL);
}

/// Test system transfer between accounts
#[test]
fn test_system_transfer() {
    let mut svm = create_litesvm();

    // Create and fund sender
    let sender = funded_keypair_10_sol(&mut svm);
    let recipient = Keypair::new();

    // Create transfer instruction
    let transfer_amount = LAMPORTS_PER_SOL;
    let ix = solana_instruction::Instruction {
        program_id: system_program::id(),
        accounts: vec![
            signer_meta(sender.pubkey()),
            writable_meta(recipient.pubkey()),
        ],
        data: {
            // System program transfer instruction (index 2)
            let mut data = vec![2, 0, 0, 0]; // Transfer instruction
            data.extend_from_slice(&transfer_amount.to_le_bytes());
            data
        },
    };

    // Execute transfer
    let result = execute_tx(&mut svm, ix, &sender, &[&sender]);
    assert!(result.is_ok(), "Transfer should succeed");

    // Verify recipient balance
    let recipient_account = svm.get_account(&recipient.pubkey()).unwrap();
    assert_eq!(recipient_account.lamports, transfer_amount);
}

/// Test PDA derivation consistency
#[test]
fn test_pda_derivation() {
    let program_id = Pubkey::new_unique();
    let user = Pubkey::new_unique();

    // Derive PDA
    let (pda1, bump1) = find_pda(&[b"calculator", user.as_ref()], &program_id);

    // Same seeds should give same result
    let (pda2, bump2) = find_pda(&[b"calculator", user.as_ref()], &program_id);
    assert_eq!(pda1, pda2);
    assert_eq!(bump1, bump2);

    // Different seeds should give different result
    let (pda3, _) = find_pda(&[b"counter", user.as_ref()], &program_id);
    assert_ne!(pda1, pda3);
}

/// Test Anchor discriminator calculation
#[test]
fn test_anchor_discriminator() {
    // Account discriminators
    let calc_disc = anchor_discriminator("Calculator");
    let counter_disc = anchor_discriminator("Counter");

    assert_eq!(calc_disc.len(), 8);
    assert_eq!(counter_disc.len(), 8);
    assert_ne!(calc_disc, counter_disc);

    // Instruction discriminators
    let init_disc = instruction_discriminator("initialize");
    let inc_disc = instruction_discriminator("increment");

    assert_eq!(init_disc.len(), 8);
    assert_eq!(inc_disc.len(), 8);
    assert_ne!(init_disc, inc_disc);
}

/// Test slot warping for time-dependent tests
#[test]
fn test_slot_warp() {
    let mut svm = create_litesvm();

    // Get initial slot
    let initial_clock = svm.get_sysvar::<solana_clock::Clock>();
    let initial_slot = initial_clock.slot;

    // Warp forward
    let target_slot = initial_slot + 100;
    warp_to_slot(&mut svm, target_slot);

    // Verify slot advanced
    let new_clock = svm.get_sysvar::<solana_clock::Clock>();
    assert!(new_clock.slot >= target_slot);
}

/// Test helper functions for account creation
#[test]
fn test_account_helpers() {
    let program_id = Pubkey::new_unique();

    // System account
    let sys_acc = system_account_with_lamports(1000);
    assert_eq!(sys_acc.lamports, 1000);
    assert_eq!(sys_acc.owner, system_program::id());

    // Program account
    let prog_acc = program_account(32, &program_id);
    assert_eq!(prog_acc.data.len(), DISCRIMINATOR_SIZE + 32);
    assert_eq!(prog_acc.owner, program_id);

    // Initialized anchor account
    let data = vec![1, 2, 3, 4];
    let anchor_acc = initialized_anchor_account("MyAccount", &data, &program_id);
    assert_eq!(anchor_acc.data.len(), DISCRIMINATOR_SIZE + 4);
    assert_eq!(anchor_acc.owner, program_id);
    // First 8 bytes should be discriminator
    assert_eq!(anchor_acc.data[8..12], data);
}

/// Demonstrate program loading (placeholder until we have compiled programs)
#[test]
fn test_program_loading_structure() {
    // This test demonstrates the structure for loading a compiled program
    // Once we have Seahorse programs compiled to .so files, we can use:
    //
    // let program_keypair = Keypair::new();
    // let svm = litesvm_with_program(&program_keypair, "target/deploy/calculator.so");
    //
    // For now, we just verify the helper exists
    let program_id = Pubkey::new_unique();
    let ix = anchor_instruction(
        program_id,
        "initialize",
        &[],
        vec![signer_meta(Pubkey::new_unique())],
    );

    // Verify instruction structure
    assert_eq!(ix.program_id, program_id);
    assert_eq!(ix.data.len(), 8); // Just discriminator
}
