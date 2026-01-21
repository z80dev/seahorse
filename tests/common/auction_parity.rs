//! Auction tests: Seahorse implementation behavior verification
//!
//! These tests verify the Seahorse auction implementation produces
//! correct behavior for time-based state transitions.
//!
//! Key behaviors to verify:
//! - Auction initialization creates correct PDAs and state
//! - Bidding enforces minimum bid requirements
//! - Bid refunds work correctly when outbid
//! - Time-based auction end enforcement
//! - Prize claim validates winner and transfers funds
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile:
//! - target/deploy/auction.so (Seahorse)

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

/// Auction program ID
fn program_id() -> Pubkey {
    Pubkey::from_str("AUCTnhLbEfvJxTt9NQekQfpjsVq4xDv5aX1VL6h9pLwD").unwrap()
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

/// Derive auction PDA
fn derive_auction_pda(auction_id: u64, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"auction", &auction_id.to_le_bytes()], program_id)
}

/// Derive bid escrow PDA
fn derive_bid_escrow_pda(auction_id: u64, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"bid_escrow", &auction_id.to_le_bytes()], program_id)
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

/// Read auction account fields
fn read_auction_seller(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

fn read_auction_id(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[40..48].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_auction_starting_price(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[80..88].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_auction_current_bid(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[88..96].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_auction_highest_bidder(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[96..128].try_into().unwrap())
}

fn read_auction_start_slot(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[128..136].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_auction_end_slot(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[136..144].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_auction_is_ended(data: &[u8]) -> bool {
    data[144] != 0
}

fn read_auction_is_claimed(data: &[u8]) -> bool {
    data[145] != 0
}

/// Build create_auction instruction data
fn create_auction_data(auction_id: u64, starting_price: u64, duration_slots: u64) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&auction_id.to_le_bytes());
    data.extend_from_slice(&starting_price.to_le_bytes());
    data.extend_from_slice(&duration_slots.to_le_bytes());
    data
}

/// Build place_bid instruction data
fn place_bid_data(bid_amount: u64) -> Vec<u8> {
    bid_amount.to_le_bytes().to_vec()
}

// =============================================================================
// PARITY TESTS
// =============================================================================

#[test]
fn test_parity_auction_initialization() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/auction.so")
        .expect("Failed to read auction.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup mint
    let mint = Keypair::new();
    svm.set_account(mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    // Setup seller
    let seller = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let auction_id: u64 = 1;
    let starting_price: u64 = 100;
    let duration_slots: u64 = 1000;

    // Derive PDAs
    let (auction_pda, _) = derive_auction_pda(auction_id, &program_id);
    let (bid_escrow_pda, _) = derive_bid_escrow_pda(auction_id, &program_id);

    // Create auction
    let ix = InstructionBuilder::new("create_auction")
        .with_signer(&seller.pubkey())
        .with_writable(&mint.pubkey())
        .with_writable(&auction_pda)
        .with_writable(&bid_escrow_pda)
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(create_auction_data(auction_id, starting_price, duration_slots))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[ix], Some(&seller.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seller], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "create_auction failed: {:?}", result);

    // Verify auction account state
    let auction = svm.get_account(&auction_pda).expect("Auction should exist");
    assert_eq!(auction.owner, program_id, "Auction should be owned by program");
    assert_eq!(read_auction_seller(&auction.data), seller.pubkey());
    assert_eq!(read_auction_id(&auction.data), auction_id);
    assert_eq!(read_auction_starting_price(&auction.data), starting_price);
    assert_eq!(read_auction_current_bid(&auction.data), 0);
    assert!(!read_auction_is_ended(&auction.data));
    assert!(!read_auction_is_claimed(&auction.data));

    let start_slot = read_auction_start_slot(&auction.data);
    assert_eq!(read_auction_end_slot(&auction.data), start_slot + duration_slots);

    // Verify bid escrow exists with zero balance
    let escrow = svm.get_account(&bid_escrow_pda).expect("Bid escrow should exist");
    assert_eq!(read_token_balance(&escrow.data), 0);
}

#[test]
fn test_parity_bid_transfers_tokens_to_escrow() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/auction.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup
    let mint = Keypair::new();
    svm.set_account(mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    let seller = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let seller_token = get_ata(&seller.pubkey(), &mint.pubkey());
    svm.set_account(seller_token, create_token_account(&seller.pubkey(), &mint.pubkey(), 0))
        .unwrap();

    let bidder = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let bidder_token = get_ata(&bidder.pubkey(), &mint.pubkey());
    svm.set_account(bidder_token, create_token_account(&bidder.pubkey(), &mint.pubkey(), 10000))
        .unwrap();

    let auction_id: u64 = 1;
    let (auction_pda, _) = derive_auction_pda(auction_id, &program_id);
    let (bid_escrow_pda, _) = derive_bid_escrow_pda(auction_id, &program_id);

    // Create auction
    let create_ix = InstructionBuilder::new("create_auction")
        .with_signer(&seller.pubkey())
        .with_writable(&mint.pubkey())
        .with_writable(&auction_pda)
        .with_writable(&bid_escrow_pda)
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(create_auction_data(auction_id, 100, 1000))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[create_ix], Some(&seller.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seller], message, blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // Place bid
    let bid_amount: u64 = 200;
    let bid_ix = InstructionBuilder::new("place_bid")
        .with_signer(&bidder.pubkey())
        .with_writable(&seller.pubkey())
        .with_writable(&mint.pubkey())
        .with_writable(&bidder_token)
        .with_writable(&seller_token)
        .with_writable(&auction_pda)
        .with_writable(&bid_escrow_pda)
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&token_program_id())
        .with_data(place_bid_data(bid_amount))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[bid_ix], Some(&bidder.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&bidder], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "place_bid failed: {:?}", result);

    // Verify bid tokens moved to escrow
    let escrow = svm.get_account(&bid_escrow_pda).unwrap();
    assert_eq!(read_token_balance(&escrow.data), bid_amount);

    // Verify bidder's balance decreased
    let bidder_account = svm.get_account(&bidder_token).unwrap();
    assert_eq!(read_token_balance(&bidder_account.data), 10000 - bid_amount);

    // Verify auction state updated
    let auction = svm.get_account(&auction_pda).unwrap();
    assert_eq!(read_auction_current_bid(&auction.data), bid_amount);
    assert_eq!(read_auction_highest_bidder(&auction.data), bidder.pubkey());
}

#[test]
fn test_parity_outbid_refunds_previous_bidder() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/auction.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup
    let mint = Keypair::new();
    svm.set_account(mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    let seller = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let seller_token = get_ata(&seller.pubkey(), &mint.pubkey());
    svm.set_account(seller_token, create_token_account(&seller.pubkey(), &mint.pubkey(), 0))
        .unwrap();

    let bidder1 = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let bidder1_token = get_ata(&bidder1.pubkey(), &mint.pubkey());
    svm.set_account(bidder1_token, create_token_account(&bidder1.pubkey(), &mint.pubkey(), 10000))
        .unwrap();

    let bidder2 = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let bidder2_token = get_ata(&bidder2.pubkey(), &mint.pubkey());
    svm.set_account(bidder2_token, create_token_account(&bidder2.pubkey(), &mint.pubkey(), 10000))
        .unwrap();

    let auction_id: u64 = 1;
    let (auction_pda, _) = derive_auction_pda(auction_id, &program_id);
    let (bid_escrow_pda, _) = derive_bid_escrow_pda(auction_id, &program_id);

    // Create auction
    let create_ix = InstructionBuilder::new("create_auction")
        .with_signer(&seller.pubkey())
        .with_writable(&mint.pubkey())
        .with_writable(&auction_pda)
        .with_writable(&bid_escrow_pda)
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(create_auction_data(auction_id, 100, 1000))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[create_ix], Some(&seller.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seller], message, blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // Bidder 1 places first bid
    let bid1_ix = InstructionBuilder::new("place_bid")
        .with_signer(&bidder1.pubkey())
        .with_writable(&seller.pubkey())
        .with_writable(&mint.pubkey())
        .with_writable(&bidder1_token)
        .with_writable(&seller_token)
        .with_writable(&auction_pda)
        .with_writable(&bid_escrow_pda)
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&token_program_id())
        .with_data(place_bid_data(150))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[bid1_ix], Some(&bidder1.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&bidder1], message, blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // Verify bidder 1's balance
    let bidder1_account = svm.get_account(&bidder1_token).unwrap();
    assert_eq!(read_token_balance(&bidder1_account.data), 9850);

    // Bidder 2 outbids
    let bid2_ix = InstructionBuilder::new("place_bid")
        .with_signer(&bidder2.pubkey())
        .with_writable(&bidder1.pubkey())
        .with_writable(&mint.pubkey())
        .with_writable(&bidder2_token)
        .with_writable(&bidder1_token)
        .with_writable(&auction_pda)
        .with_writable(&bid_escrow_pda)
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&token_program_id())
        .with_data(place_bid_data(200))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[bid2_ix], Some(&bidder2.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&bidder2], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "outbid failed: {:?}", result);

    // Verify bidder 1 was refunded
    let bidder1_account = svm.get_account(&bidder1_token).unwrap();
    assert_eq!(read_token_balance(&bidder1_account.data), 10000, "Bidder 1 should be fully refunded");

    // Verify escrow has bidder 2's bid
    let escrow = svm.get_account(&bid_escrow_pda).unwrap();
    assert_eq!(read_token_balance(&escrow.data), 200);

    // Verify auction state shows bidder 2 as winner
    let auction = svm.get_account(&auction_pda).unwrap();
    assert_eq!(read_auction_highest_bidder(&auction.data), bidder2.pubkey());
    assert_eq!(read_auction_current_bid(&auction.data), 200);
}

