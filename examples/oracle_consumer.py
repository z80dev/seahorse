# oracle_consumer
# Built with Seahorse v0.1.0
#
# Demonstrates consuming oracle price feeds (Pyth/Switchboard pattern).
# Features:
# - Read price from oracle account structure
# - Staleness check based on slot numbers
# - Price threshold checks
# - Record price snapshots
#
# NOTE: Seahorse has built-in Pyth support via seahorse.pyth module (PriceAccount,
# PriceFeed, Price types). This example demonstrates the general pattern for:
# 1. Reading arbitrary price data from oracle accounts
# 2. Staleness validation based on slot timestamps
# 3. Recording price history for auditing
#
# The mocked oracle account structure simulates Pyth-style data:
# - price: i64 (price with exponent applied)
# - conf: u64 (confidence interval)
# - expo: i32 (exponent for price, e.g., -8 means price * 10^-8)
# - publish_slot: u64 (slot when price was published)

from seahorse.prelude import *

declare_id('3ePvSJdgkK1r5CrxyEjJoMnSCJJjCMoRKNdYLi6Ws8g7')

# Maximum age of price in slots before considered stale
# At ~400ms per slot, 100 slots = ~40 seconds
MAX_PRICE_AGE_SLOTS = 100

# Scale factor for price calculations (10^9 for precision)
PRICE_SCALE = 1000000000


class PriceConfig(Account):
    # Authority who created this config
    authority: Pubkey
    # Configuration ID for PDA derivation
    config_id: u64
    # Description/name of the price feed (max 32 chars)
    feed_name: str
    # Expected oracle account address (for validation)
    oracle_address: Pubkey
    # Maximum allowed staleness in slots
    max_staleness_slots: u64
    # Last recorded price
    last_price: i64
    # Last recorded confidence
    last_confidence: u64
    # Last recorded exponent
    last_exponent: i32
    # Slot when price was last recorded
    last_record_slot: u64
    # Count of price updates recorded
    update_count: u64
    # Bump seed for PDA
    bump: u8


class PriceThreshold(Account):
    # Config this threshold belongs to
    config: Pubkey
    # Threshold ID for PDA derivation
    threshold_id: u64
    # Whether to trigger when above (True) or below (False) target
    trigger_above: bool
    # Target price (scaled by PRICE_SCALE for precision without float)
    target_price_scaled: u64
    # Whether threshold has been triggered
    is_triggered: bool
    # Slot when threshold was triggered (0 if not triggered)
    triggered_at_slot: u64
    # Bump seed for PDA
    bump: u8


# Note: Since Seahorse doesn't support reading arbitrary account data layouts,
# we simulate oracle price data by passing values as instruction parameters.
# In a real integration, you would use Seahorse's built-in PriceAccount type
# for Pyth oracles, or implement custom account deserialization in the compiled code.


@instruction
def create_price_config(
    authority: Signer,
    price_config: Empty[PriceConfig],
    config_id: u64,
    feed_name: str,
    oracle_address: Pubkey,
    max_staleness_slots: u64
):
    """Create a new price feed configuration."""
    # Validate feed name length
    assert len(feed_name) <= 32, 'Feed name too long (max 32 chars)'

    # Validate staleness parameter
    assert max_staleness_slots > 0, 'Max staleness must be greater than zero'

    # Get bump before init
    config_bump = price_config.bump()

    # Initialize price config PDA
    price_config = price_config.init(
        payer=authority,
        seeds=['price_config', config_id],
        padding=50  # Extra space for feed_name string
    )

    # Store configuration
    price_config.authority = authority.key()
    price_config.config_id = config_id
    price_config.feed_name = feed_name
    price_config.oracle_address = oracle_address
    price_config.max_staleness_slots = max_staleness_slots
    price_config.last_price = 0
    price_config.last_confidence = 0
    price_config.last_exponent = 0
    price_config.last_record_slot = 0
    price_config.update_count = 0
    price_config.bump = config_bump


@instruction
def update_price_config(
    authority: Signer,
    price_config: PriceConfig,
    new_oracle_address: Pubkey,
    new_max_staleness_slots: u64
):
    """Update price feed configuration (authority only)."""
    # Validate authority
    assert authority.key() == price_config.authority, 'Unauthorized'

    # Validate staleness parameter
    assert new_max_staleness_slots > 0, 'Max staleness must be greater than zero'

    # Update configuration
    price_config.oracle_address = new_oracle_address
    price_config.max_staleness_slots = new_max_staleness_slots


