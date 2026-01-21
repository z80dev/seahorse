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
pub struct CpiVault {
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub total_deposited: u64,
    pub total_withdrawn: u64,
    pub transfer_count: u64,
    pub bump: u8,
}

impl<'info, 'entrypoint> CpiVault {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedCpiVault<'info, 'entrypoint>> {
        let authority = account.authority.clone();
        let mint = account.mint.clone();
        let total_deposited = account.total_deposited;
        let total_withdrawn = account.total_withdrawn;
        let transfer_count = account.transfer_count;
        let bump = account.bump;

        Mutable::new(LoadedCpiVault {
            __account__: account,
            __programs__: programs_map,
            authority,
            mint,
            total_deposited,
            total_withdrawn,
            transfer_count,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedCpiVault>) {
        let mut loaded = loaded.borrow_mut();
        let authority = loaded.authority.clone();

        loaded.__account__.authority = authority;

        let mint = loaded.mint.clone();

        loaded.__account__.mint = mint;

        let total_deposited = loaded.total_deposited;

        loaded.__account__.total_deposited = total_deposited;

        let total_withdrawn = loaded.total_withdrawn;

        loaded.__account__.total_withdrawn = total_withdrawn;

        let transfer_count = loaded.transfer_count;

        loaded.__account__.transfer_count = transfer_count;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedCpiVault<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, CpiVault>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub total_deposited: u64,
    pub total_withdrawn: u64,
    pub transfer_count: u64,
    pub bump: u8,
}

#[account]
#[derive(Debug)]
pub struct MintConfig {
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub total_minted: u64,
    pub operation_count: u64,
    pub bump: u8,
}

impl<'info, 'entrypoint> MintConfig {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedMintConfig<'info, 'entrypoint>> {
        let authority = account.authority.clone();
        let mint = account.mint.clone();
        let total_minted = account.total_minted;
        let operation_count = account.operation_count;
        let bump = account.bump;

        Mutable::new(LoadedMintConfig {
            __account__: account,
            __programs__: programs_map,
            authority,
            mint,
            total_minted,
            operation_count,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedMintConfig>) {
        let mut loaded = loaded.borrow_mut();
        let authority = loaded.authority.clone();

        loaded.__account__.authority = authority;

        let mint = loaded.mint.clone();

        loaded.__account__.mint = mint;

        let total_minted = loaded.total_minted;

        loaded.__account__.total_minted = total_minted;

        let operation_count = loaded.operation_count;

        loaded.__account__.operation_count = operation_count;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedMintConfig<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, MintConfig>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub total_minted: u64,
    pub operation_count: u64,
    pub bump: u8,
}

pub fn basic_transfer_to_vault_handler<'info>(
    mut user: SeahorseSigner<'info, '_>,
    mut vault: Mutable<LoadedCpiVault<'info, '_>>,
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
        vault.borrow_mut().total_deposited,
        vault.borrow().total_deposited + amount
    );

    assign!(
        vault.borrow_mut().transfer_count,
        vault.borrow().transfer_count + 1
    );

    solana_program::msg!(
        "{} {} {}",
        "Basic transfer: ".to_string(),
        amount,
        " tokens deposited".to_string()
    );
}

pub fn chained_mint_and_transfer_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut mint_config: Mutable<LoadedMintConfig<'info, '_>>,
    mut mint: SeahorseAccount<'info, '_, Mint>,
    mut intermediate_account: SeahorseAccount<'info, '_, TokenAccount>,
    mut destination_account: SeahorseAccount<'info, '_, TokenAccount>,
    mut mint_amount: u64,
    mut transfer_amount: u64,
) -> () {
    if !(authority.key() == mint_config.borrow().authority) {
        panic!("Unauthorized");
    }

    if !(mint_amount > 0) {
        panic!("Amount must be greater than zero");
    }

    if !(transfer_amount <= mint_amount) {
        panic!("Transfer amount exceeds minted amount");
    }

    let mut bump = mint_config.borrow().bump;

    token::mint_to(
        CpiContext::new_with_signer(
            mint.programs.get("token_program"),
            token::MintTo {
                mint: mint.to_account_info(),
                authority: mint_config.borrow().__account__.to_account_info(),
                to: intermediate_account.clone().to_account_info(),
            },
            &[Mutable::new(vec![
                "mint_config".to_string().as_bytes().as_ref(),
                authority.key().as_ref(),
                bump.to_le_bytes().as_ref(),
            ])
            .borrow()
            .as_slice()],
        ),
        mint_amount.clone(),
    )
    .unwrap();

    token::transfer(
        CpiContext::new(
            intermediate_account.programs.get("token_program"),
            token::Transfer {
                from: intermediate_account.to_account_info(),
                authority: authority.clone().to_account_info(),
                to: destination_account.clone().to_account_info(),
            },
        ),
        transfer_amount.clone(),
    )
    .unwrap();

    assign!(
        mint_config.borrow_mut().total_minted,
        mint_config.borrow().total_minted + mint_amount
    );

    assign!(
        mint_config.borrow_mut().operation_count,
        mint_config.borrow().operation_count + 1
    );

    solana_program::msg!(
        "{} {} {} {}",
        "Chained CPIs: minted ".to_string(),
        mint_amount,
        " transferred ".to_string(),
        transfer_amount
    );
}

pub fn get_vault_info_handler<'info>(mut vault: Mutable<LoadedCpiVault<'info, '_>>) -> () {
    solana_program::msg!(
        "{} {:?}",
        "Vault authority: ".to_string(),
        vault.borrow().authority
    );

    solana_program::msg!("{} {:?}", "Vault mint: ".to_string(), vault.borrow().mint);

    solana_program::msg!(
        "{} {}",
        "Total deposited: ".to_string(),
        vault.borrow().total_deposited
    );

    solana_program::msg!(
        "{} {}",
        "Total withdrawn: ".to_string(),
        vault.borrow().total_withdrawn
    );

    solana_program::msg!(
        "{} {}",
        "Transfer count: ".to_string(),
        vault.borrow().transfer_count
    );

    solana_program::msg!(
        "{} {}",
        "Net balance: ".to_string(),
        (vault.borrow().total_deposited - vault.borrow().total_withdrawn)
    );
}

pub fn initialize_mint_config_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut mint_config: Empty<Mutable<LoadedMintConfig<'info, '_>>>,
    mut mint: SeahorseAccount<'info, '_, Mint>,
) -> () {
    let mut bump = mint_config.bump.unwrap();
    let mut mint_config = mint_config.account.clone();

    assign!(mint_config.borrow_mut().authority, authority.key());

    assign!(mint_config.borrow_mut().mint, mint.key());

    assign!(mint_config.borrow_mut().total_minted, 0);

    assign!(mint_config.borrow_mut().operation_count, 0);

    assign!(mint_config.borrow_mut().bump, bump);

    solana_program::msg!(
        "{} {:?}",
        "MintConfig initialized: authority=".to_string(),
        authority.key()
    );
}

pub fn initialize_vault_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut vault: Empty<Mutable<LoadedCpiVault<'info, '_>>>,
    mut vault_token_account: Empty<SeahorseAccount<'info, '_, TokenAccount>>,
    mut mint: SeahorseAccount<'info, '_, Mint>,
) -> () {
    let mut bump = vault.bump.unwrap();
    let mut vault = vault.account.clone();

    vault_token_account.account.clone();

    assign!(vault.borrow_mut().authority, authority.key());

    assign!(vault.borrow_mut().mint, mint.key());

    assign!(vault.borrow_mut().total_deposited, 0);

    assign!(vault.borrow_mut().total_withdrawn, 0);

    assign!(vault.borrow_mut().transfer_count, 0);

    assign!(vault.borrow_mut().bump, bump);

    solana_program::msg!(
        "{} {:?}",
        "Vault initialized: authority=".to_string(),
        authority.key()
    );
}

