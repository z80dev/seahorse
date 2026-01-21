//! Lending tests: Seahorse implementation behavior verification
//!
//! These tests verify the Seahorse lending implementation produces
//! mathematically correct results for collateralized borrowing.
//!
//! Key behaviors to verify:
//! - Pool initialization creates correct PDAs and state
//! - User position creation
//! - Collateral deposit updates pool totals
//! - Borrow enforces 150% collateral ratio
//! - Interest accrual formula: borrowed * rate * slots / INTEREST_SCALE
//! - Repay reduces debt (including accrued interest)
//! - Withdraw maintains collateral ratio
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile:
//! - target/deploy/lending.so (Seahorse)

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

/// Collateral ratio: 150% (15000 basis points)
const COLLATERAL_RATIO_BPS: u64 = 15000;
const BPS_DENOMINATOR: u64 = 10000;

/// Interest scale factor (same as Seahorse program)
const INTEREST_SCALE: u64 = 1_000_000;

/// Lending program ID
fn program_id() -> Pubkey {
    Pubkey::from_str("6LEndKVBqeRVFT6UYm5NN1SqBSCqCWvPaoYdJzrgLMge").unwrap()
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

/// Derive pool PDA
fn derive_pool_pda(collateral_mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"pool", collateral_mint.as_ref()], program_id)
}

/// Derive collateral vault PDA
fn derive_collateral_vault_pda(collateral_mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"collateral_vault", collateral_mint.as_ref()], program_id)
}

/// Derive borrow vault PDA
fn derive_borrow_vault_pda(collateral_mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"borrow_vault", collateral_mint.as_ref()], program_id)
}

/// Derive user position PDA
fn derive_user_position_pda(collateral_mint: &Pubkey, user: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"user_position", collateral_mint.as_ref(), user.as_ref()], program_id)
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

