//! Multisig Wallet Program - Anchor Reference
//!
//! Demonstrates M-of-N multisig wallet requiring multiple signatures to execute transactions.
//!
//! Features:
//! - create_multisig: Creates a multisig wallet with N owners and threshold M
//! - propose_transaction: Owner proposes a SOL transfer transaction
//! - approve: Owner approves a proposed transaction
//! - execute: Executes transaction once threshold is met
//!
//! Key patterns:
//! - M-of-N threshold enforcement
//! - Approval tracking via individual approval record PDAs (prevents double-approval)
//! - Transaction PDA with approval count
//! - SOL transfers from multisig vault
//!
//! Program ID matches Seahorse version for parity testing.

use anchor_lang::prelude::*;
use anchor_lang::system_program;

// Same program ID as Seahorse version for parity testing
declare_id!("5h7CmLSs5q2ZfWFns5yUjN6BTYVxVxTNddJ2X1yHnWCd");

// Maximum number of owners allowed
const MAX_OWNERS: u8 = 10;

#[program]
pub mod multisig_anchor {
    use super::*;

    /// Create a new multisig wallet
    pub fn create_multisig(
        ctx: Context<CreateMultisig>,
        multisig_id: u64,
        threshold: u8,
        owner_count: u8,
        owner_1: Pubkey,
        owner_2: Pubkey,
        owner_3: Pubkey,
    ) -> Result<()> {
        // Validate inputs
        require!(threshold > 0, MultisigError::InvalidThreshold);
        require!(owner_count >= threshold, MultisigError::InsufficientOwners);
        require!(owner_count <= MAX_OWNERS, MultisigError::TooManyOwners);
        require!(owner_count >= 1, MultisigError::NoOwners);
        require!(owner_count <= 3, MultisigError::TooManyOwnersForInstruction);

        let multisig = &mut ctx.accounts.multisig;

        multisig.multisig_id = multisig_id;
        multisig.threshold = threshold;
        multisig.owner_count = owner_count;
        multisig.next_tx_id = 1; // Start from 1

        // Store owners - use creator as placeholder for unused slots
        multisig.owner_1 = owner_1;
        multisig.owner_2 = owner_2;
        multisig.owner_3 = owner_3;
        // Initialize remaining slots with creator address as placeholder
        multisig.owner_4 = ctx.accounts.creator.key();
        multisig.owner_5 = ctx.accounts.creator.key();
        multisig.owner_6 = ctx.accounts.creator.key();
        multisig.owner_7 = ctx.accounts.creator.key();
        multisig.owner_8 = ctx.accounts.creator.key();
        multisig.owner_9 = ctx.accounts.creator.key();
        multisig.owner_10 = ctx.accounts.creator.key();
        multisig.bump = ctx.bumps.multisig;

        msg!(
            "Multisig created: id={}, threshold={}, owners={}",
            multisig_id,
            threshold,
            owner_count
        );

        Ok(())
    }

    /// Propose a new transaction
    pub fn propose_transaction(
        ctx: Context<ProposeTransaction>,
        tx_id: u64,
        recipient: Pubkey,
        amount: u64,
    ) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;

        // Validate proposer is an owner
        require!(
            is_owner(multisig, ctx.accounts.proposer.key()),
            MultisigError::NotOwner
        );

        // Validate tx_id matches expected next ID
        require!(tx_id == multisig.next_tx_id, MultisigError::InvalidTxId);
        multisig.next_tx_id += 1;

        // Validate amount
        require!(amount > 0, MultisigError::ZeroAmount);

        let transaction = &mut ctx.accounts.transaction;

        transaction.multisig = multisig.key();
        transaction.tx_id = tx_id;
        transaction.proposer = ctx.accounts.proposer.key();
        transaction.recipient = recipient;
        transaction.amount = amount;
        transaction.approval_count = 0; // No approvals yet
        transaction.is_executed = false;
        transaction.bump = ctx.bumps.transaction;

        msg!(
            "Transaction proposed: id={}, recipient={}, amount={}",
            tx_id,
            recipient,
            amount
        );

