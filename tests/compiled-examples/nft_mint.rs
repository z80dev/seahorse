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
pub struct NftConfig {
    pub authority: Pubkey,
    pub nft_id: u64,
    pub mint: Pubkey,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub is_minted: bool,
    pub bump: u8,
}

impl<'info, 'entrypoint> NftConfig {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedNftConfig<'info, 'entrypoint>> {
        let authority = account.authority.clone();
        let nft_id = account.nft_id;
        let mint = account.mint.clone();
        let name = account.name.clone();
        let symbol = account.symbol.clone();
        let uri = account.uri.clone();
        let is_minted = account.is_minted.clone();
        let bump = account.bump;

        Mutable::new(LoadedNftConfig {
            __account__: account,
            __programs__: programs_map,
            authority,
            nft_id,
            mint,
            name,
            symbol,
            uri,
            is_minted,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedNftConfig>) {
        let mut loaded = loaded.borrow_mut();
        let authority = loaded.authority.clone();

        loaded.__account__.authority = authority;

        let nft_id = loaded.nft_id;

        loaded.__account__.nft_id = nft_id;

        let mint = loaded.mint.clone();

        loaded.__account__.mint = mint;

        let name = loaded.name.clone();

        loaded.__account__.name = name;

        let symbol = loaded.symbol.clone();

        loaded.__account__.symbol = symbol;

        let uri = loaded.uri.clone();

        loaded.__account__.uri = uri;

        let is_minted = loaded.is_minted.clone();

        loaded.__account__.is_minted = is_minted;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedNftConfig<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, NftConfig>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub authority: Pubkey,
    pub nft_id: u64,
    pub mint: Pubkey,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub is_minted: bool,
    pub bump: u8,
}

pub fn create_nft_config_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut nft_config: Empty<Mutable<LoadedNftConfig<'info, '_>>>,
    mut nft_id: u64,
    mut name: String,
    mut symbol: String,
    mut uri: String,
) -> () {
    "\n    Create a new NFT configuration.\n\n    In a full implementation with Metaplex support, this would also:\n    - Create the token mint (0 decimals)\n    - Create the metadata account via CPI\n    - Create the master edition account via CPI\n\n    Since Seahorse doesn't have native Metaplex support, this creates\n    a configuration account that tracks the NFT metadata.\n    " . to_string () ;

    if !((name.chars().count() as u64) <= 32) {
        panic!("Name too long (max 32 characters)");
    }

    if !((symbol.chars().count() as u64) <= 10) {
        panic!("Symbol too long (max 10 characters)");
    }

    if !((uri.chars().count() as u64) <= 200) {
        panic!("URI too long (max 200 characters)");
    }

    let mut bump = nft_config.bump.unwrap();
    let mut nft_config = nft_config.account.clone();

    solana_program::msg!("{} {}", "NFT config created with id:".to_string(), nft_id);

    assign!(nft_config.borrow_mut().authority, authority.key());

    assign!(nft_config.borrow_mut().nft_id, nft_id);

    assign!(nft_config.borrow_mut().mint, authority.key());

    assign!(nft_config.borrow_mut().name, name);

    assign!(nft_config.borrow_mut().symbol, symbol);

    assign!(nft_config.borrow_mut().uri, uri);

    assign!(nft_config.borrow_mut().is_minted, false);

    assign!(nft_config.borrow_mut().bump, bump);
}

pub fn get_nft_info_handler<'info>(mut nft_config: Mutable<LoadedNftConfig<'info, '_>>) -> () {
    "\n    Display NFT configuration information.\n\n    This is a read-only instruction that logs the NFT metadata.\n    In practice, clients would read the account data directly.\n    " . to_string () ;

    solana_program::msg!("{}", "NFT Config Info:".to_string());

    solana_program::msg!(
        "{} {:?}",
        "  Authority:".to_string(),
        nft_config.borrow().authority
    );

    solana_program::msg!("{} {:?}", "  Mint:".to_string(), nft_config.borrow().mint);

    solana_program::msg!("{} {}", "  Name:".to_string(), nft_config.borrow().name);

    solana_program::msg!("{} {}", "  Symbol:".to_string(), nft_config.borrow().symbol);

    solana_program::msg!("{} {}", "  URI:".to_string(), nft_config.borrow().uri);

