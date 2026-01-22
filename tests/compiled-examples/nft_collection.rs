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
pub struct Collection {
    pub authority: Pubkey,
    pub collection_id: u64,
    pub collection_mint: Pubkey,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub total_nfts: u64,
    pub verified_nfts: u64,
    pub is_finalized: bool,
    pub bump: u8,
}

impl<'info, 'entrypoint> Collection {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedCollection<'info, 'entrypoint>> {
        let authority = account.authority.clone();
        let collection_id = account.collection_id;
        let collection_mint = account.collection_mint.clone();
        let name = account.name.clone();
        let symbol = account.symbol.clone();
        let uri = account.uri.clone();
        let total_nfts = account.total_nfts;
        let verified_nfts = account.verified_nfts;
        let is_finalized = account.is_finalized.clone();
        let bump = account.bump;

        Mutable::new(LoadedCollection {
            __account__: account,
            __programs__: programs_map,
            authority,
            collection_id,
            collection_mint,
            name,
            symbol,
            uri,
            total_nfts,
            verified_nfts,
            is_finalized,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedCollection>) {
        let mut loaded = loaded.borrow_mut();
        let authority = loaded.authority.clone();

        loaded.__account__.authority = authority;

        let collection_id = loaded.collection_id;

        loaded.__account__.collection_id = collection_id;

        let collection_mint = loaded.collection_mint.clone();

        loaded.__account__.collection_mint = collection_mint;

        let name = loaded.name.clone();

        loaded.__account__.name = name;

        let symbol = loaded.symbol.clone();

        loaded.__account__.symbol = symbol;

        let uri = loaded.uri.clone();

        loaded.__account__.uri = uri;

        let total_nfts = loaded.total_nfts;

        loaded.__account__.total_nfts = total_nfts;

        let verified_nfts = loaded.verified_nfts;

        loaded.__account__.verified_nfts = verified_nfts;

        let is_finalized = loaded.is_finalized.clone();

        loaded.__account__.is_finalized = is_finalized;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedCollection<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, Collection>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub authority: Pubkey,
    pub collection_id: u64,
    pub collection_mint: Pubkey,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub total_nfts: u64,
    pub verified_nfts: u64,
    pub is_finalized: bool,
    pub bump: u8,
}

#[account]
#[derive(Debug)]
pub struct CollectionMember {
    pub collection: Pubkey,
    pub nft_mint: Pubkey,
    pub is_verified: bool,
    pub bump: u8,
}

impl<'info, 'entrypoint> CollectionMember {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedCollectionMember<'info, 'entrypoint>> {
        let collection = account.collection.clone();
        let nft_mint = account.nft_mint.clone();
        let is_verified = account.is_verified.clone();
        let bump = account.bump;

        Mutable::new(LoadedCollectionMember {
            __account__: account,
            __programs__: programs_map,
            collection,
            nft_mint,
            is_verified,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedCollectionMember>) {
        let mut loaded = loaded.borrow_mut();
        let collection = loaded.collection.clone();

        loaded.__account__.collection = collection;

        let nft_mint = loaded.nft_mint.clone();

        loaded.__account__.nft_mint = nft_mint;

        let is_verified = loaded.is_verified.clone();

        loaded.__account__.is_verified = is_verified;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedCollectionMember<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, CollectionMember>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub collection: Pubkey,
    pub nft_mint: Pubkey,
    pub is_verified: bool,
    pub bump: u8,
}

pub fn add_nft_to_collection_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut collection: Mutable<LoadedCollection<'info, '_>>,
    mut nft_mint: SeahorseAccount<'info, '_, Mint>,
    mut member: Empty<Mutable<LoadedCollectionMember<'info, '_>>>,
) -> () {
    "\n    Register an NFT as a member of the collection.\n\n    In Metaplex, this would set the collection field on the NFT's metadata account.\n    The NFT starts as unverified - call verify_nft to verify.\n    " . to_string () ;

    if !(collection.borrow().authority == authority.key()) {
        panic!("Unauthorized");
    }

    let mut bump = member.bump.unwrap();
    let mut member = member.account.clone();

    assign!(
        member.borrow_mut().collection,
        collection.borrow().__account__.key()
    );

    assign!(member.borrow_mut().nft_mint, nft_mint.key());

    assign!(member.borrow_mut().is_verified, false);

    assign!(member.borrow_mut().bump, bump);

    assign!(
        collection.borrow_mut().total_nfts,
        collection.borrow().total_nfts + 1
    );

    solana_program::msg!(
        "{} {:?}",
        "NFT added to collection:".to_string(),
        nft_mint.key()
    );
}

pub fn create_collection_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut collection: Empty<Mutable<LoadedCollection<'info, '_>>>,
    mut collection_id: u64,
    mut name: String,
    mut symbol: String,
    mut uri: String,
) -> () {
    "\n    Create a new collection configuration.\n\n    In a full Metaplex implementation, this would also:\n    - Create the collection NFT mint (0 decimals, supply 1)\n    - Create metadata account for the collection NFT\n    - Create master edition for the collection NFT\n\n    Since Seahorse doesn't have native Metaplex support, this creates\n    a configuration account that tracks the collection metadata.\n    " . to_string () ;

    if !((name.chars().count() as u64) <= 32) {
        panic!("Name too long (max 32 characters)");
    }

    if !((symbol.chars().count() as u64) <= 10) {
        panic!("Symbol too long (max 10 characters)");
    }

    if !((uri.chars().count() as u64) <= 200) {
        panic!("URI too long (max 200 characters)");
    }

    let mut bump = collection.bump.unwrap();
    let mut collection = collection.account.clone();

    solana_program::msg!(
        "{} {}",
        "Collection created with id:".to_string(),
        collection_id
    );

    assign!(collection.borrow_mut().authority, authority.key());

    assign!(collection.borrow_mut().collection_id, collection_id);

    assign!(collection.borrow_mut().collection_mint, authority.key());

    assign!(collection.borrow_mut().name, name.clone());

    assign!(collection.borrow_mut().symbol, symbol.clone());

    assign!(collection.borrow_mut().uri, uri.clone());

    assign!(
        collection.borrow_mut().total_nfts,
        <u64 as TryFrom<_>>::try_from(0).unwrap()
    );

    assign!(
        collection.borrow_mut().verified_nfts,
        <u64 as TryFrom<_>>::try_from(0).unwrap()
    );

    assign!(collection.borrow_mut().is_finalized, false);

    assign!(collection.borrow_mut().bump, bump);
}

pub fn finalize_collection_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut collection: Mutable<LoadedCollection<'info, '_>>,
) -> () {
    "\n    Finalize the collection, preventing further metadata changes.\n\n    This is similar to making the collection metadata immutable in Metaplex.\n    NFTs can still be added and verified after finalization.\n    " . to_string () ;

    if !(collection.borrow().authority == authority.key()) {
        panic!("Unauthorized");
    }

    if !(!collection.borrow().is_finalized) {
        panic!("Collection already finalized");
    }

    assign!(collection.borrow_mut().is_finalized, true);

    solana_program::msg!("{}", "Collection finalized".to_string());
}

pub fn get_collection_info_handler<'info>(
    mut collection: Mutable<LoadedCollection<'info, '_>>,
) -> () {
    "\n    Display collection information.\n\n    This is a read-only instruction that logs the collection metadata.\n    " . to_string () ;

    solana_program::msg!("{}", "Collection Info:".to_string());

    solana_program::msg!(
        "{} {:?}",
        "  Authority:".to_string(),
        collection.borrow().authority
    );

    solana_program::msg!(
        "{} {:?}",
        "  Collection Mint:".to_string(),
        collection.borrow().collection_mint
    );

    solana_program::msg!("{} {}", "  Name:".to_string(), collection.borrow().name);

    solana_program::msg!("{} {}", "  Symbol:".to_string(), collection.borrow().symbol);

    solana_program::msg!("{} {}", "  URI:".to_string(), collection.borrow().uri);

    solana_program::msg!(
        "{} {}",
        "  Total NFTs:".to_string(),
        collection.borrow().total_nfts
    );

    solana_program::msg!(
        "{} {}",
        "  Verified NFTs:".to_string(),
        collection.borrow().verified_nfts
    );

    solana_program::msg!(
        "{} {}",
        "  Is Finalized:".to_string(),
        collection.borrow().is_finalized
    );
}

pub fn get_member_info_handler<'info>(
    mut member: Mutable<LoadedCollectionMember<'info, '_>>,
) -> () {
    "\n    Display member information.\n\n    This is a read-only instruction that logs the member status.\n    " . to_string () ;

    solana_program::msg!("{}", "Member Info:".to_string());

    solana_program::msg!(
        "{} {:?}",
        "  Collection:".to_string(),
        member.borrow().collection
    );

    solana_program::msg!(
        "{} {:?}",
        "  NFT Mint:".to_string(),
        member.borrow().nft_mint
    );

    solana_program::msg!(
        "{} {}",
        "  Is Verified:".to_string(),
        member.borrow().is_verified
    );
}

pub fn set_collection_mint_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut collection: Mutable<LoadedCollection<'info, '_>>,
    mut collection_mint: SeahorseAccount<'info, '_, Mint>,
) -> () {
    "\n    Set the collection mint address after creating the actual Metaplex collection NFT.\n\n    This allows linking the configuration to an externally-created collection NFT.\n    " . to_string () ;

    if !(collection.borrow().authority == authority.key()) {
        panic!("Unauthorized");
    }

    assign!(
        collection.borrow_mut().collection_mint,
        collection_mint.key()
    );

    solana_program::msg!(
        "{} {:?}",
        "Collection mint set to:".to_string(),
        collection_mint.key()
    );
}

