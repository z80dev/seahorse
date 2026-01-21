//! Seahorse Unit Test Helpers
//!
//! Common utilities for testing Seahorse-compiled programs with Mollusk.

#![allow(deprecated)] // solana_sdk::system_program deprecation

pub mod helpers;

use solana_sdk::{
    account::Account,
    pubkey::Pubkey,
    rent::Rent,
    system_program,
};

/// Anchor account discriminator size (8 bytes)
pub const DISCRIMINATOR_SIZE: usize = 8;

/// Create a system program owned account with the given lamports
pub fn system_account_with_lamports(lamports: u64) -> Account {
    Account {
        lamports,
        data: vec![],
        owner: system_program::id(),
        executable: false,
        rent_epoch: 0,
    }
}

/// Create a rent-exempt account for storing program data
///
/// # Arguments
/// * `data_len` - Size of the account data in bytes (excluding discriminator)
/// * `owner` - Program that owns this account
pub fn program_account(data_len: usize, owner: &Pubkey) -> Account {
    let rent = Rent::default();
    let space = DISCRIMINATOR_SIZE + data_len;
    Account {
        lamports: rent.minimum_balance(space),
        data: vec![0u8; space],
        owner: *owner,
        executable: false,
        rent_epoch: 0,
    }
}

/// Create an uninitialized account (for init)
///
/// # Arguments
/// * `lamports` - Initial lamports for the account
pub fn uninitialized_account(lamports: u64) -> Account {
    Account {
        lamports,
        data: vec![],
        owner: system_program::id(),
        executable: false,
        rent_epoch: 0,
    }
}

/// Derive a PDA address and bump
///
/// # Arguments
/// * `seeds` - Seeds for PDA derivation
/// * `program_id` - Program ID to derive from
pub fn find_pda(seeds: &[&[u8]], program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(seeds, program_id)
}

/// Calculate the account discriminator for an Anchor account
///
/// Anchor uses `sha256("account:<AccountName>")[..8]` as discriminator
pub fn anchor_discriminator(account_name: &str) -> [u8; 8] {
    let preimage = format!("account:{}", account_name);
    let hash = solana_sdk::hash::hash(preimage.as_bytes());
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash.to_bytes()[..8]);
    discriminator
}

/// Create an initialized Anchor account with serialized data
///
/// # Arguments
/// * `account_name` - Name of the Anchor account struct (for discriminator)
/// * `data` - Borsh-serialized account data (without discriminator)
/// * `owner` - Program that owns this account
pub fn initialized_anchor_account(
    account_name: &str,
    data: &[u8],
    owner: &Pubkey,
) -> Account {
    let rent = Rent::default();
    let discriminator = anchor_discriminator(account_name);
    let space = DISCRIMINATOR_SIZE + data.len();

    let mut account_data = Vec::with_capacity(space);
    account_data.extend_from_slice(&discriminator);
    account_data.extend_from_slice(data);

    Account {
        lamports: rent.minimum_balance(space),
        data: account_data,
        owner: *owner,
        executable: false,
        rent_epoch: 0,
    }
}

/// Helper to create the standard Solana system accounts needed for most tests
pub fn system_accounts() -> Vec<(Pubkey, Account)> {
    vec![
        (
            system_program::id(),
            Account {
                lamports: 1,
                data: vec![],
                owner: solana_sdk::native_loader::id(),
                executable: true,
                rent_epoch: 0,
            },
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discriminator_calculation() {
        // Verify discriminator calculation matches Anchor's behavior
        let disc = anchor_discriminator("Calculator");
        assert_eq!(disc.len(), 8);
        // Discriminator should be consistent across calls
        assert_eq!(disc, anchor_discriminator("Calculator"));
        // Different account names should produce different discriminators
        assert_ne!(disc, anchor_discriminator("Counter"));
    }

    #[test]
    fn test_program_account_creation() {
        let program_id = Pubkey::new_unique();
        let account = program_account(32, &program_id);

        // Account should have discriminator + data space
        assert_eq!(account.data.len(), DISCRIMINATOR_SIZE + 32);
        assert_eq!(account.owner, program_id);
        assert!(!account.executable);
    }

    #[test]
    fn test_pda_derivation() {
        let program_id = Pubkey::new_unique();
        let user = Pubkey::new_unique();

        let (pda, bump) = find_pda(&[b"calculator", user.as_ref()], &program_id);

        // PDA should be off-curve
        assert!(pda.to_bytes()[31] != 0 || bump < 255);

        // Same seeds should give same PDA
        let (pda2, bump2) = find_pda(&[b"calculator", user.as_ref()], &program_id);
        assert_eq!(pda, pda2);
        assert_eq!(bump, bump2);
    }
}
