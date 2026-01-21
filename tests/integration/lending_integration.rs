//! LiteSVM integration tests for the Seahorse Lending program
//!
//! These tests verify a simplified lending protocol with collateralization:
//! - initialize_pool: Creates lending pool with collateral and borrow vaults
//! - create_user_position: Creates user position account
//! - deposit_collateral: Deposits collateral to secure borrowing
//! - borrow: Borrows tokens against collateral (150% collateral ratio)
//! - repay: Repays borrowed amount with interest
//! - withdraw_collateral: Withdraws collateral (if ratio maintained)
//!
//! Uses LiteSVM's warp_to_slot for interest accrual testing.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile lending.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// Lending program ID (from declare_id!)
fn lending_program_id() -> Pubkey {
    Pubkey::from_str("6LEndKVBqeRVFT6UYm5NN1SqBSCqCWvPaoYdJzrgLMge").unwrap()
}

/// Pool account size: discriminator (8) + authority (32) + collateral_mint (32) + borrow_mint (32)
///                    + collateral_vault (32) + borrow_vault (32) + interest_rate (8)
///                    + total_deposited (8) + total_borrowed (8) + last_update_slot (8) + bump (1)
const POOL_SIZE: usize = 8 + 32 + 32 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 1;

/// UserPosition account size: discriminator (8) + owner (32) + pool (32)
///                            + collateral_deposited (8) + borrowed_amount (8)
///                            + last_update_slot (8) + bump (1)
const USER_POSITION_SIZE: usize = 8 + 32 + 32 + 8 + 8 + 8 + 1;

/// Collateral ratio: 150% (15000 basis points)
const COLLATERAL_RATIO_BPS: u64 = 15000;
const BPS_DENOMINATOR: u64 = 10000;

/// Interest scale factor
const INTEREST_SCALE: u64 = 1_000_000;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the lending program into LiteSVM
fn load_lending_program() -> litesvm::LiteSVM {
    let program_id = lending_program_id();
    let program_bytes = std::fs::read("../../target/deploy/lending.so")
        .expect("Failed to read lending.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);
    svm
}

/// Setup a mint and mint authority for testing
fn setup_mint(svm: &mut litesvm::LiteSVM, decimals: u8) -> (Keypair, Pubkey) {
    let mint_authority = funded_keypair_10_sol(svm);
    let mint = Keypair::new();

    let mint_account = create_mint_account(&mint_authority.pubkey(), decimals);
    svm.set_account(mint.pubkey(), mint_account).unwrap();

    (mint_authority, mint.pubkey())
}

/// Setup a mint with a specific authority (PDA)
fn setup_mint_with_authority(svm: &mut litesvm::LiteSVM, authority: &Pubkey, decimals: u8) -> Pubkey {
    let mint = Keypair::new();

    let mint_account = create_mint_account(authority, decimals);
    svm.set_account(mint.pubkey(), mint_account).unwrap();

    mint.pubkey()
}

/// Setup a user with a token account containing tokens
fn setup_user_with_tokens(
    svm: &mut litesvm::LiteSVM,
    mint: &Pubkey,
    amount: u64,
) -> (Keypair, Pubkey) {
    let user = funded_keypair_10_sol(svm);
    let user_ata = get_ata(&user.pubkey(), mint);

    let token_account = create_token_account(&user.pubkey(), mint, amount);
    svm.set_account(user_ata, token_account).unwrap();

    (user, user_ata)
}

/// Setup a user with empty token account
fn setup_user_with_empty_token_account(
    svm: &mut litesvm::LiteSVM,
    mint: &Pubkey,
) -> (Keypair, Pubkey) {
    setup_user_with_tokens(svm, mint, 0)
}

/// Derive pool PDA
fn derive_pool_pda(collateral_mint: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"pool", collateral_mint.as_ref()],
        &lending_program_id(),
    )
}

/// Derive collateral vault PDA
fn derive_collateral_vault_pda(collateral_mint: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"collateral_vault", collateral_mint.as_ref()],
        &lending_program_id(),
    )
}

/// Derive borrow vault PDA
fn derive_borrow_vault_pda(collateral_mint: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"borrow_vault", collateral_mint.as_ref()],
        &lending_program_id(),
    )
}

/// Derive user position PDA
fn derive_user_position_pda(collateral_mint: &Pubkey, user: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"user_position", collateral_mint.as_ref(), user.as_ref()],
        &lending_program_id(),
    )
}

/// Read pool fields from account data
fn read_pool_authority(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

fn read_pool_collateral_mint(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[40..72].try_into().unwrap())
}

