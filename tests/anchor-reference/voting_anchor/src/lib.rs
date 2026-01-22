//! Simple Voting Program - Anchor Reference
//!
//! Demonstrates proposal and vote mechanics:
//! - create_proposal: Creates a new proposal with voting period
//! - cast_vote: Records a vote (yes/no) using voter record PDA for double-vote prevention
//! - finalize_proposal: Finalizes proposal after voting period ends
//!
//! Key patterns:
//! - One vote per user enforced via voter record PDA
//! - Vote counting with yes_votes/no_votes
//! - Time-based voting period (slot-based)
//!
//! Program ID matches Seahorse version for parity testing.

use anchor_lang::prelude::*;

// Same program ID as Seahorse version for parity testing
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod voting_anchor {
    use super::*;

    /// Create a new proposal
    pub fn create_proposal(
        ctx: Context<CreateProposal>,
        proposal_id: u64,
        title: String,
        description: String,
        voting_duration_slots: u64,
    ) -> Result<()> {
        // Validate inputs
        require!(title.len() <= 64, VotingError::TitleTooLong);
        require!(description.len() <= 256, VotingError::DescriptionTooLong);
        require!(voting_duration_slots > 0, VotingError::InvalidDuration);

        let proposal = &mut ctx.accounts.proposal;
        let clock = Clock::get()?;

        proposal.creator = ctx.accounts.creator.key();
        proposal.proposal_id = proposal_id;
        proposal.title = title;
        proposal.description = description;
        proposal.yes_votes = 0;
        proposal.no_votes = 0;
        proposal.start_slot = clock.slot;
        proposal.end_slot = clock.slot + voting_duration_slots;
        proposal.is_finalized = false;
        proposal.bump = ctx.bumps.proposal;

        msg!("Proposal {} created by {}", proposal_id, ctx.accounts.creator.key());

        Ok(())
    }

    /// Cast a vote on a proposal
    pub fn cast_vote(ctx: Context<CastVote>, proposal_id: u64, vote_yes: bool) -> Result<()> {
        // Verify proposal_id matches
        require!(ctx.accounts.proposal.proposal_id == proposal_id, VotingError::ProposalIdMismatch);
        let proposal = &mut ctx.accounts.proposal;
        let clock = Clock::get()?;

        // Validate voting period
        require!(clock.slot < proposal.end_slot, VotingError::VotingEnded);
        require!(!proposal.is_finalized, VotingError::ProposalFinalized);

        // Initialize voter record (prevents double voting - will fail if already exists)
        let voter_record = &mut ctx.accounts.voter_record;
        voter_record.voter = ctx.accounts.voter.key();
        voter_record.proposal = proposal.key();
        voter_record.vote_yes = vote_yes;
        voter_record.voted_at_slot = clock.slot;
        voter_record.bump = ctx.bumps.voter_record;

        // Increment vote count
        if vote_yes {
            proposal.yes_votes += 1;
        } else {
            proposal.no_votes += 1;
        }

        msg!("Vote cast: voter={}, proposal={}, vote_yes={}",
             ctx.accounts.voter.key(), proposal.proposal_id, vote_yes);

        Ok(())
    }

    /// Finalize a proposal after voting period ends
    pub fn finalize_proposal(ctx: Context<FinalizeProposal>) -> Result<()> {
        let proposal = &mut ctx.accounts.proposal;
        let clock = Clock::get()?;

        // Can only finalize after voting period ends
        require!(clock.slot >= proposal.end_slot, VotingError::VotingNotEnded);
        require!(!proposal.is_finalized, VotingError::ProposalFinalized);

        proposal.is_finalized = true;

        let result = if proposal.yes_votes > proposal.no_votes {
            "PASSED"
        } else if proposal.no_votes > proposal.yes_votes {
            "REJECTED"
        } else {
            "TIE"
        };

        msg!("Proposal {} finalized: {} (yes: {}, no: {})",
             proposal.proposal_id, result, proposal.yes_votes, proposal.no_votes);

        Ok(())
    }

    /// Get proposal info (read-only, for logging)
    pub fn get_proposal_info(ctx: Context<GetProposalInfo>) -> Result<()> {
        let proposal = &ctx.accounts.proposal;

        msg!("Proposal {}: {}", proposal.proposal_id, proposal.title);
        msg!("Description: {}", proposal.description);
        msg!("Votes: yes={}, no={}", proposal.yes_votes, proposal.no_votes);
        msg!("Slots: start={}, end={}", proposal.start_slot, proposal.end_slot);
        msg!("Finalized: {}", proposal.is_finalized);

        Ok(())
    }
}

