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
    pub reward_mint: Pubkey,
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
        let reward_mint = account.reward_mint.clone();
        let reward_rate = account.reward_rate;
        let total_staked = account.total_staked;
        let last_update_slot = account.last_update_slot;
        let bump = account.bump;

        Mutable::new(LoadedPool {
            __account__: account,
            __programs__: programs_map,
            authority,
            reward_mint,
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

        let reward_mint = loaded.reward_mint.clone();

        loaded.__account__.reward_mint = reward_mint;

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
    pub reward_mint: Pubkey,
    pub reward_rate: u64,
    pub total_staked: u64,
    pub last_update_slot: u64,
    pub bump: u8,
}

#[account]
#[derive(Debug)]
pub struct StakeRecord {
    pub owner: Pubkey,
    pub pool: Pubkey,
    pub nft_mint: Pubkey,
    pub nft_vault: Pubkey,
    pub staked_at_slot: u64,
    pub last_claim_slot: u64,
    pub pending_rewards: u64,
    pub is_staked: bool,
    pub bump: u8,
}

impl<'info, 'entrypoint> StakeRecord {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedStakeRecord<'info, 'entrypoint>> {
        let owner = account.owner.clone();
        let pool = account.pool.clone();
        let nft_mint = account.nft_mint.clone();
        let nft_vault = account.nft_vault.clone();
        let staked_at_slot = account.staked_at_slot;
        let last_claim_slot = account.last_claim_slot;
        let pending_rewards = account.pending_rewards;
        let is_staked = account.is_staked.clone();
        let bump = account.bump;

        Mutable::new(LoadedStakeRecord {
            __account__: account,
            __programs__: programs_map,
            owner,
            pool,
            nft_mint,
            nft_vault,
            staked_at_slot,
            last_claim_slot,
            pending_rewards,
            is_staked,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedStakeRecord>) {
        let mut loaded = loaded.borrow_mut();
        let owner = loaded.owner.clone();

        loaded.__account__.owner = owner;

        let pool = loaded.pool.clone();

        loaded.__account__.pool = pool;

        let nft_mint = loaded.nft_mint.clone();

        loaded.__account__.nft_mint = nft_mint;

        let nft_vault = loaded.nft_vault.clone();

        loaded.__account__.nft_vault = nft_vault;

        let staked_at_slot = loaded.staked_at_slot;

        loaded.__account__.staked_at_slot = staked_at_slot;

        let last_claim_slot = loaded.last_claim_slot;

        loaded.__account__.last_claim_slot = last_claim_slot;

        let pending_rewards = loaded.pending_rewards;

        loaded.__account__.pending_rewards = pending_rewards;

        let is_staked = loaded.is_staked.clone();

        loaded.__account__.is_staked = is_staked;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedStakeRecord<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, StakeRecord>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub owner: Pubkey,
    pub pool: Pubkey,
    pub nft_mint: Pubkey,
    pub nft_vault: Pubkey,
    pub staked_at_slot: u64,
    pub last_claim_slot: u64,
    pub pending_rewards: u64,
    pub is_staked: bool,
    pub bump: u8,
}

pub fn claim_rewards_handler<'info>(
    mut user: SeahorseSigner<'info, '_>,
    mut pool: Mutable<LoadedPool<'info, '_>>,
    mut reward_mint: SeahorseAccount<'info, '_, Mint>,
    mut nft_mint: SeahorseAccount<'info, '_, Mint>,
    mut stake_record: Mutable<LoadedStakeRecord<'info, '_>>,
    mut user_reward_token: SeahorseAccount<'info, '_, TokenAccount>,
    mut clock: Sysvar<'info, Clock>,
) -> () {
    if !stake_record.borrow().is_staked {
        panic!("NFT not staked");
    }

    if !(user.key() == stake_record.borrow().owner) {
        panic!("Unauthorized");
    }

    let mut current_slot = clock.slot;
    let mut slots_staked = current_slot - stake_record.borrow().last_claim_slot;
    let mut pending = (pool.borrow().reward_rate * slots_staked) / REWARD_SCALE!();
    let mut total_rewards = stake_record.borrow().pending_rewards + pending;

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
                reward_mint.key().as_ref(),
                pool_bump.to_le_bytes().as_ref(),
            ])
            .borrow()
            .as_slice()],
        ),
        total_rewards.clone(),
    )
    .unwrap();

    assign!(stake_record.borrow_mut().pending_rewards, 0);

    assign!(stake_record.borrow_mut().last_claim_slot, current_slot);
}

