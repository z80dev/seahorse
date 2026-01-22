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
pub struct Escrow {
    pub escrow_id: u64,
    pub maker: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub token_a_amount: u64,
    pub token_b_wanted_amount: u64,
    pub bump: u8,
}

impl<'info, 'entrypoint> Escrow {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedEscrow<'info, 'entrypoint>> {
        let escrow_id = account.escrow_id;
        let maker = account.maker.clone();
        let mint_a = account.mint_a.clone();
        let mint_b = account.mint_b.clone();
        let token_a_amount = account.token_a_amount;
        let token_b_wanted_amount = account.token_b_wanted_amount;
        let bump = account.bump;

        Mutable::new(LoadedEscrow {
            __account__: account,
            __programs__: programs_map,
            escrow_id,
            maker,
            mint_a,
            mint_b,
            token_a_amount,
            token_b_wanted_amount,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedEscrow>) {
        let mut loaded = loaded.borrow_mut();
        let escrow_id = loaded.escrow_id;

        loaded.__account__.escrow_id = escrow_id;

        let maker = loaded.maker.clone();

        loaded.__account__.maker = maker;

        let mint_a = loaded.mint_a.clone();

        loaded.__account__.mint_a = mint_a;

        let mint_b = loaded.mint_b.clone();

        loaded.__account__.mint_b = mint_b;

        let token_a_amount = loaded.token_a_amount;

        loaded.__account__.token_a_amount = token_a_amount;

        let token_b_wanted_amount = loaded.token_b_wanted_amount;

        loaded.__account__.token_b_wanted_amount = token_b_wanted_amount;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedEscrow<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, Escrow>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub escrow_id: u64,
    pub maker: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub token_a_amount: u64,
    pub token_b_wanted_amount: u64,
    pub bump: u8,
}

pub fn cancel_escrow_handler<'info>(
    mut maker: SeahorseSigner<'info, '_>,
    mut mint_a: SeahorseAccount<'info, '_, Mint>,
    mut maker_token_account_a: SeahorseAccount<'info, '_, TokenAccount>,
    mut escrow: Mutable<LoadedEscrow<'info, '_>>,
    mut vault: SeahorseAccount<'info, '_, TokenAccount>,
) -> () {
    if !(escrow.borrow().maker == maker.key()) {
        panic!("Unauthorized");
    }

    let mut bump = escrow.borrow().bump;

    token::transfer(
        CpiContext::new_with_signer(
            vault.programs.get("token_program"),
            token::Transfer {
                from: vault.to_account_info(),
                authority: escrow.borrow().__account__.to_account_info(),
                to: maker_token_account_a.clone().to_account_info(),
            },
            &[Mutable::new(vec![
                "escrow".to_string().as_bytes().as_ref(),
                escrow.borrow().escrow_id.to_le_bytes().as_ref(),
                bump.to_le_bytes().as_ref(),
            ])
            .borrow()
            .as_slice()],
        ),
        escrow.borrow().token_a_amount.clone(),
    )
    .unwrap();

    solana_program::msg!(
        "{} {}",
        "Escrow cancelled:".to_string(),
        escrow.borrow().escrow_id
    );
}

pub fn make_escrow_handler<'info>(
    mut maker: SeahorseSigner<'info, '_>,
    mut mint_a: SeahorseAccount<'info, '_, Mint>,
    mut mint_b: SeahorseAccount<'info, '_, Mint>,
    mut maker_token_account_a: SeahorseAccount<'info, '_, TokenAccount>,
    mut escrow: Empty<Mutable<LoadedEscrow<'info, '_>>>,
    mut vault: Empty<SeahorseAccount<'info, '_, TokenAccount>>,
    mut escrow_id: u64,
    mut token_a_amount: u64,
    mut token_b_wanted_amount: u64,
) -> () {
    if !(token_a_amount > 0) {
        panic!("Amount must be greater than zero");
    }

    if !(token_b_wanted_amount > 0) {
        panic!("Amount must be greater than zero");
    }

    let mut bump = escrow.bump.unwrap();
    let mut escrow = escrow.account.clone();
    let mut vault = vault.account.clone();

    token::transfer(
        CpiContext::new(
            maker_token_account_a.programs.get("token_program"),
            token::Transfer {
                from: maker_token_account_a.to_account_info(),
                authority: maker.clone().to_account_info(),
                to: vault.clone().to_account_info(),
            },
        ),
        token_a_amount.clone(),
    )
    .unwrap();

    assign!(escrow.borrow_mut().escrow_id, escrow_id);

    assign!(escrow.borrow_mut().maker, maker.key());

    assign!(escrow.borrow_mut().mint_a, mint_a.key());

    assign!(escrow.borrow_mut().mint_b, mint_b.key());

    assign!(escrow.borrow_mut().token_a_amount, token_a_amount);

    assign!(
        escrow.borrow_mut().token_b_wanted_amount,
        token_b_wanted_amount
    );

    assign!(escrow.borrow_mut().bump, bump);

    solana_program::msg!("{} {}", "Escrow created:".to_string(), escrow_id);
}

pub fn take_escrow_handler<'info>(
    mut taker: SeahorseSigner<'info, '_>,
    mut maker: UncheckedAccount<'info>,
    mut mint_a: SeahorseAccount<'info, '_, Mint>,
    mut mint_b: SeahorseAccount<'info, '_, Mint>,
    mut taker_token_account_a: SeahorseAccount<'info, '_, TokenAccount>,
    mut taker_token_account_b: SeahorseAccount<'info, '_, TokenAccount>,
    mut maker_token_account_b: SeahorseAccount<'info, '_, TokenAccount>,
    mut escrow: Mutable<LoadedEscrow<'info, '_>>,
    mut vault: SeahorseAccount<'info, '_, TokenAccount>,
) -> () {
    if !(escrow.borrow().maker == maker.key()) {
        panic!("Invalid maker");
    }

    token::transfer(
        CpiContext::new(
            taker_token_account_b.programs.get("token_program"),
            token::Transfer {
                from: taker_token_account_b.to_account_info(),
                authority: taker.clone().to_account_info(),
                to: maker_token_account_b.clone().to_account_info(),
            },
        ),
        escrow.borrow().token_b_wanted_amount.clone(),
    )
    .unwrap();

    let mut bump = escrow.borrow().bump;

    token::transfer(
        CpiContext::new_with_signer(
            vault.programs.get("token_program"),
            token::Transfer {
                from: vault.to_account_info(),
                authority: escrow.borrow().__account__.to_account_info(),
                to: taker_token_account_a.clone().to_account_info(),
            },
            &[Mutable::new(vec![
                "escrow".to_string().as_bytes().as_ref(),
                escrow.borrow().escrow_id.to_le_bytes().as_ref(),
                bump.to_le_bytes().as_ref(),
            ])
            .borrow()
            .as_slice()],
        ),
        escrow.borrow().token_a_amount.clone(),
    )
    .unwrap();

    solana_program::msg!(
        "{} {}",
        "Escrow completed:".to_string(),
        escrow.borrow().escrow_id
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

declare_id!("7bNQSAJXfZjwTP86A3Z53WP8VSEHT4qFY4LMcepq6MPm");

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
mod escrow {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    pub struct CancelEscrow<'info> {
        #[account(mut)]
        pub maker: Signer<'info>,
        #[account(mut)]
        pub mint_a: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub maker_token_account_a: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub escrow: Box<Account<'info, dot::program::Escrow>>,
        #[account(mut)]
        pub vault: Box<Account<'info, TokenAccount>>,
        pub token_program: Program<'info, Token>,
    }

    pub fn cancel_escrow(ctx: Context<CancelEscrow>) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "token_program",
            ctx.accounts.token_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let maker = SeahorseSigner {
            account: &ctx.accounts.maker,
            programs: &programs_map,
        };

        let mint_a = SeahorseAccount {
            account: &ctx.accounts.mint_a,
            programs: &programs_map,
        };

        let maker_token_account_a = SeahorseAccount {
            account: &ctx.accounts.maker_token_account_a,
            programs: &programs_map,
        };

        let escrow = dot::program::Escrow::load(&mut ctx.accounts.escrow, &programs_map);
        let vault = SeahorseAccount {
            account: &ctx.accounts.vault,
            programs: &programs_map,
        };

        cancel_escrow_handler(
            maker.clone(),
            mint_a.clone(),
            maker_token_account_a.clone(),
            escrow.clone(),
            vault.clone(),
        );

        dot::program::Escrow::store(escrow);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (escrow_id : u64 , token_a_amount : u64 , token_b_wanted_amount : u64)]
    pub struct MakeEscrow<'info> {
        #[account(mut)]
        pub maker: Signer<'info>,
        #[account(mut)]
        pub mint_a: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub mint_b: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub maker_token_account_a: Box<Account<'info, TokenAccount>>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: Escrow > () + 8 , payer = maker , seeds = ["escrow" . as_bytes () . as_ref () , escrow_id . to_le_bytes () . as_ref ()] , bump)]
        pub escrow: Box<Account<'info, dot::program::Escrow>>,
        # [account (init , payer = maker , seeds = ["vault" . as_bytes () . as_ref () , escrow_id . to_le_bytes () . as_ref ()] , bump , token :: mint = mint_a , token :: authority = escrow)]
        pub vault: Box<Account<'info, TokenAccount>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
        pub token_program: Program<'info, Token>,
    }

    pub fn make_escrow(
        ctx: Context<MakeEscrow>,
        escrow_id: u64,
        token_a_amount: u64,
        token_b_wanted_amount: u64,
    ) -> Result<()> {
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
        let maker = SeahorseSigner {
            account: &ctx.accounts.maker,
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

        let maker_token_account_a = SeahorseAccount {
            account: &ctx.accounts.maker_token_account_a,
            programs: &programs_map,
        };

        let escrow = Empty {
            account: dot::program::Escrow::load(&mut ctx.accounts.escrow, &programs_map),
            bump: Some(ctx.bumps.escrow),
        };

        let vault = Empty {
            account: SeahorseAccount {
                account: &ctx.accounts.vault,
                programs: &programs_map,
            },
            bump: Some(ctx.bumps.vault),
        };

        make_escrow_handler(
            maker.clone(),
            mint_a.clone(),
            mint_b.clone(),
            maker_token_account_a.clone(),
            escrow.clone(),
            vault.clone(),
            escrow_id,
            token_a_amount,
            token_b_wanted_amount,
        );

        dot::program::Escrow::store(escrow.account);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct TakeEscrow<'info> {
        #[account(mut)]
        pub taker: Signer<'info>,
        #[account(mut)]
        #[doc = "CHECK: This account is unchecked."]
        pub maker: UncheckedAccount<'info>,
        #[account(mut)]
        pub mint_a: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub mint_b: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub taker_token_account_a: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub taker_token_account_b: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub maker_token_account_b: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub escrow: Box<Account<'info, dot::program::Escrow>>,
        #[account(mut)]
        pub vault: Box<Account<'info, TokenAccount>>,
        pub token_program: Program<'info, Token>,
    }

    pub fn take_escrow(ctx: Context<TakeEscrow>) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "token_program",
            ctx.accounts.token_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let taker = SeahorseSigner {
            account: &ctx.accounts.taker,
            programs: &programs_map,
        };

        let maker = &ctx.accounts.maker.clone();
        let mint_a = SeahorseAccount {
            account: &ctx.accounts.mint_a,
            programs: &programs_map,
        };

        let mint_b = SeahorseAccount {
            account: &ctx.accounts.mint_b,
            programs: &programs_map,
        };

        let taker_token_account_a = SeahorseAccount {
            account: &ctx.accounts.taker_token_account_a,
            programs: &programs_map,
        };

        let taker_token_account_b = SeahorseAccount {
            account: &ctx.accounts.taker_token_account_b,
            programs: &programs_map,
        };

        let maker_token_account_b = SeahorseAccount {
            account: &ctx.accounts.maker_token_account_b,
            programs: &programs_map,
        };

        let escrow = dot::program::Escrow::load(&mut ctx.accounts.escrow, &programs_map);
        let vault = SeahorseAccount {
            account: &ctx.accounts.vault,
            programs: &programs_map,
        };

        take_escrow_handler(
            taker.clone(),
            maker.clone(),
            mint_a.clone(),
            mint_b.clone(),
            taker_token_account_a.clone(),
            taker_token_account_b.clone(),
            maker_token_account_b.clone(),
            escrow.clone(),
            vault.clone(),
        );

        dot::program::Escrow::store(escrow);

        return Ok(());
    }
}

