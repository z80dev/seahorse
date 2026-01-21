//! LiteSVM integration tests for the Seahorse Token Mint program
//!
//! These tests verify token minting and burning with PDA as mint authority.
//!
//! # Prerequisites
//! Run `./scripts/build-test-programs.sh` to compile token_mint.so

use seahorse_integration_tests::helpers::*;
use seahorse_integration_tests::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use std::str::FromStr;

/// Token Mint program ID (from declare_id!)
fn token_mint_program_id() -> Pubkey {
    Pubkey::from_str("MiNT5VzVZYxLYtDmJq5DEmUKDEcPnP2zGxqHKbv2PgZ").unwrap()
}

/// MintConfig account size: discriminator (8) + mint (32) + authority (32) + decimals (1) +
/// total_minted (8) + total_burned (8) + bump (1)
#[allow(dead_code)]
const MINT_CONFIG_SIZE: usize = 8 + 32 + 32 + 1 + 8 + 8 + 1;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the token mint program into LiteSVM
fn load_token_mint_program() -> litesvm::LiteSVM {
    let program_id = token_mint_program_id();
    let program_bytes = std::fs::read("../../target/deploy/token_mint.so")
        .expect("Failed to read token_mint.so - run ./scripts/build-test-programs.sh first");

    let mut svm = litesvm::LiteSVM::new();
    svm.add_program(program_id, &program_bytes);
    svm
}

/// Derive mint_config PDA
fn derive_mint_config_pda(authority: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"mint_config", authority.as_ref()],
        &token_mint_program_id(),
    )
}

/// Derive mint PDA
fn derive_mint_pda(authority: &Pubkey) -> (Pubkey, u8) {
    find_pda(
        &[b"mint", authority.as_ref()],
        &token_mint_program_id(),
    )
}

/// Read mint config fields from account data
fn read_mint_config(data: &[u8]) -> (Pubkey, Pubkey, u8, u64, u64, u8) {
    // discriminator (8) + mint (32) + authority (32) + decimals (1) + total_minted (8) + total_burned (8) + bump (1)
    let mint = Pubkey::new_from_array(data[8..40].try_into().unwrap());
    let authority = Pubkey::new_from_array(data[40..72].try_into().unwrap());
    let decimals = data[72];
    let total_minted = u64::from_le_bytes(data[73..81].try_into().unwrap());
    let total_burned = u64::from_le_bytes(data[81..89].try_into().unwrap());
    let bump = data[89];
    (mint, authority, decimals, total_minted, total_burned, bump)
}

/// Setup a user with a token account for a mint (with tokens from existing supply)
fn setup_user_token_account(
    svm: &mut litesvm::LiteSVM,
    user: &Pubkey,
    mint: &Pubkey,
    amount: u64,
) -> Pubkey {
    let user_ata = get_ata(user, mint);
    let token_account = create_token_account(user, mint, amount);
    svm.set_account(user_ata, token_account).unwrap();
    user_ata
}

// =============================================================================
// CREATE MINT TESTS
// =============================================================================

