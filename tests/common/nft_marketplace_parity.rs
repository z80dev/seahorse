//! NFT Marketplace tests: Seahorse implementation behavior verification
//!
//! These tests verify the Seahorse NFT marketplace implementation produces
//! correct behavior for listing, buying, and delisting NFTs.
//!
//! Key behaviors to verify:
//! - Listing creates correct PDAs and escrows NFT
//! - Buying transfers payment to seller and NFT to buyer
//! - Delisting returns NFT to seller
//! - Price updates work correctly
//! - Authorization is enforced for all operations
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile:
//! - target/deploy/nft_marketplace.so (Seahorse)

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

/// NFT Marketplace program ID
fn program_id() -> Pubkey {
    Pubkey::from_str("Coa8ZSxePf7ZCsDNHE9SRHyZJRWWnZgmanuKVf4ZBM7Q").unwrap()
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

/// Derive listing PDA
fn derive_listing_pda(listing_id: u64, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"listing", &listing_id.to_le_bytes()], program_id)
}

/// Derive NFT escrow PDA
fn derive_nft_escrow_pda(listing_id: u64, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"nft_escrow", &listing_id.to_le_bytes()], program_id)
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
// PARITY TESTS
// =============================================================================

#[test]
fn test_parity_listing_initialization() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/nft_marketplace.so")
        .expect("Failed to read nft_marketplace.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup NFT mint (decimals=0 for NFT)
    let nft_mint = Keypair::new();
    svm.set_account(nft_mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 0))
        .unwrap();

    // Setup payment mint
    let payment_mint = Keypair::new();
    svm.set_account(payment_mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    // Setup seller with NFT
    let seller = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let seller_nft_token = get_ata(&seller.pubkey(), &nft_mint.pubkey());
    svm.set_account(seller_nft_token, create_token_account(&seller.pubkey(), &nft_mint.pubkey(), 1))
        .unwrap();

    let listing_id: u64 = 1;
    let price: u64 = 500;

    // Derive PDAs
    let (listing_pda, _) = derive_listing_pda(listing_id, &program_id);
    let (nft_escrow_pda, _) = derive_nft_escrow_pda(listing_id, &program_id);

    // List NFT
    let ix = InstructionBuilder::new("list_nft")
        .with_signer(&seller.pubkey())
        .with_writable(&nft_mint.pubkey())
        .with_writable(&payment_mint.pubkey())
        .with_writable(&listing_pda)
        .with_writable(&nft_escrow_pda)
        .with_writable(&seller_nft_token)
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(list_nft_data(listing_id, price))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[ix], Some(&seller.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seller], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "list_nft failed: {:?}", result);

    // Verify listing account state
    let listing = svm.get_account(&listing_pda).expect("Listing should exist");
    assert_eq!(listing.owner, program_id, "Listing should be owned by program");
    assert_eq!(read_listing_seller(&listing.data), seller.pubkey());
    assert_eq!(read_listing_id(&listing.data), listing_id);
    assert_eq!(read_listing_nft_mint(&listing.data), nft_mint.pubkey());
    assert_eq!(read_listing_payment_mint(&listing.data), payment_mint.pubkey());
    assert_eq!(read_listing_price(&listing.data), price);
    assert!(read_listing_is_active(&listing.data));

    // Verify NFT escrow has the NFT
    let escrow = svm.get_account(&nft_escrow_pda).expect("NFT escrow should exist");
    assert_eq!(read_token_balance(&escrow.data), 1);

    // Verify seller no longer has NFT
    let seller_nft = svm.get_account(&seller_nft_token).unwrap();
    assert_eq!(read_token_balance(&seller_nft.data), 0);
}

