# cpi_patterns
# Built with Seahorse v0.1.0
#
# Demonstrates various Cross-Program Invocation (CPI) patterns:
# 1. Basic CPI - Token transfer with user as signer
# 2. CPI with PDA signer seeds - PDA-signed token operations
# 3. Multiple CPIs - Chained operations in sequence
# 4. Error handling across CPI boundary - Pre-validation patterns

from seahorse.prelude import *

declare_id('Cpi1Pattrn111111111111111111111111111111111')


class CpiVault(Account):
    # Authority who controls this vault
    authority: Pubkey
    # Mint of tokens stored in this vault
    mint: Pubkey
    # Total tokens deposited into vault (cumulative)
    total_deposited: u64
    # Total tokens withdrawn from vault (cumulative)
    total_withdrawn: u64
    # Count of transfer operations
    transfer_count: u64
    # Bump seed for PDA derivation
    bump: u8


class MintConfig(Account):
    # Authority who controls minting
    authority: Pubkey
    # Mint this config controls
    mint: Pubkey
    # Total tokens minted
    total_minted: u64
    # Count of mint operations
    operation_count: u64
    # Bump seed for PDA derivation
    bump: u8


@instruction
def initialize_vault(
    authority: Signer,
    vault: Empty[CpiVault],
    vault_token_account: Empty[TokenAccount],
    mint: TokenMint
):
    """
    Initialize a CPI vault that can hold tokens and perform CPI operations
    Pattern: Account initialization with PDA
    """
    # Get bump before init
    bump = vault.bump()

    # Initialize vault state PDA
    vault = vault.init(
        payer=authority,
        seeds=['vault', authority, mint]
    )

    # Initialize vault's token account PDA with vault as authority
    vault_token_account.init(
        payer=authority,
        seeds=['vault_token', authority, mint],
        mint=mint,
        authority=vault
    )

    vault.authority = authority.key()
    vault.mint = mint.key()
    vault.total_deposited = 0
    vault.total_withdrawn = 0
    vault.transfer_count = 0
    vault.bump = bump

    print('Vault initialized: authority=', authority.key())


@instruction
def basic_transfer_to_vault(
    user: Signer,
    vault: CpiVault,
    user_token_account: TokenAccount,
    vault_token_account: TokenAccount,
    mint: TokenMint,
    amount: u64
):
    """
    Basic CPI - Transfer tokens from user to vault
    Pattern: User signs the transfer, basic CPI to token program

    This demonstrates the simplest form of CPI where the signer
    is an instruction account (not a PDA).
    """
    # Validate amount
    assert amount > 0, 'Amount must be greater than zero'

    # Basic CPI to Token Program - user is the authority (signer)
    # No signer seeds needed because user is a regular Signer
    user_token_account.transfer(
        authority=user,
        to=vault_token_account,
        amount=amount
    )

    # Update vault state
    vault.total_deposited += amount
    vault.transfer_count += 1

    print('Basic transfer: ', amount, ' tokens deposited')


@instruction
def pda_signed_transfer_from_vault(
    authority: Signer,
    vault: CpiVault,
    user_token_account: TokenAccount,
    vault_token_account: TokenAccount,
    mint: TokenMint,
    amount: u64
):
    """
    CPI with PDA signer - Transfer tokens from vault back to user
    Pattern: PDA signs the transfer using signer seeds

    This demonstrates how to use PDA signer seeds for CPI calls
    where the authority is a PDA, not a regular signer.
    """
    # Check authorization - only vault authority can withdraw
    assert authority.key() == vault.authority, 'Unauthorized: caller is not the authority'

    # Validate amount
    assert amount > 0, 'Amount must be greater than zero'

    # Check sufficient balance
    assert vault_token_account.amount() >= amount, 'Insufficient funds in vault'

    # CPI with PDA signer - vault is the authority of vault_token_account
    # Must provide signer seeds matching the vault PDA derivation
    bump = vault.bump
    vault_token_account.transfer(
        authority=vault,
        to=user_token_account,
        amount=amount,
        signer=['vault', authority, mint, bump]
    )

    # Update vault state
    vault.total_withdrawn += amount
    vault.transfer_count += 1

    print('PDA-signed transfer: ', amount, ' tokens withdrawn')


