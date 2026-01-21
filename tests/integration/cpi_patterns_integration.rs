//! Integration tests for CPI Patterns program
//!
//! Tests demonstrate various Cross-Program Invocation patterns:
//! 1. Basic CPI - Token transfer with user as signer
//! 2. CPI with PDA signer seeds - PDA-signed token operations
//! 3. Multiple CPIs - Chained operations in sequence
//! 4. Error handling across CPI boundary - Pre-validation patterns
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile cpi_patterns.so

use litesvm::LiteSVM;
use seahorse_integration_tests::{
    anchor_discriminator, find_pda,
};
use seahorse_integration_tests::helpers::{
    anchor_instruction, create_mint_account, create_token_account, execute_tx, funded_keypair,
    get_account_data, readonly_meta, signer_meta, token_program_id, writable_meta,
    read_token_balance, read_mint_supply, funded_keypair_10_sol,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::path::Path;

/// CPI Patterns program ID
const PROGRAM_ID: &str = "Cpi1Pattrn111111111111111111111111111111111";

/// Path to the compiled CPI Patterns program
const PROGRAM_PATH: &str = "../../target/deploy/cpi_patterns.so";

fn program_id() -> Pubkey {
    PROGRAM_ID.parse().unwrap()
}

/// Check if the CPI patterns program is built
fn program_exists() -> bool {
    Path::new(PROGRAM_PATH).exists()
}

/// CpiVault account layout:
/// - discriminator: 8 bytes
/// - authority: 32 bytes
/// - mint: 32 bytes
/// - total_deposited: 8 bytes
/// - total_withdrawn: 8 bytes
/// - transfer_count: 8 bytes
/// - bump: 1 byte
const CPI_VAULT_SIZE: usize = 8 + 32 + 32 + 8 + 8 + 8 + 1;

/// MintConfig account layout:
/// - discriminator: 8 bytes
/// - authority: 32 bytes
/// - mint: 32 bytes
/// - total_minted: 8 bytes
/// - operation_count: 8 bytes
/// - bump: 1 byte
const MINT_CONFIG_SIZE: usize = 8 + 32 + 32 + 8 + 8 + 1;

/// Load the CPI patterns program into LiteSVM
fn load_cpi_patterns_program() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();

    // Load the program if it exists
    if program_exists() {
        let program_bytes = std::fs::read(PROGRAM_PATH)
            .expect("Failed to read CPI patterns program");
        svm.add_program(program_id(), &program_bytes);
    }

    let authority = funded_keypair_10_sol(&mut svm);
    (svm, authority)
}

fn setup_svm() -> (LiteSVM, Keypair) {
    load_cpi_patterns_program()
}

/// Create a mint account with configurable authority for tests
fn create_test_mint(svm: &mut LiteSVM, authority: &Pubkey, decimals: u8) -> Pubkey {
    let mint = Keypair::new();

    let mint_account = create_mint_account(authority, decimals);
    svm.set_account(mint.pubkey(), mint_account).unwrap();

    mint.pubkey()
}

/// Create a mint account where the authority is a PDA (for mint_config tests)
fn create_pda_owned_mint(svm: &mut LiteSVM, mint_config_pda: &Pubkey, decimals: u8) -> Pubkey {
    let mint = Keypair::new();

    let mint_account = create_mint_account(mint_config_pda, decimals);
    svm.set_account(mint.pubkey(), mint_account).unwrap();

    mint.pubkey()
}

/// Create a token account with initial balance
fn create_test_token_account(
    svm: &mut LiteSVM,
    owner: &Pubkey,
    mint: &Pubkey,
    amount: u64,
) -> Pubkey {
    let token_account = Keypair::new();

    let account_data = create_token_account(owner, mint, amount);
    svm.set_account(token_account.pubkey(), account_data).unwrap();

    token_account.pubkey()
}

/// Create an empty token account for a PDA owner
fn create_pda_token_account(
    svm: &mut LiteSVM,
    pda_owner: &Pubkey,
    mint: &Pubkey,
    amount: u64,
) -> Pubkey {
    let token_account = Keypair::new();

    let account_data = create_token_account(pda_owner, mint, amount);
    svm.set_account(token_account.pubkey(), account_data).unwrap();

    token_account.pubkey()
}

