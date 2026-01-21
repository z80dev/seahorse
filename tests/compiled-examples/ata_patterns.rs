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
pub struct Treasury {
    pub admin: Pubkey,
    pub mint: Pubkey,
    pub treasury_token_account: Pubkey,
    pub bump: u8,
}

impl<'info, 'entrypoint> Treasury {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedTreasury<'info, 'entrypoint>> {
        let admin = account.admin.clone();
        let mint = account.mint.clone();
        let treasury_token_account = account.treasury_token_account.clone();
        let bump = account.bump;

        Mutable::new(LoadedTreasury {
            __account__: account,
            __programs__: programs_map,
            admin,
            mint,
            treasury_token_account,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedTreasury>) {
        let mut loaded = loaded.borrow_mut();
        let admin = loaded.admin.clone();

        loaded.__account__.admin = admin;

        let mint = loaded.mint.clone();

        loaded.__account__.mint = mint;

        let treasury_token_account = loaded.treasury_token_account.clone();

        loaded.__account__.treasury_token_account = treasury_token_account;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedTreasury<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, Treasury>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub admin: Pubkey,
    pub mint: Pubkey,
    pub treasury_token_account: Pubkey,
    pub bump: u8,
}

#[account]
#[derive(Debug)]
pub struct UserAccount {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub token_account: Pubkey,
    pub bump: u8,
    pub is_registered: bool,
}

impl<'info, 'entrypoint> UserAccount {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedUserAccount<'info, 'entrypoint>> {
        let owner = account.owner.clone();
        let mint = account.mint.clone();
        let token_account = account.token_account.clone();
        let bump = account.bump;
        let is_registered = account.is_registered.clone();

        Mutable::new(LoadedUserAccount {
            __account__: account,
            __programs__: programs_map,
            owner,
            mint,
            token_account,
            bump,
            is_registered,
        })
    }

    pub fn store(loaded: Mutable<LoadedUserAccount>) {
        let mut loaded = loaded.borrow_mut();
        let owner = loaded.owner.clone();

        loaded.__account__.owner = owner;

        let mint = loaded.mint.clone();

        loaded.__account__.mint = mint;

        let token_account = loaded.token_account.clone();

        loaded.__account__.token_account = token_account;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;

        let is_registered = loaded.is_registered.clone();

        loaded.__account__.is_registered = is_registered;
    }
}

#[derive(Debug)]
pub struct LoadedUserAccount<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, UserAccount>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub token_account: Pubkey,
    pub bump: u8,
    pub is_registered: bool,
}

pub fn airdrop_to_user_handler<'info>(
    mut admin: SeahorseSigner<'info, '_>,
    mut mint: SeahorseAccount<'info, '_, Mint>,
    mut treasury: Mutable<LoadedTreasury<'info, '_>>,
    mut treasury_token_account: SeahorseAccount<'info, '_, TokenAccount>,
    mut user_account: Mutable<LoadedUserAccount<'info, '_>>,
    mut user_token_account: SeahorseAccount<'info, '_, TokenAccount>,
    mut amount: u64,
) -> () {
    "\n    Airdrop tokens to a registered user's token account.\n    PDA authority signs the transfer.\n    " . to_string () ;

    if !(amount > 0) {
        panic!("Amount must be greater than zero");
    }

    if !user_account.borrow().is_registered {
        panic!("User is not registered");
    }

    let mut bump = treasury.borrow().bump;

    token::transfer(
        CpiContext::new_with_signer(
            treasury_token_account.programs.get("token_program"),
            token::Transfer {
                from: treasury_token_account.to_account_info(),
                authority: treasury.borrow().__account__.to_account_info(),
                to: user_token_account.clone().to_account_info(),
            },
            &[Mutable::new(vec![
                "treasury".to_string().as_bytes().as_ref(),
                admin.key().as_ref(),
                mint.key().as_ref(),
                bump.to_le_bytes().as_ref(),
            ])
            .borrow()
            .as_slice()],
        ),
        amount.clone(),
    )
    .unwrap();

    solana_program::msg!(
        "{} {} {}",
        "Airdropped".to_string(),
        amount,
        "tokens to user".to_string()
    );
}

pub fn create_user_token_account_handler<'info>(
    mut user: SeahorseSigner<'info, '_>,
    mut mint: SeahorseAccount<'info, '_, Mint>,
    mut user_token_account: Empty<SeahorseAccount<'info, '_, TokenAccount>>,
) -> () {
    "\n    Create a PDA token account for a user.\n    Uses seeds to derive a deterministic address.\n    " . to_string () ;

    user_token_account.account.clone();

    solana_program::msg!(
        "{} {:?}",
        "Created token account for user:".to_string(),
        user.key()
    );

    solana_program::msg!(
        "{} {:?}",
        "Token account address:".to_string(),
        user_token_account.account.key()
    );
}

