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
pub struct ArrayData {
    pub owner: Pubkey,
    pub values: [u64; 4],
    pub flags: [bool; 8],
    pub bump: u8,
}

impl<'info, 'entrypoint> ArrayData {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedArrayData<'info, 'entrypoint>> {
        let owner = account.owner.clone();
        let values = Mutable::new(account.values.clone().map(|element| element));
        let flags = Mutable::new(account.flags.clone().map(|element| element));
        let bump = account.bump;

        Mutable::new(LoadedArrayData {
            __account__: account,
            __programs__: programs_map,
            owner,
            values,
            flags,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedArrayData>) {
        let mut loaded = loaded.borrow_mut();
        let owner = loaded.owner.clone();

        loaded.__account__.owner = owner;

        let values = loaded
            .values
            .clone()
            .borrow()
            .clone()
            .map(|element| element);

        loaded.__account__.values = values;

        let flags = loaded.flags.clone().borrow().clone().map(|element| element);

        loaded.__account__.flags = flags;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedArrayData<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, ArrayData>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub owner: Pubkey,
    pub values: Mutable<[u64; 4]>,
    pub flags: Mutable<[bool; 8]>,
    pub bump: u8,
}

#[account]
#[derive(Debug)]
pub struct ComplexData {
    pub owner: Pubkey,
    pub stats: Stats,
    pub is_active: bool,
    pub bump: u8,
}

impl<'info, 'entrypoint> ComplexData {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedComplexData<'info, 'entrypoint>> {
        let owner = account.owner.clone();
        let stats = Mutable::new(Stats::load(account.stats.clone()));
        let is_active = account.is_active.clone();
        let bump = account.bump;

        Mutable::new(LoadedComplexData {
            __account__: account,
            __programs__: programs_map,
            owner,
            stats,
            is_active,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedComplexData>) {
        let mut loaded = loaded.borrow_mut();
        let owner = loaded.owner.clone();

        loaded.__account__.owner = owner;

        let stats = Stats::store(loaded.stats.clone().borrow().clone());

        loaded.__account__.stats = stats;

        let is_active = loaded.is_active.clone();

        loaded.__account__.is_active = is_active;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedComplexData<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, ComplexData>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub owner: Pubkey,
    pub stats: Mutable<Loaded!(Stats)>,
    pub is_active: bool,
    pub bump: u8,
}

#[account]
#[derive(Debug)]
pub struct SimpleData {
    pub owner: Pubkey,
    pub value_u8: u8,
    pub value_u16: u16,
    pub value_u32: u32,
    pub value_u64: u64,
    pub value_i64: i64,
    pub bump: u8,
}

impl<'info, 'entrypoint> SimpleData {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedSimpleData<'info, 'entrypoint>> {
        let owner = account.owner.clone();
        let value_u8 = account.value_u8;
        let value_u16 = account.value_u16;
        let value_u32 = account.value_u32;
        let value_u64 = account.value_u64;
        let value_i64 = account.value_i64;
        let bump = account.bump;

        Mutable::new(LoadedSimpleData {
            __account__: account,
            __programs__: programs_map,
            owner,
            value_u8,
            value_u16,
            value_u32,
            value_u64,
            value_i64,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedSimpleData>) {
        let mut loaded = loaded.borrow_mut();
        let owner = loaded.owner.clone();

        loaded.__account__.owner = owner;

        let value_u8 = loaded.value_u8;

        loaded.__account__.value_u8 = value_u8;

        let value_u16 = loaded.value_u16;

        loaded.__account__.value_u16 = value_u16;

        let value_u32 = loaded.value_u32;

        loaded.__account__.value_u32 = value_u32;

        let value_u64 = loaded.value_u64;

        loaded.__account__.value_u64 = value_u64;

        let value_i64 = loaded.value_i64;

        loaded.__account__.value_i64 = value_i64;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedSimpleData<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, SimpleData>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub owner: Pubkey,
    pub value_u8: u8,
    pub value_u16: u16,
    pub value_u32: u32,
    pub value_u64: u64,
    pub value_i64: i64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct Stats {
    pub count: u64,
    pub total: u64,
    pub average: u64,
}

#[derive(Clone, Debug, Default)]
pub struct LoadedStats {
    pub count: u64,
    pub total: u64,
    pub average: u64,
}

impl Mutable<LoadedStats> {
    pub fn __init__(&self, mut count: u64, mut total: u64, mut average: u64) -> () {
        assign!(self.borrow_mut().count, count);

        assign!(self.borrow_mut().total, total);

        assign!(self.borrow_mut().average, average);
    }
}

impl LoadedStats {
    pub fn __new__(count: u64, total: u64, average: u64) -> Mutable<Self> {
        let obj = Mutable::new(LoadedStats::default());

        obj.__init__(count, total, average);

        return obj;
    }
}

impl Loadable for Stats {
    type Loaded = LoadedStats;

    fn load(stored: Self) -> Self::Loaded {
        Self::Loaded {
            count: stored.count,
            total: stored.total,
            average: stored.average,
        }
    }

    fn store(loaded: Self::Loaded) -> Self {
        Self {
            count: loaded.count,
            total: loaded.total,
            average: loaded.average,
        }
    }
}

#[account]
#[derive(Debug)]
pub struct StringData {
    pub owner: Pubkey,
    pub name: String,
    pub description: String,
    pub bump: u8,
}

impl<'info, 'entrypoint> StringData {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedStringData<'info, 'entrypoint>> {
        let owner = account.owner.clone();
        let name = account.name.clone();
        let description = account.description.clone();
        let bump = account.bump;

        Mutable::new(LoadedStringData {
            __account__: account,
            __programs__: programs_map,
            owner,
            name,
            description,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedStringData>) {
        let mut loaded = loaded.borrow_mut();
        let owner = loaded.owner.clone();

        loaded.__account__.owner = owner;

        let name = loaded.name.clone();

        loaded.__account__.name = name;

        let description = loaded.description.clone();

        loaded.__account__.description = description;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedStringData<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, StringData>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub owner: Pubkey,
    pub name: String,
    pub description: String,
    pub bump: u8,
}

pub fn init_complex_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut data: Empty<Mutable<LoadedComplexData<'info, '_>>>,
    mut initial_count: u64,
) -> () {
    let mut bump = data.bump.unwrap();
    let mut data = data.account.clone();

    assign!(data.borrow_mut().owner, owner.key());

    assign!(
        data.borrow_mut().stats,
        <Loaded!(Stats)>::__new__(
            initial_count.clone(),
            <u64 as TryFrom<_>>::try_from(0).unwrap(),
            <u64 as TryFrom<_>>::try_from(0).unwrap()
        )
    );

    assign!(data.borrow_mut().is_active, true);

    assign!(data.borrow_mut().bump, bump);
}

pub fn init_simple_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut data: Empty<Mutable<LoadedSimpleData<'info, '_>>>,
    mut value_u8: u8,
    mut value_u64: u64,
) -> () {
    let mut bump = data.bump.unwrap();
    let mut data = data.account.clone();

    assign!(data.borrow_mut().owner, owner.key());

    assign!(data.borrow_mut().value_u8, value_u8);

    assign!(data.borrow_mut().value_u16, 0);

    assign!(data.borrow_mut().value_u32, 0);

    assign!(data.borrow_mut().value_u64, value_u64);

    assign!(data.borrow_mut().value_i64, 0);

    assign!(data.borrow_mut().bump, bump);
}

pub fn init_with_array_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut data: Empty<Mutable<LoadedArrayData<'info, '_>>>,
) -> () {
    let mut bump = data.bump.unwrap();
    let mut data = data.account.clone();

    assign!(data.borrow_mut().owner, owner.key());

    assign!(
        (*data
            .borrow_mut()
            .values
            .borrow_mut()
            .index_wrapped_mut(0.into())),
        0
    );

    assign!(
        (*data
            .borrow_mut()
            .values
            .borrow_mut()
            .index_wrapped_mut(1.into())),
        0
    );

    assign!(
        (*data
            .borrow_mut()
            .values
            .borrow_mut()
            .index_wrapped_mut(2.into())),
        0
    );

    assign!(
        (*data
            .borrow_mut()
            .values
            .borrow_mut()
            .index_wrapped_mut(3.into())),
        0
    );

    assign!(
        (*data
            .borrow_mut()
            .flags
            .borrow_mut()
            .index_wrapped_mut(0.into())),
        false
    );

    assign!(
        (*data
            .borrow_mut()
            .flags
            .borrow_mut()
            .index_wrapped_mut(1.into())),
        false
    );

    assign!(
        (*data
            .borrow_mut()
            .flags
            .borrow_mut()
            .index_wrapped_mut(2.into())),
        false
    );

    assign!(
        (*data
            .borrow_mut()
            .flags
            .borrow_mut()
            .index_wrapped_mut(3.into())),
        false
    );

    assign!(
        (*data
            .borrow_mut()
            .flags
            .borrow_mut()
            .index_wrapped_mut(4.into())),
        false
    );

    assign!(
        (*data
            .borrow_mut()
            .flags
            .borrow_mut()
            .index_wrapped_mut(5.into())),
        false
    );

    assign!(
        (*data
            .borrow_mut()
            .flags
            .borrow_mut()
            .index_wrapped_mut(6.into())),
        false
    );

    assign!(
        (*data
            .borrow_mut()
            .flags
            .borrow_mut()
            .index_wrapped_mut(7.into())),
        false
    );

    assign!(data.borrow_mut().bump, bump);
}

pub fn init_with_string_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut data: Empty<Mutable<LoadedStringData<'info, '_>>>,
    mut name: String,
    mut description: String,
) -> () {
    let mut bump = data.bump.unwrap();
    let mut data = data.account.clone();

    assign!(data.borrow_mut().owner, owner.key());

    assign!(data.borrow_mut().name, name);

    assign!(data.borrow_mut().description, description);

    assign!(data.borrow_mut().bump, bump);
}

pub fn toggle_active_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut data: Mutable<LoadedComplexData<'info, '_>>,
) -> () {
    if !(owner.key() == data.borrow().owner) {
        panic!("Unauthorized");
    }

    assign!(data.borrow_mut().is_active, !data.borrow().is_active);
}

pub fn update_array_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut data: Mutable<LoadedArrayData<'info, '_>>,
    mut v0: u64,
    mut v1: u64,
    mut v2: u64,
    mut v3: u64,
    mut f0: bool,
    mut f1: bool,
    mut f2: bool,
    mut f3: bool,
    mut f4: bool,
    mut f5: bool,
    mut f6: bool,
    mut f7: bool,
) -> () {
    if !(owner.key() == data.borrow().owner) {
        panic!("Unauthorized");
    }

    assign!(
        (*data
            .borrow_mut()
            .values
            .borrow_mut()
            .index_wrapped_mut(0.into())),
        v0
    );

    assign!(
        (*data
            .borrow_mut()
            .values
            .borrow_mut()
            .index_wrapped_mut(1.into())),
        v1
    );

    assign!(
        (*data
            .borrow_mut()
            .values
            .borrow_mut()
            .index_wrapped_mut(2.into())),
        v2
    );

    assign!(
        (*data
            .borrow_mut()
            .values
            .borrow_mut()
            .index_wrapped_mut(3.into())),
        v3
    );

    assign!(
        (*data
            .borrow_mut()
            .flags
            .borrow_mut()
            .index_wrapped_mut(0.into())),
        f0
    );

    assign!(
        (*data
            .borrow_mut()
            .flags
            .borrow_mut()
            .index_wrapped_mut(1.into())),
        f1
    );

    assign!(
        (*data
            .borrow_mut()
            .flags
            .borrow_mut()
            .index_wrapped_mut(2.into())),
        f2
    );

    assign!(
        (*data
            .borrow_mut()
            .flags
            .borrow_mut()
            .index_wrapped_mut(3.into())),
        f3
    );

    assign!(
        (*data
            .borrow_mut()
            .flags
            .borrow_mut()
            .index_wrapped_mut(4.into())),
        f4
    );

    assign!(
        (*data
            .borrow_mut()
            .flags
            .borrow_mut()
            .index_wrapped_mut(5.into())),
        f5
    );

    assign!(
        (*data
            .borrow_mut()
            .flags
            .borrow_mut()
            .index_wrapped_mut(6.into())),
        f6
    );

    assign!(
        (*data
            .borrow_mut()
            .flags
            .borrow_mut()
            .index_wrapped_mut(7.into())),
        f7
    );
}

pub fn update_complex_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut data: Mutable<LoadedComplexData<'info, '_>>,
    mut count: u64,
    mut total: u64,
) -> () {
    if !(owner.key() == data.borrow().owner) {
        panic!("Unauthorized");
    }

    assign!(data.borrow_mut().stats.borrow_mut().count, count);

    assign!(data.borrow_mut().stats.borrow_mut().total, total);

    if count > 0 {
        assign!(data.borrow_mut().stats.borrow_mut().average, total / count);
    } else {
        assign!(data.borrow_mut().stats.borrow_mut().average, 0);
    }
}

pub fn update_simple_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut data: Mutable<LoadedSimpleData<'info, '_>>,
    mut value_u8: u8,
    mut value_u16: u16,
    mut value_u32: u32,
    mut value_u64: u64,
    mut value_i64: i64,
) -> () {
    if !(owner.key() == data.borrow().owner) {
        panic!("Unauthorized");
    }

    assign!(data.borrow_mut().value_u8, value_u8);

    assign!(data.borrow_mut().value_u16, value_u16);

    assign!(data.borrow_mut().value_u32, value_u32);

    assign!(data.borrow_mut().value_u64, value_u64);

    assign!(data.borrow_mut().value_i64, value_i64);
}

pub fn update_string_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut data: Mutable<LoadedStringData<'info, '_>>,
    mut name: String,
    mut description: String,
) -> () {
    if !(owner.key() == data.borrow().owner) {
        panic!("Unauthorized");
    }

    assign!(data.borrow_mut().name, name);

    assign!(data.borrow_mut().description, description);
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

declare_id!("InitPtrn1111111111111111111111111111111111");

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
mod init_patterns {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    # [instruction (initial_count : u64)]
    pub struct InitComplex<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: ComplexData > () + 8 , payer = owner , seeds = ["complex" . as_bytes () . as_ref () , owner . key () . as_ref ()] , bump)]
        pub data: Box<Account<'info, dot::program::ComplexData>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn init_complex(ctx: Context<InitComplex>, initial_count: u64) -> Result<()> {
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

        let data = Empty {
            account: dot::program::ComplexData::load(&mut ctx.accounts.data, &programs_map),
            bump: Some(ctx.bumps.data),
        };

        init_complex_handler(owner.clone(), data.clone(), initial_count);

        dot::program::ComplexData::store(data.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (value_u8 : u8 , value_u64 : u64)]
    pub struct InitSimple<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: SimpleData > () + 8 , payer = owner , seeds = ["simple" . as_bytes () . as_ref () , owner . key () . as_ref ()] , bump)]
        pub data: Box<Account<'info, dot::program::SimpleData>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn init_simple(ctx: Context<InitSimple>, value_u8: u8, value_u64: u64) -> Result<()> {
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