pub fn unverify_nft_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut collection: Mutable<LoadedCollection<'info, '_>>,
    mut member: Mutable<LoadedCollectionMember<'info, '_>>,
) -> () {
    "\n    Remove verification status from an NFT.\n\n    In Metaplex, this is done via the UnverifyCollection instruction.\n    " . to_string () ;

    if !(collection.borrow().authority == authority.key()) {
        panic!("Unauthorized");
    }

    if !(member.borrow().collection == collection.borrow().__account__.key()) {
        panic!("NFT not in this collection");
    }

    if !member.borrow().is_verified {
        panic!("NFT not verified");
    }

    assign!(member.borrow_mut().is_verified, false);

    assign!(
        collection.borrow_mut().verified_nfts,
        collection.borrow().verified_nfts - 1
    );

    solana_program::msg!("{}", "NFT unverified from collection".to_string());
}

pub fn update_collection_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut collection: Mutable<LoadedCollection<'info, '_>>,
    mut name: String,
    mut symbol: String,
    mut uri: String,
) -> () {
    "\n    Update collection metadata before finalization.\n\n    Once a collection is finalized, its metadata cannot be changed.\n    " . to_string () ;

    if !(collection.borrow().authority == authority.key()) {
        panic!("Unauthorized");
    }

    if !(!collection.borrow().is_finalized) {
        panic!("Collection is finalized");
    }

    if (name.chars().count() as u64) > 0 {
        if !((name.chars().count() as u64) <= 32) {
            panic!("Name too long (max 32 characters)");
        }

        assign!(collection.borrow_mut().name, name.clone());
    }

    if (symbol.chars().count() as u64) > 0 {
        if !((symbol.chars().count() as u64) <= 10) {
            panic!("Symbol too long (max 10 characters)");
        }

        assign!(collection.borrow_mut().symbol, symbol.clone());
    }

    if (uri.chars().count() as u64) > 0 {
        if !((uri.chars().count() as u64) <= 200) {
            panic!("URI too long (max 200 characters)");
        }

        assign!(collection.borrow_mut().uri, uri.clone());
    }

    solana_program::msg!("{}", "Collection updated".to_string());
}

