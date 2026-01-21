# governance
# Built with Seahorse v0.1.0
#
# Demonstrates token-weighted governance where voting power is proportional
# to tokens held at the time of voting.
#
# Features:
# - create_proposal: Creates a new proposal with voting period and quorum
# - cast_vote: Records a vote weighted by voter's token balance
# - finalize_proposal: Finalizes proposal if quorum met and voting ended
#
# Key patterns:
# - Voting power = token balance at time of vote
# - Quorum requirement (minimum total votes needed)
# - Double-vote prevention via voter record PDA
# - Time-based voting period (slot-based)
#
# Design note: Proposal PDA uses proposal_id (u64) instead of mint + id
# because Seahorse can't use Account types as PDA seeds directly.

from seahorse.prelude import *

declare_id('GovToken11111111111111111111111111111111111')

# Basis points denominator (100% = 10000)
BPS_DENOMINATOR = 10000


class GovernanceConfig(Account):
    # Authority who controls governance
    authority: Pubkey
    # Governance token mint
    governance_mint: Pubkey
    # Minimum tokens required to create a proposal
    proposal_threshold: u64
    # Default voting duration in slots
    default_voting_duration: u64
    # Next proposal ID (auto-increment)
    next_proposal_id: u64
    # PDA bump seed
    bump: u8


class Proposal(Account):
    # Governance config this proposal belongs to
    governance: Pubkey
    # Creator of the proposal
    creator: Pubkey
    # Unique proposal identifier
    proposal_id: u64
    # Proposal title (max 64 chars)
    title: str
    # Proposal description (max 256 chars)
    description: str
    # Total yes votes (weighted by tokens)
    yes_votes: u64
    # Total no votes (weighted by tokens)
    no_votes: u64
    # Minimum total votes required for quorum (in tokens)
    quorum: u64
    # Slot when voting starts
    start_slot: u64
    # Slot when voting ends
    end_slot: u64
    # Whether the proposal has been finalized
    is_finalized: bool
    # Result: 0=pending, 1=passed, 2=rejected, 3=quorum_not_met
    result: u8
    # PDA bump seed
    bump: u8


class VoterRecord(Account):
    # Voter who cast this vote
    voter: Pubkey
    # Proposal that was voted on
    proposal: Pubkey
    # Whether voted yes (true) or no (false)
    vote_yes: bool
    # Voting power used (token balance at time of vote)
    voting_power: u64
    # Slot when vote was cast
    voted_at_slot: u64
    # PDA bump seed
    bump: u8


@instruction
def initialize_governance(
    authority: Signer,
    governance_mint: TokenMint,
    config: Empty[GovernanceConfig],
    proposal_threshold: u64,
    default_voting_duration: u64
):
    # Validate inputs
    assert proposal_threshold > 0, 'Proposal threshold must be greater than zero'
    assert default_voting_duration > 0, 'Default voting duration must be greater than zero'

    # Get bump before init
    config_bump = config.bump()

    # Initialize governance config PDA
    config = config.init(
        payer=authority,
        seeds=['governance', governance_mint]
    )

    # Store config
    config.authority = authority.key()
    config.governance_mint = governance_mint.key()
    config.proposal_threshold = proposal_threshold
    config.default_voting_duration = default_voting_duration
    config.next_proposal_id = 1  # Start from 1
    config.bump = config_bump

    print(f'Governance initialized with mint {governance_mint.key()}')


@instruction
def update_governance_config(
    authority: Signer,
    config: GovernanceConfig,
    proposal_threshold: u64,
    default_voting_duration: u64
):
    # Validate authority
    assert authority.key() == config.authority, 'Unauthorized'

    # Validate inputs
    assert proposal_threshold > 0, 'Proposal threshold must be greater than zero'
    assert default_voting_duration > 0, 'Default voting duration must be greater than zero'

    # Update config
    config.proposal_threshold = proposal_threshold
    config.default_voting_duration = default_voting_duration

    print(f'Governance config updated')


@instruction
def create_proposal(
    creator: Signer,
    creator_token_account: TokenAccount,
    config: GovernanceConfig,
    proposal: Empty[Proposal],
    clock: Clock,
    proposal_id: u64,
    title: str,
    description: str,
    quorum: u64,
    voting_duration_slots: u64
):
    # Validate inputs
    assert len(title) <= 64, 'Title exceeds maximum length of 64 characters'
    assert len(description) <= 256, 'Description exceeds maximum length of 256 characters'
    assert quorum > 0, 'Quorum must be greater than zero'

    # Validate proposal_id matches expected next ID
    assert proposal_id == config.next_proposal_id, 'Invalid proposal ID'
    config.next_proposal_id += 1

    # Use default voting duration if 0 is passed
    actual_duration = voting_duration_slots
    if voting_duration_slots == 0:
        actual_duration = config.default_voting_duration

    # Check creator has enough tokens to create proposal
    creator_balance = creator_token_account.amount()
    assert creator_balance >= config.proposal_threshold, 'Insufficient tokens to create proposal'

    # Verify token account is for governance token
    assert creator_token_account.mint() == config.governance_mint, 'Wrong token mint'

    # Get bump before init
    proposal_bump = proposal.bump()

    # Get current slot from clock sysvar
    current_slot = clock.slot()

    # Initialize proposal PDA with padding for strings
    proposal = proposal.init(
        payer=creator,
        seeds=['proposal', proposal_id],
        padding=400
    )

    # Store proposal data
    proposal.governance = config.key()
    proposal.creator = creator.key()
    proposal.proposal_id = proposal_id
    proposal.title = title
    proposal.description = description
    proposal.yes_votes = 0
    proposal.no_votes = 0
    proposal.quorum = quorum
    proposal.start_slot = current_slot
    proposal.end_slot = current_slot + actual_duration
    proposal.is_finalized = False
    proposal.result = 0  # Pending
    proposal.bump = proposal_bump

    print(f'Proposal {proposal_id} created by {creator.key()} with quorum {quorum}')