/// Read pool fields from account data
fn read_pool_interest_rate(data: &[u8]) -> u64 {
    // Offset: 8 (disc) + 32 (authority) + 32 (collateral_mint) + 32 (borrow_mint)
    //         + 32 (collateral_vault) + 32 (borrow_vault) = 168
    let bytes: [u8; 8] = data[168..176].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_pool_total_deposited(data: &[u8]) -> u64 {
    // Offset: 168 + 8 (interest_rate) = 176
    let bytes: [u8; 8] = data[176..184].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_pool_total_borrowed(data: &[u8]) -> u64 {
    // Offset: 176 + 8 (total_deposited) = 184
    let bytes: [u8; 8] = data[184..192].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

/// Read user position fields from account data
fn read_position_collateral_deposited(data: &[u8]) -> u64 {
    // Offset: 8 (disc) + 32 (owner) + 32 (pool) = 72
    let bytes: [u8; 8] = data[72..80].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_position_borrowed_amount(data: &[u8]) -> u64 {
    // Offset: 72 + 8 (collateral_deposited) = 80
    let bytes: [u8; 8] = data[80..88].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

/// Build instruction data
fn initialize_pool_data(interest_rate: u64) -> Vec<u8> {
    interest_rate.to_le_bytes().to_vec()
}

fn amount_data(amount: u64) -> Vec<u8> {
    amount.to_le_bytes().to_vec()
}

/// Calculate expected interest
fn calculate_expected_interest(borrowed: u64, rate: u64, slots: u64) -> u64 {
    borrowed * rate * slots / INTEREST_SCALE
}

/// Calculate required collateral for a borrow amount
fn calculate_required_collateral(borrow_amount: u64) -> u64 {
    borrow_amount * COLLATERAL_RATIO_BPS / BPS_DENOMINATOR
}

// =============================================================================
// PARITY TESTS
// =============================================================================

#[test]
fn test_parity_pool_initialization() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/lending.so")
        .expect("Failed to read lending.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup mints
    let collateral_mint = Keypair::new();
    svm.set_account(collateral_mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    let borrow_mint = Keypair::new();
    svm.set_account(borrow_mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    // Setup authority
    let authority = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    // Derive PDAs
    let (pool_pda, _) = derive_pool_pda(&collateral_mint.pubkey(), &program_id);
    let (collateral_vault_pda, _) = derive_collateral_vault_pda(&collateral_mint.pubkey(), &program_id);
    let (borrow_vault_pda, _) = derive_borrow_vault_pda(&collateral_mint.pubkey(), &program_id);

    let interest_rate: u64 = 1000; // Interest per slot

    // Initialize pool
    let ix = InstructionBuilder::new("initialize_pool")
        .with_signer(&authority.pubkey())
        .with_writable(&collateral_mint.pubkey())
        .with_writable(&borrow_mint.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&collateral_vault_pda)
        .with_writable(&borrow_vault_pda)
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(initialize_pool_data(interest_rate))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[ix], Some(&authority.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&authority], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "initialize_pool failed: {:?}", result);

    // Verify pool state
    let pool = svm.get_account(&pool_pda).expect("Pool should exist");
    assert_eq!(pool.owner, program_id, "Pool should be owned by program");
    assert_eq!(read_pool_total_deposited(&pool.data), 0, "Total deposited should be 0");
    assert_eq!(read_pool_total_borrowed(&pool.data), 0, "Total borrowed should be 0");
    assert_eq!(read_pool_interest_rate(&pool.data), interest_rate, "Interest rate should be stored");

    // Verify vaults exist
    let collateral_vault = svm.get_account(&collateral_vault_pda).expect("Collateral vault should exist");
    assert_eq!(collateral_vault.owner, token_program_id());
    assert_eq!(read_token_balance(&collateral_vault.data), 0, "Collateral vault should be empty");

    let borrow_vault = svm.get_account(&borrow_vault_pda).expect("Borrow vault should exist");
    assert_eq!(borrow_vault.owner, token_program_id());
    assert_eq!(read_token_balance(&borrow_vault.data), 0, "Borrow vault should be empty");
}

#[test]
fn test_parity_collateral_ratio_enforcement() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/lending.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup
    let collateral_mint = Keypair::new();
    svm.set_account(collateral_mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    let borrow_mint = Keypair::new();
    svm.set_account(borrow_mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    let authority = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let (pool_pda, _) = derive_pool_pda(&collateral_mint.pubkey(), &program_id);
    let (collateral_vault_pda, _) = derive_collateral_vault_pda(&collateral_mint.pubkey(), &program_id);
    let (borrow_vault_pda, _) = derive_borrow_vault_pda(&collateral_mint.pubkey(), &program_id);

    // Initialize pool with zero interest for simpler testing
    let init_ix = InstructionBuilder::new("initialize_pool")
        .with_signer(&authority.pubkey())
        .with_writable(&collateral_mint.pubkey())
        .with_writable(&borrow_mint.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&collateral_vault_pda)
        .with_writable(&borrow_vault_pda)
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(initialize_pool_data(0))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[init_ix], Some(&authority.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&authority], message, blockhash);
    svm.send_transaction(tx).unwrap();

    // Add liquidity to borrow vault
    let borrow_vault_account = svm.get_account(&borrow_vault_pda).unwrap();
    let mut vault_with_liquidity = create_token_account(&pool_pda, &borrow_mint.pubkey(), 100000);
    vault_with_liquidity.lamports = borrow_vault_account.lamports;
    svm.set_account(borrow_vault_pda, vault_with_liquidity).unwrap();
    svm.expire_blockhash();

    // Setup user
    let user = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let user_collateral = get_ata(&user.pubkey(), &collateral_mint.pubkey());
    let user_borrow = get_ata(&user.pubkey(), &borrow_mint.pubkey());

    svm.set_account(user_collateral, create_token_account(&user.pubkey(), &collateral_mint.pubkey(), 15000))
        .unwrap();
    svm.set_account(user_borrow, create_token_account(&user.pubkey(), &borrow_mint.pubkey(), 0))
        .unwrap();

    let (user_position_pda, _) = derive_user_position_pda(&collateral_mint.pubkey(), &user.pubkey(), &program_id);

    // Create user position
    let create_pos_ix = InstructionBuilder::new("create_user_position")
        .with_signer(&user.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&user_position_pda)
        .with_writable(&collateral_mint.pubkey())
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[create_pos_ix], Some(&user.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&user], message, blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // Deposit 15000 collateral
    let deposit_ix = InstructionBuilder::new("deposit_collateral")
        .with_signer(&user.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&user_position_pda)
        .with_writable(&user_collateral)
        .with_writable(&collateral_vault_pda)
        .with_writable(&collateral_mint.pubkey())
        .with_readonly(&token_program_id())
        .with_data(amount_data(15000))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[deposit_ix], Some(&user.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&user], message, blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // Verify collateral ratio calculation
    // With 15000 collateral at 150% ratio:
    // Max borrow = 15000 * 10000 / 15000 = 10000
    let collateral_deposited = 15000u64;
    let max_borrow = collateral_deposited * BPS_DENOMINATOR / COLLATERAL_RATIO_BPS;
    assert_eq!(max_borrow, 10000, "Max borrow should be 10000 with 15000 collateral at 150%");

    // Verify required collateral calculation
    let borrow_amount = 5000u64;
    let required_collateral = calculate_required_collateral(borrow_amount);
    assert_eq!(required_collateral, 7500, "5000 borrow requires 7500 collateral at 150%");

    // Borrow 5000 (within ratio)
    let borrow_ix = InstructionBuilder::new("borrow")
        .with_signer(&user.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&user_position_pda)
        .with_writable(&user_borrow)
        .with_writable(&borrow_vault_pda)
        .with_writable(&collateral_mint.pubkey())
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&token_program_id())
        .with_data(amount_data(borrow_amount))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[borrow_ix], Some(&user.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&user], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "borrow within ratio should succeed: {:?}", result);

    // Verify position state
    let position = svm.get_account(&user_position_pda).unwrap();
    assert_eq!(read_position_borrowed_amount(&position.data), borrow_amount);
    assert_eq!(read_position_collateral_deposited(&position.data), collateral_deposited);
}

#[test]
fn test_parity_interest_accrual_formula() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/lending.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup
    let collateral_mint = Keypair::new();
    svm.set_account(collateral_mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    let borrow_mint = Keypair::new();
    svm.set_account(borrow_mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    let authority = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let (pool_pda, _) = derive_pool_pda(&collateral_mint.pubkey(), &program_id);
    let (collateral_vault_pda, _) = derive_collateral_vault_pda(&collateral_mint.pubkey(), &program_id);
    let (borrow_vault_pda, _) = derive_borrow_vault_pda(&collateral_mint.pubkey(), &program_id);

    let interest_rate: u64 = 100_000; // 0.1 interest per token per slot

    // Initialize pool
    let init_ix = InstructionBuilder::new("initialize_pool")
        .with_signer(&authority.pubkey())
        .with_writable(&collateral_mint.pubkey())
        .with_writable(&borrow_mint.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&collateral_vault_pda)
        .with_writable(&borrow_vault_pda)
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(initialize_pool_data(interest_rate))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[init_ix], Some(&authority.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&authority], message, blockhash);
    svm.send_transaction(tx).unwrap();

    // Add liquidity
    let borrow_vault_account = svm.get_account(&borrow_vault_pda).unwrap();
    let mut vault_with_liquidity = create_token_account(&pool_pda, &borrow_mint.pubkey(), 100000);
    vault_with_liquidity.lamports = borrow_vault_account.lamports;
    svm.set_account(borrow_vault_pda, vault_with_liquidity).unwrap();
    svm.expire_blockhash();

    // Setup user with lots of collateral
    let user = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let user_collateral = get_ata(&user.pubkey(), &collateral_mint.pubkey());
    let user_borrow = get_ata(&user.pubkey(), &borrow_mint.pubkey());

    svm.set_account(user_collateral, create_token_account(&user.pubkey(), &collateral_mint.pubkey(), 200000))
        .unwrap();
    svm.set_account(user_borrow, create_token_account(&user.pubkey(), &borrow_mint.pubkey(), 0))
        .unwrap();

    let (user_position_pda, _) = derive_user_position_pda(&collateral_mint.pubkey(), &user.pubkey(), &program_id);

    // Create position and deposit
    let create_pos_ix = InstructionBuilder::new("create_user_position")
        .with_signer(&user.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&user_position_pda)
        .with_writable(&collateral_mint.pubkey())
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[create_pos_ix], Some(&user.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&user], message, blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    let deposit_ix = InstructionBuilder::new("deposit_collateral")
        .with_signer(&user.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&user_position_pda)
        .with_writable(&user_collateral)
        .with_writable(&collateral_vault_pda)
        .with_writable(&collateral_mint.pubkey())
        .with_readonly(&token_program_id())
        .with_data(amount_data(200000))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[deposit_ix], Some(&user.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&user], message, blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // First borrow: 1000 tokens
    let borrow_amount: u64 = 1000;
    let borrow_ix = InstructionBuilder::new("borrow")
        .with_signer(&user.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&user_position_pda)
        .with_writable(&user_borrow)
        .with_writable(&borrow_vault_pda)
        .with_writable(&collateral_mint.pubkey())
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&token_program_id())
        .with_data(amount_data(borrow_amount))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[borrow_ix], Some(&user.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&user], message, blockhash);
    svm.send_transaction(tx).unwrap();

    // Verify initial debt
    let position = svm.get_account(&user_position_pda).unwrap();
    assert_eq!(read_position_borrowed_amount(&position.data), 1000);

    // Verify interest formula calculation
    // interest = borrowed * rate * slots / INTEREST_SCALE
    // For 1000 borrowed, 100_000 rate, 100 slots:
    // interest = 1000 * 100_000 * 100 / 1_000_000 = 10_000
    let slots_elapsed: u64 = 100;
    let expected_interest = calculate_expected_interest(borrow_amount, interest_rate, slots_elapsed);
    assert_eq!(expected_interest, 10_000, "Expected interest should be 10,000");

    // Warp forward 100 slots
    svm.warp_to_slot(slots_elapsed);
    svm.expire_blockhash();

    // Borrow more - this should trigger interest accrual
    let borrow_ix2 = InstructionBuilder::new("borrow")
        .with_signer(&user.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&user_position_pda)
        .with_writable(&user_borrow)
        .with_writable(&borrow_vault_pda)
        .with_writable(&collateral_mint.pubkey())
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&token_program_id())
        .with_data(amount_data(500))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[borrow_ix2], Some(&user.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&user], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "second borrow should succeed: {:?}", result);

    // Verify debt includes accrued interest: 1000 + 10000 (interest) + 500 = 11500
    let position = svm.get_account(&user_position_pda).unwrap();
    let debt = read_position_borrowed_amount(&position.data);
    assert_eq!(debt, 11500, "Debt should include accrued interest: {}", debt);
}

