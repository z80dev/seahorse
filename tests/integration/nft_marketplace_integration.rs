//! LiteSVM integration tests for the Seahorse NFT Marketplace program
//!
//! These tests verify NFT marketplace functionality:
//! - list_nft: Creates listing PDA and escrows NFT in PDA-owned vault
//! - buy_nft: Buyer pays seller, NFT transferred from escrow to buyer
//! - delist_nft: Seller cancels listing, NFT returned from escrow
//! - update_price: Seller changes listing price while active
//!
//! Uses SPL tokens for payment (not SOL) because Seahorse supports
//! PDA-signed token transfers but not PDA-signed lamport transfers.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile nft_marketplace.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// NFT Marketplace program ID (from declare_id!)
fn nft_marketplace_program_id() -> Pubkey {
    Pubkey::from_str("Coa8ZSxePf7ZCsDNHE9SRHyZJRWWnZgmanuKVf4ZBM7Q").unwrap()
}

/// Listing account size:
/// discriminator (8) + seller (32) + listing_id (8) + nft_mint (32) + payment_mint (32)
/// + price (8) + is_active (1) + bump (1) = 122 bytes
const LISTING_ACCOUNT_SIZE: usize = 8 + 32 + 8 + 32 + 32 + 8 + 1 + 1;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the NFT marketplace program into LiteSVM
fn load_nft_marketplace_program() -> litesvm::LiteSVM {
    let program_id = nft_marketplace_program_id();
    let program_bytes = std::fs::read("../../target/deploy/nft_marketplace.so")
        .expect("Failed to read nft_marketplace.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);
    svm
}

/// Setup an NFT mint (supply=1, decimals=0)
fn setup_nft_mint(svm: &mut litesvm::LiteSVM) -> (Keypair, Pubkey) {
    let mint_authority = funded_keypair_10_sol(svm);
    let nft_mint = Keypair::new();

    let mint_account = create_mint_account(&mint_authority.pubkey(), 0); // decimals=0 for NFT
    svm.set_account(nft_mint.pubkey(), mint_account).unwrap();

    (mint_authority, nft_mint.pubkey())
}

/// Setup a payment token mint
fn setup_payment_mint(svm: &mut litesvm::LiteSVM, decimals: u8) -> (Keypair, Pubkey) {
    let mint_authority = funded_keypair_10_sol(svm);
    let mint = Keypair::new();

    let mint_account = create_mint_account(&mint_authority.pubkey(), decimals);
    svm.set_account(mint.pubkey(), mint_account).unwrap();

    (mint_authority, mint.pubkey())
}

/// Setup a user with NFT (amount=1) and create token account
fn setup_user_with_nft(
    svm: &mut litesvm::LiteSVM,
    nft_mint: &Pubkey,
) -> (Keypair, Pubkey) {
    let user = funded_keypair_10_sol(svm);
    let user_nft_account = get_ata(&user.pubkey(), nft_mint);

    // User owns 1 NFT
    let token_account = create_token_account(&user.pubkey(), nft_mint, 1);
    svm.set_account(user_nft_account, token_account).unwrap();

    (user, user_nft_account)
}

/// Setup a user with payment tokens
fn setup_user_with_payment_tokens(
    svm: &mut litesvm::LiteSVM,
    payment_mint: &Pubkey,
    amount: u64,
) -> (Keypair, Pubkey) {
    let user = funded_keypair_10_sol(svm);
    let user_payment_account = get_ata(&user.pubkey(), payment_mint);

    let token_account = create_token_account(&user.pubkey(), payment_mint, amount);
    svm.set_account(user_payment_account, token_account).unwrap();

    (user, user_payment_account)
}

/// Create a token account for a user that doesn't own any tokens yet
fn create_empty_token_account_for_user(
    svm: &mut litesvm::LiteSVM,
    user: &Pubkey,
    mint: &Pubkey,
) -> Pubkey {
    let ata = get_ata(user, mint);
    let token_account = create_token_account(user, mint, 0);
    svm.set_account(ata, token_account).unwrap();
    ata
}

/// Derive listing PDA
fn derive_listing_pda(listing_id: u64) -> (Pubkey, u8) {
    find_pda(
        &[b"listing", &listing_id.to_le_bytes()],
        &nft_marketplace_program_id(),
    )
}

/// Derive NFT escrow PDA
fn derive_nft_escrow_pda(listing_id: u64) -> (Pubkey, u8) {
    find_pda(
        &[b"nft_escrow", &listing_id.to_le_bytes()],
        &nft_marketplace_program_id(),
    )
}

/// Read listing account fields
fn read_listing_seller(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

fn read_listing_id(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[40..48].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_listing_nft_mint(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[48..80].try_into().unwrap())
}

fn read_listing_payment_mint(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[80..112].try_into().unwrap())
}

fn read_listing_price(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[112..120].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_listing_is_active(data: &[u8]) -> bool {
    data[120] != 0
}

fn read_listing_bump(data: &[u8]) -> u8 {
    data[121]
}

/// Build list_nft instruction data
fn list_nft_data(listing_id: u64, price: u64) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&listing_id.to_le_bytes());
    data.extend_from_slice(&price.to_le_bytes());
    data
}

