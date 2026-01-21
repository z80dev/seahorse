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
pub struct VestingAccount {
    pub authority: Pubkey,
    pub beneficiary: Pubkey,
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub total_amount: u64,
    pub released_amount: u64,
    pub start_slot: u64,
    pub cliff_slot: u64,
    pub end_slot: u64,
    pub is_cancelled: bool,
    pub bump: u8,
}

impl<'info, 'entrypoint> VestingAccount {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedVestingAccount<'info, 'entrypoint>> {
        let authority = account.authority.clone();
        let beneficiary = account.beneficiary.clone();
        let mint = account.mint.clone();
        let vault = account.vault.clone();
        let total_amount = account.total_amount;
        let released_amount = account.released_amount;
        let start_slot = account.start_slot;
        let cliff_slot = account.cliff_slot;
        let end_slot = account.end_slot;
        let is_cancelled = account.is_cancelled.clone();
        let bump = account.bump;

        Mutable::new(LoadedVestingAccount {
            __account__: account,
            __programs__: programs_map,
            authority,
            beneficiary,
            mint,
            vault,
            total_amount,
            released_amount,
            start_slot,
            cliff_slot,
            end_slot,
            is_cancelled,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedVestingAccount>) {
        let mut loaded = loaded.borrow_mut();
        let authority = loaded.authority.clone();

        loaded.__account__.authority = authority;

        let beneficiary = loaded.beneficiary.clone();

        loaded.__account__.beneficiary = beneficiary;

        let mint = loaded.mint.clone();

        loaded.__account__.mint = mint;

        let vault = loaded.vault.clone();

        loaded.__account__.vault = vault;

        let total_amount = loaded.total_amount;

        loaded.__account__.total_amount = total_amount;

        let released_amount = loaded.released_amount;

        loaded.__account__.released_amount = released_amount;

        let start_slot = loaded.start_slot;

        loaded.__account__.start_slot = start_slot;

        let cliff_slot = loaded.cliff_slot;

        loaded.__account__.cliff_slot = cliff_slot;

        let end_slot = loaded.end_slot;

        loaded.__account__.end_slot = end_slot;

        let is_cancelled = loaded.is_cancelled.clone();

        loaded.__account__.is_cancelled = is_cancelled;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedVestingAccount<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, VestingAccount>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub authority: Pubkey,
    pub beneficiary: Pubkey,
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub total_amount: u64,
    pub released_amount: u64,
    pub start_slot: u64,
    pub cliff_slot: u64,
    pub end_slot: u64,
    pub is_cancelled: bool,
    pub bump: u8,
}

pub fn cancel_vesting_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut vesting: Mutable<LoadedVestingAccount<'info, '_>>,
    mut vault: SeahorseAccount<'info, '_, TokenAccount>,
    mut authority_token: SeahorseAccount<'info, '_, TokenAccount>,
    mut beneficiary_token: SeahorseAccount<'info, '_, TokenAccount>,
    mut mint: SeahorseAccount<'info, '_, Mint>,
    mut clock: Sysvar<'info, Clock>,
) -> () {
    "\n    Cancel vesting schedule.\n\n    Authority can cancel vesting at any time. Vested tokens go to beneficiary,\n    unvested tokens return to authority.\n    " . to_string () ;

    if !(authority.key() == vesting.borrow().authority) {
        panic!("Unauthorized");
    }

    if !(!vesting.borrow().is_cancelled) {
        panic!("Vesting already cancelled");
    }

    let mut current_slot = clock.slot;
    let mut vested_amount = 0;

    if current_slot < vesting.borrow().cliff_slot {
        vested_amount = 0;
    } else {
        if current_slot >= vesting.borrow().end_slot {
            vested_amount = vesting.borrow().total_amount;
        } else {
            let mut vesting_period = vesting.borrow().end_slot - vesting.borrow().cliff_slot;
            let mut elapsed = current_slot - vesting.borrow().cliff_slot;

            vested_amount = (vesting.borrow().total_amount * elapsed) / vesting_period;
        }
    }

    let mut unreleased_vested = vested_amount - vesting.borrow().released_amount;
    let mut unvested = vesting.borrow().total_amount - vested_amount;
    let mut vesting_bump = vesting.borrow().bump;
    let mut beneficiary = vesting.borrow().beneficiary;

    if unreleased_vested > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                vault.programs.get("token_program"),
                token::Transfer {
                    from: vault.to_account_info(),
                    authority: vesting.borrow().__account__.to_account_info(),
                    to: beneficiary_token.clone().to_account_info(),
                },
                &[Mutable::new(vec![
                    "vesting".to_string().as_bytes().as_ref(),
                    beneficiary.as_ref(),
                    mint.key().as_ref(),
                    vesting_bump.to_le_bytes().as_ref(),
                ])
                .borrow()
                .as_slice()],
            ),
            unreleased_vested.clone(),
        )
        .unwrap();
    }

    if unvested > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                vault.programs.get("token_program"),
                token::Transfer {
                    from: vault.to_account_info(),
                    authority: vesting.borrow().__account__.to_account_info(),
                    to: authority_token.clone().to_account_info(),
                },
                &[Mutable::new(vec![
                    "vesting".to_string().as_bytes().as_ref(),
                    beneficiary.as_ref(),
                    mint.key().as_ref(),
                    vesting_bump.to_le_bytes().as_ref(),
                ])
                .borrow()
                .as_slice()],
            ),
            unvested.clone(),
        )
        .unwrap();
    }

    assign!(vesting.borrow_mut().is_cancelled, true);

    assign!(vesting.borrow_mut().released_amount, vested_amount);
}

