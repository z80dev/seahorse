//! LiteSVM integration tests for the Seahorse Token Vault program
//!
//! These tests verify SPL Token deposit/withdraw flows with PDA-owned token accounts.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile token_vault.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// Token Vault program ID (from declare_id!)
fn token_vault_program_id() -> Pubkey {
    Pubkey::from_str("3JwSuBw6X2q2FknhbhfvXnuFhdeJN9KCTtpGk6Qx9mLL").unwrap()
}

/// Vault account size: discriminator (8) + owner (32) + mint (32) + total_deposits (8) + bump (1)
const VAULT_SIZE: usize = 8 + 32 + 32 + 8 + 1;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the token vault program into LiteSVM
fn load_token_vault_program() -> litesvm::LiteSVM {
    let program_id = token_vault_program_id();
    let program_bytes = std::fs::read("../../target/deploy/token_vault.so")
        .expect("Failed to read token_vault.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);
    svm
}

/// Setup a mint and mint authority for testing
fn setup_mint(svm: &mut litesvm::LiteSVM) -> (Keypair, Pubkey) {
    let mint_authority = funded_keypair_10_sol(svm);
    let mint = Keypair::new();

    // Create the mint account directly in SVM
    let mint_account = create_mint_account(&mint_authority.pubkey(), 6);
    svm.set_account(mint.pubkey(), mint_account).unwrap();

    (mint_authority, mint.pubkey())
}

/// Setup a user with an ATA containing tokens
fn setup_user_with_tokens(
    svm: &mut litesvm::LiteSVM,
    mint: &Pubkey,
    amount: u64,
) -> (Keypair, Pubkey) {
    let user = funded_keypair_10_sol(svm);
    let user_ata = get_ata(&user.pubkey(), mint);

    // Create the user's token account with initial balance
    let token_account = create_token_account(&user.pubkey(), mint, amount);
    svm.set_account(user_ata, token_account).unwrap();

    (user, user_ata)
}

/// Derive vault PDA
fn derive_vault_pda(owner: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"vault", owner.as_ref(), mint.as_ref()],
        &token_vault_program_id(),
    )
}

/// Derive vault token account PDA
fn derive_vault_token_pda(owner: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"vault_token", owner.as_ref(), mint.as_ref()],
        &token_vault_program_id(),
    )
}

/// Read total deposits from vault account data
fn read_vault_total_deposits(data: &[u8]) -> u64 {
    // Skip discriminator (8) + owner (32) + mint (32), read total_deposits (8)
    let deposits_bytes: [u8; 8] = data[72..80].try_into().unwrap();
    u64::from_le_bytes(deposits_bytes)
}

