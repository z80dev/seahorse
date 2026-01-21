# multisig
# Built with Seahorse v0.1.0
#
# Demonstrates a multisig wallet requiring M-of-N signatures to execute transactions.
#
# Features:
# - create_multisig: Creates a multisig wallet with N owners and threshold M
# - propose_transaction: Owner proposes a transaction (SOL transfer)
# - approve: Owner approves a proposed transaction
# - execute: Executes transaction once threshold is met
#
# Key patterns:
# - M-of-N threshold enforcement (e.g., 2-of-3, 3-of-5)
# - Approval tracking via individual approval record PDAs (prevents double-approval)
# - Transaction PDA with approval count
# - SOL transfers from multisig vault via PDA signer
#
# Design notes:
# - Uses tx_id (u64) instead of complex seeds for transaction PDA derivation
# - Approval records use ['approval', tx_id, approver] seeds
# - Multisig PDA holds SOL (lamports) for outgoing transfers
# - Supports up to 10 owners

from seahorse.prelude import *

declare_id('5h7CmLSs5q2ZfWFns5yUjN6BTYVxVxTNddJ2X1yHnWCd')

# Maximum number of owners allowed
MAX_OWNERS = 10


class Multisig(Account):
    # Unique multisig identifier
    multisig_id: u64
    # Number of required approvals (threshold M)
    threshold: u8
    # Number of owners
    owner_count: u8
    # Owner addresses (stored as array of pubkeys)
    # Storing up to MAX_OWNERS pubkeys
    owner_1: Pubkey
    owner_2: Pubkey
    owner_3: Pubkey
    owner_4: Pubkey
    owner_5: Pubkey
    owner_6: Pubkey
    owner_7: Pubkey
    owner_8: Pubkey
    owner_9: Pubkey
    owner_10: Pubkey
    # Next transaction ID (auto-increment)
    next_tx_id: u64
    # PDA bump seed
    bump: u8


class Transaction(Account):
    # Multisig this transaction belongs to
    multisig: Pubkey
    # Transaction ID
    tx_id: u64
    # Proposer of the transaction
    proposer: Pubkey
    # Recipient of the transfer
    recipient: Pubkey
    # Amount of lamports to transfer
    amount: u64
    # Number of approvals received
    approval_count: u8
    # Whether the transaction has been executed
    is_executed: bool
    # PDA bump seed
    bump: u8


class Approval(Account):
    # Approver who cast this approval
    approver: Pubkey
    # Transaction that was approved
    transaction: Pubkey
    # Slot when approval was given
    approved_at_slot: u64
    # PDA bump seed
    bump: u8


def is_owner(multisig: Multisig, addr: Pubkey) -> bool:
    # Check if address is one of the owners
    # Using stored pubkeys since we can't use dynamic arrays
    if addr == multisig.owner_1:
        return True
    if addr == multisig.owner_2:
        return True
    if addr == multisig.owner_3:
        return True
    if addr == multisig.owner_4:
        return True
    if addr == multisig.owner_5:
        return True
    if addr == multisig.owner_6:
        return True
    if addr == multisig.owner_7:
        return True
    if addr == multisig.owner_8:
        return True
    if addr == multisig.owner_9:
        return True
    if addr == multisig.owner_10:
        return True
    return False


@instruction
def create_multisig(
    creator: Signer,
    multisig: Empty[Multisig],
    multisig_id: u64,
    threshold: u8,
    owner_count: u8,
    owner_1: Pubkey,
    owner_2: Pubkey,
    owner_3: Pubkey
):
    # Validate inputs
    assert threshold > 0, 'Threshold must be greater than zero'
    assert owner_count >= threshold, 'Owner count must be at least threshold'
    assert owner_count <= MAX_OWNERS, 'Too many owners'
    assert owner_count >= 1, 'Must have at least one owner'
    assert owner_count <= 3, 'This instruction supports up to 3 owners'

    # Get bump before init
    multisig_bump = multisig.bump()

    # Initialize multisig PDA
    multisig = multisig.init(
        payer=creator,
        seeds=['multisig', multisig_id]
    )

    # Store multisig data
    multisig.multisig_id = multisig_id
    multisig.threshold = threshold
    multisig.owner_count = owner_count
    multisig.next_tx_id = 1  # Start from 1

    # Store owners - use default (creator) for unused slots
    multisig.owner_1 = owner_1
    multisig.owner_2 = owner_2
    multisig.owner_3 = owner_3
    # Initialize remaining slots with creator address as placeholder
    multisig.owner_4 = creator.key()
    multisig.owner_5 = creator.key()
    multisig.owner_6 = creator.key()
    multisig.owner_7 = creator.key()
    multisig.owner_8 = creator.key()
    multisig.owner_9 = creator.key()
    multisig.owner_10 = creator.key()
    multisig.bump = multisig_bump

    print(f'Multisig created: id={multisig_id}, threshold={threshold}, owners={owner_count}')


