//! LiteSVM-specific test helpers for Seahorse programs
//!
//! These helpers make it easy to set up LiteSVM integration tests for
//! Seahorse-compiled Anchor programs.

use litesvm::LiteSVM;
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use solana_sdk_ids::system_program;
use solana_signer::Signer;
use solana_transaction::Transaction;

use crate::instruction_discriminator;

/// Path to compiled Seahorse program binaries
pub const PROGRAM_DEPLOY_DIR: &str = "target/deploy";

/// Create a LiteSVM instance and deploy a program from .so file
///
/// # Arguments
/// * `program_id` - The program's keypair
/// * `program_path` - Path to the .so file
///
/// # Returns
/// A configured LiteSVM instance with the program deployed
pub fn litesvm_with_program(program_keypair: &Keypair, program_path: &str) -> LiteSVM {
    let mut svm = LiteSVM::new();
    let program_bytes = std::fs::read(program_path)
        .unwrap_or_else(|_| panic!("Failed to read program at {}", program_path));
    svm.add_program(program_keypair.pubkey(), &program_bytes);
    svm
}

/// Create a funded keypair in the SVM
///
/// # Arguments
/// * `svm` - The LiteSVM instance
/// * `lamports` - Amount of lamports to fund
///
/// # Returns
/// A keypair with the specified lamports
pub fn funded_keypair(svm: &mut LiteSVM, lamports: u64) -> Keypair {
    let keypair = Keypair::new();
    svm.airdrop(&keypair.pubkey(), lamports).unwrap();
    keypair
}

/// Create a funded keypair with 10 SOL
pub fn funded_keypair_10_sol(svm: &mut LiteSVM) -> Keypair {
    funded_keypair(svm, 10 * LAMPORTS_PER_SOL)
}

/// Create an Anchor instruction with the given discriminator and data
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

/// Execute a transaction with the given instruction and signers
///
/// # Arguments
/// * `svm` - The LiteSVM instance
/// * `instruction` - The instruction to execute
/// * `payer` - The transaction fee payer
/// * `signers` - All signers for the transaction
///
/// # Returns
/// Transaction result (Ok = success with metadata, Err = failure with metadata)
pub fn execute_tx(
    svm: &mut LiteSVM,
    instruction: Instruction,
    payer: &Keypair,
    signers: &[&Keypair],
) -> litesvm::types::TransactionResult {
    let blockhash = svm.latest_blockhash();
    let message = Message::new(&[instruction], Some(&payer.pubkey()));
    let tx = Transaction::new(signers, message, blockhash);
    svm.send_transaction(tx)
}

/// Execute multiple instructions in a single transaction
///
/// # Arguments
/// * `svm` - The LiteSVM instance
/// * `instructions` - The instructions to execute
/// * `payer` - The transaction fee payer
/// * `signers` - All signers for the transaction
///
/// # Returns
/// Transaction result (Ok = success with metadata, Err = failure with metadata)
pub fn execute_tx_multi(
    svm: &mut LiteSVM,
    instructions: &[Instruction],
    payer: &Keypair,
    signers: &[&Keypair],
) -> litesvm::types::TransactionResult {
    let blockhash = svm.latest_blockhash();
    let message = Message::new(instructions, Some(&payer.pubkey()));
    let tx = Transaction::new(signers, message, blockhash);
    svm.send_transaction(tx)
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

/// Get an account from the SVM
pub fn get_account(svm: &LiteSVM, pubkey: &Pubkey) -> Option<Account> {
    svm.get_account(pubkey)
}

/// Get account data from the SVM
pub fn get_account_data(svm: &LiteSVM, pubkey: &Pubkey) -> Option<Vec<u8>> {
    svm.get_account(pubkey).map(|a| a.data)
}

/// Calculate rent-exempt lamports for a given data size
pub fn rent_exempt_lamports(data_size: usize) -> u64 {
    Rent::default().minimum_balance(data_size)
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

/// Warp the SVM clock forward by the given number of slots
///
/// Useful for testing time-dependent behavior like vesting or auctions
pub fn warp_to_slot(svm: &mut LiteSVM, slot: u64) {
    svm.warp_to_slot(slot);
}

/// Standard account size for Seahorse Calculator account
/// Fields: owner (32 bytes) + display (8 bytes)
pub const CALCULATOR_SIZE: usize = 8 + 32 + 8; // discriminator + owner + display

/// Standard account size for a simple counter account
/// Fields: count (8 bytes)
pub const COUNTER_SIZE: usize = 8 + 8; // discriminator + count

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_funded_keypair() {
        let mut svm = LiteSVM::new();
        let keypair = funded_keypair(&mut svm, LAMPORTS_PER_SOL);

        let account = svm.get_account(&keypair.pubkey()).unwrap();
        assert_eq!(account.lamports, LAMPORTS_PER_SOL);
    }

    #[test]
    fn test_litesvm_basic() {
        let svm = LiteSVM::new();
        // Basic smoke test - SVM should be created
        let blockhash = svm.latest_blockhash();
        assert!(!blockhash.to_bytes().iter().all(|&b| b == 0));
    }
}
