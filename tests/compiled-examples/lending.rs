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

seahorse_const! { BPS_DENOMINATOR , 10000 }

seahorse_const! { COLLATERAL_RATIO_BPS , 15000 }

seahorse_const! { INTEREST_SCALE , 1000000 }

#[account]
#[derive(Debug)]
pub struct Pool {
    pub authority: Pubkey,
    pub collateral_mint: Pubkey,
    pub borrow_mint: Pubkey,
    pub collateral_vault: Pubkey,
    pub borrow_vault: Pubkey,
    pub interest_rate: u64,
    pub total_deposited: u64,
    pub total_borrowed: u64,
    pub last_update_slot: u64,
    pub bump: u8,
}

impl<'info, 'entrypoint> Pool {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedPool<'info, 'entrypoint>> {
        let authority = account.authority.clone();
        let collateral_mint = account.collateral_mint.clone();
        let borrow_mint = account.borrow_mint.clone();
        let collateral_vault = account.collateral_vault.clone();
        let borrow_vault = account.borrow_vault.clone();
        let interest_rate = account.interest_rate;
        let total_deposited = account.total_deposited;
        let total_borrowed = account.total_borrowed;
        let last_update_slot = account.last_update_slot;
        let bump = account.bump;

        Mutable::new(LoadedPool {
            __account__: account,
            __programs__: programs_map,
            authority,
            collateral_mint,
            borrow_mint,
            collateral_vault,
            borrow_vault,
            interest_rate,
            total_deposited,
            total_borrowed,
            last_update_slot,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedPool>) {
        let mut loaded = loaded.borrow_mut();
        let authority = loaded.authority.clone();

        loaded.__account__.authority = authority;

        let collateral_mint = loaded.collateral_mint.clone();

        loaded.__account__.collateral_mint = collateral_mint;

        let borrow_mint = loaded.borrow_mint.clone();

        loaded.__account__.borrow_mint = borrow_mint;

        let collateral_vault = loaded.collateral_vault.clone();

        loaded.__account__.collateral_vault = collateral_vault;

        let borrow_vault = loaded.borrow_vault.clone();

        loaded.__account__.borrow_vault = borrow_vault;

        let interest_rate = loaded.interest_rate;

        loaded.__account__.interest_rate = interest_rate;

        let total_deposited = loaded.total_deposited;

        loaded.__account__.total_deposited = total_deposited;

        let total_borrowed = loaded.total_borrowed;

        loaded.__account__.total_borrowed = total_borrowed;

        let last_update_slot = loaded.last_update_slot;

        loaded.__account__.last_update_slot = last_update_slot;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedPool<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, Pool>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub authority: Pubkey,
    pub collateral_mint: Pubkey,
    pub borrow_mint: Pubkey,
    pub collateral_vault: Pubkey,
    pub borrow_vault: Pubkey,
    pub interest_rate: u64,
    pub total_deposited: u64,
    pub total_borrowed: u64,
    pub last_update_slot: u64,
    pub bump: u8,
}

#[account]
#[derive(Debug)]
pub struct UserPosition {
    pub owner: Pubkey,
    pub pool: Pubkey,
    pub collateral_deposited: u64,
    pub borrowed_amount: u64,
    pub last_update_slot: u64,
    pub bump: u8,
}

impl<'info, 'entrypoint> UserPosition {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedUserPosition<'info, 'entrypoint>> {
        let owner = account.owner.clone();
        let pool = account.pool.clone();
        let collateral_deposited = account.collateral_deposited;
        let borrowed_amount = account.borrowed_amount;
        let last_update_slot = account.last_update_slot;
        let bump = account.bump;

        Mutable::new(LoadedUserPosition {
            __account__: account,
            __programs__: programs_map,
            owner,
            pool,
            collateral_deposited,
            borrowed_amount,
            last_update_slot,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedUserPosition>) {
        let mut loaded = loaded.borrow_mut();
        let owner = loaded.owner.clone();

        loaded.__account__.owner = owner;

        let pool = loaded.pool.clone();

        loaded.__account__.pool = pool;

        let collateral_deposited = loaded.collateral_deposited;

        loaded.__account__.collateral_deposited = collateral_deposited;

        let borrowed_amount = loaded.borrowed_amount;

        loaded.__account__.borrowed_amount = borrowed_amount;

        let last_update_slot = loaded.last_update_slot;

        loaded.__account__.last_update_slot = last_update_slot;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedUserPosition<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, UserPosition>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub owner: Pubkey,
    pub pool: Pubkey,
    pub collateral_deposited: u64,
    pub borrowed_amount: u64,
    pub last_update_slot: u64,
    pub bump: u8,
}

pub fn borrow_handler<'info>(
    mut user: SeahorseSigner<'info, '_>,
    mut pool: Mutable<LoadedPool<'info, '_>>,
    mut user_position: Mutable<LoadedUserPosition<'info, '_>>,
    mut user_borrow_token: SeahorseAccount<'info, '_, TokenAccount>,
    mut borrow_vault: SeahorseAccount<'info, '_, TokenAccount>,
    mut collateral_mint: SeahorseAccount<'info, '_, Mint>,
    mut clock: Sysvar<'info, Clock>,
    mut amount: u64,
) -> () {
    if !(amount > 0) {
        panic!("Amount must be greater than zero");
    }

    if !(user.key() == user_position.borrow().owner) {
        panic!("Unauthorized");
    }

    let mut current_slot = clock.slot;
    let mut slots_elapsed = current_slot - user_position.borrow().last_update_slot;
    let mut accrued_interest =
        ((user_position.borrow().borrowed_amount * pool.borrow().interest_rate) * slots_elapsed)
            / INTEREST_SCALE!();

    let mut new_debt = (user_position.borrow().borrowed_amount + accrued_interest) + amount;
    let mut required_collateral = (new_debt * COLLATERAL_RATIO_BPS!()) / BPS_DENOMINATOR!();

    if !(user_position.borrow().collateral_deposited >= required_collateral) {
        panic!("Insufficient collateral");
    }

    if !(borrow_vault.amount >= amount) {
        panic!("Insufficient liquidity");
    }

    let mut pool_bump = pool.borrow().bump;

    token::transfer(
        CpiContext::new_with_signer(
            borrow_vault.programs.get("token_program"),
            token::Transfer {
                from: borrow_vault.to_account_info(),
                authority: pool.borrow().__account__.to_account_info(),
                to: user_borrow_token.clone().to_account_info(),
            },
            &[Mutable::new(vec![
                "pool".to_string().as_bytes().as_ref(),
                collateral_mint.key().as_ref(),
                pool_bump.to_le_bytes().as_ref(),
            ])
            .borrow()
            .as_slice()],
        ),
        amount.clone(),
    )
    .unwrap();

    assign!(user_position.borrow_mut().borrowed_amount, new_debt);

    assign!(user_position.borrow_mut().last_update_slot, current_slot);

    assign!(
        pool.borrow_mut().total_borrowed,
        pool.borrow().total_borrowed + amount
    );

    assign!(pool.borrow_mut().last_update_slot, current_slot);
}

pub fn create_user_position_handler<'info>(
    mut user: SeahorseSigner<'info, '_>,
    mut pool: Mutable<LoadedPool<'info, '_>>,
    mut user_position: Empty<Mutable<LoadedUserPosition<'info, '_>>>,
    mut collateral_mint: SeahorseAccount<'info, '_, Mint>,
    mut clock: Sysvar<'info, Clock>,
) -> () {
    let mut position_bump = user_position.bump.unwrap();
    let mut user_position = user_position.account.clone();
    let mut current_slot = clock.slot;

    assign!(user_position.borrow_mut().owner, user.key());

    assign!(
        user_position.borrow_mut().pool,
        pool.borrow().__account__.key()
    );

    assign!(user_position.borrow_mut().collateral_deposited, 0);

    assign!(user_position.borrow_mut().borrowed_amount, 0);

    assign!(user_position.borrow_mut().last_update_slot, current_slot);

    assign!(user_position.borrow_mut().bump, position_bump);
}

pub fn deposit_collateral_handler<'info>(
    mut user: SeahorseSigner<'info, '_>,
    mut pool: Mutable<LoadedPool<'info, '_>>,
    mut user_position: Mutable<LoadedUserPosition<'info, '_>>,
    mut user_collateral_token: SeahorseAccount<'info, '_, TokenAccount>,
    mut collateral_vault: SeahorseAccount<'info, '_, TokenAccount>,
    mut collateral_mint: SeahorseAccount<'info, '_, Mint>,
    mut amount: u64,
) -> () {
    if !(amount > 0) {
        panic!("Amount must be greater than zero");
    }

    if !(user.key() == user_position.borrow().owner) {
        panic!("Unauthorized");
    }

    token::transfer(
        CpiContext::new(
            user_collateral_token.programs.get("token_program"),
            token::Transfer {
                from: user_collateral_token.to_account_info(),
                authority: user.clone().to_account_info(),
                to: collateral_vault.clone().to_account_info(),
            },
        ),
        amount.clone(),
    )
    .unwrap();

    assign!(
        user_position.borrow_mut().collateral_deposited,
        user_position.borrow().collateral_deposited + amount
    );

    assign!(
        pool.borrow_mut().total_deposited,
        pool.borrow().total_deposited + amount
    );
}

pub fn initialize_pool_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut collateral_mint: SeahorseAccount<'info, '_, Mint>,
    mut borrow_mint: SeahorseAccount<'info, '_, Mint>,
    mut pool: Empty<Mutable<LoadedPool<'info, '_>>>,
    mut collateral_vault: Empty<SeahorseAccount<'info, '_, TokenAccount>>,
    mut borrow_vault: Empty<SeahorseAccount<'info, '_, TokenAccount>>,
    mut clock: Sysvar<'info, Clock>,
    mut interest_rate: u64,
) -> () {
    let mut pool_bump = pool.bump.unwrap();
    let mut pool = pool.account.clone();

    collateral_vault.account.clone();

    borrow_vault.account.clone();

    let mut current_slot = clock.slot;

    assign!(pool.borrow_mut().authority, authority.key());

    assign!(pool.borrow_mut().collateral_mint, collateral_mint.key());

    assign!(pool.borrow_mut().borrow_mint, borrow_mint.key());

    assign!(
        pool.borrow_mut().collateral_vault,
        collateral_vault.account.key()
    );

    assign!(pool.borrow_mut().borrow_vault, borrow_vault.account.key());

    assign!(pool.borrow_mut().interest_rate, interest_rate);

    assign!(pool.borrow_mut().total_deposited, 0);

    assign!(pool.borrow_mut().total_borrowed, 0);

    assign!(pool.borrow_mut().last_update_slot, current_slot);

    assign!(pool.borrow_mut().bump, pool_bump);
}

pub fn repay_handler<'info>(
    mut user: SeahorseSigner<'info, '_>,
    mut pool: Mutable<LoadedPool<'info, '_>>,
    mut user_position: Mutable<LoadedUserPosition<'info, '_>>,
    mut user_borrow_token: SeahorseAccount<'info, '_, TokenAccount>,
    mut borrow_vault: SeahorseAccount<'info, '_, TokenAccount>,
    mut collateral_mint: SeahorseAccount<'info, '_, Mint>,
    mut clock: Sysvar<'info, Clock>,
    mut amount: u64,
) -> () {
    if !(amount > 0) {
        panic!("Amount must be greater than zero");
    }

    if !(user.key() == user_position.borrow().owner) {
        panic!("Unauthorized");
    }

    let mut current_slot = clock.slot;
    let mut slots_elapsed = current_slot - user_position.borrow().last_update_slot;
    let mut accrued_interest =
        ((user_position.borrow().borrowed_amount * pool.borrow().interest_rate) * slots_elapsed)
            / INTEREST_SCALE!();

    let mut total_debt = user_position.borrow().borrowed_amount + accrued_interest;
    let mut repay_amount = amount;

    if repay_amount > total_debt {
        repay_amount = total_debt;
    }

    token::transfer(
        CpiContext::new(
            user_borrow_token.programs.get("token_program"),
            token::Transfer {
                from: user_borrow_token.to_account_info(),
                authority: user.clone().to_account_info(),
                to: borrow_vault.clone().to_account_info(),
            },
        ),
        repay_amount.clone(),
    )
    .unwrap();

    assign!(
        user_position.borrow_mut().borrowed_amount,
        total_debt - repay_amount
    );

    assign!(user_position.borrow_mut().last_update_slot, current_slot);

    if repay_amount <= pool.borrow().total_borrowed {
        assign!(
            pool.borrow_mut().total_borrowed,
            pool.borrow().total_borrowed - repay_amount
        );
    } else {
        assign!(pool.borrow_mut().total_borrowed, 0);
    }

    assign!(pool.borrow_mut().last_update_slot, current_slot);
}

pub fn withdraw_collateral_handler<'info>(
    mut user: SeahorseSigner<'info, '_>,
    mut pool: Mutable<LoadedPool<'info, '_>>,
    mut user_position: Mutable<LoadedUserPosition<'info, '_>>,
    mut user_collateral_token: SeahorseAccount<'info, '_, TokenAccount>,
    mut collateral_vault: SeahorseAccount<'info, '_, TokenAccount>,
    mut collateral_mint: SeahorseAccount<'info, '_, Mint>,
    mut clock: Sysvar<'info, Clock>,
    mut amount: u64,
) -> () {
    if !(amount > 0) {
        panic!("Amount must be greater than zero");
    }

    if !(user.key() == user_position.borrow().owner) {
        panic!("Unauthorized");
    }

    if !(user_position.borrow().collateral_deposited >= amount) {
        panic!("Insufficient collateral");
    }

    let mut current_slot = clock.slot;
    let mut slots_elapsed = current_slot - user_position.borrow().last_update_slot;
    let mut accrued_interest =
        ((user_position.borrow().borrowed_amount * pool.borrow().interest_rate) * slots_elapsed)
            / INTEREST_SCALE!();

    let mut total_debt = user_position.borrow().borrowed_amount + accrued_interest;
    let mut remaining_collateral = user_position.borrow().collateral_deposited - amount;

    if total_debt > 0 {
        let mut required_collateral = (total_debt * COLLATERAL_RATIO_BPS!()) / BPS_DENOMINATOR!();

        if !(remaining_collateral >= required_collateral) {
            panic!("Insufficient collateral after withdrawal");
        }
    }

    let mut pool_bump = pool.borrow().bump;

    token::transfer(
        CpiContext::new_with_signer(
            collateral_vault.programs.get("token_program"),
            token::Transfer {
                from: collateral_vault.to_account_info(),
                authority: pool.borrow().__account__.to_account_info(),
                to: user_collateral_token.clone().to_account_info(),
            },
            &[Mutable::new(vec![
                "pool".to_string().as_bytes().as_ref(),
                collateral_mint.key().as_ref(),
                pool_bump.to_le_bytes().as_ref(),
            ])
            .borrow()
            .as_slice()],
        ),
        amount.clone(),
    )
    .unwrap();

    assign!(
        user_position.borrow_mut().collateral_deposited,
        remaining_collateral
    );

    assign!(user_position.borrow_mut().borrowed_amount, total_debt);

    assign!(user_position.borrow_mut().last_update_slot, current_slot);

    assign!(
        pool.borrow_mut().total_deposited,
        pool.borrow().total_deposited - amount
    );

    assign!(pool.borrow_mut().last_update_slot, current_slot);
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

declare_id!("6LEndKVBqeRVFT6UYm5NN1SqBSCqCWvPaoYdJzrgLMge");

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
mod lending {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    # [instruction (amount : u64)]
    pub struct Borrow<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(mut)]
        pub pool: Box<Account<'info, dot::program::Pool>>,
        #[account(mut)]
        pub user_position: Box<Account<'info, dot::program::UserPosition>>,
        #[account(mut)]
        pub user_borrow_token: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub borrow_vault: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub collateral_mint: Box<Account<'info, Mint>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
        pub token_program: Program<'info, Token>,
    }

    pub fn borrow(ctx: Context<Borrow>, amount: u64) -> Result<()> {
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

        let pool = dot::program::Pool::load(&mut ctx.accounts.pool, &programs_map);
        let user_position =
            dot::program::UserPosition::load(&mut ctx.accounts.user_position, &programs_map);

        let user_borrow_token = SeahorseAccount {
            account: &ctx.accounts.user_borrow_token,
            programs: &programs_map,
        };

        let borrow_vault = SeahorseAccount {
            account: &ctx.accounts.borrow_vault,
            programs: &programs_map,
        };

        let collateral_mint = SeahorseAccount {
            account: &ctx.accounts.collateral_mint,
            programs: &programs_map,
        };

        let clock = &ctx.accounts.clock.clone();

        borrow_handler(
            user.clone(),
            pool.clone(),
            user_position.clone(),
            user_borrow_token.clone(),
            borrow_vault.clone(),
            collateral_mint.clone(),
            clock.clone(),
            amount,
        );

        dot::program::Pool::store(pool);

        dot::program::UserPosition::store(user_position);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct CreateUserPosition<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(mut)]
        pub pool: Box<Account<'info, dot::program::Pool>>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: UserPosition > () + 8 , payer = user , seeds = ["user_position" . as_bytes () . as_ref () , collateral_mint . key () . as_ref () , user . key () . as_ref ()] , bump)]
        pub user_position: Box<Account<'info, dot::program::UserPosition>>,
        #[account(mut)]
        pub collateral_mint: Box<Account<'info, Mint>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn create_user_position(ctx: Context<CreateUserPosition>) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "system_program",
            ctx.accounts.system_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let user = SeahorseSigner {
            account: &ctx.accounts.user,
            programs: &programs_map,
        };

        let pool = dot::program::Pool::load(&mut ctx.accounts.pool, &programs_map);
        let user_position = Empty {
            account: dot::program::UserPosition::load(
                &mut ctx.accounts.user_position,
                &programs_map,
            ),
            bump: Some(ctx.bumps.user_position),
        };

        let collateral_mint = SeahorseAccount {
            account: &ctx.accounts.collateral_mint,
            programs: &programs_map,
        };

        let clock = &ctx.accounts.clock.clone();

        create_user_position_handler(
            user.clone(),
            pool.clone(),
            user_position.clone(),
            collateral_mint.clone(),
            clock.clone(),
        );

        dot::program::Pool::store(pool);

        dot::program::UserPosition::store(user_position.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (amount : u64)]
    pub struct DepositCollateral<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(mut)]
        pub pool: Box<Account<'info, dot::program::Pool>>,
        #[account(mut)]
        pub user_position: Box<Account<'info, dot::program::UserPosition>>,
        #[account(mut)]
        pub user_collateral_token: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub collateral_vault: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub collateral_mint: Box<Account<'info, Mint>>,
        pub token_program: Program<'info, Token>,
    }

    pub fn deposit_collateral(ctx: Context<DepositCollateral>, amount: u64) -> Result<()> {
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

        let pool = dot::program::Pool::load(&mut ctx.accounts.pool, &programs_map);
        let user_position =
            dot::program::UserPosition::load(&mut ctx.accounts.user_position, &programs_map);

        let user_collateral_token = SeahorseAccount {
            account: &ctx.accounts.user_collateral_token,
            programs: &programs_map,
        };

        let collateral_vault = SeahorseAccount {
            account: &ctx.accounts.collateral_vault,
            programs: &programs_map,
        };

        let collateral_mint = SeahorseAccount {
            account: &ctx.accounts.collateral_mint,
            programs: &programs_map,
        };

        deposit_collateral_handler(
            user.clone(),
            pool.clone(),
            user_position.clone(),
            user_collateral_token.clone(),
            collateral_vault.clone(),
            collateral_mint.clone(),
            amount,
        );

        dot::program::Pool::store(pool);

        dot::program::UserPosition::store(user_position);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (interest_rate : u64)]
    pub struct InitializePool<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub collateral_mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub borrow_mint: Box<Account<'info, Mint>>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: Pool > () + 8 , payer = authority , seeds = ["pool" . as_bytes () . as_ref () , collateral_mint . key () . as_ref ()] , bump)]
        pub pool: Box<Account<'info, dot::program::Pool>>,
        # [account (init , payer = authority , seeds = ["collateral_vault" . as_bytes () . as_ref () , collateral_mint . key () . as_ref ()] , bump , token :: mint = collateral_mint , token :: authority = pool)]
        pub collateral_vault: Box<Account<'info, TokenAccount>>,
        # [account (init , payer = authority , seeds = ["borrow_vault" . as_bytes () . as_ref () , collateral_mint . key () . as_ref ()] , bump , token :: mint = borrow_mint , token :: authority = pool)]
        pub borrow_vault: Box<Account<'info, TokenAccount>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
        pub token_program: Program<'info, Token>,
    }

    pub fn initialize_pool(ctx: Context<InitializePool>, interest_rate: u64) -> Result<()> {
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
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let collateral_mint = SeahorseAccount {
            account: &ctx.accounts.collateral_mint,
            programs: &programs_map,
        };

        let borrow_mint = SeahorseAccount {
            account: &ctx.accounts.borrow_mint,
            programs: &programs_map,
        };

        let pool = Empty {
            account: dot::program::Pool::load(&mut ctx.accounts.pool, &programs_map),
            bump: Some(ctx.bumps.pool),
        };

        let collateral_vault = Empty {
            account: SeahorseAccount {
                account: &ctx.accounts.collateral_vault,
                programs: &programs_map,
            },
            bump: Some(ctx.bumps.collateral_vault),
        };

        let borrow_vault = Empty {
            account: SeahorseAccount {
                account: &ctx.accounts.borrow_vault,
                programs: &programs_map,
            },
            bump: Some(ctx.bumps.borrow_vault),
        };

        let clock = &ctx.accounts.clock.clone();

        initialize_pool_handler(
            authority.clone(),
            collateral_mint.clone(),
            borrow_mint.clone(),
            pool.clone(),
            collateral_vault.clone(),
            borrow_vault.clone(),
            clock.clone(),
            interest_rate,
        );

        dot::program::Pool::store(pool.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (amount : u64)]
    pub struct Repay<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(mut)]
        pub pool: Box<Account<'info, dot::program::Pool>>,
        #[account(mut)]
        pub user_position: Box<Account<'info, dot::program::UserPosition>>,
        #[account(mut)]
        pub user_borrow_token: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub borrow_vault: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub collateral_mint: Box<Account<'info, Mint>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
        pub token_program: Program<'info, Token>,
    }

    pub fn repay(ctx: Context<Repay>, amount: u64) -> Result<()> {
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

        let pool = dot::program::Pool::load(&mut ctx.accounts.pool, &programs_map);
        let user_position =
            dot::program::UserPosition::load(&mut ctx.accounts.user_position, &programs_map);

        let user_borrow_token = SeahorseAccount {
            account: &ctx.accounts.user_borrow_token,
            programs: &programs_map,
        };

        let borrow_vault = SeahorseAccount {
            account: &ctx.accounts.borrow_vault,
            programs: &programs_map,
        };

        let collateral_mint = SeahorseAccount {
            account: &ctx.accounts.collateral_mint,
            programs: &programs_map,
        };

        let clock = &ctx.accounts.clock.clone();

        repay_handler(
            user.clone(),
            pool.clone(),
            user_position.clone(),
            user_borrow_token.clone(),
            borrow_vault.clone(),
            collateral_mint.clone(),
            clock.clone(),
            amount,
        );

        dot::program::Pool::store(pool);

        dot::program::UserPosition::store(user_position);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (amount : u64)]
    pub struct WithdrawCollateral<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(mut)]
        pub pool: Box<Account<'info, dot::program::Pool>>,
        #[account(mut)]
        pub user_position: Box<Account<'info, dot::program::UserPosition>>,
        #[account(mut)]
        pub user_collateral_token: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub collateral_vault: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub collateral_mint: Box<Account<'info, Mint>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
        pub token_program: Program<'info, Token>,
    }

    pub fn withdraw_collateral(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()> {
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

        let pool = dot::program::Pool::load(&mut ctx.accounts.pool, &programs_map);
        let user_position =
            dot::program::UserPosition::load(&mut ctx.accounts.user_position, &programs_map);

        let user_collateral_token = SeahorseAccount {
            account: &ctx.accounts.user_collateral_token,
            programs: &programs_map,
        };

        let collateral_vault = SeahorseAccount {
            account: &ctx.accounts.collateral_vault,
            programs: &programs_map,
        };

        let collateral_mint = SeahorseAccount {
            account: &ctx.accounts.collateral_mint,
            programs: &programs_map,
        };

        let clock = &ctx.accounts.clock.clone();

        withdraw_collateral_handler(
            user.clone(),
            pool.clone(),
            user_position.clone(),
            user_collateral_token.clone(),
            collateral_vault.clone(),
            collateral_mint.clone(),
            clock.clone(),
            amount,
        );

        dot::program::Pool::store(pool);

        dot::program::UserPosition::store(user_position);

        return Ok(());
    }
}