pub fn initialize_pool_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut reward_mint: SeahorseAccount<'info, '_, Mint>,
    mut pool: Empty<Mutable<LoadedPool<'info, '_>>>,
    mut clock: Sysvar<'info, Clock>,
    mut reward_rate: u64,
) -> () {
    let mut pool_bump = pool.bump.unwrap();
    let mut pool = pool.account.clone();
    let mut current_slot = clock.slot;

    assign!(pool.borrow_mut().authority, authority.key());

    assign!(pool.borrow_mut().reward_mint, reward_mint.key());

    assign!(pool.borrow_mut().reward_rate, reward_rate);

    assign!(pool.borrow_mut().total_staked, 0);

    assign!(pool.borrow_mut().last_update_slot, current_slot);

    assign!(pool.borrow_mut().bump, pool_bump);
}

pub fn stake_nft_handler<'info>(
    mut user: SeahorseSigner<'info, '_>,
    mut pool: Mutable<LoadedPool<'info, '_>>,
    mut nft_mint: SeahorseAccount<'info, '_, Mint>,
    mut user_nft_token: SeahorseAccount<'info, '_, TokenAccount>,
    mut nft_vault: Empty<SeahorseAccount<'info, '_, TokenAccount>>,
    mut stake_record: Empty<Mutable<LoadedStakeRecord<'info, '_>>>,
    mut clock: Sysvar<'info, Clock>,
) -> () {
    if !(user_nft_token.amount == 1) {
        panic!("User does not own the NFT");
    }

    let mut stake_bump = stake_record.bump.unwrap();
    let mut stake_record = stake_record.account.clone();
    let mut nft_vault = nft_vault.account.clone();
    let mut current_slot = clock.slot;

    assign!(stake_record.borrow_mut().owner, user.key());

    assign!(
        stake_record.borrow_mut().pool,
        pool.borrow().__account__.key()
    );

    assign!(stake_record.borrow_mut().nft_mint, nft_mint.key());

    assign!(stake_record.borrow_mut().nft_vault, nft_vault.key());

    assign!(stake_record.borrow_mut().staked_at_slot, current_slot);

    assign!(stake_record.borrow_mut().last_claim_slot, current_slot);

    assign!(stake_record.borrow_mut().pending_rewards, 0);

    assign!(stake_record.borrow_mut().is_staked, true);

    assign!(stake_record.borrow_mut().bump, stake_bump);

    token::transfer(
        CpiContext::new(
            user_nft_token.programs.get("token_program"),
            token::Transfer {
                from: user_nft_token.to_account_info(),
                authority: user.clone().to_account_info(),
                to: nft_vault.clone().to_account_info(),
            },
        ),
        <u64 as TryFrom<_>>::try_from(1).unwrap(),
    )
    .unwrap();

    assign!(
        pool.borrow_mut().total_staked,
        pool.borrow().total_staked + 1
    );

    assign!(pool.borrow_mut().last_update_slot, current_slot);
}