        let data = Empty {
            account: dot::program::SimpleData::load(&mut ctx.accounts.data, &programs_map),
            bump: Some(ctx.bumps.data),
        };

        init_simple_handler(owner.clone(), data.clone(), value_u8, value_u64);

        dot::program::SimpleData::store(data.account);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct InitWithArray<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: ArrayData > () + 8 , payer = owner , seeds = ["array_data" . as_bytes () . as_ref () , owner . key () . as_ref ()] , bump)]
        pub data: Box<Account<'info, dot::program::ArrayData>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn init_with_array(ctx: Context<InitWithArray>) -> Result<()> {
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

        let data = Empty {
            account: dot::program::ArrayData::load(&mut ctx.accounts.data, &programs_map),
            bump: Some(ctx.bumps.data),
        };

        init_with_array_handler(owner.clone(), data.clone());

        dot::program::ArrayData::store(data.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (name : String , description : String)]
    pub struct InitWithString<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: StringData > () + 8 + (200 as usize) , payer = owner , seeds = ["string_data" . as_bytes () . as_ref () , owner . key () . as_ref ()] , bump)]
        pub data: Box<Account<'info, dot::program::StringData>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn init_with_string(
        ctx: Context<InitWithString>,
        name: String,
        description: String,
    ) -> Result<()> {
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

        let data = Empty {
            account: dot::program::StringData::load(&mut ctx.accounts.data, &programs_map),
            bump: Some(ctx.bumps.data),
        };

        init_with_string_handler(owner.clone(), data.clone(), name, description);

        dot::program::StringData::store(data.account);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct ToggleActive<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        #[account(mut)]
        pub data: Box<Account<'info, dot::program::ComplexData>>,
    }

    pub fn toggle_active(ctx: Context<ToggleActive>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let owner = SeahorseSigner {
            account: &ctx.accounts.owner,
            programs: &programs_map,
        };

        let data = dot::program::ComplexData::load(&mut ctx.accounts.data, &programs_map);

        toggle_active_handler(owner.clone(), data.clone());

        dot::program::ComplexData::store(data);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (v0 : u64 , v1 : u64 , v2 : u64 , v3 : u64 , f0 : bool , f1 : bool , f2 : bool , f3 : bool , f4 : bool , f5 : bool , f6 : bool , f7 : bool)]
    pub struct UpdateArray<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        #[account(mut)]
        pub data: Box<Account<'info, dot::program::ArrayData>>,
    }

    pub fn update_array(
        ctx: Context<UpdateArray>,
        v0: u64,
        v1: u64,
        v2: u64,
        v3: u64,
        f0: bool,
        f1: bool,
        f2: bool,
        f3: bool,
        f4: bool,
        f5: bool,
        f6: bool,
        f7: bool,
    ) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let owner = SeahorseSigner {
            account: &ctx.accounts.owner,
            programs: &programs_map,
        };

        let data = dot::program::ArrayData::load(&mut ctx.accounts.data, &programs_map);

        update_array_handler(
            owner.clone(),
            data.clone(),
            v0,
            v1,
            v2,
            v3,
            f0,
            f1,
            f2,
            f3,
            f4,
            f5,
            f6,
            f7,
        );

        dot::program::ArrayData::store(data);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (count : u64 , total : u64)]
    pub struct UpdateComplex<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        #[account(mut)]
        pub data: Box<Account<'info, dot::program::ComplexData>>,
    }

    pub fn update_complex(ctx: Context<UpdateComplex>, count: u64, total: u64) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let owner = SeahorseSigner {
            account: &ctx.accounts.owner,
            programs: &programs_map,
        };

        let data = dot::program::ComplexData::load(&mut ctx.accounts.data, &programs_map);

        update_complex_handler(owner.clone(), data.clone(), count, total);

        dot::program::ComplexData::store(data);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (value_u8 : u8 , value_u16 : u16 , value_u32 : u32 , value_u64 : u64 , value_i64 : i64)]
    pub struct UpdateSimple<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        #[account(mut)]
        pub data: Box<Account<'info, dot::program::SimpleData>>,
    }

    pub fn update_simple(
        ctx: Context<UpdateSimple>,
        value_u8: u8,
        value_u16: u16,
        value_u32: u32,
        value_u64: u64,
        value_i64: i64,
    ) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let owner = SeahorseSigner {
            account: &ctx.accounts.owner,
            programs: &programs_map,
        };

        let data = dot::program::SimpleData::load(&mut ctx.accounts.data, &programs_map);

        update_simple_handler(
            owner.clone(),
            data.clone(),
            value_u8,
            value_u16,
            value_u32,
            value_u64,
            value_i64,
        );

        dot::program::SimpleData::store(data);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (name : String , description : String)]
    pub struct UpdateString<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        #[account(mut)]
        pub data: Box<Account<'info, dot::program::StringData>>,
    }

    pub fn update_string(
        ctx: Context<UpdateString>,
        name: String,
        description: String,
    ) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let owner = SeahorseSigner {
            account: &ctx.accounts.owner,
            programs: &programs_map,
        };

        let data = dot::program::StringData::load(&mut ctx.accounts.data, &programs_map);

        update_string_handler(owner.clone(), data.clone(), name, description);

        dot::program::StringData::store(data);

        return Ok(());
    }
}

