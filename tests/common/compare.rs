//! Behavior Comparison Framework for Seahorse vs Anchor Programs
//!
//! This module provides utilities to compare the behavior of Seahorse-compiled
//! programs against their reference Anchor implementations. It helps verify
//! that Seahorse produces functionally equivalent output.
//!
//! # Usage
//!
//! ```rust,ignore
//! use seahorse_test_common::compare::*;
//!
//! // Create a comparison context
//! let mut ctx = CompareContext::new(
//!     &seahorse_keypair,
//!     "target/deploy/counter_seahorse.so",
//!     &anchor_keypair,
//!     "target/deploy/counter_anchor.so",
//! );
//!
//! // Build instruction for both programs
//! let instruction = InstructionBuilder::new("initialize")
//!     .with_signer(&user.pubkey())
//!     .with_writable(&counter_pda)
//!     .with_readonly(&system_program::id())
//!     .build();
//!
//! // Execute and compare
//! let result = ctx.execute_and_compare(instruction, &user, &[&user])?;
//! assert!(result.is_equivalent());
//! ```

use colored::Colorize;
use litesvm::LiteSVM;
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use solana_sha256_hasher::hash;
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::collections::HashMap;
use std::fmt;

/// Size of the Anchor account discriminator
pub const DISCRIMINATOR_SIZE: usize = 8;

/// Result of comparing Seahorse and Anchor program executions
#[derive(Debug)]
pub struct CompareResult {
    /// Whether the executions produced equivalent results
    pub equivalent: bool,
    /// Seahorse execution outcome
    pub seahorse_outcome: ExecutionOutcome,
    /// Anchor execution outcome
    pub anchor_outcome: ExecutionOutcome,
    /// Differences found between account states
    pub account_diffs: Vec<AccountDiff>,
    /// Summary of the comparison
    pub summary: String,
}

impl CompareResult {
    /// Check if the programs behaved equivalently
    pub fn is_equivalent(&self) -> bool {
        self.equivalent
    }

    /// Get a human-readable report of differences
    pub fn report(&self) -> String {
        let mut report = String::new();

        report.push_str(&format!("\n{}\n", "═".repeat(60).cyan()));
        report.push_str(&format!("{}\n", "Behavior Comparison Report".cyan().bold()));
        report.push_str(&format!("{}\n\n", "═".repeat(60).cyan()));

        // Execution outcomes
        report.push_str(&format!("{}:\n", "Seahorse Execution".yellow()));
        report.push_str(&format!("  {}\n\n", self.seahorse_outcome));

        report.push_str(&format!("{}:\n", "Anchor Execution".yellow()));
        report.push_str(&format!("  {}\n\n", self.anchor_outcome));

        // Account diffs
        if !self.account_diffs.is_empty() {
            report.push_str(&format!("{}:\n", "Account Differences".yellow()));
            for diff in &self.account_diffs {
                report.push_str(&format!("{}\n", diff));
            }
            report.push('\n');
        }

        // Final verdict
        if self.equivalent {
            report.push_str(&format!("{}: {}\n", "Result".green().bold(), "EQUIVALENT ✓".green()));
        } else {
            report.push_str(&format!("{}: {}\n", "Result".red().bold(), "DIVERGENT ✗".red()));
            report.push_str(&format!("  {}\n", self.summary.red()));
        }

        report.push_str(&format!("{}\n", "═".repeat(60).cyan()));
        report
    }
}

/// Outcome of a single program execution
#[derive(Debug, Clone)]
pub enum ExecutionOutcome {
    /// Execution succeeded
    Success {
        /// Compute units consumed
        compute_units: u64,
        /// Logs emitted
        logs: Vec<String>,
    },
    /// Execution failed with an error
    Failure {
        /// Error message
        error: String,
        /// Logs emitted before failure
        logs: Vec<String>,
    },
}

