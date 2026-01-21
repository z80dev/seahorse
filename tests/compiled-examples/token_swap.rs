// ===== dot/mod.rs =====

pub mod program;

// ===== dot/program.rs =====

#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
use crate::{id, seahorse_util::*};
use anchor_lang::{prelude::*, solana_program};
use anchor_spl::token::{self, Mint, Token, TokenAccount};
use std::{cell::RefCell, rc::Rc};

seahorse_const! { MINIMUM_LIQUIDITY , 100 }

#[account]
#[derive(Debug)]
pub struct Pool {
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub lp_mint: Pubkey,
    pub pool_token_a: Pubkey,
    pub pool_token_b: Pubkey,
    pub fee_bps: u16,
    pub bump: u8,
}

impl<'info, 'entrypoint> Pool {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedPool<'info, 'entrypoint>> {
        let mint_a = account.mint_a.clone();
        let mint_b = account.mint_b.clone();
        let lp_mint = account.lp_mint.clone();
        let pool_token_a = account.pool_token_a.clone();
        let pool_token_b = account.pool_token_b.clone();
        let fee_bps = account.fee_bps;
        let bump = account.bump;

        Mutable::new(LoadedPool {
            __account__: account,
            __programs__: programs_map,
            mint_a,
            mint_b,
            lp_mint,
            pool_token_a,
            pool_token_b,
            fee_bps,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedPool>) {
        let mut loaded = loaded.borrow_mut();
        let mint_a = loaded.mint_a.clone();

        loaded.__account__.mint_a = mint_a;

        let mint_b = loaded.mint_b.clone();

        loaded.__account__.mint_b = mint_b;

        let lp_mint = loaded.lp_mint.clone();

        loaded.__account__.lp_mint = lp_mint;

        let pool_token_a = loaded.pool_token_a.clone();

        loaded.__account__.pool_token_a = pool_token_a;

        let pool_token_b = loaded.pool_token_b.clone();

        loaded.__account__.pool_token_b = pool_token_b;

        let fee_bps = loaded.fee_bps;

        loaded.__account__.fee_bps = fee_bps;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedPool<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, Pool>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub lp_mint: Pubkey,
    pub pool_token_a: Pubkey,
    pub pool_token_b: Pubkey,
    pub fee_bps: u16,
    pub bump: u8,
}

pub fn add_liquidity_handler<'info>(
    mut user: SeahorseSigner<'info, '_>,
    mut mint_a: SeahorseAccount<'info, '_, Mint>,
    mut mint_b: SeahorseAccount<'info, '_, Mint>,
    mut pool: Mutable<LoadedPool<'info, '_>>,
    mut lp_mint: SeahorseAccount<'info, '_, Mint>,
    mut pool_token_a: SeahorseAccount<'info, '_, TokenAccount>,
    mut pool_token_b: SeahorseAccount<'info, '_, TokenAccount>,
    mut user_token_a: SeahorseAccount<'info, '_, TokenAccount>,
    mut user_token_b: SeahorseAccount<'info, '_, TokenAccount>,
    mut user_lp_token: SeahorseAccount<'info, '_, TokenAccount>,
    mut amount_a: u64,
    mut amount_b: u64,
    mut min_lp_tokens: u64,
) -> () {
    if !((amount_a > 0) && (amount_b > 0)) {
        panic!("Amount must be greater than zero");
    }

    let mut pool_a_balance = pool_token_a.amount;
    let mut pool_b_balance = pool_token_b.amount;
    let mut lp_supply = lp_mint.supply;
    let mut deposit_a = 0;
    let mut deposit_b = 0;
    let mut lp_tokens = 0;

    if lp_supply == 0 {
        let mut product = amount_a * amount_b;
        let mut sqrt_product = isqrt(product.clone());

        if !(sqrt_product > MINIMUM_LIQUIDITY!()) {
            panic!("Initial deposit too small");
        }

        lp_tokens = sqrt_product - MINIMUM_LIQUIDITY!();

        deposit_a = amount_a;

        deposit_b = amount_b;
    } else {
        let mut ratio_a = (amount_a * lp_supply) / pool_a_balance;
        let mut ratio_b = (amount_b * lp_supply) / pool_b_balance;

        if ratio_a <= ratio_b {
            deposit_a = amount_a;

            deposit_b = (amount_a * pool_b_balance) / pool_a_balance;

            lp_tokens = (deposit_a * lp_supply) / pool_a_balance;
        } else {
            deposit_b = amount_b;

            deposit_a = (amount_b * pool_a_balance) / pool_b_balance;

            lp_tokens = (deposit_b * lp_supply) / pool_b_balance;
        }
    }

    if !(lp_tokens >= min_lp_tokens) {
        panic!("Slippage tolerance exceeded");
    }

    token::transfer(
        CpiContext::new(
            user_token_a.programs.get("token_program"),
            token::Transfer {
                from: user_token_a.to_account_info(),
                authority: user.clone().to_account_info(),
                to: pool_token_a.clone().to_account_info(),
            },
        ),
        deposit_a.clone(),
    )
    .unwrap();

    token::transfer(
        CpiContext::new(
            user_token_b.programs.get("token_program"),
            token::Transfer {
                from: user_token_b.to_account_info(),
                authority: user.clone().to_account_info(),
                to: pool_token_b.clone().to_account_info(),
            },
        ),
        deposit_b.clone(),
    )
    .unwrap();

    let mut bump = pool.borrow().bump;

    token::mint_to(
        CpiContext::new_with_signer(
            lp_mint.programs.get("token_program"),
            token::MintTo {
                mint: lp_mint.to_account_info(),
                authority: pool.borrow().__account__.to_account_info(),
                to: user_lp_token.clone().to_account_info(),
            },
            &[Mutable::new(vec![
                "pool".to_string().as_bytes().as_ref(),
                mint_a.key().as_ref(),
                mint_b.key().as_ref(),
                bump.to_le_bytes().as_ref(),
            ])
            .borrow()
            .as_slice()],
        ),
        lp_tokens.clone(),
    )
    .unwrap();

    solana_program::msg!(
        "{} {} {} {} {} {} {}",
        "Added liquidity:".to_string(),
        deposit_a,
        "A,".to_string(),
        deposit_b,
        "B, minted".to_string(),
        lp_tokens,
        "LP".to_string()
    );
}

pub fn initialize_pool_handler<'info>(
    mut payer: SeahorseSigner<'info, '_>,
    mut mint_a: SeahorseAccount<'info, '_, Mint>,
    mut mint_b: SeahorseAccount<'info, '_, Mint>,
    mut pool: Empty<Mutable<LoadedPool<'info, '_>>>,
    mut lp_mint: Empty<SeahorseAccount<'info, '_, Mint>>,
    mut pool_token_a: Empty<SeahorseAccount<'info, '_, TokenAccount>>,
    mut pool_token_b: Empty<SeahorseAccount<'info, '_, TokenAccount>>,
    mut fee_bps: u16,
) -> () {
    if !(fee_bps < 10000) {
        panic!("Invalid fee: must be less than 10000 basis points");
    }

    let mut bump = pool.bump.unwrap();
    let mut pool = pool.account.clone();
    let mut lp_mint = lp_mint.account.clone();
    let mut pool_token_a = pool_token_a.account.clone();
    let mut pool_token_b = pool_token_b.account.clone();

    assign!(pool.borrow_mut().mint_a, mint_a.key());

    assign!(pool.borrow_mut().mint_b, mint_b.key());

    assign!(pool.borrow_mut().lp_mint, lp_mint.key());

    assign!(pool.borrow_mut().pool_token_a, pool_token_a.key());

    assign!(pool.borrow_mut().pool_token_b, pool_token_b.key());

    assign!(pool.borrow_mut().fee_bps, fee_bps);

    assign!(pool.borrow_mut().bump, bump);

    solana_program::msg!(
        "{} {} {}",
        "Pool initialized with fee:".to_string(),
        fee_bps,
        "bps".to_string()
    );
}

pub fn isqrt(mut n: u64) -> u64 {
    if n == 0 {
        return <u64 as TryFrom<_>>::try_from(0).unwrap();
    }

    let mut x = n;
    let mut y = (x + 1) / 2;

    while y < x {
        x = y;

        y = (x + (n / x)) / 2;
    }

    return x;
}

pub fn remove_liquidity_handler<'info>(
    mut user: SeahorseSigner<'info, '_>,
    mut mint_a: SeahorseAccount<'info, '_, Mint>,
    mut mint_b: SeahorseAccount<'info, '_, Mint>,
    mut pool: Mutable<LoadedPool<'info, '_>>,
    mut lp_mint: SeahorseAccount<'info, '_, Mint>,
    mut pool_token_a: SeahorseAccount<'info, '_, TokenAccount>,
    mut pool_token_b: SeahorseAccount<'info, '_, TokenAccount>,
    mut user_token_a: SeahorseAccount<'info, '_, TokenAccount>,
    mut user_token_b: SeahorseAccount<'info, '_, TokenAccount>,
    mut user_lp_token: SeahorseAccount<'info, '_, TokenAccount>,
    mut lp_amount: u64,
    mut min_amount_a: u64,
    mut min_amount_b: u64,
) -> () {
    if !(lp_amount > 0) {
        panic!("Amount must be greater than zero");
    }

    let mut pool_a_balance = pool_token_a.amount;
    let mut pool_b_balance = pool_token_b.amount;
    let mut lp_supply = lp_mint.supply;
    let mut amount_a = (lp_amount * pool_a_balance) / lp_supply;
    let mut amount_b = (lp_amount * pool_b_balance) / lp_supply;

    if !((amount_a >= min_amount_a) && (amount_b >= min_amount_b)) {
        panic!("Slippage tolerance exceeded");
    }

    token::burn(
        CpiContext::new(
            lp_mint.programs.get("token_program"),
            token::Burn {
                mint: lp_mint.to_account_info(),
                authority: user.clone().to_account_info(),
                from: user_lp_token.clone().to_account_info(),
            },
        ),
        lp_amount.clone(),
    )
    .unwrap();

    let mut bump = pool.borrow().bump;

    token::transfer(
        CpiContext::new_with_signer(
            pool_token_a.programs.get("token_program"),
            token::Transfer {
                from: pool_token_a.to_account_info(),
                authority: pool.borrow().__account__.to_account_info(),
                to: user_token_a.clone().to_account_info(),
            },
            &[Mutable::new(vec![
                "pool".to_string().as_bytes().as_ref(),
                mint_a.key().as_ref(),
                mint_b.key().as_ref(),
                bump.to_le_bytes().as_ref(),
            ])
            .borrow()
            .as_slice()],
        ),
        amount_a.clone(),
    )
    .unwrap();

    token::transfer(
        CpiContext::new_with_signer(
            pool_token_b.programs.get("token_program"),
            token::Transfer {
                from: pool_token_b.to_account_info(),
                authority: pool.borrow().__account__.to_account_info(),
                to: user_token_b.clone().to_account_info(),
            },
            &[Mutable::new(vec![
                "pool".to_string().as_bytes().as_ref(),
                mint_a.key().as_ref(),
                mint_b.key().as_ref(),
                bump.to_le_bytes().as_ref(),
            ])
            .borrow()
            .as_slice()],
        ),
        amount_b.clone(),
    )
    .unwrap();

    solana_program::msg!(
        "{} {} {} {} {} {} {}",
        "Removed liquidity:".to_string(),
        lp_amount,
        "LP, received".to_string(),
        amount_a,
        "A,".to_string(),
        amount_b,
        "B".to_string()
    );
}

