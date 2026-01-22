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
pub struct DynamicData {
    pub owner: Pubkey,
    pub data_id: u64,
    pub content: String,
    pub content_len: u64,
    pub allocated_size: u64,
    pub version: u64,
    pub is_closed: bool,
    pub bump: u8,
}

impl<'info, 'entrypoint> DynamicData {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedDynamicData<'info, 'entrypoint>> {
        let owner = account.owner.clone();
        let data_id = account.data_id;
        let content = account.content.clone();
        let content_len = account.content_len;
        let allocated_size = account.allocated_size;
        let version = account.version;
        let is_closed = account.is_closed.clone();
        let bump = account.bump;

        Mutable::new(LoadedDynamicData {
            __account__: account,
            __programs__: programs_map,
            owner,
            data_id,
            content,
            content_len,
            allocated_size,
            version,
            is_closed,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedDynamicData>) {
        let mut loaded = loaded.borrow_mut();
        let owner = loaded.owner.clone();

        loaded.__account__.owner = owner;

        let data_id = loaded.data_id;

        loaded.__account__.data_id = data_id;

        let content = loaded.content.clone();

        loaded.__account__.content = content;

        let content_len = loaded.content_len;

        loaded.__account__.content_len = content_len;

        let allocated_size = loaded.allocated_size;

        loaded.__account__.allocated_size = allocated_size;

        let version = loaded.version;

        loaded.__account__.version = version;

        let is_closed = loaded.is_closed.clone();

        loaded.__account__.is_closed = is_closed;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedDynamicData<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, DynamicData>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub owner: Pubkey,
    pub data_id: u64,
    pub content: String,
    pub content_len: u64,
    pub allocated_size: u64,
    pub version: u64,
    pub is_closed: bool,
    pub bump: u8,
}

#[account]
#[derive(Debug)]
pub struct ResizeRecord {
    pub owner: Pubkey,
    pub record_id: u64,
    pub current_size: u64,
    pub max_size_reached: u64,
    pub resize_count: u64,
    pub is_active: bool,
    pub bump: u8,
}

impl<'info, 'entrypoint> ResizeRecord {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedResizeRecord<'info, 'entrypoint>> {
        let owner = account.owner.clone();
        let record_id = account.record_id;
        let current_size = account.current_size;
        let max_size_reached = account.max_size_reached;
        let resize_count = account.resize_count;
        let is_active = account.is_active.clone();
        let bump = account.bump;

        Mutable::new(LoadedResizeRecord {
            __account__: account,
            __programs__: programs_map,
            owner,
            record_id,
            current_size,
            max_size_reached,
            resize_count,
            is_active,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedResizeRecord>) {
        let mut loaded = loaded.borrow_mut();
        let owner = loaded.owner.clone();

        loaded.__account__.owner = owner;

        let record_id = loaded.record_id;

        loaded.__account__.record_id = record_id;

        let current_size = loaded.current_size;

        loaded.__account__.current_size = current_size;

        let max_size_reached = loaded.max_size_reached;

        loaded.__account__.max_size_reached = max_size_reached;

        let resize_count = loaded.resize_count;

        loaded.__account__.resize_count = resize_count;

        let is_active = loaded.is_active.clone();

        loaded.__account__.is_active = is_active;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedResizeRecord<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, ResizeRecord>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub owner: Pubkey,
    pub record_id: u64,
    pub current_size: u64,
    pub max_size_reached: u64,
    pub resize_count: u64,
    pub is_active: bool,
    pub bump: u8,
}

pub fn close_data_account_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut data: Mutable<LoadedDynamicData<'info, '_>>,
    mut data_id: u64,
) -> () {
    "\n    Simulate closing the data account.\n\n    In Anchor, this would use the close constraint to:\n    1. Zero out the account data\n    2. Transfer remaining lamports to owner\n    3. Set account owner to system program\n\n    Here we mark it as closed since Seahorse can't actually close accounts.\n\n    Args:\n        owner: Account owner (receives lamports in real close)\n        data: The data account to close\n        data_id: Data ID for verification\n    " . to_string () ;

    if !(data.borrow().owner == owner.key()) {
        panic!("Unauthorized");
    }

    if !(!data.borrow().is_closed) {
        panic!("Account already closed");
    }

    let mut final_version = data.borrow().version;

    assign!(data.borrow_mut().is_closed, true);

    assign!(data.borrow_mut().content, "".to_string().clone());

    assign!(data.borrow_mut().content_len, 0);

    assign!(data.borrow_mut().allocated_size, 0);

    solana_program::msg!(
        "{}",
        "Closing data account, returning lamports to owner".to_string()
    );

    solana_program::msg!("{}", format!("Final version was: {}", final_version));
}

pub fn close_record_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut record: Mutable<LoadedResizeRecord<'info, '_>>,
    mut record_id: u64,
) -> () {
    "\n    Close a resize record.\n\n    In Anchor, this would close the account and return lamports.\n    Here we mark it as inactive.\n\n    Args:\n        owner: Record owner\n        record: The record to close\n        record_id: Record ID for verification\n    " . to_string () ;

    if !(record.borrow().owner == owner.key()) {
        panic!("Unauthorized");
    }

    if !record.borrow().is_active {
        panic!("Record already closed");
    }

    let mut final_count = record.borrow().resize_count;

    assign!(record.borrow_mut().is_active, false);

    assign!(record.borrow_mut().current_size, 0);

    assign!(record.borrow_mut().max_size_reached, 0);

    assign!(record.borrow_mut().resize_count, 0);

    solana_program::msg!(
        "{}",
        "Closing record, returning lamports to owner".to_string()
    );

    solana_program::msg!("{}", format!("Final resize count: {}", final_count));
}

pub fn create_record_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut record: Empty<Mutable<LoadedResizeRecord<'info, '_>>>,
    mut record_id: u64,
) -> () {
    "\n    Create a record for tracking resize operations.\n\n    Args:\n        owner: Record owner\n        record: The record account to initialize\n        record_id: Unique identifier for PDA derivation\n    " . to_string () ;

    let mut bump = record.bump.unwrap();
    let mut record = record.account.clone();

    assign!(record.borrow_mut().owner, owner.key());

    assign!(record.borrow_mut().record_id, record_id);

    assign!(record.borrow_mut().current_size, 0);

    assign!(record.borrow_mut().max_size_reached, 0);

    assign!(record.borrow_mut().resize_count, 0);

    assign!(record.borrow_mut().is_active, true);

    assign!(record.borrow_mut().bump, bump);

    solana_program::msg!("{}", format!("Created record with id: {}", record_id));
}

pub fn get_data_info_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut data: Mutable<LoadedDynamicData<'info, '_>>,
    mut data_id: u64,
) -> () {
    "\n    Get information about a data account (read-only).\n\n    Args:\n        owner: Signer for instruction\n        data: The data account to query\n        data_id: Data ID for verification\n    " . to_string () ;

    solana_program::msg!("{}", format!("Data ID: {}", data.borrow().data_id));

    solana_program::msg!("{}", format!("Owner: {:?}", data.borrow().owner));

    solana_program::msg!(
        "{}",
        format!("Content length: {}", data.borrow().content_len)
    );

    solana_program::msg!(
        "{}",
        format!("Allocated size: {}", data.borrow().allocated_size)
    );

    solana_program::msg!("{}", format!("Version: {}", data.borrow().version));

    solana_program::msg!("{}", format!("Is closed: {}", data.borrow().is_closed));
}

pub fn get_record_info_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut record: Mutable<LoadedResizeRecord<'info, '_>>,
    mut record_id: u64,
) -> () {
    "\n    Get information about a resize record (read-only).\n\n    Args:\n        owner: Signer for instruction\n        record: The record to query\n        record_id: Record ID for verification\n    " . to_string () ;

    solana_program::msg!("{}", format!("Record ID: {}", record.borrow().record_id));

    solana_program::msg!("{}", format!("Owner: {:?}", record.borrow().owner));

    solana_program::msg!(
        "{}",
        format!("Current size: {}", record.borrow().current_size)
    );

    solana_program::msg!(
        "{}",
        format!("Max size reached: {}", record.borrow().max_size_reached)
    );

    solana_program::msg!(
        "{}",
        format!("Resize count: {}", record.borrow().resize_count)
    );

    solana_program::msg!("{}", format!("Is active: {}", record.borrow().is_active));
}

pub fn grow_account_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut data: Mutable<LoadedDynamicData<'info, '_>>,
    mut data_id: u64,
    mut new_content: String,
    mut additional_space: u64,
) -> () {
    "\n    Simulate growing the account to accommodate more content.\n\n    In Anchor, this would use realloc::payer and realloc::zero = true.\n    Here we track the simulated size change.\n\n    Args:\n        owner: Account owner (must match stored owner)\n        data: The data account to grow\n        data_id: Data ID for verification\n        new_content: New content to store\n        additional_space: Additional bytes to allocate\n    " . to_string () ;

    if !(data.borrow().owner == owner.key()) {
        panic!("Unauthorized");
    }

    if !(!data.borrow().is_closed) {
        panic!("Account is closed");
    }

    let mut old_size = data.borrow().allocated_size;

    assign!(data.borrow_mut().content, new_content.clone());

    assign!(
        data.borrow_mut().content_len,
        <u64 as TryFrom<_>>::try_from((new_content.chars().count() as u64)).unwrap()
    );

    assign!(
        data.borrow_mut().allocated_size,
        data.borrow().allocated_size + additional_space
    );

    assign!(data.borrow_mut().version, data.borrow().version + 1);

    solana_program::msg!(
        "{}",
        format!(
            "Grew account from {} to {} bytes",
            old_size,
            data.borrow().allocated_size
        )
    );

    solana_program::msg!(
        "{}",
        format!("New content length: {} bytes", data.borrow().content_len)
    );

    solana_program::msg!("{}", format!("Version: {}", data.borrow().version));
}