fn read_pool_borrow_mint(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[72..104].try_into().unwrap())
}

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
fn read_position_owner(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

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

fn read_position_last_update_slot(data: &[u8]) -> u64 {
    // Offset: 80 + 8 (borrowed_amount) = 88
    let bytes: [u8; 8] = data[88..96].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

/// Build instruction data
fn initialize_pool_data(interest_rate: u64) -> Vec<u8> {
    interest_rate.to_le_bytes().to_vec()
}

fn amount_data(amount: u64) -> Vec<u8> {
    amount.to_le_bytes().to_vec()
}

// =============================================================================
// INITIALIZE POOL TESTS
// =============================================================================

#[test]
fn test_initialize_pool() {
    let mut svm = load_lending_program();
    let program_id = lending_program_id();

    // Setup collateral and borrow mints
    let (_, collateral_mint) = setup_mint(&mut svm, 6);
    let (_, borrow_mint) = setup_mint(&mut svm, 6);

    // Setup pool authority
    let authority = funded_keypair_10_sol(&mut svm);

    // Derive PDAs
    let (pool_pda, _) = derive_pool_pda(&collateral_mint);
    let (collateral_vault_pda, _) = derive_collateral_vault_pda(&collateral_mint);
    let (borrow_vault_pda, _) = derive_borrow_vault_pda(&collateral_mint);

    let interest_rate: u64 = 1000; // Interest per slot

    // Build initialize_pool instruction (Seahorse account order)
    let ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(interest_rate),
        vec![
            signer_meta(authority.pubkey()),       // authority
            writable_meta(collateral_mint),        // collateral_mint
            writable_meta(borrow_mint),            // borrow_mint
            writable_meta(pool_pda),               // pool
            writable_meta(collateral_vault_pda),   // collateral_vault
            writable_meta(borrow_vault_pda),       // borrow_vault
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),  // rent
            readonly_meta(system_program::id()),   // system_program
            readonly_meta(token_program_id()),     // token_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "initialize_pool should succeed: {:?}", result);

    // Verify pool state
    let pool_account = svm.get_account(&pool_pda).expect("Pool should exist");
    assert_eq!(pool_account.owner, program_id, "Pool should be owned by program");

    assert_eq!(read_pool_authority(&pool_account.data), authority.pubkey());
    assert_eq!(read_pool_collateral_mint(&pool_account.data), collateral_mint);
    assert_eq!(read_pool_borrow_mint(&pool_account.data), borrow_mint);
    assert_eq!(read_pool_interest_rate(&pool_account.data), interest_rate);
    assert_eq!(read_pool_total_deposited(&pool_account.data), 0);
    assert_eq!(read_pool_total_borrowed(&pool_account.data), 0);

    // Verify vaults exist
    let collateral_vault = svm.get_account(&collateral_vault_pda).expect("Collateral vault should exist");
    assert_eq!(collateral_vault.owner, token_program_id());

    let borrow_vault = svm.get_account(&borrow_vault_pda).expect("Borrow vault should exist");
    assert_eq!(borrow_vault.owner, token_program_id());
}

// =============================================================================
// DEPOSIT COLLATERAL TESTS
// =============================================================================

#[test]
fn test_deposit_collateral() {
    let mut svm = load_lending_program();
    let program_id = lending_program_id();

    // Setup
    let (_, collateral_mint) = setup_mint(&mut svm, 6);
    let (_, borrow_mint) = setup_mint(&mut svm, 6);
    let authority = funded_keypair_10_sol(&mut svm);
    let (pool_pda, _) = derive_pool_pda(&collateral_mint);
    let (collateral_vault_pda, _) = derive_collateral_vault_pda(&collateral_mint);
    let (borrow_vault_pda, _) = derive_borrow_vault_pda(&collateral_mint);

    // Initialize pool
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(1000),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collateral_mint),
            writable_meta(borrow_mint),
            writable_meta(pool_pda),
            writable_meta(collateral_vault_pda),
            writable_meta(borrow_vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Setup user with collateral tokens
    let (user, user_collateral_token) = setup_user_with_tokens(&mut svm, &collateral_mint, 10000);
    let (user_position_pda, _) = derive_user_position_pda(&collateral_mint, &user.pubkey());

    // Create user position
    svm.expire_blockhash();
    let create_position_ix = anchor_instruction(
        program_id,
        "create_user_position",
        &[],
        vec![
            signer_meta(user.pubkey()),            // user
            writable_meta(pool_pda),               // pool
            writable_meta(user_position_pda),      // user_position
            writable_meta(collateral_mint),        // collateral_mint
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),  // rent
            readonly_meta(system_program::id()),   // system_program
        ],
    );
    execute_tx(&mut svm, create_position_ix, &user, &[&user]).unwrap();

    // Deposit collateral
    svm.expire_blockhash();
    let deposit_amount: u64 = 5000;
    let deposit_ix = anchor_instruction(
        program_id,
        "deposit_collateral",
        &amount_data(deposit_amount),
        vec![
            signer_meta(user.pubkey()),            // user
            writable_meta(pool_pda),               // pool
            writable_meta(user_position_pda),      // user_position
            writable_meta(user_collateral_token),  // user_collateral_token
            writable_meta(collateral_vault_pda),   // collateral_vault
            writable_meta(collateral_mint),        // collateral_mint
            readonly_meta(token_program_id()),     // token_program
        ],
    );

    let result = execute_tx(&mut svm, deposit_ix, &user, &[&user]);
    assert!(result.is_ok(), "deposit_collateral should succeed: {:?}", result);

    // Verify user position
    let position_account = svm.get_account(&user_position_pda).unwrap();
    assert_eq!(read_position_owner(&position_account.data), user.pubkey());
    assert_eq!(read_position_collateral_deposited(&position_account.data), deposit_amount);

    // Verify pool total deposited
    let pool_account = svm.get_account(&pool_pda).unwrap();
    assert_eq!(read_pool_total_deposited(&pool_account.data), deposit_amount);

    // Verify user's token account decreased
    let user_token_account = svm.get_account(&user_collateral_token).unwrap();
    assert_eq!(read_token_balance(&user_token_account.data), 5000);

    // Verify vault received tokens
    let vault_account = svm.get_account(&collateral_vault_pda).unwrap();
    assert_eq!(read_token_balance(&vault_account.data), deposit_amount);
}

// =============================================================================
// BORROW TESTS
// =============================================================================

#[test]
fn test_borrow_within_collateral_ratio() {
    let mut svm = load_lending_program();
    let program_id = lending_program_id();

    // Setup
    let (_, collateral_mint) = setup_mint(&mut svm, 6);
    let (_, borrow_mint) = setup_mint(&mut svm, 6);
    let authority = funded_keypair_10_sol(&mut svm);
    let (pool_pda, _) = derive_pool_pda(&collateral_mint);
    let (collateral_vault_pda, _) = derive_collateral_vault_pda(&collateral_mint);
    let (borrow_vault_pda, _) = derive_borrow_vault_pda(&collateral_mint);

    // Initialize pool
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(1000),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collateral_mint),
            writable_meta(borrow_mint),
            writable_meta(pool_pda),
            writable_meta(collateral_vault_pda),
            writable_meta(borrow_vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Add liquidity to borrow vault (simulate existing liquidity)
    let borrow_vault_account = svm.get_account(&borrow_vault_pda).unwrap();
    let mut vault_with_liquidity = create_token_account(&pool_pda, &borrow_mint, 100000);
    vault_with_liquidity.lamports = borrow_vault_account.lamports;
    svm.set_account(borrow_vault_pda, vault_with_liquidity).unwrap();

    // Setup user
    let (user, user_collateral_token) = setup_user_with_tokens(&mut svm, &collateral_mint, 15000);
    let user_borrow_token = get_ata(&user.pubkey(), &borrow_mint);
    svm.set_account(user_borrow_token, create_token_account(&user.pubkey(), &borrow_mint, 0)).unwrap();

    let (user_position_pda, _) = derive_user_position_pda(&collateral_mint, &user.pubkey());

    // Create user position
    svm.expire_blockhash();
    let create_position_ix = anchor_instruction(
        program_id,
        "create_user_position",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_position_ix, &user, &[&user]).unwrap();

    // Deposit collateral: 15000 tokens
    // At 150% ratio, can borrow up to 15000 * 10000 / 15000 = 10000 tokens
    svm.expire_blockhash();
    let deposit_ix = anchor_instruction(
        program_id,
        "deposit_collateral",
        &amount_data(15000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_collateral_token),
            writable_meta(collateral_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, deposit_ix, &user, &[&user]).unwrap();

    // Borrow 5000 tokens (well within ratio)
    svm.expire_blockhash();
    let borrow_amount: u64 = 5000;
    let borrow_ix = anchor_instruction(
        program_id,
        "borrow",
        &amount_data(borrow_amount),
        vec![
            signer_meta(user.pubkey()),            // user
            writable_meta(pool_pda),               // pool
            writable_meta(user_position_pda),      // user_position
            writable_meta(user_borrow_token),      // user_borrow_token
            writable_meta(borrow_vault_pda),       // borrow_vault
            writable_meta(collateral_mint),        // collateral_mint
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(token_program_id()),     // token_program
        ],
    );

    let result = execute_tx(&mut svm, borrow_ix, &user, &[&user]);
    assert!(result.is_ok(), "borrow should succeed: {:?}", result);

    // Verify user position
    let position_account = svm.get_account(&user_position_pda).unwrap();
    assert_eq!(read_position_borrowed_amount(&position_account.data), borrow_amount);

    // Verify user received borrow tokens
    let user_token_account = svm.get_account(&user_borrow_token).unwrap();
    assert_eq!(read_token_balance(&user_token_account.data), borrow_amount);

    // Verify pool total borrowed
    let pool_account = svm.get_account(&pool_pda).unwrap();
    assert_eq!(read_pool_total_borrowed(&pool_account.data), borrow_amount);
}

#[test]
fn test_borrow_exceeds_collateral_ratio_fails() {
    let mut svm = load_lending_program();
    let program_id = lending_program_id();

    // Setup
    let (_, collateral_mint) = setup_mint(&mut svm, 6);
    let (_, borrow_mint) = setup_mint(&mut svm, 6);
    let authority = funded_keypair_10_sol(&mut svm);
    let (pool_pda, _) = derive_pool_pda(&collateral_mint);
    let (collateral_vault_pda, _) = derive_collateral_vault_pda(&collateral_mint);
    let (borrow_vault_pda, _) = derive_borrow_vault_pda(&collateral_mint);

    // Initialize pool
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(1000),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collateral_mint),
            writable_meta(borrow_mint),
            writable_meta(pool_pda),
            writable_meta(collateral_vault_pda),
            writable_meta(borrow_vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Add liquidity to borrow vault
    let borrow_vault_account = svm.get_account(&borrow_vault_pda).unwrap();
    let mut vault_with_liquidity = create_token_account(&pool_pda, &borrow_mint, 100000);
    vault_with_liquidity.lamports = borrow_vault_account.lamports;
    svm.set_account(borrow_vault_pda, vault_with_liquidity).unwrap();

    // Setup user
    let (user, user_collateral_token) = setup_user_with_tokens(&mut svm, &collateral_mint, 15000);
    let user_borrow_token = get_ata(&user.pubkey(), &borrow_mint);
    svm.set_account(user_borrow_token, create_token_account(&user.pubkey(), &borrow_mint, 0)).unwrap();

    let (user_position_pda, _) = derive_user_position_pda(&collateral_mint, &user.pubkey());

    // Create user position
    svm.expire_blockhash();
    let create_position_ix = anchor_instruction(
        program_id,
        "create_user_position",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_position_ix, &user, &[&user]).unwrap();

    // Deposit collateral: 15000 tokens
    svm.expire_blockhash();
    let deposit_ix = anchor_instruction(
        program_id,
        "deposit_collateral",
        &amount_data(15000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_collateral_token),
            writable_meta(collateral_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, deposit_ix, &user, &[&user]).unwrap();

    // Try to borrow 11000 tokens (exceeds 150% ratio: would need 16500 collateral)
    svm.expire_blockhash();
    let borrow_ix = anchor_instruction(
        program_id,
        "borrow",
        &amount_data(11000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_borrow_token),
            writable_meta(borrow_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, borrow_ix, &user, &[&user]);
    assert!(result.is_err(), "borrow exceeding collateral ratio should fail");
}

// =============================================================================
// REPAY TESTS
// =============================================================================

#[test]
fn test_repay_partial() {
    let mut svm = load_lending_program();
    let program_id = lending_program_id();

    // Setup
    let (_, collateral_mint) = setup_mint(&mut svm, 6);
    let (_, borrow_mint) = setup_mint(&mut svm, 6);
    let authority = funded_keypair_10_sol(&mut svm);
    let (pool_pda, _) = derive_pool_pda(&collateral_mint);
    let (collateral_vault_pda, _) = derive_collateral_vault_pda(&collateral_mint);
    let (borrow_vault_pda, _) = derive_borrow_vault_pda(&collateral_mint);

    // Initialize pool
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(1000),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collateral_mint),
            writable_meta(borrow_mint),
            writable_meta(pool_pda),
            writable_meta(collateral_vault_pda),
            writable_meta(borrow_vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Add liquidity to borrow vault
    let borrow_vault_account = svm.get_account(&borrow_vault_pda).unwrap();
    let mut vault_with_liquidity = create_token_account(&pool_pda, &borrow_mint, 100000);
    vault_with_liquidity.lamports = borrow_vault_account.lamports;
    svm.set_account(borrow_vault_pda, vault_with_liquidity).unwrap();

    // Setup user
    let (user, user_collateral_token) = setup_user_with_tokens(&mut svm, &collateral_mint, 15000);
    let user_borrow_token = get_ata(&user.pubkey(), &borrow_mint);
    svm.set_account(user_borrow_token, create_token_account(&user.pubkey(), &borrow_mint, 0)).unwrap();

    let (user_position_pda, _) = derive_user_position_pda(&collateral_mint, &user.pubkey());

    // Create user position, deposit collateral, and borrow
    svm.expire_blockhash();
    let create_position_ix = anchor_instruction(
        program_id,
        "create_user_position",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_position_ix, &user, &[&user]).unwrap();

    svm.expire_blockhash();
    let deposit_ix = anchor_instruction(
        program_id,
        "deposit_collateral",
        &amount_data(15000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_collateral_token),
            writable_meta(collateral_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, deposit_ix, &user, &[&user]).unwrap();

    svm.expire_blockhash();
    let borrow_ix = anchor_instruction(
        program_id,
        "borrow",
        &amount_data(5000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_borrow_token),
            writable_meta(borrow_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, borrow_ix, &user, &[&user]).unwrap();

    // Verify user has borrowed tokens
    let user_token_account = svm.get_account(&user_borrow_token).unwrap();
    assert_eq!(read_token_balance(&user_token_account.data), 5000);

    // Repay 2000 tokens
    svm.expire_blockhash();
    let repay_ix = anchor_instruction(
        program_id,
        "repay",
        &amount_data(2000),
        vec![
            signer_meta(user.pubkey()),            // user
            writable_meta(pool_pda),               // pool
            writable_meta(user_position_pda),      // user_position
            writable_meta(user_borrow_token),      // user_borrow_token
            writable_meta(borrow_vault_pda),       // borrow_vault
            writable_meta(collateral_mint),        // collateral_mint
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(token_program_id()),     // token_program
        ],
    );

    let result = execute_tx(&mut svm, repay_ix, &user, &[&user]);
    assert!(result.is_ok(), "repay should succeed: {:?}", result);

    // Verify user position debt reduced
    let position_account = svm.get_account(&user_position_pda).unwrap();
    assert_eq!(read_position_borrowed_amount(&position_account.data), 3000);

    // Verify user's borrow token account reduced
    let user_token_account = svm.get_account(&user_borrow_token).unwrap();
    assert_eq!(read_token_balance(&user_token_account.data), 3000);
}

#[test]
fn test_repay_full() {
    let mut svm = load_lending_program();
    let program_id = lending_program_id();

    // Setup
    let (_, collateral_mint) = setup_mint(&mut svm, 6);
    let (_, borrow_mint) = setup_mint(&mut svm, 6);
    let authority = funded_keypair_10_sol(&mut svm);
    let (pool_pda, _) = derive_pool_pda(&collateral_mint);
    let (collateral_vault_pda, _) = derive_collateral_vault_pda(&collateral_mint);
    let (borrow_vault_pda, _) = derive_borrow_vault_pda(&collateral_mint);

    // Initialize pool with zero interest for simpler testing
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(0),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collateral_mint),
            writable_meta(borrow_mint),
            writable_meta(pool_pda),
            writable_meta(collateral_vault_pda),
            writable_meta(borrow_vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Add liquidity to borrow vault
    let borrow_vault_account = svm.get_account(&borrow_vault_pda).unwrap();
    let mut vault_with_liquidity = create_token_account(&pool_pda, &borrow_mint, 100000);
    vault_with_liquidity.lamports = borrow_vault_account.lamports;
    svm.set_account(borrow_vault_pda, vault_with_liquidity).unwrap();

    // Setup user
    let (user, user_collateral_token) = setup_user_with_tokens(&mut svm, &collateral_mint, 15000);
    let user_borrow_token = get_ata(&user.pubkey(), &borrow_mint);
    svm.set_account(user_borrow_token, create_token_account(&user.pubkey(), &borrow_mint, 0)).unwrap();

    let (user_position_pda, _) = derive_user_position_pda(&collateral_mint, &user.pubkey());

    // Create position, deposit, borrow
    svm.expire_blockhash();
    let create_position_ix = anchor_instruction(
        program_id,
        "create_user_position",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_position_ix, &user, &[&user]).unwrap();

    svm.expire_blockhash();
    let deposit_ix = anchor_instruction(
        program_id,
        "deposit_collateral",
        &amount_data(15000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_collateral_token),
            writable_meta(collateral_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, deposit_ix, &user, &[&user]).unwrap();

    svm.expire_blockhash();
    let borrow_ix = anchor_instruction(
        program_id,
        "borrow",
        &amount_data(5000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_borrow_token),
            writable_meta(borrow_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, borrow_ix, &user, &[&user]).unwrap();

    // Repay full amount
    svm.expire_blockhash();
    let repay_ix = anchor_instruction(
        program_id,
        "repay",
        &amount_data(5000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_borrow_token),
            writable_meta(borrow_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, repay_ix, &user, &[&user]);
    assert!(result.is_ok(), "repay full should succeed: {:?}", result);

    // Verify debt is zero
    let position_account = svm.get_account(&user_position_pda).unwrap();
    assert_eq!(read_position_borrowed_amount(&position_account.data), 0);
}

// =============================================================================
// WITHDRAW COLLATERAL TESTS
// =============================================================================

#[test]
fn test_withdraw_collateral_no_debt() {
    let mut svm = load_lending_program();
    let program_id = lending_program_id();

    // Setup
    let (_, collateral_mint) = setup_mint(&mut svm, 6);
    let (_, borrow_mint) = setup_mint(&mut svm, 6);
    let authority = funded_keypair_10_sol(&mut svm);
    let (pool_pda, _) = derive_pool_pda(&collateral_mint);
    let (collateral_vault_pda, _) = derive_collateral_vault_pda(&collateral_mint);
    let (borrow_vault_pda, _) = derive_borrow_vault_pda(&collateral_mint);

    // Initialize pool
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(1000),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collateral_mint),
            writable_meta(borrow_mint),
            writable_meta(pool_pda),
            writable_meta(collateral_vault_pda),
            writable_meta(borrow_vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Setup user
    let (user, user_collateral_token) = setup_user_with_tokens(&mut svm, &collateral_mint, 10000);
    let (user_position_pda, _) = derive_user_position_pda(&collateral_mint, &user.pubkey());

    // Create position and deposit
    svm.expire_blockhash();
    let create_position_ix = anchor_instruction(
        program_id,
        "create_user_position",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_position_ix, &user, &[&user]).unwrap();

    svm.expire_blockhash();
    let deposit_ix = anchor_instruction(
        program_id,
        "deposit_collateral",
        &amount_data(5000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_collateral_token),
            writable_meta(collateral_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, deposit_ix, &user, &[&user]).unwrap();

    // Withdraw all collateral (no debt)
    svm.expire_blockhash();
    let withdraw_ix = anchor_instruction(
        program_id,
        "withdraw_collateral",
        &amount_data(5000),
        vec![
            signer_meta(user.pubkey()),            // user
            writable_meta(pool_pda),               // pool
            writable_meta(user_position_pda),      // user_position
            writable_meta(user_collateral_token),  // user_collateral_token
            writable_meta(collateral_vault_pda),   // collateral_vault
            writable_meta(collateral_mint),        // collateral_mint
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(token_program_id()),     // token_program
        ],
    );

    let result = execute_tx(&mut svm, withdraw_ix, &user, &[&user]);
    assert!(result.is_ok(), "withdraw_collateral should succeed: {:?}", result);

    // Verify user position collateral is zero
    let position_account = svm.get_account(&user_position_pda).unwrap();
    assert_eq!(read_position_collateral_deposited(&position_account.data), 0);

    // Verify user received collateral back
    let user_token_account = svm.get_account(&user_collateral_token).unwrap();
    assert_eq!(read_token_balance(&user_token_account.data), 10000);
}

#[test]
fn test_withdraw_collateral_with_debt_maintains_ratio() {
    let mut svm = load_lending_program();
    let program_id = lending_program_id();

    // Setup
    let (_, collateral_mint) = setup_mint(&mut svm, 6);
    let (_, borrow_mint) = setup_mint(&mut svm, 6);
    let authority = funded_keypair_10_sol(&mut svm);
    let (pool_pda, _) = derive_pool_pda(&collateral_mint);
    let (collateral_vault_pda, _) = derive_collateral_vault_pda(&collateral_mint);
    let (borrow_vault_pda, _) = derive_borrow_vault_pda(&collateral_mint);

    // Initialize pool with zero interest for simpler testing
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(0),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collateral_mint),
            writable_meta(borrow_mint),
            writable_meta(pool_pda),
            writable_meta(collateral_vault_pda),
            writable_meta(borrow_vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Add liquidity to borrow vault
    let borrow_vault_account = svm.get_account(&borrow_vault_pda).unwrap();
    let mut vault_with_liquidity = create_token_account(&pool_pda, &borrow_mint, 100000);
    vault_with_liquidity.lamports = borrow_vault_account.lamports;
    svm.set_account(borrow_vault_pda, vault_with_liquidity).unwrap();

    // Setup user
    let (user, user_collateral_token) = setup_user_with_tokens(&mut svm, &collateral_mint, 20000);
    let user_borrow_token = get_ata(&user.pubkey(), &borrow_mint);
    svm.set_account(user_borrow_token, create_token_account(&user.pubkey(), &borrow_mint, 0)).unwrap();

    let (user_position_pda, _) = derive_user_position_pda(&collateral_mint, &user.pubkey());

    // Create position, deposit 18000 collateral, borrow 5000
    // Required collateral at 150% ratio: 5000 * 1.5 = 7500
    // Can withdraw up to: 18000 - 7500 = 10500
    svm.expire_blockhash();
    let create_position_ix = anchor_instruction(
        program_id,
        "create_user_position",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_position_ix, &user, &[&user]).unwrap();

    svm.expire_blockhash();
    let deposit_ix = anchor_instruction(
        program_id,
        "deposit_collateral",
        &amount_data(18000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_collateral_token),
            writable_meta(collateral_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, deposit_ix, &user, &[&user]).unwrap();

    svm.expire_blockhash();
    let borrow_ix = anchor_instruction(
        program_id,
        "borrow",
        &amount_data(5000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_borrow_token),
            writable_meta(borrow_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, borrow_ix, &user, &[&user]).unwrap();

    // Withdraw 10000 collateral (should succeed - leaves 8000 > 7500 required)
    svm.expire_blockhash();
    let withdraw_ix = anchor_instruction(
        program_id,
        "withdraw_collateral",
        &amount_data(10000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_collateral_token),
            writable_meta(collateral_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, withdraw_ix, &user, &[&user]);
    assert!(result.is_ok(), "withdraw_collateral should succeed when ratio maintained: {:?}", result);

    // Verify collateral decreased
    let position_account = svm.get_account(&user_position_pda).unwrap();
    assert_eq!(read_position_collateral_deposited(&position_account.data), 8000);
}

#[test]
fn test_withdraw_collateral_breaks_ratio_fails() {
    let mut svm = load_lending_program();
    let program_id = lending_program_id();

    // Setup
    let (_, collateral_mint) = setup_mint(&mut svm, 6);
    let (_, borrow_mint) = setup_mint(&mut svm, 6);
    let authority = funded_keypair_10_sol(&mut svm);
    let (pool_pda, _) = derive_pool_pda(&collateral_mint);
    let (collateral_vault_pda, _) = derive_collateral_vault_pda(&collateral_mint);
    let (borrow_vault_pda, _) = derive_borrow_vault_pda(&collateral_mint);

    // Initialize pool with zero interest
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(0),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collateral_mint),
            writable_meta(borrow_mint),
            writable_meta(pool_pda),
            writable_meta(collateral_vault_pda),
            writable_meta(borrow_vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Add liquidity
    let borrow_vault_account = svm.get_account(&borrow_vault_pda).unwrap();
    let mut vault_with_liquidity = create_token_account(&pool_pda, &borrow_mint, 100000);
    vault_with_liquidity.lamports = borrow_vault_account.lamports;
    svm.set_account(borrow_vault_pda, vault_with_liquidity).unwrap();

    // Setup user
    let (user, user_collateral_token) = setup_user_with_tokens(&mut svm, &collateral_mint, 15000);
    let user_borrow_token = get_ata(&user.pubkey(), &borrow_mint);
    svm.set_account(user_borrow_token, create_token_account(&user.pubkey(), &borrow_mint, 0)).unwrap();

    let (user_position_pda, _) = derive_user_position_pda(&collateral_mint, &user.pubkey());

    // Create position, deposit, borrow
    svm.expire_blockhash();
    let create_position_ix = anchor_instruction(
        program_id,
        "create_user_position",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_position_ix, &user, &[&user]).unwrap();

    svm.expire_blockhash();
    let deposit_ix = anchor_instruction(
        program_id,
        "deposit_collateral",
        &amount_data(15000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_collateral_token),
            writable_meta(collateral_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, deposit_ix, &user, &[&user]).unwrap();

    svm.expire_blockhash();
    let borrow_ix = anchor_instruction(
        program_id,
        "borrow",
        &amount_data(9000), // Max borrowable is 10000, borrow 9000 for some margin
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_borrow_token),
            writable_meta(borrow_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, borrow_ix, &user, &[&user]).unwrap();

    // Try to withdraw 3000 collateral (would leave 12000, but need 13500 for 9000 debt)
    svm.expire_blockhash();
    let withdraw_ix = anchor_instruction(
        program_id,
        "withdraw_collateral",
        &amount_data(3000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_collateral_token),
            writable_meta(collateral_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, withdraw_ix, &user, &[&user]);
    assert!(result.is_err(), "withdraw_collateral breaking ratio should fail");
}

// =============================================================================
// INTEREST ACCRUAL TESTS
// =============================================================================

#[test]
fn test_interest_accrual_on_borrow() {
    let mut svm = load_lending_program();
    let program_id = lending_program_id();

    // Setup
    let (_, collateral_mint) = setup_mint(&mut svm, 6);
    let (_, borrow_mint) = setup_mint(&mut svm, 6);
    let authority = funded_keypair_10_sol(&mut svm);
    let (pool_pda, _) = derive_pool_pda(&collateral_mint);
    let (collateral_vault_pda, _) = derive_collateral_vault_pda(&collateral_mint);
    let (borrow_vault_pda, _) = derive_borrow_vault_pda(&collateral_mint);

    // Interest rate: 100_000 = 0.1 tokens per borrowed token per slot
    let interest_rate: u64 = 100_000;

    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(interest_rate),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collateral_mint),
            writable_meta(borrow_mint),
            writable_meta(pool_pda),
            writable_meta(collateral_vault_pda),
            writable_meta(borrow_vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Add liquidity
    let borrow_vault_account = svm.get_account(&borrow_vault_pda).unwrap();
    let mut vault_with_liquidity = create_token_account(&pool_pda, &borrow_mint, 100000);
    vault_with_liquidity.lamports = borrow_vault_account.lamports;
    svm.set_account(borrow_vault_pda, vault_with_liquidity).unwrap();

    // Setup user with lots of collateral (200000 to cover interest accrual)
    let (user, user_collateral_token) = setup_user_with_tokens(&mut svm, &collateral_mint, 200000);
    let user_borrow_token = get_ata(&user.pubkey(), &borrow_mint);
    svm.set_account(user_borrow_token, create_token_account(&user.pubkey(), &borrow_mint, 0)).unwrap();

    let (user_position_pda, _) = derive_user_position_pda(&collateral_mint, &user.pubkey());

    // Create position and deposit
    svm.expire_blockhash();
    let create_position_ix = anchor_instruction(
        program_id,
        "create_user_position",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_position_ix, &user, &[&user]).unwrap();

    svm.expire_blockhash();
    let deposit_ix = anchor_instruction(
        program_id,
        "deposit_collateral",
        &amount_data(200000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_collateral_token),
            writable_meta(collateral_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, deposit_ix, &user, &[&user]).unwrap();

    // First borrow: 1000 tokens
    svm.expire_blockhash();
    let borrow_ix = anchor_instruction(
        program_id,
        "borrow",
        &amount_data(1000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_borrow_token),
            writable_meta(borrow_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, borrow_ix, &user, &[&user]).unwrap();

    // Verify initial debt
    let position_account = svm.get_account(&user_position_pda).unwrap();
    assert_eq!(read_position_borrowed_amount(&position_account.data), 1000);

    // Warp forward 100 slots
    // Expected interest: 1000 * 100_000 * 100 / 1_000_000 = 10_000
    svm.warp_to_slot(100);
    svm.expire_blockhash();

    // Borrow more tokens - this should accrue interest first
    let borrow_ix2 = anchor_instruction(
        program_id,
        "borrow",
        &amount_data(500),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_borrow_token),
            writable_meta(borrow_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, borrow_ix2, &user, &[&user]);
    assert!(result.is_ok(), "second borrow should succeed: {:?}", result);

    // Verify new debt includes interest: 1000 + 10000 (interest) + 500 = 11500
    let position_account = svm.get_account(&user_position_pda).unwrap();
    let borrowed_amount = read_position_borrowed_amount(&position_account.data);

    // Interest should have been accrued
    assert!(borrowed_amount > 1500, "Debt should include accrued interest, got {}", borrowed_amount);
    // With 100_000 interest rate over 100 slots on 1000 principal:
    // interest = 1000 * 100_000 * 100 / 1_000_000 = 10_000
    // Total should be around 11500
    assert_eq!(borrowed_amount, 11500, "Debt should be 1000 + 10000 interest + 500 new borrow");
}

// =============================================================================
// ERROR CASE TESTS
// =============================================================================

#[test]
fn test_deposit_zero_amount_fails() {
    let mut svm = load_lending_program();
    let program_id = lending_program_id();

    // Setup
    let (_, collateral_mint) = setup_mint(&mut svm, 6);
    let (_, borrow_mint) = setup_mint(&mut svm, 6);
    let authority = funded_keypair_10_sol(&mut svm);
    let (pool_pda, _) = derive_pool_pda(&collateral_mint);
    let (collateral_vault_pda, _) = derive_collateral_vault_pda(&collateral_mint);
    let (borrow_vault_pda, _) = derive_borrow_vault_pda(&collateral_mint);

    // Initialize pool
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(1000),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collateral_mint),
            writable_meta(borrow_mint),
            writable_meta(pool_pda),
            writable_meta(collateral_vault_pda),
            writable_meta(borrow_vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Setup user
    let (user, user_collateral_token) = setup_user_with_tokens(&mut svm, &collateral_mint, 10000);
    let (user_position_pda, _) = derive_user_position_pda(&collateral_mint, &user.pubkey());

    // Create position
    svm.expire_blockhash();
    let create_position_ix = anchor_instruction(
        program_id,
        "create_user_position",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_position_ix, &user, &[&user]).unwrap();

    // Try to deposit zero
    svm.expire_blockhash();
    let deposit_ix = anchor_instruction(
        program_id,
        "deposit_collateral",
        &amount_data(0),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_collateral_token),
            writable_meta(collateral_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, deposit_ix, &user, &[&user]);
    assert!(result.is_err(), "deposit zero amount should fail");
}

#[test]
fn test_borrow_insufficient_liquidity_fails() {
    let mut svm = load_lending_program();
    let program_id = lending_program_id();

    // Setup
    let (_, collateral_mint) = setup_mint(&mut svm, 6);
    let (_, borrow_mint) = setup_mint(&mut svm, 6);
    let authority = funded_keypair_10_sol(&mut svm);
    let (pool_pda, _) = derive_pool_pda(&collateral_mint);
    let (collateral_vault_pda, _) = derive_collateral_vault_pda(&collateral_mint);
    let (borrow_vault_pda, _) = derive_borrow_vault_pda(&collateral_mint);

    // Initialize pool (borrow vault has NO liquidity)
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(0),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collateral_mint),
            writable_meta(borrow_mint),
            writable_meta(pool_pda),
            writable_meta(collateral_vault_pda),
            writable_meta(borrow_vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Setup user
    let (user, user_collateral_token) = setup_user_with_tokens(&mut svm, &collateral_mint, 15000);
    let user_borrow_token = get_ata(&user.pubkey(), &borrow_mint);
    svm.set_account(user_borrow_token, create_token_account(&user.pubkey(), &borrow_mint, 0)).unwrap();

    let (user_position_pda, _) = derive_user_position_pda(&collateral_mint, &user.pubkey());

    // Create position and deposit
    svm.expire_blockhash();
    let create_position_ix = anchor_instruction(
        program_id,
        "create_user_position",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_position_ix, &user, &[&user]).unwrap();

    svm.expire_blockhash();
    let deposit_ix = anchor_instruction(
        program_id,
        "deposit_collateral",
        &amount_data(15000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_collateral_token),
            writable_meta(collateral_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, deposit_ix, &user, &[&user]).unwrap();

    // Try to borrow when vault has no liquidity
    svm.expire_blockhash();
    let borrow_ix = anchor_instruction(
        program_id,
        "borrow",
        &amount_data(1000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_borrow_token),
            writable_meta(borrow_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, borrow_ix, &user, &[&user]);
    assert!(result.is_err(), "borrow with insufficient liquidity should fail");
}

// =============================================================================
// FULL WORKFLOW TEST
// =============================================================================

#[test]
fn test_full_lending_workflow() {
    let mut svm = load_lending_program();
    let program_id = lending_program_id();

    // Setup
    let (_, collateral_mint) = setup_mint(&mut svm, 6);
    let (_, borrow_mint) = setup_mint(&mut svm, 6);
    let authority = funded_keypair_10_sol(&mut svm);
    let (pool_pda, _) = derive_pool_pda(&collateral_mint);
    let (collateral_vault_pda, _) = derive_collateral_vault_pda(&collateral_mint);
    let (borrow_vault_pda, _) = derive_borrow_vault_pda(&collateral_mint);

    // Initialize pool with zero interest for simpler verification
    let init_ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(0),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(collateral_mint),
            writable_meta(borrow_mint),
            writable_meta(pool_pda),
            writable_meta(collateral_vault_pda),
            writable_meta(borrow_vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, init_ix, &authority, &[&authority]).unwrap();

    // Add liquidity
    let borrow_vault_account = svm.get_account(&borrow_vault_pda).unwrap();
    let mut vault_with_liquidity = create_token_account(&pool_pda, &borrow_mint, 50000);
    vault_with_liquidity.lamports = borrow_vault_account.lamports;
    svm.set_account(borrow_vault_pda, vault_with_liquidity).unwrap();

    // Setup user
    let (user, user_collateral_token) = setup_user_with_tokens(&mut svm, &collateral_mint, 30000);
    let user_borrow_token = get_ata(&user.pubkey(), &borrow_mint);
    svm.set_account(user_borrow_token, create_token_account(&user.pubkey(), &borrow_mint, 0)).unwrap();

    let (user_position_pda, _) = derive_user_position_pda(&collateral_mint, &user.pubkey());

    // 1. Create position
    svm.expire_blockhash();
    let create_position_ix = anchor_instruction(
        program_id,
        "create_user_position",
        &[],
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
        ],
    );
    execute_tx(&mut svm, create_position_ix, &user, &[&user]).unwrap();

    // 2. Deposit 15000 collateral
    svm.expire_blockhash();
    let deposit_ix = anchor_instruction(
        program_id,
        "deposit_collateral",
        &amount_data(15000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_collateral_token),
            writable_meta(collateral_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, deposit_ix, &user, &[&user]).unwrap();

    // Verify: collateral deposited
    let position = svm.get_account(&user_position_pda).unwrap();
    assert_eq!(read_position_collateral_deposited(&position.data), 15000);

    // 3. Borrow 5000 tokens
    svm.expire_blockhash();
    let borrow_ix = anchor_instruction(
        program_id,
        "borrow",
        &amount_data(5000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_borrow_token),
            writable_meta(borrow_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, borrow_ix, &user, &[&user]).unwrap();

    // Verify: user has borrowed tokens
    let user_token = svm.get_account(&user_borrow_token).unwrap();
    assert_eq!(read_token_balance(&user_token.data), 5000);

    // 4. Repay 2000 tokens
    svm.expire_blockhash();
    let repay_ix = anchor_instruction(
        program_id,
        "repay",
        &amount_data(2000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_borrow_token),
            writable_meta(borrow_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, repay_ix, &user, &[&user]).unwrap();

    // Verify: debt reduced
    let position = svm.get_account(&user_position_pda).unwrap();
    assert_eq!(read_position_borrowed_amount(&position.data), 3000);

    // 5. Repay remaining debt
    svm.expire_blockhash();
    let repay_ix2 = anchor_instruction(
        program_id,
        "repay",
        &amount_data(3000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_borrow_token),
            writable_meta(borrow_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, repay_ix2, &user, &[&user]).unwrap();

    // Verify: debt is zero
    let position = svm.get_account(&user_position_pda).unwrap();
    assert_eq!(read_position_borrowed_amount(&position.data), 0);

    // 6. Withdraw all collateral
    svm.expire_blockhash();
    let withdraw_ix = anchor_instruction(
        program_id,
        "withdraw_collateral",
        &amount_data(15000),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(pool_pda),
            writable_meta(user_position_pda),
            writable_meta(user_collateral_token),
            writable_meta(collateral_vault_pda),
            writable_meta(collateral_mint),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, withdraw_ix, &user, &[&user]).unwrap();

    // Verify: user got collateral back
    let user_collateral = svm.get_account(&user_collateral_token).unwrap();
    assert_eq!(read_token_balance(&user_collateral.data), 30000); // Original amount

    // Verify: position is cleared
    let position = svm.get_account(&user_position_pda).unwrap();
    assert_eq!(read_position_collateral_deposited(&position.data), 0);
    assert_eq!(read_position_borrowed_amount(&position.data), 0);
}
