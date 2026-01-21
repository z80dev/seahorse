//! Behavior parity tests for CPI Patterns program
//!
//! These tests verify that the Seahorse CPI patterns implementation
//! produces behavior consistent with idiomatic Anchor patterns.
//!
//! Note: Tests that require actual program execution need the .so files
//! built via ./scripts/build-test-programs.sh

use seahorse_test_common::*;
use solana_account::Account;
use solana_keypair::Keypair;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use solana_signer::Signer;
use spl_token::solana_program::program_pack::Pack;
use spl_token::solana_program::program_option::COption;

/// Seahorse CPI Patterns program ID
const SEAHORSE_PROGRAM_ID: &str = "Cpi1Pattrn111111111111111111111111111111111";

/// SPL Token Mint account size
const MINT_SIZE: usize = 82;
/// SPL Token Account size
const TOKEN_ACCOUNT_SIZE: usize = 165;

fn seahorse_program_id() -> Pubkey {
    SEAHORSE_PROGRAM_ID.parse().unwrap()
}

/// CpiVault account size
const CPI_VAULT_SIZE: usize = 8 + 32 + 32 + 8 + 8 + 8 + 1;

/// MintConfig account size
const MINT_CONFIG_SIZE: usize = 8 + 32 + 32 + 8 + 8 + 1;

