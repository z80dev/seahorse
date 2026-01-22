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

#[account]
#[derive(Debug)]
pub struct DataRecord {
    pub creator: Pubkey,
    pub category: String,
    pub record_id: u64,
    pub data: String,
    pub bump: u8,
}

impl<'info, 'entrypoint> DataRecord {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedDataRecord<'info, 'entrypoint>> {
        let creator = account.creator.clone();
        let category = account.category.clone();
        let record_id = account.record_id;
        let data = account.data.clone();
        let bump = account.bump;

        Mutable::new(LoadedDataRecord {
            __account__: account,
            __programs__: programs_map,
            creator,
            category,
            record_id,
            data,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedDataRecord>) {
        let mut loaded = loaded.borrow_mut();
        let creator = loaded.creator.clone();

        loaded.__account__.creator = creator;

        let category = loaded.category.clone();

        loaded.__account__.category = category;

        let record_id = loaded.record_id;

        loaded.__account__.record_id = record_id;

        let data = loaded.data.clone();

        loaded.__account__.data = data;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedDataRecord<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, DataRecord>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub creator: Pubkey,
    pub category: String,
    pub record_id: u64,
    pub data: String,
    pub bump: u8,
}

#[account]
#[derive(Debug)]
pub struct GlobalConfig {
    pub admin: Pubkey,
    pub counter: u64,
    pub bump: u8,
}

impl<'info, 'entrypoint> GlobalConfig {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedGlobalConfig<'info, 'entrypoint>> {
        let admin = account.admin.clone();
        let counter = account.counter;
        let bump = account.bump;

        Mutable::new(LoadedGlobalConfig {
            __account__: account,
            __programs__: programs_map,
            admin,
            counter,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedGlobalConfig>) {
        let mut loaded = loaded.borrow_mut();
        let admin = loaded.admin.clone();

        loaded.__account__.admin = admin;

        let counter = loaded.counter;

        loaded.__account__.counter = counter;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedGlobalConfig<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, GlobalConfig>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub admin: Pubkey,
    pub counter: u64,
    pub bump: u8,
}

#[account]
#[derive(Debug)]
pub struct UserProfile {
    pub owner: Pubkey,
    pub name: String,
    pub visits: u64,
    pub bump: u8,
}

impl<'info, 'entrypoint> UserProfile {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedUserProfile<'info, 'entrypoint>> {
        let owner = account.owner.clone();
        let name = account.name.clone();
        let visits = account.visits;
        let bump = account.bump;

        Mutable::new(LoadedUserProfile {
            __account__: account,
            __programs__: programs_map,
            owner,
            name,
            visits,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedUserProfile>) {
        let mut loaded = loaded.borrow_mut();
        let owner = loaded.owner.clone();

        loaded.__account__.owner = owner;

        let name = loaded.name.clone();

        loaded.__account__.name = name;

        let visits = loaded.visits;

        loaded.__account__.visits = visits;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedUserProfile<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, UserProfile>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub owner: Pubkey,
    pub name: String,
    pub visits: u64,
    pub bump: u8,
}

#[account]
#[derive(Debug)]
pub struct Vault {
    pub owner: Pubkey,
    pub vault_id: u64,
    pub balance: u64,
    pub bump: u8,
}

impl<'info, 'entrypoint> Vault {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedVault<'info, 'entrypoint>> {
        let owner = account.owner.clone();
        let vault_id = account.vault_id;
        let balance = account.balance;
        let bump = account.bump;

        Mutable::new(LoadedVault {
            __account__: account,
            __programs__: programs_map,
            owner,
            vault_id,
            balance,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedVault>) {
        let mut loaded = loaded.borrow_mut();
        let owner = loaded.owner.clone();

        loaded.__account__.owner = owner;

        let vault_id = loaded.vault_id;

        loaded.__account__.vault_id = vault_id;

        let balance = loaded.balance;

        loaded.__account__.balance = balance;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedVault<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, Vault>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub owner: Pubkey,
    pub vault_id: u64,
    pub balance: u64,
    pub bump: u8,
}

pub fn deposit_to_vault_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut vault: Mutable<LoadedVault<'info, '_>>,
    mut amount: u64,
) -> () {
    if !(owner.key() == vault.borrow().owner) {
        panic!("Unauthorized");
    }

    assign!(vault.borrow_mut().balance, vault.borrow().balance + amount);
}

pub fn init_data_record_handler<'info>(
    mut creator: SeahorseSigner<'info, '_>,
    mut record: Empty<Mutable<LoadedDataRecord<'info, '_>>>,
    mut category: String,
    mut record_id: u64,
    mut data: String,
) -> () {
    let mut bump = record.bump.unwrap();
    let mut record = record.account.clone();

    assign!(record.borrow_mut().creator, creator.key());

    assign!(record.borrow_mut().category, category.clone());

    assign!(record.borrow_mut().record_id, record_id);

    assign!(record.borrow_mut().data, data.clone());

    assign!(record.borrow_mut().bump, bump);
}

pub fn init_global_config_handler<'info>(
    mut admin: SeahorseSigner<'info, '_>,
    mut config: Empty<Mutable<LoadedGlobalConfig<'info, '_>>>,
) -> () {
    let mut bump = config.bump.unwrap();
    let mut config = config.account.clone();

    assign!(config.borrow_mut().admin, admin.key());

    assign!(config.borrow_mut().counter, 0);

    assign!(config.borrow_mut().bump, bump);
}

pub fn init_user_profile_handler<'info>(
    mut user: SeahorseSigner<'info, '_>,
    mut profile: Empty<Mutable<LoadedUserProfile<'info, '_>>>,
    mut name: String,
) -> () {
    let mut bump = profile.bump.unwrap();
    let mut profile = profile.account.clone();

    assign!(profile.borrow_mut().owner, user.key());

    assign!(profile.borrow_mut().name, name.clone());

    assign!(profile.borrow_mut().visits, 0);

    assign!(profile.borrow_mut().bump, bump);
}

pub fn init_vault_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut vault: Empty<Mutable<LoadedVault<'info, '_>>>,
    mut vault_id: u64,
) -> () {
    let mut bump = vault.bump.unwrap();
    let mut vault = vault.account.clone();

    assign!(vault.borrow_mut().owner, owner.key());

    assign!(vault.borrow_mut().vault_id, vault_id);

    assign!(vault.borrow_mut().balance, 0);

    assign!(vault.borrow_mut().bump, bump);
}

pub fn update_data_record_handler<'info>(
    mut creator: SeahorseSigner<'info, '_>,
    mut record: Mutable<LoadedDataRecord<'info, '_>>,
    mut data: String,
) -> () {
    if !(creator.key() == record.borrow().creator) {
        panic!("Unauthorized");
    }

    assign!(record.borrow_mut().data, data.clone());
}

pub fn update_global_config_handler<'info>(
    mut admin: SeahorseSigner<'info, '_>,
    mut config: Mutable<LoadedGlobalConfig<'info, '_>>,
) -> () {
    if !(admin.key() == config.borrow().admin) {
        panic!("Unauthorized");
    }

    assign!(config.borrow_mut().counter, config.borrow().counter + 1);
}

pub fn visit_profile_handler<'info>(
    mut user: SeahorseSigner<'info, '_>,
    mut profile: Mutable<LoadedUserProfile<'info, '_>>,
) -> () {
    if !(user.key() == profile.borrow().owner) {
        panic!("Unauthorized");
    }

    assign!(profile.borrow_mut().visits, profile.borrow().visits + 1);
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

declare_id!("DxpKiPEUYHy6NKsnEMRvRKtC3ncrtP8epGt5KKS59Xzh");

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
mod pda_patterns {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    # [instruction (amount : u64)]
    pub struct DepositToVault<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        #[account(mut)]
        pub vault: Box<Account<'info, dot::program::Vault>>,
    }

    pub fn deposit_to_vault(ctx: Context<DepositToVault>, amount: u64) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let owner = SeahorseSigner {
            account: &ctx.accounts.owner,
            programs: &programs_map,
        };

        let vault = dot::program::Vault::load(&mut ctx.accounts.vault, &programs_map);

        deposit_to_vault_handler(owner.clone(), vault.clone(), amount);

        dot::program::Vault::store(vault);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (category : String , record_id : u64 , data : String)]
    pub struct InitDataRecord<'info> {
        #[account(mut)]
        pub creator: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: DataRecord > () + 8 , payer = creator , seeds = ["data_record" . as_bytes () . as_ref () , category . as_bytes () . as_ref () , record_id . to_le_bytes () . as_ref ()] , bump)]
        pub record: Box<Account<'info, dot::program::DataRecord>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn init_data_record(
        ctx: Context<InitDataRecord>,
        category: String,
        record_id: u64,
        data: String,
    ) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "system_program",
            ctx.accounts.system_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let creator = SeahorseSigner {
            account: &ctx.accounts.creator,
            programs: &programs_map,
        };

