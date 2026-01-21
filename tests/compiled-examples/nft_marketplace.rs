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
pub struct Listing {
    pub seller: Pubkey,
    pub listing_id: u64,
    pub nft_mint: Pubkey,
    pub payment_mint: Pubkey,
    pub price: u64,
    pub is_active: bool,
    pub bump: u8,
}

impl<'info, 'entrypoint> Listing {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedListing<'info, 'entrypoint>> {
        let seller = account.seller.clone();
        let listing_id = account.listing_id;
        let nft_mint = account.nft_mint.clone();
        let payment_mint = account.payment_mint.clone();
        let price = account.price;
        let is_active = account.is_active.clone();
        let bump = account.bump;

        Mutable::new(LoadedListing {
            __account__: account,
            __programs__: programs_map,
            seller,
            listing_id,
            nft_mint,
            payment_mint,
            price,
            is_active,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedListing>) {
        let mut loaded = loaded.borrow_mut();
        let seller = loaded.seller.clone();

        loaded.__account__.seller = seller;

        let listing_id = loaded.listing_id;

        loaded.__account__.listing_id = listing_id;

        let nft_mint = loaded.nft_mint.clone();

        loaded.__account__.nft_mint = nft_mint;

        let payment_mint = loaded.payment_mint.clone();

        loaded.__account__.payment_mint = payment_mint;

        let price = loaded.price;

        loaded.__account__.price = price;

        let is_active = loaded.is_active.clone();

        loaded.__account__.is_active = is_active;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedListing<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, Listing>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub seller: Pubkey,
    pub listing_id: u64,
    pub nft_mint: Pubkey,
    pub payment_mint: Pubkey,
    pub price: u64,
    pub is_active: bool,
    pub bump: u8,
}

pub fn buy_nft_handler<'info>(
    mut buyer: SeahorseSigner<'info, '_>,
    mut seller: UncheckedAccount<'info>,
    mut nft_mint: SeahorseAccount<'info, '_, Mint>,
    mut payment_mint: SeahorseAccount<'info, '_, Mint>,
    mut listing: Mutable<LoadedListing<'info, '_>>,
    mut nft_escrow: SeahorseAccount<'info, '_, TokenAccount>,
    mut buyer_nft_token: SeahorseAccount<'info, '_, TokenAccount>,
    mut buyer_payment_token: SeahorseAccount<'info, '_, TokenAccount>,
    mut seller_payment_token: SeahorseAccount<'info, '_, TokenAccount>,
) -> () {
    "\n    Buy a listed NFT.\n\n    Payment transferred to seller, NFT transferred to buyer.\n\n    Args:\n        buyer: The person buying the NFT\n        seller: The seller's account (for validation)\n        nft_mint: The NFT mint\n        payment_mint: Token mint for payments\n        listing: The listing being purchased\n        nft_escrow: Token account holding escrowed NFT\n        buyer_nft_token: Buyer's token account to receive NFT\n        buyer_payment_token: Buyer's payment token account\n        seller_payment_token: Seller's payment token account to receive payment\n    " . to_string () ;

    if !listing.borrow().is_active {
        panic!("Listing is not active");
    }

    if !(listing.borrow().seller == seller.key()) {
        panic!("Invalid seller");
    }

    let mut listing_id = listing.borrow().listing_id;
    let mut listing_bump = listing.borrow().bump;
    let mut price = listing.borrow().price;

    token::transfer(
        CpiContext::new(
            buyer_payment_token.programs.get("token_program"),
            token::Transfer {
                from: buyer_payment_token.to_account_info(),
                authority: buyer.clone().to_account_info(),
                to: seller_payment_token.clone().to_account_info(),
            },
        ),
        price.clone(),
    )
    .unwrap();

    token::transfer(
        CpiContext::new_with_signer(
            nft_escrow.programs.get("token_program"),
            token::Transfer {
                from: nft_escrow.to_account_info(),
                authority: listing.borrow().__account__.to_account_info(),
                to: buyer_nft_token.clone().to_account_info(),
            },
            &[Mutable::new(vec![
                "listing".to_string().as_bytes().as_ref(),
                listing_id.to_le_bytes().as_ref(),
                listing_bump.to_le_bytes().as_ref(),
            ])
            .borrow()
            .as_slice()],
        ),
        <u64 as TryFrom<_>>::try_from(1).unwrap(),
    )
    .unwrap();

    assign!(listing.borrow_mut().is_active, false);
}

pub fn delist_nft_handler<'info>(
    mut seller: SeahorseSigner<'info, '_>,
    mut nft_mint: SeahorseAccount<'info, '_, Mint>,
    mut listing: Mutable<LoadedListing<'info, '_>>,
    mut nft_escrow: SeahorseAccount<'info, '_, TokenAccount>,
    mut seller_nft_token: SeahorseAccount<'info, '_, TokenAccount>,
) -> () {
    "\n    Delist an NFT (cancel listing).\n\n    Only seller can delist. NFT returned to seller.\n\n    Args:\n        seller: The seller (must match listing seller)\n        nft_mint: The NFT mint\n        listing: The listing to cancel\n        nft_escrow: Token account holding escrowed NFT\n        seller_nft_token: Seller's token account to receive NFT back\n    " . to_string () ;

    if !listing.borrow().is_active {
        panic!("Listing is not active");
    }

    if !(listing.borrow().seller == seller.key()) {
        panic!("Unauthorized");
    }

    let mut listing_id = listing.borrow().listing_id;
    let mut listing_bump = listing.borrow().bump;

    token::transfer(
        CpiContext::new_with_signer(
            nft_escrow.programs.get("token_program"),
            token::Transfer {
                from: nft_escrow.to_account_info(),
                authority: listing.borrow().__account__.to_account_info(),
                to: seller_nft_token.clone().to_account_info(),
            },
            &[Mutable::new(vec![
                "listing".to_string().as_bytes().as_ref(),
                listing_id.to_le_bytes().as_ref(),
                listing_bump.to_le_bytes().as_ref(),
            ])
            .borrow()
            .as_slice()],
        ),
        <u64 as TryFrom<_>>::try_from(1).unwrap(),
    )
    .unwrap();

    assign!(listing.borrow_mut().is_active, false);
}

pub fn list_nft_handler<'info>(
    mut seller: SeahorseSigner<'info, '_>,
    mut nft_mint: SeahorseAccount<'info, '_, Mint>,
    mut payment_mint: SeahorseAccount<'info, '_, Mint>,
    mut listing: Empty<Mutable<LoadedListing<'info, '_>>>,
    mut nft_escrow: Empty<SeahorseAccount<'info, '_, TokenAccount>>,
    mut seller_nft_token: SeahorseAccount<'info, '_, TokenAccount>,
    mut listing_id: u64,
    mut price: u64,
) -> () {
    "\n    List an NFT for sale on the marketplace.\n\n    Args:\n        seller: The NFT owner listing the NFT\n        nft_mint: The NFT mint (should have supply=1, decimals=0)\n        payment_mint: Token mint for payments\n        listing: Empty listing account to initialize\n        nft_escrow: Empty token account for NFT escrow\n        seller_nft_token: Seller's token account holding the NFT\n        listing_id: Unique identifier for this listing\n        price: Price in payment tokens\n    " . to_string () ;

    if !(price > 0) {
        panic!("Price must be greater than zero");
    }

    if !(seller_nft_token.amount >= 1) {
        panic!("Seller does not own the NFT");
    }

    let mut listing_bump = listing.bump.unwrap();
    let mut listing = listing.account.clone();
    let mut nft_escrow = nft_escrow.account.clone();

    assign!(listing.borrow_mut().seller, seller.key());

    assign!(listing.borrow_mut().listing_id, listing_id);

    assign!(listing.borrow_mut().nft_mint, nft_mint.key());

    assign!(listing.borrow_mut().payment_mint, payment_mint.key());

    assign!(listing.borrow_mut().price, price);

    assign!(listing.borrow_mut().is_active, true);

    assign!(listing.borrow_mut().bump, listing_bump);

    token::transfer(
        CpiContext::new(
            seller_nft_token.programs.get("token_program"),
            token::Transfer {
                from: seller_nft_token.to_account_info(),
                authority: seller.clone().to_account_info(),
                to: nft_escrow.clone().to_account_info(),
            },
        ),
        <u64 as TryFrom<_>>::try_from(1).unwrap(),
    )
    .unwrap();
}

