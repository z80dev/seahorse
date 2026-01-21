# events
# Demonstrates comprehensive event emission patterns in Seahorse.
#
# Events are on-chain logs that clients can listen for and index.
# Seahorse events compile to Anchor's #[event] macro.
#
# Patterns demonstrated:
# 1. Simple event with basic types (u64, str)
# 2. User action events with Pubkey and timestamp
# 3. Numeric events with various integer types
# 4. Transfer events (from, to, amount pattern)
# 5. State change events (tracking before/after values)
# 6. Multiple events in single transaction
# 7. Event counter tracking

from seahorse.prelude import *

# Program ID
declare_id('YqBL3cHjsojPxJuyLF6bcQSYJ59p9X5qePjhWkXCEGR')


# ============================================================================
# Event Definitions
# ============================================================================

# Simple event with basic primitives
@dataclass
class SimpleEvent(Event):
    value: u64
    label: str


# Event with Pubkey (common pattern for tracking user actions)
@dataclass
class UserActionEvent(Event):
    user: Pubkey
    action_type: u8
    timestamp: i64


# Event with multiple numeric types
@dataclass
class NumericEvent(Event):
    unsigned_small: u8
    unsigned_medium: u32
    unsigned_large: u64
    signed_value: i64


# Event with two pubkeys (common in token transfer patterns)
@dataclass
class TransferEvent(Event):
    from_addr: Pubkey
    to_addr: Pubkey
    amount: u64
    memo: str


# Event for tracking state changes
@dataclass
class StateChangeEvent(Event):
    account: Pubkey
    old_value: u64
    new_value: u64
    change_type: str


# ============================================================================
# Account Structures
# ============================================================================

# EventCounter: Tracks event emission statistics
class EventCounter(Account):
    authority: Pubkey
    event_count: u64
    last_event_type: u8
    bump: u8


# ============================================================================
# Instructions
# ============================================================================

@instruction
def initialize(
    authority: Signer,
    counter: Empty[EventCounter],
):
    """
    Initialize the event counter account.

    Args:
        authority: Account authority who will emit events
        counter: The counter account to initialize
    """
    # Store bump before init
    bump = counter.bump()

    # Initialize counter PDA
    counter = counter.init(payer=authority, seeds=['counter', authority])

    counter.authority = authority.key()
    counter.event_count = 0
    counter.last_event_type = 0
    counter.bump = bump

    print("Initialized event counter")


@instruction
def emit_simple(
    authority: Signer,
    counter: EventCounter,
    value: u64,
    label: str,
):
    """
    Emit a simple event with basic data types.

    Args:
        authority: Account authority (must match counter.authority)
        counter: The event counter account
        value: Numeric value to include in event
        label: String label to include in event
    """
    # Verify authority
    assert counter.authority == authority.key(), "Unauthorized"

    # Update counter
    counter.event_count = counter.event_count + 1
    counter.last_event_type = 1  # SimpleEvent type

    # Create and emit event
    event = SimpleEvent(value=value, label=label)
    event.emit()

    print("Emitted SimpleEvent")


@instruction
def emit_user_action(
    authority: Signer,
    counter: EventCounter,
    action_type: u8,
    timestamp: i64,
):
    """
    Emit a user action event (tracks who did what and when).

    Args:
        authority: Account authority (user performing action)
        counter: The event counter account
        action_type: Type of action being performed
        timestamp: Unix timestamp of the action
    """
    # Verify authority
    assert counter.authority == authority.key(), "Unauthorized"

    # Update counter
    counter.event_count = counter.event_count + 1
    counter.last_event_type = 2  # UserActionEvent type

    # Create and emit event
    event = UserActionEvent(
        user=authority.key(),
        action_type=action_type,
        timestamp=timestamp
    )
    event.emit()

    print("Emitted UserActionEvent")


@instruction
def emit_numeric(
    authority: Signer,
    counter: EventCounter,
    unsigned_small: u8,
    unsigned_medium: u32,
    unsigned_large: u64,
    signed_value: i64,
):
    """
    Emit an event with various numeric types.

    Args:
        authority: Account authority
        counter: The event counter account
        unsigned_small: u8 value
        unsigned_medium: u32 value
        unsigned_large: u64 value
        signed_value: i64 value
    """
    # Verify authority
    assert counter.authority == authority.key(), "Unauthorized"

    # Update counter
    counter.event_count = counter.event_count + 1
    counter.last_event_type = 3  # NumericEvent type

    # Create and emit event
    event = NumericEvent(
        unsigned_small=unsigned_small,
        unsigned_medium=unsigned_medium,
        unsigned_large=unsigned_large,
        signed_value=signed_value
    )
    event.emit()

    print("Emitted NumericEvent")


@instruction
def emit_transfer(
    authority: Signer,
    counter: EventCounter,
    to_addr: Pubkey,
    amount: u64,
    memo: str,
):
    """
    Emit a transfer event (common pattern in token programs).

    Args:
        authority: Account authority (sender)
        counter: The event counter account
        to_addr: Recipient address
        amount: Transfer amount
        memo: Transfer memo/description
    """
    # Verify authority
    assert counter.authority == authority.key(), "Unauthorized"

    # Update counter
    counter.event_count = counter.event_count + 1
    counter.last_event_type = 4  # TransferEvent type

    # Create and emit event
    event = TransferEvent(
        from_addr=authority.key(),
        to_addr=to_addr,
        amount=amount,
        memo=memo
    )
    event.emit()

    print("Emitted TransferEvent")


@instruction
def emit_state_change(
    authority: Signer,
    counter: EventCounter,
    old_value: u64,
    new_value: u64,
    change_type: str,
):
    """
    Emit a state change event (tracks before/after values).

    Args:
        authority: Account authority
        counter: The event counter account
        old_value: Previous value
        new_value: New value after change
        change_type: Type of change (e.g., "increment", "set", "reset")
    """
    # Verify authority
    assert counter.authority == authority.key(), "Unauthorized"

    # Update counter
    counter.event_count = counter.event_count + 1
    counter.last_event_type = 5  # StateChangeEvent type

    # Create and emit event
    event = StateChangeEvent(
        account=counter.key(),
        old_value=old_value,
        new_value=new_value,
        change_type=change_type
    )
    event.emit()

    print("Emitted StateChangeEvent")


@instruction
def emit_multiple(
    authority: Signer,
    counter: EventCounter,
    count: u8,
):
    """
    Emit multiple events in a single transaction.
    Demonstrates batch event emission pattern.

    Args:
        authority: Account authority
        counter: The event counter account
        count: Number of events to emit (max 10 for compute limits)
    """
    # Verify authority
    assert counter.authority == authority.key(), "Unauthorized"

    # Limit to prevent compute unit exhaustion
    assert count <= 10, "Maximum 10 events per transaction"

    # Emit multiple events
    i: u8 = 0
    while i < count:
        event = SimpleEvent(value=u64(i), label="batch")
        event.emit()
        counter.event_count = counter.event_count + 1
        i = i + 1

    counter.last_event_type = 6  # Multiple events type

    print(f"Emitted {count} SimpleEvents")


@instruction
def get_counter_info(
    authority: Signer,
    counter: EventCounter,
):
    """
    Get event counter info (read-only, for logging/debugging).

    Args:
        authority: Signer for instruction
        counter: The event counter account
    """
    print(f"Event count: {counter.event_count}")
    print(f"Last event type: {counter.last_event_type}")
    print(f"Authority: {counter.authority}")