        let record = Empty {
            account: dot::program::DataRecord::load(&mut ctx.accounts.record, &programs_map),
            bump: Some(ctx.bumps.record),
        };

        init_data_record_handler(creator.clone(), record.clone(), category, record_id, data);

        dot::program::DataRecord::store(record.account);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct InitGlobalConfig<'info> {
        #[account(mut)]
        pub admin: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: GlobalConfig > () + 8 , payer = admin , seeds = ["global_config" . as_bytes () . as_ref ()] , bump)]
        pub config: Box<Account<'info, dot::program::GlobalConfig>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn init_global_config(ctx: Context<InitGlobalConfig>) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "system_program",
            ctx.accounts.system_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let admin = SeahorseSigner {
            account: &ctx.accounts.admin,
            programs: &programs_map,
        };

        let config = Empty {
            account: dot::program::GlobalConfig::load(&mut ctx.accounts.config, &programs_map),
            bump: Some(ctx.bumps.config),
        };

        init_global_config_handler(admin.clone(), config.clone());

        dot::program::GlobalConfig::store(config.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (name : String)]
    pub struct InitUserProfile<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: UserProfile > () + 8 , payer = user , seeds = ["user_profile" . as_bytes () . as_ref () , user . key () . as_ref ()] , bump)]
        pub profile: Box<Account<'info, dot::program::UserProfile>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn init_user_profile(ctx: Context<InitUserProfile>, name: String) -> Result<()> {
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

        let profile = Empty {
            account: dot::program::UserProfile::load(&mut ctx.accounts.profile, &programs_map),
            bump: Some(ctx.bumps.profile),
        };

        init_user_profile_handler(user.clone(), profile.clone(), name);

        dot::program::UserProfile::store(profile.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (vault_id : u64)]
    pub struct InitVault<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: Vault > () + 8 , payer = owner , seeds = ["vault" . as_bytes () . as_ref () , owner . key () . as_ref () , vault_id . to_le_bytes () . as_ref ()] , bump)]
        pub vault: Box<Account<'info, dot::program::Vault>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn init_vault(ctx: Context<InitVault>, vault_id: u64) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "system_program",
            ctx.accounts.system_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let owner = SeahorseSigner {
            account: &ctx.accounts.owner,
            programs: &programs_map,
        };

        let vault = Empty {
            account: dot::program::Vault::load(&mut ctx.accounts.vault, &programs_map),
            bump: Some(ctx.bumps.vault),
        };

        init_vault_handler(owner.clone(), vault.clone(), vault_id);

        dot::program::Vault::store(vault.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (data : String)]
    pub struct UpdateDataRecord<'info> {
        #[account(mut)]
        pub creator: Signer<'info>,
        #[account(mut)]
        pub record: Box<Account<'info, dot::program::DataRecord>>,
    }

    pub fn update_data_record(ctx: Context<UpdateDataRecord>, data: String) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let creator = SeahorseSigner {
            account: &ctx.accounts.creator,
            programs: &programs_map,
        };

        let record = dot::program::DataRecord::load(&mut ctx.accounts.record, &programs_map);

        update_data_record_handler(creator.clone(), record.clone(), data);

        dot::program::DataRecord::store(record);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct UpdateGlobalConfig<'info> {
        #[account(mut)]
        pub admin: Signer<'info>,
        #[account(mut)]
        pub config: Box<Account<'info, dot::program::GlobalConfig>>,
    }

    pub fn update_global_config(ctx: Context<UpdateGlobalConfig>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let admin = SeahorseSigner {
            account: &ctx.accounts.admin,
            programs: &programs_map,
        };

        let config = dot::program::GlobalConfig::load(&mut ctx.accounts.config, &programs_map);

        update_global_config_handler(admin.clone(), config.clone());

        dot::program::GlobalConfig::store(config);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct VisitProfile<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(mut)]
        pub profile: Box<Account<'info, dot::program::UserProfile>>,
    }

    pub fn visit_profile(ctx: Context<VisitProfile>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let user = SeahorseSigner {
            account: &ctx.accounts.user,
            programs: &programs_map,
        };

        let profile = dot::program::UserProfile::load(&mut ctx.accounts.profile, &programs_map);

        visit_profile_handler(user.clone(), profile.clone());

        dot::program::UserProfile::store(profile);

        return Ok(());
    }
}

