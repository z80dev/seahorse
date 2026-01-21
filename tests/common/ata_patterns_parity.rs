//! Behavior parity tests: Seahorse ATA Patterns vs Anchor ATA Patterns
//!
//! This test verifies that the Seahorse-compiled ata_patterns program produces
//! comparable behavior to the reference Anchor implementation.
//!
//! Note: This example uses PDA-derived token accounts rather than true ATAs
//! because Seahorse's `associated=True` parameter has a known bug.
//!
//! Key focus areas:
//! - PDA token account creation at correct addresses
//! - Token transfers between PDA accounts
//! - Combined state account + PDA token account creation
//! - PDA-owned token accounts for treasury patterns
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile both:
//! - target/deploy/ata_patterns.so (Seahorse)
//! - target/deploy/ata_patterns_anchor.so (Anchor reference)

use seahorse_test_common::*;
use solana_account::Account;
use solana_keypair::Keypair;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use spl_token::solana_program::program_pack::Pack;
use spl_token::solana_program::program_option::COption;
use std::str::FromStr;

/// SPL Token Mint account size
const MINT_SIZE: usize = 82;
/// SPL Token Account size
const TOKEN_ACCOUNT_SIZE: usize = 165;

/// Both programs use the same program ID for testing
fn program_id() -> Pubkey {
    Pubkey::from_str("ATAPtrns1111111111111111111111111111111111AA").unwrap()
}

/// SPL Token program ID
fn token_program_id() -> Pubkey {
    spl_token::id()
}

/// Create a mint account for testing
fn create_mint_account(authority: &Pubkey, decimals: u8) -> Account {
    let rent = Rent::default();

    let mint = spl_token::state::Mint {
        mint_authority: COption::Some(*authority),
        supply: 0,
        decimals,
        is_initialized: true,
        freeze_authority: COption::None,
    };

    let mut data = vec![0u8; MINT_SIZE];
    spl_token::state::Mint::pack(mint, &mut data).unwrap();

    Account {
        lamports: rent.minimum_balance(MINT_SIZE),
        data,
        owner: spl_token::id(),
        executable: false,
        rent_epoch: 0,
    }
}

/// Create a token account for testing
fn create_token_account(owner: &Pubkey, mint: &Pubkey, amount: u64) -> Account {
    let rent = Rent::default();

    let token_account = spl_token::state::Account {
        mint: *mint,
        owner: *owner,
        amount,
        delegate: COption::None,
        state: spl_token::state::AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };

    let mut data = vec![0u8; TOKEN_ACCOUNT_SIZE];
    spl_token::state::Account::pack(token_account, &mut data).unwrap();

    Account {
        lamports: rent.minimum_balance(TOKEN_ACCOUNT_SIZE),
        data,
        owner: spl_token::id(),
        executable: false,
        rent_epoch: 0,
    }
}

/// Derive user token account PDA
fn derive_user_token_pda(user: &Pubkey, mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"user_token", user.as_ref(), mint.as_ref()],
        program_id,
    )
}

/// Derive user account PDA
fn derive_user_account_pda(user: &Pubkey, mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"user_account", user.as_ref(), mint.as_ref()],
        program_id,
    )
}

/// Derive treasury PDA
fn derive_treasury_pda(admin: &Pubkey, mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"treasury", admin.as_ref(), mint.as_ref()],
        program_id,
    )
}

/// Derive treasury token account PDA
fn derive_treasury_token_pda(admin: &Pubkey, mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"treasury_token", admin.as_ref(), mint.as_ref()],
        program_id,
    )
}

/// Read token account balance from raw account data
fn read_token_balance(data: &[u8]) -> u64 {
    let account = spl_token::state::Account::unpack(data).unwrap();
    account.amount
}

/// Read UserAccount is_registered from account data
fn read_user_account_is_registered(data: &[u8]) -> bool {
    // Skip discriminator (8) + owner (32) + mint (32) + token_account (32) + bump (1)
    data[105] != 0
}