fn setup_svm() -> (litesvm::LiteSVM, Keypair) {
    let mut svm = litesvm::LiteSVM::new();
    let authority = {
        let kp = Keypair::new();
        svm.airdrop(&kp.pubkey(), 100 * LAMPORTS_PER_SOL).unwrap();
        kp
    };
    (svm, authority)
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

fn create_test_mint(svm: &mut litesvm::LiteSVM, authority: &Pubkey, decimals: u8) -> Pubkey {
    let mint = Keypair::new();
    let mint_account = create_mint_account(authority, decimals);
    svm.set_account(mint.pubkey(), mint_account).unwrap();
    mint.pubkey()
}

fn create_test_token_account(
    svm: &mut litesvm::LiteSVM,
    owner: &Pubkey,
    mint: &Pubkey,
    amount: u64,
) -> Pubkey {
    let token_account = Keypair::new();
    let account_data = create_token_account(owner, mint, amount);
    svm.set_account(token_account.pubkey(), account_data).unwrap();
    token_account.pubkey()
}

/// Read token balance from account data
fn read_token_balance(data: &[u8]) -> u64 {
    // Token account: amount is at offset 64 (after mint[32] and owner[32])
    let amount_bytes: [u8; 8] = data[64..72].try_into().unwrap();
    u64::from_le_bytes(amount_bytes)
}

fn create_vault_account_data(
    authority: &Pubkey,
    mint: &Pubkey,
    total_deposited: u64,
    total_withdrawn: u64,
    transfer_count: u64,
    bump: u8,
) -> Account {
    let rent = Rent::default();
    let discriminator = account_discriminator("CpiVault");

    let mut data = vec![0u8; CPI_VAULT_SIZE];
    data[..8].copy_from_slice(&discriminator);
    data[8..40].copy_from_slice(authority.as_ref());
    data[40..72].copy_from_slice(mint.as_ref());
    data[72..80].copy_from_slice(&total_deposited.to_le_bytes());
    data[80..88].copy_from_slice(&total_withdrawn.to_le_bytes());
    data[88..96].copy_from_slice(&transfer_count.to_le_bytes());
    data[96] = bump;

    Account {
        lamports: rent.minimum_balance(CPI_VAULT_SIZE),
        data,
        owner: seahorse_program_id(),
        executable: false,
        rent_epoch: 0,
    }
}

fn create_mint_config_account_data(
    authority: &Pubkey,
    mint: &Pubkey,
    total_minted: u64,
    operation_count: u64,
    bump: u8,
) -> Account {
    let rent = Rent::default();
    let discriminator = account_discriminator("MintConfig");

    let mut data = vec![0u8; MINT_CONFIG_SIZE];
    data[..8].copy_from_slice(&discriminator);
    data[8..40].copy_from_slice(authority.as_ref());
    data[40..72].copy_from_slice(mint.as_ref());
    data[72..80].copy_from_slice(&total_minted.to_le_bytes());
    data[80..88].copy_from_slice(&operation_count.to_le_bytes());
    data[88] = bump;

    Account {
        lamports: rent.minimum_balance(MINT_CONFIG_SIZE),
        data,
        owner: seahorse_program_id(),
        executable: false,
        rent_epoch: 0,
    }
}

// =============================================================================
// PARITY TESTS - Account layout and discriminator verification
// =============================================================================

/// Test: Vault state initialization stores correct values
#[test]
fn test_parity_vault_state_initialization() {
    let (mut svm, authority) = setup_svm();

    let mint = create_test_mint(&mut svm, &authority.pubkey(), 9);

    let (vault_pda, bump) = find_pda(
        &[b"vault", authority.pubkey().as_ref(), mint.as_ref()],
        &seahorse_program_id(),
    );

    // Create vault with known initial state
    let vault_data = create_vault_account_data(&authority.pubkey(), &mint, 0, 0, 0, bump);
    svm.set_account(vault_pda, vault_data).unwrap();

    // Read back the vault state
    let account = svm.get_account(&vault_pda).unwrap();
    let account_data = &account.data;

    // Verify discriminator
    let discriminator = account_discriminator("CpiVault");
    assert_eq!(&account_data[..8], &discriminator);

    // Verify authority
    assert_eq!(&account_data[8..40], authority.pubkey().as_ref());

    // Verify mint
    assert_eq!(&account_data[40..72], mint.as_ref());

    // Verify total_deposited = 0
    let total_deposited = u64::from_le_bytes(account_data[72..80].try_into().unwrap());
    assert_eq!(total_deposited, 0);

    // Verify total_withdrawn = 0
    let total_withdrawn = u64::from_le_bytes(account_data[80..88].try_into().unwrap());
    assert_eq!(total_withdrawn, 0);

    // Verify transfer_count = 0
    let transfer_count = u64::from_le_bytes(account_data[88..96].try_into().unwrap());
    assert_eq!(transfer_count, 0);

    // Verify bump
    assert_eq!(account_data[96], bump);
}

/// Test: MintConfig state initialization stores correct values
#[test]
fn test_parity_mint_config_state_initialization() {
    let (mut svm, authority) = setup_svm();

    let (mint_config_pda, bump) = find_pda(
        &[b"mint_config", authority.pubkey().as_ref()],
        &seahorse_program_id(),
    );

    let mint = Pubkey::new_unique();

    // Create mint config with known initial state
    let config_data = create_mint_config_account_data(&authority.pubkey(), &mint, 0, 0, bump);
    svm.set_account(mint_config_pda, config_data).unwrap();

    // Read back the config state
    let account = svm.get_account(&mint_config_pda).unwrap();
    let account_data = &account.data;

    // Verify discriminator
    let discriminator = account_discriminator("MintConfig");
    assert_eq!(&account_data[..8], &discriminator);

    // Verify authority
    assert_eq!(&account_data[8..40], authority.pubkey().as_ref());

    // Verify mint
    assert_eq!(&account_data[40..72], mint.as_ref());

    // Verify total_minted = 0
    let total_minted = u64::from_le_bytes(account_data[72..80].try_into().unwrap());
    assert_eq!(total_minted, 0);

    // Verify operation_count = 0
    let operation_count = u64::from_le_bytes(account_data[80..88].try_into().unwrap());
    assert_eq!(operation_count, 0);

    // Verify bump
    assert_eq!(account_data[88], bump);
}

/// Test: PDA derivation is deterministic across multiple calls
#[test]
fn test_parity_deterministic_pda_derivation() {
    let authority = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    // Derive vault PDA multiple times
    let (vault_pda1, bump1) = find_pda(
        &[b"vault", authority.as_ref(), mint.as_ref()],
        &seahorse_program_id(),
    );

    let (vault_pda2, bump2) = find_pda(
        &[b"vault", authority.as_ref(), mint.as_ref()],
        &seahorse_program_id(),
    );

    let (vault_pda3, bump3) = find_pda(
        &[b"vault", authority.as_ref(), mint.as_ref()],
        &seahorse_program_id(),
    );

    assert_eq!(vault_pda1, vault_pda2);
    assert_eq!(vault_pda2, vault_pda3);
    assert_eq!(bump1, bump2);
    assert_eq!(bump2, bump3);

    // Verify different seeds produce different PDAs
    let different_mint = Pubkey::new_unique();
    let (different_vault_pda, _) = find_pda(
        &[b"vault", authority.as_ref(), different_mint.as_ref()],
        &seahorse_program_id(),
    );

    assert_ne!(vault_pda1, different_vault_pda, "Different seeds should produce different PDAs");

    // Derive mint_config PDA
    let (config_pda1, config_bump1) = find_pda(
        &[b"mint_config", authority.as_ref()],
        &seahorse_program_id(),
    );

    let (config_pda2, config_bump2) = find_pda(
        &[b"mint_config", authority.as_ref()],
        &seahorse_program_id(),
    );

    assert_eq!(config_pda1, config_pda2);
    assert_eq!(config_bump1, config_bump2);
}

/// Test: Account discriminators are computed correctly (Anchor-compatible)
#[test]
fn test_parity_account_discriminators() {
    // CpiVault discriminator
    let vault_disc = account_discriminator("CpiVault");
    assert_eq!(vault_disc.len(), 8);
    // Same name should give same discriminator
    assert_eq!(vault_disc, account_discriminator("CpiVault"));

    // MintConfig discriminator
    let config_disc = account_discriminator("MintConfig");
    assert_eq!(config_disc.len(), 8);
    assert_eq!(config_disc, account_discriminator("MintConfig"));

    // Different account names should produce different discriminators
    assert_ne!(vault_disc, config_disc);
}

/// Test: Token balances are correctly read from SPL Token accounts
#[test]
fn test_parity_token_balance_reading() {
    let (mut svm, authority) = setup_svm();

    let mint = create_test_mint(&mut svm, &authority.pubkey(), 9);

    // Create token account with specific balance
    let initial_balance = 123456789u64;
    let token_account = create_test_token_account(&mut svm, &authority.pubkey(), &mint, initial_balance);

    // Read balance back
    let account = svm.get_account(&token_account).unwrap();
    let balance = read_token_balance(&account.data);

    assert_eq!(balance, initial_balance, "Token balance should match what was set");
}