#[test]
fn test_parity_time_based_auction_end() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/auction.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup
    let mint = Keypair::new();
    svm.set_account(mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    let seller = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let auction_id: u64 = 1;
    let duration_slots: u64 = 100;
    let (auction_pda, _) = derive_auction_pda(auction_id, &program_id);
    let (bid_escrow_pda, _) = derive_bid_escrow_pda(auction_id, &program_id);

    // Create auction
    let create_ix = InstructionBuilder::new("create_auction")
        .with_signer(&seller.pubkey())
        .with_writable(&mint.pubkey())
        .with_writable(&auction_pda)
        .with_writable(&bid_escrow_pda)
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(create_auction_data(auction_id, 100, duration_slots))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[create_ix], Some(&seller.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seller], message, blockhash);
    svm.send_transaction(tx).unwrap();

    // Get end slot
    let auction = svm.get_account(&auction_pda).unwrap();
    let end_slot = read_auction_end_slot(&auction.data);

    // Try to end before time (should fail)
    svm.expire_blockhash();
    let caller = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let early_end_ix = InstructionBuilder::new("end_auction")
        .with_signer(&caller.pubkey())
        .with_writable(&auction_pda)
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[early_end_ix], Some(&caller.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&caller], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_err(), "End auction before time should fail");

    // Warp past end slot
    svm.warp_to_slot(end_slot + 1);
    svm.expire_blockhash();

    // Now end should succeed
    let end_ix = InstructionBuilder::new("end_auction")
        .with_signer(&caller.pubkey())
        .with_writable(&auction_pda)
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[end_ix], Some(&caller.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&caller], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "End auction after time should succeed: {:?}", result);

    // Verify auction is ended
    let auction = svm.get_account(&auction_pda).unwrap();
    assert!(read_auction_is_ended(&auction.data));
}