impl fmt::Display for ExecutionOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionOutcome::Success { compute_units, logs } => {
                write!(f, "Success (CU: {})", compute_units)?;
                if !logs.is_empty() {
                    write!(f, "\n    Logs: {:?}", logs)?;
                }
                Ok(())
            }
            ExecutionOutcome::Failure { error, logs } => {
                write!(f, "Failure: {}", error)?;
                if !logs.is_empty() {
                    write!(f, "\n    Logs: {:?}", logs)?;
                }
                Ok(())
            }
        }
    }
}

/// Difference found in an account between Seahorse and Anchor executions
#[derive(Debug)]
pub struct AccountDiff {
    /// Public key of the account
    pub pubkey: Pubkey,
    /// Type of difference
    pub diff_type: DiffType,
}

impl fmt::Display for AccountDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.diff_type {
            DiffType::LamportsDiff { seahorse, anchor } => {
                write!(
                    f,
                    "  {} [{}]: lamports differ - Seahorse: {}, Anchor: {}",
                    "LAMPORTS".red(),
                    truncate_pubkey(&self.pubkey),
                    seahorse,
                    anchor
                )
            }
            DiffType::DataDiff { seahorse, anchor, byte_positions } => {
                write!(
                    f,
                    "  {} [{}]: data differs at {} position(s)",
                    "DATA".red(),
                    truncate_pubkey(&self.pubkey),
                    byte_positions.len()
                )?;
                // Show first few differences
                for (i, pos) in byte_positions.iter().take(5).enumerate() {
                    let s_byte = seahorse.get(*pos).map(|b| format!("{:02x}", b)).unwrap_or_else(|| "??".to_string());
                    let a_byte = anchor.get(*pos).map(|b| format!("{:02x}", b)).unwrap_or_else(|| "??".to_string());
                    write!(f, "\n      byte[{}]: Seahorse=0x{}, Anchor=0x{}", pos, s_byte, a_byte)?;
                    if i == 4 && byte_positions.len() > 5 {
                        write!(f, "\n      ... and {} more", byte_positions.len() - 5)?;
                    }
                }
                Ok(())
            }
            DiffType::DataSizeDiff { seahorse, anchor } => {
                write!(
                    f,
                    "  {} [{}]: data size differs - Seahorse: {} bytes, Anchor: {} bytes",
                    "SIZE".red(),
                    truncate_pubkey(&self.pubkey),
                    seahorse,
                    anchor
                )
            }
            DiffType::OwnerDiff { seahorse, anchor } => {
                write!(
                    f,
                    "  {} [{}]: owner differs - Seahorse: {}, Anchor: {}",
                    "OWNER".red(),
                    truncate_pubkey(&self.pubkey),
                    truncate_pubkey(seahorse),
                    truncate_pubkey(anchor)
                )
            }
            DiffType::ExistsDiff { in_seahorse } => {
                if *in_seahorse {
                    write!(
                        f,
                        "  {} [{}]: account exists in Seahorse but not Anchor",
                        "EXISTS".red(),
                        truncate_pubkey(&self.pubkey)
                    )
                } else {
                    write!(
                        f,
                        "  {} [{}]: account exists in Anchor but not Seahorse",
                        "EXISTS".red(),
                        truncate_pubkey(&self.pubkey)
                    )
                }
            }
        }
    }
}

/// Type of difference between account states
#[derive(Debug)]
pub enum DiffType {
    /// Lamport balances differ
    LamportsDiff { seahorse: u64, anchor: u64 },
    /// Account data differs
    DataDiff {
        seahorse: Vec<u8>,
        anchor: Vec<u8>,
        byte_positions: Vec<usize>,
    },
    /// Account data sizes differ
    DataSizeDiff { seahorse: usize, anchor: usize },
    /// Account owners differ
    OwnerDiff { seahorse: Pubkey, anchor: Pubkey },
    /// Account exists in one but not the other
    ExistsDiff { in_seahorse: bool },
}

