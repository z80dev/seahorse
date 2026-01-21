//! LiteSVM integration tests for the Seahorse Auction program
//!
//! These tests verify auction functionality with time-based state transitions:
//! - create_auction: Creates auction with starting price and duration
//! - place_bid: Places bid, previous bidder refunded
//! - end_auction: Ends auction after time expires
//! - claim_prize: Winner claims, seller receives payment
//!
//! Uses LiteSVM's warp_to_slot for time-based auction testing.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile auction.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// Auction program ID (from declare_id!)
fn auction_program_id() -> Pubkey {
    Pubkey::from_str("AUCTnhLbEfvJxTt9NQekQfpjsVq4xDv5aX1VL6h9pLwD").unwrap()
}

/// Auction account size:
/// discriminator (8) + seller (32) + auction_id (8) + bid_mint (32) + starting_price (8)
/// + current_bid (8) + highest_bidder (32) + start_slot (8) + end_slot (8)
/// + is_ended (1) + is_claimed (1) + bump (1) = 147 bytes
const AUCTION_ACCOUNT_SIZE: usize = 8 + 32 + 8 + 32 + 8 + 8 + 32 + 8 + 8 + 1 + 1 + 1;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the auction program into LiteSVM
fn load_auction_program() -> litesvm::LiteSVM {
    let program_id = auction_program_id();
    let program_bytes = std::fs::read("../../target/deploy/auction.so")
        .expect("Failed to read auction.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);
    svm
}

/// Setup a mint for testing
fn setup_mint(svm: &mut litesvm::LiteSVM, decimals: u8) -> (Keypair, Pubkey) {
    let mint_authority = funded_keypair_10_sol(svm);
    let mint = Keypair::new();

    let mint_account = create_mint_account(&mint_authority.pubkey(), decimals);
    svm.set_account(mint.pubkey(), mint_account).unwrap();

    (mint_authority, mint.pubkey())
}

/// Setup a user with tokens
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

/// Derive auction PDA
fn derive_auction_pda(auction_id: u64) -> (Pubkey, u8) {
    find_pda(
        &[b"auction", &auction_id.to_le_bytes()],
        &auction_program_id(),
    )
}

/// Derive bid escrow PDA
fn derive_bid_escrow_pda(auction_id: u64) -> (Pubkey, u8) {
    find_pda(
        &[b"bid_escrow", &auction_id.to_le_bytes()],
        &auction_program_id(),
    )
}

/// Read auction account fields
fn read_auction_seller(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[8..40].try_into().unwrap())
}

