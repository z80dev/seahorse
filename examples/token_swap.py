# token_swap
# Built with Seahorse v0.1.0
#
# Demonstrates a basic AMM (Automated Market Maker) with constant product formula.
# Features:
# - Initialize liquidity pool with two tokens and LP token
# - Add liquidity and receive LP tokens (proportional to contribution)
# - Remove liquidity by burning LP tokens
# - Swap tokens using constant product formula (x * y = k)

from seahorse.prelude import *

declare_id('SwAP5VzVZYxLYtDmJq5DEmUKDEcPnP2zGxqHKbv2AmM')

# Minimum liquidity locked on first deposit to prevent manipulation
MINIMUM_LIQUIDITY = 100


class Pool(Account):
    # Mint of token A
    mint_a: Pubkey
    # Mint of token B
    mint_b: Pubkey
    # LP token mint address
    lp_mint: Pubkey
    # Pool's token A account
    pool_token_a: Pubkey
    # Pool's token B account
    pool_token_b: Pubkey
    # Fee in basis points (e.g., 30 = 0.3%)
    fee_bps: u16
    # Bump seed for pool PDA
    bump: u8


@instruction
def initialize_pool(
    payer: Signer,
    mint_a: TokenMint,
    mint_b: TokenMint,
    pool: Empty[Pool],
    lp_mint: Empty[TokenMint],
    pool_token_a: Empty[TokenAccount],
    pool_token_b: Empty[TokenAccount],
    fee_bps: u16
):
    # Validate fee
    assert fee_bps < 10000, 'Invalid fee: must be less than 10000 basis points'

    # Get bump before init
    bump = pool.bump()

    # Initialize pool state PDA
    pool = pool.init(
        payer=payer,
        seeds=['pool', mint_a, mint_b]
    )

    # Initialize LP token mint with pool as authority
    lp_mint = lp_mint.init(
        payer=payer,
        seeds=['lp_mint', mint_a, mint_b],
        decimals=6,
        authority=pool
    )

    # Initialize pool's token A account with pool as authority
    pool_token_a = pool_token_a.init(
        payer=payer,
        seeds=['pool_token_a', mint_a, mint_b],
        mint=mint_a,
        authority=pool
    )

    # Initialize pool's token B account with pool as authority
    pool_token_b = pool_token_b.init(
        payer=payer,
        seeds=['pool_token_b', mint_a, mint_b],
        mint=mint_b,
        authority=pool
    )

    # Set pool state
    pool.mint_a = mint_a.key()
    pool.mint_b = mint_b.key()
    pool.lp_mint = lp_mint.key()
    pool.pool_token_a = pool_token_a.key()
    pool.pool_token_b = pool_token_b.key()
    pool.fee_bps = fee_bps
    pool.bump = bump

    print('Pool initialized with fee:', fee_bps, 'bps')


@instruction
def add_liquidity(
    user: Signer,
    mint_a: TokenMint,
    mint_b: TokenMint,
    pool: Pool,
    lp_mint: TokenMint,
    pool_token_a: TokenAccount,
    pool_token_b: TokenAccount,
    user_token_a: TokenAccount,
    user_token_b: TokenAccount,
    user_lp_token: TokenAccount,
    amount_a: u64,
    amount_b: u64,
    min_lp_tokens: u64
):
    # Validate amounts
    assert amount_a > 0 and amount_b > 0, 'Amount must be greater than zero'

    pool_a_balance: u64 = pool_token_a.amount()
    pool_b_balance: u64 = pool_token_b.amount()
    lp_supply: u64 = lp_mint.supply()

    # Calculate actual deposits and LP tokens to mint
    deposit_a: u64 = 0
    deposit_b: u64 = 0
    lp_tokens: u64 = 0

    if lp_supply == 0:
        # First deposit: use geometric mean and lock minimum liquidity
        # lp_tokens = sqrt(amount_a * amount_b) - MINIMUM_LIQUIDITY
        product: u64 = amount_a * amount_b
        sqrt_product: u64 = isqrt(product)
        assert sqrt_product > MINIMUM_LIQUIDITY, 'Initial deposit too small'
        lp_tokens = sqrt_product - MINIMUM_LIQUIDITY
        deposit_a = amount_a
        deposit_b = amount_b
    else:
        # Subsequent deposits: calculate proportional amounts
        # Use the smaller ratio to determine actual deposits
        ratio_a: u64 = (amount_a * lp_supply) // pool_a_balance
        ratio_b: u64 = (amount_b * lp_supply) // pool_b_balance

        if ratio_a <= ratio_b:
            deposit_a = amount_a
            deposit_b = (amount_a * pool_b_balance) // pool_a_balance
            lp_tokens = (deposit_a * lp_supply) // pool_a_balance
        else:
            deposit_b = amount_b
            deposit_a = (amount_b * pool_a_balance) // pool_b_balance
            lp_tokens = (deposit_b * lp_supply) // pool_b_balance

    # Check slippage
    assert lp_tokens >= min_lp_tokens, 'Slippage tolerance exceeded'

    # Transfer tokens from user to pool
    user_token_a.transfer(
        authority=user,
        to=pool_token_a,
        amount=deposit_a
    )

    user_token_b.transfer(
        authority=user,
        to=pool_token_b,
        amount=deposit_b
    )

    # Mint LP tokens to user using pool PDA signer
    bump = pool.bump
    lp_mint.mint(
        authority=pool,
        to=user_lp_token,
        amount=lp_tokens,
        signer=['pool', mint_a, mint_b, bump]
    )

    print('Added liquidity:', deposit_a, 'A,', deposit_b, 'B, minted', lp_tokens, 'LP')