/// Build update_price instruction data
fn update_price_data(new_price: u64) -> Vec<u8> {
    new_price.to_le_bytes().to_vec()
}

// =============================================================================
// LIST NFT TESTS
// =============================================================================

#[test]
fn test_list_nft() {
    let mut svm = load_nft_marketplace_program();
    let program_id = nft_marketplace_program_id();

    // Setup NFT and payment mints
    let (_, nft_mint) = setup_nft_mint(&mut svm);
    let (_, payment_mint) = setup_payment_mint(&mut svm, 6);

    // Setup seller with NFT
    let (seller, seller_nft_token) = setup_user_with_nft(&mut svm, &nft_mint);

    let listing_id: u64 = 1;
    let price: u64 = 1000;

    // Derive PDAs
    let (listing_pda, _) = derive_listing_pda(listing_id);
    let (nft_escrow_pda, _) = derive_nft_escrow_pda(listing_id);

    // Build list_nft instruction (Seahorse account order from compiled output)
    let ix = anchor_instruction(
        program_id,
        "list_nft",
        &list_nft_data(listing_id, price),
        vec![
            signer_meta(seller.pubkey()),           // seller
            writable_meta(nft_mint),                 // nft_mint
            writable_meta(payment_mint),             // payment_mint
            writable_meta(listing_pda),              // listing
            writable_meta(nft_escrow_pda),           // nft_escrow
            writable_meta(seller_nft_token),         // seller_nft_token
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),  // rent
            readonly_meta(system_program::id()),    // system_program
            readonly_meta(token_program_id()),      // token_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &seller, &[&seller]);
    assert!(result.is_ok(), "list_nft should succeed: {:?}", result);

    // Verify listing account state
    let listing_account = svm.get_account(&listing_pda).expect("Listing should exist");
    assert_eq!(listing_account.owner, program_id);

    assert_eq!(read_listing_seller(&listing_account.data), seller.pubkey());
    assert_eq!(read_listing_id(&listing_account.data), listing_id);
    assert_eq!(read_listing_nft_mint(&listing_account.data), nft_mint);
    assert_eq!(read_listing_payment_mint(&listing_account.data), payment_mint);
    assert_eq!(read_listing_price(&listing_account.data), price);
    assert!(read_listing_is_active(&listing_account.data));

    // Verify NFT escrowed
    let escrow_account = svm.get_account(&nft_escrow_pda).expect("NFT escrow should exist");
    assert_eq!(read_token_balance(&escrow_account.data), 1);

    // Verify seller no longer has NFT
    let seller_nft_account = svm.get_account(&seller_nft_token).unwrap();
    assert_eq!(read_token_balance(&seller_nft_account.data), 0);
}

#[test]
fn test_list_nft_different_listing_ids() {
    let mut svm = load_nft_marketplace_program();
    let program_id = nft_marketplace_program_id();

    let (_, nft_mint_1) = setup_nft_mint(&mut svm);
    let (_, nft_mint_2) = setup_nft_mint(&mut svm);
    let (_, payment_mint) = setup_payment_mint(&mut svm, 6);

    // Seller 1 with NFT 1
    let (seller1, seller1_nft_token) = setup_user_with_nft(&mut svm, &nft_mint_1);
    // Seller 2 with NFT 2
    let (seller2, seller2_nft_token) = setup_user_with_nft(&mut svm, &nft_mint_2);

    // List first NFT
    let (listing_pda_1, _) = derive_listing_pda(1);
    let (nft_escrow_pda_1, _) = derive_nft_escrow_pda(1);

    let ix1 = anchor_instruction(
        program_id,
        "list_nft",
        &list_nft_data(1, 1000),
        vec![
            signer_meta(seller1.pubkey()),
            writable_meta(nft_mint_1),
            writable_meta(payment_mint),
            writable_meta(listing_pda_1),
            writable_meta(nft_escrow_pda_1),
            writable_meta(seller1_nft_token),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix1, &seller1, &[&seller1]).unwrap();

    // List second NFT
    svm.expire_blockhash();
    let (listing_pda_2, _) = derive_listing_pda(2);
    let (nft_escrow_pda_2, _) = derive_nft_escrow_pda(2);

    let ix2 = anchor_instruction(
        program_id,
        "list_nft",
        &list_nft_data(2, 2000),
        vec![
            signer_meta(seller2.pubkey()),
            writable_meta(nft_mint_2),
            writable_meta(payment_mint),
            writable_meta(listing_pda_2),
            writable_meta(nft_escrow_pda_2),
            writable_meta(seller2_nft_token),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix2, &seller2, &[&seller2]).unwrap();

    // Verify both listings exist with correct data
    let listing_1 = svm.get_account(&listing_pda_1).unwrap();
    let listing_2 = svm.get_account(&listing_pda_2).unwrap();

    assert_eq!(read_listing_id(&listing_1.data), 1);
    assert_eq!(read_listing_id(&listing_2.data), 2);
    assert_eq!(read_listing_price(&listing_1.data), 1000);
    assert_eq!(read_listing_price(&listing_2.data), 2000);
    assert_eq!(read_listing_nft_mint(&listing_1.data), nft_mint_1);
    assert_eq!(read_listing_nft_mint(&listing_2.data), nft_mint_2);
}

#[test]
fn test_list_nft_zero_price_fails() {
    let mut svm = load_nft_marketplace_program();
    let program_id = nft_marketplace_program_id();

    let (_, nft_mint) = setup_nft_mint(&mut svm);
    let (_, payment_mint) = setup_payment_mint(&mut svm, 6);
    let (seller, seller_nft_token) = setup_user_with_nft(&mut svm, &nft_mint);

    let (listing_pda, _) = derive_listing_pda(1);
    let (nft_escrow_pda, _) = derive_nft_escrow_pda(1);

    let ix = anchor_instruction(
        program_id,
        "list_nft",
        &list_nft_data(1, 0), // Zero price
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(nft_mint),
            writable_meta(payment_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(seller_nft_token),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &seller, &[&seller]);
    assert!(result.is_err(), "list_nft with zero price should fail");
}

#[test]
fn test_list_nft_seller_doesnt_own_nft_fails() {
    let mut svm = load_nft_marketplace_program();
    let program_id = nft_marketplace_program_id();

    let (_, nft_mint) = setup_nft_mint(&mut svm);
    let (_, payment_mint) = setup_payment_mint(&mut svm, 6);

    // Create seller without NFT (empty token account)
    let seller = funded_keypair_10_sol(&mut svm);
    let seller_nft_token = get_ata(&seller.pubkey(), &nft_mint);
    let token_account = create_token_account(&seller.pubkey(), &nft_mint, 0); // 0 NFTs
    svm.set_account(seller_nft_token, token_account).unwrap();

    let (listing_pda, _) = derive_listing_pda(1);
    let (nft_escrow_pda, _) = derive_nft_escrow_pda(1);

    let ix = anchor_instruction(
        program_id,
        "list_nft",
        &list_nft_data(1, 1000),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(nft_mint),
            writable_meta(payment_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(seller_nft_token),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &seller, &[&seller]);
    assert!(result.is_err(), "list_nft by non-owner should fail");
}

// =============================================================================
// BUY NFT TESTS
// =============================================================================

#[test]
fn test_buy_nft() {
    let mut svm = load_nft_marketplace_program();
    let program_id = nft_marketplace_program_id();

    let (_, nft_mint) = setup_nft_mint(&mut svm);
    let (_, payment_mint) = setup_payment_mint(&mut svm, 6);

    let (seller, seller_nft_token) = setup_user_with_nft(&mut svm, &nft_mint);
    let (buyer, buyer_payment_token) = setup_user_with_payment_tokens(&mut svm, &payment_mint, 10000);

    // Create buyer's NFT token account (empty) and seller's payment token account (empty)
    let buyer_nft_token = create_empty_token_account_for_user(&mut svm, &buyer.pubkey(), &nft_mint);
    let seller_payment_token = create_empty_token_account_for_user(&mut svm, &seller.pubkey(), &payment_mint);

    let listing_id: u64 = 1;
    let price: u64 = 500;

    let (listing_pda, _) = derive_listing_pda(listing_id);
    let (nft_escrow_pda, _) = derive_nft_escrow_pda(listing_id);

    // List the NFT
    let list_ix = anchor_instruction(
        program_id,
        "list_nft",
        &list_nft_data(listing_id, price),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(nft_mint),
            writable_meta(payment_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(seller_nft_token),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, list_ix, &seller, &[&seller]).unwrap();

    // Buy the NFT
    svm.expire_blockhash();
    let buy_ix = anchor_instruction(
        program_id,
        "buy_nft",
        &[], // No instruction data
        vec![
            signer_meta(buyer.pubkey()),            // buyer
            writable_meta(seller.pubkey()),         // seller
            writable_meta(nft_mint),                 // nft_mint
            writable_meta(payment_mint),             // payment_mint
            writable_meta(listing_pda),              // listing
            writable_meta(nft_escrow_pda),           // nft_escrow
            writable_meta(buyer_nft_token),          // buyer_nft_token
            writable_meta(buyer_payment_token),      // buyer_payment_token
            writable_meta(seller_payment_token),     // seller_payment_token
            readonly_meta(token_program_id()),      // token_program
        ],
    );

    let result = execute_tx(&mut svm, buy_ix, &buyer, &[&buyer]);
    assert!(result.is_ok(), "buy_nft should succeed: {:?}", result);

    // Verify buyer received NFT
    let buyer_nft_account = svm.get_account(&buyer_nft_token).unwrap();
    assert_eq!(read_token_balance(&buyer_nft_account.data), 1);

    // Verify seller received payment
    let seller_payment_account = svm.get_account(&seller_payment_token).unwrap();
    assert_eq!(read_token_balance(&seller_payment_account.data), price);

    // Verify buyer's payment decreased
    let buyer_payment_account = svm.get_account(&buyer_payment_token).unwrap();
    assert_eq!(read_token_balance(&buyer_payment_account.data), 10000 - price);

    // Verify escrow is empty
    let escrow_account = svm.get_account(&nft_escrow_pda).unwrap();
    assert_eq!(read_token_balance(&escrow_account.data), 0);

    // Verify listing is no longer active
    let listing_account = svm.get_account(&listing_pda).unwrap();
    assert!(!read_listing_is_active(&listing_account.data));
}

#[test]
fn test_buy_nft_insufficient_payment_fails() {
    let mut svm = load_nft_marketplace_program();
    let program_id = nft_marketplace_program_id();

    let (_, nft_mint) = setup_nft_mint(&mut svm);
    let (_, payment_mint) = setup_payment_mint(&mut svm, 6);

    let (seller, seller_nft_token) = setup_user_with_nft(&mut svm, &nft_mint);
    // Buyer only has 100 tokens, price is 500
    let (buyer, buyer_payment_token) = setup_user_with_payment_tokens(&mut svm, &payment_mint, 100);

    let buyer_nft_token = create_empty_token_account_for_user(&mut svm, &buyer.pubkey(), &nft_mint);
    let seller_payment_token = create_empty_token_account_for_user(&mut svm, &seller.pubkey(), &payment_mint);

    let listing_id: u64 = 1;
    let price: u64 = 500;

    let (listing_pda, _) = derive_listing_pda(listing_id);
    let (nft_escrow_pda, _) = derive_nft_escrow_pda(listing_id);

    // List the NFT
    let list_ix = anchor_instruction(
        program_id,
        "list_nft",
        &list_nft_data(listing_id, price),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(nft_mint),
            writable_meta(payment_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(seller_nft_token),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, list_ix, &seller, &[&seller]).unwrap();

    // Try to buy with insufficient funds
    svm.expire_blockhash();
    let buy_ix = anchor_instruction(
        program_id,
        "buy_nft",
        &[],
        vec![
            signer_meta(buyer.pubkey()),
            writable_meta(seller.pubkey()),
            writable_meta(nft_mint),
            writable_meta(payment_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(buyer_nft_token),
            writable_meta(buyer_payment_token),
            writable_meta(seller_payment_token),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, buy_ix, &buyer, &[&buyer]);
    assert!(result.is_err(), "buy_nft with insufficient payment should fail");
}

#[test]
fn test_buy_nft_inactive_listing_fails() {
    let mut svm = load_nft_marketplace_program();
    let program_id = nft_marketplace_program_id();

    let (_, nft_mint) = setup_nft_mint(&mut svm);
    let (_, payment_mint) = setup_payment_mint(&mut svm, 6);

    let (seller, seller_nft_token) = setup_user_with_nft(&mut svm, &nft_mint);
    let (buyer, buyer_payment_token) = setup_user_with_payment_tokens(&mut svm, &payment_mint, 10000);

    let buyer_nft_token = create_empty_token_account_for_user(&mut svm, &buyer.pubkey(), &nft_mint);
    let seller_payment_token = create_empty_token_account_for_user(&mut svm, &seller.pubkey(), &payment_mint);

    let listing_id: u64 = 1;
    let price: u64 = 500;

    let (listing_pda, _) = derive_listing_pda(listing_id);
    let (nft_escrow_pda, _) = derive_nft_escrow_pda(listing_id);

    // List the NFT
    let list_ix = anchor_instruction(
        program_id,
        "list_nft",
        &list_nft_data(listing_id, price),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(nft_mint),
            writable_meta(payment_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(seller_nft_token),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, list_ix, &seller, &[&seller]).unwrap();

    // Delist the NFT
    svm.expire_blockhash();
    let delist_ix = anchor_instruction(
        program_id,
        "delist_nft",
        &[],
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(nft_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(seller_nft_token),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, delist_ix, &seller, &[&seller]).unwrap();

    // Try to buy delisted NFT
    svm.expire_blockhash();
    let buy_ix = anchor_instruction(
        program_id,
        "buy_nft",
        &[],
        vec![
            signer_meta(buyer.pubkey()),
            writable_meta(seller.pubkey()),
            writable_meta(nft_mint),
            writable_meta(payment_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(buyer_nft_token),
            writable_meta(buyer_payment_token),
            writable_meta(seller_payment_token),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, buy_ix, &buyer, &[&buyer]);
    assert!(result.is_err(), "buy_nft on inactive listing should fail");
}

// =============================================================================
// DELIST NFT TESTS
// =============================================================================

#[test]
fn test_delist_nft() {
    let mut svm = load_nft_marketplace_program();
    let program_id = nft_marketplace_program_id();

    let (_, nft_mint) = setup_nft_mint(&mut svm);
    let (_, payment_mint) = setup_payment_mint(&mut svm, 6);

    let (seller, seller_nft_token) = setup_user_with_nft(&mut svm, &nft_mint);

    let listing_id: u64 = 1;
    let price: u64 = 500;

    let (listing_pda, _) = derive_listing_pda(listing_id);
    let (nft_escrow_pda, _) = derive_nft_escrow_pda(listing_id);

    // List the NFT
    let list_ix = anchor_instruction(
        program_id,
        "list_nft",
        &list_nft_data(listing_id, price),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(nft_mint),
            writable_meta(payment_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(seller_nft_token),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, list_ix, &seller, &[&seller]).unwrap();

    // Verify NFT escrowed
    let escrow_before = svm.get_account(&nft_escrow_pda).unwrap();
    assert_eq!(read_token_balance(&escrow_before.data), 1);

    // Delist the NFT
    svm.expire_blockhash();
    let delist_ix = anchor_instruction(
        program_id,
        "delist_nft",
        &[],
        vec![
            signer_meta(seller.pubkey()),           // seller
            writable_meta(nft_mint),                 // nft_mint
            writable_meta(listing_pda),              // listing
            writable_meta(nft_escrow_pda),           // nft_escrow
            writable_meta(seller_nft_token),         // seller_nft_token
            readonly_meta(token_program_id()),      // token_program
        ],
    );

    let result = execute_tx(&mut svm, delist_ix, &seller, &[&seller]);
    assert!(result.is_ok(), "delist_nft should succeed: {:?}", result);

    // Verify NFT returned to seller
    let seller_nft_account = svm.get_account(&seller_nft_token).unwrap();
    assert_eq!(read_token_balance(&seller_nft_account.data), 1);

    // Verify escrow is empty
    let escrow_after = svm.get_account(&nft_escrow_pda).unwrap();
    assert_eq!(read_token_balance(&escrow_after.data), 0);

    // Verify listing is inactive
    let listing_account = svm.get_account(&listing_pda).unwrap();
    assert!(!read_listing_is_active(&listing_account.data));
}

#[test]
fn test_delist_nft_unauthorized_fails() {
    let mut svm = load_nft_marketplace_program();
    let program_id = nft_marketplace_program_id();

    let (_, nft_mint) = setup_nft_mint(&mut svm);
    let (_, payment_mint) = setup_payment_mint(&mut svm, 6);

    let (seller, seller_nft_token) = setup_user_with_nft(&mut svm, &nft_mint);
    let attacker = funded_keypair_10_sol(&mut svm);
    let attacker_nft_token = create_empty_token_account_for_user(&mut svm, &attacker.pubkey(), &nft_mint);

    let listing_id: u64 = 1;
    let (listing_pda, _) = derive_listing_pda(listing_id);
    let (nft_escrow_pda, _) = derive_nft_escrow_pda(listing_id);

    // List the NFT
    let list_ix = anchor_instruction(
        program_id,
        "list_nft",
        &list_nft_data(listing_id, 500),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(nft_mint),
            writable_meta(payment_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(seller_nft_token),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, list_ix, &seller, &[&seller]).unwrap();

    // Attacker tries to delist
    svm.expire_blockhash();
    let delist_ix = anchor_instruction(
        program_id,
        "delist_nft",
        &[],
        vec![
            signer_meta(attacker.pubkey()),         // Wrong seller!
            writable_meta(nft_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(attacker_nft_token),      // Attacker's token account
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, delist_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "delist_nft by non-seller should fail");
}

#[test]
fn test_delist_nft_inactive_listing_fails() {
    let mut svm = load_nft_marketplace_program();
    let program_id = nft_marketplace_program_id();

    let (_, nft_mint) = setup_nft_mint(&mut svm);
    let (_, payment_mint) = setup_payment_mint(&mut svm, 6);

    let (seller, seller_nft_token) = setup_user_with_nft(&mut svm, &nft_mint);

    let listing_id: u64 = 1;
    let (listing_pda, _) = derive_listing_pda(listing_id);
    let (nft_escrow_pda, _) = derive_nft_escrow_pda(listing_id);

    // List and then delist
    let list_ix = anchor_instruction(
        program_id,
        "list_nft",
        &list_nft_data(listing_id, 500),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(nft_mint),
            writable_meta(payment_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(seller_nft_token),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, list_ix, &seller, &[&seller]).unwrap();

    svm.expire_blockhash();
    let delist_ix = anchor_instruction(
        program_id,
        "delist_nft",
        &[],
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(nft_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(seller_nft_token),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, delist_ix, &seller, &[&seller]).unwrap();

    // Try to delist again
    svm.expire_blockhash();
    let delist_ix2 = anchor_instruction(
        program_id,
        "delist_nft",
        &[],
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(nft_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(seller_nft_token),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, delist_ix2, &seller, &[&seller]);
    assert!(result.is_err(), "delist_nft on inactive listing should fail");
}

// =============================================================================
// UPDATE PRICE TESTS
// =============================================================================

#[test]
fn test_update_price() {
    let mut svm = load_nft_marketplace_program();
    let program_id = nft_marketplace_program_id();

    let (_, nft_mint) = setup_nft_mint(&mut svm);
    let (_, payment_mint) = setup_payment_mint(&mut svm, 6);

    let (seller, seller_nft_token) = setup_user_with_nft(&mut svm, &nft_mint);

    let listing_id: u64 = 1;
    let initial_price: u64 = 500;
    let new_price: u64 = 750;

    let (listing_pda, _) = derive_listing_pda(listing_id);
    let (nft_escrow_pda, _) = derive_nft_escrow_pda(listing_id);

    // List the NFT
    let list_ix = anchor_instruction(
        program_id,
        "list_nft",
        &list_nft_data(listing_id, initial_price),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(nft_mint),
            writable_meta(payment_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(seller_nft_token),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, list_ix, &seller, &[&seller]).unwrap();

    // Verify initial price
    let listing_before = svm.get_account(&listing_pda).unwrap();
    assert_eq!(read_listing_price(&listing_before.data), initial_price);

    // Update price
    svm.expire_blockhash();
    let update_ix = anchor_instruction(
        program_id,
        "update_price",
        &update_price_data(new_price),
        vec![
            signer_meta(seller.pubkey()),           // seller
            writable_meta(listing_pda),              // listing
        ],
    );

    let result = execute_tx(&mut svm, update_ix, &seller, &[&seller]);
    assert!(result.is_ok(), "update_price should succeed: {:?}", result);

    // Verify price updated
    let listing_after = svm.get_account(&listing_pda).unwrap();
    assert_eq!(read_listing_price(&listing_after.data), new_price);
    assert!(read_listing_is_active(&listing_after.data)); // Still active
}

#[test]
fn test_update_price_unauthorized_fails() {
    let mut svm = load_nft_marketplace_program();
    let program_id = nft_marketplace_program_id();

    let (_, nft_mint) = setup_nft_mint(&mut svm);
    let (_, payment_mint) = setup_payment_mint(&mut svm, 6);

    let (seller, seller_nft_token) = setup_user_with_nft(&mut svm, &nft_mint);
    let attacker = funded_keypair_10_sol(&mut svm);

    let listing_id: u64 = 1;
    let (listing_pda, _) = derive_listing_pda(listing_id);
    let (nft_escrow_pda, _) = derive_nft_escrow_pda(listing_id);

    // List the NFT
    let list_ix = anchor_instruction(
        program_id,
        "list_nft",
        &list_nft_data(listing_id, 500),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(nft_mint),
            writable_meta(payment_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(seller_nft_token),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, list_ix, &seller, &[&seller]).unwrap();

    // Attacker tries to update price
    svm.expire_blockhash();
    let update_ix = anchor_instruction(
        program_id,
        "update_price",
        &update_price_data(1),
        vec![
            signer_meta(attacker.pubkey()),         // Wrong seller!
            writable_meta(listing_pda),
        ],
    );

    let result = execute_tx(&mut svm, update_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "update_price by non-seller should fail");
}

#[test]
fn test_update_price_zero_fails() {
    let mut svm = load_nft_marketplace_program();
    let program_id = nft_marketplace_program_id();

    let (_, nft_mint) = setup_nft_mint(&mut svm);
    let (_, payment_mint) = setup_payment_mint(&mut svm, 6);

    let (seller, seller_nft_token) = setup_user_with_nft(&mut svm, &nft_mint);

    let listing_id: u64 = 1;
    let (listing_pda, _) = derive_listing_pda(listing_id);
    let (nft_escrow_pda, _) = derive_nft_escrow_pda(listing_id);

    // List the NFT
    let list_ix = anchor_instruction(
        program_id,
        "list_nft",
        &list_nft_data(listing_id, 500),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(nft_mint),
            writable_meta(payment_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(seller_nft_token),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, list_ix, &seller, &[&seller]).unwrap();

    // Try to set price to zero
    svm.expire_blockhash();
    let update_ix = anchor_instruction(
        program_id,
        "update_price",
        &update_price_data(0),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(listing_pda),
        ],
    );

    let result = execute_tx(&mut svm, update_ix, &seller, &[&seller]);
    assert!(result.is_err(), "update_price to zero should fail");
}

#[test]
fn test_update_price_inactive_listing_fails() {
    let mut svm = load_nft_marketplace_program();
    let program_id = nft_marketplace_program_id();

    let (_, nft_mint) = setup_nft_mint(&mut svm);
    let (_, payment_mint) = setup_payment_mint(&mut svm, 6);

    let (seller, seller_nft_token) = setup_user_with_nft(&mut svm, &nft_mint);

    let listing_id: u64 = 1;
    let (listing_pda, _) = derive_listing_pda(listing_id);
    let (nft_escrow_pda, _) = derive_nft_escrow_pda(listing_id);

    // List and delist
    let list_ix = anchor_instruction(
        program_id,
        "list_nft",
        &list_nft_data(listing_id, 500),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(nft_mint),
            writable_meta(payment_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(seller_nft_token),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, list_ix, &seller, &[&seller]).unwrap();

    svm.expire_blockhash();
    let delist_ix = anchor_instruction(
        program_id,
        "delist_nft",
        &[],
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(nft_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(seller_nft_token),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, delist_ix, &seller, &[&seller]).unwrap();

    // Try to update price on inactive listing
    svm.expire_blockhash();
    let update_ix = anchor_instruction(
        program_id,
        "update_price",
        &update_price_data(750),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(listing_pda),
        ],
    );

    let result = execute_tx(&mut svm, update_ix, &seller, &[&seller]);
    assert!(result.is_err(), "update_price on inactive listing should fail");
}

// =============================================================================
// FULL WORKFLOW TESTS
// =============================================================================

#[test]
fn test_full_marketplace_workflow_buy() {
    let mut svm = load_nft_marketplace_program();
    let program_id = nft_marketplace_program_id();

    let (_, nft_mint) = setup_nft_mint(&mut svm);
    let (_, payment_mint) = setup_payment_mint(&mut svm, 6);

    let (seller, seller_nft_token) = setup_user_with_nft(&mut svm, &nft_mint);
    let (buyer, buyer_payment_token) = setup_user_with_payment_tokens(&mut svm, &payment_mint, 10000);

    let buyer_nft_token = create_empty_token_account_for_user(&mut svm, &buyer.pubkey(), &nft_mint);
    let seller_payment_token = create_empty_token_account_for_user(&mut svm, &seller.pubkey(), &payment_mint);

    let listing_id: u64 = 1;
    let initial_price: u64 = 500;
    let updated_price: u64 = 750;

    let (listing_pda, _) = derive_listing_pda(listing_id);
    let (nft_escrow_pda, _) = derive_nft_escrow_pda(listing_id);

    // 1. List the NFT
    let list_ix = anchor_instruction(
        program_id,
        "list_nft",
        &list_nft_data(listing_id, initial_price),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(nft_mint),
            writable_meta(payment_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(seller_nft_token),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, list_ix, &seller, &[&seller]).unwrap();

    // Verify listing state
    let listing = svm.get_account(&listing_pda).unwrap();
    assert!(read_listing_is_active(&listing.data));
    assert_eq!(read_listing_price(&listing.data), initial_price);

    // 2. Update price
    svm.expire_blockhash();
    let update_ix = anchor_instruction(
        program_id,
        "update_price",
        &update_price_data(updated_price),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(listing_pda),
        ],
    );
    execute_tx(&mut svm, update_ix, &seller, &[&seller]).unwrap();

    // Verify price updated
    let listing = svm.get_account(&listing_pda).unwrap();
    assert_eq!(read_listing_price(&listing.data), updated_price);

    // 3. Buy the NFT
    svm.expire_blockhash();
    let buy_ix = anchor_instruction(
        program_id,
        "buy_nft",
        &[],
        vec![
            signer_meta(buyer.pubkey()),
            writable_meta(seller.pubkey()),
            writable_meta(nft_mint),
            writable_meta(payment_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(buyer_nft_token),
            writable_meta(buyer_payment_token),
            writable_meta(seller_payment_token),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, buy_ix, &buyer, &[&buyer]).unwrap();

    // Verify final state
    let buyer_nft = svm.get_account(&buyer_nft_token).unwrap();
    assert_eq!(read_token_balance(&buyer_nft.data), 1, "Buyer should have NFT");

    let seller_nft = svm.get_account(&seller_nft_token).unwrap();
    assert_eq!(read_token_balance(&seller_nft.data), 0, "Seller should not have NFT");

    let seller_payment = svm.get_account(&seller_payment_token).unwrap();
    assert_eq!(read_token_balance(&seller_payment.data), updated_price, "Seller should receive updated price");

    let buyer_payment = svm.get_account(&buyer_payment_token).unwrap();
    assert_eq!(read_token_balance(&buyer_payment.data), 10000 - updated_price, "Buyer paid updated price");

    let listing = svm.get_account(&listing_pda).unwrap();
    assert!(!read_listing_is_active(&listing.data), "Listing should be inactive");
}

#[test]
fn test_full_marketplace_workflow_delist() {
    let mut svm = load_nft_marketplace_program();
    let program_id = nft_marketplace_program_id();

    let (_, nft_mint) = setup_nft_mint(&mut svm);
    let (_, payment_mint) = setup_payment_mint(&mut svm, 6);

    let (seller, seller_nft_token) = setup_user_with_nft(&mut svm, &nft_mint);

    let listing_id: u64 = 1;
    let (listing_pda, _) = derive_listing_pda(listing_id);
    let (nft_escrow_pda, _) = derive_nft_escrow_pda(listing_id);

    // 1. List the NFT
    let list_ix = anchor_instruction(
        program_id,
        "list_nft",
        &list_nft_data(listing_id, 500),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(nft_mint),
            writable_meta(payment_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(seller_nft_token),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, list_ix, &seller, &[&seller]).unwrap();

    // Verify NFT escrowed
    let seller_nft = svm.get_account(&seller_nft_token).unwrap();
    assert_eq!(read_token_balance(&seller_nft.data), 0);
    let escrow = svm.get_account(&nft_escrow_pda).unwrap();
    assert_eq!(read_token_balance(&escrow.data), 1);

    // 2. Change mind, delist
    svm.expire_blockhash();
    let delist_ix = anchor_instruction(
        program_id,
        "delist_nft",
        &[],
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(nft_mint),
            writable_meta(listing_pda),
            writable_meta(nft_escrow_pda),
            writable_meta(seller_nft_token),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, delist_ix, &seller, &[&seller]).unwrap();

    // Verify NFT returned
    let seller_nft = svm.get_account(&seller_nft_token).unwrap();
    assert_eq!(read_token_balance(&seller_nft.data), 1, "Seller should have NFT back");

    let escrow = svm.get_account(&nft_escrow_pda).unwrap();
    assert_eq!(read_token_balance(&escrow.data), 0, "Escrow should be empty");

    let listing = svm.get_account(&listing_pda).unwrap();
    assert!(!read_listing_is_active(&listing.data), "Listing should be inactive");
}

#[test]
fn test_pda_derivation_deterministic() {
    // Verify PDA derivation is deterministic
    let (listing_pda_1, bump_1) = derive_listing_pda(1);
    let (listing_pda_1_again, bump_1_again) = derive_listing_pda(1);

    assert_eq!(listing_pda_1, listing_pda_1_again);
    assert_eq!(bump_1, bump_1_again);

    let (listing_pda_2, _) = derive_listing_pda(2);
    assert_ne!(listing_pda_1, listing_pda_2, "Different listing IDs should produce different PDAs");

    let (nft_escrow_1, _) = derive_nft_escrow_pda(1);
    let (nft_escrow_2, _) = derive_nft_escrow_pda(2);

    assert_ne!(nft_escrow_1, nft_escrow_2, "Different listing IDs should produce different escrow PDAs");
    assert_ne!(listing_pda_1, nft_escrow_1, "Listing and escrow PDAs should be different");
}