@instruction
def initialize_mint_config(
    authority: Signer,
    mint_config: Empty[MintConfig],
    mint: TokenMint
):
    """
    Initialize mint config - Create a mint authority PDA
    The mint_config PDA will be set as the mint authority
    """
    # Get bump before init
    bump = mint_config.bump()

    # Initialize mint config PDA
    mint_config = mint_config.init(
        payer=authority,
        seeds=['mint_config', authority]
    )

    mint_config.authority = authority.key()
    mint_config.mint = mint.key()
    mint_config.total_minted = 0
    mint_config.operation_count = 0
    mint_config.bump = bump

    print('MintConfig initialized: authority=', authority.key())


@instruction
def chained_mint_and_transfer(
    authority: Signer,
    mint_config: MintConfig,
    mint: TokenMint,
    intermediate_account: TokenAccount,
    destination_account: TokenAccount,
    mint_amount: u64,
    transfer_amount: u64
):
    """
    Multiple CPIs - Mint tokens and immediately transfer to another account
    Pattern: Chained CPI operations in sequence

    This demonstrates executing multiple CPIs in a single instruction:
    1. Mint tokens to an intermediate account (PDA-signed)
    2. Transfer some tokens to final destination (user-signed)
    """
    # Validation
    assert authority.key() == mint_config.authority, 'Unauthorized'
    assert mint_amount > 0, 'Amount must be greater than zero'
    assert transfer_amount <= mint_amount, 'Transfer amount exceeds minted amount'

    # First CPI: Mint tokens to intermediate account (PDA-signed)
    bump = mint_config.bump
    mint.mint(
        authority=mint_config,
        to=intermediate_account,
        amount=mint_amount,
        signer=['mint_config', authority, bump]
    )

    # Second CPI: Transfer some tokens to final destination
    # User (authority) is the owner of intermediate_account, so no PDA signer needed
    intermediate_account.transfer(
        authority=authority,
        to=destination_account,
        amount=transfer_amount
    )

    # Update state
    mint_config.total_minted += mint_amount
    mint_config.operation_count += 1

    print('Chained CPIs: minted ', mint_amount, ' transferred ', transfer_amount)


@instruction
def validated_burn(
    owner: Signer,
    source_account: TokenAccount,
    mint: TokenMint,
    amount: u64
):
    """
    Error handling across CPI - Demonstrate pre-validation patterns
    Pattern: Validate conditions before CPI to provide better error messages

    This demonstrates proper error handling for CPI calls:
    - Pre-validation gives clearer error messages
    - Prevents wasting compute units on failed CPIs
    """
    # Validate amount
    assert amount > 0, 'Amount must be greater than zero'

    # Pre-validation before CPI (better UX than letting CPI fail)
    # Check balance BEFORE making the CPI call
    available_balance = source_account.amount()
    assert available_balance >= amount, 'Insufficient balance for burn'

    # CPI to burn tokens
    mint.burn(
        authority=owner,
        holder=source_account,
        amount=amount
    )

    print('Validated burn: ', amount, ' tokens burned')


@instruction
def get_vault_info(
    vault: CpiVault
):
    """
    Read-only operation to display vault state
    Demonstrates accessing CPI-affected state
    """
    print('Vault authority: ', vault.authority)
    print('Vault mint: ', vault.mint)
    print('Total deposited: ', vault.total_deposited)
    print('Total withdrawn: ', vault.total_withdrawn)
    print('Transfer count: ', vault.transfer_count)
    print('Net balance: ', vault.total_deposited - vault.total_withdrawn)
