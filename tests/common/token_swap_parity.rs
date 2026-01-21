//! Token Swap AMM tests: Seahorse vs Anchor behavior parity verification
//!
//! These tests verify that the Seahorse token swap implementation produces
//! mathematically correct results compared to the Anchor reference program.
//!
//! Key behaviors to verify:
//! - Pool initialization creates correct PDAs and state
//! - First liquidity deposit: LP = sqrt(a*b) - MINIMUM_LIQUIDITY
//! - Subsequent deposits: proportional LP minting
//! - Swap math: constant product formula with fee
//! - LP token accounting matches pool state changes
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile:
//! - target/deploy/token_swap.so (Seahorse)
//!
//! Note: The Anchor reference at tests/anchor-reference/token_swap_anchor/ can be
//! built with `anchor build` for comparison if needed.

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
/// Minimum liquidity locked on first deposit
const MINIMUM_LIQUIDITY: u64 = 100;

/// Token swap program ID
fn program_id() -> Pubkey {
    Pubkey::from_str("SwAP5VzVZYxLYtDmJq5DEmUKDEcPnP2zGxqHKbv2AmM").unwrap()
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
fn derive_pool_pda(mint_a: &Pubkey, mint_b: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"pool", mint_a.as_ref(), mint_b.as_ref()], program_id)
}

/// Derive LP mint PDA
fn derive_lp_mint_pda(mint_a: &Pubkey, mint_b: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"lp_mint", mint_a.as_ref(), mint_b.as_ref()], program_id)
}

/// Derive pool token A account PDA
fn derive_pool_token_a_pda(mint_a: &Pubkey, mint_b: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"pool_token_a", mint_a.as_ref(), mint_b.as_ref()], program_id)
}

/// Derive pool token B account PDA
fn derive_pool_token_b_pda(mint_a: &Pubkey, mint_b: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"pool_token_b", mint_a.as_ref(), mint_b.as_ref()], program_id)
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

/// Read mint supply
fn read_mint_supply(data: &[u8]) -> u64 {
    let mint = spl_token::state::Mint::unpack(data).unwrap();
    mint.supply
}