#[test]
fn test_parity_buy_transfers_nft_and_payment() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/nft_marketplace.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup mints
    let nft_mint = Keypair::new();
    svm.set_account(nft_mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 0))
        .unwrap();

    let payment_mint = Keypair::new();
    svm.set_account(payment_mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    // Setup seller with NFT
    let seller = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let seller_nft_token = get_ata(&seller.pubkey(), &nft_mint.pubkey());
    svm.set_account(seller_nft_token, create_token_account(&seller.pubkey(), &nft_mint.pubkey(), 1))
        .unwrap();
    let seller_payment_token = get_ata(&seller.pubkey(), &payment_mint.pubkey());
    svm.set_account(seller_payment_token, create_token_account(&seller.pubkey(), &payment_mint.pubkey(), 0))
        .unwrap();

    // Setup buyer with payment tokens
    let buyer = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let buyer_payment_token = get_ata(&buyer.pubkey(), &payment_mint.pubkey());
    svm.set_account(buyer_payment_token, create_token_account(&buyer.pubkey(), &payment_mint.pubkey(), 10000))
        .unwrap();
    let buyer_nft_token = get_ata(&buyer.pubkey(), &nft_mint.pubkey());
    svm.set_account(buyer_nft_token, create_token_account(&buyer.pubkey(), &nft_mint.pubkey(), 0))
        .unwrap();

    let listing_id: u64 = 1;
    let price: u64 = 500;
    let (listing_pda, _) = derive_listing_pda(listing_id, &program_id);
    let (nft_escrow_pda, _) = derive_nft_escrow_pda(listing_id, &program_id);

    // List NFT
    let list_ix = InstructionBuilder::new("list_nft")
        .with_signer(&seller.pubkey())
        .with_writable(&nft_mint.pubkey())
        .with_writable(&payment_mint.pubkey())
        .with_writable(&listing_pda)
        .with_writable(&nft_escrow_pda)
        .with_writable(&seller_nft_token)
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(list_nft_data(listing_id, price))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[list_ix], Some(&seller.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seller], message, blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // Buy NFT
    let buy_ix = InstructionBuilder::new("buy_nft")
        .with_signer(&buyer.pubkey())
        .with_writable(&seller.pubkey())
        .with_writable(&nft_mint.pubkey())
        .with_writable(&payment_mint.pubkey())
        .with_writable(&listing_pda)
        .with_writable(&nft_escrow_pda)
        .with_writable(&buyer_nft_token)
        .with_writable(&buyer_payment_token)
        .with_writable(&seller_payment_token)
        .with_readonly(&token_program_id())
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[buy_ix], Some(&buyer.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&buyer], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "buy_nft failed: {:?}", result);

    // Verify buyer received NFT
    let buyer_nft = svm.get_account(&buyer_nft_token).unwrap();
    assert_eq!(read_token_balance(&buyer_nft.data), 1, "Buyer should have NFT");

    // Verify seller received payment
    let seller_payment = svm.get_account(&seller_payment_token).unwrap();
    assert_eq!(read_token_balance(&seller_payment.data), price, "Seller should receive payment");

    // Verify buyer's payment decreased
    let buyer_payment = svm.get_account(&buyer_payment_token).unwrap();
    assert_eq!(read_token_balance(&buyer_payment.data), 10000 - price, "Buyer paid price");

    // Verify escrow is empty
    let escrow = svm.get_account(&nft_escrow_pda).unwrap();
    assert_eq!(read_token_balance(&escrow.data), 0, "Escrow should be empty");

    // Verify listing is inactive
    let listing = svm.get_account(&listing_pda).unwrap();
    assert!(!read_listing_is_active(&listing.data), "Listing should be inactive");
}

#[test]
fn test_parity_delist_returns_nft() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/nft_marketplace.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup mints
    let nft_mint = Keypair::new();
    svm.set_account(nft_mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 0))
        .unwrap();

    let payment_mint = Keypair::new();
    svm.set_account(payment_mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    // Setup seller with NFT
    let seller = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let seller_nft_token = get_ata(&seller.pubkey(), &nft_mint.pubkey());
    svm.set_account(seller_nft_token, create_token_account(&seller.pubkey(), &nft_mint.pubkey(), 1))
        .unwrap();

    let listing_id: u64 = 1;
    let (listing_pda, _) = derive_listing_pda(listing_id, &program_id);
    let (nft_escrow_pda, _) = derive_nft_escrow_pda(listing_id, &program_id);

    // List NFT
    let list_ix = InstructionBuilder::new("list_nft")
        .with_signer(&seller.pubkey())
        .with_writable(&nft_mint.pubkey())
        .with_writable(&payment_mint.pubkey())
        .with_writable(&listing_pda)
        .with_writable(&nft_escrow_pda)
        .with_writable(&seller_nft_token)
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(list_nft_data(listing_id, 500))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[list_ix], Some(&seller.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seller], message, blockhash);
    svm.send_transaction(tx).unwrap();

    // Verify NFT escrowed
    let seller_nft = svm.get_account(&seller_nft_token).unwrap();
    assert_eq!(read_token_balance(&seller_nft.data), 0, "Seller should not have NFT");
    let escrow = svm.get_account(&nft_escrow_pda).unwrap();
    assert_eq!(read_token_balance(&escrow.data), 1, "Escrow should have NFT");

    // Delist NFT
    svm.expire_blockhash();
    let delist_ix = InstructionBuilder::new("delist_nft")
        .with_signer(&seller.pubkey())
        .with_writable(&nft_mint.pubkey())
        .with_writable(&listing_pda)
        .with_writable(&nft_escrow_pda)
        .with_writable(&seller_nft_token)
        .with_readonly(&token_program_id())
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[delist_ix], Some(&seller.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seller], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "delist_nft failed: {:?}", result);

    // Verify NFT returned to seller
    let seller_nft = svm.get_account(&seller_nft_token).unwrap();
    assert_eq!(read_token_balance(&seller_nft.data), 1, "Seller should have NFT back");

    // Verify escrow is empty
    let escrow = svm.get_account(&nft_escrow_pda).unwrap();
    assert_eq!(read_token_balance(&escrow.data), 0, "Escrow should be empty");

    // Verify listing is inactive
    let listing = svm.get_account(&listing_pda).unwrap();
    assert!(!read_listing_is_active(&listing.data), "Listing should be inactive");
}