#[test]
fn test_parity_deterministic_pda_derivation() {
    let program_id = program_id();

    // Verify auction PDA derivation is deterministic
    let (auction1, bump1) = derive_auction_pda(1, &program_id);
    let (auction2, bump2) = derive_auction_pda(1, &program_id);

    assert_eq!(auction1, auction2, "Auction PDAs should be deterministic");
    assert_eq!(bump1, bump2, "Auction bumps should be deterministic");

    // Verify bid escrow PDA derivation is deterministic
    let (escrow1, escrow_bump1) = derive_bid_escrow_pda(1, &program_id);
    let (escrow2, escrow_bump2) = derive_bid_escrow_pda(1, &program_id);

    assert_eq!(escrow1, escrow2, "Escrow PDAs should be deterministic");
    assert_eq!(escrow_bump1, escrow_bump2, "Escrow bumps should be deterministic");

    // Verify different auction IDs produce different PDAs
    let (auction3, _) = derive_auction_pda(2, &program_id);
    assert_ne!(auction1, auction3, "Different auction IDs should produce different PDAs");
}

#[test]
fn test_parity_full_auction_workflow() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/auction.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup
    let mint = Keypair::new();
    svm.set_account(mint.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    let seller = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let seller_token = get_ata(&seller.pubkey(), &mint.pubkey());
    svm.set_account(seller_token, create_token_account(&seller.pubkey(), &mint.pubkey(), 0))
        .unwrap();

    let bidder = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let bidder_token = get_ata(&bidder.pubkey(), &mint.pubkey());
    svm.set_account(bidder_token, create_token_account(&bidder.pubkey(), &mint.pubkey(), 10000))
        .unwrap();

    let auction_id: u64 = 1;
    let (auction_pda, _) = derive_auction_pda(auction_id, &program_id);
    let (bid_escrow_pda, _) = derive_bid_escrow_pda(auction_id, &program_id);

    // 1. Create auction
    let create_ix = InstructionBuilder::new("create_auction")
        .with_signer(&seller.pubkey())
        .with_writable(&mint.pubkey())
        .with_writable(&auction_pda)
        .with_writable(&bid_escrow_pda)
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(create_auction_data(auction_id, 100, 100))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[create_ix], Some(&seller.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seller], message, blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // 2. Place bid
    let bid_amount: u64 = 500;
    let bid_ix = InstructionBuilder::new("place_bid")
        .with_signer(&bidder.pubkey())
        .with_writable(&seller.pubkey())
        .with_writable(&mint.pubkey())
        .with_writable(&bidder_token)
        .with_writable(&seller_token)
        .with_writable(&auction_pda)
        .with_writable(&bid_escrow_pda)
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .with_readonly(&token_program_id())
        .with_data(place_bid_data(bid_amount))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[bid_ix], Some(&bidder.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&bidder], message, blockhash);
    svm.send_transaction(tx).unwrap();

    // 3. End auction
    let auction = svm.get_account(&auction_pda).unwrap();
    let end_slot = read_auction_end_slot(&auction.data);
    svm.warp_to_slot(end_slot + 1);
    svm.expire_blockhash();

    let end_ix = InstructionBuilder::new("end_auction")
        .with_signer(&seller.pubkey())
        .with_writable(&auction_pda)
        .with_readonly(&solana_sdk_ids::sysvar::clock::id())
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[end_ix], Some(&seller.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&seller], message, blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // 4. Claim prize
    let claim_ix = InstructionBuilder::new("claim_prize")
        .with_signer(&bidder.pubkey())
        .with_writable(&seller.pubkey())
        .with_writable(&mint.pubkey())
        .with_writable(&seller_token)
        .with_writable(&auction_pda)
        .with_writable(&bid_escrow_pda)
        .with_readonly(&token_program_id())
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[claim_ix], Some(&bidder.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&bidder], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "claim_prize failed: {:?}", result);

    // Verify final state
    let seller_account = svm.get_account(&seller_token).unwrap();
    assert_eq!(read_token_balance(&seller_account.data), bid_amount, "Seller should receive winning bid");

    let bidder_account = svm.get_account(&bidder_token).unwrap();
    assert_eq!(read_token_balance(&bidder_account.data), 10000 - bid_amount, "Bidder paid bid amount");

    let escrow = svm.get_account(&bid_escrow_pda).unwrap();
    assert_eq!(read_token_balance(&escrow.data), 0, "Escrow should be empty");

    let auction = svm.get_account(&auction_pda).unwrap();
    assert!(read_auction_is_ended(&auction.data));
    assert!(read_auction_is_claimed(&auction.data));
}