@instruction
def remove_liquidity(
    user: Signer,
    mint_a: TokenMint,
    mint_b: TokenMint,
    pool: Pool,
    lp_mint: TokenMint,
    pool_token_a: TokenAccount,
    pool_token_b: TokenAccount,
    user_token_a: TokenAccount,
    user_token_b: TokenAccount,
    user_lp_token: TokenAccount,
    lp_amount: u64,
    min_amount_a: u64,
    min_amount_b: u64
):
    # Validate amount
    assert lp_amount > 0, 'Amount must be greater than zero'

    pool_a_balance: u64 = pool_token_a.amount()
    pool_b_balance: u64 = pool_token_b.amount()
    lp_supply: u64 = lp_mint.supply()

    # Calculate proportional withdrawals
    amount_a: u64 = (lp_amount * pool_a_balance) // lp_supply
    amount_b: u64 = (lp_amount * pool_b_balance) // lp_supply

    # Check slippage
    assert amount_a >= min_amount_a and amount_b >= min_amount_b, 'Slippage tolerance exceeded'

    # Burn LP tokens from user
    lp_mint.burn(
        authority=user,
        holder=user_lp_token,
        amount=lp_amount
    )

    # Transfer tokens from pool to user using pool PDA signer
    bump = pool.bump
    pool_token_a.transfer(
        authority=pool,
        to=user_token_a,
        amount=amount_a,
        signer=['pool', mint_a, mint_b, bump]
    )

    pool_token_b.transfer(
        authority=pool,
        to=user_token_b,
        amount=amount_b,
        signer=['pool', mint_a, mint_b, bump]
    )

    print('Removed liquidity:', lp_amount, 'LP, received', amount_a, 'A,', amount_b, 'B')


@instruction
def swap(
    user: Signer,
    mint_a: TokenMint,
    mint_b: TokenMint,
    pool: Pool,
    pool_token_a: TokenAccount,
    pool_token_b: TokenAccount,
    user_token_a: TokenAccount,
    user_token_b: TokenAccount,
    amount_in: u64,
    min_amount_out: u64,
    swap_a_to_b: bool
):
    # Validate amount
    assert amount_in > 0, 'Amount must be greater than zero'

    pool_a_balance: u64 = pool_token_a.amount()
    pool_b_balance: u64 = pool_token_b.amount()
    fee_bps: u64 = u64(pool.fee_bps)

    # Calculate output using constant product formula with fee
    # output = (input * (10000 - fee_bps) * reserve_out) / (reserve_in * 10000 + input * (10000 - fee_bps))
    amount_in_with_fee: u64 = amount_in * (10000 - fee_bps)

    reserve_in: u64 = 0
    reserve_out: u64 = 0
    if swap_a_to_b:
        reserve_in = pool_a_balance
        reserve_out = pool_b_balance
    else:
        reserve_in = pool_b_balance
        reserve_out = pool_a_balance

    numerator: u64 = amount_in_with_fee * reserve_out
    denominator: u64 = reserve_in * 10000 + amount_in_with_fee
    amount_out: u64 = numerator // denominator

    # Check slippage
    assert amount_out >= min_amount_out, 'Slippage tolerance exceeded'

    bump = pool.bump

    if swap_a_to_b:
        # Transfer A from user to pool
        user_token_a.transfer(
            authority=user,
            to=pool_token_a,
            amount=amount_in
        )

        # Transfer B from pool to user
        pool_token_b.transfer(
            authority=pool,
            to=user_token_b,
            amount=amount_out,
            signer=['pool', mint_a, mint_b, bump]
        )
    else:
        # Transfer B from user to pool
        user_token_b.transfer(
            authority=user,
            to=pool_token_b,
            amount=amount_in
        )

        # Transfer A from pool to user
        pool_token_a.transfer(
            authority=pool,
            to=user_token_a,
            amount=amount_out,
            signer=['pool', mint_a, mint_b, bump]
        )

    print('Swapped', amount_in, 'in for', amount_out, 'out')


# Integer square root using Newton's method
def isqrt(n: u64) -> u64:
    if n == 0:
        return u64(0)
    x: u64 = n
    y: u64 = (x + 1) // 2
    while y < x:
        x = y
        y = (x + n // x) // 2
    return x
