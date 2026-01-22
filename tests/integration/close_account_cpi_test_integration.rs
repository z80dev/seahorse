//! LiteSVM integration tests for the Seahorse close_account_cpi_test program
//!
//! These tests verify the SPL Token close_account CPI functionality:
//! - close_token_account: Closes a token account and returns rent to destination
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile close_account_cpi_test.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::path::Path;
use std::str::FromStr;

/// close_account_cpi_test program ID (from declare_id!)
fn program_id() -> Pubkey {
    Pubkey::from_str("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS").unwrap()
}

/// Path to the compiled program
const PROGRAM_PATH: &str = "../../target/deploy/close_account_cpi_test.so";

/// Check if the program is built
fn program_exists() -> bool {
    Path::new(PROGRAM_PATH).exists()
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the close_account_cpi_test program into LiteSVM
fn load_program() -> litesvm::LiteSVM {
    let prog_id = program_id();

    if !program_exists() {
        panic!("Failed to read close_account_cpi_test.so - run ./scripts/build-test-programs.sh first");
    }

    let program_bytes = std::fs::read(PROGRAM_PATH)
        .expect("Failed to read close_account_cpi_test.so");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(prog_id, &program_bytes);
    svm
}

/// Build close_token_account instruction data (no args beyond discriminator)
fn close_token_account_data() -> Vec<u8> {
    Vec::new() // No additional args
}

// =============================================================================
// CLOSE TOKEN ACCOUNT TESTS
// =============================================================================

#[test]
fn test_close_token_account_returns_rent() {
    if !program_exists() {
        eprintln!("Skipping test_close_token_account_returns_rent: program not built. Run ./scripts/build-test-programs.sh");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    // Setup accounts
    let authority = funded_keypair_10_sol(&mut svm);
    let destination = funded_keypair_10_sol(&mut svm);

    // Create a mint
    let mint_keypair = solana_keypair::Keypair::new();
    let mint_account = create_mint_account(&authority.pubkey(), 9);
    svm.set_account(mint_keypair.pubkey(), mint_account).unwrap();

    // Create a token account owned by authority with 0 tokens
    let token_account_keypair = solana_keypair::Keypair::new();
    let token_account = create_token_account(
        &authority.pubkey(),
        &mint_keypair.pubkey(),
        0, // Empty balance - required for close
    );
    let token_account_lamports = token_account.lamports;
    svm.set_account(token_account_keypair.pubkey(), token_account).unwrap();

    // Record destination lamports before close
    let dest_lamports_before = get_account(&svm, &destination.pubkey()).unwrap().lamports;

    // Build close_token_account instruction
    // Account order from close_account_cpi_test.py:
    // - authority: Signer
    // - token_account: TokenAccount
    // - destination: Signer
    // Plus token program for the CPI
    let ix = anchor_instruction(
        prog_id,
        "close_token_account",
        &close_token_account_data(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(token_account_keypair.pubkey()),
            signer_meta(destination.pubkey()),
            readonly_meta(token_program_id()),
        ],
    );

    // Execute transaction
    let result = execute_tx(&mut svm, ix, &authority, &[&authority, &destination]);
    assert!(result.is_ok(), "Failed to close token account: {:?}", result.err());

    // Verify token account is closed (no longer exists or has 0 lamports)
    let closed_account = get_account(&svm, &token_account_keypair.pubkey());
    match closed_account {
        None => {
            // Account was closed and removed - expected behavior
        }
        Some(acc) => {
            // Account exists but should have 0 lamports
            assert_eq!(acc.lamports, 0, "Closed account should have 0 lamports");
        }
    }

    // Verify destination received the rent
    let dest_lamports_after = get_account(&svm, &destination.pubkey()).unwrap().lamports;
    // Destination should have received the token account's rent
    // (minus any tx fees if destination was fee payer, but authority is fee payer here)
    assert!(dest_lamports_after > dest_lamports_before,
        "Destination should have received rent: before={}, after={}",
        dest_lamports_before, dest_lamports_after);

    // Verify the difference is approximately the token account rent
    let rent_received = dest_lamports_after - dest_lamports_before;
    println!("Token account rent returned: {} lamports", rent_received);
    assert!(rent_received > 0, "Should have received positive rent");
}

#[test]
fn test_close_token_account_with_balance_fails() {
    if !program_exists() {
        eprintln!("Skipping test_close_token_account_with_balance_fails: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    // Setup accounts
    let authority = funded_keypair_10_sol(&mut svm);
    let destination = funded_keypair_10_sol(&mut svm);

    // Create a mint
    let mint_keypair = solana_keypair::Keypair::new();
    let mint_account = create_mint_account(&authority.pubkey(), 9);
    svm.set_account(mint_keypair.pubkey(), mint_account).unwrap();

    // Create a token account with non-zero balance
    let token_account_keypair = solana_keypair::Keypair::new();
    let token_account = create_token_account(
        &authority.pubkey(),
        &mint_keypair.pubkey(),
        1000, // Non-zero balance - cannot close
    );
    svm.set_account(token_account_keypair.pubkey(), token_account).unwrap();

    // Build close_token_account instruction
    let ix = anchor_instruction(
        prog_id,
        "close_token_account",
        &close_token_account_data(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(token_account_keypair.pubkey()),
            signer_meta(destination.pubkey()),
            readonly_meta(token_program_id()),
        ],
    );

    // Execute transaction - should fail because account has balance
    let result = execute_tx(&mut svm, ix, &authority, &[&authority, &destination]);
    assert!(result.is_err(), "Should fail to close account with non-zero balance");
}

#[test]
fn test_close_token_account_wrong_authority_fails() {
    if !program_exists() {
        eprintln!("Skipping test_close_token_account_wrong_authority_fails: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    // Setup accounts
    let actual_owner = funded_keypair_10_sol(&mut svm);
    let wrong_authority = funded_keypair_10_sol(&mut svm);
    let destination = funded_keypair_10_sol(&mut svm);

    // Create a mint
    let mint_keypair = solana_keypair::Keypair::new();
    let mint_account = create_mint_account(&actual_owner.pubkey(), 9);
    svm.set_account(mint_keypair.pubkey(), mint_account).unwrap();

    // Create a token account owned by actual_owner
    let token_account_keypair = solana_keypair::Keypair::new();
    let token_account = create_token_account(
        &actual_owner.pubkey(), // actual_owner owns the token account
        &mint_keypair.pubkey(),
        0,
    );
    svm.set_account(token_account_keypair.pubkey(), token_account).unwrap();

    // Try to close with wrong_authority (not the owner)
    let ix = anchor_instruction(
        prog_id,
        "close_token_account",
        &close_token_account_data(),
        vec![
            signer_meta(wrong_authority.pubkey()), // Wrong authority
            writable_meta(token_account_keypair.pubkey()),
            signer_meta(destination.pubkey()),
            readonly_meta(token_program_id()),
        ],
    );

    // Execute transaction - should fail due to wrong authority
    let result = execute_tx(&mut svm, ix, &wrong_authority, &[&wrong_authority, &destination]);
    assert!(result.is_err(), "Should fail with wrong authority");
}

#[test]
fn test_close_token_account_destination_receives_exact_rent() {
    if !program_exists() {
        eprintln!("Skipping test_close_token_account_destination_receives_exact_rent: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    // Setup accounts
    let authority = funded_keypair_10_sol(&mut svm);
    let destination = funded_keypair_10_sol(&mut svm);

    // Create a mint
    let mint_keypair = solana_keypair::Keypair::new();
    let mint_account = create_mint_account(&authority.pubkey(), 9);
    svm.set_account(mint_keypair.pubkey(), mint_account).unwrap();

    // Create a token account
    let token_account_keypair = solana_keypair::Keypair::new();
    let token_account = create_token_account(
        &authority.pubkey(),
        &mint_keypair.pubkey(),
        0,
    );
    let expected_rent = token_account.lamports;
    svm.set_account(token_account_keypair.pubkey(), token_account).unwrap();

    // Record destination lamports before close
    let dest_lamports_before = get_account(&svm, &destination.pubkey()).unwrap().lamports;

    // Close the token account
    let ix = anchor_instruction(
        prog_id,
        "close_token_account",
        &close_token_account_data(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(token_account_keypair.pubkey()),
            signer_meta(destination.pubkey()),
            readonly_meta(token_program_id()),
        ],
    );

    execute_tx(&mut svm, ix, &authority, &[&authority, &destination]).unwrap();

    // Verify destination received exactly the token account's rent
    let dest_lamports_after = get_account(&svm, &destination.pubkey()).unwrap().lamports;
    let rent_received = dest_lamports_after - dest_lamports_before;

    assert_eq!(rent_received, expected_rent,
        "Destination should receive exact rent: expected={}, received={}",
        expected_rent, rent_received);
}

#[test]
fn test_close_multiple_token_accounts() {
    if !program_exists() {
        eprintln!("Skipping test_close_multiple_token_accounts: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    // Setup accounts
    let authority = funded_keypair_10_sol(&mut svm);
    let destination = funded_keypair_10_sol(&mut svm);

    // Create a mint
    let mint_keypair = solana_keypair::Keypair::new();
    let mint_account = create_mint_account(&authority.pubkey(), 9);
    svm.set_account(mint_keypair.pubkey(), mint_account).unwrap();

    // Create multiple token accounts
    let mut token_accounts = Vec::new();
    let mut total_rent = 0u64;

    for _ in 0..3 {
        let token_account_keypair = solana_keypair::Keypair::new();
        let token_account = create_token_account(
            &authority.pubkey(),
            &mint_keypair.pubkey(),
            0,
        );
        total_rent += token_account.lamports;
        svm.set_account(token_account_keypair.pubkey(), token_account).unwrap();
        token_accounts.push(token_account_keypair);
    }

    // Record destination lamports before closing
    let dest_lamports_before = get_account(&svm, &destination.pubkey()).unwrap().lamports;

    // Close all token accounts
    for token_account_keypair in &token_accounts {
        let ix = anchor_instruction(
            prog_id,
            "close_token_account",
            &close_token_account_data(),
            vec![
                signer_meta(authority.pubkey()),
                writable_meta(token_account_keypair.pubkey()),
                signer_meta(destination.pubkey()),
                readonly_meta(token_program_id()),
            ],
        );

        let result = execute_tx(&mut svm, ix, &authority, &[&authority, &destination]);
        assert!(result.is_ok(), "Failed to close token account: {:?}", result.err());
        svm.expire_blockhash();
    }

    // Verify destination received all the rent
    let dest_lamports_after = get_account(&svm, &destination.pubkey()).unwrap().lamports;
    let total_received = dest_lamports_after - dest_lamports_before;

    assert_eq!(total_received, total_rent,
        "Destination should receive total rent from all accounts: expected={}, received={}",
        total_rent, total_received);
}

#[test]
fn test_close_token_account_same_authority_and_destination() {
    if !program_exists() {
        eprintln!("Skipping test_close_token_account_same_authority_and_destination: program not built");
        return;
    }

    let mut svm = load_program();
    let prog_id = program_id();

    // Setup - authority and destination are the same account
    let authority = funded_keypair_10_sol(&mut svm);

    // Create a mint
    let mint_keypair = solana_keypair::Keypair::new();
    let mint_account = create_mint_account(&authority.pubkey(), 9);
    svm.set_account(mint_keypair.pubkey(), mint_account).unwrap();

    // Create a token account
    let token_account_keypair = solana_keypair::Keypair::new();
    let token_account = create_token_account(
        &authority.pubkey(),
        &mint_keypair.pubkey(),
        0,
    );
    let expected_rent = token_account.lamports;
    svm.set_account(token_account_keypair.pubkey(), token_account).unwrap();

    // Record authority lamports before close
    let auth_lamports_before = get_account(&svm, &authority.pubkey()).unwrap().lamports;

    // Close with authority as both authority and destination
    let ix = anchor_instruction(
        prog_id,
        "close_token_account",
        &close_token_account_data(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(token_account_keypair.pubkey()),
            signer_meta(authority.pubkey()), // Same as authority
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Should succeed with same authority and destination: {:?}", result.err());

    // Verify authority received rent (minus tx fee)
    let auth_lamports_after = get_account(&svm, &authority.pubkey()).unwrap().lamports;
    // Authority paid tx fee but received rent, so net change should be positive
    // (rent is ~2039280 for token account, tx fee is ~5000)
    let net_change = auth_lamports_after as i64 - auth_lamports_before as i64;
    assert!(net_change > 0,
        "Authority should have net positive change (rent - fee): net={}",
        net_change);
}
