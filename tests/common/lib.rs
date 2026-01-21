//! Seahorse Test Common - Behavior Comparison Framework
//!
//! This crate provides utilities for comparing the behavior of Seahorse-compiled
//! programs against their reference Anchor implementations.
//!
//! # Overview
//!
//! The comparison framework helps verify that Seahorse produces functionally
//! equivalent output to hand-written Anchor programs. It does this by:
//!
//! 1. Running the same instructions against both program versions
//! 2. Comparing execution outcomes (success/failure)
//! 3. Diffing account state changes
//! 4. Generating clear reports when behavior diverges
//!
//! # Example
//!
//! ```rust,ignore
//! use seahorse_test_common::compare::*;
//!
//! // Set up programs
//! let seahorse_keypair = Keypair::new();
//! let anchor_keypair = Keypair::new();
//!
//! let mut ctx = CompareContext::new(
//!     &seahorse_keypair,
//!     "target/deploy/counter_seahorse.so",
//!     &anchor_keypair,
//!     "target/deploy/counter_anchor.so",
//! );
//!
//! // Fund accounts
//! let payer = ctx.funded_keypair_10_sol();
//!
//! // Track accounts for state comparison
//! let (counter_pda, _) = find_pda(&[b"counter"], &ctx.seahorse_program_id());
//! ctx.track_account(counter_pda, "counter");
//!
//! // Execute and compare
//! let init_ix = InstructionBuilder::new("initialize")
//!     .with_signer(&payer.pubkey())
//!     .with_writable(&counter_pda)
//!     .with_readonly(&system_program::id());
//!
//! let result = ctx.execute_and_compare(&init_ix, &payer, &[&payer]);
//! assert!(result.is_equivalent(), "{}", result.report());
//! ```
//!
//! # Module Structure
//!
//! - [`compare`] - Core comparison framework with `CompareContext` and `InstructionBuilder`

pub mod compare;

pub use compare::{
    account_discriminator, compare_accounts, find_pda, instruction_discriminator,
    program_account, AccountDiff, CompareContext, CompareResult, DiffType,
    ExecutionOutcome, InstructionBuilder, DISCRIMINATOR_SIZE,
};
