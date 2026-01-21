//! Escrow tests: Seahorse implementation behavior verification
//!
//! NOTE: The Seahorse and Anchor escrow implementations have INTENTIONALLY DIFFERENT
//! account interfaces:
//! - Seahorse: Uses basic token accounts, simpler structure (Box<Account>)
//! - Anchor reference: Uses ATA constraints, InterfaceAccount types, production patterns
//!
//! Due to these interface differences, true parity testing isn't meaningful.
//! These tests verify the Seahorse implementation behaves correctly independently.
//!
//! For full escrow integration tests, see tests/integration/escrow_integration.rs
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile:
//! - target/deploy/escrow.so (Seahorse)

use seahorse_test_common::*;
use solana_account::Account;
use solana_keypair::Keypair;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use spl_token::solana_program::program_option::COption;
use spl_token::solana_program::program_pack::Pack;
use std::str::FromStr;

/// SPL Token Mint account size
const MINT_SIZE: usize = 82;
/// SPL Token Account size
const TOKEN_ACCOUNT_SIZE: usize = 165;

/// Escrow program ID
fn program_id() -> Pubkey {
    Pubkey::from_str("7bNQSAJXfZjwTP86A3Z53WP8VSEHT4qFY4LMcepq6MPm").unwrap()
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

/// Derive escrow PDA
fn derive_escrow_pda(escrow_id: u64, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"escrow", &escrow_id.to_le_bytes()], program_id)
}

/// Derive vault PDA
fn derive_vault_pda(escrow_id: u64, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"vault", &escrow_id.to_le_bytes()], program_id)
}

/// Get ATA address
fn get_ata(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    spl_associated_token_account::get_associated_token_address(wallet, mint)
}

/// Read token account balance
fn read_token_balance(data: &[u8]) -> u64 {
    let account = spl_token::state::Account::unpack(data).unwrap();
    account.amount
}