pub fn initialize_data_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut data: Empty<Mutable<LoadedDynamicData<'info, '_>>>,
    mut data_id: u64,
    mut initial_content: String,
    mut initial_allocated_size: u64,
) -> () {
    "\n    Initialize a dynamic data account with initial content.\n\n    Args:\n        owner: Account owner who will control the data\n        data: The data account to initialize\n        data_id: Unique identifier for PDA derivation\n        initial_content: Initial string content\n        initial_allocated_size: Simulated initial allocation size\n    " . to_string () ;

    let mut bump = data.bump.unwrap();
    let mut data = data.account.clone();

    assign!(data.borrow_mut().owner, owner.key());

    assign!(data.borrow_mut().data_id, data_id);

    assign!(data.borrow_mut().content, initial_content.clone());

    assign!(
        data.borrow_mut().content_len,
        <u64 as TryFrom<_>>::try_from((initial_content.chars().count() as u64)).unwrap()
    );

    assign!(data.borrow_mut().allocated_size, initial_allocated_size);

    assign!(data.borrow_mut().version, 1);

    assign!(data.borrow_mut().is_closed, false);

    assign!(data.borrow_mut().bump, bump);

    solana_program::msg!(
        "{}",
        format!("Initialized data account with id: {}", data_id)
    );

    solana_program::msg!(
        "{}",
        format!("Content length: {} bytes", data.borrow().content_len)
    );

    solana_program::msg!(
        "{}",
        format!("Allocated size: {} bytes", data.borrow().allocated_size)
    );
}