// =============================================================================
// INITIALIZE VAULT TESTS
// =============================================================================

#[test]
fn test_initialize_vault() {
    if !program_exists() {
        eprintln!("Skipping test_initialize_vault: program not built. Run ./scripts/build-test-programs.sh");
        return;
    }

    let (mut svm, authority) = setup_svm();

    // Create mint
    let mint = create_test_mint(&mut svm, &authority.pubkey(), 9);

    // Derive vault PDA
    let (vault_pda, _bump) = find_pda(
        &[b"vault", authority.pubkey().as_ref(), mint.as_ref()],
        &program_id(),
    );

    // Derive vault token account PDA
    let (_vault_token_pda, _) = find_pda(
        &[b"vault_token", authority.pubkey().as_ref(), mint.as_ref()],
        &program_id(),
    );

    // Create vault token account (will be owned by vault PDA)
    let vault_token_account = create_pda_token_account(&mut svm, &vault_pda, &mint, 0);

    // Build instruction
    let ix = anchor_instruction(
        program_id(),
        "initialize_vault",
        &[],
        vec![
            signer_meta(authority.pubkey()),     // authority
            writable_meta(vault_pda),            // vault
            writable_meta(vault_token_account),  // vault_token_account
            writable_meta(mint),                 // mint
            readonly_meta(solana_sdk_ids::sysvar::rent::id()), // rent
            readonly_meta(system_program::id()), // system_program
            readonly_meta(token_program_id()),   // token_program
        ],
    );

    // Execute
    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "initialize_vault failed: {:?}", result.err());
}

// =============================================================================
// BASIC TRANSFER TO VAULT TESTS
// =============================================================================

#[test]
fn test_basic_transfer_to_vault() {
    if !program_exists() {
        eprintln!("Skipping test_basic_transfer_to_vault: program not built. Run ./scripts/build-test-programs.sh");
        return;
    }

    let (mut svm, authority) = setup_svm();

    // Create mint
    let mint = create_test_mint(&mut svm, &authority.pubkey(), 9);

    // Derive vault PDA
    let (vault_pda, bump) = find_pda(
        &[b"vault", authority.pubkey().as_ref(), mint.as_ref()],
        &program_id(),
    );

    // Create vault account data
    let vault_data = create_vault_account_data(&authority.pubkey(), &mint, 0, 0, 0, bump);
    svm.set_account(vault_pda, vault_data).unwrap();

    // Create user token account with 1000 tokens
    let user_token_account = create_test_token_account(&mut svm, &authority.pubkey(), &mint, 1000);

    // Create vault token account (owned by vault PDA)
    let vault_token_account = create_pda_token_account(&mut svm, &vault_pda, &mint, 0);

    // Build instruction
    let amount: u64 = 500;
    let ix = anchor_instruction(
        program_id(),
        "basic_transfer_to_vault",
        &amount.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),     // user
            writable_meta(vault_pda),            // vault
            writable_meta(user_token_account),   // user_token_account
            writable_meta(vault_token_account),  // vault_token_account
            writable_meta(mint),                 // mint
            readonly_meta(token_program_id()),   // token_program
        ],
    );

    // Execute
    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "basic_transfer_to_vault failed: {:?}", result.err());

    // Verify balances
    let user_balance = read_token_balance(&get_account_data(&svm, &user_token_account).unwrap());
    let vault_balance = read_token_balance(&get_account_data(&svm, &vault_token_account).unwrap());

    assert_eq!(user_balance, 500, "User should have 500 tokens remaining");
    assert_eq!(vault_balance, 500, "Vault should have 500 tokens");
}