pub fn update_price_handler<'info>(
    mut seller: SeahorseSigner<'info, '_>,
    mut listing: Mutable<LoadedListing<'info, '_>>,
    mut new_price: u64,
) -> () {
    "\n    Update listing price.\n\n    Only seller can update price while listing is active.\n\n    Args:\n        seller: The seller (must match listing seller)\n        listing: The listing to update\n        new_price: New price in payment tokens\n    " . to_string () ;

    if !listing.borrow().is_active {
        panic!("Listing is not active");
    }

    if !(listing.borrow().seller == seller.key()) {
        panic!("Unauthorized");
    }

    if !(new_price > 0) {
        panic!("Price must be greater than zero");
    }

    assign!(listing.borrow_mut().price, new_price);
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

declare_id!("Coa8ZSxePf7ZCsDNHE9SRHyZJRWWnZgmanuKVf4ZBM7Q");

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
mod nft_marketplace {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    pub struct BuyNft<'info> {
        #[account(mut)]
        pub buyer: Signer<'info>,
        #[account(mut)]
        #[doc = "CHECK: This account is unchecked."]
        pub seller: UncheckedAccount<'info>,
        #[account(mut)]
        pub nft_mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub payment_mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub listing: Box<Account<'info, dot::program::Listing>>,
        #[account(mut)]
        pub nft_escrow: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub buyer_nft_token: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub buyer_payment_token: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub seller_payment_token: Box<Account<'info, TokenAccount>>,
        pub token_program: Program<'info, Token>,
    }

    pub fn buy_nft(ctx: Context<BuyNft>) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "token_program",
            ctx.accounts.token_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let buyer = SeahorseSigner {
            account: &ctx.accounts.buyer,
            programs: &programs_map,
        };

        let seller = &ctx.accounts.seller.clone();
        let nft_mint = SeahorseAccount {
            account: &ctx.accounts.nft_mint,
            programs: &programs_map,
        };

        let payment_mint = SeahorseAccount {
            account: &ctx.accounts.payment_mint,
            programs: &programs_map,
        };

        let listing = dot::program::Listing::load(&mut ctx.accounts.listing, &programs_map);
        let nft_escrow = SeahorseAccount {
            account: &ctx.accounts.nft_escrow,
            programs: &programs_map,
        };

        let buyer_nft_token = SeahorseAccount {
            account: &ctx.accounts.buyer_nft_token,
            programs: &programs_map,
        };

        let buyer_payment_token = SeahorseAccount {
            account: &ctx.accounts.buyer_payment_token,
            programs: &programs_map,
        };

        let seller_payment_token = SeahorseAccount {
            account: &ctx.accounts.seller_payment_token,
            programs: &programs_map,
        };

        buy_nft_handler(
            buyer.clone(),
            seller.clone(),
            nft_mint.clone(),
            payment_mint.clone(),
            listing.clone(),
            nft_escrow.clone(),
            buyer_nft_token.clone(),
            buyer_payment_token.clone(),
            seller_payment_token.clone(),
        );

        dot::program::Listing::store(listing);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct DelistNft<'info> {
        #[account(mut)]
        pub seller: Signer<'info>,
        #[account(mut)]
        pub nft_mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub listing: Box<Account<'info, dot::program::Listing>>,
        #[account(mut)]
        pub nft_escrow: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub seller_nft_token: Box<Account<'info, TokenAccount>>,
        pub token_program: Program<'info, Token>,
    }

    pub fn delist_nft(ctx: Context<DelistNft>) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "token_program",
            ctx.accounts.token_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let seller = SeahorseSigner {
            account: &ctx.accounts.seller,
            programs: &programs_map,
        };

        let nft_mint = SeahorseAccount {
            account: &ctx.accounts.nft_mint,
            programs: &programs_map,
        };

        let listing = dot::program::Listing::load(&mut ctx.accounts.listing, &programs_map);
        let nft_escrow = SeahorseAccount {
            account: &ctx.accounts.nft_escrow,
            programs: &programs_map,
        };

        let seller_nft_token = SeahorseAccount {
            account: &ctx.accounts.seller_nft_token,
            programs: &programs_map,
        };

        delist_nft_handler(
            seller.clone(),
            nft_mint.clone(),
            listing.clone(),
            nft_escrow.clone(),
            seller_nft_token.clone(),
        );

        dot::program::Listing::store(listing);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (listing_id : u64 , price : u64)]
    pub struct ListNft<'info> {
        #[account(mut)]
        pub seller: Signer<'info>,
        #[account(mut)]
        pub nft_mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub payment_mint: Box<Account<'info, Mint>>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: Listing > () + 8 , payer = seller , seeds = ["listing" . as_bytes () . as_ref () , listing_id . to_le_bytes () . as_ref ()] , bump)]
        pub listing: Box<Account<'info, dot::program::Listing>>,
        # [account (init , payer = seller , seeds = ["nft_escrow" . as_bytes () . as_ref () , listing_id . to_le_bytes () . as_ref ()] , bump , token :: mint = nft_mint , token :: authority = listing)]
        pub nft_escrow: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub seller_nft_token: Box<Account<'info, TokenAccount>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
        pub token_program: Program<'info, Token>,
    }

    pub fn list_nft(ctx: Context<ListNft>, listing_id: u64, price: u64) -> Result<()> {
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
        let seller = SeahorseSigner {
            account: &ctx.accounts.seller,
            programs: &programs_map,
        };

        let nft_mint = SeahorseAccount {
            account: &ctx.accounts.nft_mint,
            programs: &programs_map,
        };

        let payment_mint = SeahorseAccount {
            account: &ctx.accounts.payment_mint,
            programs: &programs_map,
        };

        let listing = Empty {
            account: dot::program::Listing::load(&mut ctx.accounts.listing, &programs_map),
            bump: Some(ctx.bumps.listing),
        };

        let nft_escrow = Empty {
            account: SeahorseAccount {
                account: &ctx.accounts.nft_escrow,
                programs: &programs_map,
            },
            bump: Some(ctx.bumps.nft_escrow),
        };

        let seller_nft_token = SeahorseAccount {
            account: &ctx.accounts.seller_nft_token,
            programs: &programs_map,
        };

        list_nft_handler(
            seller.clone(),
            nft_mint.clone(),
            payment_mint.clone(),
            listing.clone(),
            nft_escrow.clone(),
            seller_nft_token.clone(),
            listing_id,
            price,
        );

        dot::program::Listing::store(listing.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (new_price : u64)]
    pub struct UpdatePrice<'info> {
        #[account(mut)]
        pub seller: Signer<'info>,
        #[account(mut)]
        pub listing: Box<Account<'info, dot::program::Listing>>,
    }

    pub fn update_price(ctx: Context<UpdatePrice>, new_price: u64) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let seller = SeahorseSigner {
            account: &ctx.accounts.seller,
            programs: &programs_map,
        };

        let listing = dot::program::Listing::load(&mut ctx.accounts.listing, &programs_map);

        update_price_handler(seller.clone(), listing.clone(), new_price);

        dot::program::Listing::store(listing);

        return Ok(());
    }
}