/// Context for comparing Seahorse and Anchor program executions
pub struct CompareContext {
    /// LiteSVM instance for Seahorse program
    seahorse_svm: LiteSVM,
    /// LiteSVM instance for Anchor program
    anchor_svm: LiteSVM,
    /// Seahorse program ID
    seahorse_program_id: Pubkey,
    /// Anchor program ID
    anchor_program_id: Pubkey,
    /// Accounts to track for comparison (pubkey -> label)
    tracked_accounts: HashMap<Pubkey, String>,
}

impl CompareContext {
    /// Create a new comparison context
    ///
    /// # Arguments
    /// * `seahorse_program_keypair` - Keypair for the Seahorse program
    /// * `seahorse_program_path` - Path to the Seahorse program .so file
    /// * `anchor_program_keypair` - Keypair for the Anchor program
    /// * `anchor_program_path` - Path to the Anchor program .so file
    pub fn new(
        seahorse_program_keypair: &Keypair,
        seahorse_program_path: &str,
        anchor_program_keypair: &Keypair,
        anchor_program_path: &str,
    ) -> Self {
        let mut seahorse_svm = LiteSVM::new();
        let mut anchor_svm = LiteSVM::new();

        // Load programs
        let seahorse_bytes = std::fs::read(seahorse_program_path)
            .unwrap_or_else(|_| panic!("Failed to read Seahorse program at {}", seahorse_program_path));
        let anchor_bytes = std::fs::read(anchor_program_path)
            .unwrap_or_else(|_| panic!("Failed to read Anchor program at {}", anchor_program_path));

        seahorse_svm.add_program(seahorse_program_keypair.pubkey(), &seahorse_bytes);
        anchor_svm.add_program(anchor_program_keypair.pubkey(), &anchor_bytes);

        Self {
            seahorse_svm,
            anchor_svm,
            seahorse_program_id: seahorse_program_keypair.pubkey(),
            anchor_program_id: anchor_program_keypair.pubkey(),
            tracked_accounts: HashMap::new(),
        }
    }

    /// Create a context from program bytes (useful when programs are already loaded)
    pub fn from_bytes(
        seahorse_program_id: Pubkey,
        seahorse_bytes: &[u8],
        anchor_program_id: Pubkey,
        anchor_bytes: &[u8],
    ) -> Self {
        let mut seahorse_svm = LiteSVM::new();
        let mut anchor_svm = LiteSVM::new();

        seahorse_svm.add_program(seahorse_program_id, seahorse_bytes);
        anchor_svm.add_program(anchor_program_id, anchor_bytes);

        Self {
            seahorse_svm,
            anchor_svm,
            seahorse_program_id,
            anchor_program_id,
            tracked_accounts: HashMap::new(),
        }
    }

    /// Get the Seahorse program ID
    pub fn seahorse_program_id(&self) -> Pubkey {
        self.seahorse_program_id
    }

    /// Get the Anchor program ID
    pub fn anchor_program_id(&self) -> Pubkey {
        self.anchor_program_id
    }

    /// Track an account for state comparison
    ///
    /// # Arguments
    /// * `pubkey` - Account to track
    /// * `label` - Human-readable label for the account
    pub fn track_account(&mut self, pubkey: Pubkey, label: &str) {
        self.tracked_accounts.insert(pubkey, label.to_string());
    }

    /// Fund an account in both SVMs
    pub fn fund_account(&mut self, pubkey: &Pubkey, lamports: u64) {
        self.seahorse_svm.airdrop(pubkey, lamports).unwrap();
        self.anchor_svm.airdrop(pubkey, lamports).unwrap();
    }

    /// Create a funded keypair in both SVMs
    pub fn funded_keypair(&mut self, lamports: u64) -> Keypair {
        let keypair = Keypair::new();
        self.fund_account(&keypair.pubkey(), lamports);
        keypair
    }

    /// Create a funded keypair with 10 SOL in both SVMs
    pub fn funded_keypair_10_sol(&mut self) -> Keypair {
        self.funded_keypair(10 * LAMPORTS_PER_SOL)
    }

