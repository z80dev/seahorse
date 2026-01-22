//! Structural parity tests for close_account_cpi_test program
//!
//! These tests verify discriminators and instruction IDs for the Seahorse
//! close_account_cpi_test program. There is no Anchor reference program, so we
//! check structural invariants instead of behavioral parity.

use solana_pubkey::Pubkey;
use solana_sha256_hasher::hash;
use std::str::FromStr;

/// close_account_cpi_test program ID (Seahorse)
fn program_id() -> Pubkey {
    Pubkey::from_str("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS").unwrap()
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
fn test_program_id_not_default() {
    assert_ne!(program_id(), Pubkey::default());
}

#[test]
fn test_instruction_discriminator() {
    let disc = instruction_discriminator("close_token_account");
    assert_eq!(disc.len(), 8);
    assert_ne!(disc, [0u8; 8]);
}