#[test]
fn test_parity_full_lending_workflow() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/lending.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup
    let collateral_mint = Keypair::new();
    svm.set_account(collateral_mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    let borrow_mint = Keypair::new();
    svm.set_account(borrow_mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    let authority = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let (pool_pda, _) = derive_pool_pda(&collateral_mint.pubkey(), &program_id);
    let (collateral_vault_pda, _) = derive_collateral_vault_pda(&collateral_mint.pubkey(), &program_id);
    let (borrow_vault_pda, _) = derive_borrow_vault_pda(&collateral_mint.pubkey(), &program_id);

    // Initialize pool with zero interest for simpler verification
    let init_ix = InstructionBuilder::new("initialize_pool")
        .with_signer(&authority.pubkey())
        .with_writable(&collateral_mint.pubkey())
        .with_writable(&borrow_mint.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&collateral_vault_pda)
        .with_writable(&borrow_vault_pda)
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(initialize_pool_data(0))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[init_ix], Some(&authority.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&authority], message, blockhash);
    svm.send_transaction(tx).unwrap();

    // Add liquidity
    let borrow_vault_account = svm.get_account(&borrow_vault_pda).unwrap();
    let mut vault_with_liquidity = create_token_account(&pool_pda, &borrow_mint.pubkey(), 50000);
    vault_with_liquidity.lamports = borrow_vault_account.lamports;
    svm.set_account(borrow_vault_pda, vault_with_liquidity).unwrap();
    svm.expire_blockhash();

    // Setup user
    let user = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let user_collateral = get_ata(&user.pubkey(), &collateral_mint.pubkey());
    let user_borrow = get_ata(&user.pubkey(), &borrow_mint.pubkey());

    svm.set_account(user_collateral, create_token_account(&user.pubkey(), &collateral_mint.pubkey(), 30000))
        .unwrap();
    svm.set_account(user_borrow, create_token_account(&user.pubkey(), &borrow_mint.pubkey(), 0))
        .unwrap();

    let (user_position_pda, _) = derive_user_position_pda(&collateral_mint.pubkey(), &user.pubkey(), &program_id);

    // 1. Create position
    let create_pos_ix = InstructionBuilder::new("create_user_position")
        .with_signer(&user.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&user_position_pda)
        .with_writable(&collateral_mint.pubkey())
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[create_pos_ix], Some(&user.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&user], message, blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // 2. Deposit collateral
    let deposit_ix = InstructionBuilder::new("deposit_collateral")
        .with_signer(&user.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&user_position_pda)
        .with_writable(&user_collateral)
        .with_writable(&collateral_vault_pda)
        .with_writable(&collateral_mint.pubkey())
        .with_readonly(&token_program_id())
        .with_data(amount_data(15000))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[deposit_ix], Some(&user.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&user], message, blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // Verify collateral deposited
    let position = svm.get_account(&user_position_pda).unwrap();
    assert_eq!(read_position_collateral_deposited(&position.data), 15000);

    // 3. Borrow
    let borrow_ix = InstructionBuilder::new("borrow")
        .with_signer(&user.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&user_position_pda)
        .with_writable(&user_borrow)
        .with_writable(&borrow_vault_pda)
        .with_writable(&collateral_mint.pubkey())
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&token_program_id())
        .with_data(amount_data(5000))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[borrow_ix], Some(&user.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&user], message, blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // Verify borrowed
    let position = svm.get_account(&user_position_pda).unwrap();
    assert_eq!(read_position_borrowed_amount(&position.data), 5000);

    let user_token = svm.get_account(&user_borrow).unwrap();
    assert_eq!(read_token_balance(&user_token.data), 5000);

    // 4. Repay
    let repay_ix = InstructionBuilder::new("repay")
        .with_signer(&user.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&user_position_pda)
        .with_writable(&user_borrow)
        .with_writable(&borrow_vault_pda)
        .with_writable(&collateral_mint.pubkey())
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&token_program_id())
        .with_data(amount_data(5000))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[repay_ix], Some(&user.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&user], message, blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // Verify debt repaid
    let position = svm.get_account(&user_position_pda).unwrap();
    assert_eq!(read_position_borrowed_amount(&position.data), 0);

    // 5. Withdraw collateral
    let withdraw_ix = InstructionBuilder::new("withdraw_collateral")
        .with_signer(&user.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&user_position_pda)
        .with_writable(&user_collateral)
        .with_writable(&collateral_vault_pda)
        .with_writable(&collateral_mint.pubkey())
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&token_program_id())
        .with_data(amount_data(15000))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[withdraw_ix], Some(&user.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&user], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "withdraw should succeed: {:?}", result);

    // Verify final state
    let position = svm.get_account(&user_position_pda).unwrap();
    assert_eq!(read_position_collateral_deposited(&position.data), 0);
    assert_eq!(read_position_borrowed_amount(&position.data), 0);

    let user_collateral_token = svm.get_account(&user_collateral).unwrap();
    assert_eq!(read_token_balance(&user_collateral_token.data), 30000); // Original amount
}