    /// Set an account in both SVMs (useful for pre-initializing state)
    pub fn set_account(&mut self, pubkey: &Pubkey, account: Account) {
        self.seahorse_svm.set_account(*pubkey, account.clone()).unwrap();
        self.anchor_svm.set_account(*pubkey, account).unwrap();
    }

    /// Execute the same instruction on both programs and compare results
    ///
    /// # Arguments
    /// * `builder` - Instruction builder with accounts and data
    /// * `payer` - Transaction fee payer
    /// * `signers` - All transaction signers
    ///
    /// # Returns
    /// A `CompareResult` containing the comparison outcome
    pub fn execute_and_compare(
        &mut self,
        builder: &InstructionBuilder,
        payer: &Keypair,
        signers: &[&Keypair],
    ) -> CompareResult {
        // Build instructions for each program
        let seahorse_ix = builder.build(self.seahorse_program_id);
        let anchor_ix = builder.build(self.anchor_program_id);

        // Execute on Seahorse
        let seahorse_outcome = execute_instruction_on_svm(&mut self.seahorse_svm, seahorse_ix, payer, signers);

        // Execute on Anchor
        let anchor_outcome = execute_instruction_on_svm(&mut self.anchor_svm, anchor_ix, payer, signers);

        // Compare outcomes and account states
        self.compare_results(seahorse_outcome, anchor_outcome)
    }

    /// Execute a sequence of instructions and compare results after each
    ///
    /// # Arguments
    /// * `instructions` - Vector of (instruction_builder, payer, signers) tuples
    ///
    /// # Returns
    /// A vector of `CompareResult` for each instruction
    pub fn execute_sequence_and_compare(
        &mut self,
        instructions: Vec<(&InstructionBuilder, &Keypair, Vec<&Keypair>)>,
    ) -> Vec<CompareResult> {
        let mut results = Vec::new();

        for (builder, payer, signers) in instructions {
            let result = self.execute_and_compare(builder, payer, &signers);
            let should_continue = result.is_equivalent();
            results.push(result);

            // Stop if programs diverged
            if !should_continue {
                break;
            }
        }

        results
    }

    fn compare_results(
        &self,
        seahorse_outcome: ExecutionOutcome,
        anchor_outcome: ExecutionOutcome,
    ) -> CompareResult {
        let mut account_diffs = Vec::new();
        let mut equivalent = true;
        let mut summary = String::new();

        // Check if both succeeded or both failed
        match (&seahorse_outcome, &anchor_outcome) {
            (ExecutionOutcome::Success { .. }, ExecutionOutcome::Success { .. }) => {
                // Both succeeded - compare account states
                account_diffs = self.compare_tracked_accounts();
                if !account_diffs.is_empty() {
                    equivalent = false;
                    summary = format!("Both executions succeeded but {} account(s) differ", account_diffs.len());
                }
            }
            (ExecutionOutcome::Failure { error: s_err, .. }, ExecutionOutcome::Failure { error: a_err, .. }) => {
                // Both failed - this is considered equivalent if errors are similar
                if s_err != a_err {
                    equivalent = false;
                    summary = format!(
                        "Both failed but with different errors:\n  Seahorse: {}\n  Anchor: {}",
                        s_err, a_err
                    );
                }
            }
            (ExecutionOutcome::Success { .. }, ExecutionOutcome::Failure { error, .. }) => {
                equivalent = false;
                summary = format!("Seahorse succeeded but Anchor failed: {}", error);
            }
            (ExecutionOutcome::Failure { error, .. }, ExecutionOutcome::Success { .. }) => {
                equivalent = false;
                summary = format!("Seahorse failed but Anchor succeeded: {}", error);
            }
        }

        CompareResult {
            equivalent,
            seahorse_outcome,
            anchor_outcome,
            account_diffs,
            summary,
        }
    }