pub fn record_resize_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut record: Mutable<LoadedResizeRecord<'info, '_>>,
    mut record_id: u64,
    mut new_size: u64,
    mut is_grow: bool,
) -> () {
    "\n    Record a resize event for tracking/analytics.\n\n    Args:\n        owner: Record owner (must match stored owner)\n        record: The record to update\n        record_id: Record ID for verification\n        new_size: New size after resize\n        is_grow: True if growing, False if shrinking\n    " . to_string () ;

    if !(record.borrow().owner == owner.key()) {
        panic!("Unauthorized");
    }

    if !record.borrow().is_active {
        panic!("Record is inactive");
    }

    assign!(record.borrow_mut().current_size, new_size);

    assign!(
        record.borrow_mut().resize_count,
        record.borrow().resize_count + 1
    );

    if new_size > record.borrow().max_size_reached {
        assign!(record.borrow_mut().max_size_reached, new_size);
    }

    if is_grow {
        solana_program::msg!("{}", format!("Recorded resize: grow to {} bytes", new_size));
    } else {
        solana_program::msg!(
            "{}",
            format!("Recorded resize: shrink to {} bytes", new_size)
        );
    }

    solana_program::msg!(
        "{}",
        format!("Total resizes: {}", record.borrow().resize_count)
    );
}

