//! Token-Weighted Governance Program - Anchor Reference
//!
//! Demonstrates token-weighted governance where voting power is proportional
//! to tokens held at the time of voting.
//!
//! Features:
//! - initialize_governance: Creates governance config for a token
//! - create_proposal: Creates a new proposal with voting period and quorum
//! - cast_vote: Records a vote weighted by voter's token balance
//! - finalize_proposal: Finalizes proposal if quorum met and voting ended
//!
//! Key patterns:
//! - Voting power = token balance at time of vote
//! - Quorum requirement (minimum total votes needed)
//! - Double-vote prevention via voter record PDA
//! - Time-based voting period (slot-based)
//!
//! Program ID matches Seahorse version for parity testing.

use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, TokenAccount};

// Same program ID as Seahorse version for parity testing
declare_id!("GovToken11111111111111111111111111111111111");

// Result codes
pub const RESULT_PENDING: u8 = 0;
pub const RESULT_PASSED: u8 = 1;
pub const RESULT_REJECTED: u8 = 2;
pub const RESULT_QUORUM_NOT_MET: u8 = 3;

#[program]
pub mod governance_anchor {
    use super::*;

    /// Initialize governance configuration for a token
    pub fn initialize_governance(
        ctx: Context<InitializeGovernance>,
        proposal_threshold: u64,
        default_voting_duration: u64,
    ) -> Result<()> {
        require!(proposal_threshold > 0, GovernanceError::InvalidThreshold);
        require!(
            default_voting_duration > 0,
            GovernanceError::InvalidDuration
        );

        let config = &mut ctx.accounts.config;

        config.authority = ctx.accounts.authority.key();
        config.governance_mint = ctx.accounts.governance_mint.key();
        config.proposal_threshold = proposal_threshold;
        config.default_voting_duration = default_voting_duration;
        config.next_proposal_id = 1; // Start from 1
        config.bump = ctx.bumps.config;

        msg!(
            "Governance initialized with mint {}",
            ctx.accounts.governance_mint.key()
        );

        Ok(())
    }

    /// Update governance configuration
    pub fn update_governance_config(
        ctx: Context<UpdateGovernanceConfig>,
        proposal_threshold: u64,
        default_voting_duration: u64,
    ) -> Result<()> {
        require!(proposal_threshold > 0, GovernanceError::InvalidThreshold);
        require!(
            default_voting_duration > 0,
            GovernanceError::InvalidDuration
        );

        let config = &mut ctx.accounts.config;

        require!(
            ctx.accounts.authority.key() == config.authority,
            GovernanceError::Unauthorized
        );

        config.proposal_threshold = proposal_threshold;
        config.default_voting_duration = default_voting_duration;

        msg!("Governance config updated");

        Ok(())
    }

    /// Create a new proposal
    pub fn create_proposal(
        ctx: Context<CreateProposal>,
        proposal_id: u64,
        title: String,
        description: String,
        quorum: u64,
        voting_duration_slots: u64,
    ) -> Result<()> {
        // Validate inputs
        require!(title.len() <= 64, GovernanceError::TitleTooLong);
        require!(
            description.len() <= 256,
            GovernanceError::DescriptionTooLong
        );
        require!(quorum > 0, GovernanceError::InvalidQuorum);

        let config = &mut ctx.accounts.config;

        // Validate proposal_id matches expected
        require!(
            proposal_id == config.next_proposal_id,
            GovernanceError::InvalidProposalId
        );
        config.next_proposal_id += 1;

        // Check creator has enough tokens
        let creator_balance = ctx.accounts.creator_token_account.amount;
        require!(
            creator_balance >= config.proposal_threshold,
            GovernanceError::InsufficientTokens
        );

        // Verify token account is for governance token
        require!(
            ctx.accounts.creator_token_account.mint == config.governance_mint,
            GovernanceError::WrongTokenMint
        );

        // Use default voting duration if 0 is passed
        let actual_duration = if voting_duration_slots == 0 {
            config.default_voting_duration
        } else {
            voting_duration_slots
        };

        let clock = Clock::get()?;
        let proposal = &mut ctx.accounts.proposal;

        proposal.governance = config.key();
        proposal.creator = ctx.accounts.creator.key();
        proposal.proposal_id = proposal_id;
        proposal.title = title;
        proposal.description = description;
        proposal.yes_votes = 0;
        proposal.no_votes = 0;
        proposal.quorum = quorum;
        proposal.start_slot = clock.slot;
        proposal.end_slot = clock.slot + actual_duration;
        proposal.is_finalized = false;
        proposal.result = RESULT_PENDING;
        proposal.bump = ctx.bumps.proposal;

        msg!(
            "Proposal {} created by {} with quorum {}",
            proposal_id,
            ctx.accounts.creator.key(),
            quorum
        );

        Ok(())
    }

