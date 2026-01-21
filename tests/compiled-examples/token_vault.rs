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
    pub mint: Pubkey,
    pub total_deposits: u64,
    pub bump: u8,
}

impl<'info, 'entrypoint> Vault {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedVault<'info, 'entrypoint>> {
        let owner = account.owner.clone();
        let mint = account.mint.clone();
        let total_deposits = account.total_deposits;
        let bump = account.bump;

        Mutable::new(LoadedVault {
            __account__: account,
            __programs__: programs_map,
            owner,
            mint,
            total_deposits,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedVault>) {
        let mut loaded = loaded.borrow_mut();
        let owner = loaded.owner.clone();

        loaded.__account__.owner = owner;

        let mint = loaded.mint.clone();

        loaded.__account__.mint = mint;

        let total_deposits = loaded.total_deposits;

        loaded.__account__.total_deposits = total_deposits;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedVault<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, Vault>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub total_deposits: u64,
    pub bump: u8,
}

pub fn deposit_handler<'info>(
    mut user: SeahorseSigner<'info, '_>,
    mut vault: Mutable<LoadedVault<'info, '_>>,
    mut user_token_account: SeahorseAccount<'info, '_, TokenAccount>,
    mut vault_token_account: SeahorseAccount<'info, '_, TokenAccount>,
    mut mint: SeahorseAccount<'info, '_, Mint>,
    mut amount: u64,
) -> () {
    if !(amount > 0) {
        panic!("Amount must be greater than zero");
    }

    token::transfer(
        CpiContext::new(
            user_token_account.programs.get("token_program"),
            token::Transfer {
                from: user_token_account.to_account_info(),
                authority: user.clone().to_account_info(),
                to: vault_token_account.clone().to_account_info(),
            },
        ),
        amount.clone(),
    )
    .unwrap();

    assign!(
        vault.borrow_mut().total_deposits,
        vault.borrow().total_deposits + amount
    );
}

pub fn initialize_vault_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut vault: Empty<Mutable<LoadedVault<'info, '_>>>,
    mut vault_token_account: Empty<SeahorseAccount<'info, '_, TokenAccount>>,
    mut mint: SeahorseAccount<'info, '_, Mint>,
) -> () {
    let mut bump = vault.bump.unwrap();
    let mut vault = vault.account.clone();

    vault_token_account.account.clone();

    assign!(vault.borrow_mut().owner, owner.key());

    assign!(vault.borrow_mut().mint, mint.key());

    assign!(vault.borrow_mut().total_deposits, 0);

    assign!(vault.borrow_mut().bump, bump);
}

pub fn withdraw_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut vault: Mutable<LoadedVault<'info, '_>>,
    mut owner_token_account: SeahorseAccount<'info, '_, TokenAccount>,
    mut vault_token_account: SeahorseAccount<'info, '_, TokenAccount>,
    mut mint: SeahorseAccount<'info, '_, Mint>,
    mut amount: u64,
) -> () {
    if !(owner.key() == vault.borrow().owner) {
        panic!("Unauthorized");
    }

    if !(amount > 0) {
        panic!("Amount must be greater than zero");
    }

    if !(vault_token_account.amount >= amount) {
        panic!("Insufficient funds in vault");
    }

    let mut bump = vault.borrow().bump;

    token::transfer(
        CpiContext::new_with_signer(
            vault_token_account.programs.get("token_program"),
            token::Transfer {
                from: vault_token_account.to_account_info(),
                authority: vault.borrow().__account__.to_account_info(),
                to: owner_token_account.clone().to_account_info(),
            },
            &[Mutable::new(vec![
                "vault".to_string().as_bytes().as_ref(),
                owner.key().as_ref(),
                mint.key().as_ref(),
                bump.to_le_bytes().as_ref(),
            ])
            .borrow()
            .as_slice()],
        ),
        amount.clone(),
    )
    .unwrap();

    assign!(
        vault.borrow_mut().total_deposits,
        vault.borrow().total_deposits - amount
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

declare_id!("3JwSuBw6X2q2FknhbhfvXnuFhdeJN9KCTtpGk6Qx9mLL");

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
mod token_vault {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    # [instruction (amount : u64)]
    pub struct Deposit<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(mut)]
        pub vault: Box<Account<'info, dot::program::Vault>>,
        #[account(mut)]
        pub user_token_account: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub vault_token_account: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub mint: Box<Account<'info, Mint>>,
        pub token_program: Program<'info, Token>,
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
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

        let vault = dot::program::Vault::load(&mut ctx.accounts.vault, &programs_map);
        let user_token_account = SeahorseAccount {
            account: &ctx.accounts.user_token_account,
            programs: &programs_map,
        };

        let vault_token_account = SeahorseAccount {
            account: &ctx.accounts.vault_token_account,
            programs: &programs_map,
        };

        let mint = SeahorseAccount {
            account: &ctx.accounts.mint,
            programs: &programs_map,
        };

        deposit_handler(
            user.clone(),
            vault.clone(),
            user_token_account.clone(),
            vault_token_account.clone(),
            mint.clone(),
            amount,
        );

        dot::program::Vault::store(vault);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct InitializeVault<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: Vault > () + 8 , payer = owner , seeds = ["vault" . as_bytes () . as_ref () , owner . key () . as_ref () , mint . key () . as_ref ()] , bump)]
        pub vault: Box<Account<'info, dot::program::Vault>>,
        # [account (init , payer = owner , seeds = ["vault_token" . as_bytes () . as_ref () , owner . key () . as_ref () , mint . key () . as_ref ()] , bump , token :: mint = mint , token :: authority = vault)]
        pub vault_token_account: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub mint: Box<Account<'info, Mint>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
        pub token_program: Program<'info, Token>,
    }

    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
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
        let owner = SeahorseSigner {
            account: &ctx.accounts.owner,
            programs: &programs_map,
        };

        let vault = Empty {
            account: dot::program::Vault::load(&mut ctx.accounts.vault, &programs_map),
            bump: Some(ctx.bumps.vault),
        };

        let vault_token_account = Empty {
            account: SeahorseAccount {
                account: &ctx.accounts.vault_token_account,
                programs: &programs_map,
            },
            bump: Some(ctx.bumps.vault_token_account),
        };

        let mint = SeahorseAccount {
            account: &ctx.accounts.mint,
            programs: &programs_map,
        };

        initialize_vault_handler(
            owner.clone(),
            vault.clone(),
            vault_token_account.clone(),
            mint.clone(),
        );

        dot::program::Vault::store(vault.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (amount : u64)]
    pub struct Withdraw<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        #[account(mut)]
        pub vault: Box<Account<'info, dot::program::Vault>>,
        #[account(mut)]
        pub owner_token_account: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub vault_token_account: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub mint: Box<Account<'info, Mint>>,
        pub token_program: Program<'info, Token>,
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "token_program",
            ctx.accounts.token_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let owner = SeahorseSigner {
            account: &ctx.accounts.owner,
            programs: &programs_map,
        };

        let vault = dot::program::Vault::load(&mut ctx.accounts.vault, &programs_map);
        let owner_token_account = SeahorseAccount {
            account: &ctx.accounts.owner_token_account,
            programs: &programs_map,
        };

        let vault_token_account = SeahorseAccount {
            account: &ctx.accounts.vault_token_account,
            programs: &programs_map,
        };

        let mint = SeahorseAccount {
            account: &ctx.accounts.mint,
            programs: &programs_map,
        };

        withdraw_handler(
            owner.clone(),
            vault.clone(),
            owner_token_account.clone(),
            vault_token_account.clone(),
            mint.clone(),
            amount,
        );

        dot::program::Vault::store(vault);

        return Ok(());
    }
}

