//! LiteSVM integration tests for the Seahorse ATA Patterns program
//!
//! Note: This example uses PDA-derived token accounts rather than true ATAs
//! because Seahorse's `associated=True` parameter has a known bug.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile ata_patterns.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// ATA Patterns program ID (from declare_id!)
fn ata_patterns_program_id() -> Pubkey {
    Pubkey::from_str("ATAPtrns1111111111111111111111111111111111AA").unwrap()
}

/// UserAccount size: discriminator (8) + owner (32) + mint (32) + token_account (32) + bump (1) + is_registered (1)
const USER_ACCOUNT_SIZE: usize = 8 + 32 + 32 + 32 + 1 + 1;

/// Treasury size: discriminator (8) + admin (32) + mint (32) + treasury_token_account (32) + bump (1)
const TREASURY_SIZE: usize = 8 + 32 + 32 + 32 + 1;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the ATA patterns program into LiteSVM
fn load_ata_patterns_program() -> litesvm::LiteSVM {
    let program_id = ata_patterns_program_id();
    let program_bytes = std::fs::read("../../target/deploy/ata_patterns.so")
        .expect("Failed to read ata_patterns.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);
    svm
}

/// Setup a mint for testing
fn setup_mint(svm: &mut litesvm::LiteSVM) -> (Keypair, Pubkey) {
    let mint_authority = funded_keypair_10_sol(svm);
    let mint = Keypair::new();

    // Create the mint account directly in SVM
    let mint_account = create_mint_account(&mint_authority.pubkey(), 6);
    svm.set_account(mint.pubkey(), mint_account).unwrap();

    (mint_authority, mint.pubkey())
}

/// Derive user token account PDA
fn derive_user_token_pda(user: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"user_token", user.as_ref(), mint.as_ref()],
        &ata_patterns_program_id(),
    )
}

/// Derive user account PDA
fn derive_user_account_pda(user: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"user_account", user.as_ref(), mint.as_ref()],
        &ata_patterns_program_id(),
    )
}

/// Derive treasury PDA
fn derive_treasury_pda(admin: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"treasury", admin.as_ref(), mint.as_ref()],
        &ata_patterns_program_id(),
    )
}

/// Derive treasury token account PDA
fn derive_treasury_token_pda(admin: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"treasury_token", admin.as_ref(), mint.as_ref()],
        &ata_patterns_program_id(),
    )
}