#[test]
fn test_basic_transfer_to_vault_zero_amount_fails() {
    let (mut svm, authority) = setup_svm();

    let mint = create_test_mint(&mut svm, &authority.pubkey(), 9);

    let (vault_pda, bump) = find_pda(
        &[b"vault", authority.pubkey().as_ref(), mint.as_ref()],
        &program_id(),
    );

    let vault_data = create_vault_account_data(&authority.pubkey(), &mint, 0, 0, 0, bump);
    svm.set_account(vault_pda, vault_data).unwrap();

    let user_token_account = create_test_token_account(&mut svm, &authority.pubkey(), &mint, 1000);
    let vault_token_account = create_pda_token_account(&mut svm, &vault_pda, &mint, 0);

    // Try to transfer 0 tokens
    let amount: u64 = 0;
    let ix = anchor_instruction(
        program_id(),
        "basic_transfer_to_vault",
        &amount.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(vault_pda),
            writable_meta(user_token_account),
            writable_meta(vault_token_account),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "Should fail with zero amount");
}

#[test]
fn test_basic_transfer_to_vault_multiple_deposits() {
    if !program_exists() {
        eprintln!("Skipping test_basic_transfer_to_vault_multiple_deposits: program not built. Run ./scripts/build-test-programs.sh");
        return;
    }

    let (mut svm, authority) = setup_svm();

    let mint = create_test_mint(&mut svm, &authority.pubkey(), 9);

    let (vault_pda, bump) = find_pda(
        &[b"vault", authority.pubkey().as_ref(), mint.as_ref()],
        &program_id(),
    );

    let vault_data = create_vault_account_data(&authority.pubkey(), &mint, 0, 0, 0, bump);
    svm.set_account(vault_pda, vault_data).unwrap();

    let user_token_account = create_test_token_account(&mut svm, &authority.pubkey(), &mint, 1000);
    let vault_token_account = create_pda_token_account(&mut svm, &vault_pda, &mint, 0);

    // First deposit
    let amount1: u64 = 300;
    let ix1 = anchor_instruction(
        program_id(),
        "basic_transfer_to_vault",
        &amount1.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(vault_pda),
            writable_meta(user_token_account),
            writable_meta(vault_token_account),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );
    let result1 = execute_tx(&mut svm, ix1, &authority, &[&authority]);
    assert!(result1.is_ok(), "First deposit failed");

    // Expire blockhash for next transaction
    svm.expire_blockhash();

    // Second deposit
    let amount2: u64 = 400;
    let ix2 = anchor_instruction(
        program_id(),
        "basic_transfer_to_vault",
        &amount2.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(vault_pda),
            writable_meta(user_token_account),
            writable_meta(vault_token_account),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );
    let result2 = execute_tx(&mut svm, ix2, &authority, &[&authority]);
    assert!(result2.is_ok(), "Second deposit failed");

    // Verify final balances
    let user_balance = read_token_balance(&get_account_data(&svm, &user_token_account).unwrap());
    let vault_balance = read_token_balance(&get_account_data(&svm, &vault_token_account).unwrap());

    assert_eq!(user_balance, 300, "User should have 300 tokens remaining");
    assert_eq!(vault_balance, 700, "Vault should have 700 tokens");
}

// =============================================================================
// PDA-SIGNED TRANSFER FROM VAULT TESTS
// =============================================================================

#[test]
fn test_pda_signed_transfer_from_vault() {
    if !program_exists() {
        eprintln!("Skipping test_pda_signed_transfer_from_vault: program not built. Run ./scripts/build-test-programs.sh");
        return;
    }

    let (mut svm, authority) = setup_svm();

    let mint = create_test_mint(&mut svm, &authority.pubkey(), 9);

    let (vault_pda, bump) = find_pda(
        &[b"vault", authority.pubkey().as_ref(), mint.as_ref()],
        &program_id(),
    );

    // Create vault with some deposited tokens
    let vault_data = create_vault_account_data(&authority.pubkey(), &mint, 1000, 0, 1, bump);
    svm.set_account(vault_pda, vault_data).unwrap();

    // Create user token account (empty)
    let user_token_account = create_test_token_account(&mut svm, &authority.pubkey(), &mint, 0);

    // Create vault token account with tokens
    let vault_token_account = create_pda_token_account(&mut svm, &vault_pda, &mint, 1000);

    // Withdraw using PDA-signed transfer
    let amount: u64 = 400;
    let ix = anchor_instruction(
        program_id(),
        "pda_signed_transfer_from_vault",
        &amount.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),     // authority
            writable_meta(vault_pda),            // vault
            writable_meta(user_token_account),   // user_token_account
            writable_meta(vault_token_account),  // vault_token_account
            writable_meta(mint),                 // mint
            readonly_meta(token_program_id()),   // token_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "PDA-signed transfer failed: {:?}", result.err());

    // Verify balances
    let user_balance = read_token_balance(&get_account_data(&svm, &user_token_account).unwrap());
    let vault_balance = read_token_balance(&get_account_data(&svm, &vault_token_account).unwrap());

    assert_eq!(user_balance, 400, "User should have 400 tokens");
    assert_eq!(vault_balance, 600, "Vault should have 600 tokens remaining");
}