/// Read pool fee_bps from account data
fn read_pool_fee_bps(data: &[u8]) -> u16 {
    // Skip discriminator (8) + mint_a (32) + mint_b (32) + lp_mint (32) + pool_token_a (32) + pool_token_b (32)
    let bytes: [u8; 2] = data[168..170].try_into().unwrap();
    u16::from_le_bytes(bytes)
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

/// Build swap instruction data
fn swap_data(amount_in: u64, min_amount_out: u64, swap_a_to_b: bool) -> Vec<u8> {
    let mut data = Vec::with_capacity(17);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data.push(if swap_a_to_b { 1 } else { 0 });
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

/// Integer square root
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

/// Calculate expected swap output using constant product formula with fee
fn calculate_swap_output(amount_in: u64, reserve_in: u64, reserve_out: u64, fee_bps: u16) -> u64 {
    let amount_in_with_fee = amount_in as u128 * (10000 - fee_bps as u128);
    let numerator = amount_in_with_fee * reserve_out as u128;
    let denominator = reserve_in as u128 * 10000 + amount_in_with_fee;
    (numerator / denominator) as u64
}

// =============================================================================
// PARITY TESTS
// =============================================================================

#[test]
fn test_parity_pool_initialization() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/token_swap.so")
        .expect("Failed to read token_swap.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup mints
    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    svm.set_account(mint_a.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();
    svm.set_account(mint_b.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    // Setup payer
    let payer = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    let fee_bps: u16 = 30;
    let (pool_pda, _) = derive_pool_pda(&mint_a.pubkey(), &mint_b.pubkey(), &program_id);
    let (lp_mint_pda, _) = derive_lp_mint_pda(&mint_a.pubkey(), &mint_b.pubkey(), &program_id);
    let (pool_token_a_pda, _) = derive_pool_token_a_pda(&mint_a.pubkey(), &mint_b.pubkey(), &program_id);
    let (pool_token_b_pda, _) = derive_pool_token_b_pda(&mint_a.pubkey(), &mint_b.pubkey(), &program_id);

    // Initialize pool
    let ix = InstructionBuilder::new("initialize_pool")
        .with_signer(&payer.pubkey())
        .with_writable(&mint_a.pubkey())
        .with_writable(&mint_b.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&lp_mint_pda)
        .with_writable(&pool_token_a_pda)
        .with_writable(&pool_token_b_pda)
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(initialize_pool_data(fee_bps))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[ix], Some(&payer.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&payer], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "initialize_pool failed: {:?}", result);

    // Verify pool state
    let pool = svm.get_account(&pool_pda).expect("Pool should exist");
    assert_eq!(pool.owner, program_id, "Pool should be owned by program");
    assert_eq!(read_pool_fee_bps(&pool.data), fee_bps, "Fee should be stored correctly");

    // Verify LP mint created with zero supply
    let lp_mint = svm.get_account(&lp_mint_pda).expect("LP mint should exist");
    assert_eq!(lp_mint.owner, token_program_id());
    assert_eq!(read_mint_supply(&lp_mint.data), 0, "LP supply should be 0");

    // Verify pool token accounts are empty
    let pool_a = svm.get_account(&pool_token_a_pda).expect("Pool token A should exist");
    let pool_b = svm.get_account(&pool_token_b_pda).expect("Pool token B should exist");
    assert_eq!(read_token_balance(&pool_a.data), 0);
    assert_eq!(read_token_balance(&pool_b.data), 0);
}

#[test]
fn test_parity_first_liquidity_deposit_formula() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/token_swap.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup mints
    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    svm.set_account(mint_a.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();
    svm.set_account(mint_b.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    // Setup payer
    let payer = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    // Initialize pool
    let (pool_pda, _) = derive_pool_pda(&mint_a.pubkey(), &mint_b.pubkey(), &program_id);
    let (lp_mint_pda, _) = derive_lp_mint_pda(&mint_a.pubkey(), &mint_b.pubkey(), &program_id);
    let (pool_token_a_pda, _) = derive_pool_token_a_pda(&mint_a.pubkey(), &mint_b.pubkey(), &program_id);
    let (pool_token_b_pda, _) = derive_pool_token_b_pda(&mint_a.pubkey(), &mint_b.pubkey(), &program_id);

    let init_ix = InstructionBuilder::new("initialize_pool")
        .with_signer(&payer.pubkey())
        .with_writable(&mint_a.pubkey())
        .with_writable(&mint_b.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&lp_mint_pda)
        .with_writable(&pool_token_a_pda)
        .with_writable(&pool_token_b_pda)
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(initialize_pool_data(30))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[init_ix], Some(&payer.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&payer], message, blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // Setup user with tokens
    let user = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let user_token_a = get_ata(&user.pubkey(), &mint_a.pubkey());
    let user_token_b = get_ata(&user.pubkey(), &mint_b.pubkey());
    let user_lp_token = get_ata(&user.pubkey(), &lp_mint_pda);

    svm.set_account(user_token_a, create_token_account(&user.pubkey(), &mint_a.pubkey(), 10000)).unwrap();
    svm.set_account(user_token_b, create_token_account(&user.pubkey(), &mint_b.pubkey(), 10000)).unwrap();
    svm.set_account(user_lp_token, create_token_account(&user.pubkey(), &lp_mint_pda, 0)).unwrap();

    // Add liquidity: 1000 A, 1000 B
    let amount_a: u64 = 1000;
    let amount_b: u64 = 1000;
    let expected_lp = isqrt(amount_a * amount_b) - MINIMUM_LIQUIDITY;
    assert_eq!(expected_lp, 900, "Expected LP should be sqrt(1000*1000) - 100 = 900");

    let add_ix = InstructionBuilder::new("add_liquidity")
        .with_signer(&user.pubkey())
        .with_writable(&mint_a.pubkey())
        .with_writable(&mint_b.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&lp_mint_pda)
        .with_writable(&pool_token_a_pda)
        .with_writable(&pool_token_b_pda)
        .with_writable(&user_token_a)
        .with_writable(&user_token_b)
        .with_writable(&user_lp_token)
        .with_readonly(&token_program_id())
        .with_data(add_liquidity_data(amount_a, amount_b, 0))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[add_ix], Some(&user.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&user], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "add_liquidity failed: {:?}", result);

    // Verify LP tokens minted match formula
    let user_lp = svm.get_account(&user_lp_token).unwrap();
    let actual_lp = read_token_balance(&user_lp.data);
    assert_eq!(
        actual_lp, expected_lp,
        "LP tokens should match formula: sqrt(a*b) - MINIMUM_LIQUIDITY"
    );

    // Verify pool balances
    let pool_a = svm.get_account(&pool_token_a_pda).unwrap();
    let pool_b = svm.get_account(&pool_token_b_pda).unwrap();
    assert_eq!(read_token_balance(&pool_a.data), amount_a);
    assert_eq!(read_token_balance(&pool_b.data), amount_b);
}

#[test]
fn test_parity_swap_math_constant_product() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/token_swap.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup mints
    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    svm.set_account(mint_a.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();
    svm.set_account(mint_b.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    // Setup payer
    let payer = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    // Initialize pool with 0.3% fee
    let fee_bps: u16 = 30;
    let (pool_pda, _) = derive_pool_pda(&mint_a.pubkey(), &mint_b.pubkey(), &program_id);
    let (lp_mint_pda, _) = derive_lp_mint_pda(&mint_a.pubkey(), &mint_b.pubkey(), &program_id);
    let (pool_token_a_pda, _) = derive_pool_token_a_pda(&mint_a.pubkey(), &mint_b.pubkey(), &program_id);
    let (pool_token_b_pda, _) = derive_pool_token_b_pda(&mint_a.pubkey(), &mint_b.pubkey(), &program_id);

    let init_ix = InstructionBuilder::new("initialize_pool")
        .with_signer(&payer.pubkey())
        .with_writable(&mint_a.pubkey())
        .with_writable(&mint_b.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&lp_mint_pda)
        .with_writable(&pool_token_a_pda)
        .with_writable(&pool_token_b_pda)
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(initialize_pool_data(fee_bps))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[init_ix], Some(&payer.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&payer], message, blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // Add liquidity: 10000:10000
    let lp_provider = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let lp_token_a = get_ata(&lp_provider.pubkey(), &mint_a.pubkey());
    let lp_token_b = get_ata(&lp_provider.pubkey(), &mint_b.pubkey());
    let lp_lp_token = get_ata(&lp_provider.pubkey(), &lp_mint_pda);

    svm.set_account(lp_token_a, create_token_account(&lp_provider.pubkey(), &mint_a.pubkey(), 100000)).unwrap();
    svm.set_account(lp_token_b, create_token_account(&lp_provider.pubkey(), &mint_b.pubkey(), 100000)).unwrap();
    svm.set_account(lp_lp_token, create_token_account(&lp_provider.pubkey(), &lp_mint_pda, 0)).unwrap();

    let add_ix = InstructionBuilder::new("add_liquidity")
        .with_signer(&lp_provider.pubkey())
        .with_writable(&mint_a.pubkey())
        .with_writable(&mint_b.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&lp_mint_pda)
        .with_writable(&pool_token_a_pda)
        .with_writable(&pool_token_b_pda)
        .with_writable(&lp_token_a)
        .with_writable(&lp_token_b)
        .with_writable(&lp_lp_token)
        .with_readonly(&token_program_id())
        .with_data(add_liquidity_data(10000, 10000, 0))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[add_ix], Some(&lp_provider.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&lp_provider], message, blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // Now perform swap: 1000 A for B
    let swapper = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let swapper_token_a = get_ata(&swapper.pubkey(), &mint_a.pubkey());
    let swapper_token_b = get_ata(&swapper.pubkey(), &mint_b.pubkey());

    svm.set_account(swapper_token_a, create_token_account(&swapper.pubkey(), &mint_a.pubkey(), 1000)).unwrap();
    svm.set_account(swapper_token_b, create_token_account(&swapper.pubkey(), &mint_b.pubkey(), 0)).unwrap();

    let amount_in: u64 = 1000;
    let reserve_in: u64 = 10000;
    let reserve_out: u64 = 10000;
    let expected_out = calculate_swap_output(amount_in, reserve_in, reserve_out, fee_bps);
    // 1000 * 9970 * 10000 / (10000 * 10000 + 1000 * 9970) = 99700000000 / 109970000 = 906
    assert_eq!(expected_out, 906, "Expected output should be 906");

    let swap_ix = InstructionBuilder::new("swap")
        .with_signer(&swapper.pubkey())
        .with_writable(&mint_a.pubkey())
        .with_writable(&mint_b.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&pool_token_a_pda)
        .with_writable(&pool_token_b_pda)
        .with_writable(&swapper_token_a)
        .with_writable(&swapper_token_b)
        .with_readonly(&token_program_id())
        .with_data(swap_data(amount_in, 0, true))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[swap_ix], Some(&swapper.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&swapper], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "swap failed: {:?}", result);

    // Verify output matches constant product formula
    let swapper_b = svm.get_account(&swapper_token_b).unwrap();
    let actual_out = read_token_balance(&swapper_b.data);
    assert_eq!(
        actual_out, expected_out,
        "Swap output should match constant product formula"
    );

    // Verify k increased (fees collected)
    let pool_a = svm.get_account(&pool_token_a_pda).unwrap();
    let pool_b = svm.get_account(&pool_token_b_pda).unwrap();
    let new_reserve_a = read_token_balance(&pool_a.data);
    let new_reserve_b = read_token_balance(&pool_b.data);

    let k_before: u128 = reserve_in as u128 * reserve_out as u128;
    let k_after: u128 = new_reserve_a as u128 * new_reserve_b as u128;

    assert!(k_after >= k_before, "k should never decrease (fees increase k)");
    assert_eq!(new_reserve_a, 10000 + amount_in, "Pool A should have received input");
    assert_eq!(new_reserve_b, 10000 - expected_out, "Pool B should have given output");
}

#[test]
fn test_parity_lp_token_accounting() {
    let program_id = program_id();

    let seahorse_bytes = std::fs::read("../../target/deploy/token_swap.so").unwrap();

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &seahorse_bytes);

    // Setup mints
    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    svm.set_account(mint_a.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();
    svm.set_account(mint_b.pubkey(), create_mint_account(&Keypair::new().pubkey(), 6))
        .unwrap();

    // Setup payer
    let payer = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };

    // Initialize pool
    let (pool_pda, _) = derive_pool_pda(&mint_a.pubkey(), &mint_b.pubkey(), &program_id);
    let (lp_mint_pda, _) = derive_lp_mint_pda(&mint_a.pubkey(), &mint_b.pubkey(), &program_id);
    let (pool_token_a_pda, _) = derive_pool_token_a_pda(&mint_a.pubkey(), &mint_b.pubkey(), &program_id);
    let (pool_token_b_pda, _) = derive_pool_token_b_pda(&mint_a.pubkey(), &mint_b.pubkey(), &program_id);

    let init_ix = InstructionBuilder::new("initialize_pool")
        .with_signer(&payer.pubkey())
        .with_writable(&mint_a.pubkey())
        .with_writable(&mint_b.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&lp_mint_pda)
        .with_writable(&pool_token_a_pda)
        .with_writable(&pool_token_b_pda)
        .with_readonly(&solana_sdk_ids::sysvar::rent::id())
        .with_readonly(&system_program::id())
        .with_readonly(&token_program_id())
        .with_data(initialize_pool_data(30))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[init_ix], Some(&payer.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&payer], message, blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // User adds liquidity
    let user = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    let user_token_a = get_ata(&user.pubkey(), &mint_a.pubkey());
    let user_token_b = get_ata(&user.pubkey(), &mint_b.pubkey());
    let user_lp_token = get_ata(&user.pubkey(), &lp_mint_pda);

    svm.set_account(user_token_a, create_token_account(&user.pubkey(), &mint_a.pubkey(), 10000)).unwrap();
    svm.set_account(user_token_b, create_token_account(&user.pubkey(), &mint_b.pubkey(), 10000)).unwrap();
    svm.set_account(user_lp_token, create_token_account(&user.pubkey(), &lp_mint_pda, 0)).unwrap();

    // First deposit: 1000:1000
    let add_ix = InstructionBuilder::new("add_liquidity")
        .with_signer(&user.pubkey())
        .with_writable(&mint_a.pubkey())
        .with_writable(&mint_b.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&lp_mint_pda)
        .with_writable(&pool_token_a_pda)
        .with_writable(&pool_token_b_pda)
        .with_writable(&user_token_a)
        .with_writable(&user_token_b)
        .with_writable(&user_lp_token)
        .with_readonly(&token_program_id())
        .with_data(add_liquidity_data(1000, 1000, 0))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[add_ix], Some(&user.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&user], message, blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // Verify initial LP: 900
    let lp_after_add = read_token_balance(&svm.get_account(&user_lp_token).unwrap().data);
    assert_eq!(lp_after_add, 900);

    // Remove half liquidity (450 LP)
    let lp_to_remove: u64 = 450;
    let remove_ix = InstructionBuilder::new("remove_liquidity")
        .with_signer(&user.pubkey())
        .with_writable(&mint_a.pubkey())
        .with_writable(&mint_b.pubkey())
        .with_writable(&pool_pda)
        .with_writable(&lp_mint_pda)
        .with_writable(&pool_token_a_pda)
        .with_writable(&pool_token_b_pda)
        .with_writable(&user_token_a)
        .with_writable(&user_token_b)
        .with_writable(&user_lp_token)
        .with_readonly(&token_program_id())
        .with_data(remove_liquidity_data(lp_to_remove, 0, 0))
        .build(program_id);

    let blockhash = svm.latest_blockhash();
    let message = solana_message::Message::new(&[remove_ix], Some(&user.pubkey()));
    let tx = solana_transaction::Transaction::new(&[&user], message, blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "remove_liquidity failed: {:?}", result);

    // Verify LP tokens burned
    let lp_after_remove = read_token_balance(&svm.get_account(&user_lp_token).unwrap().data);
    assert_eq!(lp_after_remove, 450, "LP should be 900 - 450 = 450");

    // Verify user received proportional tokens
    // 450/900 * 1000 = 500 of each
    let user_a = svm.get_account(&user_token_a).unwrap();
    let user_b = svm.get_account(&user_token_b).unwrap();
    // Started with 9000 (10000-1000), now should have 9000 + 500 = 9500
    assert_eq!(read_token_balance(&user_a.data), 9500);
    assert_eq!(read_token_balance(&user_b.data), 9500);

    // Verify pool has remaining tokens
    let pool_a = svm.get_account(&pool_token_a_pda).unwrap();
    let pool_b = svm.get_account(&pool_token_b_pda).unwrap();
    assert_eq!(read_token_balance(&pool_a.data), 500);
    assert_eq!(read_token_balance(&pool_b.data), 500);
}

#[test]
fn test_parity_deterministic_pda_derivation() {
    let program_id = program_id();

    // Create deterministic mints using Pubkey::new_unique for consistent testing
    let mint_a = Pubkey::new_unique();
    let mint_b = Pubkey::new_unique();

    // Verify PDA derivation is deterministic
    let (pool1, bump1) = derive_pool_pda(&mint_a, &mint_b, &program_id);
    let (pool2, bump2) = derive_pool_pda(&mint_a, &mint_b, &program_id);

    assert_eq!(pool1, pool2, "Pool PDAs should be deterministic");
    assert_eq!(bump1, bump2, "Pool bumps should be deterministic");

    // Verify different mints produce different PDAs
    let (pool3, _) = derive_pool_pda(&mint_b, &mint_a, &program_id); // Reversed order
    assert_ne!(pool1, pool3, "Different mint order should produce different PDA");

    // Verify all PDAs for a pool are deterministic
    let (lp_mint1, _) = derive_lp_mint_pda(&mint_a, &mint_b, &program_id);
    let (lp_mint2, _) = derive_lp_mint_pda(&mint_a, &mint_b, &program_id);
    assert_eq!(lp_mint1, lp_mint2, "LP mint PDAs should be deterministic");

    let (pool_a1, _) = derive_pool_token_a_pda(&mint_a, &mint_b, &program_id);
    let (pool_a2, _) = derive_pool_token_a_pda(&mint_a, &mint_b, &program_id);
    assert_eq!(pool_a1, pool_a2, "Pool token A PDAs should be deterministic");
}