pub fn pda_signed_transfer_from_vault_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut vault: Mutable<LoadedCpiVault<'info, '_>>,
    mut user_token_account: SeahorseAccount<'info, '_, TokenAccount>,
    mut vault_token_account: SeahorseAccount<'info, '_, TokenAccount>,
    mut mint: SeahorseAccount<'info, '_, Mint>,
    mut amount: u64,
) -> () {
    if !(authority.key() == vault.borrow().authority) {
        panic!("Unauthorized: caller is not the authority");
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
                to: user_token_account.clone().to_account_info(),
            },
            &[Mutable::new(vec![
                "vault".to_string().as_bytes().as_ref(),
                authority.key().as_ref(),
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
        vault.borrow_mut().total_withdrawn,
        vault.borrow().total_withdrawn + amount
    );

    assign!(
        vault.borrow_mut().transfer_count,
        vault.borrow().transfer_count + 1
    );

    solana_program::msg!(
        "{} {} {}",
        "PDA-signed transfer: ".to_string(),
        amount,
        " tokens withdrawn".to_string()
    );
}

pub fn validated_burn_handler<'info>(
    mut owner: SeahorseSigner<'info, '_>,
    mut source_account: SeahorseAccount<'info, '_, TokenAccount>,
    mut mint: SeahorseAccount<'info, '_, Mint>,
    mut amount: u64,
) -> () {
    if !(amount > 0) {
        panic!("Amount must be greater than zero");
    }

    let mut available_balance = source_account.amount;

    if !(available_balance >= amount) {
        panic!("Insufficient balance for burn");
    }

    token::burn(
        CpiContext::new(
            mint.programs.get("token_program"),
            token::Burn {
                mint: mint.to_account_info(),
                authority: owner.clone().to_account_info(),
                from: source_account.clone().to_account_info(),
            },
        ),
        amount.clone(),
    )
    .unwrap();

    solana_program::msg!(
        "{} {} {}",
        "Validated burn: ".to_string(),
        amount,
        " tokens burned".to_string()
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

declare_id!("Cpi1Pattrn111111111111111111111111111111111");

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
mod cpi_patterns {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    # [instruction (amount : u64)]
    pub struct BasicTransferToVault<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(mut)]
        pub vault: Box<Account<'info, dot::program::CpiVault>>,
        #[account(mut)]
        pub user_token_account: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub vault_token_account: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub mint: Box<Account<'info, Mint>>,
        pub token_program: Program<'info, Token>,
    }

    pub fn basic_transfer_to_vault(ctx: Context<BasicTransferToVault>, amount: u64) -> Result<()> {
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

        let vault = dot::program::CpiVault::load(&mut ctx.accounts.vault, &programs_map);
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

        basic_transfer_to_vault_handler(
            user.clone(),
            vault.clone(),
            user_token_account.clone(),
            vault_token_account.clone(),
            mint.clone(),
            amount,
        );

        dot::program::CpiVault::store(vault);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (mint_amount : u64 , transfer_amount : u64)]
    pub struct ChainedMintAndTransfer<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub mint_config: Box<Account<'info, dot::program::MintConfig>>,
        #[account(mut)]
        pub mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub intermediate_account: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub destination_account: Box<Account<'info, TokenAccount>>,
        pub token_program: Program<'info, Token>,
    }

    pub fn chained_mint_and_transfer(
        ctx: Context<ChainedMintAndTransfer>,
        mint_amount: u64,
        transfer_amount: u64,
    ) -> Result<()> {
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

        let intermediate_account = SeahorseAccount {
            account: &ctx.accounts.intermediate_account,
            programs: &programs_map,
        };

        let destination_account = SeahorseAccount {
            account: &ctx.accounts.destination_account,
            programs: &programs_map,
        };

        chained_mint_and_transfer_handler(
            authority.clone(),
            mint_config.clone(),
            mint.clone(),
            intermediate_account.clone(),
            destination_account.clone(),
            mint_amount,
            transfer_amount,
        );

        dot::program::MintConfig::store(mint_config);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct GetVaultInfo<'info> {
        #[account(mut)]
        pub vault: Box<Account<'info, dot::program::CpiVault>>,
    }

    pub fn get_vault_info(ctx: Context<GetVaultInfo>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let vault = dot::program::CpiVault::load(&mut ctx.accounts.vault, &programs_map);

        get_vault_info_handler(vault.clone());

        dot::program::CpiVault::store(vault);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct InitializeMintConfig<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: MintConfig > () + 8 , payer = authority , seeds = ["mint_config" . as_bytes () . as_ref () , authority . key () . as_ref ()] , bump)]
        pub mint_config: Box<Account<'info, dot::program::MintConfig>>,
        #[account(mut)]
        pub mint: Box<Account<'info, Mint>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn initialize_mint_config(ctx: Context<InitializeMintConfig>) -> Result<()> {
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

        let mint_config = Empty {
            account: dot::program::MintConfig::load(&mut ctx.accounts.mint_config, &programs_map),
            bump: Some(ctx.bumps.mint_config),
        };

        let mint = SeahorseAccount {
            account: &ctx.accounts.mint,
            programs: &programs_map,
        };

        initialize_mint_config_handler(authority.clone(), mint_config.clone(), mint.clone());

        dot::program::MintConfig::store(mint_config.account);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct InitializeVault<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: CpiVault > () + 8 , payer = authority , seeds = ["vault" . as_bytes () . as_ref () , authority . key () . as_ref () , mint . key () . as_ref ()] , bump)]
        pub vault: Box<Account<'info, dot::program::CpiVault>>,
        # [account (init , payer = authority , seeds = ["vault_token" . as_bytes () . as_ref () , authority . key () . as_ref () , mint . key () . as_ref ()] , bump , token :: mint = mint , token :: authority = vault)]
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
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let vault = Empty {
            account: dot::program::CpiVault::load(&mut ctx.accounts.vault, &programs_map),
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
            authority.clone(),
            vault.clone(),
            vault_token_account.clone(),
            mint.clone(),
        );

        dot::program::CpiVault::store(vault.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (amount : u64)]
    pub struct PdaSignedTransferFromVault<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub vault: Box<Account<'info, dot::program::CpiVault>>,
        #[account(mut)]
        pub user_token_account: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub vault_token_account: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub mint: Box<Account<'info, Mint>>,
        pub token_program: Program<'info, Token>,
    }

    pub fn pda_signed_transfer_from_vault(
        ctx: Context<PdaSignedTransferFromVault>,
        amount: u64,
    ) -> Result<()> {
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

        let vault = dot::program::CpiVault::load(&mut ctx.accounts.vault, &programs_map);
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

        pda_signed_transfer_from_vault_handler(
            authority.clone(),
            vault.clone(),
            user_token_account.clone(),
            vault_token_account.clone(),
            mint.clone(),
            amount,
        );

        dot::program::CpiVault::store(vault);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (amount : u64)]
    pub struct ValidatedBurn<'info> {
        #[account(mut)]
        pub owner: Signer<'info>,
        #[account(mut)]
        pub source_account: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub mint: Box<Account<'info, Mint>>,
        pub token_program: Program<'info, Token>,
    }

    pub fn validated_burn(ctx: Context<ValidatedBurn>, amount: u64) -> Result<()> {
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

        let source_account = SeahorseAccount {
            account: &ctx.accounts.source_account,
            programs: &programs_map,
        };

        let mint = SeahorseAccount {
            account: &ctx.accounts.mint,
            programs: &programs_map,
        };

        validated_burn_handler(owner.clone(), source_account.clone(), mint.clone(), amount);

        return Ok(());
    }
}