pub fn shrink_account_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut data: Mutable<LoadedDynamicData<'info, '_>>,
    mut data_id: u64,
    mut new_content: String,
    mut new_size: u64,
) -> () {
    "\n    Simulate shrinking the account to reclaim space.\n\n    In Anchor, this would use realloc with smaller space.\n    The excess lamports would be returned to the payer.\n\n    Args:\n        owner: Account owner (must match stored owner)\n        data: The data account to shrink\n        data_id: Data ID for verification\n        new_content: New (smaller) content\n        new_size: New allocated size (must be >= content length)\n    " . to_string () ;

    if !(data.borrow().owner == owner.key()) {
        panic!("Unauthorized");
    }

    if !(!data.borrow().is_closed) {
        panic!("Account is closed");
    }

    if !(new_size < data.borrow().allocated_size) {
        panic!("New size must be smaller");
    }

    if !(<u64 as TryFrom<_>>::try_from((new_content.chars().count() as u64)).unwrap() <= new_size) {
        panic!("Content too large for new size");
    }

    let mut old_size = data.borrow().allocated_size;

    assign!(data.borrow_mut().content, new_content.clone());

    assign!(
        data.borrow_mut().content_len,
        <u64 as TryFrom<_>>::try_from((new_content.chars().count() as u64)).unwrap()
    );

    assign!(data.borrow_mut().allocated_size, new_size);

    assign!(data.borrow_mut().version, data.borrow().version + 1);

    solana_program::msg!(
        "{}",
        format!(
            "Shrunk account from {} to {} bytes",
            old_size,
            data.borrow().allocated_size
        )
    );

    solana_program::msg!(
        "{}",
        format!("New content length: {} bytes", data.borrow().content_len)
    );

    solana_program::msg!("{}", format!("Version: {}", data.borrow().version));
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

declare_id!("F4fSecp12t3QtQaXTUWrjxLec1wQXJ31iQzuE2WjPsoB");

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
mod realloc_close {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    # [instruction (data_id : u64)]
    pub struct CloseDataAccount<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        #[account(mut)]
        pub data: Box<Account<'info, dot::program::DynamicData>>,
    }

    pub fn close_data_account(ctx: Context<CloseDataAccount>, data_id: u64) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let owner = SeahorseSigner {
            account: &ctx.accounts.owner,
            programs: &programs_map,
        };

        let data = dot::program::DynamicData::load(&mut ctx.accounts.data, &programs_map);

        close_data_account_handler(owner.clone(), data.clone(), data_id);

        dot::program::DynamicData::store(data);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (record_id : u64)]
    pub struct CloseRecord<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        #[account(mut)]
        pub record: Box<Account<'info, dot::program::ResizeRecord>>,
    }

    pub fn close_record(ctx: Context<CloseRecord>, record_id: u64) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let owner = SeahorseSigner {
            account: &ctx.accounts.owner,
            programs: &programs_map,
        };

        let record = dot::program::ResizeRecord::load(&mut ctx.accounts.record, &programs_map);

        close_record_handler(owner.clone(), record.clone(), record_id);

        dot::program::ResizeRecord::store(record);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (record_id : u64)]
    pub struct CreateRecord<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: ResizeRecord > () + 8 , payer = owner , seeds = ["record" . as_bytes () . as_ref () , record_id . to_le_bytes () . as_ref ()] , bump)]
        pub record: Box<Account<'info, dot::program::ResizeRecord>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn create_record(ctx: Context<CreateRecord>, record_id: u64) -> Result<()> {
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

        let record = Empty {
            account: dot::program::ResizeRecord::load(&mut ctx.accounts.record, &programs_map),
            bump: Some(ctx.bumps.record),
        };

        create_record_handler(owner.clone(), record.clone(), record_id);

        dot::program::ResizeRecord::store(record.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (data_id : u64)]
    pub struct GetDataInfo<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        #[account(mut)]
        pub data: Box<Account<'info, dot::program::DynamicData>>,
    }

    pub fn get_data_info(ctx: Context<GetDataInfo>, data_id: u64) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let owner = SeahorseSigner {
            account: &ctx.accounts.owner,
            programs: &programs_map,
        };

        let data = dot::program::DynamicData::load(&mut ctx.accounts.data, &programs_map);

        get_data_info_handler(owner.clone(), data.clone(), data_id);

        dot::program::DynamicData::store(data);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (record_id : u64)]
    pub struct GetRecordInfo<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        #[account(mut)]
        pub record: Box<Account<'info, dot::program::ResizeRecord>>,
    }

    pub fn get_record_info(ctx: Context<GetRecordInfo>, record_id: u64) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let owner = SeahorseSigner {
            account: &ctx.accounts.owner,
            programs: &programs_map,
        };

        let record = dot::program::ResizeRecord::load(&mut ctx.accounts.record, &programs_map);

        get_record_info_handler(owner.clone(), record.clone(), record_id);

        dot::program::ResizeRecord::store(record);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (data_id : u64 , new_content : String , additional_space : u64)]
    pub struct GrowAccount<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        #[account(mut)]
        pub data: Box<Account<'info, dot::program::DynamicData>>,
    }

    pub fn grow_account(
        ctx: Context<GrowAccount>,
        data_id: u64,
        new_content: String,
        additional_space: u64,
    ) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let owner = SeahorseSigner {
            account: &ctx.accounts.owner,
            programs: &programs_map,
        };

        let data = dot::program::DynamicData::load(&mut ctx.accounts.data, &programs_map);

        grow_account_handler(
            owner.clone(),
            data.clone(),
            data_id,
            new_content,
            additional_space,
        );

        dot::program::DynamicData::store(data);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (data_id : u64 , initial_content : String , initial_allocated_size : u64)]
    pub struct InitializeData<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: DynamicData > () + 8 + (200 as usize) , payer = owner , seeds = ["data" . as_bytes () . as_ref () , data_id . to_le_bytes () . as_ref ()] , bump)]
        pub data: Box<Account<'info, dot::program::DynamicData>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn initialize_data(
        ctx: Context<InitializeData>,
        data_id: u64,
        initial_content: String,
        initial_allocated_size: u64,
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
            account: dot::program::DynamicData::load(&mut ctx.accounts.data, &programs_map),
            bump: Some(ctx.bumps.data),
        };

        initialize_data_handler(
            owner.clone(),
            data.clone(),
            data_id,
            initial_content,
            initial_allocated_size,
        );

        dot::program::DynamicData::store(data.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (record_id : u64 , new_size : u64 , is_grow : bool)]
    pub struct RecordResize<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        #[account(mut)]
        pub record: Box<Account<'info, dot::program::ResizeRecord>>,
    }

    pub fn record_resize(
        ctx: Context<RecordResize>,
        record_id: u64,
        new_size: u64,
        is_grow: bool,
    ) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let owner = SeahorseSigner {
            account: &ctx.accounts.owner,
            programs: &programs_map,
        };

        let record = dot::program::ResizeRecord::load(&mut ctx.accounts.record, &programs_map);

        record_resize_handler(owner.clone(), record.clone(), record_id, new_size, is_grow);

        dot::program::ResizeRecord::store(record);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (data_id : u64 , new_content : String , new_size : u64)]
    pub struct ShrinkAccount<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        #[account(mut)]
        pub data: Box<Account<'info, dot::program::DynamicData>>,
    }

    pub fn shrink_account(
        ctx: Context<ShrinkAccount>,
        data_id: u64,
        new_content: String,
        new_size: u64,
    ) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let owner = SeahorseSigner {
            account: &ctx.accounts.owner,
            programs: &programs_map,
        };

        let data = dot::program::DynamicData::load(&mut ctx.accounts.data, &programs_map);

        shrink_account_handler(owner.clone(), data.clone(), data_id, new_content, new_size);

        dot::program::DynamicData::store(data);

        return Ok(());
    }
}