        Ok(())
    }

    /// Approve a proposed transaction
    pub fn approve(ctx: Context<Approve>, tx_id: u64) -> Result<()> {
        let multisig = &ctx.accounts.multisig;

        // Validate approver is an owner
        require!(
            is_owner(multisig, ctx.accounts.approver.key()),
            MultisigError::NotOwner
        );

        // Validate transaction belongs to this multisig
        require!(
            ctx.accounts.transaction.multisig == multisig.key(),
            MultisigError::WrongMultisig
        );

        // Validate tx_id matches
        require!(
            ctx.accounts.transaction.tx_id == tx_id,
            MultisigError::TxIdMismatch
        );

        // Validate not already executed
        require!(
            !ctx.accounts.transaction.is_executed,
            MultisigError::AlreadyExecuted
        );

        let clock = Clock::get()?;

        // Initialize approval record (will fail if already exists - double approval prevention)
        let approval = &mut ctx.accounts.approval;
        approval.approver = ctx.accounts.approver.key();
        approval.transaction = ctx.accounts.transaction.key();
        approval.approved_at_slot = clock.slot;
        approval.bump = ctx.bumps.approval;

        // Increment approval count
        let transaction = &mut ctx.accounts.transaction;
        transaction.approval_count += 1;

        msg!(
            "Transaction {} approved by {}, count={}/{}",
            tx_id,
            ctx.accounts.approver.key(),
            transaction.approval_count,
            multisig.threshold
        );

        Ok(())
    }

    /// Execute a transaction once threshold is met
    pub fn execute(ctx: Context<Execute>, tx_id: u64) -> Result<()> {
        let multisig = &ctx.accounts.multisig;

        // Validate executor is an owner
        require!(
            is_owner(multisig, ctx.accounts.executor.key()),
            MultisigError::NotOwner
        );

        // Validate transaction belongs to this multisig
        require!(
            ctx.accounts.transaction.multisig == multisig.key(),
            MultisigError::WrongMultisig
        );

        // Validate tx_id matches
        require!(
            ctx.accounts.transaction.tx_id == tx_id,
            MultisigError::TxIdMismatch
        );

        // Validate not already executed
        require!(
            !ctx.accounts.transaction.is_executed,
            MultisigError::AlreadyExecuted
        );

        // Validate recipient matches
        require!(
            ctx.accounts.transaction.recipient == ctx.accounts.recipient.key(),
            MultisigError::RecipientMismatch
        );

        // Validate threshold is met
        require!(
            ctx.accounts.transaction.approval_count >= multisig.threshold,
            MultisigError::ThresholdNotMet
        );

        // Mark as executed
        let transaction = &mut ctx.accounts.transaction;
        transaction.is_executed = true;

        // Transfer lamports from multisig PDA to recipient
        let amount = transaction.amount;

        // Use direct lamport manipulation (same as Seahorse approach)
        **ctx
            .accounts
            .multisig
            .to_account_info()
            .try_borrow_mut_lamports()? -= amount;

        **ctx
            .accounts
            .recipient
            .to_account_info()
            .try_borrow_mut_lamports()? += amount;

        msg!(
            "Transaction {} executed: {} lamports to {}",
            tx_id,
            amount,
            ctx.accounts.recipient.key()
        );

        Ok(())
    }

    /// Get multisig info (read-only, for logging)
    pub fn get_multisig_info(ctx: Context<GetMultisigInfo>) -> Result<()> {
        let multisig = &ctx.accounts.multisig;

        msg!("Multisig {}:", multisig.multisig_id);
        msg!("Threshold: {}", multisig.threshold);
        msg!("Owner count: {}", multisig.owner_count);
        msg!("Next tx ID: {}", multisig.next_tx_id);

        Ok(())
    }

    /// Get transaction info (read-only, for logging)
    pub fn get_transaction_info(ctx: Context<GetTransactionInfo>) -> Result<()> {
        let transaction = &ctx.accounts.transaction;

        msg!("Transaction {}:", transaction.tx_id);
        msg!("Recipient: {}", transaction.recipient);
        msg!("Amount: {}", transaction.amount);
        msg!("Approvals: {}", transaction.approval_count);
        msg!("Executed: {}", transaction.is_executed);

        Ok(())
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Check if an address is one of the multisig owners
fn is_owner(multisig: &Multisig, addr: Pubkey) -> bool {
    if addr == multisig.owner_1 {
        return true;
    }
    if addr == multisig.owner_2 {
        return true;
    }
    if addr == multisig.owner_3 {
        return true;
    }
    if addr == multisig.owner_4 {
        return true;
    }
    if addr == multisig.owner_5 {
        return true;
    }
    if addr == multisig.owner_6 {
        return true;
    }
    if addr == multisig.owner_7 {
        return true;
    }
    if addr == multisig.owner_8 {
        return true;
    }
    if addr == multisig.owner_9 {
        return true;
    }
    if addr == multisig.owner_10 {
        return true;
    }
    false
}

// =============================================================================
// ACCOUNT STRUCTURES
// =============================================================================

#[account]
pub struct Multisig {
    /// Unique multisig identifier
    pub multisig_id: u64,
    /// Number of required approvals (threshold M)
    pub threshold: u8,
    /// Number of owners
    pub owner_count: u8,
    /// Owner addresses (stored as array of pubkeys)
    pub owner_1: Pubkey,
    pub owner_2: Pubkey,
    pub owner_3: Pubkey,
    pub owner_4: Pubkey,
    pub owner_5: Pubkey,
    pub owner_6: Pubkey,
    pub owner_7: Pubkey,
    pub owner_8: Pubkey,
    pub owner_9: Pubkey,
    pub owner_10: Pubkey,
    /// Next transaction ID (auto-increment)
    pub next_tx_id: u64,
    /// PDA bump seed
    pub bump: u8,
}

#[account]
pub struct Transaction {
    /// Multisig this transaction belongs to
    pub multisig: Pubkey,
    /// Transaction ID
    pub tx_id: u64,
    /// Proposer of the transaction
    pub proposer: Pubkey,
    /// Recipient of the transfer
    pub recipient: Pubkey,
    /// Amount of lamports to transfer
    pub amount: u64,
    /// Number of approvals received
    pub approval_count: u8,
    /// Whether the transaction has been executed
    pub is_executed: bool,
    /// PDA bump seed
    pub bump: u8,
}

#[account]
pub struct Approval {
    /// Approver who cast this approval
    pub approver: Pubkey,
    /// Transaction that was approved
    pub transaction: Pubkey,
    /// Slot when approval was given
    pub approved_at_slot: u64,
    /// PDA bump seed
    pub bump: u8,
}

// =============================================================================
// INSTRUCTION CONTEXTS
// =============================================================================

#[derive(Accounts)]
#[instruction(multisig_id: u64, threshold: u8, owner_count: u8, owner_1: Pubkey, owner_2: Pubkey, owner_3: Pubkey)]
pub struct CreateMultisig<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        init,
        payer = creator,
        space = 8 + std::mem::size_of::<Multisig>(),
        seeds = [b"multisig", multisig_id.to_le_bytes().as_ref()],
        bump
    )]
    pub multisig: Account<'info, Multisig>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(tx_id: u64, recipient: Pubkey, amount: u64)]