pub fn swap_handler<'info>(
    mut user: SeahorseSigner<'info, '_>,
    mut mint_a: SeahorseAccount<'info, '_, Mint>,
    mut mint_b: SeahorseAccount<'info, '_, Mint>,
    mut pool: Mutable<LoadedPool<'info, '_>>,
    mut pool_token_a: SeahorseAccount<'info, '_, TokenAccount>,
    mut pool_token_b: SeahorseAccount<'info, '_, TokenAccount>,
    mut user_token_a: SeahorseAccount<'info, '_, TokenAccount>,
    mut user_token_b: SeahorseAccount<'info, '_, TokenAccount>,
    mut amount_in: u64,
    mut min_amount_out: u64,
    mut swap_a_to_b: bool,
) -> () {
    if !(amount_in > 0) {
        panic!("Amount must be greater than zero");
    }

    let mut pool_a_balance = pool_token_a.amount;
    let mut pool_b_balance = pool_token_b.amount;
    let mut fee_bps = <u64 as TryFrom<_>>::try_from(pool.borrow().fee_bps.clone()).unwrap();
    let mut amount_in_with_fee = amount_in * (10000 - fee_bps);
    let mut reserve_in = 0;
    let mut reserve_out = 0;

    if swap_a_to_b {
        reserve_in = pool_a_balance;

        reserve_out = pool_b_balance;
    } else {
        reserve_in = pool_b_balance;

        reserve_out = pool_a_balance;
    }

    let mut numerator = amount_in_with_fee * reserve_out;
    let mut denominator = (reserve_in * 10000) + amount_in_with_fee;
    let mut amount_out = numerator / denominator;

    if !(amount_out >= min_amount_out) {
        panic!("Slippage tolerance exceeded");
    }

    let mut bump = pool.borrow().bump;

    if swap_a_to_b {
        token::transfer(
            CpiContext::new(
                user_token_a.programs.get("token_program"),
                token::Transfer {
                    from: user_token_a.to_account_info(),
                    authority: user.clone().to_account_info(),
                    to: pool_token_a.clone().to_account_info(),
                },
            ),
            amount_in.clone(),
        )
        .unwrap();

        token::transfer(
            CpiContext::new_with_signer(
                pool_token_b.programs.get("token_program"),
                token::Transfer {
                    from: pool_token_b.to_account_info(),
                    authority: pool.borrow().__account__.to_account_info(),
                    to: user_token_b.clone().to_account_info(),
                },
                &[Mutable::new(vec![
                    "pool".to_string().as_bytes().as_ref(),
                    mint_a.key().as_ref(),
                    mint_b.key().as_ref(),
                    bump.to_le_bytes().as_ref(),
                ])
                .borrow()
                .as_slice()],
            ),
            amount_out.clone(),
        )
        .unwrap();
    } else {
        token::transfer(
            CpiContext::new(
                user_token_b.programs.get("token_program"),
                token::Transfer {
                    from: user_token_b.to_account_info(),
                    authority: user.clone().to_account_info(),
                    to: pool_token_b.clone().to_account_info(),
                },
            ),
            amount_in.clone(),
        )
        .unwrap();

        token::transfer(
            CpiContext::new_with_signer(
                pool_token_a.programs.get("token_program"),
                token::Transfer {
                    from: pool_token_a.to_account_info(),
                    authority: pool.borrow().__account__.to_account_info(),
                    to: user_token_a.clone().to_account_info(),
                },
                &[Mutable::new(vec![
                    "pool".to_string().as_bytes().as_ref(),
                    mint_a.key().as_ref(),
                    mint_b.key().as_ref(),
                    bump.to_le_bytes().as_ref(),
                ])
                .borrow()
                .as_slice()],
            ),
            amount_out.clone(),
        )
        .unwrap();
    }

    solana_program::msg!(
        "{} {} {} {} {}",
        "Swapped".to_string(),
        amount_in,
        "in for".to_string(),
        amount_out,
        "out".to_string()
    );
}