#[test]
fn test_pda_signed_transfer_unauthorized_fails() {
    let (mut svm, authority) = setup_svm();
    let unauthorized = funded_keypair(&mut svm, 10_000_000_000);

    let mint = create_test_mint(&mut svm, &authority.pubkey(), 9);

    let (vault_pda, bump) = find_pda(
        &[b"vault", authority.pubkey().as_ref(), mint.as_ref()],
        &program_id(),
    );

    // Vault is owned by authority, not unauthorized
    let vault_data = create_vault_account_data(&authority.pubkey(), &mint, 1000, 0, 1, bump);
    svm.set_account(vault_pda, vault_data).unwrap();

    let user_token_account = create_test_token_account(&mut svm, &unauthorized.pubkey(), &mint, 0);
    let vault_token_account = create_pda_token_account(&mut svm, &vault_pda, &mint, 1000);

    // Try to withdraw with unauthorized signer
    let amount: u64 = 100;
    let ix = anchor_instruction(
        program_id(),
        "pda_signed_transfer_from_vault",
        &amount.to_le_bytes(),
        vec![
            signer_meta(unauthorized.pubkey()), // wrong authority!
            writable_meta(vault_pda),
            writable_meta(user_token_account),
            writable_meta(vault_token_account),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &unauthorized, &[&unauthorized]);
    assert!(result.is_err(), "Should fail with unauthorized authority");
}

#[test]
fn test_pda_signed_transfer_insufficient_funds_fails() {
    let (mut svm, authority) = setup_svm();

    let mint = create_test_mint(&mut svm, &authority.pubkey(), 9);

    let (vault_pda, bump) = find_pda(
        &[b"vault", authority.pubkey().as_ref(), mint.as_ref()],
        &program_id(),
    );

    // Vault has 100 tokens deposited
    let vault_data = create_vault_account_data(&authority.pubkey(), &mint, 100, 0, 1, bump);
    svm.set_account(vault_pda, vault_data).unwrap();

    let user_token_account = create_test_token_account(&mut svm, &authority.pubkey(), &mint, 0);
    // Vault token account only has 100 tokens
    let vault_token_account = create_pda_token_account(&mut svm, &vault_pda, &mint, 100);

    // Try to withdraw more than available
    let amount: u64 = 500;
    let ix = anchor_instruction(
        program_id(),
        "pda_signed_transfer_from_vault",
        &amount.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(vault_pda),
            writable_meta(user_token_account),
            writable_meta(vault_token_account),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "Should fail with insufficient funds");
}

// =============================================================================
// CHAINED MINT AND TRANSFER TESTS
// =============================================================================

#[test]
fn test_chained_mint_and_transfer() {
    if !program_exists() {
        eprintln!("Skipping test_chained_mint_and_transfer: program not built. Run ./scripts/build-test-programs.sh");
        return;
    }

    let (mut svm, authority) = setup_svm();

    // Derive mint_config PDA
    let (mint_config_pda, bump) = find_pda(
        &[b"mint_config", authority.pubkey().as_ref()],
        &program_id(),
    );

    // Create mint with mint_config_pda as authority
    let mint = create_pda_owned_mint(&mut svm, &mint_config_pda, 9);

    // Create mint_config account
    let mint_config_data = create_mint_config_account_data(&authority.pubkey(), &mint, 0, 0, bump);
    svm.set_account(mint_config_pda, mint_config_data).unwrap();

    // Create intermediate token account (owned by authority)
    let intermediate_account = create_test_token_account(&mut svm, &authority.pubkey(), &mint, 0);

    // Create destination token account
    let destination_account = create_test_token_account(&mut svm, &authority.pubkey(), &mint, 0);

    // Mint 1000 tokens, transfer 300 to destination
    let mint_amount: u64 = 1000;
    let transfer_amount: u64 = 300;

    let mut data = Vec::new();
    data.extend_from_slice(&mint_amount.to_le_bytes());
    data.extend_from_slice(&transfer_amount.to_le_bytes());

    let ix = anchor_instruction(
        program_id(),
        "chained_mint_and_transfer",
        &data,
        vec![
            signer_meta(authority.pubkey()),       // authority
            writable_meta(mint_config_pda),        // mint_config
            writable_meta(mint),                   // mint
            writable_meta(intermediate_account),   // intermediate_account
            writable_meta(destination_account),    // destination_account
            readonly_meta(token_program_id()),     // token_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Chained mint and transfer failed: {:?}", result.err());

    // Verify balances
    let intermediate_balance = read_token_balance(&get_account_data(&svm, &intermediate_account).unwrap());
    let destination_balance = read_token_balance(&get_account_data(&svm, &destination_account).unwrap());

    assert_eq!(intermediate_balance, 700, "Intermediate should have 700 tokens");
    assert_eq!(destination_balance, 300, "Destination should have 300 tokens");

    // Verify mint supply
    let supply = read_mint_supply(&get_account_data(&svm, &mint).unwrap());
    assert_eq!(supply, 1000, "Total supply should be 1000");
}

#[test]
fn test_chained_mint_and_transfer_zero_amount_fails() {
    let (mut svm, authority) = setup_svm();

    let (mint_config_pda, bump) = find_pda(
        &[b"mint_config", authority.pubkey().as_ref()],
        &program_id(),
    );

    let mint = create_pda_owned_mint(&mut svm, &mint_config_pda, 9);
    let mint_config_data = create_mint_config_account_data(&authority.pubkey(), &mint, 0, 0, bump);
    svm.set_account(mint_config_pda, mint_config_data).unwrap();

    let intermediate_account = create_test_token_account(&mut svm, &authority.pubkey(), &mint, 0);
    let destination_account = create_test_token_account(&mut svm, &authority.pubkey(), &mint, 0);

    // Try to mint 0 tokens
    let mint_amount: u64 = 0;
    let transfer_amount: u64 = 0;

    let mut data = Vec::new();
    data.extend_from_slice(&mint_amount.to_le_bytes());
    data.extend_from_slice(&transfer_amount.to_le_bytes());

    let ix = anchor_instruction(
        program_id(),
        "chained_mint_and_transfer",
        &data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint_config_pda),
            writable_meta(mint),
            writable_meta(intermediate_account),
            writable_meta(destination_account),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "Should fail with zero mint amount");
}

#[test]
fn test_chained_mint_and_transfer_exceeds_mint_fails() {
    let (mut svm, authority) = setup_svm();

    let (mint_config_pda, bump) = find_pda(
        &[b"mint_config", authority.pubkey().as_ref()],
        &program_id(),
    );

    let mint = create_pda_owned_mint(&mut svm, &mint_config_pda, 9);
    let mint_config_data = create_mint_config_account_data(&authority.pubkey(), &mint, 0, 0, bump);
    svm.set_account(mint_config_pda, mint_config_data).unwrap();

    let intermediate_account = create_test_token_account(&mut svm, &authority.pubkey(), &mint, 0);
    let destination_account = create_test_token_account(&mut svm, &authority.pubkey(), &mint, 0);

    // Try to transfer more than minted
    let mint_amount: u64 = 100;
    let transfer_amount: u64 = 500; // More than minted!

    let mut data = Vec::new();
    data.extend_from_slice(&mint_amount.to_le_bytes());
    data.extend_from_slice(&transfer_amount.to_le_bytes());

    let ix = anchor_instruction(
        program_id(),
        "chained_mint_and_transfer",
        &data,
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint_config_pda),
            writable_meta(mint),
            writable_meta(intermediate_account),
            writable_meta(destination_account),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "Should fail when transfer exceeds mint");
}

// =============================================================================
// VALIDATED BURN TESTS
// =============================================================================

#[test]
fn test_validated_burn() {
    if !program_exists() {
        eprintln!("Skipping test_validated_burn: program not built. Run ./scripts/build-test-programs.sh");
        return;
    }

    let (mut svm, authority) = setup_svm();

    let mint = create_test_mint(&mut svm, &authority.pubkey(), 9);
    let source_account = create_test_token_account(&mut svm, &authority.pubkey(), &mint, 1000);

    // Burn 300 tokens
    let amount: u64 = 300;
    let ix = anchor_instruction(
        program_id(),
        "validated_burn",
        &amount.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),   // owner
            writable_meta(source_account),     // source_account
            writable_meta(mint),               // mint
            readonly_meta(token_program_id()), // token_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Validated burn failed: {:?}", result.err());

    // Verify balance
    let balance = read_token_balance(&get_account_data(&svm, &source_account).unwrap());
    assert_eq!(balance, 700, "Should have 700 tokens remaining");
}

#[test]
fn test_validated_burn_zero_amount_fails() {
    let (mut svm, authority) = setup_svm();

    let mint = create_test_mint(&mut svm, &authority.pubkey(), 9);
    let source_account = create_test_token_account(&mut svm, &authority.pubkey(), &mint, 1000);

    let amount: u64 = 0;
    let ix = anchor_instruction(
        program_id(),
        "validated_burn",
        &amount.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(source_account),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "Should fail with zero amount");
}

#[test]
fn test_validated_burn_insufficient_balance_fails() {
    let (mut svm, authority) = setup_svm();

    let mint = create_test_mint(&mut svm, &authority.pubkey(), 9);
    let source_account = create_test_token_account(&mut svm, &authority.pubkey(), &mint, 100);

    // Try to burn more than balance
    let amount: u64 = 500;
    let ix = anchor_instruction(
        program_id(),
        "validated_burn",
        &amount.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(source_account),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_err(), "Should fail with insufficient balance");
}

// =============================================================================
// FULL WORKFLOW TESTS
// =============================================================================

#[test]
fn test_full_vault_workflow_deposit_withdraw() {
    if !program_exists() {
        eprintln!("Skipping test_full_vault_workflow_deposit_withdraw: program not built. Run ./scripts/build-test-programs.sh");
        return;
    }

    let (mut svm, authority) = setup_svm();

    let mint = create_test_mint(&mut svm, &authority.pubkey(), 9);

    let (vault_pda, bump) = find_pda(
        &[b"vault", authority.pubkey().as_ref(), mint.as_ref()],
        &program_id(),
    );

    let vault_data = create_vault_account_data(&authority.pubkey(), &mint, 0, 0, 0, bump);
    svm.set_account(vault_pda, vault_data).unwrap();

    let user_token_account = create_test_token_account(&mut svm, &authority.pubkey(), &mint, 1000);
    let vault_token_account = create_pda_token_account(&mut svm, &vault_pda, &mint, 0);

    // Step 1: Deposit 500 tokens (basic CPI)
    let deposit_amount: u64 = 500;
    let ix1 = anchor_instruction(
        program_id(),
        "basic_transfer_to_vault",
        &deposit_amount.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(vault_pda),
            writable_meta(user_token_account),
            writable_meta(vault_token_account),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );
    let result1 = execute_tx(&mut svm, ix1, &authority, &[&authority]);
    assert!(result1.is_ok(), "Deposit failed");

    // Verify intermediate state
    let user_balance = read_token_balance(&get_account_data(&svm, &user_token_account).unwrap());
    let vault_balance = read_token_balance(&get_account_data(&svm, &vault_token_account).unwrap());
    assert_eq!(user_balance, 500);
    assert_eq!(vault_balance, 500);

    svm.expire_blockhash();

    // Step 2: Withdraw 200 tokens (PDA-signed CPI)
    let withdraw_amount: u64 = 200;
    let ix2 = anchor_instruction(
        program_id(),
        "pda_signed_transfer_from_vault",
        &withdraw_amount.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(vault_pda),
            writable_meta(user_token_account),
            writable_meta(vault_token_account),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );
    let result2 = execute_tx(&mut svm, ix2, &authority, &[&authority]);
    assert!(result2.is_ok(), "Withdraw failed");

    // Verify final state
    let user_balance = read_token_balance(&get_account_data(&svm, &user_token_account).unwrap());
    let vault_balance = read_token_balance(&get_account_data(&svm, &vault_token_account).unwrap());
    assert_eq!(user_balance, 700, "User should have 700 tokens (1000 - 500 + 200)");
    assert_eq!(vault_balance, 300, "Vault should have 300 tokens (500 - 200)");
}

#[test]
fn test_pda_derivation_deterministic() {
    let authority = Keypair::new();
    let mint = Pubkey::new_unique();

    // Derive vault PDA
    let (vault_pda1, bump1) = find_pda(
        &[b"vault", authority.pubkey().as_ref(), mint.as_ref()],
        &program_id(),
    );

    // Derive again - should be identical
    let (vault_pda2, bump2) = find_pda(
        &[b"vault", authority.pubkey().as_ref(), mint.as_ref()],
        &program_id(),
    );

    assert_eq!(vault_pda1, vault_pda2, "PDA derivation should be deterministic");
    assert_eq!(bump1, bump2, "Bump should be deterministic");

    // Derive mint_config PDA
    let (config_pda1, config_bump1) = find_pda(
        &[b"mint_config", authority.pubkey().as_ref()],
        &program_id(),
    );

    let (config_pda2, config_bump2) = find_pda(
        &[b"mint_config", authority.pubkey().as_ref()],
        &program_id(),
    );

    assert_eq!(config_pda1, config_pda2, "MintConfig PDA should be deterministic");
    assert_eq!(config_bump1, config_bump2, "MintConfig bump should be deterministic");
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

fn create_vault_account_data(
    authority: &Pubkey,
    mint: &Pubkey,
    total_deposited: u64,
    total_withdrawn: u64,
    transfer_count: u64,
    bump: u8,
) -> solana_account::Account {
    let rent = Rent::default();

    // Account discriminator for CpiVault
    let discriminator = anchor_discriminator("CpiVault");

    let mut data = vec![0u8; CPI_VAULT_SIZE];
    data[..8].copy_from_slice(&discriminator);
    data[8..40].copy_from_slice(authority.as_ref());
    data[40..72].copy_from_slice(mint.as_ref());
    data[72..80].copy_from_slice(&total_deposited.to_le_bytes());
    data[80..88].copy_from_slice(&total_withdrawn.to_le_bytes());
    data[88..96].copy_from_slice(&transfer_count.to_le_bytes());
    data[96] = bump;

    solana_account::Account {
        lamports: rent.minimum_balance(CPI_VAULT_SIZE),
        data,
        owner: program_id(),
        executable: false,
        rent_epoch: 0,
    }
}

fn create_mint_config_account_data(
    authority: &Pubkey,
    mint: &Pubkey,
    total_minted: u64,
    operation_count: u64,
    bump: u8,
) -> solana_account::Account {
    let rent = Rent::default();

    // Account discriminator for MintConfig
    let discriminator = anchor_discriminator("MintConfig");

    let mut data = vec![0u8; MINT_CONFIG_SIZE];
    data[..8].copy_from_slice(&discriminator);
    data[8..40].copy_from_slice(authority.as_ref());
    data[40..72].copy_from_slice(mint.as_ref());
    data[72..80].copy_from_slice(&total_minted.to_le_bytes());
    data[80..88].copy_from_slice(&operation_count.to_le_bytes());
    data[88] = bump;

    solana_account::Account {
        lamports: rent.minimum_balance(MINT_CONFIG_SIZE),
        data,
        owner: program_id(),
        executable: false,
        rent_epoch: 0,
    }
}