// =============================================================================
// ACCOUNT STRUCTURES
// =============================================================================

#[account]
pub struct Proposal {
    /// Creator of the proposal
    pub creator: Pubkey,
    /// Unique proposal identifier
    pub proposal_id: u64,
    /// Proposal title (max 64 chars)
    pub title: String,
    /// Proposal description (max 256 chars)
    pub description: String,
    /// Number of yes votes
    pub yes_votes: u64,
    /// Number of no votes
    pub no_votes: u64,
    /// Slot when voting starts
    pub start_slot: u64,
    /// Slot when voting ends
    pub end_slot: u64,
    /// Whether the proposal has been finalized
    pub is_finalized: bool,
    /// PDA bump seed
    pub bump: u8,
}

#[account]
pub struct VoterRecord {
    /// Voter who cast this vote
    pub voter: Pubkey,
    /// Proposal that was voted on
    pub proposal: Pubkey,
    /// Whether voted yes (true) or no (false)
    pub vote_yes: bool,
    /// Slot when vote was cast
    pub voted_at_slot: u64,
    /// PDA bump seed
    pub bump: u8,
}

// =============================================================================
// INSTRUCTION CONTEXTS
// =============================================================================

#[derive(Accounts)]
#[instruction(proposal_id: u64)]
pub struct CreateProposal<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        init,
        payer = creator,
        space = 8 + 32 + 8 + (4 + 64) + (4 + 256) + 8 + 8 + 8 + 8 + 1 + 1,
        seeds = [b"proposal", proposal_id.to_le_bytes().as_ref()],
        bump
    )]
    pub proposal: Account<'info, Proposal>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(proposal_id: u64, vote_yes: bool)]
pub struct CastVote<'info> {
    #[account(mut)]
    pub voter: Signer<'info>,

    #[account(
        mut,
        seeds = [b"proposal", proposal_id.to_le_bytes().as_ref()],
        bump = proposal.bump
    )]
    pub proposal: Account<'info, Proposal>,

    // Use proposal_id instead of proposal.key() to match Seahorse PDA derivation
    #[account(
        init,
        payer = voter,
        space = 8 + 32 + 32 + 1 + 8 + 1,
        seeds = [b"voter_record", proposal_id.to_le_bytes().as_ref(), voter.key().as_ref()],
        bump
    )]
    pub voter_record: Account<'info, VoterRecord>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FinalizeProposal<'info> {
    #[account(mut)]
    pub finalizer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"proposal", proposal.proposal_id.to_le_bytes().as_ref()],
        bump = proposal.bump
    )]
    pub proposal: Account<'info, Proposal>,
}

#[derive(Accounts)]
pub struct GetProposalInfo<'info> {
    pub proposal: Account<'info, Proposal>,
}

// =============================================================================
// ERROR CODES
// =============================================================================

#[error_code]
pub enum VotingError {
    #[msg("Title exceeds maximum length of 64 characters")]
    TitleTooLong,
    #[msg("Description exceeds maximum length of 256 characters")]
    DescriptionTooLong,
    #[msg("Voting duration must be greater than zero")]
    InvalidDuration,
    #[msg("Voting period has ended")]
    VotingEnded,
    #[msg("Voting period has not ended yet")]
    VotingNotEnded,
    #[msg("Proposal has already been finalized")]
    ProposalFinalized,
    #[msg("Proposal ID mismatch")]
    ProposalIdMismatch,
}