fn read_auction_id(data: &[u8]) -> u64 {
    let bytes: [u8; 8] = data[40..48].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn read_auction_bid_mint(data: &[u8]) -> Pubkey {
    Pubkey::new_from_array(data[48..80].try_into().unwrap())
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

fn read_auction_bump(data: &[u8]) -> u8 {
    data[146]
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
// CREATE AUCTION TESTS
// =============================================================================

#[test]
fn test_create_auction() {
    let mut svm = load_auction_program();
    let program_id = auction_program_id();

    // Setup mint
    let (_, mint) = setup_mint(&mut svm, 6);

    // Setup seller
    let (seller, _seller_token) = setup_user_with_tokens(&mut svm, &mint, 10000);

    let auction_id: u64 = 1;
    let starting_price: u64 = 100;
    let duration_slots: u64 = 1000;

    // Derive PDAs
    let (auction_pda, _) = derive_auction_pda(auction_id);
    let (bid_escrow_pda, _) = derive_bid_escrow_pda(auction_id);

    // Build create_auction instruction (Seahorse account order)
    let ix = anchor_instruction(
        program_id,
        "create_auction",
        &create_auction_data(auction_id, starting_price, duration_slots),
        vec![
            signer_meta(seller.pubkey()),         // seller
            writable_meta(mint),                   // bid_mint
            writable_meta(auction_pda),            // auction
            writable_meta(bid_escrow_pda),         // bid_escrow
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),  // rent
            readonly_meta(system_program::id()),  // system_program
            readonly_meta(token_program_id()),    // token_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &seller, &[&seller]);
    assert!(result.is_ok(), "create_auction should succeed: {:?}", result);

    // Verify auction account state
    let auction_account = svm.get_account(&auction_pda).expect("Auction should exist");
    assert_eq!(auction_account.owner, program_id);

    assert_eq!(read_auction_seller(&auction_account.data), seller.pubkey());
    assert_eq!(read_auction_id(&auction_account.data), auction_id);
    assert_eq!(read_auction_bid_mint(&auction_account.data), mint);
    assert_eq!(read_auction_starting_price(&auction_account.data), starting_price);
    assert_eq!(read_auction_current_bid(&auction_account.data), 0);
    // Highest bidder is seller initially (placeholder)
    assert_eq!(read_auction_highest_bidder(&auction_account.data), seller.pubkey());
    assert!(!read_auction_is_ended(&auction_account.data));
    assert!(!read_auction_is_claimed(&auction_account.data));

    let start_slot = read_auction_start_slot(&auction_account.data);
    assert_eq!(read_auction_end_slot(&auction_account.data), start_slot + duration_slots);

    // Verify bid escrow exists with zero balance
    let escrow_account = svm.get_account(&bid_escrow_pda).expect("Bid escrow should exist");
    assert_eq!(read_token_balance(&escrow_account.data), 0);
}

#[test]
fn test_create_auction_different_auction_ids() {
    let mut svm = load_auction_program();
    let program_id = auction_program_id();

    let (_, mint) = setup_mint(&mut svm, 6);
    let (seller, _) = setup_user_with_tokens(&mut svm, &mint, 10000);

    // Create first auction
    let (auction_pda_1, _) = derive_auction_pda(1);
    let (bid_escrow_pda_1, _) = derive_bid_escrow_pda(1);

    let ix1 = anchor_instruction(
        program_id,
        "create_auction",
        &create_auction_data(1, 100, 1000),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(auction_pda_1),
            writable_meta(bid_escrow_pda_1),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );

    execute_tx(&mut svm, ix1, &seller, &[&seller]).unwrap();

    // Create second auction
    svm.expire_blockhash();
    let (auction_pda_2, _) = derive_auction_pda(2);
    let (bid_escrow_pda_2, _) = derive_bid_escrow_pda(2);

    let ix2 = anchor_instruction(
        program_id,
        "create_auction",
        &create_auction_data(2, 200, 2000),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(auction_pda_2),
            writable_meta(bid_escrow_pda_2),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );

    execute_tx(&mut svm, ix2, &seller, &[&seller]).unwrap();

    // Verify both auctions exist with different values
    let auction_1 = svm.get_account(&auction_pda_1).unwrap();
    let auction_2 = svm.get_account(&auction_pda_2).unwrap();

    assert_eq!(read_auction_id(&auction_1.data), 1);
    assert_eq!(read_auction_id(&auction_2.data), 2);
    assert_eq!(read_auction_starting_price(&auction_1.data), 100);
    assert_eq!(read_auction_starting_price(&auction_2.data), 200);
}

#[test]
fn test_create_auction_zero_starting_price_fails() {
    let mut svm = load_auction_program();
    let program_id = auction_program_id();

    let (_, mint) = setup_mint(&mut svm, 6);
    let (seller, _) = setup_user_with_tokens(&mut svm, &mint, 10000);

    let (auction_pda, _) = derive_auction_pda(1);
    let (bid_escrow_pda, _) = derive_bid_escrow_pda(1);

    let ix = anchor_instruction(
        program_id,
        "create_auction",
        &create_auction_data(1, 0, 1000), // Zero starting price
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &seller, &[&seller]);
    assert!(result.is_err(), "create_auction with zero starting price should fail");
}

#[test]
fn test_create_auction_zero_duration_fails() {
    let mut svm = load_auction_program();
    let program_id = auction_program_id();

    let (_, mint) = setup_mint(&mut svm, 6);
    let (seller, _) = setup_user_with_tokens(&mut svm, &mint, 10000);

    let (auction_pda, _) = derive_auction_pda(1);
    let (bid_escrow_pda, _) = derive_bid_escrow_pda(1);

    let ix = anchor_instruction(
        program_id,
        "create_auction",
        &create_auction_data(1, 100, 0), // Zero duration
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &seller, &[&seller]);
    assert!(result.is_err(), "create_auction with zero duration should fail");
}

// =============================================================================
// PLACE BID TESTS
// =============================================================================

#[test]
fn test_place_bid_first_bid() {
    let mut svm = load_auction_program();
    let program_id = auction_program_id();

    let (_, mint) = setup_mint(&mut svm, 6);
    let (seller, seller_token) = setup_user_with_tokens(&mut svm, &mint, 10000);
    let (bidder, bidder_token) = setup_user_with_tokens(&mut svm, &mint, 10000);

    let auction_id: u64 = 1;
    let (auction_pda, _) = derive_auction_pda(auction_id);
    let (bid_escrow_pda, _) = derive_bid_escrow_pda(auction_id);

    // Create auction
    let create_ix = anchor_instruction(
        program_id,
        "create_auction",
        &create_auction_data(auction_id, 100, 1000),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &seller, &[&seller]).unwrap();

    // Place first bid (must exceed starting price of 100)
    svm.expire_blockhash();
    let bid_amount: u64 = 150;

    let bid_ix = anchor_instruction(
        program_id,
        "place_bid",
        &place_bid_data(bid_amount),
        vec![
            signer_meta(bidder.pubkey()),          // bidder
            writable_meta(seller.pubkey()),        // previous_bidder (seller placeholder)
            writable_meta(mint),                    // bid_mint
            writable_meta(bidder_token),            // bidder_token_account
            writable_meta(seller_token),            // previous_bidder_token_account
            writable_meta(auction_pda),             // auction
            writable_meta(bid_escrow_pda),          // bid_escrow
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
            readonly_meta(token_program_id()),     // token_program
        ],
    );

    let result = execute_tx(&mut svm, bid_ix, &bidder, &[&bidder]);
    assert!(result.is_ok(), "place_bid should succeed: {:?}", result);

    // Verify auction state updated
    let auction_account = svm.get_account(&auction_pda).unwrap();
    assert_eq!(read_auction_current_bid(&auction_account.data), bid_amount);
    assert_eq!(read_auction_highest_bidder(&auction_account.data), bidder.pubkey());

    // Verify tokens transferred to escrow
    let escrow_account = svm.get_account(&bid_escrow_pda).unwrap();
    assert_eq!(read_token_balance(&escrow_account.data), bid_amount);

    // Verify bidder's tokens decreased
    let bidder_account = svm.get_account(&bidder_token).unwrap();
    assert_eq!(read_token_balance(&bidder_account.data), 10000 - bid_amount);
}

#[test]
fn test_place_bid_outbid_with_refund() {
    let mut svm = load_auction_program();
    let program_id = auction_program_id();

    let (_, mint) = setup_mint(&mut svm, 6);
    let (seller, seller_token) = setup_user_with_tokens(&mut svm, &mint, 10000);
    let (bidder1, bidder1_token) = setup_user_with_tokens(&mut svm, &mint, 10000);
    let (bidder2, bidder2_token) = setup_user_with_tokens(&mut svm, &mint, 10000);

    let auction_id: u64 = 1;
    let (auction_pda, _) = derive_auction_pda(auction_id);
    let (bid_escrow_pda, _) = derive_bid_escrow_pda(auction_id);

    // Create auction
    let create_ix = anchor_instruction(
        program_id,
        "create_auction",
        &create_auction_data(auction_id, 100, 1000),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &seller, &[&seller]).unwrap();

    // Bidder 1 places first bid
    svm.expire_blockhash();
    let bid1_ix = anchor_instruction(
        program_id,
        "place_bid",
        &place_bid_data(150),
        vec![
            signer_meta(bidder1.pubkey()),
            writable_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(bidder1_token),
            writable_meta(seller_token),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, bid1_ix, &bidder1, &[&bidder1]).unwrap();

    // Verify bidder 1's state
    let bidder1_account = svm.get_account(&bidder1_token).unwrap();
    assert_eq!(read_token_balance(&bidder1_account.data), 9850);

    // Bidder 2 outbids (must exceed 150)
    svm.expire_blockhash();
    let bid2_ix = anchor_instruction(
        program_id,
        "place_bid",
        &place_bid_data(200),
        vec![
            signer_meta(bidder2.pubkey()),
            writable_meta(bidder1.pubkey()),       // Previous bidder for refund
            writable_meta(mint),
            writable_meta(bidder2_token),
            writable_meta(bidder1_token),          // Refund goes here
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, bid2_ix, &bidder2, &[&bidder2]);
    assert!(result.is_ok(), "place_bid outbid should succeed: {:?}", result);

    // Verify auction state shows bidder2 as winner
    let auction_account = svm.get_account(&auction_pda).unwrap();
    assert_eq!(read_auction_current_bid(&auction_account.data), 200);
    assert_eq!(read_auction_highest_bidder(&auction_account.data), bidder2.pubkey());

    // Verify escrow has new bid amount
    let escrow_account = svm.get_account(&bid_escrow_pda).unwrap();
    assert_eq!(read_token_balance(&escrow_account.data), 200);

    // Verify bidder1 was refunded (back to original amount)
    let bidder1_account = svm.get_account(&bidder1_token).unwrap();
    assert_eq!(read_token_balance(&bidder1_account.data), 10000);

    // Verify bidder2's tokens decreased
    let bidder2_account = svm.get_account(&bidder2_token).unwrap();
    assert_eq!(read_token_balance(&bidder2_account.data), 9800);
}

#[test]
fn test_place_bid_too_low_fails() {
    let mut svm = load_auction_program();
    let program_id = auction_program_id();

    let (_, mint) = setup_mint(&mut svm, 6);
    let (seller, seller_token) = setup_user_with_tokens(&mut svm, &mint, 10000);
    let (bidder, bidder_token) = setup_user_with_tokens(&mut svm, &mint, 10000);

    let auction_id: u64 = 1;
    let (auction_pda, _) = derive_auction_pda(auction_id);
    let (bid_escrow_pda, _) = derive_bid_escrow_pda(auction_id);

    // Create auction with starting price 100
    let create_ix = anchor_instruction(
        program_id,
        "create_auction",
        &create_auction_data(auction_id, 100, 1000),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &seller, &[&seller]).unwrap();

    // Try to bid 50 (less than starting price of 100)
    svm.expire_blockhash();
    let bid_ix = anchor_instruction(
        program_id,
        "place_bid",
        &place_bid_data(50),
        vec![
            signer_meta(bidder.pubkey()),
            writable_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(bidder_token),
            writable_meta(seller_token),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, bid_ix, &bidder, &[&bidder]);
    assert!(result.is_err(), "Bid below starting price should fail");
}

#[test]
fn test_place_bid_after_auction_expires_fails() {
    let mut svm = load_auction_program();
    let program_id = auction_program_id();

    let (_, mint) = setup_mint(&mut svm, 6);
    let (seller, seller_token) = setup_user_with_tokens(&mut svm, &mint, 10000);
    let (bidder, bidder_token) = setup_user_with_tokens(&mut svm, &mint, 10000);

    let auction_id: u64 = 1;
    let (auction_pda, _) = derive_auction_pda(auction_id);
    let (bid_escrow_pda, _) = derive_bid_escrow_pda(auction_id);

    // Create auction with short duration
    let create_ix = anchor_instruction(
        program_id,
        "create_auction",
        &create_auction_data(auction_id, 100, 100), // Only 100 slots
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &seller, &[&seller]).unwrap();

    // Get end slot
    let auction_account = svm.get_account(&auction_pda).unwrap();
    let end_slot = read_auction_end_slot(&auction_account.data);

    // Warp past end slot
    svm.warp_to_slot(end_slot + 10);
    svm.expire_blockhash();

    // Try to bid after auction expired
    let bid_ix = anchor_instruction(
        program_id,
        "place_bid",
        &place_bid_data(150),
        vec![
            signer_meta(bidder.pubkey()),
            writable_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(bidder_token),
            writable_meta(seller_token),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, bid_ix, &bidder, &[&bidder]);
    assert!(result.is_err(), "Bid after auction expires should fail");
}

// =============================================================================
// END AUCTION TESTS
// =============================================================================

#[test]
fn test_end_auction_after_time_expires() {
    let mut svm = load_auction_program();
    let program_id = auction_program_id();

    let (_, mint) = setup_mint(&mut svm, 6);
    let (seller, seller_token) = setup_user_with_tokens(&mut svm, &mint, 10000);
    let (bidder, bidder_token) = setup_user_with_tokens(&mut svm, &mint, 10000);

    let auction_id: u64 = 1;
    let (auction_pda, _) = derive_auction_pda(auction_id);
    let (bid_escrow_pda, _) = derive_bid_escrow_pda(auction_id);

    // Create auction
    let create_ix = anchor_instruction(
        program_id,
        "create_auction",
        &create_auction_data(auction_id, 100, 100),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &seller, &[&seller]).unwrap();

    // Place a bid
    svm.expire_blockhash();
    let bid_ix = anchor_instruction(
        program_id,
        "place_bid",
        &place_bid_data(150),
        vec![
            signer_meta(bidder.pubkey()),
            writable_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(bidder_token),
            writable_meta(seller_token),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, bid_ix, &bidder, &[&bidder]).unwrap();

    // Get end slot
    let auction_account = svm.get_account(&auction_pda).unwrap();
    let end_slot = read_auction_end_slot(&auction_account.data);

    // Warp past end slot
    svm.warp_to_slot(end_slot + 1);
    svm.expire_blockhash();

    // Anyone can end the auction
    let caller = funded_keypair_10_sol(&mut svm);
    let end_ix = anchor_instruction(
        program_id,
        "end_auction",
        &[],
        vec![
            signer_meta(caller.pubkey()),          // caller
            writable_meta(auction_pda),             // auction
            readonly_meta(solana_sdk_ids::sysvar::clock::id()), // clock
        ],
    );

    let result = execute_tx(&mut svm, end_ix, &caller, &[&caller]);
    assert!(result.is_ok(), "end_auction should succeed: {:?}", result);

    // Verify auction is ended
    let auction_account = svm.get_account(&auction_pda).unwrap();
    assert!(read_auction_is_ended(&auction_account.data));
}

#[test]
fn test_end_auction_before_time_fails() {
    let mut svm = load_auction_program();
    let program_id = auction_program_id();

    let (_, mint) = setup_mint(&mut svm, 6);
    let (seller, _) = setup_user_with_tokens(&mut svm, &mint, 10000);

    let auction_id: u64 = 1;
    let (auction_pda, _) = derive_auction_pda(auction_id);
    let (bid_escrow_pda, _) = derive_bid_escrow_pda(auction_id);

    // Create auction with long duration
    let create_ix = anchor_instruction(
        program_id,
        "create_auction",
        &create_auction_data(auction_id, 100, 10000),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &seller, &[&seller]).unwrap();

    // Try to end auction early
    svm.expire_blockhash();
    let caller = funded_keypair_10_sol(&mut svm);
    let end_ix = anchor_instruction(
        program_id,
        "end_auction",
        &[],
        vec![
            signer_meta(caller.pubkey()),
            writable_meta(auction_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
    );

    let result = execute_tx(&mut svm, end_ix, &caller, &[&caller]);
    assert!(result.is_err(), "end_auction before time should fail");
}

#[test]
fn test_end_auction_already_ended_fails() {
    let mut svm = load_auction_program();
    let program_id = auction_program_id();

    let (_, mint) = setup_mint(&mut svm, 6);
    let (seller, _) = setup_user_with_tokens(&mut svm, &mint, 10000);

    let auction_id: u64 = 1;
    let (auction_pda, _) = derive_auction_pda(auction_id);
    let (bid_escrow_pda, _) = derive_bid_escrow_pda(auction_id);

    // Create and end auction
    let create_ix = anchor_instruction(
        program_id,
        "create_auction",
        &create_auction_data(auction_id, 100, 100),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &seller, &[&seller]).unwrap();

    let auction_account = svm.get_account(&auction_pda).unwrap();
    let end_slot = read_auction_end_slot(&auction_account.data);
    svm.warp_to_slot(end_slot + 1);
    svm.expire_blockhash();

    let caller = funded_keypair_10_sol(&mut svm);
    let end_ix = anchor_instruction(
        program_id,
        "end_auction",
        &[],
        vec![
            signer_meta(caller.pubkey()),
            writable_meta(auction_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
    );
    execute_tx(&mut svm, end_ix, &caller, &[&caller]).unwrap();

    // Try to end again
    svm.expire_blockhash();
    let end_ix2 = anchor_instruction(
        program_id,
        "end_auction",
        &[],
        vec![
            signer_meta(caller.pubkey()),
            writable_meta(auction_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
    );

    let result = execute_tx(&mut svm, end_ix2, &caller, &[&caller]);
    assert!(result.is_err(), "end_auction twice should fail");
}

// =============================================================================
// CLAIM PRIZE TESTS
// =============================================================================

#[test]
fn test_claim_prize_success() {
    let mut svm = load_auction_program();
    let program_id = auction_program_id();

    let (_, mint) = setup_mint(&mut svm, 6);
    let (seller, seller_token) = setup_user_with_tokens(&mut svm, &mint, 0);
    let (bidder, bidder_token) = setup_user_with_tokens(&mut svm, &mint, 10000);

    let auction_id: u64 = 1;
    let (auction_pda, _) = derive_auction_pda(auction_id);
    let (bid_escrow_pda, _) = derive_bid_escrow_pda(auction_id);

    // Create auction
    let create_ix = anchor_instruction(
        program_id,
        "create_auction",
        &create_auction_data(auction_id, 100, 100),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &seller, &[&seller]).unwrap();

    // Place bid
    svm.expire_blockhash();
    let bid_ix = anchor_instruction(
        program_id,
        "place_bid",
        &place_bid_data(500),
        vec![
            signer_meta(bidder.pubkey()),
            writable_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(bidder_token),
            writable_meta(seller_token),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, bid_ix, &bidder, &[&bidder]).unwrap();

    // End auction
    let auction_account = svm.get_account(&auction_pda).unwrap();
    let end_slot = read_auction_end_slot(&auction_account.data);
    svm.warp_to_slot(end_slot + 1);
    svm.expire_blockhash();

    let end_ix = anchor_instruction(
        program_id,
        "end_auction",
        &[],
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(auction_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
    );
    execute_tx(&mut svm, end_ix, &seller, &[&seller]).unwrap();

    // Claim prize
    svm.expire_blockhash();
    let claim_ix = anchor_instruction(
        program_id,
        "claim_prize",
        &[],
        vec![
            signer_meta(bidder.pubkey()),          // winner
            writable_meta(seller.pubkey()),        // seller
            writable_meta(mint),                    // bid_mint
            writable_meta(seller_token),            // seller_token_account
            writable_meta(auction_pda),             // auction
            writable_meta(bid_escrow_pda),          // bid_escrow
            readonly_meta(token_program_id()),     // token_program
        ],
    );

    let result = execute_tx(&mut svm, claim_ix, &bidder, &[&bidder]);
    assert!(result.is_ok(), "claim_prize should succeed: {:?}", result);

    // Verify seller received payment
    let seller_account = svm.get_account(&seller_token).unwrap();
    assert_eq!(read_token_balance(&seller_account.data), 500);

    // Verify escrow is empty
    let escrow_account = svm.get_account(&bid_escrow_pda).unwrap();
    assert_eq!(read_token_balance(&escrow_account.data), 0);

    // Verify auction is claimed
    let auction_account = svm.get_account(&auction_pda).unwrap();
    assert!(read_auction_is_claimed(&auction_account.data));
}

#[test]
fn test_claim_prize_before_end_fails() {
    let mut svm = load_auction_program();
    let program_id = auction_program_id();

    let (_, mint) = setup_mint(&mut svm, 6);
    let (seller, seller_token) = setup_user_with_tokens(&mut svm, &mint, 0);
    let (bidder, bidder_token) = setup_user_with_tokens(&mut svm, &mint, 10000);

    let auction_id: u64 = 1;
    let (auction_pda, _) = derive_auction_pda(auction_id);
    let (bid_escrow_pda, _) = derive_bid_escrow_pda(auction_id);

    // Create auction and bid but don't end
    let create_ix = anchor_instruction(
        program_id,
        "create_auction",
        &create_auction_data(auction_id, 100, 10000),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &seller, &[&seller]).unwrap();

    svm.expire_blockhash();
    let bid_ix = anchor_instruction(
        program_id,
        "place_bid",
        &place_bid_data(500),
        vec![
            signer_meta(bidder.pubkey()),
            writable_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(bidder_token),
            writable_meta(seller_token),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, bid_ix, &bidder, &[&bidder]).unwrap();

    // Try to claim without ending
    svm.expire_blockhash();
    let claim_ix = anchor_instruction(
        program_id,
        "claim_prize",
        &[],
        vec![
            signer_meta(bidder.pubkey()),
            writable_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(seller_token),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, claim_ix, &bidder, &[&bidder]);
    assert!(result.is_err(), "claim_prize before end should fail");
}

#[test]
fn test_claim_prize_not_winner_fails() {
    let mut svm = load_auction_program();
    let program_id = auction_program_id();

    let (_, mint) = setup_mint(&mut svm, 6);
    let (seller, seller_token) = setup_user_with_tokens(&mut svm, &mint, 0);
    let (bidder, bidder_token) = setup_user_with_tokens(&mut svm, &mint, 10000);
    let (attacker, _) = setup_user_with_tokens(&mut svm, &mint, 10000);

    let auction_id: u64 = 1;
    let (auction_pda, _) = derive_auction_pda(auction_id);
    let (bid_escrow_pda, _) = derive_bid_escrow_pda(auction_id);

    // Create, bid, and end auction
    let create_ix = anchor_instruction(
        program_id,
        "create_auction",
        &create_auction_data(auction_id, 100, 100),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &seller, &[&seller]).unwrap();

    svm.expire_blockhash();
    let bid_ix = anchor_instruction(
        program_id,
        "place_bid",
        &place_bid_data(500),
        vec![
            signer_meta(bidder.pubkey()),
            writable_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(bidder_token),
            writable_meta(seller_token),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, bid_ix, &bidder, &[&bidder]).unwrap();

    let auction_account = svm.get_account(&auction_pda).unwrap();
    let end_slot = read_auction_end_slot(&auction_account.data);
    svm.warp_to_slot(end_slot + 1);
    svm.expire_blockhash();

    let end_ix = anchor_instruction(
        program_id,
        "end_auction",
        &[],
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(auction_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
    );
    execute_tx(&mut svm, end_ix, &seller, &[&seller]).unwrap();

    // Attacker tries to claim
    svm.expire_blockhash();
    let claim_ix = anchor_instruction(
        program_id,
        "claim_prize",
        &[],
        vec![
            signer_meta(attacker.pubkey()),        // Wrong winner!
            writable_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(seller_token),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, claim_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "claim_prize by non-winner should fail");
}

#[test]
fn test_claim_prize_no_bids_fails() {
    let mut svm = load_auction_program();
    let program_id = auction_program_id();

    let (_, mint) = setup_mint(&mut svm, 6);
    let (seller, seller_token) = setup_user_with_tokens(&mut svm, &mint, 0);

    let auction_id: u64 = 1;
    let (auction_pda, _) = derive_auction_pda(auction_id);
    let (bid_escrow_pda, _) = derive_bid_escrow_pda(auction_id);

    // Create and end auction without any bids
    let create_ix = anchor_instruction(
        program_id,
        "create_auction",
        &create_auction_data(auction_id, 100, 100),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &seller, &[&seller]).unwrap();

    let auction_account = svm.get_account(&auction_pda).unwrap();
    let end_slot = read_auction_end_slot(&auction_account.data);
    svm.warp_to_slot(end_slot + 1);
    svm.expire_blockhash();

    let end_ix = anchor_instruction(
        program_id,
        "end_auction",
        &[],
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(auction_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
    );
    execute_tx(&mut svm, end_ix, &seller, &[&seller]).unwrap();

    // Seller tries to claim (they are the "highest bidder" placeholder)
    svm.expire_blockhash();
    let claim_ix = anchor_instruction(
        program_id,
        "claim_prize",
        &[],
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(seller_token),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, claim_ix, &seller, &[&seller]);
    assert!(result.is_err(), "claim_prize with no bids should fail");
}

// =============================================================================
// FULL WORKFLOW TESTS
// =============================================================================

#[test]
fn test_full_auction_workflow() {
    let mut svm = load_auction_program();
    let program_id = auction_program_id();

    let (_, mint) = setup_mint(&mut svm, 6);
    let (seller, seller_token) = setup_user_with_tokens(&mut svm, &mint, 0);
    let (bidder1, bidder1_token) = setup_user_with_tokens(&mut svm, &mint, 10000);
    let (bidder2, bidder2_token) = setup_user_with_tokens(&mut svm, &mint, 10000);

    let auction_id: u64 = 1;
    let (auction_pda, _) = derive_auction_pda(auction_id);
    let (bid_escrow_pda, _) = derive_bid_escrow_pda(auction_id);

    // 1. Create auction
    let create_ix = anchor_instruction(
        program_id,
        "create_auction",
        &create_auction_data(auction_id, 100, 500),
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &seller, &[&seller]).unwrap();

    // 2. Bidder 1 bids 200
    svm.expire_blockhash();
    let bid1_ix = anchor_instruction(
        program_id,
        "place_bid",
        &place_bid_data(200),
        vec![
            signer_meta(bidder1.pubkey()),
            writable_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(bidder1_token),
            writable_meta(seller_token),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, bid1_ix, &bidder1, &[&bidder1]).unwrap();

    // 3. Bidder 2 outbids with 300
    svm.expire_blockhash();
    let bid2_ix = anchor_instruction(
        program_id,
        "place_bid",
        &place_bid_data(300),
        vec![
            signer_meta(bidder2.pubkey()),
            writable_meta(bidder1.pubkey()),
            writable_meta(mint),
            writable_meta(bidder2_token),
            writable_meta(bidder1_token),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, bid2_ix, &bidder2, &[&bidder2]).unwrap();

    // Verify bidder 1 was refunded
    let bidder1_account = svm.get_account(&bidder1_token).unwrap();
    assert_eq!(read_token_balance(&bidder1_account.data), 10000);

    // 4. Bidder 1 counter-bids with 500
    svm.expire_blockhash();
    let bid3_ix = anchor_instruction(
        program_id,
        "place_bid",
        &place_bid_data(500),
        vec![
            signer_meta(bidder1.pubkey()),
            writable_meta(bidder2.pubkey()),
            writable_meta(mint),
            writable_meta(bidder1_token),
            writable_meta(bidder2_token),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, bid3_ix, &bidder1, &[&bidder1]).unwrap();

    // Verify bidder 2 was refunded
    let bidder2_account = svm.get_account(&bidder2_token).unwrap();
    assert_eq!(read_token_balance(&bidder2_account.data), 10000);

    // 5. End auction
    let auction_account = svm.get_account(&auction_pda).unwrap();
    let end_slot = read_auction_end_slot(&auction_account.data);
    svm.warp_to_slot(end_slot + 1);
    svm.expire_blockhash();

    let end_ix = anchor_instruction(
        program_id,
        "end_auction",
        &[],
        vec![
            signer_meta(seller.pubkey()),
            writable_meta(auction_pda),
            readonly_meta(solana_sdk_ids::sysvar::clock::id()),
        ],
    );
    execute_tx(&mut svm, end_ix, &seller, &[&seller]).unwrap();

    // 6. Winner (bidder 1) claims prize
    svm.expire_blockhash();
    let claim_ix = anchor_instruction(
        program_id,
        "claim_prize",
        &[],
        vec![
            signer_meta(bidder1.pubkey()),
            writable_meta(seller.pubkey()),
            writable_meta(mint),
            writable_meta(seller_token),
            writable_meta(auction_pda),
            writable_meta(bid_escrow_pda),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, claim_ix, &bidder1, &[&bidder1]).unwrap();

    // Verify final state
    let seller_account = svm.get_account(&seller_token).unwrap();
    assert_eq!(read_token_balance(&seller_account.data), 500, "Seller should receive winning bid");

    let bidder1_account = svm.get_account(&bidder1_token).unwrap();
    assert_eq!(read_token_balance(&bidder1_account.data), 9500, "Bidder 1 paid 500");

    let bidder2_account = svm.get_account(&bidder2_token).unwrap();
    assert_eq!(read_token_balance(&bidder2_account.data), 10000, "Bidder 2 fully refunded");

    let escrow_account = svm.get_account(&bid_escrow_pda).unwrap();
    assert_eq!(read_token_balance(&escrow_account.data), 0, "Escrow should be empty");

    let auction_account = svm.get_account(&auction_pda).unwrap();
    assert!(read_auction_is_ended(&auction_account.data));
    assert!(read_auction_is_claimed(&auction_account.data));
    assert_eq!(read_auction_highest_bidder(&auction_account.data), bidder1.pubkey());
    assert_eq!(read_auction_current_bid(&auction_account.data), 500);
}