/// Read Treasury admin from account data
fn read_treasury_admin(data: &[u8]) -> Pubkey {
    // Skip discriminator (8), read admin (32)
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

#[test]
fn test_create_user_token_account_address_parity() {
    let program_id = program_id();

    // Load Seahorse version
    let seahorse_bytes = std::fs::read("../../target/deploy/ata_patterns.so")
        .expect("Failed to read ata_patterns.so - run ./scripts/build-test-programs.sh first");

    // Load Anchor reference version
    let anchor_bytes = std::fs::read("../../target/deploy/ata_patterns_anchor.so")
        .expect("Failed to read ata_patterns_anchor.so - run ./scripts/build-test-programs.sh first");

    // Create SVMs for each program
    let mut seahorse_svm = litesvm::LiteSVM::new();
    seahorse_svm.add_program(program_id, &seahorse_bytes);
    let seahorse_user = {
        let kp = Keypair::new();
        seahorse_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let mut anchor_svm = litesvm::LiteSVM::new();
    anchor_svm.add_program(program_id, &anchor_bytes);
    let anchor_user = {
        let kp = Keypair::new();
        anchor_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    // Setup mint for both
    let mint = Keypair::new();
    let mint_account = create_mint_account(&Keypair::new().pubkey(), 6);
    seahorse_svm.set_account(mint.pubkey(), mint_account.clone()).unwrap();
    anchor_svm.set_account(mint.pubkey(), mint_account).unwrap();

    // Derive expected PDA addresses
    let (seahorse_token_pda, _) = derive_user_token_pda(&seahorse_user.pubkey(), &mint.pubkey(), &program_id);
    let (anchor_token_pda, _) = derive_user_token_pda(&anchor_user.pubkey(), &mint.pubkey(), &program_id);

    // Create token account using Seahorse
    let seahorse_ix = InstructionBuilder::new("create_user_token_account")
        .with_signer(&seahorse_user.pubkey())
        .with_readonly(&mint.pubkey())
        .with_writable(&seahorse_token_pda)
        .with_readonly(&token_program_id())
        .with_readonly(&system_program::id())
        .build(program_id);

    let blockhash = seahorse_svm.latest_blockhash();
    let message = solana_message::Message::new(&[seahorse_ix], Some(&seahorse_user.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seahorse_user], message, blockhash);
    let seahorse_result = seahorse_svm.send_transaction(tx);

    // Create token account using Anchor
    let anchor_ix = InstructionBuilder::new("create_user_token_account")
        .with_signer(&anchor_user.pubkey())
        .with_readonly(&mint.pubkey())
        .with_writable(&anchor_token_pda)
        .with_readonly(&token_program_id())
        .with_readonly(&system_program::id())
        .build(program_id);

    let blockhash = anchor_svm.latest_blockhash();
    let message = solana_message::Message::new(&[anchor_ix], Some(&anchor_user.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&anchor_user], message, blockhash);
    let anchor_result = anchor_svm.send_transaction(tx);

    // Both should succeed
    assert!(seahorse_result.is_ok(), "Seahorse create_user_token_account failed: {:?}", seahorse_result);
    assert!(anchor_result.is_ok(), "Anchor create_user_token_account failed: {:?}", anchor_result);

    // Verify token accounts exist
    let seahorse_token_account = seahorse_svm.get_account(&seahorse_token_pda);
    let anchor_token_account = anchor_svm.get_account(&anchor_token_pda);

    assert!(seahorse_token_account.is_some(), "Seahorse token account should exist");
    assert!(anchor_token_account.is_some(), "Anchor token account should exist");

    // Both should be owned by token program
    assert_eq!(seahorse_token_account.as_ref().unwrap().owner, token_program_id());
    assert_eq!(anchor_token_account.as_ref().unwrap().owner, token_program_id());

    // Both should have 0 balance initially
    assert_eq!(read_token_balance(&seahorse_token_account.unwrap().data), 0);
    assert_eq!(read_token_balance(&anchor_token_account.unwrap().data), 0);

    // PDA derivation formula should be consistent
    let (seahorse_token_pda_2, _) = derive_user_token_pda(&seahorse_user.pubkey(), &mint.pubkey(), &program_id);
    assert_eq!(seahorse_token_pda, seahorse_token_pda_2, "PDA derivation should be deterministic");
}

#[test]
fn test_transfer_tokens_balance_parity() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/ata_patterns.so").unwrap();
    let anchor_bytes = std::fs::read("../../target/deploy/ata_patterns_anchor.so").unwrap();

    // Create SVMs
    let mut seahorse_svm = litesvm::LiteSVM::new();
    seahorse_svm.add_program(program_id, &seahorse_bytes);

    let mut anchor_svm = litesvm::LiteSVM::new();
    anchor_svm.add_program(program_id, &anchor_bytes);

    // Setup mint (same for both)
    let mint = Keypair::new();
    let mint_account = create_mint_account(&Keypair::new().pubkey(), 6);
    seahorse_svm.set_account(mint.pubkey(), mint_account.clone()).unwrap();
    anchor_svm.set_account(mint.pubkey(), mint_account).unwrap();

    // Setup senders with tokens
    let seahorse_sender = {
        let kp = Keypair::new();
        seahorse_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let anchor_sender = {
        let kp = Keypair::new();
        anchor_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let (seahorse_sender_token, _) = derive_user_token_pda(&seahorse_sender.pubkey(), &mint.pubkey(), &program_id);
    let (anchor_sender_token, _) = derive_user_token_pda(&anchor_sender.pubkey(), &mint.pubkey(), &program_id);

    // Give senders 1000 tokens
    let sender_token_account = create_token_account(&seahorse_sender.pubkey(), &mint.pubkey(), 1000);
    seahorse_svm.set_account(seahorse_sender_token, sender_token_account.clone()).unwrap();

    let sender_token_account = create_token_account(&anchor_sender.pubkey(), &mint.pubkey(), 1000);
    anchor_svm.set_account(anchor_sender_token, sender_token_account).unwrap();

    // Setup recipients
    let seahorse_recipient = {
        let kp = Keypair::new();
        seahorse_svm.airdrop(&kp.pubkey(), LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let anchor_recipient = {
        let kp = Keypair::new();
        anchor_svm.airdrop(&kp.pubkey(), LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let (seahorse_recipient_token, _) = derive_user_token_pda(&seahorse_recipient.pubkey(), &mint.pubkey(), &program_id);
    let (anchor_recipient_token, _) = derive_user_token_pda(&anchor_recipient.pubkey(), &mint.pubkey(), &program_id);

    // Create recipient token accounts with 0 balance
    let empty_token_account = create_token_account(&seahorse_recipient.pubkey(), &mint.pubkey(), 0);
    seahorse_svm.set_account(seahorse_recipient_token, empty_token_account).unwrap();

    let empty_token_account = create_token_account(&anchor_recipient.pubkey(), &mint.pubkey(), 0);
    anchor_svm.set_account(anchor_recipient_token, empty_token_account).unwrap();

    // Transfer 300 tokens using Seahorse
    let amount: u64 = 300;
    let seahorse_ix = InstructionBuilder::new("transfer_tokens")
        .with_signer(&seahorse_sender.pubkey())
        .with_readonly(&mint.pubkey())
        .with_writable(&seahorse_sender_token)
        .with_writable(&seahorse_recipient_token)
        .with_readonly(&token_program_id())
        .with_data(amount.to_le_bytes().to_vec())
        .build(program_id);

    let blockhash = seahorse_svm.latest_blockhash();
    let message = solana_message::Message::new(&[seahorse_ix], Some(&seahorse_sender.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seahorse_sender], message, blockhash);
    let seahorse_result = seahorse_svm.send_transaction(tx);

    // Transfer 300 tokens using Anchor
    let anchor_ix = InstructionBuilder::new("transfer_tokens")
        .with_signer(&anchor_sender.pubkey())
        .with_readonly(&mint.pubkey())
        .with_writable(&anchor_sender_token)
        .with_writable(&anchor_recipient_token)
        .with_readonly(&token_program_id())
        .with_data(amount.to_le_bytes().to_vec())
        .build(program_id);

    let blockhash = anchor_svm.latest_blockhash();
    let message = solana_message::Message::new(&[anchor_ix], Some(&anchor_sender.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&anchor_sender], message, blockhash);
    let anchor_result = anchor_svm.send_transaction(tx);

    // Both should succeed
    assert!(seahorse_result.is_ok(), "Seahorse transfer failed: {:?}", seahorse_result);
    assert!(anchor_result.is_ok(), "Anchor transfer failed: {:?}", anchor_result);

    // Verify balances match
    let seahorse_sender_account = seahorse_svm.get_account(&seahorse_sender_token).unwrap();
    let anchor_sender_account = anchor_svm.get_account(&anchor_sender_token).unwrap();
    assert_eq!(
        read_token_balance(&seahorse_sender_account.data),
        read_token_balance(&anchor_sender_account.data),
        "Sender balances should match"
    );
    assert_eq!(read_token_balance(&seahorse_sender_account.data), 700, "Sender should have 700");

    let seahorse_recipient_account = seahorse_svm.get_account(&seahorse_recipient_token).unwrap();
    let anchor_recipient_account = anchor_svm.get_account(&anchor_recipient_token).unwrap();
    assert_eq!(
        read_token_balance(&seahorse_recipient_account.data),
        read_token_balance(&anchor_recipient_account.data),
        "Recipient balances should match"
    );
    assert_eq!(read_token_balance(&seahorse_recipient_account.data), 300, "Recipient should have 300");
}

#[test]
fn test_register_user_state_parity() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/ata_patterns.so").unwrap();
    let anchor_bytes = std::fs::read("../../target/deploy/ata_patterns_anchor.so").unwrap();

    let mut seahorse_svm = litesvm::LiteSVM::new();
    seahorse_svm.add_program(program_id, &seahorse_bytes);
    let seahorse_user = {
        let kp = Keypair::new();
        seahorse_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let mut anchor_svm = litesvm::LiteSVM::new();
    anchor_svm.add_program(program_id, &anchor_bytes);
    let anchor_user = {
        let kp = Keypair::new();
        anchor_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    // Setup mint
    let mint = Keypair::new();
    let mint_account = create_mint_account(&Keypair::new().pubkey(), 6);
    seahorse_svm.set_account(mint.pubkey(), mint_account.clone()).unwrap();
    anchor_svm.set_account(mint.pubkey(), mint_account).unwrap();

    // Derive PDAs
    let (seahorse_user_account, _) = derive_user_account_pda(&seahorse_user.pubkey(), &mint.pubkey(), &program_id);
    let (anchor_user_account, _) = derive_user_account_pda(&anchor_user.pubkey(), &mint.pubkey(), &program_id);

    let (seahorse_token_pda, _) = derive_user_token_pda(&seahorse_user.pubkey(), &mint.pubkey(), &program_id);
    let (anchor_token_pda, _) = derive_user_token_pda(&anchor_user.pubkey(), &mint.pubkey(), &program_id);

    // Register user using Seahorse
    let seahorse_ix = InstructionBuilder::new("register_user")
        .with_signer(&seahorse_user.pubkey())
        .with_readonly(&mint.pubkey())
        .with_writable(&seahorse_user_account)
        .with_writable(&seahorse_token_pda)
        .with_readonly(&token_program_id())
        .with_readonly(&system_program::id())
        .build(program_id);

    let blockhash = seahorse_svm.latest_blockhash();
    let message = solana_message::Message::new(&[seahorse_ix], Some(&seahorse_user.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seahorse_user], message, blockhash);
    let seahorse_result = seahorse_svm.send_transaction(tx);

    // Register user using Anchor
    let anchor_ix = InstructionBuilder::new("register_user")
        .with_signer(&anchor_user.pubkey())
        .with_readonly(&mint.pubkey())
        .with_writable(&anchor_user_account)
        .with_writable(&anchor_token_pda)
        .with_readonly(&token_program_id())
        .with_readonly(&system_program::id())
        .build(program_id);

    let blockhash = anchor_svm.latest_blockhash();
    let message = solana_message::Message::new(&[anchor_ix], Some(&anchor_user.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&anchor_user], message, blockhash);
    let anchor_result = anchor_svm.send_transaction(tx);

    // Both should succeed
    assert!(seahorse_result.is_ok(), "Seahorse register_user failed: {:?}", seahorse_result);
    assert!(anchor_result.is_ok(), "Anchor register_user failed: {:?}", anchor_result);

    // Verify user accounts exist and match
    let seahorse_account = seahorse_svm.get_account(&seahorse_user_account).unwrap();
    let anchor_account = anchor_svm.get_account(&anchor_user_account).unwrap();

    // Both should be owned by the program
    assert_eq!(seahorse_account.owner, program_id);
    assert_eq!(anchor_account.owner, program_id);

    // Both should have is_registered = true
    assert!(read_user_account_is_registered(&seahorse_account.data), "Seahorse user should be registered");
    assert!(read_user_account_is_registered(&anchor_account.data), "Anchor user should be registered");

    // Account sizes should match
    assert_eq!(
        seahorse_account.data.len(),
        anchor_account.data.len(),
        "UserAccount sizes should match: Seahorse={}, Anchor={}",
        seahorse_account.data.len(),
        anchor_account.data.len()
    );

    // Token accounts should exist
    assert!(seahorse_svm.get_account(&seahorse_token_pda).is_some(), "Seahorse token account should exist");
    assert!(anchor_svm.get_account(&anchor_token_pda).is_some(), "Anchor token account should exist");
}

#[test]
fn test_treasury_initialization_parity() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/ata_patterns.so").unwrap();
    let anchor_bytes = std::fs::read("../../target/deploy/ata_patterns_anchor.so").unwrap();

    let mut seahorse_svm = litesvm::LiteSVM::new();
    seahorse_svm.add_program(program_id, &seahorse_bytes);
    let seahorse_admin = {
        let kp = Keypair::new();
        seahorse_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let mut anchor_svm = litesvm::LiteSVM::new();
    anchor_svm.add_program(program_id, &anchor_bytes);
    let anchor_admin = {
        let kp = Keypair::new();
        anchor_svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    // Setup mint
    let mint = Keypair::new();
    let mint_account = create_mint_account(&Keypair::new().pubkey(), 6);
    seahorse_svm.set_account(mint.pubkey(), mint_account.clone()).unwrap();
    anchor_svm.set_account(mint.pubkey(), mint_account).unwrap();

    // Derive treasury PDAs
    let (seahorse_treasury, _) = derive_treasury_pda(&seahorse_admin.pubkey(), &mint.pubkey(), &program_id);
    let (anchor_treasury, _) = derive_treasury_pda(&anchor_admin.pubkey(), &mint.pubkey(), &program_id);

    let (seahorse_treasury_token, _) = derive_treasury_token_pda(&seahorse_admin.pubkey(), &mint.pubkey(), &program_id);
    let (anchor_treasury_token, _) = derive_treasury_token_pda(&anchor_admin.pubkey(), &mint.pubkey(), &program_id);

    // Initialize treasury using Seahorse
    let seahorse_ix = InstructionBuilder::new("initialize_treasury")
        .with_signer(&seahorse_admin.pubkey())
        .with_readonly(&mint.pubkey())
        .with_writable(&seahorse_treasury)
        .with_writable(&seahorse_treasury_token)
        .with_readonly(&token_program_id())
        .with_readonly(&system_program::id())
        .build(program_id);

    let blockhash = seahorse_svm.latest_blockhash();
    let message = solana_message::Message::new(&[seahorse_ix], Some(&seahorse_admin.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seahorse_admin], message, blockhash);
    let seahorse_result = seahorse_svm.send_transaction(tx);

    // Initialize treasury using Anchor
    let anchor_ix = InstructionBuilder::new("initialize_treasury")
        .with_signer(&anchor_admin.pubkey())
        .with_readonly(&mint.pubkey())
        .with_writable(&anchor_treasury)
        .with_writable(&anchor_treasury_token)
        .with_readonly(&token_program_id())
        .with_readonly(&system_program::id())
        .build(program_id);

    let blockhash = anchor_svm.latest_blockhash();
    let message = solana_message::Message::new(&[anchor_ix], Some(&anchor_admin.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&anchor_admin], message, blockhash);
    let anchor_result = anchor_svm.send_transaction(tx);

    // Both should succeed
    assert!(seahorse_result.is_ok(), "Seahorse initialize_treasury failed: {:?}", seahorse_result);
    assert!(anchor_result.is_ok(), "Anchor initialize_treasury failed: {:?}", anchor_result);

    // Verify treasury accounts exist
    let seahorse_account = seahorse_svm.get_account(&seahorse_treasury).unwrap();
    let anchor_account = anchor_svm.get_account(&anchor_treasury).unwrap();

    // Both should be owned by the program
    assert_eq!(seahorse_account.owner, program_id);
    assert_eq!(anchor_account.owner, program_id);

    // Admin fields should match the respective admin pubkeys
    assert_eq!(read_treasury_admin(&seahorse_account.data), seahorse_admin.pubkey());
    assert_eq!(read_treasury_admin(&anchor_account.data), anchor_admin.pubkey());

    // Account sizes should match
    assert_eq!(
        seahorse_account.data.len(),
        anchor_account.data.len(),
        "Treasury account sizes should match: Seahorse={}, Anchor={}",
        seahorse_account.data.len(),
        anchor_account.data.len()
    );

    // Treasury token accounts should exist
    let seahorse_token_account = seahorse_svm.get_account(&seahorse_treasury_token);
    let anchor_token_account = anchor_svm.get_account(&anchor_treasury_token);

    assert!(seahorse_token_account.is_some(), "Seahorse treasury token account should exist");
    assert!(anchor_token_account.is_some(), "Anchor treasury token account should exist");

    // Both should be owned by token program
    assert_eq!(seahorse_token_account.as_ref().unwrap().owner, token_program_id());
    assert_eq!(anchor_token_account.as_ref().unwrap().owner, token_program_id());
}
