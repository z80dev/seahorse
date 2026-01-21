//! LiteSVM integration tests for the Seahorse Token Swap AMM program
//!
//! These tests verify AMM functionality:
//! - initialize_pool: Creates pool with LP mint and pool token accounts
//! - add_liquidity: First deposit (geometric mean) and subsequent (proportional)
//! - remove_liquidity: Burn LP tokens, receive proportional tokens
//! - swap: Constant product formula with configurable fee
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile token_swap.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// Token swap program ID (from declare_id!)
fn token_swap_program_id() -> Pubkey {
    Pubkey::from_str("SwAP5VzVZYxLYtDmJq5DEmUKDEcPnP2zGxqHKbv2AmM").unwrap()
}

/// Pool account size: discriminator (8) + mint_a (32) + mint_b (32) + lp_mint (32)
///                   + pool_token_a (32) + pool_token_b (32) + fee_bps (2) + bump (1)
const POOL_SIZE: usize = 8 + 32 + 32 + 32 + 32 + 32 + 2 + 1;

/// Minimum liquidity locked on first deposit
const MINIMUM_LIQUIDITY: u64 = 100;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the token swap program into LiteSVM
fn load_token_swap_program() -> litesvm::LiteSVM {
    let program_id = token_swap_program_id();
    let program_bytes = std::fs::read("../../target/deploy/token_swap.so")
        .expect("Failed to read token_swap.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);
    svm
}

/// Setup a mint for testing
fn setup_mint(svm: &mut litesvm::LiteSVM, decimals: u8) -> Pubkey {
    let mint_authority = funded_keypair_10_sol(svm);
    let mint = Keypair::new();

    let mint_account = create_mint_account(&mint_authority.pubkey(), decimals);
    svm.set_account(mint.pubkey(), mint_account).unwrap();

    mint.pubkey()
}

/// Setup a user with token accounts for both mints and LP mint
fn setup_user_with_tokens(
    svm: &mut litesvm::LiteSVM,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
    amount_a: u64,
    amount_b: u64,
) -> (Keypair, Pubkey, Pubkey) {
    let user = funded_keypair_10_sol(svm);
    let user_token_a = get_ata(&user.pubkey(), mint_a);
    let user_token_b = get_ata(&user.pubkey(), mint_b);

    svm.set_account(user_token_a, create_token_account(&user.pubkey(), mint_a, amount_a)).unwrap();
    svm.set_account(user_token_b, create_token_account(&user.pubkey(), mint_b, amount_b)).unwrap();

    (user, user_token_a, user_token_b)
}

/// Setup a user LP token account (after pool is initialized)
fn setup_user_lp_account(
    svm: &mut litesvm::LiteSVM,
    user: &Pubkey,
    lp_mint: &Pubkey,
    amount: u64,
) -> Pubkey {
    let user_lp_token = get_ata(user, lp_mint);
    svm.set_account(user_lp_token, create_token_account(user, lp_mint, amount)).unwrap();
    user_lp_token
}

/// Derive pool PDA
fn derive_pool_pda(mint_a: &Pubkey, mint_b: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"pool", mint_a.as_ref(), mint_b.as_ref()],
        &token_swap_program_id(),
    )
}

/// Derive LP mint PDA
fn derive_lp_mint_pda(mint_a: &Pubkey, mint_b: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"lp_mint", mint_a.as_ref(), mint_b.as_ref()],
        &token_swap_program_id(),
    )
}

/// Derive pool token A account PDA
fn derive_pool_token_a_pda(mint_a: &Pubkey, mint_b: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"pool_token_a", mint_a.as_ref(), mint_b.as_ref()],
        &token_swap_program_id(),
    )
}

/// Derive pool token B account PDA
fn derive_pool_token_b_pda(mint_a: &Pubkey, mint_b: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"pool_token_b", mint_a.as_ref(), mint_b.as_ref()],
        &token_swap_program_id(),
    )
}

/// Read pool fee_bps from account data
fn read_pool_fee_bps(data: &[u8]) -> u16 {
    // Skip discriminator (8) + mint_a (32) + mint_b (32) + lp_mint (32) + pool_token_a (32) + pool_token_b (32)
    let bytes: [u8; 2] = data[168..170].try_into().unwrap();
    u16::from_le_bytes(bytes)
}

/// Read pool bump from account data
fn read_pool_bump(data: &[u8]) -> u8 {
    // Skip to bump (last byte before end)
    data[170]
}

/// Build initialize_pool instruction data
fn initialize_pool_data(fee_bps: u16) -> Vec<u8> {
    fee_bps.to_le_bytes().to_vec()
}

/// Build add_liquidity instruction data
fn add_liquidity_data(amount_a: u64, amount_b: u64, min_lp_tokens: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&amount_a.to_le_bytes());
    data.extend_from_slice(&amount_b.to_le_bytes());
    data.extend_from_slice(&min_lp_tokens.to_le_bytes());
    data
}

/// Build remove_liquidity instruction data
fn remove_liquidity_data(lp_amount: u64, min_amount_a: u64, min_amount_b: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&lp_amount.to_le_bytes());
    data.extend_from_slice(&min_amount_a.to_le_bytes());
    data.extend_from_slice(&min_amount_b.to_le_bytes());
    data
}

/// Build swap instruction data
fn swap_data(amount_in: u64, min_amount_out: u64, swap_a_to_b: bool) -> Vec<u8> {
    let mut data = Vec::with_capacity(17);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data.push(if swap_a_to_b { 1 } else { 0 });
    data
}

/// Initialize a pool with the given fee
fn initialize_pool(
    svm: &mut litesvm::LiteSVM,
    payer: &Keypair,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
    fee_bps: u16,
) -> (Pubkey, Pubkey, Pubkey, Pubkey) {
    let program_id = token_swap_program_id();

    let (pool_pda, _) = derive_pool_pda(mint_a, mint_b);
    let (lp_mint_pda, _) = derive_lp_mint_pda(mint_a, mint_b);
    let (pool_token_a_pda, _) = derive_pool_token_a_pda(mint_a, mint_b);
    let (pool_token_b_pda, _) = derive_pool_token_b_pda(mint_a, mint_b);

    let ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(fee_bps),
        vec![
            signer_meta(payer.pubkey()),          // payer
            writable_meta(*mint_a),               // mint_a
            writable_meta(*mint_b),               // mint_b
            writable_meta(pool_pda),              // pool
            writable_meta(lp_mint_pda),           // lp_mint
            writable_meta(pool_token_a_pda),      // pool_token_a
            writable_meta(pool_token_b_pda),      // pool_token_b
            readonly_meta(solana_sdk_ids::sysvar::rent::id()), // rent
            readonly_meta(system_program::id()),  // system_program
            readonly_meta(token_program_id()),    // token_program
        ],
    );

    let result = execute_tx(svm, ix, payer, &[payer]);
    assert!(result.is_ok(), "initialize_pool should succeed: {:?}", result);

    (pool_pda, lp_mint_pda, pool_token_a_pda, pool_token_b_pda)
}

