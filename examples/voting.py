# voting
# Built with Seahorse v0.1.0
#
# Demonstrates basic voting with proposal and vote mechanics:
# - create_proposal: Creates a new proposal with voting period
# - cast_vote: Records a vote (yes/no) with voter record PDA for double-vote prevention
# - finalize_proposal: Finalizes proposal after voting period ends
#
# Key patterns:
# - One vote per user enforced via voter record PDA (fails if already exists)
# - Vote counting with yes_votes/no_votes
# - Time-based voting period (slot-based)
#
# Design note: Proposal PDA uses proposal_id (u64) to avoid UncheckedAccount limitation.
# Voter record uses proposal key + voter key for unique per-voter-per-proposal record.

from seahorse.prelude import *

declare_id('VoteSimp1e111111111111111111111111111111111')


class Proposal(Account):
    # Creator of the proposal
    creator: Pubkey
    # Unique proposal identifier
    proposal_id: u64
    # Proposal title (max 64 chars - padded via init)
    title: str
    # Proposal description (max 256 chars - padded via init)
    description: str
    # Number of yes votes
    yes_votes: u64
    # Number of no votes
    no_votes: u64
    # Slot when voting starts
    start_slot: u64
    # Slot when voting ends
    end_slot: u64
    # Whether the proposal has been finalized
    is_finalized: bool
    # PDA bump seed
    bump: u8


class VoterRecord(Account):
    # Voter who cast this vote
    voter: Pubkey
    # Proposal that was voted on (stored as Pubkey for reference)
    proposal: Pubkey
    # Whether voted yes (true) or no (false)
    vote_yes: bool
    # Slot when vote was cast
    voted_at_slot: u64
    # PDA bump seed
    bump: u8


@instruction
def create_proposal(
    creator: Signer,
    proposal: Empty[Proposal],
    clock: Clock,
    proposal_id: u64,
    title: str,
    description: str,
    voting_duration_slots: u64
):
    # Validate inputs
    assert len(title) <= 64, 'Title exceeds maximum length of 64 characters'
    assert len(description) <= 256, 'Description exceeds maximum length of 256 characters'
    assert voting_duration_slots > 0, 'Voting duration must be greater than zero'

    # Get bump before init
    proposal_bump = proposal.bump()

    # Get current slot from clock sysvar
    current_slot = clock.slot()

    # Initialize proposal PDA - use padding for variable-length strings
    # Space: discriminator(8) + creator(32) + proposal_id(8) + title(4+64) + description(4+256)
    #        + yes_votes(8) + no_votes(8) + start_slot(8) + end_slot(8) + is_finalized(1) + bump(1)
    proposal = proposal.init(
        payer=creator,
        seeds=['proposal', proposal_id],
        padding=400  # Extra space for title and description strings
    )

    # Store proposal data
    proposal.creator = creator.key()
    proposal.proposal_id = proposal_id
    proposal.title = title
    proposal.description = description
    proposal.yes_votes = 0
    proposal.no_votes = 0
    proposal.start_slot = current_slot
    proposal.end_slot = current_slot + voting_duration_slots
    proposal.is_finalized = False
    proposal.bump = proposal_bump

    print(f'Proposal {proposal_id} created by {creator.key()}')


@instruction
def cast_vote(
    voter: Signer,
    proposal: Proposal,
    voter_record: Empty[VoterRecord],
    clock: Clock,
    proposal_id: u64,
    vote_yes: bool
):
    # Get current slot from clock sysvar
    current_slot = clock.slot()

    # Validate proposal_id matches
    assert proposal.proposal_id == proposal_id, 'Proposal ID mismatch'

    # Validate voting period
    assert current_slot < proposal.end_slot, 'Voting period has ended'
    assert not proposal.is_finalized, 'Proposal has already been finalized'

    # Get bump before init
    record_bump = voter_record.bump()

    # Initialize voter record PDA
    # This will fail if the voter has already voted (PDA already exists)
    # Seeds: ['voter_record', proposal_id, voter.key()]
    # Using proposal_id instead of proposal.key() because Seahorse can't cast Account as Seed
    voter_record = voter_record.init(
        payer=voter,
        seeds=['voter_record', proposal_id, voter]
    )

    # Record the vote
    voter_record.voter = voter.key()
    voter_record.proposal = proposal.key()
    voter_record.vote_yes = vote_yes
    voter_record.voted_at_slot = current_slot
    voter_record.bump = record_bump

    # Increment vote count
    if vote_yes:
        proposal.yes_votes += 1
    else:
        proposal.no_votes += 1

    print(f'Vote cast: voter={voter.key()}, proposal={proposal.proposal_id}, vote_yes={vote_yes}')


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

    # Log result based on vote counts
    if proposal.yes_votes > proposal.no_votes:
        print(f'Proposal {proposal.proposal_id} finalized: PASSED (yes: {proposal.yes_votes}, no: {proposal.no_votes})')
    elif proposal.no_votes > proposal.yes_votes:
        print(f'Proposal {proposal.proposal_id} finalized: REJECTED (yes: {proposal.yes_votes}, no: {proposal.no_votes})')
    else:
        print(f'Proposal {proposal.proposal_id} finalized: TIE (yes: {proposal.yes_votes}, no: {proposal.no_votes})')


@instruction
def get_proposal_info(proposal: Proposal):
    print(f'Proposal {proposal.proposal_id}: {proposal.title}')
    print(f'Description: {proposal.description}')
    print(f'Votes: yes={proposal.yes_votes}, no={proposal.no_votes}')
    print(f'Slots: start={proposal.start_slot}, end={proposal.end_slot}')
    print(f'Finalized: {proposal.is_finalized}')
