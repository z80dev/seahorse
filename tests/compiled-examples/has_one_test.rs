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
pub struct Vault {
    pub owner: Pubkey,
    pub authority: Pubkey,
    pub data: u64,
}

impl<'info, 'entrypoint> Vault {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedVault<'info, 'entrypoint>> {
        let owner = account.owner.clone();
        let authority = account.authority.clone();
        let data = account.data;

        Mutable::new(LoadedVault {
            __account__: account,
            __programs__: programs_map,
            owner,
            authority,
            data,
        })
    }

    pub fn store(loaded: Mutable<LoadedVault>) {
        let mut loaded = loaded.borrow_mut();
        let owner = loaded.owner.clone();

        loaded.__account__.owner = owner;

        let authority = loaded.authority.clone();

        loaded.__account__.authority = authority;

        let data = loaded.data;

        loaded.__account__.data = data;
    }
}

#[derive(Debug)]
pub struct LoadedVault<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, Vault>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub owner: Pubkey,
    pub authority: Pubkey,
    pub data: u64,
}

pub fn create_vault_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut vault: Empty<Mutable<LoadedVault<'info, '_>>>,
) -> () {
    let mut vault = vault.account.clone();

    assign!(vault.borrow_mut().owner, owner.key());

    assign!(vault.borrow_mut().authority, owner.key());

    assign!(vault.borrow_mut().data, 0);
}

pub fn transfer_authority_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut new_authority: SeahorseSigner<'info, '_>,
    mut vault: Mutable<LoadedVault<'info, '_>>,
) -> () {
    ();

    assign!(vault.borrow_mut().authority, new_authority.key());
}

pub fn update_vault_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut vault: Mutable<LoadedVault<'info, '_>>,
) -> () {
    ();

    assign!(vault.borrow_mut().data, vault.borrow().data + 1);
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

declare_id!("HAS1111111111111111111111111111111111111111");

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
mod has_one_test {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    pub struct CreateVault<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: Vault > () + 8 , payer = owner , seeds = ["vault" . as_bytes () . as_ref () , owner . key () . as_ref ()] , bump)]
        pub vault: Box<Account<'info, dot::program::Vault>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn create_vault(ctx: Context<CreateVault>) -> Result<()> {
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

        create_vault_handler(owner.clone(), vault.clone());

        dot::program::Vault::store(vault.account);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct TransferAuthority<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub new_authority: Signer<'info>,
        # [account (mut , has_one = authority)]
        pub vault: Box<Account<'info, dot::program::Vault>>,
    }

    pub fn transfer_authority(ctx: Context<TransferAuthority>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let new_authority = SeahorseSigner {
            account: &ctx.accounts.new_authority,
            programs: &programs_map,
        };

        let vault = dot::program::Vault::load(&mut ctx.accounts.vault, &programs_map);

        transfer_authority_handler(authority.clone(), new_authority.clone(), vault.clone());

        dot::program::Vault::store(vault);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct UpdateVault<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        # [account (mut , has_one = authority)]
        pub vault: Box<Account<'info, dot::program::Vault>>,
    }

    pub fn update_vault(ctx: Context<UpdateVault>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let vault = dot::program::Vault::load(&mut ctx.accounts.vault, &programs_map);

        update_vault_handler(authority.clone(), vault.clone());

        dot::program::Vault::store(vault);

        return Ok(());
    }
}