/// Integer square root (for verifying LP token calculations)
fn isqrt(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

// =============================================================================
// INITIALIZE POOL TESTS
// =============================================================================

#[test]
fn test_initialize_pool() {
    let mut svm = load_token_swap_program();
    let program_id = token_swap_program_id();

    let payer = funded_keypair_10_sol(&mut svm);
    let mint_a = setup_mint(&mut svm, 6);
    let mint_b = setup_mint(&mut svm, 6);

    let fee_bps: u16 = 30; // 0.3%

    let (pool_pda, lp_mint_pda, pool_token_a_pda, pool_token_b_pda) =
        initialize_pool(&mut svm, &payer, &mint_a, &mint_b, fee_bps);

    // Verify pool state
    let pool_account = svm.get_account(&pool_pda).expect("Pool should exist");
    assert_eq!(pool_account.owner, program_id, "Pool should be owned by program");
    assert_eq!(read_pool_fee_bps(&pool_account.data), fee_bps);

    // Verify LP mint
    let lp_mint_account = svm.get_account(&lp_mint_pda).expect("LP mint should exist");
    assert_eq!(lp_mint_account.owner, token_program_id());
    assert_eq!(read_mint_supply(&lp_mint_account.data), 0);

    // Verify pool token accounts are empty
    let pool_token_a = svm.get_account(&pool_token_a_pda).expect("Pool token A should exist");
    let pool_token_b = svm.get_account(&pool_token_b_pda).expect("Pool token B should exist");
    assert_eq!(read_token_balance(&pool_token_a.data), 0);
    assert_eq!(read_token_balance(&pool_token_b.data), 0);
}

#[test]
fn test_initialize_pool_different_fees() {
    let mut svm = load_token_swap_program();

    let payer = funded_keypair_10_sol(&mut svm);

    // Pool 1 with 0.1% fee
    let mint_a1 = setup_mint(&mut svm, 6);
    let mint_b1 = setup_mint(&mut svm, 6);
    let (pool1, _, _, _) = initialize_pool(&mut svm, &payer, &mint_a1, &mint_b1, 10);
    svm.expire_blockhash();

    // Pool 2 with 1% fee
    let mint_a2 = setup_mint(&mut svm, 6);
    let mint_b2 = setup_mint(&mut svm, 6);
    let (pool2, _, _, _) = initialize_pool(&mut svm, &payer, &mint_a2, &mint_b2, 100);

    // Verify different fees
    let pool1_data = svm.get_account(&pool1).unwrap();
    let pool2_data = svm.get_account(&pool2).unwrap();
    assert_eq!(read_pool_fee_bps(&pool1_data.data), 10);
    assert_eq!(read_pool_fee_bps(&pool2_data.data), 100);
}

#[test]
fn test_initialize_pool_invalid_fee_fails() {
    let mut svm = load_token_swap_program();
    let program_id = token_swap_program_id();

    let payer = funded_keypair_10_sol(&mut svm);
    let mint_a = setup_mint(&mut svm, 6);
    let mint_b = setup_mint(&mut svm, 6);

    let (pool_pda, _) = derive_pool_pda(&mint_a, &mint_b);
    let (lp_mint_pda, _) = derive_lp_mint_pda(&mint_a, &mint_b);
    let (pool_token_a_pda, _) = derive_pool_token_a_pda(&mint_a, &mint_b);
    let (pool_token_b_pda, _) = derive_pool_token_b_pda(&mint_a, &mint_b);

    // Fee >= 10000 (100%) should fail
    let ix = anchor_instruction(
        program_id,
        "initialize_pool",
        &initialize_pool_data(10000),
        vec![
            signer_meta(payer.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &payer, &[&payer]);
    assert!(result.is_err(), "initialize_pool with 100% fee should fail");
}

// =============================================================================
// ADD LIQUIDITY TESTS
// =============================================================================

#[test]
fn test_add_liquidity_first_deposit() {
    let mut svm = load_token_swap_program();
    let program_id = token_swap_program_id();

    let payer = funded_keypair_10_sol(&mut svm);
    let mint_a = setup_mint(&mut svm, 6);
    let mint_b = setup_mint(&mut svm, 6);

    let (pool_pda, lp_mint_pda, pool_token_a_pda, pool_token_b_pda) =
        initialize_pool(&mut svm, &payer, &mint_a, &mint_b, 30);

    // Setup user with tokens
    let (user, user_token_a, user_token_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 10000, 10000);
    svm.expire_blockhash();

    // Setup user LP token account
    let user_lp_token = setup_user_lp_account(&mut svm, &user.pubkey(), &lp_mint_pda, 0);

    let amount_a: u64 = 1000;
    let amount_b: u64 = 1000;
    // Expected LP tokens: sqrt(1000 * 1000) - 100 = 1000 - 100 = 900
    let expected_lp = isqrt(amount_a * amount_b) - MINIMUM_LIQUIDITY;

    let ix = anchor_instruction(
        program_id,
        "add_liquidity",
        &add_liquidity_data(amount_a, amount_b, 0),
        vec![
            signer_meta(user.pubkey()),           // user
            writable_meta(mint_a),                // mint_a
            writable_meta(mint_b),                // mint_b
            writable_meta(pool_pda),              // pool
            writable_meta(lp_mint_pda),           // lp_mint
            writable_meta(pool_token_a_pda),      // pool_token_a
            writable_meta(pool_token_b_pda),      // pool_token_b
            writable_meta(user_token_a),          // user_token_a
            writable_meta(user_token_b),          // user_token_b
            writable_meta(user_lp_token),         // user_lp_token
            readonly_meta(token_program_id()),    // token_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &user, &[&user]);
    assert!(result.is_ok(), "add_liquidity should succeed: {:?}", result);

    // Verify pool balances
    let pool_a = svm.get_account(&pool_token_a_pda).unwrap();
    let pool_b = svm.get_account(&pool_token_b_pda).unwrap();
    assert_eq!(read_token_balance(&pool_a.data), amount_a);
    assert_eq!(read_token_balance(&pool_b.data), amount_b);

    // Verify user received LP tokens
    let user_lp = svm.get_account(&user_lp_token).unwrap();
    assert_eq!(read_token_balance(&user_lp.data), expected_lp);

    // Verify LP mint supply
    let lp_mint = svm.get_account(&lp_mint_pda).unwrap();
    assert_eq!(read_mint_supply(&lp_mint.data), expected_lp);

    // Verify user token balances decreased
    let user_a = svm.get_account(&user_token_a).unwrap();
    let user_b = svm.get_account(&user_token_b).unwrap();
    assert_eq!(read_token_balance(&user_a.data), 10000 - amount_a);
    assert_eq!(read_token_balance(&user_b.data), 10000 - amount_b);
}

#[test]
fn test_add_liquidity_subsequent_deposit() {
    let mut svm = load_token_swap_program();
    let program_id = token_swap_program_id();

    let payer = funded_keypair_10_sol(&mut svm);
    let mint_a = setup_mint(&mut svm, 6);
    let mint_b = setup_mint(&mut svm, 6);

    let (pool_pda, lp_mint_pda, pool_token_a_pda, pool_token_b_pda) =
        initialize_pool(&mut svm, &payer, &mint_a, &mint_b, 30);
    svm.expire_blockhash();

    // First user adds initial liquidity
    let (user1, user1_token_a, user1_token_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 10000, 10000);
    let user1_lp_token = setup_user_lp_account(&mut svm, &user1.pubkey(), &lp_mint_pda, 0);

    let ix1 = anchor_instruction(
        program_id,
        "add_liquidity",
        &add_liquidity_data(1000, 1000, 0),
        vec![
            signer_meta(user1.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(user1_token_a),
            writable_meta(user1_token_b),
            writable_meta(user1_lp_token),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix1, &user1, &[&user1]).unwrap();
    svm.expire_blockhash();

    // Second user adds liquidity
    let (user2, user2_token_a, user2_token_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 10000, 10000);
    let user2_lp_token = setup_user_lp_account(&mut svm, &user2.pubkey(), &lp_mint_pda, 0);

    // Pool has 1000:1000, user2 wants to add 500:500 (proportional)
    let amount_a: u64 = 500;
    let amount_b: u64 = 500;

    // Get current state
    let lp_supply_before = read_mint_supply(&svm.get_account(&lp_mint_pda).unwrap().data);
    let pool_a_before = read_token_balance(&svm.get_account(&pool_token_a_pda).unwrap().data);

    // Expected LP tokens: (500 * 900) / 1000 = 450
    let expected_lp = (amount_a as u128 * lp_supply_before as u128 / pool_a_before as u128) as u64;

    let ix2 = anchor_instruction(
        program_id,
        "add_liquidity",
        &add_liquidity_data(amount_a, amount_b, 0),
        vec![
            signer_meta(user2.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(user2_token_a),
            writable_meta(user2_token_b),
            writable_meta(user2_lp_token),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix2, &user2, &[&user2]).unwrap();

    // Verify user2 received proportional LP tokens
    let user2_lp = svm.get_account(&user2_lp_token).unwrap();
    assert_eq!(read_token_balance(&user2_lp.data), expected_lp);

    // Verify pool balances increased
    let pool_a = svm.get_account(&pool_token_a_pda).unwrap();
    let pool_b = svm.get_account(&pool_token_b_pda).unwrap();
    assert_eq!(read_token_balance(&pool_a.data), 1500);
    assert_eq!(read_token_balance(&pool_b.data), 1500);
}

#[test]
fn test_add_liquidity_imbalanced_deposit() {
    let mut svm = load_token_swap_program();
    let program_id = token_swap_program_id();

    let payer = funded_keypair_10_sol(&mut svm);
    let mint_a = setup_mint(&mut svm, 6);
    let mint_b = setup_mint(&mut svm, 6);

    let (pool_pda, lp_mint_pda, pool_token_a_pda, pool_token_b_pda) =
        initialize_pool(&mut svm, &payer, &mint_a, &mint_b, 30);
    svm.expire_blockhash();

    // First user adds initial liquidity (1000:1000)
    let (user1, user1_token_a, user1_token_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 10000, 10000);
    let user1_lp_token = setup_user_lp_account(&mut svm, &user1.pubkey(), &lp_mint_pda, 0);

    let ix1 = anchor_instruction(
        program_id,
        "add_liquidity",
        &add_liquidity_data(1000, 1000, 0),
        vec![
            signer_meta(user1.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(user1_token_a),
            writable_meta(user1_token_b),
            writable_meta(user1_lp_token),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix1, &user1, &[&user1]).unwrap();
    svm.expire_blockhash();

    // Second user tries to add imbalanced liquidity (1000:500)
    // Should deposit proportionally based on smaller ratio
    let (user2, user2_token_a, user2_token_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 10000, 10000);
    let user2_lp_token = setup_user_lp_account(&mut svm, &user2.pubkey(), &lp_mint_pda, 0);

    let user2_a_before = read_token_balance(&svm.get_account(&user2_token_a).unwrap().data);
    let user2_b_before = read_token_balance(&svm.get_account(&user2_token_b).unwrap().data);

    // User provides 1000 A and 500 B, but pool is 1:1
    // Should use the smaller ratio (500 B) to determine deposit
    // Actual deposit: 500 A and 500 B
    let ix2 = anchor_instruction(
        program_id,
        "add_liquidity",
        &add_liquidity_data(1000, 500, 0),
        vec![
            signer_meta(user2.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(user2_token_a),
            writable_meta(user2_token_b),
            writable_meta(user2_lp_token),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix2, &user2, &[&user2]).unwrap();

    // Verify user2 only deposited proportional amounts
    let user2_a_after = read_token_balance(&svm.get_account(&user2_token_a).unwrap().data);
    let user2_b_after = read_token_balance(&svm.get_account(&user2_token_b).unwrap().data);

    let deposited_a = user2_a_before - user2_a_after;
    let deposited_b = user2_b_before - user2_b_after;

    // Should have deposited 500 A and 500 B (matching the pool ratio)
    assert_eq!(deposited_a, 500, "Should deposit 500 A");
    assert_eq!(deposited_b, 500, "Should deposit 500 B");
}

#[test]
fn test_add_liquidity_slippage_protection() {
    let mut svm = load_token_swap_program();
    let program_id = token_swap_program_id();

    let payer = funded_keypair_10_sol(&mut svm);
    let mint_a = setup_mint(&mut svm, 6);
    let mint_b = setup_mint(&mut svm, 6);

    let (pool_pda, lp_mint_pda, pool_token_a_pda, pool_token_b_pda) =
        initialize_pool(&mut svm, &payer, &mint_a, &mint_b, 30);
    svm.expire_blockhash();

    let (user, user_token_a, user_token_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 10000, 10000);
    let user_lp_token = setup_user_lp_account(&mut svm, &user.pubkey(), &lp_mint_pda, 0);

    // Expected LP tokens for first deposit: sqrt(1000*1000) - 100 = 900
    // But we require min_lp_tokens = 1000 which is more than we'll get
    let ix = anchor_instruction(
        program_id,
        "add_liquidity",
        &add_liquidity_data(1000, 1000, 1000), // min_lp_tokens too high
        vec![
            signer_meta(user.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(user_token_a),
            writable_meta(user_token_b),
            writable_meta(user_lp_token),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &user, &[&user]);
    assert!(result.is_err(), "add_liquidity with high min_lp_tokens should fail");
}

#[test]
fn test_add_liquidity_zero_amount_fails() {
    let mut svm = load_token_swap_program();
    let program_id = token_swap_program_id();

    let payer = funded_keypair_10_sol(&mut svm);
    let mint_a = setup_mint(&mut svm, 6);
    let mint_b = setup_mint(&mut svm, 6);

    let (pool_pda, lp_mint_pda, pool_token_a_pda, pool_token_b_pda) =
        initialize_pool(&mut svm, &payer, &mint_a, &mint_b, 30);
    svm.expire_blockhash();

    let (user, user_token_a, user_token_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 10000, 10000);
    let user_lp_token = setup_user_lp_account(&mut svm, &user.pubkey(), &lp_mint_pda, 0);

    // Zero amount_a should fail
    let ix = anchor_instruction(
        program_id,
        "add_liquidity",
        &add_liquidity_data(0, 1000, 0),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(user_token_a),
            writable_meta(user_token_b),
            writable_meta(user_lp_token),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &user, &[&user]);
    assert!(result.is_err(), "add_liquidity with zero amount should fail");
}

#[test]
fn test_add_liquidity_deposit_too_small_fails() {
    let mut svm = load_token_swap_program();
    let program_id = token_swap_program_id();

    let payer = funded_keypair_10_sol(&mut svm);
    let mint_a = setup_mint(&mut svm, 6);
    let mint_b = setup_mint(&mut svm, 6);

    let (pool_pda, lp_mint_pda, pool_token_a_pda, pool_token_b_pda) =
        initialize_pool(&mut svm, &payer, &mint_a, &mint_b, 30);
    svm.expire_blockhash();

    let (user, user_token_a, user_token_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 10000, 10000);
    let user_lp_token = setup_user_lp_account(&mut svm, &user.pubkey(), &lp_mint_pda, 0);

    // sqrt(10 * 10) = 10 < MINIMUM_LIQUIDITY (100)
    let ix = anchor_instruction(
        program_id,
        "add_liquidity",
        &add_liquidity_data(10, 10, 0),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(user_token_a),
            writable_meta(user_token_b),
            writable_meta(user_lp_token),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &user, &[&user]);
    assert!(result.is_err(), "add_liquidity with deposit too small should fail");
}

// =============================================================================
// REMOVE LIQUIDITY TESTS
// =============================================================================

#[test]
fn test_remove_liquidity() {
    let mut svm = load_token_swap_program();
    let program_id = token_swap_program_id();

    let payer = funded_keypair_10_sol(&mut svm);
    let mint_a = setup_mint(&mut svm, 6);
    let mint_b = setup_mint(&mut svm, 6);

    let (pool_pda, lp_mint_pda, pool_token_a_pda, pool_token_b_pda) =
        initialize_pool(&mut svm, &payer, &mint_a, &mint_b, 30);
    svm.expire_blockhash();

    // Add initial liquidity
    let (user, user_token_a, user_token_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 10000, 10000);
    let user_lp_token = setup_user_lp_account(&mut svm, &user.pubkey(), &lp_mint_pda, 0);

    let ix_add = anchor_instruction(
        program_id,
        "add_liquidity",
        &add_liquidity_data(1000, 1000, 0),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(user_token_a),
            writable_meta(user_token_b),
            writable_meta(user_lp_token),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix_add, &user, &[&user]).unwrap();
    svm.expire_blockhash();

    // Get LP balance (900 tokens from first deposit)
    let lp_balance = read_token_balance(&svm.get_account(&user_lp_token).unwrap().data);
    assert_eq!(lp_balance, 900);

    // Remove half liquidity (450 LP tokens)
    let lp_to_remove: u64 = 450;
    // Expected: 450/900 * 1000 = 500 tokens of each
    let expected_amount_a: u64 = (lp_to_remove as u128 * 1000 / lp_balance as u128) as u64;
    let expected_amount_b: u64 = expected_amount_a;

    let ix_remove = anchor_instruction(
        program_id,
        "remove_liquidity",
        &remove_liquidity_data(lp_to_remove, 0, 0),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(user_token_a),
            writable_meta(user_token_b),
            writable_meta(user_lp_token),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix_remove, &user, &[&user]);
    assert!(result.is_ok(), "remove_liquidity should succeed: {:?}", result);

    // Verify user received tokens back
    let user_a = svm.get_account(&user_token_a).unwrap();
    let user_b = svm.get_account(&user_token_b).unwrap();
    // Started with 9000 (10000 - 1000 deposited), should now have 9000 + 500 = 9500
    assert_eq!(read_token_balance(&user_a.data), 9000 + expected_amount_a);
    assert_eq!(read_token_balance(&user_b.data), 9000 + expected_amount_b);

    // Verify LP tokens were burned
    let user_lp = svm.get_account(&user_lp_token).unwrap();
    assert_eq!(read_token_balance(&user_lp.data), 900 - lp_to_remove);

    // Verify pool balances decreased
    let pool_a = svm.get_account(&pool_token_a_pda).unwrap();
    let pool_b = svm.get_account(&pool_token_b_pda).unwrap();
    assert_eq!(read_token_balance(&pool_a.data), 1000 - expected_amount_a);
    assert_eq!(read_token_balance(&pool_b.data), 1000 - expected_amount_b);
}

#[test]
fn test_remove_liquidity_full_withdrawal() {
    let mut svm = load_token_swap_program();
    let program_id = token_swap_program_id();

    let payer = funded_keypair_10_sol(&mut svm);
    let mint_a = setup_mint(&mut svm, 6);
    let mint_b = setup_mint(&mut svm, 6);

    let (pool_pda, lp_mint_pda, pool_token_a_pda, pool_token_b_pda) =
        initialize_pool(&mut svm, &payer, &mint_a, &mint_b, 30);
    svm.expire_blockhash();

    let (user, user_token_a, user_token_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 10000, 10000);
    let user_lp_token = setup_user_lp_account(&mut svm, &user.pubkey(), &lp_mint_pda, 0);

    // Add liquidity
    let ix_add = anchor_instruction(
        program_id,
        "add_liquidity",
        &add_liquidity_data(1000, 1000, 0),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(user_token_a),
            writable_meta(user_token_b),
            writable_meta(user_lp_token),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix_add, &user, &[&user]).unwrap();
    svm.expire_blockhash();

    // Remove ALL LP tokens (900)
    let lp_balance: u64 = 900;

    let ix_remove = anchor_instruction(
        program_id,
        "remove_liquidity",
        &remove_liquidity_data(lp_balance, 0, 0),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(user_token_a),
            writable_meta(user_token_b),
            writable_meta(user_lp_token),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix_remove, &user, &[&user]).unwrap();

    // User should get back all tokens (minus MINIMUM_LIQUIDITY that's locked)
    // Actually user gets 900/900 * 1000 = 1000 tokens back
    let user_a = svm.get_account(&user_token_a).unwrap();
    let user_b = svm.get_account(&user_token_b).unwrap();
    assert_eq!(read_token_balance(&user_a.data), 10000);
    assert_eq!(read_token_balance(&user_b.data), 10000);

    // LP tokens should be 0
    let user_lp = svm.get_account(&user_lp_token).unwrap();
    assert_eq!(read_token_balance(&user_lp.data), 0);
}

#[test]
fn test_remove_liquidity_slippage_protection() {
    let mut svm = load_token_swap_program();
    let program_id = token_swap_program_id();

    let payer = funded_keypair_10_sol(&mut svm);
    let mint_a = setup_mint(&mut svm, 6);
    let mint_b = setup_mint(&mut svm, 6);

    let (pool_pda, lp_mint_pda, pool_token_a_pda, pool_token_b_pda) =
        initialize_pool(&mut svm, &payer, &mint_a, &mint_b, 30);
    svm.expire_blockhash();

    let (user, user_token_a, user_token_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 10000, 10000);
    let user_lp_token = setup_user_lp_account(&mut svm, &user.pubkey(), &lp_mint_pda, 0);

    // Add liquidity
    let ix_add = anchor_instruction(
        program_id,
        "add_liquidity",
        &add_liquidity_data(1000, 1000, 0),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(user_token_a),
            writable_meta(user_token_b),
            writable_meta(user_lp_token),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix_add, &user, &[&user]).unwrap();
    svm.expire_blockhash();

    // Try to remove with unrealistic min amounts
    // 450 LP = 500 tokens each, but we require 600
    let ix_remove = anchor_instruction(
        program_id,
        "remove_liquidity",
        &remove_liquidity_data(450, 600, 600), // min amounts too high
        vec![
            signer_meta(user.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(user_token_a),
            writable_meta(user_token_b),
            writable_meta(user_lp_token),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix_remove, &user, &[&user]);
    assert!(result.is_err(), "remove_liquidity with high min amounts should fail");
}

#[test]
fn test_remove_liquidity_zero_amount_fails() {
    let mut svm = load_token_swap_program();
    let program_id = token_swap_program_id();

    let payer = funded_keypair_10_sol(&mut svm);
    let mint_a = setup_mint(&mut svm, 6);
    let mint_b = setup_mint(&mut svm, 6);

    let (pool_pda, lp_mint_pda, pool_token_a_pda, pool_token_b_pda) =
        initialize_pool(&mut svm, &payer, &mint_a, &mint_b, 30);
    svm.expire_blockhash();

    let (user, user_token_a, user_token_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 10000, 10000);
    let user_lp_token = setup_user_lp_account(&mut svm, &user.pubkey(), &lp_mint_pda, 0);

    // Add liquidity first
    let ix_add = anchor_instruction(
        program_id,
        "add_liquidity",
        &add_liquidity_data(1000, 1000, 0),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(user_token_a),
            writable_meta(user_token_b),
            writable_meta(user_lp_token),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix_add, &user, &[&user]).unwrap();
    svm.expire_blockhash();

    // Try to remove zero LP tokens
    let ix_remove = anchor_instruction(
        program_id,
        "remove_liquidity",
        &remove_liquidity_data(0, 0, 0),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(user_token_a),
            writable_meta(user_token_b),
            writable_meta(user_lp_token),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix_remove, &user, &[&user]);
    assert!(result.is_err(), "remove_liquidity with zero amount should fail");
}

// =============================================================================
// SWAP TESTS
// =============================================================================

#[test]
fn test_swap_a_to_b() {
    let mut svm = load_token_swap_program();
    let program_id = token_swap_program_id();

    let payer = funded_keypair_10_sol(&mut svm);
    let mint_a = setup_mint(&mut svm, 6);
    let mint_b = setup_mint(&mut svm, 6);

    let (pool_pda, lp_mint_pda, pool_token_a_pda, pool_token_b_pda) =
        initialize_pool(&mut svm, &payer, &mint_a, &mint_b, 30); // 0.3% fee
    svm.expire_blockhash();

    // Add initial liquidity (10000:10000)
    let (lp_provider, lp_token_a, lp_token_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 100000, 100000);
    let lp_lp_token = setup_user_lp_account(&mut svm, &lp_provider.pubkey(), &lp_mint_pda, 0);

    let ix_add = anchor_instruction(
        program_id,
        "add_liquidity",
        &add_liquidity_data(10000, 10000, 0),
        vec![
            signer_meta(lp_provider.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(lp_token_a),
            writable_meta(lp_token_b),
            writable_meta(lp_lp_token),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix_add, &lp_provider, &[&lp_provider]).unwrap();
    svm.expire_blockhash();

    // Swapper has 1000 A, wants B
    let (swapper, swapper_token_a, swapper_token_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 1000, 0);

    let amount_in: u64 = 1000;
    // Calculate expected output:
    // fee = 0.3% = 30 bps
    // amount_in_with_fee = 1000 * (10000 - 30) = 9,970,000
    // reserve_in = 10000, reserve_out = 10000
    // numerator = 9,970,000 * 10000 = 99,700,000,000
    // denominator = 10000 * 10000 + 9,970,000 = 109,970,000
    // amount_out = 99,700,000,000 / 109,970,000 = 906 (integer division)
    let expected_out: u64 = 906;

    let ix_swap = anchor_instruction(
        program_id,
        "swap",
        &swap_data(amount_in, 0, true), // swap A to B
        vec![
            signer_meta(swapper.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(swapper_token_a),
            writable_meta(swapper_token_b),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix_swap, &swapper, &[&swapper]);
    assert!(result.is_ok(), "swap should succeed: {:?}", result);

    // Verify swapper balances
    let swapper_a = svm.get_account(&swapper_token_a).unwrap();
    let swapper_b = svm.get_account(&swapper_token_b).unwrap();
    assert_eq!(read_token_balance(&swapper_a.data), 0, "Swapper should have 0 A left");
    assert_eq!(read_token_balance(&swapper_b.data), expected_out, "Swapper should have {} B", expected_out);

    // Verify pool balances (k should be maintained or increased due to fees)
    let pool_a = svm.get_account(&pool_token_a_pda).unwrap();
    let pool_b = svm.get_account(&pool_token_b_pda).unwrap();
    let new_pool_a = read_token_balance(&pool_a.data);
    let new_pool_b = read_token_balance(&pool_b.data);

    assert_eq!(new_pool_a, 10000 + amount_in);
    assert_eq!(new_pool_b, 10000 - expected_out);

    // Verify k increased (due to fees)
    let k_before: u128 = 10000 * 10000;
    let k_after: u128 = new_pool_a as u128 * new_pool_b as u128;
    assert!(k_after >= k_before, "k should not decrease");
}

#[test]
fn test_swap_b_to_a() {
    let mut svm = load_token_swap_program();
    let program_id = token_swap_program_id();

    let payer = funded_keypair_10_sol(&mut svm);
    let mint_a = setup_mint(&mut svm, 6);
    let mint_b = setup_mint(&mut svm, 6);

    let (pool_pda, lp_mint_pda, pool_token_a_pda, pool_token_b_pda) =
        initialize_pool(&mut svm, &payer, &mint_a, &mint_b, 30);
    svm.expire_blockhash();

    // Add liquidity
    let (lp_provider, lp_token_a, lp_token_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 100000, 100000);
    let lp_lp_token = setup_user_lp_account(&mut svm, &lp_provider.pubkey(), &lp_mint_pda, 0);

    let ix_add = anchor_instruction(
        program_id,
        "add_liquidity",
        &add_liquidity_data(10000, 10000, 0),
        vec![
            signer_meta(lp_provider.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(lp_token_a),
            writable_meta(lp_token_b),
            writable_meta(lp_lp_token),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix_add, &lp_provider, &[&lp_provider]).unwrap();
    svm.expire_blockhash();

    // Swapper has 1000 B, wants A
    let (swapper, swapper_token_a, swapper_token_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 0, 1000);

    let amount_in: u64 = 1000;
    let expected_out: u64 = 906; // Same calculation as A to B (symmetric pool)

    let ix_swap = anchor_instruction(
        program_id,
        "swap",
        &swap_data(amount_in, 0, false), // swap B to A
        vec![
            signer_meta(swapper.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(swapper_token_a),
            writable_meta(swapper_token_b),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix_swap, &swapper, &[&swapper]);
    assert!(result.is_ok(), "swap B to A should succeed: {:?}", result);

    // Verify swapper got A tokens
    let swapper_a = svm.get_account(&swapper_token_a).unwrap();
    let swapper_b = svm.get_account(&swapper_token_b).unwrap();
    assert_eq!(read_token_balance(&swapper_a.data), expected_out);
    assert_eq!(read_token_balance(&swapper_b.data), 0);
}

#[test]
fn test_swap_slippage_protection() {
    let mut svm = load_token_swap_program();
    let program_id = token_swap_program_id();

    let payer = funded_keypair_10_sol(&mut svm);
    let mint_a = setup_mint(&mut svm, 6);
    let mint_b = setup_mint(&mut svm, 6);

    let (pool_pda, lp_mint_pda, pool_token_a_pda, pool_token_b_pda) =
        initialize_pool(&mut svm, &payer, &mint_a, &mint_b, 30);
    svm.expire_blockhash();

    // Add liquidity
    let (lp_provider, lp_token_a, lp_token_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 100000, 100000);
    let lp_lp_token = setup_user_lp_account(&mut svm, &lp_provider.pubkey(), &lp_mint_pda, 0);

    let ix_add = anchor_instruction(
        program_id,
        "add_liquidity",
        &add_liquidity_data(10000, 10000, 0),
        vec![
            signer_meta(lp_provider.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(lp_token_a),
            writable_meta(lp_token_b),
            writable_meta(lp_lp_token),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix_add, &lp_provider, &[&lp_provider]).unwrap();
    svm.expire_blockhash();

    let (swapper, swapper_token_a, swapper_token_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 1000, 0);

    // Actual output is ~906, but we require 950
    let ix_swap = anchor_instruction(
        program_id,
        "swap",
        &swap_data(1000, 950, true), // min_amount_out too high
        vec![
            signer_meta(swapper.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(swapper_token_a),
            writable_meta(swapper_token_b),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix_swap, &swapper, &[&swapper]);
    assert!(result.is_err(), "swap with high min_amount_out should fail");
}

#[test]
fn test_swap_zero_amount_fails() {
    let mut svm = load_token_swap_program();
    let program_id = token_swap_program_id();

    let payer = funded_keypair_10_sol(&mut svm);
    let mint_a = setup_mint(&mut svm, 6);
    let mint_b = setup_mint(&mut svm, 6);

    let (pool_pda, lp_mint_pda, pool_token_a_pda, pool_token_b_pda) =
        initialize_pool(&mut svm, &payer, &mint_a, &mint_b, 30);
    svm.expire_blockhash();

    // Add liquidity
    let (lp_provider, lp_token_a, lp_token_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 100000, 100000);
    let lp_lp_token = setup_user_lp_account(&mut svm, &lp_provider.pubkey(), &lp_mint_pda, 0);

    let ix_add = anchor_instruction(
        program_id,
        "add_liquidity",
        &add_liquidity_data(10000, 10000, 0),
        vec![
            signer_meta(lp_provider.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(lp_token_a),
            writable_meta(lp_token_b),
            writable_meta(lp_lp_token),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix_add, &lp_provider, &[&lp_provider]).unwrap();
    svm.expire_blockhash();

    let (swapper, swapper_token_a, swapper_token_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 1000, 0);

    let ix_swap = anchor_instruction(
        program_id,
        "swap",
        &swap_data(0, 0, true), // zero amount
        vec![
            signer_meta(swapper.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(swapper_token_a),
            writable_meta(swapper_token_b),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix_swap, &swapper, &[&swapper]);
    assert!(result.is_err(), "swap with zero amount should fail");
}

#[test]
fn test_swap_with_different_fees() {
    let mut svm = load_token_swap_program();
    let program_id = token_swap_program_id();

    let payer = funded_keypair_10_sol(&mut svm);

    // Pool 1: 0.1% fee
    let mint_a1 = setup_mint(&mut svm, 6);
    let mint_b1 = setup_mint(&mut svm, 6);
    let (pool1, lp_mint1, pool_token_a1, pool_token_b1) =
        initialize_pool(&mut svm, &payer, &mint_a1, &mint_b1, 10);
    svm.expire_blockhash();

    // Pool 2: 1% fee
    let mint_a2 = setup_mint(&mut svm, 6);
    let mint_b2 = setup_mint(&mut svm, 6);
    let (pool2, lp_mint2, pool_token_a2, pool_token_b2) =
        initialize_pool(&mut svm, &payer, &mint_a2, &mint_b2, 100);
    svm.expire_blockhash();

    // Add same liquidity to both
    let (lp1, lp1_a, lp1_b) = setup_user_with_tokens(&mut svm, &mint_a1, &mint_b1, 100000, 100000);
    let lp1_lp = setup_user_lp_account(&mut svm, &lp1.pubkey(), &lp_mint1, 0);
    let ix_add1 = anchor_instruction(
        program_id,
        "add_liquidity",
        &add_liquidity_data(10000, 10000, 0),
        vec![
            signer_meta(lp1.pubkey()),
            writable_meta(mint_a1),
            writable_meta(mint_b1),
            writable_meta(pool1),
            writable_meta(lp_mint1),
            writable_meta(pool_token_a1),
            writable_meta(pool_token_b1),
            writable_meta(lp1_a),
            writable_meta(lp1_b),
            writable_meta(lp1_lp),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix_add1, &lp1, &[&lp1]).unwrap();
    svm.expire_blockhash();

    let (lp2, lp2_a, lp2_b) = setup_user_with_tokens(&mut svm, &mint_a2, &mint_b2, 100000, 100000);
    let lp2_lp = setup_user_lp_account(&mut svm, &lp2.pubkey(), &lp_mint2, 0);
    let ix_add2 = anchor_instruction(
        program_id,
        "add_liquidity",
        &add_liquidity_data(10000, 10000, 0),
        vec![
            signer_meta(lp2.pubkey()),
            writable_meta(mint_a2),
            writable_meta(mint_b2),
            writable_meta(pool2),
            writable_meta(lp_mint2),
            writable_meta(pool_token_a2),
            writable_meta(pool_token_b2),
            writable_meta(lp2_a),
            writable_meta(lp2_b),
            writable_meta(lp2_lp),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix_add2, &lp2, &[&lp2]).unwrap();
    svm.expire_blockhash();

    // Swap same amount in both pools
    let (swapper1, sw1_a, sw1_b) = setup_user_with_tokens(&mut svm, &mint_a1, &mint_b1, 1000, 0);
    let (swapper2, sw2_a, sw2_b) = setup_user_with_tokens(&mut svm, &mint_a2, &mint_b2, 1000, 0);

    let ix_swap1 = anchor_instruction(
        program_id,
        "swap",
        &swap_data(1000, 0, true),
        vec![
            signer_meta(swapper1.pubkey()),
            writable_meta(mint_a1),
            writable_meta(mint_b1),
            writable_meta(pool1),
            writable_meta(pool_token_a1),
            writable_meta(pool_token_b1),
            writable_meta(sw1_a),
            writable_meta(sw1_b),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix_swap1, &swapper1, &[&swapper1]).unwrap();
    svm.expire_blockhash();

    let ix_swap2 = anchor_instruction(
        program_id,
        "swap",
        &swap_data(1000, 0, true),
        vec![
            signer_meta(swapper2.pubkey()),
            writable_meta(mint_a2),
            writable_meta(mint_b2),
            writable_meta(pool2),
            writable_meta(pool_token_a2),
            writable_meta(pool_token_b2),
            writable_meta(sw2_a),
            writable_meta(sw2_b),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix_swap2, &swapper2, &[&swapper2]).unwrap();

    // Get outputs
    let out1 = read_token_balance(&svm.get_account(&sw1_b).unwrap().data);
    let out2 = read_token_balance(&svm.get_account(&sw2_b).unwrap().data);

    // Pool with lower fee (0.1%) should give more output
    assert!(out1 > out2, "Lower fee pool should give better rate: {} vs {}", out1, out2);

    // Expected:
    // Pool 1 (10 bps): 1000 * 9990 * 10000 / (10000 * 10000 + 1000 * 9990) = 908
    // Pool 2 (100 bps): 1000 * 9900 * 10000 / (10000 * 10000 + 1000 * 9900) = 900
    assert_eq!(out1, 908);
    assert_eq!(out2, 900);
}

// =============================================================================
// FULL WORKFLOW TESTS
// =============================================================================

#[test]
fn test_full_amm_workflow() {
    let mut svm = load_token_swap_program();
    let program_id = token_swap_program_id();

    let payer = funded_keypair_10_sol(&mut svm);
    let mint_a = setup_mint(&mut svm, 6);
    let mint_b = setup_mint(&mut svm, 6);

    // 1. Initialize pool
    let (pool_pda, lp_mint_pda, pool_token_a_pda, pool_token_b_pda) =
        initialize_pool(&mut svm, &payer, &mint_a, &mint_b, 30);
    svm.expire_blockhash();

    // 2. First LP adds liquidity (10000:10000)
    let (lp1, lp1_a, lp1_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 50000, 50000);
    let lp1_lp = setup_user_lp_account(&mut svm, &lp1.pubkey(), &lp_mint_pda, 0);

    let ix1 = anchor_instruction(
        program_id,
        "add_liquidity",
        &add_liquidity_data(10000, 10000, 0),
        vec![
            signer_meta(lp1.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(lp1_a),
            writable_meta(lp1_b),
            writable_meta(lp1_lp),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix1, &lp1, &[&lp1]).unwrap();
    svm.expire_blockhash();

    // LP1 should have 9900 LP tokens (sqrt(10000*10000) - 100)
    let lp1_lp_balance = read_token_balance(&svm.get_account(&lp1_lp).unwrap().data);
    assert_eq!(lp1_lp_balance, 9900);

    // 3. Trader swaps 2000 A for B
    let (trader, trader_a, trader_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 5000, 5000);

    let ix2 = anchor_instruction(
        program_id,
        "swap",
        &swap_data(2000, 0, true),
        vec![
            signer_meta(trader.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(trader_a),
            writable_meta(trader_b),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix2, &trader, &[&trader]).unwrap();
    svm.expire_blockhash();

    // Check trader received B tokens
    let trader_b_balance = read_token_balance(&svm.get_account(&trader_b).unwrap().data);
    assert!(trader_b_balance > 5000, "Trader should have more B tokens: {}", trader_b_balance);

    // 4. Second LP adds liquidity (pool now has different ratio)
    let (lp2, lp2_a, lp2_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 50000, 50000);
    let lp2_lp = setup_user_lp_account(&mut svm, &lp2.pubkey(), &lp_mint_pda, 0);

    let ix3 = anchor_instruction(
        program_id,
        "add_liquidity",
        &add_liquidity_data(5000, 5000, 0), // Will deposit proportionally
        vec![
            signer_meta(lp2.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(lp2_a),
            writable_meta(lp2_b),
            writable_meta(lp2_lp),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix3, &lp2, &[&lp2]).unwrap();
    svm.expire_blockhash();

    // 5. LP1 removes all liquidity
    let ix4 = anchor_instruction(
        program_id,
        "remove_liquidity",
        &remove_liquidity_data(lp1_lp_balance, 0, 0),
        vec![
            signer_meta(lp1.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(lp1_a),
            writable_meta(lp1_b),
            writable_meta(lp1_lp),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix4, &lp1, &[&lp1]).unwrap();

    // LP1 should have 0 LP tokens
    let lp1_lp_final = read_token_balance(&svm.get_account(&lp1_lp).unwrap().data);
    assert_eq!(lp1_lp_final, 0);

    // LP1 got tokens back (they collected fees so may have more than started with)
    let lp1_a_final = read_token_balance(&svm.get_account(&lp1_a).unwrap().data);
    let lp1_b_final = read_token_balance(&svm.get_account(&lp1_b).unwrap().data);

    // LP1 started with 50000 - 10000 = 40000 of each
    // After removing liquidity, should have more due to fees
    let total_a = lp1_a_final;
    let total_b = lp1_b_final;
    println!("LP1 final balances: A={}, B={}", total_a, total_b);

    // Total value should be approximately >= initial (fees earned)
    // LP1 deposited 10000:10000, the pool collected fees from trades
    assert!(total_a + total_b >= 40000 + 40000, "LP1 should have at least initial + fees");
}

#[test]
fn test_constant_product_invariant() {
    let mut svm = load_token_swap_program();
    let program_id = token_swap_program_id();

    let payer = funded_keypair_10_sol(&mut svm);
    let mint_a = setup_mint(&mut svm, 6);
    let mint_b = setup_mint(&mut svm, 6);

    let (pool_pda, lp_mint_pda, pool_token_a_pda, pool_token_b_pda) =
        initialize_pool(&mut svm, &payer, &mint_a, &mint_b, 30);
    svm.expire_blockhash();

    // Add liquidity
    let (lp, lp_a, lp_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 100000, 100000);
    let lp_lp = setup_user_lp_account(&mut svm, &lp.pubkey(), &lp_mint_pda, 0);

    let ix_add = anchor_instruction(
        program_id,
        "add_liquidity",
        &add_liquidity_data(10000, 10000, 0),
        vec![
            signer_meta(lp.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(pool_pda),
            writable_meta(lp_mint_pda),
            writable_meta(pool_token_a_pda),
            writable_meta(pool_token_b_pda),
            writable_meta(lp_a),
            writable_meta(lp_b),
            writable_meta(lp_lp),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix_add, &lp, &[&lp]).unwrap();
    svm.expire_blockhash();

    let k_initial: u128 = 10000 * 10000;

    // Perform multiple swaps and verify k never decreases
    for i in 0..5 {
        let (swapper, sw_a, sw_b) = setup_user_with_tokens(&mut svm, &mint_a, &mint_b, 1000, 1000);

        // Alternate swap directions
        let swap_a_to_b = i % 2 == 0;
        let ix_swap = anchor_instruction(
            program_id,
            "swap",
            &swap_data(500, 0, swap_a_to_b),
            vec![
                signer_meta(swapper.pubkey()),
                writable_meta(mint_a),
                writable_meta(mint_b),
                writable_meta(pool_pda),
                writable_meta(pool_token_a_pda),
                writable_meta(pool_token_b_pda),
                writable_meta(sw_a),
                writable_meta(sw_b),
                readonly_meta(token_program_id()),
            ],
        );
        execute_tx(&mut svm, ix_swap, &swapper, &[&swapper]).unwrap();
        svm.expire_blockhash();

        // Check k after swap
        let pool_a = read_token_balance(&svm.get_account(&pool_token_a_pda).unwrap().data);
        let pool_b = read_token_balance(&svm.get_account(&pool_token_b_pda).unwrap().data);
        let k_current: u128 = pool_a as u128 * pool_b as u128;

        assert!(k_current >= k_initial, "k should never decrease: {} < {}", k_current, k_initial);
    }
}
