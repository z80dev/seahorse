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
pub struct MintConfig {
    pub mint: Pubkey,
    pub authority: Pubkey,
    pub decimals: u8,
    pub total_minted: u64,
    pub total_burned: u64,
    pub bump: u8,
}

impl<'info, 'entrypoint> MintConfig {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedMintConfig<'info, 'entrypoint>> {
        let mint = account.mint.clone();
        let authority = account.authority.clone();
        let decimals = account.decimals;
        let total_minted = account.total_minted;
        let total_burned = account.total_burned;
        let bump = account.bump;

        Mutable::new(LoadedMintConfig {
            __account__: account,
            __programs__: programs_map,
            mint,
            authority,
            decimals,
            total_minted,
            total_burned,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedMintConfig>) {
        let mut loaded = loaded.borrow_mut();
        let mint = loaded.mint.clone();

        loaded.__account__.mint = mint;

        let authority = loaded.authority.clone();

        loaded.__account__.authority = authority;

        let decimals = loaded.decimals;

        loaded.__account__.decimals = decimals;

        let total_minted = loaded.total_minted;

        loaded.__account__.total_minted = total_minted;

        let total_burned = loaded.total_burned;

        loaded.__account__.total_burned = total_burned;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedMintConfig<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, MintConfig>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub mint: Pubkey,
    pub authority: Pubkey,
    pub decimals: u8,
    pub total_minted: u64,
    pub total_burned: u64,
    pub bump: u8,
}

pub fn burn_tokens_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut authority: UncheckedAccount<'info>,
    mut mint_config: Mutable<LoadedMintConfig<'info, '_>>,
    mut mint: SeahorseAccount<'info, '_, Mint>,
    mut source: SeahorseAccount<'info, '_, TokenAccount>,
    mut amount: u64,
) -> () {
    if !(amount > 0) {
        panic!("Amount must be greater than zero");
    }

    if !(source.amount >= amount) {
        panic!("Insufficient funds");
    }

    token::burn(
        CpiContext::new(
            mint.programs.get("token_program"),
            token::Burn {
                mint: mint.to_account_info(),
                authority: owner.clone().to_account_info(),
                from: source.clone().to_account_info(),
            },
        ),
        amount.clone(),
    )
    .unwrap();

    assign!(
        mint_config.borrow_mut().total_burned,
        mint_config.borrow().total_burned + amount
    );

    solana_program::msg!(
        "{} {} {}",
        "Burned".to_string(),
        amount,
        "tokens".to_string()
    );
}

pub fn create_mint_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut mint_config: Empty<Mutable<LoadedMintConfig<'info, '_>>>,
    mut mint: Empty<SeahorseAccount<'info, '_, Mint>>,
    mut decimals: u8,
) -> () {
    let mut config_bump = mint_config.bump.unwrap();
    let mut mint_config = mint_config.account.clone();

    mint.account.clone();

    assign!(mint_config.borrow_mut().mint, mint.account.key());

    assign!(mint_config.borrow_mut().authority, authority.key());

    assign!(mint_config.borrow_mut().decimals, decimals);

    assign!(mint_config.borrow_mut().total_minted, 0);

    assign!(mint_config.borrow_mut().total_burned, 0);

    assign!(mint_config.borrow_mut().bump, config_bump);

    solana_program::msg!("{} {:?}", "Created mint:".to_string(), mint.account.key());

    solana_program::msg!("{} {}", "Decimals:".to_string(), decimals);
}

pub fn mint_tokens_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut mint_config: Mutable<LoadedMintConfig<'info, '_>>,
    mut mint: SeahorseAccount<'info, '_, Mint>,
    mut destination: SeahorseAccount<'info, '_, TokenAccount>,
    mut amount: u64,
) -> () {
    if !(authority.key() == mint_config.borrow().authority) {
        panic!("Unauthorized");
    }

    if !(amount > 0) {
        panic!("Amount must be greater than zero");
    }

    let mut bump = mint_config.borrow().bump;

    token::mint_to(
        CpiContext::new_with_signer(
            mint.programs.get("token_program"),
            token::MintTo {
                mint: mint.to_account_info(),
                authority: mint_config.borrow().__account__.to_account_info(),
                to: destination.clone().to_account_info(),
            },
            &[Mutable::new(vec![
                "mint_config".to_string().as_bytes().as_ref(),
                authority.key().as_ref(),
                bump.to_le_bytes().as_ref(),
            ])
            .borrow()
            .as_slice()],
        ),
        amount.clone(),
    )
    .unwrap();

    assign!(
        mint_config.borrow_mut().total_minted,
        mint_config.borrow().total_minted + amount
    );

    solana_program::msg!(
        "{} {} {}",
        "Minted".to_string(),
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

declare_id!("MiNT5VzVZYxLYtDmJq5DEmUKDEcPnP2zGxqHKbv2PgZ");

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
mod token_mint {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    # [instruction (amount : u64)]
    pub struct BurnTokens<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        #[account(mut)]
        #[doc = "CHECK: This account is unchecked."]
        pub authority: UncheckedAccount<'info>,
        #[account(mut)]
        pub mint_config: Box<Account<'info, dot::program::MintConfig>>,
        #[account(mut)]
        pub mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub source: Box<Account<'info, TokenAccount>>,
        pub token_program: Program<'info, Token>,
    }

    pub fn burn_tokens(ctx: Context<BurnTokens>, amount: u64) -> Result<()> {
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

        let authority = &ctx.accounts.authority.clone();
        let mint_config =
            dot::program::MintConfig::load(&mut ctx.accounts.mint_config, &programs_map);

        let mint = SeahorseAccount {
            account: &ctx.accounts.mint,
            programs: &programs_map,
        };

        let source = SeahorseAccount {
            account: &ctx.accounts.source,
            programs: &programs_map,
        };

        burn_tokens_handler(
            owner.clone(),
            authority.clone(),
            mint_config.clone(),
            mint.clone(),
            source.clone(),
            amount,
        );

        dot::program::MintConfig::store(mint_config);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (decimals : u8)]
    pub struct CreateMint<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: MintConfig > () + 8 , payer = authority , seeds = ["mint_config" . as_bytes () . as_ref () , authority . key () . as_ref ()] , bump)]
        pub mint_config: Box<Account<'info, dot::program::MintConfig>>,
        # [account (init , payer = authority , seeds = ["mint" . as_bytes () . as_ref () , authority . key () . as_ref ()] , bump , mint :: decimals = decimals , mint :: authority = mint_config)]
        pub mint: Box<Account<'info, Mint>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
        pub token_program: Program<'info, Token>,
    }

    pub fn create_mint(ctx: Context<CreateMint>, decimals: u8) -> Result<()> {
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

        let mint_config = Empty {
            account: dot::program::MintConfig::load(&mut ctx.accounts.mint_config, &programs_map),
            bump: Some(ctx.bumps.mint_config),
        };

        let mint = Empty {
            account: SeahorseAccount {
                account: &ctx.accounts.mint,
                programs: &programs_map,
            },
            bump: Some(ctx.bumps.mint),
        };

        create_mint_handler(
            authority.clone(),
            mint_config.clone(),
            mint.clone(),
            decimals,
        );

        dot::program::MintConfig::store(mint_config.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (amount : u64)]
    pub struct MintTokens<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub mint_config: Box<Account<'info, dot::program::MintConfig>>,
        #[account(mut)]
        pub mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub destination: Box<Account<'info, TokenAccount>>,
        pub token_program: Program<'info, Token>,
    }

    pub fn mint_tokens(ctx: Context<MintTokens>, amount: u64) -> Result<()> {
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

        let mint_config =
            dot::program::MintConfig::load(&mut ctx.accounts.mint_config, &programs_map);

        let mint = SeahorseAccount {
            account: &ctx.accounts.mint,
            programs: &programs_map,
        };

        let destination = SeahorseAccount {
            account: &ctx.accounts.destination,
            programs: &programs_map,
        };

        mint_tokens_handler(
            authority.clone(),
            mint_config.clone(),
            mint.clone(),
            destination.clone(),
            amount,
        );

        dot::program::MintConfig::store(mint_config);

        return Ok(());
    }
}