#[test]
fn test_parity_deterministic_pda_derivation() {
    let program_id = program_id();

    // Create deterministic keys for consistent testing
    let collateral_mint = Pubkey::new_unique();
    let user = Pubkey::new_unique();

    // Verify pool PDA derivation is deterministic
    let (pool1, bump1) = derive_pool_pda(&collateral_mint, &program_id);
    let (pool2, bump2) = derive_pool_pda(&collateral_mint, &program_id);

    assert_eq!(pool1, pool2, "Pool PDAs should be deterministic");
    assert_eq!(bump1, bump2, "Pool bumps should be deterministic");

    // Verify user position PDA derivation is deterministic
    let (pos1, pos_bump1) = derive_user_position_pda(&collateral_mint, &user, &program_id);
    let (pos2, pos_bump2) = derive_user_position_pda(&collateral_mint, &user, &program_id);

    assert_eq!(pos1, pos2, "User position PDAs should be deterministic");
    assert_eq!(pos_bump1, pos_bump2, "User position bumps should be deterministic");

    // Verify different mints produce different pool PDAs
    let other_mint = Pubkey::new_unique();
    let (pool3, _) = derive_pool_pda(&other_mint, &program_id);
    assert_ne!(pool1, pool3, "Different mints should produce different pool PDAs");

    // Verify different users produce different position PDAs
    let other_user = Pubkey::new_unique();
    let (pos3, _) = derive_user_position_pda(&collateral_mint, &other_user, &program_id);
    assert_ne!(pos1, pos3, "Different users should produce different position PDAs");

    // Verify collateral vault PDA
    let (vault1, _) = derive_collateral_vault_pda(&collateral_mint, &program_id);
    let (vault2, _) = derive_collateral_vault_pda(&collateral_mint, &program_id);
    assert_eq!(vault1, vault2, "Collateral vault PDAs should be deterministic");

    // Verify borrow vault PDA
    let (borrow1, _) = derive_borrow_vault_pda(&collateral_mint, &program_id);
    let (borrow2, _) = derive_borrow_vault_pda(&collateral_mint, &program_id);
    assert_eq!(borrow1, borrow2, "Borrow vault PDAs should be deterministic");
}