@instruction
def cast_vote(
    voter: Signer,
    voter_token_account: TokenAccount,
    config: GovernanceConfig,
    proposal: Proposal,
    voter_record: Empty[VoterRecord],
    clock: Clock,
    proposal_id: u64,
    vote_yes: bool
):
    # Validate proposal_id matches
    assert proposal.proposal_id == proposal_id, 'Proposal ID mismatch'

    # Get current slot from clock sysvar
    current_slot = clock.slot()

    # Validate voting period
    assert current_slot >= proposal.start_slot, 'Voting has not started yet'
    assert current_slot < proposal.end_slot, 'Voting period has ended'
    assert not proposal.is_finalized, 'Proposal has already been finalized'

    # Verify token account is for governance token
    assert voter_token_account.mint() == config.governance_mint, 'Wrong token mint'

    # Get voting power (token balance at time of vote)
    voting_power = voter_token_account.amount()
    assert voting_power > 0, 'No voting power (zero token balance)'

    # Get bump before init
    record_bump = voter_record.bump()

    # Initialize voter record PDA - will fail if voter already voted
    voter_record = voter_record.init(
        payer=voter,
        seeds=['voter_record', proposal_id, voter]
    )

    # Record the vote
    voter_record.voter = voter.key()
    voter_record.proposal = proposal.key()
    voter_record.vote_yes = vote_yes
    voter_record.voting_power = voting_power
    voter_record.voted_at_slot = current_slot
    voter_record.bump = record_bump

    # Add weighted vote count
    if vote_yes:
        proposal.yes_votes += voting_power
    else:
        proposal.no_votes += voting_power

    print(f'Vote cast: voter={voter.key()}, proposal={proposal_id}, vote_yes={vote_yes}, power={voting_power}')


@instruction
def finalize_proposal(
    finalizer: Signer,
    proposal: Proposal,
    clock: Clock
):
    # Get current slot from clock sysvar
    current_slot = clock.slot()

    # Can only finalize after voting period ends
    assert current_slot >= proposal.end_slot, 'Voting period has not ended yet'
    assert not proposal.is_finalized, 'Proposal has already been finalized'

    # Mark as finalized
    proposal.is_finalized = True

    # Check if quorum was met
    total_votes = proposal.yes_votes + proposal.no_votes
    if total_votes < proposal.quorum:
        proposal.result = 3  # Quorum not met
        print(f'Proposal {proposal.proposal_id} finalized: QUORUM NOT MET (total: {total_votes}, quorum: {proposal.quorum})')
    elif proposal.yes_votes > proposal.no_votes:
        proposal.result = 1  # Passed
        print(f'Proposal {proposal.proposal_id} finalized: PASSED (yes: {proposal.yes_votes}, no: {proposal.no_votes})')
    elif proposal.no_votes > proposal.yes_votes:
        proposal.result = 2  # Rejected
        print(f'Proposal {proposal.proposal_id} finalized: REJECTED (yes: {proposal.yes_votes}, no: {proposal.no_votes})')
    else:
        proposal.result = 2  # Tie = rejected (conservative)
        print(f'Proposal {proposal.proposal_id} finalized: TIE/REJECTED (yes: {proposal.yes_votes}, no: {proposal.no_votes})')


@instruction
def get_governance_info(config: GovernanceConfig):
    print(f'Governance: mint={config.governance_mint}')
    print(f'Authority: {config.authority}')
    print(f'Proposal threshold: {config.proposal_threshold}')
    print(f'Default voting duration: {config.default_voting_duration}')
    print(f'Next proposal ID: {config.next_proposal_id}')


@instruction
def get_proposal_info(proposal: Proposal):
    print(f'Proposal {proposal.proposal_id}: {proposal.title}')
    print(f'Description: {proposal.description}')
    print(f'Votes: yes={proposal.yes_votes}, no={proposal.no_votes}')
    print(f'Quorum: {proposal.quorum}')
    print(f'Slots: start={proposal.start_slot}, end={proposal.end_slot}')
    print(f'Finalized: {proposal.is_finalized}, Result: {proposal.result}')
