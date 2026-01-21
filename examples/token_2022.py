# token_2022
# Built with Seahorse v0.1.0
#
# Demonstrates Token-2022 transfer fee extension pattern.
#
# NOTE: Seahorse does not have native Token-2022 support. This example shows:
# - How to store transfer fee configuration
# - How to use UncheckedAccount for Token-2022 accounts
# - The pattern for interacting with Token-2022 (actual CPI requires raw invoke)
#
# For full Token-2022 functionality, use Anchor directly.
# See tests/anchor-reference/token_2022_anchor/ for the complete implementation.

from seahorse.prelude import *

declare_id('T2Ex5VzVZYxLYtDmJq5DEmUKDEcPnP2zGxqHKbv2Tbn')

# Maximum basis points (100%)
MAX_FEE_BASIS_POINTS = 10000


class FeeConfig(Account):
    # The Token-2022 mint address (stored for reference)
    mint: Pubkey
    # The authority who controls the fee config
    authority: Pubkey
    # Token decimals
    decimals: u8
    # Transfer fee in basis points (1% = 100 bps)
    transfer_fee_bps: u16
    # Maximum transfer fee in token base units
    max_fee: u64
    # Total fees collected (for tracking)
    total_fees_collected: u64
    # Bump for fee_config PDA
    bump: u8


@instruction
def initialize_fee_config(
    authority: Signer,
    fee_config: Empty[FeeConfig],
    mint: UncheckedAccount,  # Token-2022 mint (created externally)
    decimals: u8,
    transfer_fee_bps: u16,
    max_fee: u64
):
    """
    Initialize a fee configuration for an existing Token-2022 mint.

    The mint must be created externally with the transfer fee extension enabled.
    This instruction stores the fee configuration in a program-owned account
    for tracking and validation purposes.

    NOTE: In a full implementation, this would also call Token-2022 to set up
    the transfer fee extension. Seahorse doesn't support this natively.
    """
    # Validate fee
    assert transfer_fee_bps <= MAX_FEE_BASIS_POINTS, 'Fee exceeds maximum (10000 bps)'

    # Get bump before init
    config_bump = fee_config.bump()

    # Initialize fee config PDA
    fee_config = fee_config.init(
        payer=authority,
        seeds=['fee_config', authority]
    )

    # Store configuration
    fee_config.mint = mint.key()
    fee_config.authority = authority.key()
    fee_config.decimals = decimals
    fee_config.transfer_fee_bps = transfer_fee_bps
    fee_config.max_fee = max_fee
    fee_config.total_fees_collected = 0
    fee_config.bump = config_bump

    print('Initialized fee config for mint:', mint.key())
    print('Transfer fee:', transfer_fee_bps, 'bps')
    print('Max fee:', max_fee)


@instruction
def update_fee_rate(
    authority: Signer,
    fee_config: FeeConfig,
    new_transfer_fee_bps: u16,
    new_max_fee: u64
):
    """
    Update the transfer fee rate.

    NOTE: In a full implementation, this would also call Token-2022's
    set_transfer_fee instruction. Seahorse doesn't support this natively.
    """
    # Validate authority
    assert authority.key() == fee_config.authority, 'Unauthorized'

    # Validate fee
    assert new_transfer_fee_bps <= MAX_FEE_BASIS_POINTS, 'Fee exceeds maximum (10000 bps)'

    # Update config
    fee_config.transfer_fee_bps = new_transfer_fee_bps
    fee_config.max_fee = new_max_fee

    print('Updated fee rate to:', new_transfer_fee_bps, 'bps')
    print('New max fee:', new_max_fee)


@instruction
def calculate_transfer_fee(
    fee_config: FeeConfig,
    amount: u64
) -> u64:
    """
    Calculate the transfer fee for a given amount.

    This demonstrates the fee calculation logic used by Token-2022:
    fee = min(amount * fee_bps / 10000, max_fee)
    """
    # Calculate fee (basis points / 10000)
    fee = (amount * u64(fee_config.transfer_fee_bps)) // 10000

    # Cap at max fee
    if fee > fee_config.max_fee:
        fee = fee_config.max_fee

    print('Amount:', amount)
    print('Fee rate:', fee_config.transfer_fee_bps, 'bps')
    print('Calculated fee:', fee)

    return fee


@instruction
def record_fee_collection(
    authority: Signer,
    fee_config: FeeConfig,
    amount_collected: u64
):
    """
    Record fees collected from Token-2022 transfers.

    This is called after withdrawing withheld fees from Token-2022
    to update the tracking in our fee config account.
    """
    # Validate authority
    assert authority.key() == fee_config.authority, 'Unauthorized'

    # Update total collected
    fee_config.total_fees_collected += amount_collected

    print('Recorded fee collection:', amount_collected)
    print('Total fees collected:', fee_config.total_fees_collected)


@instruction
def get_fee_info(fee_config: FeeConfig):
    """
    Display the current fee configuration.
    """
    print('Mint:', fee_config.mint)
    print('Authority:', fee_config.authority)
    print('Decimals:', fee_config.decimals)
    print('Transfer fee:', fee_config.transfer_fee_bps, 'bps')
    print('Max fee:', fee_config.max_fee)
    print('Total collected:', fee_config.total_fees_collected)