// ===== lib.rs =====

#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]

pub mod dot;

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{self, AssociatedToken},
    token::{self, Mint, Token, TokenAccount},
};

use dot::program::*;
use std::{cell::RefCell, rc::Rc};

declare_id!("SwAP5VzVZYxLYtDmJq5DEmUKDEcPnP2zGxqHKbv2AmM");

pub mod seahorse_util {
    use super::*;
    use std::{
        collections::HashMap,
        fmt::Debug,
        ops::{Deref, Index, IndexMut},
    };

    pub struct Mutable<T>(Rc<RefCell<T>>);

    impl<T> Mutable<T> {
        pub fn new(obj: T) -> Self {
            Self(Rc::new(RefCell::new(obj)))
        }
    }

    impl<T> Clone for Mutable<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }

    impl<T> Deref for Mutable<T> {
        type Target = Rc<RefCell<T>>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<T: Debug> Debug for Mutable<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }

    impl<T: Default> Default for Mutable<T> {
        fn default() -> Self {
            Self::new(T::default())
        }
    }

    pub trait IndexWrapped {
        type Output;

        fn index_wrapped(&self, index: i128) -> &Self::Output;
    }

    pub trait IndexWrappedMut: IndexWrapped {
        fn index_wrapped_mut(&mut self, index: i128) -> &mut <Self as IndexWrapped>::Output;
    }

    impl<T> IndexWrapped for Vec<T> {
        type Output = T;

        fn index_wrapped(&self, mut index: i128) -> &Self::Output {
            if index < 0 {
                index += self.len() as i128;
            }

            let index: usize = index.try_into().unwrap();

            self.index(index)
        }
    }

    impl<T> IndexWrappedMut for Vec<T> {
        fn index_wrapped_mut(&mut self, mut index: i128) -> &mut <Self as IndexWrapped>::Output {
            if index < 0 {
                index += self.len() as i128;
            }

            let index: usize = index.try_into().unwrap();

            self.index_mut(index)
        }
    }

    impl<T, const N: usize> IndexWrapped for [T; N] {
        type Output = T;

        fn index_wrapped(&self, mut index: i128) -> &Self::Output {
            if index < 0 {
                index += N as i128;
            }

            let index: usize = index.try_into().unwrap();

            self.index(index)
        }
    }

    impl<T, const N: usize> IndexWrappedMut for [T; N] {
        fn index_wrapped_mut(&mut self, mut index: i128) -> &mut <Self as IndexWrapped>::Output {
            if index < 0 {
                index += N as i128;
            }

            let index: usize = index.try_into().unwrap();

            self.index_mut(index)
        }
    }

    #[derive(Clone)]
    pub struct Empty<T: Clone> {
        pub account: T,
        pub bump: Option<u8>,
    }

    #[derive(Clone, Debug)]
    pub struct ProgramsMap<'info>(pub HashMap<&'static str, AccountInfo<'info>>);

    impl<'info> ProgramsMap<'info> {
        pub fn get(&self, name: &'static str) -> AccountInfo<'info> {
            self.0.get(name).unwrap().clone()
        }
    }

    #[derive(Clone, Debug)]
    pub struct WithPrograms<'info, 'entrypoint, A> {
        pub account: &'entrypoint A,
        pub programs: &'entrypoint ProgramsMap<'info>,
    }

    impl<'info, 'entrypoint, A> Deref for WithPrograms<'info, 'entrypoint, A> {
        type Target = A;

        fn deref(&self) -> &Self::Target {
            &self.account
        }
    }

    pub type SeahorseAccount<'info, 'entrypoint, A> =
        WithPrograms<'info, 'entrypoint, Box<Account<'info, A>>>;

    pub type SeahorseSigner<'info, 'entrypoint> = WithPrograms<'info, 'entrypoint, Signer<'info>>;

    #[derive(Clone, Debug)]
    pub struct CpiAccount<'info> {
        #[doc = "CHECK: CpiAccounts temporarily store AccountInfos."]
        pub account_info: AccountInfo<'info>,
        pub is_writable: bool,
        pub is_signer: bool,
        pub seeds: Option<Vec<Vec<u8>>>,
    }

    #[macro_export]
    macro_rules! seahorse_const {
        ($ name : ident , $ value : expr) => {
            macro_rules! $name {
                () => {
                    $value
                };
            }

            pub(crate) use $name;
        };
    }

    pub trait Loadable {
        type Loaded;

        fn load(stored: Self) -> Self::Loaded;

        fn store(loaded: Self::Loaded) -> Self;
    }

    macro_rules! Loaded {
        ($ name : ty) => {
            <$name as Loadable>::Loaded
        };
    }

    pub(crate) use Loaded;

    #[macro_export]
    macro_rules! assign {
        ($ lval : expr , $ rval : expr) => {{
            let temp = $rval;

            $lval = temp;
        }};
    }

    #[macro_export]
    macro_rules! index_assign {
        ($ lval : expr , $ idx : expr , $ rval : expr) => {
            let temp_rval = $rval;
            let temp_idx = $idx;

            $lval[temp_idx] = temp_rval;
        };
    }

    pub(crate) use assign;

    pub(crate) use index_assign;

    pub(crate) use seahorse_const;
}