#[test]
fn test_create_mint() {
    let mut svm = load_token_mint_program();
    let program_id = token_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let (mint_config_pda, _) = derive_mint_config_pda(&authority.pubkey());
    let (mint_pda, _) = derive_mint_pda(&authority.pubkey());

    let decimals: u8 = 6;
    let ix = anchor_instruction(
        program_id,
        "create_mint",
        &[decimals],
        vec![
            signer_meta(authority.pubkey()),      // authority
            writable_meta(mint_config_pda),       // mint_config
            writable_meta(mint_pda),              // mint
            readonly_meta(solana_sdk_ids::sysvar::rent::id()), // rent
            readonly_meta(system_program::id()), // system_program
            readonly_meta(token_program_id()),   // token_program
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Create mint should succeed: {:?}", result);

    // Verify mint_config was created
    let mint_config_account = svm.get_account(&mint_config_pda).expect("MintConfig should exist");
    assert_eq!(mint_config_account.owner, program_id, "MintConfig should be owned by program");

    let (stored_mint, stored_authority, stored_decimals, total_minted, total_burned, _) =
        read_mint_config(&mint_config_account.data);

    assert_eq!(stored_mint, mint_pda, "Stored mint should match");
    assert_eq!(stored_authority, authority.pubkey(), "Stored authority should match");
    assert_eq!(stored_decimals, decimals, "Stored decimals should match");
    assert_eq!(total_minted, 0, "Initial total_minted should be 0");
    assert_eq!(total_burned, 0, "Initial total_burned should be 0");

    // Verify mint was created
    let mint_account = svm.get_account(&mint_pda).expect("Mint should exist");
    assert_eq!(mint_account.owner, token_program_id(), "Mint should be owned by token program");

    let mint_supply = read_mint_supply(&mint_account.data);
    assert_eq!(mint_supply, 0, "Initial supply should be 0");
}

#[test]
fn test_create_mint_different_decimals() {
    let mut svm = load_token_mint_program();
    let program_id = token_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let (mint_config_pda, _) = derive_mint_config_pda(&authority.pubkey());
    let (mint_pda, _) = derive_mint_pda(&authority.pubkey());

    // Test with 9 decimals (common for many tokens)
    let decimals: u8 = 9;
    let ix = anchor_instruction(
        program_id,
        "create_mint",
        &[decimals],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint_config_pda),
            writable_meta(mint_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Create mint with 9 decimals should succeed");

    let mint_config_account = svm.get_account(&mint_config_pda).unwrap();
    let (_, _, stored_decimals, _, _, _) = read_mint_config(&mint_config_account.data);
    assert_eq!(stored_decimals, 9);
}

#[test]
fn test_create_mint_different_authorities() {
    let mut svm = load_token_mint_program();
    let program_id = token_mint_program_id();

    let authority1 = funded_keypair_10_sol(&mut svm);
    let authority2 = funded_keypair_10_sol(&mut svm);

    // Authority 1's mint
    let (mint_config1, _) = derive_mint_config_pda(&authority1.pubkey());
    let (mint1, _) = derive_mint_pda(&authority1.pubkey());

    // Authority 2's mint
    let (mint_config2, _) = derive_mint_config_pda(&authority2.pubkey());
    let (mint2, _) = derive_mint_pda(&authority2.pubkey());

    // PDAs should be different
    assert_ne!(mint_config1, mint_config2, "MintConfig PDAs should differ");
    assert_ne!(mint1, mint2, "Mint PDAs should differ");

    // Create first mint
    let ix1 = anchor_instruction(
        program_id,
        "create_mint",
        &[6u8],
        vec![
            signer_meta(authority1.pubkey()),
            writable_meta(mint_config1),
            writable_meta(mint1),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix1, &authority1, &[&authority1]).unwrap();

    // Create second mint
    svm.expire_blockhash();
    let ix2 = anchor_instruction(
        program_id,
        "create_mint",
        &[9u8],
        vec![
            signer_meta(authority2.pubkey()),
            writable_meta(mint_config2),
            writable_meta(mint2),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, ix2, &authority2, &[&authority2]).unwrap();

    // Verify both exist independently
    let config1 = svm.get_account(&mint_config1).unwrap();
    let config2 = svm.get_account(&mint_config2).unwrap();

    let (_, auth1, dec1, _, _, _) = read_mint_config(&config1.data);
    let (_, auth2, dec2, _, _, _) = read_mint_config(&config2.data);

    assert_eq!(auth1, authority1.pubkey());
    assert_eq!(auth2, authority2.pubkey());
    assert_eq!(dec1, 6);
    assert_eq!(dec2, 9);
}

// =============================================================================
// MINT TOKENS TESTS
// =============================================================================

#[test]
fn test_mint_tokens() {
    let mut svm = load_token_mint_program();
    let program_id = token_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let (mint_config_pda, _) = derive_mint_config_pda(&authority.pubkey());
    let (mint_pda, _) = derive_mint_pda(&authority.pubkey());

    // Create mint first
    let create_ix = anchor_instruction(
        program_id,
        "create_mint",
        &[6u8],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint_config_pda),
            writable_meta(mint_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Create destination token account
    let destination_ata = setup_user_token_account(&mut svm, &authority.pubkey(), &mint_pda, 0);

    // Mint tokens
    svm.expire_blockhash();
    let mint_amount: u64 = 1000_000_000; // 1000 tokens with 6 decimals
    let mint_ix = anchor_instruction(
        program_id,
        "mint_tokens",
        &mint_amount.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),      // authority
            writable_meta(mint_config_pda),       // mint_config
            writable_meta(mint_pda),              // mint
            writable_meta(destination_ata),       // destination
            readonly_meta(token_program_id()),   // token_program
        ],
    );

    let result = execute_tx(&mut svm, mint_ix, &authority, &[&authority]);
    assert!(result.is_ok(), "Mint tokens should succeed: {:?}", result);

    // Verify token balance
    let dest_account = svm.get_account(&destination_ata).unwrap();
    assert_eq!(read_token_balance(&dest_account.data), mint_amount, "Destination should have minted tokens");

    // Verify mint supply
    let mint_account = svm.get_account(&mint_pda).unwrap();
    assert_eq!(read_mint_supply(&mint_account.data), mint_amount, "Supply should match");

    // Verify total_minted in config
    let config_account = svm.get_account(&mint_config_pda).unwrap();
    let (_, _, _, total_minted, _, _) = read_mint_config(&config_account.data);
    assert_eq!(total_minted, mint_amount, "total_minted should be tracked");
}

#[test]
fn test_mint_tokens_multiple_times() {
    let mut svm = load_token_mint_program();
    let program_id = token_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let (mint_config_pda, _) = derive_mint_config_pda(&authority.pubkey());
    let (mint_pda, _) = derive_mint_pda(&authority.pubkey());

    // Create mint
    let create_ix = anchor_instruction(
        program_id,
        "create_mint",
        &[6u8],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint_config_pda),
            writable_meta(mint_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    let destination_ata = setup_user_token_account(&mut svm, &authority.pubkey(), &mint_pda, 0);

    // Mint 3 times
    let mut total = 0u64;
    for i in 1..=3 {
        svm.expire_blockhash();
        let amount: u64 = i * 100_000_000; // 100, 200, 300 tokens
        total += amount;

        let mint_ix = anchor_instruction(
            program_id,
            "mint_tokens",
            &amount.to_le_bytes(),
            vec![
                signer_meta(authority.pubkey()),
                writable_meta(mint_config_pda),
                writable_meta(mint_pda),
                writable_meta(destination_ata),
                readonly_meta(token_program_id()),
            ],
        );
        let result = execute_tx(&mut svm, mint_ix, &authority, &[&authority]);
        assert!(result.is_ok(), "Mint {} should succeed", i);
    }

    // Verify final balance
    let dest_account = svm.get_account(&destination_ata).unwrap();
    assert_eq!(read_token_balance(&dest_account.data), total);

    let config_account = svm.get_account(&mint_config_pda).unwrap();
    let (_, _, _, total_minted, _, _) = read_mint_config(&config_account.data);
    assert_eq!(total_minted, total);
}

#[test]
fn test_mint_to_different_recipients() {
    let mut svm = load_token_mint_program();
    let program_id = token_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let recipient1 = Keypair::new();
    let recipient2 = Keypair::new();

    let (mint_config_pda, _) = derive_mint_config_pda(&authority.pubkey());
    let (mint_pda, _) = derive_mint_pda(&authority.pubkey());

    // Create mint
    let create_ix = anchor_instruction(
        program_id,
        "create_mint",
        &[6u8],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint_config_pda),
            writable_meta(mint_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Setup token accounts for recipients
    let ata1 = setup_user_token_account(&mut svm, &recipient1.pubkey(), &mint_pda, 0);
    let ata2 = setup_user_token_account(&mut svm, &recipient2.pubkey(), &mint_pda, 0);

    // Mint to recipient1
    svm.expire_blockhash();
    let mint_ix1 = anchor_instruction(
        program_id,
        "mint_tokens",
        &500_000_000u64.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint_config_pda),
            writable_meta(mint_pda),
            writable_meta(ata1),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, mint_ix1, &authority, &[&authority]).unwrap();

    // Mint to recipient2
    svm.expire_blockhash();
    let mint_ix2 = anchor_instruction(
        program_id,
        "mint_tokens",
        &300_000_000u64.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint_config_pda),
            writable_meta(mint_pda),
            writable_meta(ata2),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, mint_ix2, &authority, &[&authority]).unwrap();

    // Verify individual balances
    let account1 = svm.get_account(&ata1).unwrap();
    assert_eq!(read_token_balance(&account1.data), 500_000_000);

    let account2 = svm.get_account(&ata2).unwrap();
    assert_eq!(read_token_balance(&account2.data), 300_000_000);

    // Verify total supply
    let mint_account = svm.get_account(&mint_pda).unwrap();
    assert_eq!(read_mint_supply(&mint_account.data), 800_000_000);
}

#[test]
fn test_mint_tokens_zero_fails() {
    let mut svm = load_token_mint_program();
    let program_id = token_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let (mint_config_pda, _) = derive_mint_config_pda(&authority.pubkey());
    let (mint_pda, _) = derive_mint_pda(&authority.pubkey());

    // Create mint
    let create_ix = anchor_instruction(
        program_id,
        "create_mint",
        &[6u8],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint_config_pda),
            writable_meta(mint_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    let destination_ata = setup_user_token_account(&mut svm, &authority.pubkey(), &mint_pda, 0);

    // Try to mint 0 tokens
    svm.expire_blockhash();
    let mint_ix = anchor_instruction(
        program_id,
        "mint_tokens",
        &0u64.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint_config_pda),
            writable_meta(mint_pda),
            writable_meta(destination_ata),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, mint_ix, &authority, &[&authority]);
    assert!(result.is_err(), "Minting 0 tokens should fail");
}

#[test]
fn test_mint_tokens_unauthorized_fails() {
    let mut svm = load_token_mint_program();
    let program_id = token_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let attacker = funded_keypair_10_sol(&mut svm);

    let (mint_config_pda, _) = derive_mint_config_pda(&authority.pubkey());
    let (mint_pda, _) = derive_mint_pda(&authority.pubkey());

    // Create mint by authority
    let create_ix = anchor_instruction(
        program_id,
        "create_mint",
        &[6u8],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint_config_pda),
            writable_meta(mint_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    let attacker_ata = setup_user_token_account(&mut svm, &attacker.pubkey(), &mint_pda, 0);

    // Attacker tries to mint
    svm.expire_blockhash();
    let mint_ix = anchor_instruction(
        program_id,
        "mint_tokens",
        &1000u64.to_le_bytes(),
        vec![
            signer_meta(attacker.pubkey()),       // attacker signs
            writable_meta(mint_config_pda),       // but mint_config.authority != attacker
            writable_meta(mint_pda),
            writable_meta(attacker_ata),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, mint_ix, &attacker, &[&attacker]);
    assert!(result.is_err(), "Unauthorized mint should fail");
}

// =============================================================================
// BURN TOKENS TESTS
// =============================================================================

#[test]
fn test_burn_tokens() {
    let mut svm = load_token_mint_program();
    let program_id = token_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let token_owner = funded_keypair_10_sol(&mut svm);

    let (mint_config_pda, _) = derive_mint_config_pda(&authority.pubkey());
    let (mint_pda, _) = derive_mint_pda(&authority.pubkey());

    // Create mint
    let create_ix = anchor_instruction(
        program_id,
        "create_mint",
        &[6u8],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint_config_pda),
            writable_meta(mint_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Mint tokens to token_owner
    let owner_ata = setup_user_token_account(&mut svm, &token_owner.pubkey(), &mint_pda, 0);

    svm.expire_blockhash();
    let mint_amount: u64 = 1000_000_000;
    let mint_ix = anchor_instruction(
        program_id,
        "mint_tokens",
        &mint_amount.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint_config_pda),
            writable_meta(mint_pda),
            writable_meta(owner_ata),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, mint_ix, &authority, &[&authority]).unwrap();

    // Burn tokens
    svm.expire_blockhash();
    let burn_amount: u64 = 300_000_000;
    let burn_ix = anchor_instruction(
        program_id,
        "burn_tokens",
        &burn_amount.to_le_bytes(),
        vec![
            signer_meta(token_owner.pubkey()),    // owner (token account authority)
            writable_meta(authority.pubkey()),   // authority (for PDA derivation, Seahorse marks mut)
            writable_meta(mint_config_pda),      // mint_config
            writable_meta(mint_pda),             // mint
            writable_meta(owner_ata),            // source
            readonly_meta(token_program_id()),   // token_program
        ],
    );

    let result = execute_tx(&mut svm, burn_ix, &token_owner, &[&token_owner]);
    assert!(result.is_ok(), "Burn should succeed: {:?}", result);

    // Verify balance decreased
    let owner_account = svm.get_account(&owner_ata).unwrap();
    assert_eq!(read_token_balance(&owner_account.data), 700_000_000);

    // Verify supply decreased
    let mint_account = svm.get_account(&mint_pda).unwrap();
    assert_eq!(read_mint_supply(&mint_account.data), 700_000_000);

    // Verify total_burned tracked
    let config_account = svm.get_account(&mint_config_pda).unwrap();
    let (_, _, _, _, total_burned, _) = read_mint_config(&config_account.data);
    assert_eq!(total_burned, 300_000_000);
}

#[test]
fn test_burn_tokens_zero_fails() {
    let mut svm = load_token_mint_program();
    let program_id = token_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let token_owner = funded_keypair_10_sol(&mut svm);

    let (mint_config_pda, _) = derive_mint_config_pda(&authority.pubkey());
    let (mint_pda, _) = derive_mint_pda(&authority.pubkey());

    // Create mint and mint tokens
    let create_ix = anchor_instruction(
        program_id,
        "create_mint",
        &[6u8],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint_config_pda),
            writable_meta(mint_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    let owner_ata = setup_user_token_account(&mut svm, &token_owner.pubkey(), &mint_pda, 1000_000_000);

    // Try to burn 0
    svm.expire_blockhash();
    let burn_ix = anchor_instruction(
        program_id,
        "burn_tokens",
        &0u64.to_le_bytes(),
        vec![
            signer_meta(token_owner.pubkey()),
            writable_meta(authority.pubkey()),
            writable_meta(mint_config_pda),
            writable_meta(mint_pda),
            writable_meta(owner_ata),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, burn_ix, &token_owner, &[&token_owner]);
    assert!(result.is_err(), "Burning 0 should fail");
}

#[test]
fn test_burn_tokens_insufficient_funds_fails() {
    let mut svm = load_token_mint_program();
    let program_id = token_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let token_owner = funded_keypair_10_sol(&mut svm);

    let (mint_config_pda, _) = derive_mint_config_pda(&authority.pubkey());
    let (mint_pda, _) = derive_mint_pda(&authority.pubkey());

    // Create mint
    let create_ix = anchor_instruction(
        program_id,
        "create_mint",
        &[6u8],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint_config_pda),
            writable_meta(mint_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // Setup owner with only 100 tokens
    let owner_ata = setup_user_token_account(&mut svm, &token_owner.pubkey(), &mint_pda, 100_000_000);

    // Try to burn 200 tokens
    svm.expire_blockhash();
    let burn_ix = anchor_instruction(
        program_id,
        "burn_tokens",
        &200_000_000u64.to_le_bytes(),
        vec![
            signer_meta(token_owner.pubkey()),
            writable_meta(authority.pubkey()),
            writable_meta(mint_config_pda),
            writable_meta(mint_pda),
            writable_meta(owner_ata),
            readonly_meta(token_program_id()),
        ],
    );

    let result = execute_tx(&mut svm, burn_ix, &token_owner, &[&token_owner]);
    assert!(result.is_err(), "Burning more than balance should fail");
}

// =============================================================================
// FULL WORKFLOW TESTS
// =============================================================================

#[test]
fn test_full_mint_burn_workflow() {
    let mut svm = load_token_mint_program();
    let program_id = token_mint_program_id();

    let authority = funded_keypair_10_sol(&mut svm);
    let user = funded_keypair_10_sol(&mut svm);

    let (mint_config_pda, _) = derive_mint_config_pda(&authority.pubkey());
    let (mint_pda, _) = derive_mint_pda(&authority.pubkey());

    // 1. Create mint
    let create_ix = anchor_instruction(
        program_id,
        "create_mint",
        &[6u8],
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint_config_pda),
            writable_meta(mint_pda),
            readonly_meta(solana_sdk_ids::sysvar::rent::id()),
            readonly_meta(system_program::id()),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, create_ix, &authority, &[&authority]).unwrap();

    // 2. Setup user token account
    let user_ata = setup_user_token_account(&mut svm, &user.pubkey(), &mint_pda, 0);

    // 3. Mint 1000 tokens to user
    svm.expire_blockhash();
    let mint_ix1 = anchor_instruction(
        program_id,
        "mint_tokens",
        &1000_000_000u64.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint_config_pda),
            writable_meta(mint_pda),
            writable_meta(user_ata),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, mint_ix1, &authority, &[&authority]).unwrap();

    // 4. User burns 300 tokens
    svm.expire_blockhash();
    let burn_ix = anchor_instruction(
        program_id,
        "burn_tokens",
        &300_000_000u64.to_le_bytes(),
        vec![
            signer_meta(user.pubkey()),
            writable_meta(authority.pubkey()),
            writable_meta(mint_config_pda),
            writable_meta(mint_pda),
            writable_meta(user_ata),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, burn_ix, &user, &[&user]).unwrap();

    // 5. Mint another 500 tokens
    svm.expire_blockhash();
    let mint_ix2 = anchor_instruction(
        program_id,
        "mint_tokens",
        &500_000_000u64.to_le_bytes(),
        vec![
            signer_meta(authority.pubkey()),
            writable_meta(mint_config_pda),
            writable_meta(mint_pda),
            writable_meta(user_ata),
            readonly_meta(token_program_id()),
        ],
    );
    execute_tx(&mut svm, mint_ix2, &authority, &[&authority]).unwrap();

    // Final state: 1000 - 300 + 500 = 1200 tokens
    let user_account = svm.get_account(&user_ata).unwrap();
    assert_eq!(read_token_balance(&user_account.data), 1200_000_000);

    let config_account = svm.get_account(&mint_config_pda).unwrap();
    let (_, _, _, total_minted, total_burned, _) = read_mint_config(&config_account.data);
    assert_eq!(total_minted, 1500_000_000, "Total minted: 1000 + 500");
    assert_eq!(total_burned, 300_000_000, "Total burned: 300");

    // Current supply: 1500 - 300 = 1200
    let mint_account = svm.get_account(&mint_pda).unwrap();
    assert_eq!(read_mint_supply(&mint_account.data), 1200_000_000);
}