    solana_program::msg!(
        "{} {}",
        "  Is Minted:".to_string(),
        nft_config.borrow().is_minted
    );
}

pub fn mark_as_minted_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut mint: SeahorseAccount<'info, '_, Mint>,
    mut nft_config: Mutable<LoadedNftConfig<'info, '_>>,
) -> () {
    "\n    Mark the NFT as minted and record the mint address.\n\n    In a full Metaplex implementation, this step would:\n    - Mint 1 token to the owner's token account\n    - Create the metadata account\n    - Create the master edition (supply = 0 for unique 1/1 NFT)\n\n    This function simulates that step by:\n    - Recording the mint address in the config\n    - Setting the is_minted flag\n\n    The actual token mint should be created externally (since Seahorse\n    can only create mints via Empty[TokenMint].init, not Metaplex NFT mints).\n    " . to_string () ;

    if !(nft_config.borrow().authority == authority.key()) {
        panic!("Unauthorized");
    }

    if !(!nft_config.borrow().is_minted) {
        panic!("NFT already minted");
    }

    assign!(nft_config.borrow_mut().mint, mint.key());

    assign!(nft_config.borrow_mut().is_minted, true);

    solana_program::msg!(
        "{} {:?}",
        "NFT marked as minted with mint:".to_string(),
        mint.key()
    );
}

pub fn update_nft_config_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut nft_config: Mutable<LoadedNftConfig<'info, '_>>,
    mut name: String,
    mut symbol: String,
    mut uri: String,
) -> () {
    "\n    Update NFT configuration before minting.\n\n    Once an NFT is minted, its on-chain metadata typically becomes immutable\n    (depending on the is_mutable flag set during creation).\n    " . to_string () ;

    if !(nft_config.borrow().authority == authority.key()) {
        panic!("Unauthorized");
    }

    if !(!nft_config.borrow().is_minted) {
        panic!("NFT already minted");
    }

    if (name.chars().count() as u64) > 0 {
        if !((name.chars().count() as u64) <= 32) {
            panic!("Name too long (max 32 characters)");
        }

        assign!(nft_config.borrow_mut().name, name);
    }

    if (symbol.chars().count() as u64) > 0 {
        if !((symbol.chars().count() as u64) <= 10) {
            panic!("Symbol too long (max 10 characters)");
        }

        assign!(nft_config.borrow_mut().symbol, symbol);
    }

    if (uri.chars().count() as u64) > 0 {
        if !((uri.chars().count() as u64) <= 200) {
            panic!("URI too long (max 200 characters)");
        }

        assign!(nft_config.borrow_mut().uri, uri);
    }

    solana_program::msg!("{}", "NFT config updated".to_string());
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

declare_id!("NFTm1nt111111111111111111111111111111111111");

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
mod nft_mint {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    # [instruction (nft_id : u64 , name : String , symbol : String , uri : String)]
    pub struct CreateNftConfig<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: NftConfig > () + 8 + (300 as usize) , payer = authority , seeds = ["nft_config" . as_bytes () . as_ref () , nft_id . to_le_bytes () . as_ref ()] , bump)]
        pub nft_config: Box<Account<'info, dot::program::NftConfig>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn create_nft_config(
        ctx: Context<CreateNftConfig>,
        nft_id: u64,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
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

        let nft_config = Empty {
            account: dot::program::NftConfig::load(&mut ctx.accounts.nft_config, &programs_map),
            bump: Some(ctx.bumps.nft_config),
        };

        create_nft_config_handler(
            authority.clone(),
            nft_config.clone(),
            nft_id,
            name,
            symbol,
            uri,
        );

        dot::program::NftConfig::store(nft_config.account);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct GetNftInfo<'info> {
        #[account(mut)]
        pub nft_config: Box<Account<'info, dot::program::NftConfig>>,
    }

    pub fn get_nft_info(ctx: Context<GetNftInfo>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let nft_config = dot::program::NftConfig::load(&mut ctx.accounts.nft_config, &programs_map);

        get_nft_info_handler(nft_config.clone());

        dot::program::NftConfig::store(nft_config);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct MarkAsMinted<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub nft_config: Box<Account<'info, dot::program::NftConfig>>,
    }

    pub fn mark_as_minted(ctx: Context<MarkAsMinted>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let mint = SeahorseAccount {
            account: &ctx.accounts.mint,
            programs: &programs_map,
        };

        let nft_config = dot::program::NftConfig::load(&mut ctx.accounts.nft_config, &programs_map);

        mark_as_minted_handler(authority.clone(), mint.clone(), nft_config.clone());

        dot::program::NftConfig::store(nft_config);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (name : String , symbol : String , uri : String)]
    pub struct UpdateNftConfig<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub nft_config: Box<Account<'info, dot::program::NftConfig>>,
    }

    pub fn update_nft_config(
        ctx: Context<UpdateNftConfig>,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let nft_config = dot::program::NftConfig::load(&mut ctx.accounts.nft_config, &programs_map);

        update_nft_config_handler(authority.clone(), nft_config.clone(), name, symbol, uri);

        dot::program::NftConfig::store(nft_config);

        return Ok(());
    }
}