@instruction
def propose_transaction(
    proposer: Signer,
    multisig: Multisig,
    transaction: Empty[Transaction],
    tx_id: u64,
    recipient: Pubkey,
    amount: u64
):
    # Validate proposer is an owner
    assert is_owner(multisig, proposer.key()), 'Only owners can propose transactions'

    # Validate tx_id matches expected next ID
    assert tx_id == multisig.next_tx_id, 'Invalid transaction ID'
    multisig.next_tx_id += 1

    # Validate amount
    assert amount > 0, 'Amount must be greater than zero'

    # Get bump before init
    tx_bump = transaction.bump()

    # Initialize transaction PDA
    transaction = transaction.init(
        payer=proposer,
        seeds=['transaction', multisig.multisig_id, tx_id]
    )

    # Store transaction data
    transaction.multisig = multisig.key()
    transaction.tx_id = tx_id
    transaction.proposer = proposer.key()
    transaction.recipient = recipient
    transaction.amount = amount
    transaction.approval_count = 0  # No approvals yet
    transaction.is_executed = False
    transaction.bump = tx_bump

    print(f'Transaction proposed: id={tx_id}, recipient={recipient}, amount={amount}')


@instruction
def approve(
    approver: Signer,
    multisig: Multisig,
    transaction: Transaction,
    approval: Empty[Approval],
    clock: Clock,
    tx_id: u64
):
    # Validate approver is an owner
    assert is_owner(multisig, approver.key()), 'Only owners can approve'

    # Validate transaction belongs to this multisig
    assert transaction.multisig == multisig.key(), 'Transaction does not belong to this multisig'

    # Validate tx_id matches
    assert transaction.tx_id == tx_id, 'Transaction ID mismatch'

    # Validate not already executed
    assert not transaction.is_executed, 'Transaction already executed'

    # Get bump before init
    approval_bump = approval.bump()

    # Get current slot
    current_slot = clock.slot()

    # Initialize approval PDA - will fail if already exists (double approval prevention)
    approval = approval.init(
        payer=approver,
        seeds=['approval', multisig.multisig_id, tx_id, approver]
    )

    # Store approval data
    approval.approver = approver.key()
    approval.transaction = transaction.key()
    approval.approved_at_slot = current_slot
    approval.bump = approval_bump

    # Increment approval count
    transaction.approval_count += 1

    print(f'Transaction {tx_id} approved by {approver.key()}, count={transaction.approval_count}/{multisig.threshold}')


@instruction
def execute(
    executor: Signer,
    multisig: Multisig,
    transaction: Transaction,
    recipient: UncheckedAccount,
    tx_id: u64
):
    # Validate executor is an owner
    assert is_owner(multisig, executor.key()), 'Only owners can execute'

    # Validate transaction belongs to this multisig
    assert transaction.multisig == multisig.key(), 'Transaction does not belong to this multisig'

    # Validate tx_id matches
    assert transaction.tx_id == tx_id, 'Transaction ID mismatch'

    # Validate not already executed
    assert not transaction.is_executed, 'Transaction already executed'

    # Validate recipient matches
    assert transaction.recipient == recipient.key(), 'Recipient mismatch'

    # Validate threshold is met
    assert transaction.approval_count >= multisig.threshold, 'Threshold not met'

    # Mark as executed
    transaction.is_executed = True

    # Transfer lamports from multisig PDA to recipient
    # Using Signer.transfer_lamports because multisig is the authority
    multisig.transfer_lamports(
        to=recipient,
        amount=transaction.amount
    )

    print(f'Transaction {tx_id} executed: {transaction.amount} lamports to {recipient.key()}')


@instruction
def get_multisig_info(multisig: Multisig):
    print(f'Multisig {multisig.multisig_id}:')
    print(f'Threshold: {multisig.threshold}')
    print(f'Owner count: {multisig.owner_count}')
    print(f'Next tx ID: {multisig.next_tx_id}')


@instruction
def get_transaction_info(transaction: Transaction):
    print(f'Transaction {transaction.tx_id}:')
    print(f'Recipient: {transaction.recipient}')
    print(f'Amount: {transaction.amount}')
    print(f'Approvals: {transaction.approval_count}')
    print(f'Executed: {transaction.is_executed}')