    fn compare_tracked_accounts(&self) -> Vec<AccountDiff> {
        let mut diffs = Vec::new();

        for pubkey in self.tracked_accounts.keys() {
            let seahorse_account = self.seahorse_svm.get_account(pubkey);
            let anchor_account = self.anchor_svm.get_account(pubkey);

            match (seahorse_account, anchor_account) {
                (Some(s), Some(a)) => {
                    if let Some(diff) = compare_accounts(*pubkey, &s, &a) {
                        diffs.push(diff);
                    }
                }
                (Some(_), None) => {
                    diffs.push(AccountDiff {
                        pubkey: *pubkey,
                        diff_type: DiffType::ExistsDiff { in_seahorse: true },
                    });
                }
                (None, Some(_)) => {
                    diffs.push(AccountDiff {
                        pubkey: *pubkey,
                        diff_type: DiffType::ExistsDiff { in_seahorse: false },
                    });
                }
                (None, None) => {
                    // Both don't exist - equivalent
                }
            }
        }

        diffs
    }

    /// Get the Seahorse SVM for direct manipulation
    pub fn seahorse_svm(&mut self) -> &mut LiteSVM {
        &mut self.seahorse_svm
    }

    /// Get the Anchor SVM for direct manipulation
    pub fn anchor_svm(&mut self) -> &mut LiteSVM {
        &mut self.anchor_svm
    }

    /// Warp both SVMs to the given slot
    pub fn warp_to_slot(&mut self, slot: u64) {
        self.seahorse_svm.warp_to_slot(slot);
        self.anchor_svm.warp_to_slot(slot);
    }
}

/// Builder for creating instructions that can be executed on both programs
#[derive(Clone, Debug)]
pub struct InstructionBuilder {
    instruction_name: String,
    accounts: Vec<AccountMetaTemplate>,
    data: Vec<u8>,
}

/// Account meta that can be templated for different program IDs
#[derive(Clone, Debug)]
enum AccountMetaTemplate {
    /// Literal pubkey
    Fixed(AccountMeta),
    /// Use the program ID being executed
    ProgramId { is_signer: bool, is_writable: bool },
}

impl InstructionBuilder {
    /// Create a new instruction builder
    ///
    /// # Arguments
    /// * `instruction_name` - Name of the instruction (snake_case)
    pub fn new(instruction_name: &str) -> Self {
        Self {
            instruction_name: instruction_name.to_string(),
            accounts: Vec::new(),
            data: Vec::new(),
        }
    }

    /// Add a writable signer account
    pub fn with_signer(mut self, pubkey: &Pubkey) -> Self {
        self.accounts.push(AccountMetaTemplate::Fixed(AccountMeta::new(*pubkey, true)));
        self
    }

    /// Add a writable non-signer account
    pub fn with_writable(mut self, pubkey: &Pubkey) -> Self {
        self.accounts.push(AccountMetaTemplate::Fixed(AccountMeta::new(*pubkey, false)));
        self
    }

    /// Add a readonly non-signer account
    pub fn with_readonly(mut self, pubkey: &Pubkey) -> Self {
        self.accounts.push(AccountMetaTemplate::Fixed(AccountMeta::new_readonly(*pubkey, false)));
        self
    }

    /// Add a readonly signer account
    pub fn with_readonly_signer(mut self, pubkey: &Pubkey) -> Self {
        self.accounts.push(AccountMetaTemplate::Fixed(AccountMeta::new_readonly(*pubkey, true)));
        self
    }

    /// Add the program ID as a readonly account (for CPI)
    pub fn with_program_id(mut self) -> Self {
        self.accounts.push(AccountMetaTemplate::ProgramId {
            is_signer: false,
            is_writable: false,
        });
        self
    }