pub fn claim_vested_handler<'info>(
    mut beneficiary: SeahorseSigner<'info, '_>,
    mut vesting: Mutable<LoadedVestingAccount<'info, '_>>,
    mut vault: SeahorseAccount<'info, '_, TokenAccount>,
    mut beneficiary_token: SeahorseAccount<'info, '_, TokenAccount>,
    mut mint: SeahorseAccount<'info, '_, Mint>,
    mut clock: Sysvar<'info, Clock>,
) -> () {
    "\n    Claim vested tokens.\n\n    Calculates the amount of tokens that have vested and haven't been\n    released yet, then transfers them to the beneficiary.\n    " . to_string () ;

    if !(beneficiary.key() == vesting.borrow().beneficiary) {
        panic!("Unauthorized");
    }

    if !(!vesting.borrow().is_cancelled) {
        panic!("Vesting has been cancelled");
    }

    let mut current_slot = clock.slot;
    let mut vested_amount = 0;

    if current_slot < vesting.borrow().cliff_slot {
        vested_amount = 0;
    } else {
        if current_slot >= vesting.borrow().end_slot {
            vested_amount = vesting.borrow().total_amount;
        } else {
            let mut vesting_period = vesting.borrow().end_slot - vesting.borrow().cliff_slot;
            let mut elapsed = current_slot - vesting.borrow().cliff_slot;

            vested_amount = (vesting.borrow().total_amount * elapsed) / vesting_period;
        }
    }

    let mut releasable = vested_amount - vesting.borrow().released_amount;

    if !(releasable > 0) {
        panic!("No tokens available to claim");
    }

    let mut vesting_bump = vesting.borrow().bump;

    token::transfer(
        CpiContext::new_with_signer(
            vault.programs.get("token_program"),
            token::Transfer {
                from: vault.to_account_info(),
                authority: vesting.borrow().__account__.to_account_info(),
                to: beneficiary_token.clone().to_account_info(),
            },
            &[Mutable::new(vec![
                "vesting".to_string().as_bytes().as_ref(),
                beneficiary.key().as_ref(),
                mint.key().as_ref(),
                vesting_bump.to_le_bytes().as_ref(),
            ])
            .borrow()
            .as_slice()],
        ),
        releasable.clone(),
    )
    .unwrap();

    assign!(
        vesting.borrow_mut().released_amount,
        vesting.borrow().released_amount + releasable
    );
}