/// Read escrow_id from escrow account data
fn read_escrow_id(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[8..16].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

/// Read token_a_amount from escrow account data
fn read_escrow_token_a_amount(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[112..120].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

/// Read token_b_wanted_amount from escrow account data
fn read_escrow_token_b_wanted(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[120..128].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

/// Build make_escrow instruction data
fn make_escrow_data(escrow_id: u64, token_a_amount: u64, token_b_wanted: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&escrow_id.to_le_bytes());
    data.extend_from_slice(&token_a_amount.to_le_bytes());
    data.extend_from_slice(&token_b_wanted.to_le_bytes());
    data
}

#[test]
fn test_seahorse_escrow_state_initialization() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/escrow.so")
        .expect("Failed to read escrow.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup mints
    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    svm.set_account(mint_a.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();
    svm.set_account(mint_b.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    // Setup maker
    let maker = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let maker_token_a = get_ata(&maker.pubkey(), &mint_a.pubkey());
    svm.set_account(
        maker_token_a,
        create_token_account(&maker.pubkey(), &mint_a.pubkey(), 1000),
    )
    .unwrap();

    let escrow_id: u64 = 1;
    let token_a_amount: u64 = 500;
    let token_b_wanted: u64 = 300;

    let (escrow_pda, _) = derive_escrow_pda(escrow_id, &program_id);
    let (vault_pda, _) = derive_vault_pda(escrow_id, &program_id);

    // Execute make_escrow
    let ix = InstructionBuilder::new("make_escrow")
        .with_signer(&maker.pubkey())
        .with_writable(&mint_a.pubkey())
        .with_writable(&mint_b.pubkey())
        .with_writable(&maker_token_a)
        .with_writable(&escrow_pda)
        .with_writable(&vault_pda)
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(make_escrow_data(escrow_id, token_a_amount, token_b_wanted))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[ix], Some(&maker.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&maker], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "make_escrow failed: {:?}", result);

    // Verify escrow state
    let escrow = svm.get_account(&escrow_pda).unwrap();
    assert_eq!(escrow.owner, program_id);
    assert_eq!(read_escrow_id(&escrow.data), escrow_id);
    assert_eq!(read_escrow_token_a_amount(&escrow.data), token_a_amount);
    assert_eq!(read_escrow_token_b_wanted(&escrow.data), token_b_wanted);

    // Verify vault balance
    let vault = svm.get_account(&vault_pda).unwrap();
    assert_eq!(read_token_balance(&vault.data), token_a_amount);

    // Verify maker balance decreased
    let maker_balance = read_token_balance(&svm.get_account(&maker_token_a).unwrap().data);
    assert_eq!(maker_balance, 1000 - token_a_amount);
}

#[test]
fn test_seahorse_escrow_full_workflow() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/escrow.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup mints
    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    svm.set_account(mint_a.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();
    svm.set_account(mint_b.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    // Setup maker with Token A
    let maker = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let maker_token_a = get_ata(&maker.pubkey(), &mint_a.pubkey());
    let maker_token_b = get_ata(&maker.pubkey(), &mint_b.pubkey());
    svm.set_account(
        maker_token_a,
        create_token_account(&maker.pubkey(), &mint_a.pubkey(), 1000),
    )
    .unwrap();
    svm.set_account(
        maker_token_b,
        create_token_account(&maker.pubkey(), &mint_b.pubkey(), 0),
    )
    .unwrap();

    // Setup taker with Token B
    let taker = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let taker_token_a = get_ata(&taker.pubkey(), &mint_a.pubkey());
    let taker_token_b = get_ata(&taker.pubkey(), &mint_b.pubkey());
    svm.set_account(
        taker_token_a,
        create_token_account(&taker.pubkey(), &mint_a.pubkey(), 0),
    )
    .unwrap();
    svm.set_account(
        taker_token_b,
        create_token_account(&taker.pubkey(), &mint_b.pubkey(), 1000),
    )
    .unwrap();

    let escrow_id: u64 = 42;
    let token_a_amount: u64 = 600;
    let token_b_wanted: u64 = 400;

    let (escrow_pda, _) = derive_escrow_pda(escrow_id, &program_id);
    let (vault_pda, _) = derive_vault_pda(escrow_id, &program_id);

    // Make escrow
    let make_ix = InstructionBuilder::new("make_escrow")
        .with_signer(&maker.pubkey())
        .with_writable(&mint_a.pubkey())
        .with_writable(&mint_b.pubkey())
        .with_writable(&maker_token_a)
        .with_writable(&escrow_pda)
        .with_writable(&vault_pda)
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(make_escrow_data(escrow_id, token_a_amount, token_b_wanted))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[make_ix], Some(&maker.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&maker], message, blockhash);
    svm.send_transaction(tx).unwrap();

    // Take escrow
    svm.expire_blockhash();
    let take_ix = InstructionBuilder::new("take_escrow")
        .with_signer(&taker.pubkey())
        .with_writable(&maker.pubkey())
        .with_writable(&mint_a.pubkey())
        .with_writable(&mint_b.pubkey())
        .with_writable(&taker_token_a)
        .with_writable(&taker_token_b)
        .with_writable(&maker_token_b)
        .with_writable(&escrow_pda)
        .with_writable(&vault_pda)
        .with_readonly(&token_program_id())
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[take_ix], Some(&taker.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&taker], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "take_escrow failed: {:?}", result);

    // Verify final balances
    // Maker: kept 400 Token A, received 400 Token B
    let maker_a_final = read_token_balance(&svm.get_account(&maker_token_a).unwrap().data);
    let maker_b_final = read_token_balance(&svm.get_account(&maker_token_b).unwrap().data);
    assert_eq!(maker_a_final, 400, "Maker should have 400 Token A");
    assert_eq!(maker_b_final, 400, "Maker should have 400 Token B");

    // Taker: received 600 Token A, sent 400 Token B
    let taker_a_final = read_token_balance(&svm.get_account(&taker_token_a).unwrap().data);
    let taker_b_final = read_token_balance(&svm.get_account(&taker_token_b).unwrap().data);
    assert_eq!(taker_a_final, 600, "Taker should have 600 Token A");
    assert_eq!(taker_b_final, 600, "Taker should have 600 Token B");
}

#[test]
fn test_seahorse_escrow_cancel() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/escrow.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup mints
    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    svm.set_account(mint_a.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();
    svm.set_account(mint_b.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    // Setup maker
    let maker = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let maker_token_a = get_ata(&maker.pubkey(), &mint_a.pubkey());
    svm.set_account(
        maker_token_a,
        create_token_account(&maker.pubkey(), &mint_a.pubkey(), 1000),
    )
    .unwrap();

    let escrow_id: u64 = 1;
    let (escrow_pda, _) = derive_escrow_pda(escrow_id, &program_id);
    let (vault_pda, _) = derive_vault_pda(escrow_id, &program_id);

    // Make escrow
    let make_ix = InstructionBuilder::new("make_escrow")
        .with_signer(&maker.pubkey())
        .with_writable(&mint_a.pubkey())
        .with_writable(&mint_b.pubkey())
        .with_writable(&maker_token_a)
        .with_writable(&escrow_pda)
        .with_writable(&vault_pda)
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(make_escrow_data(escrow_id, 500, 300))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[make_ix], Some(&maker.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&maker], message, blockhash);
    svm.send_transaction(tx).unwrap();

    // Verify escrow created
    assert!(svm.get_account(&escrow_pda).is_some());
    assert_eq!(
        read_token_balance(&svm.get_account(&maker_token_a).unwrap().data),
        500
    );

    // Cancel escrow
    svm.expire_blockhash();
    let cancel_ix = InstructionBuilder::new("cancel_escrow")
        .with_signer(&maker.pubkey())
        .with_writable(&mint_a.pubkey())
        .with_writable(&maker_token_a)
        .with_writable(&escrow_pda)
        .with_writable(&vault_pda)
        .with_readonly(&token_program_id())
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[cancel_ix], Some(&maker.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&maker], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "cancel_escrow failed: {:?}", result);

    // Verify maker got tokens back
    let maker_final = read_token_balance(&svm.get_account(&maker_token_a).unwrap().data);
    assert_eq!(maker_final, 1000, "Maker should have all 1000 tokens back");
}

#[test]
fn test_seahorse_escrow_deterministic_pda() {
    let program_id = program_id();

    // Verify PDA derivation is deterministic
    let escrow_id: u64 = 123;
    let (pda1, bump1) = derive_escrow_pda(escrow_id, &program_id);
    let (pda2, bump2) = derive_escrow_pda(escrow_id, &program_id);

    assert_eq!(pda1, pda2, "PDAs should be deterministic");
    assert_eq!(bump1, bump2, "Bumps should be deterministic");

    // Different escrow IDs should have different PDAs
    let (pda3, _) = derive_escrow_pda(456, &program_id);
    assert_ne!(pda1, pda3, "Different escrow IDs should have different PDAs");
}