#[program]
mod token_swap {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    # [instruction (amount_a : u64 , amount_b : u64 , min_lp_tokens : u64)]
    pub struct AddLiquidity<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(mut)]
        pub mint_a: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub mint_b: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub pool: Box<Account<'info, dot::program::Pool>>,
        #[account(mut)]
        pub lp_mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub pool_token_a: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub pool_token_b: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub user_token_a: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub user_token_b: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub user_lp_token: Box<Account<'info, TokenAccount>>,
        pub token_program: Program<'info, Token>,
    }

    pub fn add_liquidity(
        ctx: Context<AddLiquidity>,
        amount_a: u64,
        amount_b: u64,
        min_lp_tokens: u64,
    ) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "token_program",
            ctx.accounts.token_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let user = SeahorseSigner {
            account: &ctx.accounts.user,
            programs: &programs_map,
        };

        let mint_a = SeahorseAccount {
            account: &ctx.accounts.mint_a,
            programs: &programs_map,
        };

        let mint_b = SeahorseAccount {
            account: &ctx.accounts.mint_b,
            programs: &programs_map,
        };

        let pool = dot::program::Pool::load(&mut ctx.accounts.pool, &programs_map);
        let lp_mint = SeahorseAccount {
            account: &ctx.accounts.lp_mint,
            programs: &programs_map,
        };

        let pool_token_a = SeahorseAccount {
            account: &ctx.accounts.pool_token_a,
            programs: &programs_map,
        };

        let pool_token_b = SeahorseAccount {
            account: &ctx.accounts.pool_token_b,
            programs: &programs_map,
        };

        let user_token_a = SeahorseAccount {
            account: &ctx.accounts.user_token_a,
            programs: &programs_map,
        };

        let user_token_b = SeahorseAccount {
            account: &ctx.accounts.user_token_b,
            programs: &programs_map,
        };

        let user_lp_token = SeahorseAccount {
            account: &ctx.accounts.user_lp_token,
            programs: &programs_map,
        };

        add_liquidity_handler(
            user.clone(),
            mint_a.clone(),
            mint_b.clone(),
            pool.clone(),
            lp_mint.clone(),
            pool_token_a.clone(),
            pool_token_b.clone(),
            user_token_a.clone(),
            user_token_b.clone(),
            user_lp_token.clone(),
            amount_a,
            amount_b,
            min_lp_tokens,
        );

        dot::program::Pool::store(pool);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (fee_bps : u16)]
    pub struct InitializePool<'info> {
        #[account(mut)]
        pub payer: Signer<'info>,
        #[account(mut)]
        pub mint_a: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub mint_b: Box<Account<'info, Mint>>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: Pool > () + 8 , payer = payer , seeds = ["pool" . as_bytes () . as_ref () , mint_a . key () . as_ref () , mint_b . key () . as_ref ()] , bump)]
        pub pool: Box<Account<'info, dot::program::Pool>>,
        # [account (init , payer = payer , seeds = ["lp_mint" . as_bytes () . as_ref () , mint_a . key () . as_ref () , mint_b . key () . as_ref ()] , bump , mint :: decimals = 6 , mint :: authority = pool)]
        pub lp_mint: Box<Account<'info, Mint>>,
        # [account (init , payer = payer , seeds = ["pool_token_a" . as_bytes () . as_ref () , mint_a . key () . as_ref () , mint_b . key () . as_ref ()] , bump , token :: mint = mint_a , token :: authority = pool)]
        pub pool_token_a: Box<Account<'info, TokenAccount>>,
        # [account (init , payer = payer , seeds = ["pool_token_b" . as_bytes () . as_ref () , mint_a . key () . as_ref () , mint_b . key () . as_ref ()] , bump , token :: mint = mint_b , token :: authority = pool)]
        pub pool_token_b: Box<Account<'info, TokenAccount>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
        pub token_program: Program<'info, Token>,
    }

    pub fn initialize_pool(ctx: Context<InitializePool>, fee_bps: u16) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "system_program",
            ctx.accounts.system_program.to_account_info(),
        );

        programs.insert(
            "token_program",
            ctx.accounts.token_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let payer = SeahorseSigner {
            account: &ctx.accounts.payer,
            programs: &programs_map,
        };

        let mint_a = SeahorseAccount {
            account: &ctx.accounts.mint_a,
            programs: &programs_map,
        };

        let mint_b = SeahorseAccount {
            account: &ctx.accounts.mint_b,
            programs: &programs_map,
        };

        let pool = Empty {
            account: dot::program::Pool::load(&mut ctx.accounts.pool, &programs_map),
            bump: Some(ctx.bumps.pool),
        };

        let lp_mint = Empty {
            account: SeahorseAccount {
                account: &ctx.accounts.lp_mint,
                programs: &programs_map,
            },
            bump: Some(ctx.bumps.lp_mint),
        };

        let pool_token_a = Empty {
            account: SeahorseAccount {
                account: &ctx.accounts.pool_token_a,
                programs: &programs_map,
            },
            bump: Some(ctx.bumps.pool_token_a),
        };

        let pool_token_b = Empty {
            account: SeahorseAccount {
                account: &ctx.accounts.pool_token_b,
                programs: &programs_map,
            },
            bump: Some(ctx.bumps.pool_token_b),
        };

        initialize_pool_handler(
            payer.clone(),
            mint_a.clone(),
            mint_b.clone(),
            pool.clone(),
            lp_mint.clone(),
            pool_token_a.clone(),
            pool_token_b.clone(),
            fee_bps,
        );

        dot::program::Pool::store(pool.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (lp_amount : u64 , min_amount_a : u64 , min_amount_b : u64)]
    pub struct RemoveLiquidity<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(mut)]
        pub mint_a: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub mint_b: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub pool: Box<Account<'info, dot::program::Pool>>,
        #[account(mut)]
        pub lp_mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub pool_token_a: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub pool_token_b: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub user_token_a: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub user_token_b: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub user_lp_token: Box<Account<'info, TokenAccount>>,
        pub token_program: Program<'info, Token>,
    }

    pub fn remove_liquidity(
        ctx: Context<RemoveLiquidity>,
        lp_amount: u64,
        min_amount_a: u64,
        min_amount_b: u64,
    ) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "token_program",
            ctx.accounts.token_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let user = SeahorseSigner {
            account: &ctx.accounts.user,
            programs: &programs_map,
        };

        let mint_a = SeahorseAccount {
            account: &ctx.accounts.mint_a,
            programs: &programs_map,
        };

        let mint_b = SeahorseAccount {
            account: &ctx.accounts.mint_b,
            programs: &programs_map,
        };

        let pool = dot::program::Pool::load(&mut ctx.accounts.pool, &programs_map);
        let lp_mint = SeahorseAccount {
            account: &ctx.accounts.lp_mint,
            programs: &programs_map,
        };

        let pool_token_a = SeahorseAccount {
            account: &ctx.accounts.pool_token_a,
            programs: &programs_map,
        };

        let pool_token_b = SeahorseAccount {
            account: &ctx.accounts.pool_token_b,
            programs: &programs_map,
        };

        let user_token_a = SeahorseAccount {
            account: &ctx.accounts.user_token_a,
            programs: &programs_map,
        };

        let user_token_b = SeahorseAccount {
            account: &ctx.accounts.user_token_b,
            programs: &programs_map,
        };

        let user_lp_token = SeahorseAccount {
            account: &ctx.accounts.user_lp_token,
            programs: &programs_map,
        };

        remove_liquidity_handler(
            user.clone(),
            mint_a.clone(),
            mint_b.clone(),
            pool.clone(),
            lp_mint.clone(),
            pool_token_a.clone(),
            pool_token_b.clone(),
            user_token_a.clone(),
            user_token_b.clone(),
            user_lp_token.clone(),
            lp_amount,
            min_amount_a,
            min_amount_b,
        );

        dot::program::Pool::store(pool);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (amount_in : u64 , min_amount_out : u64 , swap_a_to_b : bool)]
    pub struct Swap<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(mut)]
        pub mint_a: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub mint_b: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub pool: Box<Account<'info, dot::program::Pool>>,
        #[account(mut)]
        pub pool_token_a: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub pool_token_b: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub user_token_a: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub user_token_b: Box<Account<'info, TokenAccount>>,
        pub token_program: Program<'info, Token>,
    }

    pub fn swap(
        ctx: Context<Swap>,
        amount_in: u64,
        min_amount_out: u64,
        swap_a_to_b: bool,
    ) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "token_program",
            ctx.accounts.token_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let user = SeahorseSigner {
            account: &ctx.accounts.user,
            programs: &programs_map,
        };

        let mint_a = SeahorseAccount {
            account: &ctx.accounts.mint_a,
            programs: &programs_map,
        };

        let mint_b = SeahorseAccount {
            account: &ctx.accounts.mint_b,
            programs: &programs_map,
        };

        let pool = dot::program::Pool::load(&mut ctx.accounts.pool, &programs_map);
        let pool_token_a = SeahorseAccount {
            account: &ctx.accounts.pool_token_a,
            programs: &programs_map,
        };

        let pool_token_b = SeahorseAccount {
            account: &ctx.accounts.pool_token_b,
            programs: &programs_map,
        };

        let user_token_a = SeahorseAccount {
            account: &ctx.accounts.user_token_a,
            programs: &programs_map,
        };

        let user_token_b = SeahorseAccount {
            account: &ctx.accounts.user_token_b,
            programs: &programs_map,
        };

        swap_handler(
            user.clone(),
            mint_a.clone(),
            mint_b.clone(),
            pool.clone(),
            pool_token_a.clone(),
            pool_token_b.clone(),
            user_token_a.clone(),
            user_token_b.clone(),
            amount_in,
            min_amount_out,
            swap_a_to_b,
        );

        dot::program::Pool::store(pool);

        return Ok(());
    }
}

