//! Structural parity tests for init_if_needed_test program
//!
//! These tests verify discriminators and PDA derivations for the Seahorse
//! init_if_needed_test program. There is no Anchor reference program, so we
//! check structural invariants instead of behavioral parity.

use solana_pubkey::Pubkey;
use solana_sha256_hasher::hash;
use std::str::FromStr;

/// init_if_needed_test program ID (Seahorse)
fn program_id() -> Pubkey {
    Pubkey::from_str("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS").unwrap()
}

/// Calculate Anchor account discriminator
fn account_discriminator(account_name: &str) -> [u8; 8] {
    let preimage = format!("account:{}", account_name);
    let hash = hash(preimage.as_bytes());
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash.as_ref()[..8]);
    discriminator
}

/// Calculate Anchor instruction discriminator
fn instruction_discriminator(name: &str) -> [u8; 8] {
    let preimage = format!("global:{}", name);
    let hash = hash(preimage.as_bytes());
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash.as_ref()[..8]);
    discriminator
}

#[test]
fn test_counter_account_discriminator() {
    let disc = account_discriminator("Counter");
    assert_eq!(disc.len(), 8);
    assert_ne!(disc, [0u8; 8]);
}

#[test]
fn test_instruction_discriminator() {
    let disc = instruction_discriminator("create_or_update_counter");
    assert_eq!(disc.len(), 8);
    assert_ne!(disc, [0u8; 8]);
}

#[test]
fn test_counter_pda_derivation() {
    let owner = Pubkey::new_unique();
    let (pda, _bump) =
        Pubkey::find_program_address(&[b"counter", owner.as_ref()], &program_id());
    assert_ne!(pda, Pubkey::default());
}
