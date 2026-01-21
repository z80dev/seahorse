# realloc_close
# Demonstrates realloc (account resizing) and close (account closing) patterns.
#
# NOTE: Seahorse doesn't have native support for account realloc or close operations.
# This example demonstrates a tracking pattern where:
# - DynamicData tracks simulated resize operations and content
# - ResizeRecord tracks resize history and statistics
# - Instructions demonstrate the conceptual workflow
#
# The Anchor reference (realloc_close_anchor) implements actual realloc/close
# functionality using Anchor's realloc and close constraints.
#
# Patterns demonstrated:
# 1. initialize_data - Create account with initial content
# 2. grow_account - Track growth operation (simulated realloc)
# 3. shrink_account - Track shrink operation (simulated realloc)
# 4. close_data_account - Mark account as closed (simulated close)
# 5. create_record - Create resize tracking record
# 6. record_resize - Record a resize event for analytics
# 7. close_record - Mark record as inactive (simulated close)

from seahorse.prelude import *

# Program ID
declare_id('F4fSecp12t3QtQaXTUWrjxLec1wQXJ31iQzuE2WjPsoB')


# ============================================================================
# Account Structures
# ============================================================================

# DynamicData: Tracks simulated resize operations.
# In a real Anchor program, this would use realloc constraints.
# Here we track conceptual size since Seahorse doesn't support native realloc.
class DynamicData(Account):
    owner: Pubkey
    data_id: u64
    content: str       # Content stored in account (uses padding for max size)
    content_len: u64   # Actual content length (simulates variable sizing)
    allocated_size: u64  # Simulated allocated space
    version: u64
    is_closed: bool    # Simulates closed state (true = account would be closed)
    bump: u8


# ResizeRecord: Tracks resize operations for analytics.
# Demonstrates tracking size changes, max size, and resize count.
class ResizeRecord(Account):
    owner: Pubkey
    record_id: u64
    current_size: u64
    max_size_reached: u64
    resize_count: u64
    is_active: bool
    bump: u8


# ============================================================================
# Instructions
# ============================================================================

@instruction
def initialize_data(
    owner: Signer,
    data: Empty[DynamicData],
    data_id: u64,
    initial_content: str,
    initial_allocated_size: u64,
):
    """
    Initialize a dynamic data account with initial content.

    Args:
        owner: Account owner who will control the data
        data: The data account to initialize
        data_id: Unique identifier for PDA derivation
        initial_content: Initial string content
        initial_allocated_size: Simulated initial allocation size
    """
    # Store bump before init
    bump = data.bump()

    # Initialize with padding for max content (200 bytes for string)
    data = data.init(payer=owner, seeds=['data', data_id], padding=200)

    # Set initial state
    data.owner = owner.key()
    data.data_id = data_id
    data.content = initial_content
    data.content_len = u64(len(initial_content))
    data.allocated_size = initial_allocated_size
    data.version = 1
    data.is_closed = False
    data.bump = bump

    print(f"Initialized data account with id: {data_id}")
    print(f"Content length: {data.content_len} bytes")
    print(f"Allocated size: {data.allocated_size} bytes")


@instruction
def grow_account(
    owner: Signer,
    data: DynamicData,
    data_id: u64,
    new_content: str,
    additional_space: u64,
):
    """
    Simulate growing the account to accommodate more content.

    In Anchor, this would use realloc::payer and realloc::zero = true.
    Here we track the simulated size change.

    Args:
        owner: Account owner (must match stored owner)
        data: The data account to grow
        data_id: Data ID for verification
        new_content: New content to store
        additional_space: Additional bytes to allocate
    """
    # Verify owner
    assert data.owner == owner.key(), "Unauthorized"

    # Verify not closed
    assert not data.is_closed, "Account is closed"

    # Update content and simulated size
    old_size = data.allocated_size
    data.content = new_content
    data.content_len = u64(len(new_content))
    data.allocated_size = data.allocated_size + additional_space
    data.version = data.version + 1

    print(f"Grew account from {old_size} to {data.allocated_size} bytes")
    print(f"New content length: {data.content_len} bytes")
    print(f"Version: {data.version}")


@instruction
def shrink_account(
    owner: Signer,
    data: DynamicData,
    data_id: u64,
    new_content: str,
    new_size: u64,
):
    """
    Simulate shrinking the account to reclaim space.

    In Anchor, this would use realloc with smaller space.
    The excess lamports would be returned to the payer.

    Args:
        owner: Account owner (must match stored owner)
        data: The data account to shrink
        data_id: Data ID for verification
        new_content: New (smaller) content
        new_size: New allocated size (must be >= content length)
    """
    # Verify owner
    assert data.owner == owner.key(), "Unauthorized"

    # Verify not closed
    assert not data.is_closed, "Account is closed"

    # Verify new size is smaller
    assert new_size < data.allocated_size, "New size must be smaller"

    # Verify content fits in new size
    assert u64(len(new_content)) <= new_size, "Content too large for new size"

    # Update content and simulated size
    old_size = data.allocated_size
    data.content = new_content
    data.content_len = u64(len(new_content))
    data.allocated_size = new_size
    data.version = data.version + 1

    print(f"Shrunk account from {old_size} to {data.allocated_size} bytes")
    print(f"New content length: {data.content_len} bytes")
    print(f"Version: {data.version}")