pub fn create_vesting_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut beneficiary: Pubkey,
    mut mint: SeahorseAccount<'info, '_, Mint>,
    mut authority_token: SeahorseAccount<'info, '_, TokenAccount>,
    mut vesting: Empty<Mutable<LoadedVestingAccount<'info, '_>>>,
    mut vault: Empty<SeahorseAccount<'info, '_, TokenAccount>>,
    mut clock: Sysvar<'info, Clock>,
    mut amount: u64,
    mut cliff_slots: u64,
    mut vesting_duration_slots: u64,
) -> () {
    "\n    Create a new vesting schedule.\n\n    Args:\n        authority: Creator and cancellation authority\n        beneficiary: Recipient of vested tokens\n        mint: Token mint\n        authority_token: Authority's token account (source of tokens)\n        vesting: Empty vesting account to initialize\n        vault: Empty token account for vault\n        clock: Clock sysvar for time\n        amount: Total tokens to vest\n        cliff_slots: Slots until cliff (0 for no cliff)\n        vesting_duration_slots: Slots for linear vesting after cliff\n    " . to_string () ;

    if !(amount > 0) {
        panic!("Amount must be greater than zero");
    }

    if !(vesting_duration_slots > 0) {
        panic!("Vesting duration must be greater than zero");
    }

    let mut vesting_bump = vesting.bump.unwrap();
    let mut vesting = vesting.account.clone();
    let mut vault = vault.account.clone();
    let mut current_slot = clock.slot;

    token::transfer(
        CpiContext::new(
            authority_token.programs.get("token_program"),
            token::Transfer {
                from: authority_token.to_account_info(),
                authority: authority.clone().to_account_info(),
                to: vault.clone().to_account_info(),
            },
        ),
        amount.clone(),
    )
    .unwrap();

    assign!(vesting.borrow_mut().authority, authority.key());

    assign!(vesting.borrow_mut().beneficiary, beneficiary);

    assign!(vesting.borrow_mut().mint, mint.key());

    assign!(vesting.borrow_mut().vault, vault.key());

    assign!(vesting.borrow_mut().total_amount, amount);

    assign!(vesting.borrow_mut().released_amount, 0);

    assign!(vesting.borrow_mut().start_slot, current_slot);

    assign!(vesting.borrow_mut().cliff_slot, current_slot + cliff_slots);

    assign!(
        vesting.borrow_mut().end_slot,
        (current_slot + cliff_slots) + vesting_duration_slots
    );

    assign!(vesting.borrow_mut().is_cancelled, false);

    assign!(vesting.borrow_mut().bump, vesting_bump);
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

declare_id!("VESTngW9a1XxRgwWhmK7PNYWMFp4mW8q3xk2BgfCNih");

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
mod vesting {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    pub struct CancelVesting<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub vesting: Box<Account<'info, dot::program::VestingAccount>>,
        #[account(mut)]
        pub vault: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub authority_token: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub beneficiary_token: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub mint: Box<Account<'info, Mint>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
        pub token_program: Program<'info, Token>,
    }

    pub fn cancel_vesting(ctx: Context<CancelVesting>) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "token_program",
            ctx.accounts.token_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let vesting = dot::program::VestingAccount::load(&mut ctx.accounts.vesting, &programs_map);
        let vault = SeahorseAccount {
            account: &ctx.accounts.vault,
            programs: &programs_map,
        };

        let authority_token = SeahorseAccount {
            account: &ctx.accounts.authority_token,
            programs: &programs_map,
        };

        let beneficiary_token = SeahorseAccount {
            account: &ctx.accounts.beneficiary_token,
            programs: &programs_map,
        };

        let mint = SeahorseAccount {
            account: &ctx.accounts.mint,
            programs: &programs_map,
        };

        let clock = &ctx.accounts.clock.clone();

        cancel_vesting_handler(
            authority.clone(),
            vesting.clone(),
            vault.clone(),
            authority_token.clone(),
            beneficiary_token.clone(),
            mint.clone(),
            clock.clone(),
        );

        dot::program::VestingAccount::store(vesting);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct ClaimVested<'info> {
        #[account(mut)]
        pub beneficiary: Signer<'info>,
        #[account(mut)]
        pub vesting: Box<Account<'info, dot::program::VestingAccount>>,
        #[account(mut)]
        pub vault: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub beneficiary_token: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub mint: Box<Account<'info, Mint>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
        pub token_program: Program<'info, Token>,
    }

    pub fn claim_vested(ctx: Context<ClaimVested>) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "token_program",
            ctx.accounts.token_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let beneficiary = SeahorseSigner {
            account: &ctx.accounts.beneficiary,
            programs: &programs_map,
        };

        let vesting = dot::program::VestingAccount::load(&mut ctx.accounts.vesting, &programs_map);
        let vault = SeahorseAccount {
            account: &ctx.accounts.vault,
            programs: &programs_map,
        };

        let beneficiary_token = SeahorseAccount {
            account: &ctx.accounts.beneficiary_token,
            programs: &programs_map,
        };

        let mint = SeahorseAccount {
            account: &ctx.accounts.mint,
            programs: &programs_map,
        };

        let clock = &ctx.accounts.clock.clone();

        claim_vested_handler(
            beneficiary.clone(),
            vesting.clone(),
            vault.clone(),
            beneficiary_token.clone(),
            mint.clone(),
            clock.clone(),
        );

        dot::program::VestingAccount::store(vesting);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (beneficiary : Pubkey , amount : u64 , cliff_slots : u64 , vesting_duration_slots : u64)]
    pub struct CreateVesting<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub authority_token: Box<Account<'info, TokenAccount>>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: VestingAccount > () + 8 , payer = authority , seeds = ["vesting" . as_bytes () . as_ref () , beneficiary . as_ref () , mint . key () . as_ref ()] , bump)]
        pub vesting: Box<Account<'info, dot::program::VestingAccount>>,
        # [account (init , payer = authority , seeds = ["vesting_vault" . as_bytes () . as_ref () , beneficiary . as_ref () , mint . key () . as_ref ()] , bump , token :: mint = mint , token :: authority = vesting)]
        pub vault: Box<Account<'info, TokenAccount>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
        pub token_program: Program<'info, Token>,
    }

    pub fn create_vesting(
        ctx: Context<CreateVesting>,
        beneficiary: Pubkey,
        amount: u64,
        cliff_slots: u64,
        vesting_duration_slots: u64,
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
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let mint = SeahorseAccount {
            account: &ctx.accounts.mint,
            programs: &programs_map,
        };

        let authority_token = SeahorseAccount {
            account: &ctx.accounts.authority_token,
            programs: &programs_map,
        };

        let vesting = Empty {
            account: dot::program::VestingAccount::load(&mut ctx.accounts.vesting, &programs_map),
            bump: Some(ctx.bumps.vesting),
        };

        let vault = Empty {
            account: SeahorseAccount {
                account: &ctx.accounts.vault,
                programs: &programs_map,
            },
            bump: Some(ctx.bumps.vault),
        };

        let clock = &ctx.accounts.clock.clone();

        create_vesting_handler(
            authority.clone(),
            beneficiary,
            mint.clone(),
            authority_token.clone(),
            vesting.clone(),
            vault.clone(),
            clock.clone(),
            amount,
            cliff_slots,
            vesting_duration_slots,
        );

        dot::program::VestingAccount::store(vesting.account);

        return Ok(());
    }
}

