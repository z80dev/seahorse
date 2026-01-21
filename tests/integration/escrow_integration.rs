//! LiteSVM integration tests for the Seahorse Escrow program
//!
//! These tests verify trustless token swap flows:
//! - make_escrow: Maker deposits Token A, specifies wanted Token B amount
//! - take_escrow: Taker sends Token B to maker, receives Token A from vault
//! - cancel_escrow: Maker recovers Token A before taker accepts
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile escrow.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// Escrow program ID (from declare_id!)
fn escrow_program_id() -> Pubkey {
    Pubkey::from_str("7bNQSAJXfZjwTP86A3Z53WP8VSEHT4qFY4LMcepq6MPm").unwrap()
}

/// Escrow account size: discriminator (8) + escrow_id (8) + maker (32) + mint_a (32)
///                      + mint_b (32) + token_a_amount (8) + token_b_wanted_amount (8) + bump (1)
const ESCROW_SIZE: usize = 8 + 8 + 32 + 32 + 32 + 8 + 8 + 1;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the escrow program into LiteSVM
fn load_escrow_program() -> litesvm::LiteSVM {
    let program_id = escrow_program_id();
    let program_bytes = std::fs::read("../../target/deploy/escrow.so")
        .expect("Failed to read escrow.so - run ./scripts/build-test-programs.sh first");

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

/// Setup a user with an ATA containing tokens
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

/// Derive escrow PDA
fn derive_escrow_pda(escrow_id: u64) -> (Pubkey, u8) {
    find_pda(
        &[b"escrow", &escrow_id.to_le_bytes()],
        &escrow_program_id(),
    )
}

/// Derive vault PDA
fn derive_vault_pda(escrow_id: u64) -> (Pubkey, u8) {
    find_pda(
        &[b"vault", &escrow_id.to_le_bytes()],
        &escrow_program_id(),
    )
}

/// Read escrow_id from escrow account data
fn read_escrow_id(data: &[u8]) -> u64 {
    // Skip discriminator (8), read escrow_id (8)
    let bytes: [u8; 8] = data[8..16].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

/// Read maker from escrow account data
fn read_escrow_maker(data: &[u8]) -> Pubkey {
    // Skip discriminator (8) + escrow_id (8), read maker (32)
    Pubkey::new_from_array(data[16..48].try_into().unwrap())
}

/// Read mint_a from escrow account data
fn read_escrow_mint_a(data: &[u8]) -> Pubkey {
    // Skip discriminator (8) + escrow_id (8) + maker (32), read mint_a (32)
    Pubkey::new_from_array(data[48..80].try_into().unwrap())
}

/// Read mint_b from escrow account data
fn read_escrow_mint_b(data: &[u8]) -> Pubkey {
    // Skip disc (8) + escrow_id (8) + maker (32) + mint_a (32), read mint_b (32)
    Pubkey::new_from_array(data[80..112].try_into().unwrap())
}

/// Read token_a_amount from escrow account data
fn read_escrow_token_a_amount(data: &[u8]) -> u64 {
    // Skip disc (8) + escrow_id (8) + maker (32) + mint_a (32) + mint_b (32)
    let bytes: [u8; 8] = data[112..120].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

/// Read token_b_wanted_amount from escrow account data
fn read_escrow_token_b_wanted(data: &[u8]) -> u64 {
    // Skip disc (8) + escrow_id (8) + maker (32) + mint_a (32) + mint_b (32) + token_a_amount (8)
    let bytes: [u8; 8] = data[120..128].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

/// Read bump from escrow account data
fn read_escrow_bump(data: &[u8]) -> u8 {
    // Skip disc (8) + escrow_id (8) + maker (32) + mint_a (32) + mint_b (32) + amounts (16)
    data[128]
}

/// Build make_escrow instruction data
fn make_escrow_data(escrow_id: u64, token_a_amount: u64, token_b_wanted_amount: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&escrow_id.to_le_bytes());
    data.extend_from_slice(&token_a_amount.to_le_bytes());
    data.extend_from_slice(&token_b_wanted_amount.to_le_bytes());
    data
}

// =============================================================================
// MAKE ESCROW TESTS
// =============================================================================

#[test]
fn test_make_escrow() {
    let mut svm = load_escrow_program();
    let program_id = escrow_program_id();

    // Setup mints
    let (_, mint_a) = setup_mint(&mut svm, 6);
    let (_, mint_b) = setup_mint(&mut svm, 6);

    // Setup maker with Token A
    let (maker, maker_token_a) = setup_user_with_tokens(&mut svm, &mint_a, 1000);

    let escrow_id: u64 = 1;
    let token_a_amount: u64 = 500;
    let token_b_wanted_amount: u64 = 300;

    let (escrow_pda, _) = derive_escrow_pda(escrow_id);
    let (vault_pda, _) = derive_vault_pda(escrow_id);

    // Build make_escrow instruction (Seahorse account order)
    let ix = anchor_instruction(
        program_id,
        "make_escrow",
        &make_escrow_data(escrow_id, token_a_amount, token_b_wanted_amount),
        vec![
            signer_meta(maker.pubkey()),        // maker
            writable_meta(mint_a),              // mint_a
            writable_meta(mint_b),              // mint_b
            writable_meta(maker_token_a),       // maker_token_account_a
            writable_meta(escrow_pda),          // escrow
            writable_meta(vault_pda),           // vault
            readonly_meta(solana_sdk_ids::sysvar::rent::id()), // rent
            readonly_meta(system_program::id()), // system_program
            readonly_meta(token_program_id()),   // token_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &maker, &[&maker]);
    assert!(result.is_ok(), "make_escrow should succeed: {:?}", result);

    // Verify escrow state
    let escrow_account = svm.get_account(&escrow_pda).expect("Escrow should exist");
    assert_eq!(escrow_account.owner, program_id, "Escrow should be owned by program");

    assert_eq!(read_escrow_id(&escrow_account.data), escrow_id);
    assert_eq!(read_escrow_maker(&escrow_account.data), maker.pubkey());
    assert_eq!(read_escrow_mint_a(&escrow_account.data), mint_a);
    assert_eq!(read_escrow_mint_b(&escrow_account.data), mint_b);
    assert_eq!(read_escrow_token_a_amount(&escrow_account.data), token_a_amount);
    assert_eq!(read_escrow_token_b_wanted(&escrow_account.data), token_b_wanted_amount);

    // Verify vault received tokens
    let vault_account = svm.get_account(&vault_pda).expect("Vault should exist");
    assert_eq!(vault_account.owner, token_program_id(), "Vault should be owned by token program");
    assert_eq!(read_token_balance(&vault_account.data), token_a_amount, "Vault should have token_a_amount");

    // Verify maker's token account decreased
    let maker_account = svm.get_account(&maker_token_a).unwrap();
    assert_eq!(read_token_balance(&maker_account.data), 500, "Maker should have 500 tokens left");
}

#[test]
fn test_make_escrow_multiple() {
    let mut svm = load_escrow_program();
    let program_id = escrow_program_id();

    let (_, mint_a) = setup_mint(&mut svm, 6);
    let (_, mint_b) = setup_mint(&mut svm, 6);

    let (maker1, maker1_token_a) = setup_user_with_tokens(&mut svm, &mint_a, 1000);
    let (maker2, maker2_token_a) = setup_user_with_tokens(&mut svm, &mint_a, 1000);

    // Escrow 1
    let escrow_id_1: u64 = 1;
    let (escrow_pda_1, _) = derive_escrow_pda(escrow_id_1);
    let (vault_pda_1, _) = derive_vault_pda(escrow_id_1);

    let ix1 = anchor_instruction(
        program_id,
        "make_escrow",
        &make_escrow_data(escrow_id_1, 200, 100),
        vec![
            signer_meta(maker1.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(maker1_token_a),
            writable_meta(escrow_pda_1),
            writable_meta(vault_pda_1),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    let result1 = execute_tx(&mut svm, ix1, &maker1, &[&maker1]);
    assert!(result1.is_ok(), "Escrow 1 should succeed");

    // Escrow 2
    svm.expire_blockhash();
    let escrow_id_2: u64 = 2;
    let (escrow_pda_2, _) = derive_escrow_pda(escrow_id_2);
    let (vault_pda_2, _) = derive_vault_pda(escrow_id_2);

    let ix2 = anchor_instruction(
        program_id,
        "make_escrow",
        &make_escrow_data(escrow_id_2, 300, 150),
        vec![
            signer_meta(maker2.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(maker2_token_a),
            writable_meta(escrow_pda_2),
            writable_meta(vault_pda_2),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    let result2 = execute_tx(&mut svm, ix2, &maker2, &[&maker2]);
    assert!(result2.is_ok(), "Escrow 2 should succeed");

    // Verify both escrows exist independently
    let escrow1 = svm.get_account(&escrow_pda_1).unwrap();
    let escrow2 = svm.get_account(&escrow_pda_2).unwrap();

    assert_eq!(read_escrow_id(&escrow1.data), escrow_id_1);
    assert_eq!(read_escrow_id(&escrow2.data), escrow_id_2);
    assert_eq!(read_escrow_maker(&escrow1.data), maker1.pubkey());
    assert_eq!(read_escrow_maker(&escrow2.data), maker2.pubkey());
}

#[test]
fn test_make_escrow_zero_amount_fails() {
    let mut svm = load_escrow_program();
    let program_id = escrow_program_id();

    let (_, mint_a) = setup_mint(&mut svm, 6);
    let (_, mint_b) = setup_mint(&mut svm, 6);
    let (maker, maker_token_a) = setup_user_with_tokens(&mut svm, &mint_a, 1000);

    let escrow_id: u64 = 1;
    let (escrow_pda, _) = derive_escrow_pda(escrow_id);
    let (vault_pda, _) = derive_vault_pda(escrow_id);

    // Zero token_a_amount should fail
    let ix = anchor_instruction(
        program_id,
        "make_escrow",
        &make_escrow_data(escrow_id, 0, 100),
        vec![
            signer_meta(maker.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(maker_token_a),
            writable_meta(escrow_pda),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &maker, &[&maker]);
    assert!(result.is_err(), "make_escrow with zero amount should fail");
}

#[test]
fn test_make_escrow_zero_wanted_amount_fails() {
    let mut svm = load_escrow_program();
    let program_id = escrow_program_id();

    let (_, mint_a) = setup_mint(&mut svm, 6);
    let (_, mint_b) = setup_mint(&mut svm, 6);
    let (maker, maker_token_a) = setup_user_with_tokens(&mut svm, &mint_a, 1000);

    let escrow_id: u64 = 1;
    let (escrow_pda, _) = derive_escrow_pda(escrow_id);
    let (vault_pda, _) = derive_vault_pda(escrow_id);

    // Zero token_b_wanted_amount should fail
    let ix = anchor_instruction(
        program_id,
        "make_escrow",
        &make_escrow_data(escrow_id, 100, 0),
        vec![
            signer_meta(maker.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(maker_token_a),
            writable_meta(escrow_pda),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &maker, &[&maker]);
    assert!(result.is_err(), "make_escrow with zero wanted amount should fail");
}

// =============================================================================
// TAKE ESCROW TESTS
// =============================================================================

#[test]
fn test_take_escrow() {
    let mut svm = load_escrow_program();
    let program_id = escrow_program_id();

    // Setup mints
    let (_, mint_a) = setup_mint(&mut svm, 6);
    let (_, mint_b) = setup_mint(&mut svm, 6);

    // Setup maker with Token A
    let (maker, maker_token_a) = setup_user_with_tokens(&mut svm, &mint_a, 1000);
    // Setup maker's Token B account (empty, will receive from taker)
    let maker_token_b = get_ata(&maker.pubkey(), &mint_b);
    svm.set_account(maker_token_b, create_token_account(&maker.pubkey(), &mint_b, 0)).unwrap();

    // Setup taker with Token B
    let (taker, taker_token_b) = setup_user_with_tokens(&mut svm, &mint_b, 1000);
    // Setup taker's Token A account (empty, will receive from vault)
    let taker_token_a = get_ata(&taker.pubkey(), &mint_a);
    svm.set_account(taker_token_a, create_token_account(&taker.pubkey(), &mint_a, 0)).unwrap();

    // Create escrow
    let escrow_id: u64 = 1;
    let token_a_amount: u64 = 500;
    let token_b_wanted: u64 = 300;
    let (escrow_pda, _) = derive_escrow_pda(escrow_id);
    let (vault_pda, _) = derive_vault_pda(escrow_id);

    let make_ix = anchor_instruction(
        program_id,
        "make_escrow",
        &make_escrow_data(escrow_id, token_a_amount, token_b_wanted),
        vec![
            signer_meta(maker.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(maker_token_a),
            writable_meta(escrow_pda),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, make_ix, &maker, &[&maker]).unwrap();

    // Take escrow
    svm.expire_blockhash();
    let take_ix = anchor_instruction(
        program_id,
        "take_escrow",
        &[], // No instruction data for take_escrow
        vec![
            signer_meta(taker.pubkey()),        // taker
            writable_meta(maker.pubkey()),      // maker (UncheckedAccount)
            writable_meta(mint_a),              // mint_a
            writable_meta(mint_b),              // mint_b
            writable_meta(taker_token_a),       // taker_token_account_a
            writable_meta(taker_token_b),       // taker_token_account_b
            writable_meta(maker_token_b),       // maker_token_account_b
            writable_meta(escrow_pda),          // escrow
            writable_meta(vault_pda),           // vault
            readonly_meta(token_program_id()),   // token_program
        ],
    );

    let result = execute_tx(&mut svm, take_ix, &taker, &[&taker]);
    assert!(result.is_ok(), "take_escrow should succeed: {:?}", result);

    // Verify taker received Token A from vault
    let taker_a_account = svm.get_account(&taker_token_a).unwrap();
    assert_eq!(read_token_balance(&taker_a_account.data), token_a_amount, "Taker should receive token_a_amount");

    // Verify taker's Token B decreased
    let taker_b_account = svm.get_account(&taker_token_b).unwrap();
    assert_eq!(read_token_balance(&taker_b_account.data), 1000 - token_b_wanted, "Taker should have sent token_b_wanted");

    // Verify maker received Token B
    let maker_b_account = svm.get_account(&maker_token_b).unwrap();
    assert_eq!(read_token_balance(&maker_b_account.data), token_b_wanted, "Maker should receive token_b_wanted");

    // Verify vault is empty (or closed depending on implementation)
    let vault_account = svm.get_account(&vault_pda);
    if let Some(vault) = vault_account {
        assert_eq!(read_token_balance(&vault.data), 0, "Vault should be empty after take");
    }
}

#[test]
fn test_take_escrow_wrong_maker_fails() {
    let mut svm = load_escrow_program();
    let program_id = escrow_program_id();

    let (_, mint_a) = setup_mint(&mut svm, 6);
    let (_, mint_b) = setup_mint(&mut svm, 6);

    // Real maker
    let (maker, maker_token_a) = setup_user_with_tokens(&mut svm, &mint_a, 1000);
    let maker_token_b = get_ata(&maker.pubkey(), &mint_b);
    svm.set_account(maker_token_b, create_token_account(&maker.pubkey(), &mint_b, 0)).unwrap();

    // Fake maker (attacker)
    let fake_maker = funded_keypair_10_sol(&mut svm);
    let fake_maker_token_b = get_ata(&fake_maker.pubkey(), &mint_b);
    svm.set_account(fake_maker_token_b, create_token_account(&fake_maker.pubkey(), &mint_b, 0)).unwrap();

    // Taker
    let (taker, taker_token_b) = setup_user_with_tokens(&mut svm, &mint_b, 1000);
    let taker_token_a = get_ata(&taker.pubkey(), &mint_a);
    svm.set_account(taker_token_a, create_token_account(&taker.pubkey(), &mint_a, 0)).unwrap();

    // Create escrow with real maker
    let escrow_id: u64 = 1;
    let (escrow_pda, _) = derive_escrow_pda(escrow_id);
    let (vault_pda, _) = derive_vault_pda(escrow_id);

    let make_ix = anchor_instruction(
        program_id,
        "make_escrow",
        &make_escrow_data(escrow_id, 500, 300),
        vec![
            signer_meta(maker.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(maker_token_a),
            writable_meta(escrow_pda),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, make_ix, &maker, &[&maker]).unwrap();

    // Try to take escrow with wrong maker (attacker tries to get tokens sent to fake_maker)
    svm.expire_blockhash();
    let take_ix = anchor_instruction(
        program_id,
        "take_escrow",
        &[],
        vec![
            signer_meta(taker.pubkey()),
            writable_meta(fake_maker.pubkey()), // Wrong maker!
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(taker_token_a),
            writable_meta(taker_token_b),
            writable_meta(fake_maker_token_b),  // Attacker's token account
            writable_meta(escrow_pda),
            writable_meta(vault_pda),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, take_ix, &taker, &[&taker]);
    assert!(result.is_err(), "take_escrow with wrong maker should fail");
}

#[test]
fn test_take_escrow_insufficient_funds_fails() {
    let mut svm = load_escrow_program();
    let program_id = escrow_program_id();

    let (_, mint_a) = setup_mint(&mut svm, 6);
    let (_, mint_b) = setup_mint(&mut svm, 6);

    let (maker, maker_token_a) = setup_user_with_tokens(&mut svm, &mint_a, 1000);
    let maker_token_b = get_ata(&maker.pubkey(), &mint_b);
    svm.set_account(maker_token_b, create_token_account(&maker.pubkey(), &mint_b, 0)).unwrap();

    // Taker has only 100 Token B but escrow wants 300
    let (taker, taker_token_b) = setup_user_with_tokens(&mut svm, &mint_b, 100);
    let taker_token_a = get_ata(&taker.pubkey(), &mint_a);
    svm.set_account(taker_token_a, create_token_account(&taker.pubkey(), &mint_a, 0)).unwrap();

    let escrow_id: u64 = 1;
    let (escrow_pda, _) = derive_escrow_pda(escrow_id);
    let (vault_pda, _) = derive_vault_pda(escrow_id);

    let make_ix = anchor_instruction(
        program_id,
        "make_escrow",
        &make_escrow_data(escrow_id, 500, 300), // Wants 300 Token B
        vec![
            signer_meta(maker.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(maker_token_a),
            writable_meta(escrow_pda),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, make_ix, &maker, &[&maker]).unwrap();

    svm.expire_blockhash();
    let take_ix = anchor_instruction(
        program_id,
        "take_escrow",
        &[],
        vec![
            signer_meta(taker.pubkey()),
            writable_meta(maker.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(taker_token_a),
            writable_meta(taker_token_b),
            writable_meta(maker_token_b),
            writable_meta(escrow_pda),
            writable_meta(vault_pda),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, take_ix, &taker, &[&taker]);
    assert!(result.is_err(), "take_escrow with insufficient funds should fail");
}

// =============================================================================
// CANCEL ESCROW TESTS
// =============================================================================

#[test]
fn test_cancel_escrow() {
    let mut svm = load_escrow_program();
    let program_id = escrow_program_id();

    let (_, mint_a) = setup_mint(&mut svm, 6);
    let (_, mint_b) = setup_mint(&mut svm, 6);

    let (maker, maker_token_a) = setup_user_with_tokens(&mut svm, &mint_a, 1000);

    let escrow_id: u64 = 1;
    let token_a_amount: u64 = 500;
    let (escrow_pda, _) = derive_escrow_pda(escrow_id);
    let (vault_pda, _) = derive_vault_pda(escrow_id);

    // Create escrow
    let make_ix = anchor_instruction(
        program_id,
        "make_escrow",
        &make_escrow_data(escrow_id, token_a_amount, 300),
        vec![
            signer_meta(maker.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(maker_token_a),
            writable_meta(escrow_pda),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, make_ix, &maker, &[&maker]).unwrap();

    // Verify maker has 500 tokens after making escrow
    let maker_account = svm.get_account(&maker_token_a).unwrap();
    assert_eq!(read_token_balance(&maker_account.data), 500);

    // Cancel escrow
    svm.expire_blockhash();
    let cancel_ix = anchor_instruction(
        program_id,
        "cancel_escrow",
        &[],
        vec![
            signer_meta(maker.pubkey()),        // maker
            writable_meta(mint_a),              // mint_a
            writable_meta(maker_token_a),       // maker_token_account_a
            writable_meta(escrow_pda),          // escrow
            writable_meta(vault_pda),           // vault
            readonly_meta(token_program_id()),   // token_program
        ],
    );

    let result = execute_tx(&mut svm, cancel_ix, &maker, &[&maker]);
    assert!(result.is_ok(), "cancel_escrow should succeed: {:?}", result);

    // Verify maker got tokens back
    let maker_account = svm.get_account(&maker_token_a).unwrap();
    assert_eq!(read_token_balance(&maker_account.data), 1000, "Maker should have all tokens back");

    // Verify vault is empty
    let vault_account = svm.get_account(&vault_pda);
    if let Some(vault) = vault_account {
        assert_eq!(read_token_balance(&vault.data), 0, "Vault should be empty after cancel");
    }
}

#[test]
fn test_cancel_escrow_unauthorized_fails() {
    let mut svm = load_escrow_program();
    let program_id = escrow_program_id();

    let (_, mint_a) = setup_mint(&mut svm, 6);
    let (_, mint_b) = setup_mint(&mut svm, 6);

    let (maker, maker_token_a) = setup_user_with_tokens(&mut svm, &mint_a, 1000);

    // Attacker
    let attacker = funded_keypair_10_sol(&mut svm);
    let attacker_token_a = get_ata(&attacker.pubkey(), &mint_a);
    svm.set_account(attacker_token_a, create_token_account(&attacker.pubkey(), &mint_a, 0)).unwrap();

    let escrow_id: u64 = 1;
    let (escrow_pda, _) = derive_escrow_pda(escrow_id);
    let (vault_pda, _) = derive_vault_pda(escrow_id);

    // Create escrow
    let make_ix = anchor_instruction(
        program_id,
        "make_escrow",
        &make_escrow_data(escrow_id, 500, 300),
        vec![
            signer_meta(maker.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(maker_token_a),
            writable_meta(escrow_pda),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, make_ix, &maker, &[&maker]).unwrap();

    // Attacker tries to cancel
    svm.expire_blockhash();
    let cancel_ix = anchor_instruction(
        program_id,
        "cancel_escrow",
        &[],
        vec![
            signer_meta(attacker.pubkey()),     // Attacker, not maker!
            writable_meta(mint_a),
            writable_meta(attacker_token_a),    // Attacker's token account
            writable_meta(escrow_pda),
            writable_meta(vault_pda),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, cancel_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "cancel_escrow by non-maker should fail");

    // Verify vault still has tokens
    let vault_account = svm.get_account(&vault_pda).unwrap();
    assert_eq!(read_token_balance(&vault_account.data), 500, "Vault should still have tokens");
}

// =============================================================================
// FULL WORKFLOW TESTS
// =============================================================================

#[test]
fn test_full_escrow_workflow_happy_path() {
    let mut svm = load_escrow_program();
    let program_id = escrow_program_id();

    let (_, mint_a) = setup_mint(&mut svm, 6);
    let (_, mint_b) = setup_mint(&mut svm, 6);

    // Maker starts with 1000 Token A, wants 500 Token B
    let (maker, maker_token_a) = setup_user_with_tokens(&mut svm, &mint_a, 1000);
    let maker_token_b = get_ata(&maker.pubkey(), &mint_b);
    svm.set_account(maker_token_b, create_token_account(&maker.pubkey(), &mint_b, 0)).unwrap();

    // Taker starts with 1000 Token B, will get Token A
    let (taker, taker_token_b) = setup_user_with_tokens(&mut svm, &mint_b, 1000);
    let taker_token_a = get_ata(&taker.pubkey(), &mint_a);
    svm.set_account(taker_token_a, create_token_account(&taker.pubkey(), &mint_a, 0)).unwrap();

    let escrow_id: u64 = 42;
    let token_a_amount: u64 = 600;
    let token_b_wanted: u64 = 400;
    let (escrow_pda, _) = derive_escrow_pda(escrow_id);
    let (vault_pda, _) = derive_vault_pda(escrow_id);

    // Step 1: Maker creates escrow, depositing 600 Token A
    let make_ix = anchor_instruction(
        program_id,
        "make_escrow",
        &make_escrow_data(escrow_id, token_a_amount, token_b_wanted),
        vec![
            signer_meta(maker.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(maker_token_a),
            writable_meta(escrow_pda),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, make_ix, &maker, &[&maker]).unwrap();

    // Verify state after make_escrow
    let maker_a_balance = read_token_balance(&svm.get_account(&maker_token_a).unwrap().data);
    assert_eq!(maker_a_balance, 400, "Maker should have 400 Token A left");

    let vault_balance = read_token_balance(&svm.get_account(&vault_pda).unwrap().data);
    assert_eq!(vault_balance, 600, "Vault should have 600 Token A");

    // Step 2: Taker accepts escrow
    svm.expire_blockhash();
    let take_ix = anchor_instruction(
        program_id,
        "take_escrow",
        &[],
        vec![
            signer_meta(taker.pubkey()),
            writable_meta(maker.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(taker_token_a),
            writable_meta(taker_token_b),
            writable_meta(maker_token_b),
            writable_meta(escrow_pda),
            writable_meta(vault_pda),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, take_ix, &taker, &[&taker]).unwrap();

    // Verify final state
    // Maker: kept 400 Token A, received 400 Token B
    let maker_a_final = read_token_balance(&svm.get_account(&maker_token_a).unwrap().data);
    let maker_b_final = read_token_balance(&svm.get_account(&maker_token_b).unwrap().data);
    assert_eq!(maker_a_final, 400, "Maker should still have 400 Token A");
    assert_eq!(maker_b_final, 400, "Maker should have received 400 Token B");

    // Taker: received 600 Token A, sent 400 Token B
    let taker_a_final = read_token_balance(&svm.get_account(&taker_token_a).unwrap().data);
    let taker_b_final = read_token_balance(&svm.get_account(&taker_token_b).unwrap().data);
    assert_eq!(taker_a_final, 600, "Taker should have received 600 Token A");
    assert_eq!(taker_b_final, 600, "Taker should have 600 Token B left");
}

#[test]
fn test_escrow_workflow_with_cancellation() {
    let mut svm = load_escrow_program();
    let program_id = escrow_program_id();

    let (_, mint_a) = setup_mint(&mut svm, 6);
    let (_, mint_b) = setup_mint(&mut svm, 6);

    let (maker, maker_token_a) = setup_user_with_tokens(&mut svm, &mint_a, 1000);

    let escrow_id: u64 = 1;
    let (escrow_pda, _) = derive_escrow_pda(escrow_id);
    let (vault_pda, _) = derive_vault_pda(escrow_id);

    // Create escrow
    let make_ix = anchor_instruction(
        program_id,
        "make_escrow",
        &make_escrow_data(escrow_id, 500, 300),
        vec![
            signer_meta(maker.pubkey()),
            writable_meta(mint_a),
            writable_meta(mint_b),
            writable_meta(maker_token_a),
            writable_meta(escrow_pda),
            writable_meta(vault_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, make_ix, &maker, &[&maker]).unwrap();

    // Verify escrow exists
    assert!(svm.get_account(&escrow_pda).is_some(), "Escrow should exist");
    let maker_balance = read_token_balance(&svm.get_account(&maker_token_a).unwrap().data);
    assert_eq!(maker_balance, 500, "Maker should have 500 after depositing");

    // Cancel escrow
    svm.expire_blockhash();
    let cancel_ix = anchor_instruction(
        program_id,
        "cancel_escrow",
        &[],
        vec![
            signer_meta(maker.pubkey()),
            writable_meta(mint_a),
            writable_meta(maker_token_a),
            writable_meta(escrow_pda),
            writable_meta(vault_pda),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, cancel_ix, &maker, &[&maker]).unwrap();

    // Verify maker got all tokens back
    let maker_final = read_token_balance(&svm.get_account(&maker_token_a).unwrap().data);
    assert_eq!(maker_final, 1000, "Maker should have all tokens back after cancel");
}