pub fn initialize_treasury_handler<'info>(
    mut admin: SeahorseSigner<'info, '_>,
    mut mint: SeahorseAccount<'info, '_, Mint>,
    mut treasury: Empty<Mutable<LoadedTreasury<'info, '_>>>,
    mut treasury_token_account: Empty<SeahorseAccount<'info, '_, TokenAccount>>,
) -> () {
    "\n    Initialize a treasury with its token account.\n    Treasury PDA owns the token account.\n    " . to_string () ;

    let mut bump = treasury.bump.unwrap();
    let mut treasury = treasury.account.clone();
    let mut treasury_token_account = treasury_token_account.account.clone();

    assign!(treasury.borrow_mut().admin, admin.key());

    assign!(treasury.borrow_mut().mint, mint.key());

    assign!(
        treasury.borrow_mut().treasury_token_account,
        treasury_token_account.key()
    );

    assign!(treasury.borrow_mut().bump, bump);

    solana_program::msg!(
        "{} {:?}",
        "Initialized treasury with token account:".to_string(),
        treasury_token_account.key()
    );
}

pub fn register_user_handler<'info>(
    mut user: SeahorseSigner<'info, '_>,
    mut mint: SeahorseAccount<'info, '_, Mint>,
    mut user_account: Empty<Mutable<LoadedUserAccount<'info, '_>>>,
    mut user_token_account: Empty<SeahorseAccount<'info, '_, TokenAccount>>,
) -> () {
    "\n    Register a user in the system and create their token account.\n    Demonstrates combining custom state accounts with token account creation.\n    " . to_string () ;

    let mut bump = user_account.bump.unwrap();
    let mut user_account = user_account.account.clone();
    let mut user_token_account = user_token_account.account.clone();

    assign!(user_account.borrow_mut().owner, user.key());

    assign!(user_account.borrow_mut().mint, mint.key());

    assign!(
        user_account.borrow_mut().token_account,
        user_token_account.key()
    );

    assign!(user_account.borrow_mut().bump, bump);

    assign!(user_account.borrow_mut().is_registered, true);

    solana_program::msg!("{} {:?}", "Registered user:".to_string(), user.key());

    solana_program::msg!(
        "{} {:?}",
        "User token account:".to_string(),
        user_token_account.key()
    );
}