    /// Cast a vote on a proposal
    pub fn cast_vote(ctx: Context<CastVote>, proposal_id: u64, vote_yes: bool) -> Result<()> {
        // Verify proposal_id matches
        require!(
            ctx.accounts.proposal.proposal_id == proposal_id,
            GovernanceError::ProposalIdMismatch
        );

        let clock = Clock::get()?;
        let proposal = &mut ctx.accounts.proposal;

        // Validate voting period
        require!(
            clock.slot >= proposal.start_slot,
            GovernanceError::VotingNotStarted
        );
        require!(clock.slot < proposal.end_slot, GovernanceError::VotingEnded);
        require!(!proposal.is_finalized, GovernanceError::ProposalFinalized);

        // Verify token account is for governance token
        require!(
            ctx.accounts.voter_token_account.mint == ctx.accounts.config.governance_mint,
            GovernanceError::WrongTokenMint
        );

        // Get voting power (token balance at time of vote)
        let voting_power = ctx.accounts.voter_token_account.amount;
        require!(voting_power > 0, GovernanceError::NoVotingPower);

        // Initialize voter record (prevents double voting - will fail if already exists)
        let voter_record = &mut ctx.accounts.voter_record;
        voter_record.voter = ctx.accounts.voter.key();
        voter_record.proposal = proposal.key();
        voter_record.vote_yes = vote_yes;
        voter_record.voting_power = voting_power;
        voter_record.voted_at_slot = clock.slot;
        voter_record.bump = ctx.bumps.voter_record;

        // Add weighted vote count
        if vote_yes {
            proposal.yes_votes += voting_power;
        } else {
            proposal.no_votes += voting_power;
        }

        msg!(
            "Vote cast: voter={}, proposal={}, vote_yes={}, power={}",
            ctx.accounts.voter.key(),
            proposal.proposal_id,
            vote_yes,
            voting_power
        );

        Ok(())
    }

    /// Finalize a proposal after voting period ends
    pub fn finalize_proposal(ctx: Context<FinalizeProposal>) -> Result<()> {
        let proposal = &mut ctx.accounts.proposal;
        let clock = Clock::get()?;

        // Can only finalize after voting period ends
        require!(
            clock.slot >= proposal.end_slot,
            GovernanceError::VotingNotEnded
        );
        require!(!proposal.is_finalized, GovernanceError::ProposalFinalized);

        proposal.is_finalized = true;

        // Check if quorum was met
        let total_votes = proposal.yes_votes + proposal.no_votes;
        if total_votes < proposal.quorum {
            proposal.result = RESULT_QUORUM_NOT_MET;
            msg!(
                "Proposal {} finalized: QUORUM NOT MET (total: {}, quorum: {})",
                proposal.proposal_id,
                total_votes,
                proposal.quorum
            );
        } else if proposal.yes_votes > proposal.no_votes {
            proposal.result = RESULT_PASSED;
            msg!(
                "Proposal {} finalized: PASSED (yes: {}, no: {})",
                proposal.proposal_id,
                proposal.yes_votes,
                proposal.no_votes
            );
        } else {
            // Tie or more no votes = rejected
            proposal.result = RESULT_REJECTED;
            let status = if proposal.yes_votes == proposal.no_votes {
                "TIE/REJECTED"
            } else {
                "REJECTED"
            };
            msg!(
                "Proposal {} finalized: {} (yes: {}, no: {})",
                proposal.proposal_id,
                status,
                proposal.yes_votes,
                proposal.no_votes
            );
        }

        Ok(())
    }

    /// Get governance info (read-only, for logging)
    pub fn get_governance_info(ctx: Context<GetGovernanceInfo>) -> Result<()> {
        let config = &ctx.accounts.config;

        msg!("Governance: mint={}", config.governance_mint);
        msg!("Authority: {}", config.authority);
        msg!("Proposal threshold: {}", config.proposal_threshold);
        msg!("Default voting duration: {}", config.default_voting_duration);
        msg!("Next proposal ID: {}", config.next_proposal_id);

        Ok(())
    }

    /// Get proposal info (read-only, for logging)
    pub fn get_proposal_info(ctx: Context<GetProposalInfo>) -> Result<()> {
        let proposal = &ctx.accounts.proposal;

        msg!("Proposal {}: {}", proposal.proposal_id, proposal.title);
        msg!("Description: {}", proposal.description);
        msg!(
            "Votes: yes={}, no={}",
            proposal.yes_votes,
            proposal.no_votes
        );
        msg!("Quorum: {}", proposal.quorum);
        msg!(
            "Slots: start={}, end={}",
            proposal.start_slot,
            proposal.end_slot
        );
        msg!(
            "Finalized: {}, Result: {}",
            proposal.is_finalized,
            proposal.result
        );

        Ok(())
    }
}

// =============================================================================
// ACCOUNT STRUCTURES
// =============================================================================