@instruction
def record_price(
    authority: Signer,
    price_config: PriceConfig,
    clock: Clock,
    # Price data passed as parameters (simulates reading from oracle account)
    oracle_price: i64,
    oracle_confidence: u64,
    oracle_exponent: i32,
    oracle_publish_slot: u64
):
    """
    Record a price from the oracle.

    In production, this would read from the actual oracle account.
    For testing, we pass price data as parameters.
    """
    # Validate authority
    assert authority.key() == price_config.authority, 'Unauthorized'

    # Get current slot
    current_slot = clock.slot()

    # Staleness check: ensure price is not too old
    price_age = current_slot - oracle_publish_slot
    assert price_age <= price_config.max_staleness_slots, 'Price is stale'

    # Record the price
    price_config.last_price = oracle_price
    price_config.last_confidence = oracle_confidence
    price_config.last_exponent = oracle_exponent
    price_config.last_record_slot = current_slot
    price_config.update_count += 1

    # Log the recorded price
    print(oracle_price)
    print(oracle_exponent)


@instruction
def create_price_threshold(
    authority: Signer,
    price_config: PriceConfig,
    price_threshold: Empty[PriceThreshold],
    config_id: u64,
    threshold_id: u64,
    trigger_above: bool,
    target_price_scaled: u64
):
    """Create a price threshold for monitoring."""
    # Validate authority
    assert authority.key() == price_config.authority, 'Unauthorized'

    # Validate config_id matches price_config
    assert config_id == price_config.config_id, 'Config ID mismatch'

    # Validate target price
    assert target_price_scaled > 0, 'Target price must be greater than zero'

    # Get bump before init
    threshold_bump = price_threshold.bump()

    # Initialize threshold PDA
    # Seeds: ['threshold', config_id, threshold_id]
    price_threshold = price_threshold.init(
        payer=authority,
        seeds=['threshold', config_id, threshold_id]
    )

    # Store threshold configuration
    price_threshold.config = price_config.key()
    price_threshold.threshold_id = threshold_id
    price_threshold.trigger_above = trigger_above
    price_threshold.target_price_scaled = target_price_scaled
    price_threshold.is_triggered = False
    price_threshold.triggered_at_slot = 0
    price_threshold.bump = threshold_bump


@instruction
def check_threshold(
    price_config: PriceConfig,
    price_threshold: PriceThreshold,
    clock: Clock
):
    """Check if price threshold has been crossed."""
    # Validate threshold belongs to this config
    assert price_threshold.config == price_config.key(), 'Threshold does not belong to this config'

    # Skip if already triggered
    if price_threshold.is_triggered:
        print('Threshold already triggered')
        return

    # Get the recorded price and normalize to PRICE_SCALE
    # Price is stored as: actual_price = price * 10^exponent
    # We scale it to our PRICE_SCALE for comparison
    raw_price = price_config.last_price
    exponent = price_config.last_exponent

    # Calculate scaled price
    # If exponent is negative (typical): price_scaled = raw_price * PRICE_SCALE / 10^|exponent|
    # If exponent is positive: price_scaled = raw_price * PRICE_SCALE * 10^exponent
    # For simplicity, we'll just compare raw price * scaling factor
    # Most oracle prices use negative exponents (e.g., -8)

    # Convert raw price to u64 for comparison (handle sign)
    if raw_price < 0:
        print('Negative price not supported for threshold')
        return

    price_u64 = u64(raw_price)

    # Simple threshold check using raw price
    # In production, you'd apply proper exponent scaling
    target = price_threshold.target_price_scaled
    trigger_above = price_threshold.trigger_above

    triggered = False
    if trigger_above:
        if price_u64 >= target:
            triggered = True
    else:
        if price_u64 <= target:
            triggered = True

    if triggered:
        current_slot = clock.slot()
        price_threshold.is_triggered = True
        price_threshold.triggered_at_slot = current_slot
        print('Threshold triggered!')


@instruction
def reset_threshold(
    authority: Signer,
    price_config: PriceConfig,
    price_threshold: PriceThreshold
):
    """Reset a triggered threshold (authority only)."""
    # Validate authority
    assert authority.key() == price_config.authority, 'Unauthorized'

    # Validate threshold belongs to this config
    assert price_threshold.config == price_config.key(), 'Threshold does not belong to this config'

    # Reset threshold
    price_threshold.is_triggered = False
    price_threshold.triggered_at_slot = 0


@instruction
def get_price_info(
    price_config: PriceConfig,
    clock: Clock
):
    """Display current price information and staleness status."""
    current_slot = clock.slot()

    # Calculate age of last recorded price
    age_slots = current_slot - price_config.last_record_slot

    # Check if stale
    is_stale = age_slots > price_config.max_staleness_slots

    # Log price info
    print(price_config.last_price)
    print(price_config.last_confidence)
    print(price_config.last_exponent)
    print(age_slots)

    if is_stale:
        print('Price is STALE')
    else:
        print('Price is FRESH')