pub fn unstake_nft_handler<'info>(
    mut user: SeahorseSigner<'info, '_>,
    mut pool: Mutable<LoadedPool<'info, '_>>,
    mut reward_mint: SeahorseAccount<'info, '_, Mint>,
    mut nft_mint: SeahorseAccount<'info, '_, Mint>,
    mut stake_record: Mutable<LoadedStakeRecord<'info, '_>>,
    mut nft_vault: SeahorseAccount<'info, '_, TokenAccount>,
    mut user_nft_token: SeahorseAccount<'info, '_, TokenAccount>,
    mut user_reward_token: SeahorseAccount<'info, '_, TokenAccount>,
    mut clock: Sysvar<'info, Clock>,
) -> () {
    if !stake_record.borrow().is_staked {
        panic!("NFT not staked");
    }

    if !(user.key() == stake_record.borrow().owner) {
        panic!("Unauthorized");
    }

    let mut current_slot = clock.slot;
    let mut slots_staked = current_slot - stake_record.borrow().last_claim_slot;
    let mut pending = (pool.borrow().reward_rate * slots_staked) / REWARD_SCALE!();
    let mut total_rewards = stake_record.borrow().pending_rewards + pending;
    let mut stake_bump = stake_record.borrow().bump;

    token::transfer(
        CpiContext::new_with_signer(
            nft_vault.programs.get("token_program"),
            token::Transfer {
                from: nft_vault.to_account_info(),
                authority: stake_record.borrow().__account__.to_account_info(),
                to: user_nft_token.clone().to_account_info(),
            },
            &[Mutable::new(vec![
                "stake_record".to_string().as_bytes().as_ref(),
                nft_mint.key().as_ref(),
                stake_bump.to_le_bytes().as_ref(),
            ])
            .borrow()
            .as_slice()],
        ),
        <u64 as TryFrom<_>>::try_from(1).unwrap(),
    )
    .unwrap();

    if total_rewards > 0 {
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
                    reward_mint.key().as_ref(),
                    pool_bump.to_le_bytes().as_ref(),
                ])
                .borrow()
                .as_slice()],
            ),
            total_rewards.clone(),
        )
        .unwrap();
    }

    assign!(stake_record.borrow_mut().is_staked, false);

    assign!(stake_record.borrow_mut().pending_rewards, 0);

    assign!(stake_record.borrow_mut().last_claim_slot, current_slot);

    assign!(
        pool.borrow_mut().total_staked,
        pool.borrow().total_staked - 1
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

declare_id!("8DtVbTgYbcM8b2igVzb7RjbVKLLLDQ2bzXriSFp8fxhb");

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
mod nft_staking {
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
        pub nft_mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub stake_record: Box<Account<'info, dot::program::StakeRecord>>,
        #[account(mut)]
        pub user_reward_token: Box<Account<'info, TokenAccount>>,
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

        let nft_mint = SeahorseAccount {
            account: &ctx.accounts.nft_mint,
            programs: &programs_map,
        };

        let stake_record =
            dot::program::StakeRecord::load(&mut ctx.accounts.stake_record, &programs_map);

        let user_reward_token = SeahorseAccount {
            account: &ctx.accounts.user_reward_token,
            programs: &programs_map,
        };

        let clock = &ctx.accounts.clock.clone();

        claim_rewards_handler(
            user.clone(),
            pool.clone(),
            reward_mint.clone(),
            nft_mint.clone(),
            stake_record.clone(),
            user_reward_token.clone(),
            clock.clone(),
        );

        dot::program::Pool::store(pool);

        dot::program::StakeRecord::store(stake_record);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (reward_rate : u64)]
    pub struct InitializePool<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub reward_mint: Box<Account<'info, Mint>>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: Pool > () + 8 , payer = authority , seeds = ["pool" . as_bytes () . as_ref () , reward_mint . key () . as_ref ()] , bump)]
        pub pool: Box<Account<'info, dot::program::Pool>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn initialize_pool(ctx: Context<InitializePool>, reward_rate: u64) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "system_program",
            ctx.accounts.system_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
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

        let clock = &ctx.accounts.clock.clone();

        initialize_pool_handler(
            authority.clone(),
            reward_mint.clone(),
            pool.clone(),
            clock.clone(),
            reward_rate,
        );

        dot::program::Pool::store(pool.account);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct StakeNft<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(mut)]
        pub pool: Box<Account<'info, dot::program::Pool>>,
        #[account(mut)]
        pub nft_mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub user_nft_token: Box<Account<'info, TokenAccount>>,
        # [account (init , payer = user , seeds = ["nft_vault" . as_bytes () . as_ref () , nft_mint . key () . as_ref ()] , bump , token :: mint = nft_mint , token :: authority = stake_record)]
        pub nft_vault: Box<Account<'info, TokenAccount>>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: StakeRecord > () + 8 , payer = user , seeds = ["stake_record" . as_bytes () . as_ref () , nft_mint . key () . as_ref ()] , bump)]
        pub stake_record: Box<Account<'info, dot::program::StakeRecord>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
        pub token_program: Program<'info, Token>,
    }

    pub fn stake_nft(ctx: Context<StakeNft>) -> Result<()> {
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
        let user = SeahorseSigner {
            account: &ctx.accounts.user,
            programs: &programs_map,
        };

        let pool = dot::program::Pool::load(&mut ctx.accounts.pool, &programs_map);
        let nft_mint = SeahorseAccount {
            account: &ctx.accounts.nft_mint,
            programs: &programs_map,
        };

        let user_nft_token = SeahorseAccount {
            account: &ctx.accounts.user_nft_token,
            programs: &programs_map,
        };

        let nft_vault = Empty {
            account: SeahorseAccount {
                account: &ctx.accounts.nft_vault,
                programs: &programs_map,
            },
            bump: Some(ctx.bumps.nft_vault),
        };

        let stake_record = Empty {
            account: dot::program::StakeRecord::load(&mut ctx.accounts.stake_record, &programs_map),
            bump: Some(ctx.bumps.stake_record),
        };

        let clock = &ctx.accounts.clock.clone();

        stake_nft_handler(
            user.clone(),
            pool.clone(),
            nft_mint.clone(),
            user_nft_token.clone(),
            nft_vault.clone(),
            stake_record.clone(),
            clock.clone(),
        );

        dot::program::Pool::store(pool);

        dot::program::StakeRecord::store(stake_record.account);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct UnstakeNft<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(mut)]
        pub pool: Box<Account<'info, dot::program::Pool>>,
        #[account(mut)]
        pub reward_mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub nft_mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub stake_record: Box<Account<'info, dot::program::StakeRecord>>,
        #[account(mut)]
        pub nft_vault: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub user_nft_token: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub user_reward_token: Box<Account<'info, TokenAccount>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
        pub token_program: Program<'info, Token>,
    }

    pub fn unstake_nft(ctx: Context<UnstakeNft>) -> Result<()> {
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

        let nft_mint = SeahorseAccount {
            account: &ctx.accounts.nft_mint,
            programs: &programs_map,
        };

        let stake_record =
            dot::program::StakeRecord::load(&mut ctx.accounts.stake_record, &programs_map);

        let nft_vault = SeahorseAccount {
            account: &ctx.accounts.nft_vault,
            programs: &programs_map,
        };

        let user_nft_token = SeahorseAccount {
            account: &ctx.accounts.user_nft_token,
            programs: &programs_map,
        };

        let user_reward_token = SeahorseAccount {
            account: &ctx.accounts.user_reward_token,
            programs: &programs_map,
        };

        let clock = &ctx.accounts.clock.clone();

        unstake_nft_handler(
            user.clone(),
            pool.clone(),
            reward_mint.clone(),
            nft_mint.clone(),
            stake_record.clone(),
            nft_vault.clone(),
            user_nft_token.clone(),
            user_reward_token.clone(),
            clock.clone(),
        );

        dot::program::Pool::store(pool);

        dot::program::StakeRecord::store(stake_record);

        return Ok(());
    }
}