    /// Set the instruction data (Borsh-serialized arguments without discriminator)
    pub fn with_data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    /// Build the instruction for a specific program
    pub fn build(&self, program_id: Pubkey) -> Instruction {
        let discriminator = instruction_discriminator(&self.instruction_name);

        let mut instruction_data = Vec::with_capacity(8 + self.data.len());
        instruction_data.extend_from_slice(&discriminator);
        instruction_data.extend_from_slice(&self.data);

        let accounts: Vec<AccountMeta> = self
            .accounts
            .iter()
            .map(|template| match template {
                AccountMetaTemplate::Fixed(meta) => meta.clone(),
                AccountMetaTemplate::ProgramId { is_signer, is_writable } => {
                    if *is_writable {
                        AccountMeta::new(program_id, *is_signer)
                    } else {
                        AccountMeta::new_readonly(program_id, *is_signer)
                    }
                }
            })
            .collect();

        Instruction {
            program_id,
            accounts,
            data: instruction_data,
        }
    }
}

/// Execute an instruction on a LiteSVM instance and return the outcome
fn execute_instruction_on_svm(
    svm: &mut LiteSVM,
    instruction: Instruction,
    payer: &Keypair,
    signers: &[&Keypair],
) -> ExecutionOutcome {
    let blockhash = svm.latest_blockhash();
    let message = Message::new(&[instruction], Some(&payer.pubkey()));
    let tx = Transaction::new(signers, message, blockhash);

    match svm.send_transaction(tx) {
        Ok(meta) => ExecutionOutcome::Success {
            compute_units: meta.compute_units_consumed,
            logs: meta.logs.clone(),
        },
        Err(failed) => ExecutionOutcome::Failure {
            error: format!("{:?}", failed.err),
            logs: failed.meta.logs.clone(),
        },
    }
}

/// Compare two accounts and return the diff if they differ
pub fn compare_accounts(pubkey: Pubkey, seahorse: &Account, anchor: &Account) -> Option<AccountDiff> {
    // Check lamports
    if seahorse.lamports != anchor.lamports {
        return Some(AccountDiff {
            pubkey,
            diff_type: DiffType::LamportsDiff {
                seahorse: seahorse.lamports,
                anchor: anchor.lamports,
            },
        });
    }

    // Check owner
    if seahorse.owner != anchor.owner {
        return Some(AccountDiff {
            pubkey,
            diff_type: DiffType::OwnerDiff {
                seahorse: seahorse.owner,
                anchor: anchor.owner,
            },
        });
    }

    // Check data size
    if seahorse.data.len() != anchor.data.len() {
        return Some(AccountDiff {
            pubkey,
            diff_type: DiffType::DataSizeDiff {
                seahorse: seahorse.data.len(),
                anchor: anchor.data.len(),
            },
        });
    }

    // Check data content (skip discriminator for more meaningful comparison)
    let byte_positions: Vec<usize> = seahorse
        .data
        .iter()
        .zip(anchor.data.iter())
        .enumerate()
        .filter(|(_, (s, a))| s != a)
        .map(|(i, _)| i)
        .collect();

    if !byte_positions.is_empty() {
        return Some(AccountDiff {
            pubkey,
            diff_type: DiffType::DataDiff {
                seahorse: seahorse.data.clone(),
                anchor: anchor.data.clone(),
                byte_positions,
            },
        });
    }

    None
}

/// Calculate the instruction discriminator for an Anchor instruction
pub fn instruction_discriminator(name: &str) -> [u8; 8] {
    let preimage = format!("global:{}", name);
    let hash = hash(preimage.as_bytes());
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash.to_bytes()[..8]);
    discriminator
}

/// Calculate the account discriminator for an Anchor account
pub fn account_discriminator(account_name: &str) -> [u8; 8] {
    let preimage = format!("account:{}", account_name);
    let hash = hash(preimage.as_bytes());
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash.to_bytes()[..8]);
    discriminator
}

/// Create a rent-exempt account for storing program data
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

/// Derive a PDA address and bump
pub fn find_pda(seeds: &[&[u8]], program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(seeds, program_id)
}