#[account]
pub struct GovernanceConfig {
    /// Authority who controls governance
    pub authority: Pubkey,
    /// Governance token mint
    pub governance_mint: Pubkey,
    /// Minimum tokens required to create a proposal
    pub proposal_threshold: u64,
    /// Default voting duration in slots
    pub default_voting_duration: u64,
    /// Next proposal ID (auto-increment)
    pub next_proposal_id: u64,
    /// PDA bump seed
    pub bump: u8,
}

#[account]
pub struct Proposal {
    /// Governance config this proposal belongs to
    pub governance: Pubkey,
    /// Creator of the proposal
    pub creator: Pubkey,
    /// Unique proposal identifier
    pub proposal_id: u64,
    /// Proposal title (max 64 chars)
    pub title: String,
    /// Proposal description (max 256 chars)
    pub description: String,
    /// Total yes votes (weighted by tokens)
    pub yes_votes: u64,
    /// Total no votes (weighted by tokens)
    pub no_votes: u64,
    /// Minimum total votes required for quorum (in tokens)
    pub quorum: u64,
    /// Slot when voting starts
    pub start_slot: u64,
    /// Slot when voting ends
    pub end_slot: u64,
    /// Whether the proposal has been finalized
    pub is_finalized: bool,
    /// Result: 0=pending, 1=passed, 2=rejected, 3=quorum_not_met
    pub result: u8,
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
    /// Voting power used (token balance at time of vote)
    pub voting_power: u64,
    /// Slot when vote was cast
    pub voted_at_slot: u64,
    /// PDA bump seed
    pub bump: u8,
}

// =============================================================================
// INSTRUCTION CONTEXTS
// =============================================================================

#[derive(Accounts)]
#[instruction(proposal_threshold: u64, default_voting_duration: u64)]
pub struct InitializeGovernance<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    pub governance_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 32 + 8 + 8 + 8 + 1,
        seeds = [b"governance", governance_mint.key().as_ref()],
        bump
    )]
    pub config: Account<'info, GovernanceConfig>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(proposal_threshold: u64, default_voting_duration: u64)]
pub struct UpdateGovernanceConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub config: Account<'info, GovernanceConfig>,
}

#[derive(Accounts)]
#[instruction(proposal_id: u64, title: String, description: String, quorum: u64, voting_duration_slots: u64)]
pub struct CreateProposal<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    pub creator_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"governance", config.governance_mint.as_ref()],
        bump = config.bump
    )]
    pub config: Account<'info, GovernanceConfig>,

    #[account(
        init,
        payer = creator,
        space = 8 + 32 + 32 + 8 + (4 + 64) + (4 + 256) + 8 + 8 + 8 + 8 + 8 + 1 + 1 + 1,
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

    pub voter_token_account: Account<'info, TokenAccount>,

    #[account(
        seeds = [b"governance", config.governance_mint.as_ref()],
        bump = config.bump
    )]
    pub config: Account<'info, GovernanceConfig>,

    #[account(
        mut,
        seeds = [b"proposal", proposal_id.to_le_bytes().as_ref()],
        bump = proposal.bump
    )]
    pub proposal: Account<'info, Proposal>,

    #[account(
        init,
        payer = voter,
        space = 8 + 32 + 32 + 1 + 8 + 8 + 1,
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
pub struct GetGovernanceInfo<'info> {
    pub config: Account<'info, GovernanceConfig>,
}

#[derive(Accounts)]
pub struct GetProposalInfo<'info> {
    pub proposal: Account<'info, Proposal>,
}

// =============================================================================
// ERROR CODES
// =============================================================================

#[error_code]
pub enum GovernanceError {
    #[msg("Proposal threshold must be greater than zero")]
    InvalidThreshold,
    #[msg("Voting duration must be greater than zero")]
    InvalidDuration,
    #[msg("Title exceeds maximum length of 64 characters")]
    TitleTooLong,
    #[msg("Description exceeds maximum length of 256 characters")]
    DescriptionTooLong,
    #[msg("Quorum must be greater than zero")]
    InvalidQuorum,
    #[msg("Invalid proposal ID")]
    InvalidProposalId,
    #[msg("Insufficient tokens to create proposal")]
    InsufficientTokens,
    #[msg("Wrong token mint")]
    WrongTokenMint,
    #[msg("Proposal ID mismatch")]
    ProposalIdMismatch,
    #[msg("Voting has not started yet")]
    VotingNotStarted,
    #[msg("Voting period has ended")]
    VotingEnded,
    #[msg("Voting period has not ended yet")]
    VotingNotEnded,
    #[msg("Proposal has already been finalized")]
    ProposalFinalized,
    #[msg("No voting power (zero token balance)")]
    NoVotingPower,
    #[msg("Unauthorized")]
    Unauthorized,
}
