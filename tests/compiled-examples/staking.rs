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

seahorse_const! { REWARD_SCALE , 1000000 }

#[account]
#[derive(Debug)]
pub struct Pool {
    pub authority: Pubkey,
    pub stake_mint: Pubkey,
    pub reward_mint: Pubkey,
    pub stake_vault: Pubkey,
    pub reward_rate: u64,
    pub total_staked: u64,
    pub last_update_slot: u64,
    pub bump: u8,
}

impl<'info, 'entrypoint> Pool {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedPool<'info, 'entrypoint>> {
        let authority = account.authority.clone();
        let stake_mint = account.stake_mint.clone();
        let reward_mint = account.reward_mint.clone();
        let stake_vault = account.stake_vault.clone();
        let reward_rate = account.reward_rate;
        let total_staked = account.total_staked;
        let last_update_slot = account.last_update_slot;
        let bump = account.bump;

        Mutable::new(LoadedPool {
            __account__: account,
            __programs__: programs_map,
            authority,
            stake_mint,
            reward_mint,
            stake_vault,
            reward_rate,
            total_staked,
            last_update_slot,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedPool>) {
        let mut loaded = loaded.borrow_mut();
        let authority = loaded.authority.clone();

        loaded.__account__.authority = authority;

        let stake_mint = loaded.stake_mint.clone();

        loaded.__account__.stake_mint = stake_mint;

        let reward_mint = loaded.reward_mint.clone();

        loaded.__account__.reward_mint = reward_mint;

        let stake_vault = loaded.stake_vault.clone();

        loaded.__account__.stake_vault = stake_vault;

        let reward_rate = loaded.reward_rate;

        loaded.__account__.reward_rate = reward_rate;

        let total_staked = loaded.total_staked;

        loaded.__account__.total_staked = total_staked;

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
    pub stake_mint: Pubkey,
    pub reward_mint: Pubkey,
    pub stake_vault: Pubkey,
    pub reward_rate: u64,
    pub total_staked: u64,
    pub last_update_slot: u64,
    pub bump: u8,
}

#[account]
#[derive(Debug)]
pub struct UserStake {
    pub owner: Pubkey,
    pub pool: Pubkey,
    pub staked_amount: u64,
    pub pending_rewards: u64,
    pub last_stake_slot: u64,
    pub bump: u8,
}

impl<'info, 'entrypoint> UserStake {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedUserStake<'info, 'entrypoint>> {
        let owner = account.owner.clone();
        let pool = account.pool.clone();
        let staked_amount = account.staked_amount;
        let pending_rewards = account.pending_rewards;
        let last_stake_slot = account.last_stake_slot;
        let bump = account.bump;

        Mutable::new(LoadedUserStake {
            __account__: account,
            __programs__: programs_map,
            owner,
            pool,
            staked_amount,
            pending_rewards,
            last_stake_slot,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedUserStake>) {
        let mut loaded = loaded.borrow_mut();
        let owner = loaded.owner.clone();

        loaded.__account__.owner = owner;

        let pool = loaded.pool.clone();

        loaded.__account__.pool = pool;

        let staked_amount = loaded.staked_amount;

        loaded.__account__.staked_amount = staked_amount;

        let pending_rewards = loaded.pending_rewards;

        loaded.__account__.pending_rewards = pending_rewards;

        let last_stake_slot = loaded.last_stake_slot;

        loaded.__account__.last_stake_slot = last_stake_slot;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedUserStake<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, UserStake>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub owner: Pubkey,
    pub pool: Pubkey,
    pub staked_amount: u64,
    pub pending_rewards: u64,
    pub last_stake_slot: u64,
    pub bump: u8,
}

pub fn claim_rewards_handler<'info>(
    mut user: SeahorseSigner<'info, '_>,
    mut pool: Mutable<LoadedPool<'info, '_>>,
    mut reward_mint: SeahorseAccount<'info, '_, Mint>,
    mut user_stake: Mutable<LoadedUserStake<'info, '_>>,
    mut user_reward_token: SeahorseAccount<'info, '_, TokenAccount>,
    mut stake_mint: SeahorseAccount<'info, '_, Mint>,
    mut clock: Sysvar<'info, Clock>,
) -> () {
    if !(user.key() == user_stake.borrow().owner) {
        panic!("Unauthorized");
    }

    if !(user_stake.borrow().staked_amount > 0) {
        panic!("No tokens staked");
    }

    let mut current_slot = clock.slot;
    let mut slots_staked = current_slot - user_stake.borrow().last_stake_slot;
    let mut pending = ((user_stake.borrow().staked_amount * pool.borrow().reward_rate)
        * slots_staked)
        / REWARD_SCALE!();

    let mut total_rewards = user_stake.borrow().pending_rewards + pending;

    if !(total_rewards > 0) {
        panic!("No rewards to claim");
    }

    let mut pool_bump = pool.borrow().bump;

    token::mint_to(
        CpiContext::new_with_signer(
            reward_mint.programs.get("token_program"),
            token::MintTo {
                mint: reward_mint.to_account_info(),
                authority: pool.borrow().__account__.to_account_info(),
                to: user_reward_token.clone().to_account_info(),
            },
            &[Mutable::new(vec![
                "pool".to_string().as_bytes().as_ref(),
                stake_mint.key().as_ref(),
                pool_bump.to_le_bytes().as_ref(),
            ])
            .borrow()
            .as_slice()],
        ),
        total_rewards.clone(),
    )
    .unwrap();

    assign!(user_stake.borrow_mut().pending_rewards, 0);

    assign!(user_stake.borrow_mut().last_stake_slot, current_slot);
}

pub fn create_user_stake_handler<'info>(
    mut user: SeahorseSigner<'info, '_>,
    mut pool: Mutable<LoadedPool<'info, '_>>,
    mut user_stake: Empty<Mutable<LoadedUserStake<'info, '_>>>,
    mut stake_mint: SeahorseAccount<'info, '_, Mint>,
    mut clock: Sysvar<'info, Clock>,
) -> () {
    let mut stake_bump = user_stake.bump.unwrap();
    let mut user_stake = user_stake.account.clone();
    let mut current_slot = clock.slot;

    assign!(user_stake.borrow_mut().owner, user.key());

    assign!(
        user_stake.borrow_mut().pool,
        pool.borrow().__account__.key()
    );

    assign!(user_stake.borrow_mut().staked_amount, 0);

    assign!(user_stake.borrow_mut().pending_rewards, 0);

    assign!(user_stake.borrow_mut().last_stake_slot, current_slot);

    assign!(user_stake.borrow_mut().bump, stake_bump);
}

pub fn initialize_pool_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut stake_mint: SeahorseAccount<'info, '_, Mint>,
    mut reward_mint: SeahorseAccount<'info, '_, Mint>,
    mut pool: Empty<Mutable<LoadedPool<'info, '_>>>,
    mut stake_vault: Empty<SeahorseAccount<'info, '_, TokenAccount>>,
    mut clock: Sysvar<'info, Clock>,
    mut reward_rate: u64,
) -> () {
    let mut pool_bump = pool.bump.unwrap();
    let mut pool = pool.account.clone();

    stake_vault.account.clone();

    let mut current_slot = clock.slot;

    assign!(pool.borrow_mut().authority, authority.key());

    assign!(pool.borrow_mut().stake_mint, stake_mint.key());

    assign!(pool.borrow_mut().reward_mint, reward_mint.key());

    assign!(pool.borrow_mut().stake_vault, stake_vault.account.key());

    assign!(pool.borrow_mut().reward_rate, reward_rate);

    assign!(pool.borrow_mut().total_staked, 0);

    assign!(pool.borrow_mut().last_update_slot, current_slot);

    assign!(pool.borrow_mut().bump, pool_bump);
}

pub fn stake_handler<'info>(
    mut user: SeahorseSigner<'info, '_>,
    mut pool: Mutable<LoadedPool<'info, '_>>,
    mut user_stake: Mutable<LoadedUserStake<'info, '_>>,
    mut user_stake_token: SeahorseAccount<'info, '_, TokenAccount>,
    mut stake_vault: SeahorseAccount<'info, '_, TokenAccount>,
    mut stake_mint: SeahorseAccount<'info, '_, Mint>,
    mut clock: Sysvar<'info, Clock>,
    mut amount: u64,
) -> () {
    if !(amount > 0) {
        panic!("Amount must be greater than zero");
    }

    if !(user.key() == user_stake.borrow().owner) {
        panic!("Unauthorized");
    }

    let mut current_slot = clock.slot;

    if user_stake.borrow().staked_amount > 0 {
        let mut slots_staked = current_slot - user_stake.borrow().last_stake_slot;
        let mut pending = ((user_stake.borrow().staked_amount * pool.borrow().reward_rate)
            * slots_staked)
            / REWARD_SCALE!();

        assign!(
            user_stake.borrow_mut().pending_rewards,
            user_stake.borrow().pending_rewards + pending
        );
    }

    token::transfer(
        CpiContext::new(
            user_stake_token.programs.get("token_program"),
            token::Transfer {
                from: user_stake_token.to_account_info(),
                authority: user.clone().to_account_info(),
                to: stake_vault.clone().to_account_info(),
            },
        ),
        amount.clone(),
    )
    .unwrap();

    assign!(
        user_stake.borrow_mut().staked_amount,
        user_stake.borrow().staked_amount + amount
    );

    assign!(user_stake.borrow_mut().last_stake_slot, current_slot);

    assign!(
        pool.borrow_mut().total_staked,
        pool.borrow().total_staked + amount
    );

    assign!(pool.borrow_mut().last_update_slot, current_slot);
}

pub fn unstake_handler<'info>(
    mut user: SeahorseSigner<'info, '_>,
    mut pool: Mutable<LoadedPool<'info, '_>>,
    mut reward_mint: SeahorseAccount<'info, '_, Mint>,
    mut user_stake: Mutable<LoadedUserStake<'info, '_>>,
    mut user_stake_token: SeahorseAccount<'info, '_, TokenAccount>,
    mut user_reward_token: SeahorseAccount<'info, '_, TokenAccount>,
    mut stake_vault: SeahorseAccount<'info, '_, TokenAccount>,
    mut stake_mint: SeahorseAccount<'info, '_, Mint>,
    mut clock: Sysvar<'info, Clock>,
    mut amount: u64,
) -> () {
    if !(amount > 0) {
        panic!("Amount must be greater than zero");
    }

    if !(user.key() == user_stake.borrow().owner) {
        panic!("Unauthorized");
    }

    if !(user_stake.borrow().staked_amount >= amount) {
        panic!("Insufficient staked amount");
    }

    let mut current_slot = clock.slot;
    let mut slots_staked = current_slot - user_stake.borrow().last_stake_slot;
    let mut pending = ((user_stake.borrow().staked_amount * pool.borrow().reward_rate)
        * slots_staked)
        / REWARD_SCALE!();

    let mut total_rewards = user_stake.borrow().pending_rewards + pending;
    let mut pool_bump = pool.borrow().bump;

    token::transfer(
        CpiContext::new_with_signer(
            stake_vault.programs.get("token_program"),
            token::Transfer {
                from: stake_vault.to_account_info(),
                authority: pool.borrow().__account__.to_account_info(),
                to: user_stake_token.clone().to_account_info(),
            },
            &[Mutable::new(vec![
                "pool".to_string().as_bytes().as_ref(),
                stake_mint.key().as_ref(),
                pool_bump.to_le_bytes().as_ref(),
            ])
            .borrow()
            .as_slice()],
        ),
        amount.clone(),
    )
    .unwrap();

    if total_rewards > 0 {
        token::mint_to(
            CpiContext::new_with_signer(
                reward_mint.programs.get("token_program"),
                token::MintTo {
                    mint: reward_mint.to_account_info(),
                    authority: pool.borrow().__account__.to_account_info(),
                    to: user_reward_token.clone().to_account_info(),
                },
                &[Mutable::new(vec![
                    "pool".to_string().as_bytes().as_ref(),
                    stake_mint.key().as_ref(),
                    pool_bump.to_le_bytes().as_ref(),
                ])
                .borrow()
                .as_slice()],
            ),
            total_rewards.clone(),
        )
        .unwrap();
    }

    assign!(
        user_stake.borrow_mut().staked_amount,
        user_stake.borrow().staked_amount - amount
    );

    assign!(user_stake.borrow_mut().pending_rewards, 0);

    assign!(user_stake.borrow_mut().last_stake_slot, current_slot);

    assign!(
        pool.borrow_mut().total_staked,
        pool.borrow().total_staked - amount
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

declare_id!("HFQhM2FYhiP1mbpVFnpHZ7hJf5xFzekJAjqoxexvKA5Q");

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
mod staking {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    pub struct ClaimRewards<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(mut)]
        pub pool: Box<Account<'info, dot::program::Pool>>,
        #[account(mut)]
        pub reward_mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub user_stake: Box<Account<'info, dot::program::UserStake>>,
        #[account(mut)]
        pub user_reward_token: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub stake_mint: Box<Account<'info, Mint>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
        pub token_program: Program<'info, Token>,
    }

    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
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
        let reward_mint = SeahorseAccount {
            account: &ctx.accounts.reward_mint,
            programs: &programs_map,
        };

        let user_stake = dot::program::UserStake::load(&mut ctx.accounts.user_stake, &programs_map);
        let user_reward_token = SeahorseAccount {
            account: &ctx.accounts.user_reward_token,
            programs: &programs_map,
        };

        let stake_mint = SeahorseAccount {
            account: &ctx.accounts.stake_mint,
            programs: &programs_map,
        };

        let clock = &ctx.accounts.clock.clone();

        claim_rewards_handler(
            user.clone(),
            pool.clone(),
            reward_mint.clone(),
            user_stake.clone(),
            user_reward_token.clone(),
            stake_mint.clone(),
            clock.clone(),
        );

        dot::program::Pool::store(pool);

        dot::program::UserStake::store(user_stake);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct CreateUserStake<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(mut)]
        pub pool: Box<Account<'info, dot::program::Pool>>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: UserStake > () + 8 , payer = user , seeds = ["user_stake" . as_bytes () . as_ref () , stake_mint . key () . as_ref () , user . key () . as_ref ()] , bump)]
        pub user_stake: Box<Account<'info, dot::program::UserStake>>,
        #[account(mut)]
        pub stake_mint: Box<Account<'info, Mint>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn create_user_stake(ctx: Context<CreateUserStake>) -> Result<()> {
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
        let user_stake = Empty {
            account: dot::program::UserStake::load(&mut ctx.accounts.user_stake, &programs_map),
            bump: Some(ctx.bumps.user_stake),
        };