/// Truncate a pubkey for display
fn truncate_pubkey(pubkey: &Pubkey) -> String {
    let s = pubkey.to_string();
    if s.len() > 12 {
        format!("{}...{}", &s[..6], &s[s.len()-4..])
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_discriminator() {
        let disc = instruction_discriminator("initialize");
        assert_eq!(disc.len(), 8);
        assert_eq!(disc, instruction_discriminator("initialize"));
        assert_ne!(disc, instruction_discriminator("increment"));
    }

    #[test]
    fn test_account_discriminator() {
        let disc = account_discriminator("Counter");
        assert_eq!(disc.len(), 8);
        assert_eq!(disc, account_discriminator("Counter"));
        assert_ne!(disc, account_discriminator("Calculator"));
    }

    #[test]
    fn test_instruction_builder() {
        let program_id = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let account = Pubkey::new_unique();

        let builder = InstructionBuilder::new("initialize")
            .with_signer(&user)
            .with_writable(&account)
            .with_data(vec![1, 2, 3, 4]);

        let ix = builder.build(program_id);

        // Should have discriminator + data
        assert_eq!(ix.data.len(), 8 + 4);
        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 2);
        assert!(ix.accounts[0].is_signer);
        assert!(!ix.accounts[1].is_signer);
    }

    #[test]
    fn test_compare_accounts_equal() {
        let pubkey = Pubkey::new_unique();
        let account = Account {
            lamports: 1000,
            data: vec![1, 2, 3, 4],
            owner: Pubkey::new_unique(),
            executable: false,
            rent_epoch: 0,
        };

        let result = compare_accounts(pubkey, &account, &account.clone());
        assert!(result.is_none());
    }

    #[test]
    fn test_compare_accounts_lamports_diff() {
        let pubkey = Pubkey::new_unique();
        let owner = Pubkey::new_unique();

        let account1 = Account {
            lamports: 1000,
            data: vec![1, 2, 3],
            owner,
            executable: false,
            rent_epoch: 0,
        };
        let account2 = Account {
            lamports: 2000,
            data: vec![1, 2, 3],
            owner,
            executable: false,
            rent_epoch: 0,
        };

        let result = compare_accounts(pubkey, &account1, &account2);
        assert!(result.is_some());
        assert!(matches!(result.unwrap().diff_type, DiffType::LamportsDiff { .. }));
    }

    #[test]
    fn test_compare_accounts_data_diff() {
        let pubkey = Pubkey::new_unique();
        let owner = Pubkey::new_unique();

        let account1 = Account {
            lamports: 1000,
            data: vec![1, 2, 3],
            owner,
            executable: false,
            rent_epoch: 0,
        };
        let account2 = Account {
            lamports: 1000,
            data: vec![1, 5, 3],
            owner,
            executable: false,
            rent_epoch: 0,
        };

        let result = compare_accounts(pubkey, &account1, &account2);
        assert!(result.is_some());
        match result.unwrap().diff_type {
            DiffType::DataDiff { byte_positions, .. } => {
                assert_eq!(byte_positions, vec![1]);
            }
            _ => panic!("Expected DataDiff"),
        }
    }

    #[test]
    fn test_compare_result_equivalent() {
        let result = CompareResult {
            equivalent: true,
            seahorse_outcome: ExecutionOutcome::Success {
                compute_units: 1000,
                logs: vec![],
            },
            anchor_outcome: ExecutionOutcome::Success {
                compute_units: 1000,
                logs: vec![],
            },
            account_diffs: vec![],
            summary: String::new(),
        };

        assert!(result.is_equivalent());
        let report = result.report();
        assert!(report.contains("EQUIVALENT"));
    }

    #[test]
    fn test_truncate_pubkey() {
        let pubkey = Pubkey::new_unique();
        let truncated = truncate_pubkey(&pubkey);
        assert!(truncated.len() <= 13); // 6 + ... + 4
    }
}
