//! Mollusk-specific test helpers for Seahorse programs
//!
//! These helpers make it easy to set up Mollusk tests for Seahorse-compiled
//! Anchor programs.

#![allow(deprecated)] // solana_sdk::system_program deprecation

use mollusk_svm::Mollusk;
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    rent::Rent,
    system_program,
};

/// Path to compiled Seahorse program binaries
pub const PROGRAM_DEPLOY_DIR: &str = "target/deploy";

/// Create a Mollusk instance for a Seahorse program
///
/// # Arguments
/// * `program_id` - The program's public key
/// * `program_name` - Name of the program (used to find the .so file)
///
/// # Note
/// The program must be compiled to `target/deploy/<program_name>.so` first.
/// Use `anchor build` after `seahorse compile` to build the binary.
pub fn mollusk_for_program(program_id: &Pubkey, program_name: &str) -> Mollusk {
    Mollusk::new(program_id, program_name)
}

/// Create a signer account with sufficient lamports
pub fn signer_account(lamports: u64) -> Account {
    Account {
        lamports,
        data: vec![],
        owner: system_program::id(),
        executable: false,
        rent_epoch: 0,
    }
}

/// Create an Anchor instruction with the given discriminator and data
///
/// Anchor instructions start with an 8-byte discriminator derived from
/// `sighash("global:<instruction_name>")`.
///
/// # Arguments
/// * `program_id` - The program to call
/// * `instruction_name` - Name of the instruction (snake_case)
/// * `data` - Borsh-serialized instruction arguments
/// * `accounts` - Account metas for the instruction
pub fn anchor_instruction(
    program_id: Pubkey,
    instruction_name: &str,
    data: &[u8],
    accounts: Vec<AccountMeta>,
) -> Instruction {
    let discriminator = instruction_discriminator(instruction_name);
    let mut instruction_data = Vec::with_capacity(8 + data.len());
    instruction_data.extend_from_slice(&discriminator);
    instruction_data.extend_from_slice(data);

    Instruction {
        program_id,
        accounts,
        data: instruction_data,
    }
}

/// Calculate the instruction discriminator for an Anchor instruction
///
/// Anchor uses `sha256("global:<instruction_name>")[..8]` as discriminator
pub fn instruction_discriminator(name: &str) -> [u8; 8] {
    let preimage = format!("global:{}", name);
    let hash = solana_sdk::hash::hash(preimage.as_bytes());
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash.to_bytes()[..8]);
    discriminator
}

/// Create account meta for a writable signer
pub fn signer_meta(pubkey: Pubkey) -> AccountMeta {
    AccountMeta::new(pubkey, true)
}

/// Create account meta for a writable non-signer
pub fn writable_meta(pubkey: Pubkey) -> AccountMeta {
    AccountMeta::new(pubkey, false)
}

/// Create account meta for a read-only non-signer
pub fn readonly_meta(pubkey: Pubkey) -> AccountMeta {
    AccountMeta::new_readonly(pubkey, false)
}

/// Calculate rent-exempt lamports for a given data size
pub fn rent_exempt_lamports(data_size: usize) -> u64 {
    Rent::default().minimum_balance(data_size)
}

/// Standard account size for Seahorse Calculator account
/// Fields: owner (32 bytes) + display (8 bytes)
pub const CALCULATOR_SIZE: usize = 8 + 32 + 8; // discriminator + owner + display

/// Standard account size for the Counter account
/// Fields: count (8 bytes) + authority (32 bytes)
pub const COUNTER_SIZE: usize = 8 + 8 + 32; // discriminator + count + authority

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_discriminator() {
        // Verify discriminator calculation
        let disc = instruction_discriminator("initialize");
        assert_eq!(disc.len(), 8);

        // Same name should give same discriminator
        assert_eq!(disc, instruction_discriminator("initialize"));

        // Different names should give different discriminators
        assert_ne!(disc, instruction_discriminator("increment"));
    }

    #[test]
    fn test_anchor_instruction_format() {
        let program_id = Pubkey::new_unique();
        let user = Pubkey::new_unique();

        let ix = anchor_instruction(
            program_id,
            "initialize",
            &[],
            vec![signer_meta(user)],
        );

        // Instruction should have 8-byte discriminator
        assert_eq!(ix.data.len(), 8);
        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 1);
        assert!(ix.accounts[0].is_signer);
        assert!(ix.accounts[0].is_writable);
    }
}