pub fn verify_nft_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut collection: Mutable<LoadedCollection<'info, '_>>,
    mut member: Mutable<LoadedCollectionMember<'info, '_>>,
) -> () {
    "\n    Verify an NFT's membership in the collection.\n\n    In Metaplex, this is done via the SetAndVerifyCollection or VerifyCollection\n    instruction, which requires the collection authority to sign.\n\n    Only the collection authority can verify NFTs.\n    " . to_string () ;

    if !(collection.borrow().authority == authority.key()) {
        panic!("Unauthorized");
    }

    if !(member.borrow().collection == collection.borrow().__account__.key()) {
        panic!("NFT not in this collection");
    }

    if !(!member.borrow().is_verified) {
        panic!("NFT already verified");
    }

    assign!(member.borrow_mut().is_verified, true);

    assign!(
        collection.borrow_mut().verified_nfts,
        collection.borrow().verified_nfts + 1
    );

    solana_program::msg!("{}", "NFT verified in collection".to_string());
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

declare_id!("NFTCoL1ect1on111111111111111111111111111111");

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
mod nft_collection {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    pub struct AddNftToCollection<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub collection: Box<Account<'info, dot::program::Collection>>,
        #[account(mut)]
        pub nft_mint: Box<Account<'info, Mint>>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: CollectionMember > () + 8 , payer = authority , seeds = ["member" . as_bytes () . as_ref () , collection . key () . as_ref () , nft_mint . key () . as_ref ()] , bump)]
        pub member: Box<Account<'info, dot::program::CollectionMember>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn add_nft_to_collection(ctx: Context<AddNftToCollection>) -> Result<()> {
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

        let collection =
            dot::program::Collection::load(&mut ctx.accounts.collection, &programs_map);

        let nft_mint = SeahorseAccount {
            account: &ctx.accounts.nft_mint,
            programs: &programs_map,
        };

        let member = Empty {
            account: dot::program::CollectionMember::load(&mut ctx.accounts.member, &programs_map),
            bump: Some(ctx.bumps.member),
        };

        add_nft_to_collection_handler(
            authority.clone(),
            collection.clone(),
            nft_mint.clone(),
            member.clone(),
        );

        dot::program::Collection::store(collection);

        dot::program::CollectionMember::store(member.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (collection_id : u64 , name : String , symbol : String , uri : String)]
    pub struct CreateCollection<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: Collection > () + 8 + (300 as usize) , payer = authority , seeds = ["collection" . as_bytes () . as_ref () , collection_id . to_le_bytes () . as_ref ()] , bump)]
        pub collection: Box<Account<'info, dot::program::Collection>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn create_collection(
        ctx: Context<CreateCollection>,
        collection_id: u64,
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

        let collection = Empty {
            account: dot::program::Collection::load(&mut ctx.accounts.collection, &programs_map),
            bump: Some(ctx.bumps.collection),
        };

        create_collection_handler(
            authority.clone(),
            collection.clone(),
            collection_id,
            name,
            symbol,
            uri,
        );

        dot::program::Collection::store(collection.account);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct FinalizeCollection<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub collection: Box<Account<'info, dot::program::Collection>>,
    }

    pub fn finalize_collection(ctx: Context<FinalizeCollection>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let collection =
            dot::program::Collection::load(&mut ctx.accounts.collection, &programs_map);

        finalize_collection_handler(authority.clone(), collection.clone());

        dot::program::Collection::store(collection);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct GetCollectionInfo<'info> {
        #[account(mut)]
        pub collection: Box<Account<'info, dot::program::Collection>>,
    }

    pub fn get_collection_info(ctx: Context<GetCollectionInfo>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let collection =
            dot::program::Collection::load(&mut ctx.accounts.collection, &programs_map);

        get_collection_info_handler(collection.clone());

        dot::program::Collection::store(collection);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct GetMemberInfo<'info> {
        #[account(mut)]
        pub member: Box<Account<'info, dot::program::CollectionMember>>,
    }

    pub fn get_member_info(ctx: Context<GetMemberInfo>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let member = dot::program::CollectionMember::load(&mut ctx.accounts.member, &programs_map);

        get_member_info_handler(member.clone());

        dot::program::CollectionMember::store(member);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct SetCollectionMint<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub collection: Box<Account<'info, dot::program::Collection>>,
        #[account(mut)]
        pub collection_mint: Box<Account<'info, Mint>>,
    }

    pub fn set_collection_mint(ctx: Context<SetCollectionMint>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let collection =
            dot::program::Collection::load(&mut ctx.accounts.collection, &programs_map);

        let collection_mint = SeahorseAccount {
            account: &ctx.accounts.collection_mint,
            programs: &programs_map,
        };

        set_collection_mint_handler(
            authority.clone(),
            collection.clone(),
            collection_mint.clone(),
        );

        dot::program::Collection::store(collection);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct UnverifyNft<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub collection: Box<Account<'info, dot::program::Collection>>,
        #[account(mut)]
        pub member: Box<Account<'info, dot::program::CollectionMember>>,
    }

    pub fn unverify_nft(ctx: Context<UnverifyNft>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let collection =
            dot::program::Collection::load(&mut ctx.accounts.collection, &programs_map);

        let member = dot::program::CollectionMember::load(&mut ctx.accounts.member, &programs_map);

        unverify_nft_handler(authority.clone(), collection.clone(), member.clone());

        dot::program::Collection::store(collection);

        dot::program::CollectionMember::store(member);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (name : String , symbol : String , uri : String)]
    pub struct UpdateCollection<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub collection: Box<Account<'info, dot::program::Collection>>,
    }

    pub fn update_collection(
        ctx: Context<UpdateCollection>,
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

        let collection =
            dot::program::Collection::load(&mut ctx.accounts.collection, &programs_map);

        update_collection_handler(authority.clone(), collection.clone(), name, symbol, uri);

        dot::program::Collection::store(collection);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct VerifyNft<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub collection: Box<Account<'info, dot::program::Collection>>,
        #[account(mut)]
        pub member: Box<Account<'info, dot::program::CollectionMember>>,
    }

    pub fn verify_nft(ctx: Context<VerifyNft>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let collection =
            dot::program::Collection::load(&mut ctx.accounts.collection, &programs_map);

        let member = dot::program::CollectionMember::load(&mut ctx.accounts.member, &programs_map);

        verify_nft_handler(authority.clone(), collection.clone(), member.clone());

        dot::program::Collection::store(collection);

        dot::program::CollectionMember::store(member);

        return Ok(());
    }
}