/// Read user account owner from account data
fn read_user_account_owner(data: &[u8]) -> Pubkey {
    // Skip discriminator (8), read owner (32)
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

/// Read user account is_registered from account data
fn read_user_account_is_registered(data: &[u8]) -> bool {
    // Skip discriminator (8) + owner (32) + mint (32) + token_account (32) + bump (1)
    data[105] != 0
}

/// Read user account token_account from account data
fn read_user_account_token_account(data: &[u8]) -> Pubkey {
    // Skip discriminator (8) + owner (32) + mint (32), read token_account (32)
    Pubkey::new_from_array(data[72..104].try_into().unwrap())
}

/// Read treasury admin from account data
fn read_treasury_admin(data: &[u8]) -> Pubkey {
    // Skip discriminator (8), read admin (32)
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

/// Read treasury token account address from account data
fn read_treasury_token_account_address(data: &[u8]) -> Pubkey {
    // Skip discriminator (8) + admin (32) + mint (32), read treasury_token_account (32)
    Pubkey::new_from_array(data[72..104].try_into().unwrap())
}

// =============================================================================
// CREATE USER TOKEN ACCOUNT TESTS
// =============================================================================

#[test]
fn test_create_user_token_account() {
    let mut svm = load_ata_patterns_program();
    let program_id = ata_patterns_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let user = funded_keypair_10_sol(&mut svm);
    let (user_token_pda, _) = derive_user_token_pda(&user.pubkey(), &mint);

    // Create user token account instruction
    let ix = anchor_instruction(
        program_id,
        "create_user_token_account",
        &[],
        vec![
            signer_meta(user.pubkey()),          // user
            readonly_meta(mint),                  // mint
            writable_meta(user_token_pda),        // user_token_account
            readonly_meta(token_program_id()),    // token_program
            readonly_meta(system_program::id()),  // system_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &user, &[&user]);
    assert!(result.is_ok(), "Create user token account should succeed: {:?}", result);

    // Verify token account was created
    let token_account = svm.get_account(&user_token_pda).expect("Token account should exist");
    assert_eq!(token_account.owner, token_program_id(), "Token account should be owned by token program");

    // Verify token account has correct balance
    let balance = read_token_balance(&token_account.data);
    assert_eq!(balance, 0, "Initial balance should be 0");
}

#[test]
fn test_create_user_token_account_multiple_users() {
    let mut svm = load_ata_patterns_program();
    let program_id = ata_patterns_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let user1 = funded_keypair_10_sol(&mut svm);
    let user2 = funded_keypair_10_sol(&mut svm);
    let (user1_token_pda, _) = derive_user_token_pda(&user1.pubkey(), &mint);
    let (user2_token_pda, _) = derive_user_token_pda(&user2.pubkey(), &mint);

    // PDAs should have different addresses
    assert_ne!(user1_token_pda, user2_token_pda, "Different users should have different token PDAs");

    // Create user1's token account
    let ix1 = anchor_instruction(
        program_id,
        "create_user_token_account",
        &[],
        vec![
            signer_meta(user1.pubkey()),
            readonly_meta(mint),
            writable_meta(user1_token_pda),
            readonly_meta(token_program_id()),
            readonly_meta(system_program::id()),
        ],
    );
    let result1 = execute_tx(&mut svm, ix1, &user1, &[&user1]);
    assert!(result1.is_ok(), "User1 token account creation should succeed");

    // Create user2's token account
    svm.expire_blockhash();
    let ix2 = anchor_instruction(
        program_id,
        "create_user_token_account",
        &[],
        vec![
            signer_meta(user2.pubkey()),
            readonly_meta(mint),
            writable_meta(user2_token_pda),
            readonly_meta(token_program_id()),
            readonly_meta(system_program::id()),
        ],
    );
    let result2 = execute_tx(&mut svm, ix2, &user2, &[&user2]);
    assert!(result2.is_ok(), "User2 token account creation should succeed");

    // Verify both token accounts exist
    assert!(svm.get_account(&user1_token_pda).is_some(), "User1 token account should exist");
    assert!(svm.get_account(&user2_token_pda).is_some(), "User2 token account should exist");
}

// =============================================================================
// TRANSFER TOKENS TESTS
// =============================================================================

#[test]
fn test_transfer_tokens() {
    let mut svm = load_ata_patterns_program();
    let program_id = ata_patterns_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let sender = funded_keypair_10_sol(&mut svm);
    let recipient = funded_keypair_10_sol(&mut svm);
    let (sender_token_pda, _) = derive_user_token_pda(&sender.pubkey(), &mint);
    let (recipient_token_pda, _) = derive_user_token_pda(&recipient.pubkey(), &mint);

    // Create sender's token account with initial balance
    let sender_token_account = create_token_account(&sender.pubkey(), &mint, 1000);
    svm.set_account(sender_token_pda, sender_token_account).unwrap();

    // Create recipient's token account
    let recipient_token_account = create_token_account(&recipient.pubkey(), &mint, 0);
    svm.set_account(recipient_token_pda, recipient_token_account).unwrap();

    // Transfer 500 tokens
    let amount: u64 = 500;
    let ix = anchor_instruction(
        program_id,
        "transfer_tokens",
        &amount.to_le_bytes(),
        vec![
            signer_meta(sender.pubkey()),           // sender
            readonly_meta(mint),                    // mint
            writable_meta(sender_token_pda),        // sender_token_account
            writable_meta(recipient_token_pda),     // recipient_token_account
            readonly_meta(token_program_id()),      // token_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &sender, &[&sender]);
    assert!(result.is_ok(), "Transfer tokens should succeed: {:?}", result);

    // Verify balances
    let sender_account = svm.get_account(&sender_token_pda).unwrap();
    assert_eq!(read_token_balance(&sender_account.data), 500, "Sender should have 500 tokens left");

    let recipient_account = svm.get_account(&recipient_token_pda).unwrap();
    assert_eq!(read_token_balance(&recipient_account.data), 500, "Recipient should have 500 tokens");
}

#[test]
fn test_transfer_tokens_zero_fails() {
    let mut svm = load_ata_patterns_program();
    let program_id = ata_patterns_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let sender = funded_keypair_10_sol(&mut svm);
    let recipient = funded_keypair_10_sol(&mut svm);
    let (sender_token_pda, _) = derive_user_token_pda(&sender.pubkey(), &mint);
    let (recipient_token_pda, _) = derive_user_token_pda(&recipient.pubkey(), &mint);

    // Create sender's token account with initial balance
    let sender_token_account = create_token_account(&sender.pubkey(), &mint, 1000);
    svm.set_account(sender_token_pda, sender_token_account).unwrap();

    // Create recipient's token account
    let recipient_token_account = create_token_account(&recipient.pubkey(), &mint, 0);
    svm.set_account(recipient_token_pda, recipient_token_account).unwrap();

    // Try to transfer 0 tokens - should fail
    let amount: u64 = 0;
    let ix = anchor_instruction(
        program_id,
        "transfer_tokens",
        &amount.to_le_bytes(),
        vec![
            signer_meta(sender.pubkey()),
            readonly_meta(mint),
            writable_meta(sender_token_pda),
            writable_meta(recipient_token_pda),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &sender, &[&sender]);
    assert!(result.is_err(), "Transfer of 0 tokens should fail");
}

// =============================================================================
// REGISTER USER TESTS
// =============================================================================

#[test]
fn test_register_user() {
    let mut svm = load_ata_patterns_program();
    let program_id = ata_patterns_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let user = funded_keypair_10_sol(&mut svm);
    let (user_account_pda, _user_account_bump) = derive_user_account_pda(&user.pubkey(), &mint);
    let (user_token_pda, _) = derive_user_token_pda(&user.pubkey(), &mint);

    // Register user
    let ix = anchor_instruction(
        program_id,
        "register_user",
        &[],
        vec![
            signer_meta(user.pubkey()),           // user
            readonly_meta(mint),                   // mint
            writable_meta(user_account_pda),       // user_account
            writable_meta(user_token_pda),         // user_token_account
            readonly_meta(token_program_id()),     // token_program
            readonly_meta(system_program::id()),   // system_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &user, &[&user]);
    assert!(result.is_ok(), "Register user should succeed: {:?}", result);

    // Verify user account
    let user_account = svm.get_account(&user_account_pda).expect("User account should exist");
    assert_eq!(user_account.owner, program_id, "User account should be owned by program");

    let owner = read_user_account_owner(&user_account.data);
    assert_eq!(owner, user.pubkey(), "Owner should match user");

    let is_registered = read_user_account_is_registered(&user_account.data);
    assert!(is_registered, "User should be registered");

    let stored_token_account = read_user_account_token_account(&user_account.data);
    assert_eq!(stored_token_account, user_token_pda, "Stored token account should match");

    // Verify token account was created
    let token_account = svm.get_account(&user_token_pda).expect("User token account should exist");
    assert_eq!(token_account.owner, token_program_id(), "Token account should be owned by token program");
}

#[test]
fn test_register_multiple_users() {
    let mut svm = load_ata_patterns_program();
    let program_id = ata_patterns_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let user1 = funded_keypair_10_sol(&mut svm);
    let user2 = funded_keypair_10_sol(&mut svm);

    let (user1_account_pda, _) = derive_user_account_pda(&user1.pubkey(), &mint);
    let (user2_account_pda, _) = derive_user_account_pda(&user2.pubkey(), &mint);
    let (user1_token_pda, _) = derive_user_token_pda(&user1.pubkey(), &mint);
    let (user2_token_pda, _) = derive_user_token_pda(&user2.pubkey(), &mint);

    // Register user1
    let ix1 = anchor_instruction(
        program_id,
        "register_user",
        &[],
        vec![
            signer_meta(user1.pubkey()),
            readonly_meta(mint),
            writable_meta(user1_account_pda),
            writable_meta(user1_token_pda),
            readonly_meta(token_program_id()),
            readonly_meta(system_program::id()),
        ],
    );
    let result1 = execute_tx(&mut svm, ix1, &user1, &[&user1]);
    assert!(result1.is_ok(), "Register user1 should succeed");

    // Register user2
    svm.expire_blockhash();
    let ix2 = anchor_instruction(
        program_id,
        "register_user",
        &[],
        vec![
            signer_meta(user2.pubkey()),
            readonly_meta(mint),
            writable_meta(user2_account_pda),
            writable_meta(user2_token_pda),
            readonly_meta(token_program_id()),
            readonly_meta(system_program::id()),
        ],
    );
    let result2 = execute_tx(&mut svm, ix2, &user2, &[&user2]);
    assert!(result2.is_ok(), "Register user2 should succeed");

    // Verify both users are registered independently
    let user1_account = svm.get_account(&user1_account_pda).unwrap();
    let user2_account = svm.get_account(&user2_account_pda).unwrap();

    assert_eq!(read_user_account_owner(&user1_account.data), user1.pubkey());
    assert_eq!(read_user_account_owner(&user2_account.data), user2.pubkey());

    assert!(read_user_account_is_registered(&user1_account.data));
    assert!(read_user_account_is_registered(&user2_account.data));
}

// =============================================================================
// INITIALIZE TREASURY TESTS
// =============================================================================

#[test]
fn test_initialize_treasury() {
    let mut svm = load_ata_patterns_program();
    let program_id = ata_patterns_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let admin = funded_keypair_10_sol(&mut svm);
    let (treasury_pda, _treasury_bump) = derive_treasury_pda(&admin.pubkey(), &mint);
    let (treasury_token_pda, _) = derive_treasury_token_pda(&admin.pubkey(), &mint);

    // Initialize treasury
    let ix = anchor_instruction(
        program_id,
        "initialize_treasury",
        &[],
        vec![
            signer_meta(admin.pubkey()),           // admin
            readonly_meta(mint),                    // mint
            writable_meta(treasury_pda),            // treasury
            writable_meta(treasury_token_pda),      // treasury_token_account
            readonly_meta(token_program_id()),      // token_program
            readonly_meta(system_program::id()),    // system_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &admin, &[&admin]);
    assert!(result.is_ok(), "Initialize treasury should succeed: {:?}", result);

    // Verify treasury account
    let treasury_account = svm.get_account(&treasury_pda).expect("Treasury should exist");
    assert_eq!(treasury_account.owner, program_id, "Treasury should be owned by program");

    let treasury_admin = read_treasury_admin(&treasury_account.data);
    assert_eq!(treasury_admin, admin.pubkey(), "Admin should match");

    let stored_treasury_token = read_treasury_token_account_address(&treasury_account.data);
    assert_eq!(stored_treasury_token, treasury_token_pda, "Treasury token account address should match");

    // Verify treasury token account was created (owned by treasury PDA)
    let token_account = svm.get_account(&treasury_token_pda).expect("Treasury token account should exist");
    assert_eq!(token_account.owner, token_program_id(), "Token account should be owned by token program");

    let balance = read_token_balance(&token_account.data);
    assert_eq!(balance, 0, "Initial treasury balance should be 0");
}

#[test]
fn test_initialize_treasury_multiple_admins() {
    let mut svm = load_ata_patterns_program();
    let program_id = ata_patterns_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let admin1 = funded_keypair_10_sol(&mut svm);
    let admin2 = funded_keypair_10_sol(&mut svm);

    let (treasury1_pda, _) = derive_treasury_pda(&admin1.pubkey(), &mint);
    let (treasury2_pda, _) = derive_treasury_pda(&admin2.pubkey(), &mint);
    let (treasury1_token_pda, _) = derive_treasury_token_pda(&admin1.pubkey(), &mint);
    let (treasury2_token_pda, _) = derive_treasury_token_pda(&admin2.pubkey(), &mint);

    // PDAs should be different
    assert_ne!(treasury1_pda, treasury2_pda, "Different admins should have different treasury PDAs");

    // Initialize admin1's treasury
    let ix1 = anchor_instruction(
        program_id,
        "initialize_treasury",
        &[],
        vec![
            signer_meta(admin1.pubkey()),
            readonly_meta(mint),
            writable_meta(treasury1_pda),
            writable_meta(treasury1_token_pda),
            readonly_meta(token_program_id()),
            readonly_meta(system_program::id()),
        ],
    );
    let result1 = execute_tx(&mut svm, ix1, &admin1, &[&admin1]);
    assert!(result1.is_ok(), "Admin1 treasury init should succeed");

    // Initialize admin2's treasury
    svm.expire_blockhash();
    let ix2 = anchor_instruction(
        program_id,
        "initialize_treasury",
        &[],
        vec![
            signer_meta(admin2.pubkey()),
            readonly_meta(mint),
            writable_meta(treasury2_pda),
            writable_meta(treasury2_token_pda),
            readonly_meta(token_program_id()),
            readonly_meta(system_program::id()),
        ],
    );
    let result2 = execute_tx(&mut svm, ix2, &admin2, &[&admin2]);
    assert!(result2.is_ok(), "Admin2 treasury init should succeed");

    // Verify both treasuries exist independently
    let treasury1 = svm.get_account(&treasury1_pda).unwrap();
    let treasury2 = svm.get_account(&treasury2_pda).unwrap();

    assert_eq!(read_treasury_admin(&treasury1.data), admin1.pubkey());
    assert_eq!(read_treasury_admin(&treasury2.data), admin2.pubkey());
}

// =============================================================================
// AIRDROP TO USER TESTS
// =============================================================================

#[test]
fn test_airdrop_to_user() {
    let mut svm = load_ata_patterns_program();
    let program_id = ata_patterns_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let admin = funded_keypair_10_sol(&mut svm);
    let user = funded_keypair_10_sol(&mut svm);

    let (treasury_pda, _) = derive_treasury_pda(&admin.pubkey(), &mint);
    let (treasury_token_pda, _) = derive_treasury_token_pda(&admin.pubkey(), &mint);
    let (user_account_pda, _) = derive_user_account_pda(&user.pubkey(), &mint);
    let (user_token_pda, _) = derive_user_token_pda(&user.pubkey(), &mint);

    // Initialize treasury
    let init_treasury_ix = anchor_instruction(
        program_id,
        "initialize_treasury",
        &[],
        vec![
            signer_meta(admin.pubkey()),
            readonly_meta(mint),
            writable_meta(treasury_pda),
            writable_meta(treasury_token_pda),
            readonly_meta(token_program_id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_treasury_ix, &admin, &[&admin]).unwrap();

    // Fund the treasury with tokens
    let treasury_token_account = create_token_account(&treasury_pda, &mint, 10000);
    svm.set_account(treasury_token_pda, treasury_token_account).unwrap();

    // Register user
    svm.expire_blockhash();
    let register_ix = anchor_instruction(
        program_id,
        "register_user",
        &[],
        vec![
            signer_meta(user.pubkey()),
            readonly_meta(mint),
            writable_meta(user_account_pda),
            writable_meta(user_token_pda),
            readonly_meta(token_program_id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, register_ix, &user, &[&user]).unwrap();

    // Airdrop 500 tokens to user
    svm.expire_blockhash();
    let amount: u64 = 500;
    let airdrop_ix = anchor_instruction(
        program_id,
        "airdrop_to_user",
        &amount.to_le_bytes(),
        vec![
            signer_meta(admin.pubkey()),       // admin
            readonly_meta(mint),                // mint
            readonly_meta(treasury_pda),        // treasury
            writable_meta(treasury_token_pda),  // treasury_token_account
            readonly_meta(user_account_pda),    // user_account
            writable_meta(user_token_pda),      // user_token_account
            readonly_meta(token_program_id()),  // token_program
        ],
    );

    let result = execute_tx(&mut svm, airdrop_ix, &admin, &[&admin]);
    assert!(result.is_ok(), "Airdrop should succeed: {:?}", result);

    // Verify balances
    let treasury_account = svm.get_account(&treasury_token_pda).unwrap();
    assert_eq!(read_token_balance(&treasury_account.data), 9500, "Treasury should have 9500 tokens left");

    let user_account = svm.get_account(&user_token_pda).unwrap();
    assert_eq!(read_token_balance(&user_account.data), 500, "User should have 500 tokens");
}

#[test]
fn test_airdrop_to_unregistered_user_fails() {
    let mut svm = load_ata_patterns_program();
    let program_id = ata_patterns_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let admin = funded_keypair_10_sol(&mut svm);
    let user = funded_keypair_10_sol(&mut svm);

    let (treasury_pda, _) = derive_treasury_pda(&admin.pubkey(), &mint);
    let (treasury_token_pda, _) = derive_treasury_token_pda(&admin.pubkey(), &mint);
    let (user_account_pda, _) = derive_user_account_pda(&user.pubkey(), &mint);
    let (user_token_pda, _) = derive_user_token_pda(&user.pubkey(), &mint);

    // Initialize treasury
    let init_treasury_ix = anchor_instruction(
        program_id,
        "initialize_treasury",
        &[],
        vec![
            signer_meta(admin.pubkey()),
            readonly_meta(mint),
            writable_meta(treasury_pda),
            writable_meta(treasury_token_pda),
            readonly_meta(token_program_id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_treasury_ix, &admin, &[&admin]).unwrap();

    // Fund the treasury
    let treasury_token_account = create_token_account(&treasury_pda, &mint, 10000);
    svm.set_account(treasury_token_pda, treasury_token_account).unwrap();

    // Create user's token account but don't register (no user_account PDA)
    let user_token_account = create_token_account(&user.pubkey(), &mint, 0);
    svm.set_account(user_token_pda, user_token_account).unwrap();

    // Try to airdrop without user being registered - should fail
    // (The user_account_pda doesn't exist)
    svm.expire_blockhash();
    let amount: u64 = 500;
    let airdrop_ix = anchor_instruction(
        program_id,
        "airdrop_to_user",
        &amount.to_le_bytes(),
        vec![
            signer_meta(admin.pubkey()),
            readonly_meta(mint),
            readonly_meta(treasury_pda),
            writable_meta(treasury_token_pda),
            readonly_meta(user_account_pda), // This account doesn't exist
            writable_meta(user_token_pda),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, airdrop_ix, &admin, &[&admin]);
    assert!(result.is_err(), "Airdrop to unregistered user should fail");
}

#[test]
fn test_airdrop_zero_fails() {
    let mut svm = load_ata_patterns_program();
    let program_id = ata_patterns_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let admin = funded_keypair_10_sol(&mut svm);
    let user = funded_keypair_10_sol(&mut svm);

    let (treasury_pda, _) = derive_treasury_pda(&admin.pubkey(), &mint);
    let (treasury_token_pda, _) = derive_treasury_token_pda(&admin.pubkey(), &mint);
    let (user_account_pda, _) = derive_user_account_pda(&user.pubkey(), &mint);
    let (user_token_pda, _) = derive_user_token_pda(&user.pubkey(), &mint);

    // Initialize treasury
    let init_treasury_ix = anchor_instruction(
        program_id,
        "initialize_treasury",
        &[],
        vec![
            signer_meta(admin.pubkey()),
            readonly_meta(mint),
            writable_meta(treasury_pda),
            writable_meta(treasury_token_pda),
            readonly_meta(token_program_id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_treasury_ix, &admin, &[&admin]).unwrap();

    // Fund the treasury
    let treasury_token_account = create_token_account(&treasury_pda, &mint, 10000);
    svm.set_account(treasury_token_pda, treasury_token_account).unwrap();

    // Register user
    svm.expire_blockhash();
    let register_ix = anchor_instruction(
        program_id,
        "register_user",
        &[],
        vec![
            signer_meta(user.pubkey()),
            readonly_meta(mint),
            writable_meta(user_account_pda),
            writable_meta(user_token_pda),
            readonly_meta(token_program_id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, register_ix, &user, &[&user]).unwrap();

    // Try to airdrop 0 tokens - should fail
    svm.expire_blockhash();
    let amount: u64 = 0;
    let airdrop_ix = anchor_instruction(
        program_id,
        "airdrop_to_user",
        &amount.to_le_bytes(),
        vec![
            signer_meta(admin.pubkey()),
            readonly_meta(mint),
            readonly_meta(treasury_pda),
            writable_meta(treasury_token_pda),
            readonly_meta(user_account_pda),
            writable_meta(user_token_pda),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, airdrop_ix, &admin, &[&admin]);
    assert!(result.is_err(), "Airdrop of 0 tokens should fail");
}

// =============================================================================
// FULL WORKFLOW TESTS
// =============================================================================

#[test]
fn test_full_token_workflow() {
    let mut svm = load_ata_patterns_program();
    let program_id = ata_patterns_program_id();

    let (_, mint) = setup_mint(&mut svm);
    let admin = funded_keypair_10_sol(&mut svm);
    let user1 = funded_keypair_10_sol(&mut svm);
    let user2 = funded_keypair_10_sol(&mut svm);

    let (treasury_pda, _) = derive_treasury_pda(&admin.pubkey(), &mint);
    let (treasury_token_pda, _) = derive_treasury_token_pda(&admin.pubkey(), &mint);
    let (user1_account_pda, _) = derive_user_account_pda(&user1.pubkey(), &mint);
    let (user1_token_pda, _) = derive_user_token_pda(&user1.pubkey(), &mint);
    let (user2_token_pda, _) = derive_user_token_pda(&user2.pubkey(), &mint);

    // 1. Initialize treasury
    let init_treasury_ix = anchor_instruction(
        program_id,
        "initialize_treasury",
        &[],
        vec![
            signer_meta(admin.pubkey()),
            readonly_meta(mint),
            writable_meta(treasury_pda),
            writable_meta(treasury_token_pda),
            readonly_meta(token_program_id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, init_treasury_ix, &admin, &[&admin]).unwrap();

    // Fund the treasury
    let treasury_token_account = create_token_account(&treasury_pda, &mint, 10000);
    svm.set_account(treasury_token_pda, treasury_token_account).unwrap();

    // 2. Register user1
    svm.expire_blockhash();
    let register_ix = anchor_instruction(
        program_id,
        "register_user",
        &[],
        vec![
            signer_meta(user1.pubkey()),
            readonly_meta(mint),
            writable_meta(user1_account_pda),
            writable_meta(user1_token_pda),
            readonly_meta(token_program_id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, register_ix, &user1, &[&user1]).unwrap();

    // 3. Airdrop 1000 tokens to user1
    svm.expire_blockhash();
    let airdrop_ix = anchor_instruction(
        program_id,
        "airdrop_to_user",
        &1000u64.to_le_bytes(),
        vec![
            signer_meta(admin.pubkey()),
            readonly_meta(mint),
            readonly_meta(treasury_pda),
            writable_meta(treasury_token_pda),
            readonly_meta(user1_account_pda),
            writable_meta(user1_token_pda),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, airdrop_ix, &admin, &[&admin]).unwrap();

    // 4. User2 creates their token account
    svm.expire_blockhash();
    let create_user2_ix = anchor_instruction(
        program_id,
        "create_user_token_account",
        &[],
        vec![
            signer_meta(user2.pubkey()),
            readonly_meta(mint),
            writable_meta(user2_token_pda),
            readonly_meta(token_program_id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_user2_ix, &user2, &[&user2]).unwrap();

    // 5. User1 transfers 500 tokens to user2
    svm.expire_blockhash();
    let transfer_ix = anchor_instruction(
        program_id,
        "transfer_tokens",
        &500u64.to_le_bytes(),
        vec![
            signer_meta(user1.pubkey()),
            readonly_meta(mint),
            writable_meta(user1_token_pda),
            writable_meta(user2_token_pda),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, transfer_ix, &user1, &[&user1]).unwrap();

    // Verify final balances
    // Treasury: 10000 - 1000 = 9000
    // User1: 1000 - 500 = 500
    // User2: 500
    let treasury_account = svm.get_account(&treasury_token_pda).unwrap();
    assert_eq!(read_token_balance(&treasury_account.data), 9000, "Treasury should have 9000");

    let user1_account = svm.get_account(&user1_token_pda).unwrap();
    assert_eq!(read_token_balance(&user1_account.data), 500, "User1 should have 500");

    let user2_account = svm.get_account(&user2_token_pda).unwrap();
    assert_eq!(read_token_balance(&user2_account.data), 500, "User2 should have 500");
}

#[test]
fn test_pda_address_derivation() {
    let mut svm = load_ata_patterns_program();

    let (_, mint) = setup_mint(&mut svm);
    let user = funded_keypair_10_sol(&mut svm);

    // Derive PDA token account using our helper
    let (expected_token_pda, _) = derive_user_token_pda(&user.pubkey(), &mint);

    // PDA should be deterministic
    let (expected_token_pda_2, _) = derive_user_token_pda(&user.pubkey(), &mint);
    assert_eq!(expected_token_pda, expected_token_pda_2, "PDA derivation should be deterministic");

    // Different users should have different PDAs
    let other_user = funded_keypair_10_sol(&mut svm);
    let (other_token_pda, _) = derive_user_token_pda(&other_user.pubkey(), &mint);
    assert_ne!(expected_token_pda, other_token_pda, "Different users should have different PDAs");

    // Different mints should have different PDAs
    let (_, other_mint) = setup_mint(&mut svm);
    let (other_mint_pda, _) = derive_user_token_pda(&user.pubkey(), &other_mint);
    assert_ne!(expected_token_pda, other_mint_pda, "Different mints should have different PDAs");
}