pub fn transfer_tokens_handler<'info>(
    mut sender: SeahorseSigner<'info, '_>,
    mut mint: SeahorseAccount<'info, '_, Mint>,
    mut sender_token_account: SeahorseAccount<'info, '_, TokenAccount>,
    mut recipient_token_account: SeahorseAccount<'info, '_, TokenAccount>,
    mut amount: u64,
) -> () {
    "\n    Transfer tokens from sender's account to recipient's account.\n    Both accounts must already exist.\n    " . to_string () ;

    if !(amount > 0) {
        panic!("Amount must be greater than zero");
    }

    token::transfer(
        CpiContext::new(
            sender_token_account.programs.get("token_program"),
            token::Transfer {
                from: sender_token_account.to_account_info(),
                authority: sender.clone().to_account_info(),
                to: recipient_token_account.clone().to_account_info(),
            },
        ),
        amount.clone(),
    )
    .unwrap();

    solana_program::msg!(
        "{} {} {}",
        "Transferred".to_string(),
        amount,
        "tokens".to_string()
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

declare_id!("ATAPtrns1111111111111111111111111111111111AA");

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
mod ata_patterns {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    # [instruction (amount : u64)]
    pub struct AirdropToUser<'info> {
        #[account(mut)]
        pub admin: Signer<'info>,
        #[account(mut)]
        pub mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub treasury: Box<Account<'info, dot::program::Treasury>>,
        #[account(mut)]
        pub treasury_token_account: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub user_account: Box<Account<'info, dot::program::UserAccount>>,
        #[account(mut)]
        pub user_token_account: Box<Account<'info, TokenAccount>>,
        pub token_program: Program<'info, Token>,
    }

    pub fn airdrop_to_user(ctx: Context<AirdropToUser>, amount: u64) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "token_program",
            ctx.accounts.token_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let admin = SeahorseSigner {
            account: &ctx.accounts.admin,
            programs: &programs_map,
        };

        let mint = SeahorseAccount {
            account: &ctx.accounts.mint,
            programs: &programs_map,
        };

        let treasury = dot::program::Treasury::load(&mut ctx.accounts.treasury, &programs_map);
        let treasury_token_account = SeahorseAccount {
            account: &ctx.accounts.treasury_token_account,
            programs: &programs_map,
        };

        let user_account =
            dot::program::UserAccount::load(&mut ctx.accounts.user_account, &programs_map);

        let user_token_account = SeahorseAccount {
            account: &ctx.accounts.user_token_account,
            programs: &programs_map,
        };

        airdrop_to_user_handler(
            admin.clone(),
            mint.clone(),
            treasury.clone(),
            treasury_token_account.clone(),
            user_account.clone(),
            user_token_account.clone(),
            amount,
        );

        dot::program::Treasury::store(treasury);

        dot::program::UserAccount::store(user_account);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct CreateUserTokenAccount<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(mut)]
        pub mint: Box<Account<'info, Mint>>,
        # [account (init , payer = user , seeds = ["user_token" . as_bytes () . as_ref () , user . key () . as_ref () , mint . key () . as_ref ()] , bump , token :: mint = mint , token :: authority = user)]
        pub user_token_account: Box<Account<'info, TokenAccount>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
        pub token_program: Program<'info, Token>,
    }

    pub fn create_user_token_account(ctx: Context<CreateUserTokenAccount>) -> Result<()> {
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

        let mint = SeahorseAccount {
            account: &ctx.accounts.mint,
            programs: &programs_map,
        };

        let user_token_account = Empty {
            account: SeahorseAccount {
                account: &ctx.accounts.user_token_account,
                programs: &programs_map,
            },
            bump: Some(ctx.bumps.user_token_account),
        };

        create_user_token_account_handler(user.clone(), mint.clone(), user_token_account.clone());

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct InitializeTreasury<'info> {
        #[account(mut)]
        pub admin: Signer<'info>,
        #[account(mut)]
        pub mint: Box<Account<'info, Mint>>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: Treasury > () + 8 , payer = admin , seeds = ["treasury" . as_bytes () . as_ref () , admin . key () . as_ref () , mint . key () . as_ref ()] , bump)]
        pub treasury: Box<Account<'info, dot::program::Treasury>>,
        # [account (init , payer = admin , seeds = ["treasury_token" . as_bytes () . as_ref () , admin . key () . as_ref () , mint . key () . as_ref ()] , bump , token :: mint = mint , token :: authority = treasury)]
        pub treasury_token_account: Box<Account<'info, TokenAccount>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
        pub token_program: Program<'info, Token>,
    }

    pub fn initialize_treasury(ctx: Context<InitializeTreasury>) -> Result<()> {
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
        let admin = SeahorseSigner {
            account: &ctx.accounts.admin,
            programs: &programs_map,
        };

        let mint = SeahorseAccount {
            account: &ctx.accounts.mint,
            programs: &programs_map,
        };

        let treasury = Empty {
            account: dot::program::Treasury::load(&mut ctx.accounts.treasury, &programs_map),
            bump: Some(ctx.bumps.treasury),
        };

        let treasury_token_account = Empty {
            account: SeahorseAccount {
                account: &ctx.accounts.treasury_token_account,
                programs: &programs_map,
            },
            bump: Some(ctx.bumps.treasury_token_account),
        };

        initialize_treasury_handler(
            admin.clone(),
            mint.clone(),
            treasury.clone(),
            treasury_token_account.clone(),
        );

        dot::program::Treasury::store(treasury.account);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct RegisterUser<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(mut)]
        pub mint: Box<Account<'info, Mint>>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: UserAccount > () + 8 , payer = user , seeds = ["user_account" . as_bytes () . as_ref () , user . key () . as_ref () , mint . key () . as_ref ()] , bump)]
        pub user_account: Box<Account<'info, dot::program::UserAccount>>,
        # [account (init , payer = user , seeds = ["user_token" . as_bytes () . as_ref () , user . key () . as_ref () , mint . key () . as_ref ()] , bump , token :: mint = mint , token :: authority = user)]
        pub user_token_account: Box<Account<'info, TokenAccount>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
        pub token_program: Program<'info, Token>,
    }

    pub fn register_user(ctx: Context<RegisterUser>) -> Result<()> {
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

        let mint = SeahorseAccount {
            account: &ctx.accounts.mint,
            programs: &programs_map,
        };

        let user_account = Empty {
            account: dot::program::UserAccount::load(&mut ctx.accounts.user_account, &programs_map),
            bump: Some(ctx.bumps.user_account),
        };

        let user_token_account = Empty {
            account: SeahorseAccount {
                account: &ctx.accounts.user_token_account,
                programs: &programs_map,
            },
            bump: Some(ctx.bumps.user_token_account),
        };

        register_user_handler(
            user.clone(),
            mint.clone(),
            user_account.clone(),
            user_token_account.clone(),
        );

        dot::program::UserAccount::store(user_account.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (amount : u64)]
    pub struct TransferTokens<'info> {
        #[account(mut)]
        pub sender: Signer<'info>,
        #[account(mut)]
        pub mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub sender_token_account: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub recipient_token_account: Box<Account<'info, TokenAccount>>,
        pub token_program: Program<'info, Token>,
    }

    pub fn transfer_tokens(ctx: Context<TransferTokens>, amount: u64) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "token_program",
            ctx.accounts.token_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let sender = SeahorseSigner {
            account: &ctx.accounts.sender,
            programs: &programs_map,
        };

        let mint = SeahorseAccount {
            account: &ctx.accounts.mint,
            programs: &programs_map,
        };

        let sender_token_account = SeahorseAccount {
            account: &ctx.accounts.sender_token_account,
            programs: &programs_map,
        };

        let recipient_token_account = SeahorseAccount {
            account: &ctx.accounts.recipient_token_account,
            programs: &programs_map,
        };

        transfer_tokens_handler(
            sender.clone(),
            mint.clone(),
            sender_token_account.clone(),
            recipient_token_account.clone(),
            amount,
        );

        return Ok(());
    }
}