        let stake_mint = SeahorseAccount {
            account: &ctx.accounts.stake_mint,
            programs: &programs_map,
        };

        let clock = &ctx.accounts.clock.clone();

        create_user_stake_handler(
            user.clone(),
            pool.clone(),
            user_stake.clone(),
            stake_mint.clone(),
            clock.clone(),
        );

        dot::program::Pool::store(pool);

        dot::program::UserStake::store(user_stake.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (reward_rate : u64)]
    pub struct InitializePool<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub stake_mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub reward_mint: Box<Account<'info, Mint>>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: Pool > () + 8 , payer = authority , seeds = ["pool" . as_bytes () . as_ref () , stake_mint . key () . as_ref ()] , bump)]
        pub pool: Box<Account<'info, dot::program::Pool>>,
        # [account (init , payer = authority , seeds = ["stake_vault" . as_bytes () . as_ref () , stake_mint . key () . as_ref ()] , bump , token :: mint = stake_mint , token :: authority = pool)]
        pub stake_vault: Box<Account<'info, TokenAccount>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
        pub token_program: Program<'info, Token>,
    }

    pub fn initialize_pool(ctx: Context<InitializePool>, reward_rate: u64) -> Result<()> {
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

        let stake_mint = SeahorseAccount {
            account: &ctx.accounts.stake_mint,
            programs: &programs_map,
        };

        let reward_mint = SeahorseAccount {
            account: &ctx.accounts.reward_mint,
            programs: &programs_map,
        };

        let pool = Empty {
            account: dot::program::Pool::load(&mut ctx.accounts.pool, &programs_map),
            bump: Some(ctx.bumps.pool),
        };

        let stake_vault = Empty {
            account: SeahorseAccount {
                account: &ctx.accounts.stake_vault,
                programs: &programs_map,
            },
            bump: Some(ctx.bumps.stake_vault),
        };

        let clock = &ctx.accounts.clock.clone();

        initialize_pool_handler(
            authority.clone(),
            stake_mint.clone(),
            reward_mint.clone(),
            pool.clone(),
            stake_vault.clone(),
            clock.clone(),
            reward_rate,
        );

        dot::program::Pool::store(pool.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (amount : u64)]
    pub struct Stake<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(mut)]
        pub pool: Box<Account<'info, dot::program::Pool>>,
        #[account(mut)]
        pub user_stake: Box<Account<'info, dot::program::UserStake>>,
        #[account(mut)]
        pub user_stake_token: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub stake_vault: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub stake_mint: Box<Account<'info, Mint>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
        pub token_program: Program<'info, Token>,
    }

    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
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
        let user_stake = dot::program::UserStake::load(&mut ctx.accounts.user_stake, &programs_map);
        let user_stake_token = SeahorseAccount {
            account: &ctx.accounts.user_stake_token,
            programs: &programs_map,
        };

        let stake_vault = SeahorseAccount {
            account: &ctx.accounts.stake_vault,
            programs: &programs_map,
        };

        let stake_mint = SeahorseAccount {
            account: &ctx.accounts.stake_mint,
            programs: &programs_map,
        };

        let clock = &ctx.accounts.clock.clone();

        stake_handler(
            user.clone(),
            pool.clone(),
            user_stake.clone(),
            user_stake_token.clone(),
            stake_vault.clone(),
            stake_mint.clone(),
            clock.clone(),
            amount,
        );

        dot::program::Pool::store(pool);

        dot::program::UserStake::store(user_stake);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (amount : u64)]
    pub struct Unstake<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(mut)]
        pub pool: Box<Account<'info, dot::program::Pool>>,
        #[account(mut)]
        pub reward_mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub user_stake: Box<Account<'info, dot::program::UserStake>>,
        #[account(mut)]
        pub user_stake_token: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub user_reward_token: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub stake_vault: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub stake_mint: Box<Account<'info, Mint>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
        pub token_program: Program<'info, Token>,
    }

    pub fn unstake(ctx: Context<Unstake>, amount: u64) -> Result<()> {
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
        let reward_mint = SeahorseAccount {
            account: &ctx.accounts.reward_mint,
            programs: &programs_map,
        };

        let user_stake = dot::program::UserStake::load(&mut ctx.accounts.user_stake, &programs_map);
        let user_stake_token = SeahorseAccount {
            account: &ctx.accounts.user_stake_token,
            programs: &programs_map,
        };

        let user_reward_token = SeahorseAccount {
            account: &ctx.accounts.user_reward_token,
            programs: &programs_map,
        };

        let stake_vault = SeahorseAccount {
            account: &ctx.accounts.stake_vault,
            programs: &programs_map,
        };

        let stake_mint = SeahorseAccount {
            account: &ctx.accounts.stake_mint,
            programs: &programs_map,
        };

        let clock = &ctx.accounts.clock.clone();

        unstake_handler(
            user.clone(),
            pool.clone(),
            reward_mint.clone(),
            user_stake.clone(),
            user_stake_token.clone(),
            user_reward_token.clone(),
            stake_vault.clone(),
            stake_mint.clone(),
            clock.clone(),
            amount,
        );

        dot::program::Pool::store(pool);

        dot::program::UserStake::store(user_stake);

        return Ok(());
    }
}