/// Read vault owner from account data
fn read_vault_owner(data: &[u8]) -> Pubkey {
    // Skip discriminator (8), read owner (32)
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

/// Read vault mint from account data
fn read_vault_mint(data: &[u8]) -> Pubkey {
    // Skip discriminator (8) + owner (32), read mint (32)
    Pubkey::new_from_array(data[40..72].try_into().unwrap())
}

// =============================================================================
// INITIALIZE VAULT TESTS
// =============================================================================

#[test]
fn test_initialize_vault() {
    let mut svm = load_token_vault_program();
    let program_id = token_vault_program_id();

    let (mint_authority, mint) = setup_mint(&mut svm);
    let owner = funded_keypair_10_sol(&mut svm);

    let (vault_pda, _vault_bump) = derive_vault_pda(&owner.pubkey(), &mint);
    let (vault_token_pda, _vault_token_bump) = derive_vault_token_pda(&owner.pubkey(), &mint);

    // Create initialize_vault instruction
    let ix = anchor_instruction(
        program_id,
        "initialize_vault",
        &[],
        vec![
            signer_meta(owner.pubkey()),        // owner
            writable_meta(vault_pda),           // vault
            writable_meta(vault_token_pda),     // vault_token_account
            writable_meta(mint),                // mint
            readonly_meta(solana_sdk_ids::sysvar::rent::id()), // rent
            readonly_meta(system_program::id()), // system_program
            readonly_meta(token_program_id()),   // token_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Initialize vault should succeed: {:?}", result);

    // Verify vault state
    let vault_account = svm.get_account(&vault_pda).expect("Vault should exist");
    assert_eq!(vault_account.owner, program_id, "Vault should be owned by program");

    let vault_owner = read_vault_owner(&vault_account.data);
    assert_eq!(vault_owner, owner.pubkey(), "Vault owner should match");

    let vault_mint = read_vault_mint(&vault_account.data);
    assert_eq!(vault_mint, mint, "Vault mint should match");

    let total_deposits = read_vault_total_deposits(&vault_account.data);
    assert_eq!(total_deposits, 0, "Initial deposits should be 0");

    // Verify vault token account was created
    let vault_token_account = svm.get_account(&vault_token_pda).expect("Vault token account should exist");
    assert_eq!(vault_token_account.owner, token_program_id(), "Token account should be owned by token program");

    let vault_token_balance = read_token_balance(&vault_token_account.data);
    assert_eq!(vault_token_balance, 0, "Vault token balance should be 0");
}

#[test]
fn test_initialize_vault_different_users() {
    let mut svm = load_token_vault_program();
    let program_id = token_vault_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let user1 = funded_keypair_10_sol(&mut svm);
    let user2 = funded_keypair_10_sol(&mut svm);

    // User 1's vault PDAs
    let (vault1_pda, _) = derive_vault_pda(&user1.pubkey(), &mint);
    let (vault1_token_pda, _) = derive_vault_token_pda(&user1.pubkey(), &mint);

    // User 2's vault PDAs
    let (vault2_pda, _) = derive_vault_pda(&user2.pubkey(), &mint);
    let (vault2_token_pda, _) = derive_vault_token_pda(&user2.pubkey(), &mint);

    // PDAs should be different
    assert_ne!(vault1_pda, vault2_pda, "Vault PDAs should differ between users");
    assert_ne!(vault1_token_pda, vault2_token_pda, "Vault token PDAs should differ");

    // Initialize user1's vault
    let ix1 = anchor_instruction(
        program_id,
        "initialize_vault",
        &[],
        vec![
            signer_meta(user1.pubkey()),
            writable_meta(vault1_pda),
            writable_meta(vault1_token_pda),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    let result1 = execute_tx(&mut svm, ix1, &user1, &[&user1]);
    assert!(result1.is_ok(), "User1 init should succeed");

    // Initialize user2's vault
    svm.expire_blockhash();
    let ix2 = anchor_instruction(
        program_id,
        "initialize_vault",
        &[],
        vec![
            signer_meta(user2.pubkey()),
            writable_meta(vault2_pda),
            writable_meta(vault2_token_pda),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    let result2 = execute_tx(&mut svm, ix2, &user2, &[&user2]);
    assert!(result2.is_ok(), "User2 init should succeed");

    // Verify both vaults exist independently
    let vault1 = svm.get_account(&vault1_pda).unwrap();
    let vault2 = svm.get_account(&vault2_pda).unwrap();

    assert_eq!(read_vault_owner(&vault1.data), user1.pubkey());
    assert_eq!(read_vault_owner(&vault2.data), user2.pubkey());
}

// =============================================================================
// DEPOSIT TESTS
// =============================================================================

#[test]
fn test_deposit() {
    let mut svm = load_token_vault_program();
    let program_id = token_vault_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let owner = funded_keypair_10_sol(&mut svm);
    let (user, user_ata) = setup_user_with_tokens(&mut svm, &mint, 1000);

    // Initialize vault
    let (vault_pda, _) = derive_vault_pda(&owner.pubkey(), &mint);
    let (vault_token_pda, _) = derive_vault_token_pda(&owner.pubkey(), &mint);

    let init_ix = anchor_instruction(
        program_id,
        "initialize_vault",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // Deposit 500 tokens
    svm.expire_blockhash();
    let deposit_amount: u64 = 500;
    let deposit_ix = anchor_instruction(
        program_id,
        "deposit",
        &deposit_amount.to_le_bytes(),
        vec![
            signer_meta(user.pubkey()),         // user
            writable_meta(vault_pda),           // vault
            writable_meta(user_ata),            // user_token_account
            writable_meta(vault_token_pda),     // vault_token_account
            writable_meta(mint),                // mint
            readonly_meta(token_program_id()),  // token_program
        ],
    );

    let result = execute_tx(&mut svm, deposit_ix, &user, &[&user]);
    assert!(result.is_ok(), "Deposit should succeed: {:?}", result);

    // Verify balances
    let user_account = svm.get_account(&user_ata).unwrap();
    assert_eq!(read_token_balance(&user_account.data), 500, "User should have 500 tokens left");

    let vault_token_account = svm.get_account(&vault_token_pda).unwrap();
    assert_eq!(read_token_balance(&vault_token_account.data), 500, "Vault should have 500 tokens");

    let vault_account = svm.get_account(&vault_pda).unwrap();
    assert_eq!(read_vault_total_deposits(&vault_account.data), 500, "Total deposits should be 500");
}

#[test]
fn test_deposit_multiple_times() {
    let mut svm = load_token_vault_program();
    let program_id = token_vault_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let owner = funded_keypair_10_sol(&mut svm);
    let (user, user_ata) = setup_user_with_tokens(&mut svm, &mint, 1000);

    // Initialize vault
    let (vault_pda, _) = derive_vault_pda(&owner.pubkey(), &mint);
    let (vault_token_pda, _) = derive_vault_token_pda(&owner.pubkey(), &mint);

    let init_ix = anchor_instruction(
        program_id,
        "initialize_vault",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // Deposit 3 times
    for i in 1..=3 {
        svm.expire_blockhash();
        let deposit_amount: u64 = 100;
        let deposit_ix = anchor_instruction(
            program_id,
            "deposit",
            &deposit_amount.to_le_bytes(),
            vec![
                signer_meta(user.pubkey()),
                writable_meta(vault_pda),
                writable_meta(user_ata),
                writable_meta(vault_token_pda),
                writable_meta(mint),
                readonly_meta(token_program_id()),
            ],
        );
        let result = execute_tx(&mut svm, deposit_ix, &user, &[&user]);
        assert!(result.is_ok(), "Deposit {} should succeed", i);
    }

    // Verify final balances
    let user_account = svm.get_account(&user_ata).unwrap();
    assert_eq!(read_token_balance(&user_account.data), 700, "User should have 700 tokens left");

    let vault_token_account = svm.get_account(&vault_token_pda).unwrap();
    assert_eq!(read_token_balance(&vault_token_account.data), 300, "Vault should have 300 tokens");

    let vault_account = svm.get_account(&vault_pda).unwrap();
    assert_eq!(read_vault_total_deposits(&vault_account.data), 300, "Total deposits should be 300");
}

#[test]
fn test_deposit_zero_fails() {
    let mut svm = load_token_vault_program();
    let program_id = token_vault_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let owner = funded_keypair_10_sol(&mut svm);
    let (user, user_ata) = setup_user_with_tokens(&mut svm, &mint, 1000);

    // Initialize vault
    let (vault_pda, _) = derive_vault_pda(&owner.pubkey(), &mint);
    let (vault_token_pda, _) = derive_vault_token_pda(&owner.pubkey(), &mint);

    let init_ix = anchor_instruction(
        program_id,
        "initialize_vault",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // Try to deposit 0 tokens - should fail
    svm.expire_blockhash();
    let deposit_amount: u64 = 0;
    let deposit_ix = anchor_instruction(
        program_id,
        "deposit",
        &deposit_amount.to_le_bytes(),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(vault_pda),
            writable_meta(user_ata),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, deposit_ix, &user, &[&user]);
    assert!(result.is_err(), "Deposit of 0 should fail");
}

// =============================================================================
// WITHDRAW TESTS
// =============================================================================

#[test]
fn test_withdraw() {
    let mut svm = load_token_vault_program();
    let program_id = token_vault_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let (owner, owner_ata) = setup_user_with_tokens(&mut svm, &mint, 1000);

    // Initialize vault
    let (vault_pda, _) = derive_vault_pda(&owner.pubkey(), &mint);
    let (vault_token_pda, _) = derive_vault_token_pda(&owner.pubkey(), &mint);

    let init_ix = anchor_instruction(
        program_id,
        "initialize_vault",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // Deposit 500 tokens
    svm.expire_blockhash();
    let deposit_amount: u64 = 500;
    let deposit_ix = anchor_instruction(
        program_id,
        "deposit",
        &deposit_amount.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            writable_meta(owner_ata),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, deposit_ix, &owner, &[&owner]).unwrap();

    // Withdraw 200 tokens
    svm.expire_blockhash();
    let withdraw_amount: u64 = 200;
    let withdraw_ix = anchor_instruction(
        program_id,
        "withdraw",
        &withdraw_amount.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),        // owner
            writable_meta(vault_pda),           // vault
            writable_meta(owner_ata),           // owner_token_account
            writable_meta(vault_token_pda),     // vault_token_account
            writable_meta(mint),                // mint
            readonly_meta(token_program_id()),  // token_program
        ],
    );

    let result = execute_tx(&mut svm, withdraw_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Withdraw should succeed: {:?}", result);

    // Verify balances
    let owner_account = svm.get_account(&owner_ata).unwrap();
    assert_eq!(read_token_balance(&owner_account.data), 700, "Owner should have 700 tokens");

    let vault_token_account = svm.get_account(&vault_token_pda).unwrap();
    assert_eq!(read_token_balance(&vault_token_account.data), 300, "Vault should have 300 tokens");

    let vault_account = svm.get_account(&vault_pda).unwrap();
    assert_eq!(read_vault_total_deposits(&vault_account.data), 300, "Total deposits should be 300");
}

#[test]
fn test_withdraw_all() {
    let mut svm = load_token_vault_program();
    let program_id = token_vault_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let (owner, owner_ata) = setup_user_with_tokens(&mut svm, &mint, 1000);

    // Initialize and deposit
    let (vault_pda, _) = derive_vault_pda(&owner.pubkey(), &mint);
    let (vault_token_pda, _) = derive_vault_token_pda(&owner.pubkey(), &mint);

    let init_ix = anchor_instruction(
        program_id,
        "initialize_vault",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    svm.expire_blockhash();
    let deposit_amount: u64 = 500;
    let deposit_ix = anchor_instruction(
        program_id,
        "deposit",
        &deposit_amount.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            writable_meta(owner_ata),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, deposit_ix, &owner, &[&owner]).unwrap();

    // Withdraw all tokens
    svm.expire_blockhash();
    let withdraw_amount: u64 = 500;
    let withdraw_ix = anchor_instruction(
        program_id,
        "withdraw",
        &withdraw_amount.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            writable_meta(owner_ata),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, withdraw_ix, &owner, &[&owner]);
    assert!(result.is_ok(), "Withdraw all should succeed");

    // Verify vault is empty
    let vault_token_account = svm.get_account(&vault_token_pda).unwrap();
    assert_eq!(read_token_balance(&vault_token_account.data), 0, "Vault should be empty");

    let vault_account = svm.get_account(&vault_pda).unwrap();
    assert_eq!(read_vault_total_deposits(&vault_account.data), 0, "Total deposits should be 0");
}

#[test]
fn test_withdraw_zero_fails() {
    let mut svm = load_token_vault_program();
    let program_id = token_vault_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let (owner, owner_ata) = setup_user_with_tokens(&mut svm, &mint, 1000);

    // Initialize and deposit
    let (vault_pda, _) = derive_vault_pda(&owner.pubkey(), &mint);
    let (vault_token_pda, _) = derive_vault_token_pda(&owner.pubkey(), &mint);

    let init_ix = anchor_instruction(
        program_id,
        "initialize_vault",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    svm.expire_blockhash();
    let deposit_amount: u64 = 500;
    let deposit_ix = anchor_instruction(
        program_id,
        "deposit",
        &deposit_amount.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            writable_meta(owner_ata),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, deposit_ix, &owner, &[&owner]).unwrap();

    // Try to withdraw 0 - should fail
    svm.expire_blockhash();
    let withdraw_amount: u64 = 0;
    let withdraw_ix = anchor_instruction(
        program_id,
        "withdraw",
        &withdraw_amount.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            writable_meta(owner_ata),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, withdraw_ix, &owner, &[&owner]);
    assert!(result.is_err(), "Withdraw of 0 should fail");
}

#[test]
fn test_withdraw_insufficient_funds_fails() {
    let mut svm = load_token_vault_program();
    let program_id = token_vault_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let (owner, owner_ata) = setup_user_with_tokens(&mut svm, &mint, 1000);

    // Initialize and deposit
    let (vault_pda, _) = derive_vault_pda(&owner.pubkey(), &mint);
    let (vault_token_pda, _) = derive_vault_token_pda(&owner.pubkey(), &mint);

    let init_ix = anchor_instruction(
        program_id,
        "initialize_vault",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    svm.expire_blockhash();
    let deposit_amount: u64 = 100;
    let deposit_ix = anchor_instruction(
        program_id,
        "deposit",
        &deposit_amount.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            writable_meta(owner_ata),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, deposit_ix, &owner, &[&owner]).unwrap();

    // Try to withdraw more than deposited - should fail
    svm.expire_blockhash();
    let withdraw_amount: u64 = 200;
    let withdraw_ix = anchor_instruction(
        program_id,
        "withdraw",
        &withdraw_amount.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            writable_meta(owner_ata),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, withdraw_ix, &owner, &[&owner]);
    assert!(result.is_err(), "Withdraw more than balance should fail");
}

// =============================================================================
// AUTHORIZATION TESTS
// =============================================================================

#[test]
fn test_withdraw_unauthorized_fails() {
    let mut svm = load_token_vault_program();
    let program_id = token_vault_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let (owner, owner_ata) = setup_user_with_tokens(&mut svm, &mint, 1000);
    let attacker = funded_keypair_10_sol(&mut svm);
    let attacker_ata = get_ata(&attacker.pubkey(), &mint);

    // Create attacker's token account (empty)
    let attacker_token_account = create_token_account(&attacker.pubkey(), &mint, 0);
    svm.set_account(attacker_ata, attacker_token_account).unwrap();

    // Owner initializes vault and deposits
    let (vault_pda, _) = derive_vault_pda(&owner.pubkey(), &mint);
    let (vault_token_pda, _) = derive_vault_token_pda(&owner.pubkey(), &mint);

    let init_ix = anchor_instruction(
        program_id,
        "initialize_vault",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    svm.expire_blockhash();
    let deposit_amount: u64 = 500;
    let deposit_ix = anchor_instruction(
        program_id,
        "deposit",
        &deposit_amount.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            writable_meta(owner_ata),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, deposit_ix, &owner, &[&owner]).unwrap();

    // Attacker tries to withdraw - should fail
    svm.expire_blockhash();
    let withdraw_amount: u64 = 100;
    let withdraw_ix = anchor_instruction(
        program_id,
        "withdraw",
        &withdraw_amount.to_le_bytes(),
        vec![
            signer_meta(attacker.pubkey()),     // attacker pretends to be owner
            writable_meta(vault_pda),
            writable_meta(attacker_ata),        // attacker's token account
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, withdraw_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "Unauthorized withdraw should fail");

    // Verify vault balance unchanged
    let vault_token_account = svm.get_account(&vault_token_pda).unwrap();
    assert_eq!(read_token_balance(&vault_token_account.data), 500, "Vault should still have 500");
}

// =============================================================================
// FULL WORKFLOW TESTS
// =============================================================================

#[test]
fn test_full_deposit_withdraw_workflow() {
    let mut svm = load_token_vault_program();
    let program_id = token_vault_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let (owner, owner_ata) = setup_user_with_tokens(&mut svm, &mint, 10000);

    // Initialize vault
    let (vault_pda, _) = derive_vault_pda(&owner.pubkey(), &mint);
    let (vault_token_pda, _) = derive_vault_token_pda(&owner.pubkey(), &mint);

    let init_ix = anchor_instruction(
        program_id,
        "initialize_vault",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // Deposit 5000
    svm.expire_blockhash();
    let deposit_ix = anchor_instruction(
        program_id,
        "deposit",
        &5000u64.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            writable_meta(owner_ata),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, deposit_ix, &owner, &[&owner]).unwrap();

    // Withdraw 2000
    svm.expire_blockhash();
    let withdraw_ix = anchor_instruction(
        program_id,
        "withdraw",
        &2000u64.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            writable_meta(owner_ata),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, withdraw_ix, &owner, &[&owner]).unwrap();

    // Deposit 1000 more
    svm.expire_blockhash();
    let deposit2_ix = anchor_instruction(
        program_id,
        "deposit",
        &1000u64.to_le_bytes(),
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            writable_meta(owner_ata),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, deposit2_ix, &owner, &[&owner]).unwrap();

    // Final balances: owner should have 10000 - 5000 + 2000 - 1000 = 6000
    // Vault should have 5000 - 2000 + 1000 = 4000
    let owner_account = svm.get_account(&owner_ata).unwrap();
    assert_eq!(read_token_balance(&owner_account.data), 6000);

    let vault_token_account = svm.get_account(&vault_token_pda).unwrap();
    assert_eq!(read_token_balance(&vault_token_account.data), 4000);

    let vault_account = svm.get_account(&vault_pda).unwrap();
    assert_eq!(read_vault_total_deposits(&vault_account.data), 4000);
}

#[test]
fn test_multiple_depositors() {
    let mut svm = load_token_vault_program();
    let program_id = token_vault_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let owner = funded_keypair_10_sol(&mut svm);
    let (user1, user1_ata) = setup_user_with_tokens(&mut svm, &mint, 1000);
    let (user2, user2_ata) = setup_user_with_tokens(&mut svm, &mint, 1000);

    // Owner initializes vault
    let (vault_pda, _) = derive_vault_pda(&owner.pubkey(), &mint);
    let (vault_token_pda, _) = derive_vault_token_pda(&owner.pubkey(), &mint);

    let init_ix = anchor_instruction(
        program_id,
        "initialize_vault",
        &[],
        vec![
            signer_meta(owner.pubkey()),
            writable_meta(vault_pda),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &owner, &[&owner]).unwrap();

    // User1 deposits 300
    svm.expire_blockhash();
    let deposit1_ix = anchor_instruction(
        program_id,
        "deposit",
        &300u64.to_le_bytes(),
        vec![
            signer_meta(user1.pubkey()),
            writable_meta(vault_pda),
            writable_meta(user1_ata),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, deposit1_ix, &user1, &[&user1]).unwrap();

    // User2 deposits 500
    svm.expire_blockhash();
    let deposit2_ix = anchor_instruction(
        program_id,
        "deposit",
        &500u64.to_le_bytes(),
        vec![
            signer_meta(user2.pubkey()),
            writable_meta(vault_pda),
            writable_meta(user2_ata),
            writable_meta(vault_token_pda),
            writable_meta(mint),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, deposit2_ix, &user2, &[&user2]).unwrap();

    // Verify total deposits
    let vault_token_account = svm.get_account(&vault_token_pda).unwrap();
    assert_eq!(read_token_balance(&vault_token_account.data), 800, "Total vault balance should be 800");

    let vault_account = svm.get_account(&vault_pda).unwrap();
    assert_eq!(read_vault_total_deposits(&vault_account.data), 800, "Total deposits tracked should be 800");

    // Verify individual user balances
    let user1_account = svm.get_account(&user1_ata).unwrap();
    assert_eq!(read_token_balance(&user1_account.data), 700);

    let user2_account = svm.get_account(&user2_ata).unwrap();
    assert_eq!(read_token_balance(&user2_account.data), 500);
}