pub struct ProposeTransaction<'info> {
    #[account(mut)]
    pub proposer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"multisig", multisig.multisig_id.to_le_bytes().as_ref()],
        bump = multisig.bump
    )]
    pub multisig: Account<'info, Multisig>,

    #[account(
        init,
        payer = proposer,
        space = 8 + std::mem::size_of::<Transaction>(),
        seeds = [b"transaction", multisig.multisig_id.to_le_bytes().as_ref(), tx_id.to_le_bytes().as_ref()],
        bump
    )]
    pub transaction: Account<'info, Transaction>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(tx_id: u64)]
pub struct Approve<'info> {
    #[account(mut)]
    pub approver: Signer<'info>,

    #[account(
        mut,
        seeds = [b"multisig", multisig.multisig_id.to_le_bytes().as_ref()],
        bump = multisig.bump
    )]
    pub multisig: Account<'info, Multisig>,

    #[account(
        mut,
        seeds = [b"transaction", multisig.multisig_id.to_le_bytes().as_ref(), tx_id.to_le_bytes().as_ref()],
        bump = transaction.bump
    )]
    pub transaction: Account<'info, Transaction>,

    #[account(
        init,
        payer = approver,
        space = 8 + std::mem::size_of::<Approval>(),
        seeds = [b"approval", multisig.multisig_id.to_le_bytes().as_ref(), tx_id.to_le_bytes().as_ref(), approver.key().as_ref()],
        bump
    )]
    pub approval: Account<'info, Approval>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(tx_id: u64)]
pub struct Execute<'info> {
    #[account(mut)]
    pub executor: Signer<'info>,

    #[account(
        mut,
        seeds = [b"multisig", multisig.multisig_id.to_le_bytes().as_ref()],
        bump = multisig.bump
    )]
    pub multisig: Account<'info, Multisig>,

    #[account(
        mut,
        seeds = [b"transaction", multisig.multisig_id.to_le_bytes().as_ref(), tx_id.to_le_bytes().as_ref()],
        bump = transaction.bump
    )]
    pub transaction: Account<'info, Transaction>,

    /// CHECK: This is the recipient of the transfer, verified against transaction data
    #[account(mut)]
    pub recipient: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct GetMultisigInfo<'info> {
    #[account(
        seeds = [b"multisig", multisig.multisig_id.to_le_bytes().as_ref()],
        bump = multisig.bump
    )]
    pub multisig: Account<'info, Multisig>,
}

#[derive(Accounts)]
pub struct GetTransactionInfo<'info> {
    pub transaction: Account<'info, Transaction>,
}

// =============================================================================
// ERROR CODES
// =============================================================================

#[error_code]
pub enum MultisigError {
    #[msg("Threshold must be greater than zero")]
    InvalidThreshold,
    #[msg("Owner count must be at least threshold")]
    InsufficientOwners,
    #[msg("Too many owners")]
    TooManyOwners,
    #[msg("Must have at least one owner")]
    NoOwners,
    #[msg("This instruction supports up to 3 owners")]
    TooManyOwnersForInstruction,
    #[msg("Only owners can perform this action")]
    NotOwner,
    #[msg("Invalid transaction ID")]
    InvalidTxId,
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Transaction does not belong to this multisig")]
    WrongMultisig,
    #[msg("Transaction ID mismatch")]
    TxIdMismatch,
    #[msg("Transaction already executed")]
    AlreadyExecuted,
    #[msg("Recipient mismatch")]
    RecipientMismatch,
    #[msg("Threshold not met")]
    ThresholdNotMet,
}