@instruction
def close_data_account(
    owner: Signer,
    data: DynamicData,
    data_id: u64,
):
    """
    Simulate closing the data account.

    In Anchor, this would use the close constraint to:
    1. Zero out the account data
    2. Transfer remaining lamports to owner
    3. Set account owner to system program

    Here we mark it as closed since Seahorse can't actually close accounts.

    Args:
        owner: Account owner (receives lamports in real close)
        data: The data account to close
        data_id: Data ID for verification
    """
    # Verify owner
    assert data.owner == owner.key(), "Unauthorized"

    # Verify not already closed
    assert not data.is_closed, "Account already closed"

    # Mark as closed (simulated)
    final_version = data.version
    data.is_closed = True
    data.content = ""
    data.content_len = 0
    data.allocated_size = 0

    print("Closing data account, returning lamports to owner")
    print(f"Final version was: {final_version}")


@instruction
def create_record(
    owner: Signer,
    record: Empty[ResizeRecord],
    record_id: u64,
):
    """
    Create a record for tracking resize operations.

    Args:
        owner: Record owner
        record: The record account to initialize
        record_id: Unique identifier for PDA derivation
    """
    # Store bump before init
    bump = record.bump()

    # Initialize record
    record = record.init(payer=owner, seeds=['record', record_id])

    record.owner = owner.key()
    record.record_id = record_id
    record.current_size = 0
    record.max_size_reached = 0
    record.resize_count = 0
    record.is_active = True
    record.bump = bump

    print(f"Created record with id: {record_id}")


@instruction
def record_resize(
    owner: Signer,
    record: ResizeRecord,
    record_id: u64,
    new_size: u64,
    is_grow: bool,
):
    """
    Record a resize event for tracking/analytics.

    Args:
        owner: Record owner (must match stored owner)
        record: The record to update
        record_id: Record ID for verification
        new_size: New size after resize
        is_grow: True if growing, False if shrinking
    """
    # Verify owner
    assert record.owner == owner.key(), "Unauthorized"

    # Verify record is active
    assert record.is_active, "Record is inactive"

    # Update tracking
    record.current_size = new_size
    record.resize_count = record.resize_count + 1

    if new_size > record.max_size_reached:
        record.max_size_reached = new_size

    if is_grow:
        print(f"Recorded resize: grow to {new_size} bytes")
    else:
        print(f"Recorded resize: shrink to {new_size} bytes")

    print(f"Total resizes: {record.resize_count}")


@instruction
def close_record(
    owner: Signer,
    record: ResizeRecord,
    record_id: u64,
):
    """
    Close a resize record.

    In Anchor, this would close the account and return lamports.
    Here we mark it as inactive.

    Args:
        owner: Record owner
        record: The record to close
        record_id: Record ID for verification
    """
    # Verify owner
    assert record.owner == owner.key(), "Unauthorized"

    # Verify not already closed
    assert record.is_active, "Record already closed"

    final_count = record.resize_count

    # Mark as inactive (simulated close)
    record.is_active = False
    record.current_size = 0
    record.max_size_reached = 0
    record.resize_count = 0

    print("Closing record, returning lamports to owner")
    print(f"Final resize count: {final_count}")


@instruction
def get_data_info(
    owner: Signer,
    data: DynamicData,
    data_id: u64,
):
    """
    Get information about a data account (read-only).

    Args:
        owner: Signer for instruction
        data: The data account to query
        data_id: Data ID for verification
    """
    print(f"Data ID: {data.data_id}")
    print(f"Owner: {data.owner}")
    print(f"Content length: {data.content_len}")
    print(f"Allocated size: {data.allocated_size}")
    print(f"Version: {data.version}")
    print(f"Is closed: {data.is_closed}")


@instruction
def get_record_info(
    owner: Signer,
    record: ResizeRecord,
    record_id: u64,
):
    """
    Get information about a resize record (read-only).

    Args:
        owner: Signer for instruction
        record: The record to query
        record_id: Record ID for verification
    """
    print(f"Record ID: {record.record_id}")
    print(f"Owner: {record.owner}")
    print(f"Current size: {record.current_size}")
    print(f"Max size reached: {record.max_size_reached}")
    print(f"Resize count: {record.resize_count}")
    print(f"Is active: {record.is_active}")