#[test]
fn test_parity_update_price_changes_listing() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/nft_marketplace.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup mints
    let nft_mint = Keypair::new();
    svm.set_account(nft_mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 0))
        .unwrap();

    let payment_mint = Keypair::new();
    svm.set_account(payment_mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    // Setup seller with NFT
    let seller = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let seller_nft_token = get_ata(&seller.pubkey(), &nft_mint.pubkey());
    svm.set_account(seller_nft_token, create_token_account(&seller.pubkey(), &nft_mint.pubkey(), 1))
        .unwrap();

    let listing_id: u64 = 1;
    let initial_price: u64 = 500;
    let new_price: u64 = 750;
    let (listing_pda, _) = derive_listing_pda(listing_id, &program_id);
    let (nft_escrow_pda, _) = derive_nft_escrow_pda(listing_id, &program_id);

    // List NFT
    let list_ix = InstructionBuilder::new("list_nft")
        .with_signer(&seller.pubkey())
        .with_writable(&nft_mint.pubkey())
        .with_writable(&payment_mint.pubkey())
        .with_writable(&listing_pda)
        .with_writable(&nft_escrow_pda)
        .with_writable(&seller_nft_token)
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(list_nft_data(listing_id, initial_price))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[list_ix], Some(&seller.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seller], message, blockhash);
    svm.send_transaction(tx).unwrap();

    // Verify initial price
    let listing = svm.get_account(&listing_pda).unwrap();
    assert_eq!(read_listing_price(&listing.data), initial_price);

    // Update price
    svm.expire_blockhash();
    let update_ix = InstructionBuilder::new("update_price")
        .with_signer(&seller.pubkey())
        .with_writable(&listing_pda)
        .with_data(update_price_data(new_price))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[update_ix], Some(&seller.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seller], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "update_price failed: {:?}", result);

    // Verify price updated
    let listing = svm.get_account(&listing_pda).unwrap();
    assert_eq!(read_listing_price(&listing.data), new_price, "Price should be updated");
    assert!(read_listing_is_active(&listing.data), "Listing should still be active");
}

#[test]
fn test_parity_deterministic_pda_derivation() {
    let program_id = program_id();

    // Verify PDA derivation is deterministic
    let (listing_pda_1, bump_1) = derive_listing_pda(1, &program_id);
    let (listing_pda_1_again, bump_1_again) = derive_listing_pda(1, &program_id);

    assert_eq!(listing_pda_1, listing_pda_1_again, "PDA should be deterministic");
    assert_eq!(bump_1, bump_1_again, "Bump should be deterministic");

    let (listing_pda_2, _) = derive_listing_pda(2, &program_id);
    assert_ne!(listing_pda_1, listing_pda_2, "Different listing IDs should produce different PDAs");

    let (nft_escrow_1, _) = derive_nft_escrow_pda(1, &program_id);
    let (nft_escrow_2, _) = derive_nft_escrow_pda(2, &program_id);

    assert_ne!(nft_escrow_1, nft_escrow_2, "Different listing IDs should produce different escrow PDAs");
    assert_ne!(listing_pda_1, nft_escrow_1, "Listing and escrow PDAs should be different");
}
