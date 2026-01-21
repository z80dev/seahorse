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
pub struct Auction {
    pub seller: Pubkey,
    pub auction_id: u64,
    pub bid_mint: Pubkey,
    pub starting_price: u64,
    pub current_bid: u64,
    pub highest_bidder: Pubkey,
    pub start_slot: u64,
    pub end_slot: u64,
    pub is_ended: bool,
    pub is_claimed: bool,
    pub bump: u8,
}

impl<'info, 'entrypoint> Auction {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedAuction<'info, 'entrypoint>> {
        let seller = account.seller.clone();
        let auction_id = account.auction_id;
        let bid_mint = account.bid_mint.clone();
        let starting_price = account.starting_price;
        let current_bid = account.current_bid;
        let highest_bidder = account.highest_bidder.clone();
        let start_slot = account.start_slot;
        let end_slot = account.end_slot;
        let is_ended = account.is_ended.clone();
        let is_claimed = account.is_claimed.clone();
        let bump = account.bump;

        Mutable::new(LoadedAuction {
            __account__: account,
            __programs__: programs_map,
            seller,
            auction_id,
            bid_mint,
            starting_price,
            current_bid,
            highest_bidder,
            start_slot,
            end_slot,
            is_ended,
            is_claimed,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedAuction>) {
        let mut loaded = loaded.borrow_mut();
        let seller = loaded.seller.clone();

        loaded.__account__.seller = seller;

        let auction_id = loaded.auction_id;

        loaded.__account__.auction_id = auction_id;

        let bid_mint = loaded.bid_mint.clone();

        loaded.__account__.bid_mint = bid_mint;

        let starting_price = loaded.starting_price;

        loaded.__account__.starting_price = starting_price;

        let current_bid = loaded.current_bid;

        loaded.__account__.current_bid = current_bid;

        let highest_bidder = loaded.highest_bidder.clone();

        loaded.__account__.highest_bidder = highest_bidder;

        let start_slot = loaded.start_slot;

        loaded.__account__.start_slot = start_slot;

        let end_slot = loaded.end_slot;

        loaded.__account__.end_slot = end_slot;

        let is_ended = loaded.is_ended.clone();

        loaded.__account__.is_ended = is_ended;

        let is_claimed = loaded.is_claimed.clone();

        loaded.__account__.is_claimed = is_claimed;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedAuction<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, Auction>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub seller: Pubkey,
    pub auction_id: u64,
    pub bid_mint: Pubkey,
    pub starting_price: u64,
    pub current_bid: u64,
    pub highest_bidder: Pubkey,
    pub start_slot: u64,
    pub end_slot: u64,
    pub is_ended: bool,
    pub is_claimed: bool,
    pub bump: u8,
}

pub fn claim_prize_handler<'info>(
    mut winner: SeahorseSigner<'info, '_>,
    mut seller: UncheckedAccount<'info>,
    mut bid_mint: SeahorseAccount<'info, '_, Mint>,
    mut seller_token_account: SeahorseAccount<'info, '_, TokenAccount>,
    mut auction: Mutable<LoadedAuction<'info, '_>>,
    mut bid_escrow: SeahorseAccount<'info, '_, TokenAccount>,
) -> () {
    "\n    Claim auction prize.\n\n    Winner confirms claim, seller receives payment from escrow.\n    Can only be called after auction is ended.\n\n    Args:\n        winner: The auction winner (highest bidder)\n        seller: The seller to receive payment\n        bid_mint: Token mint for bids\n        seller_token_account: Seller's token account to receive payment\n        auction: The auction\n        bid_escrow: Token account holding bid escrow\n    " . to_string () ;

    if !auction.borrow().is_ended {
        panic!("Auction has not ended yet");
    }

    if !(!auction.borrow().is_claimed) {
        panic!("Prize has already been claimed");
    }

    if !(auction.borrow().current_bid > 0) {
        panic!("No bids were placed");
    }

    if !(auction.borrow().highest_bidder == winner.key()) {
        panic!("Not the winner");
    }

    if !(auction.borrow().seller == seller.key()) {
        panic!("Invalid seller");
    }

    let mut auction_bump = auction.borrow().bump;
    let mut auction_id = auction.borrow().auction_id;

    token::transfer(
        CpiContext::new_with_signer(
            bid_escrow.programs.get("token_program"),
            token::Transfer {
                from: bid_escrow.to_account_info(),
                authority: auction.borrow().__account__.to_account_info(),
                to: seller_token_account.clone().to_account_info(),
            },
            &[Mutable::new(vec![
                "auction".to_string().as_bytes().as_ref(),
                auction_id.to_le_bytes().as_ref(),
                auction_bump.to_le_bytes().as_ref(),
            ])
            .borrow()
            .as_slice()],
        ),
        auction.borrow().current_bid.clone(),
    )
    .unwrap();

    assign!(auction.borrow_mut().is_claimed, true);
}

pub fn create_auction_handler<'info>(
    mut seller: SeahorseSigner<'info, '_>,
    mut bid_mint: SeahorseAccount<'info, '_, Mint>,
    mut auction: Empty<Mutable<LoadedAuction<'info, '_>>>,
    mut bid_escrow: Empty<SeahorseAccount<'info, '_, TokenAccount>>,
    mut clock: Sysvar<'info, Clock>,
    mut auction_id: u64,
    mut starting_price: u64,
    mut duration_slots: u64,
) -> () {
    "\n    Create a new auction.\n\n    Args:\n        seller: Auction creator who receives payment\n        bid_mint: Token mint for bids\n        auction: Empty auction account to initialize\n        bid_escrow: Empty token account for bid escrow\n        clock: Clock sysvar for time\n        auction_id: Unique identifier for this auction\n        starting_price: Minimum bid in tokens\n        duration_slots: Number of slots auction runs for\n    " . to_string () ;

    if !(starting_price > 0) {
        panic!("Starting price must be greater than zero");
    }

    if !(duration_slots > 0) {
        panic!("Duration must be greater than zero");
    }

    let mut current_slot = clock.slot;
    let mut auction_bump = auction.bump.unwrap();
    let mut auction = auction.account.clone();

    bid_escrow.account.clone();

    assign!(auction.borrow_mut().seller, seller.key());

    assign!(auction.borrow_mut().auction_id, auction_id);

    assign!(auction.borrow_mut().bid_mint, bid_mint.key());

    assign!(auction.borrow_mut().starting_price, starting_price);

    assign!(auction.borrow_mut().current_bid, 0);

    assign!(auction.borrow_mut().highest_bidder, seller.key());

    assign!(auction.borrow_mut().start_slot, current_slot);

    assign!(auction.borrow_mut().end_slot, current_slot + duration_slots);

    assign!(auction.borrow_mut().is_ended, false);

    assign!(auction.borrow_mut().is_claimed, false);

    assign!(auction.borrow_mut().bump, auction_bump);
}

pub fn end_auction_handler<'info>(
    mut caller: SeahorseSigner<'info, '_>,
    mut auction: Mutable<LoadedAuction<'info, '_>>,
    mut clock: Sysvar<'info, Clock>,
) -> () {
    "\n    End the auction.\n\n    Can only be called after end_slot has passed.\n    Marks the auction as ended, enabling prize claim.\n\n    Args:\n        caller: Anyone can call this after time expires\n        auction: The auction to end\n        clock: Clock sysvar for time\n    " . to_string () ;

    if !(!auction.borrow().is_ended) {
        panic!("Auction has already been ended");
    }

    let mut current_slot = clock.slot;

    if !(current_slot >= auction.borrow().end_slot) {
        panic!("Auction is still active");
    }

    assign!(auction.borrow_mut().is_ended, true);
}

pub fn place_bid_handler<'info>(
    mut bidder: SeahorseSigner<'info, '_>,
    mut previous_bidder: UncheckedAccount<'info>,
    mut bid_mint: SeahorseAccount<'info, '_, Mint>,
    mut bidder_token_account: SeahorseAccount<'info, '_, TokenAccount>,
    mut previous_bidder_token_account: SeahorseAccount<'info, '_, TokenAccount>,
    mut auction: Mutable<LoadedAuction<'info, '_>>,
    mut bid_escrow: SeahorseAccount<'info, '_, TokenAccount>,
    mut clock: Sysvar<'info, Clock>,
    mut bid_amount: u64,
) -> () {
    "\n    Place a bid on an auction.\n\n    Bid amount must exceed current bid (or starting price if no bids).\n    Previous highest bidder is automatically refunded.\n\n    Args:\n        bidder: The person placing the bid\n        previous_bidder: Account of previous highest bidder (for refund)\n        bid_mint: Token mint for bids\n        bidder_token_account: Bidder's token account\n        previous_bidder_token_account: Previous bidder's token account for refund\n        auction: The auction being bid on\n        bid_escrow: Token account holding bid escrow\n        clock: Clock sysvar for time\n        bid_amount: Amount of tokens to bid\n    " . to_string () ;

    if !(!auction.borrow().is_ended) {
        panic!("Auction has ended");
    }

    let mut current_slot = clock.slot;

    if !(current_slot < auction.borrow().end_slot) {
        panic!("Auction time has expired");
    }

    let mut min_bid = 0;

    if auction.borrow().current_bid > 0 {
        min_bid = auction.borrow().current_bid;
    } else {
        min_bid = auction.borrow().starting_price;
    }

    if !(bid_amount > min_bid) {
        panic!("Bid amount is too low");
    }

    let mut auction_bump = auction.borrow().bump;
    let mut auction_id = auction.borrow().auction_id;

    if auction.borrow().current_bid > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                bid_escrow.programs.get("token_program"),
                token::Transfer {
                    from: bid_escrow.to_account_info(),
                    authority: auction.borrow().__account__.to_account_info(),
                    to: previous_bidder_token_account.clone().to_account_info(),
                },
                &[Mutable::new(vec![
                    "auction".to_string().as_bytes().as_ref(),
                    auction_id.to_le_bytes().as_ref(),
                    auction_bump.to_le_bytes().as_ref(),
                ])
                .borrow()
                .as_slice()],
            ),
            auction.borrow().current_bid.clone(),
        )
        .unwrap();
    }

    token::transfer(
        CpiContext::new(
            bidder_token_account.programs.get("token_program"),
            token::Transfer {
                from: bidder_token_account.to_account_info(),
                authority: bidder.clone().to_account_info(),
                to: bid_escrow.clone().to_account_info(),
            },
        ),
        bid_amount.clone(),
    )
    .unwrap();

    assign!(auction.borrow_mut().highest_bidder, bidder.key());

    assign!(auction.borrow_mut().current_bid, bid_amount);
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

declare_id!("AUCTnhLbEfvJxTt9NQekQfpjsVq4xDv5aX1VL6h9pLwD");

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
mod auction {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    pub struct ClaimPrize<'info> {
        #[account(mut)]
        pub winner: Signer<'info>,
        #[account(mut)]
        #[doc = "CHECK: This account is unchecked."]
        pub seller: UncheckedAccount<'info>,
        #[account(mut)]
        pub bid_mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub seller_token_account: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub auction: Box<Account<'info, dot::program::Auction>>,
        #[account(mut)]
        pub bid_escrow: Box<Account<'info, TokenAccount>>,
        pub token_program: Program<'info, Token>,
    }

    pub fn claim_prize(ctx: Context<ClaimPrize>) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "token_program",
            ctx.accounts.token_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let winner = SeahorseSigner {
            account: &ctx.accounts.winner,
            programs: &programs_map,
        };

        let seller = &ctx.accounts.seller.clone();
        let bid_mint = SeahorseAccount {
            account: &ctx.accounts.bid_mint,
            programs: &programs_map,
        };

        let seller_token_account = SeahorseAccount {
            account: &ctx.accounts.seller_token_account,
            programs: &programs_map,
        };

        let auction = dot::program::Auction::load(&mut ctx.accounts.auction, &programs_map);
        let bid_escrow = SeahorseAccount {
            account: &ctx.accounts.bid_escrow,
            programs: &programs_map,
        };

        claim_prize_handler(
            winner.clone(),
            seller.clone(),
            bid_mint.clone(),
            seller_token_account.clone(),
            auction.clone(),
            bid_escrow.clone(),
        );

        dot::program::Auction::store(auction);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (auction_id : u64 , starting_price : u64 , duration_slots : u64)]
    pub struct CreateAuction<'info> {
        #[account(mut)]
        pub seller: Signer<'info>,
        #[account(mut)]
        pub bid_mint: Box<Account<'info, Mint>>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: Auction > () + 8 , payer = seller , seeds = ["auction" . as_bytes () . as_ref () , auction_id . to_le_bytes () . as_ref ()] , bump)]
        pub auction: Box<Account<'info, dot::program::Auction>>,
        # [account (init , payer = seller , seeds = ["bid_escrow" . as_bytes () . as_ref () , auction_id . to_le_bytes () . as_ref ()] , bump , token :: mint = bid_mint , token :: authority = auction)]
        pub bid_escrow: Box<Account<'info, TokenAccount>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
        pub token_program: Program<'info, Token>,
    }

    pub fn create_auction(
        ctx: Context<CreateAuction>,
        auction_id: u64,
        starting_price: u64,
        duration_slots: u64,
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
        let seller = SeahorseSigner {
            account: &ctx.accounts.seller,
            programs: &programs_map,
        };

        let bid_mint = SeahorseAccount {
            account: &ctx.accounts.bid_mint,
            programs: &programs_map,
        };

        let auction = Empty {
            account: dot::program::Auction::load(&mut ctx.accounts.auction, &programs_map),
            bump: Some(ctx.bumps.auction),
        };

        let bid_escrow = Empty {
            account: SeahorseAccount {
                account: &ctx.accounts.bid_escrow,
                programs: &programs_map,
            },
            bump: Some(ctx.bumps.bid_escrow),
        };

        let clock = &ctx.accounts.clock.clone();

        create_auction_handler(
            seller.clone(),
            bid_mint.clone(),
            auction.clone(),
            bid_escrow.clone(),
            clock.clone(),
            auction_id,
            starting_price,
            duration_slots,
        );

        dot::program::Auction::store(auction.account);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct EndAuction<'info> {
        #[account(mut)]
        pub caller: Signer<'info>,
        #[account(mut)]
        pub auction: Box<Account<'info, dot::program::Auction>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
    }

    pub fn end_auction(ctx: Context<EndAuction>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let caller = SeahorseSigner {
            account: &ctx.accounts.caller,
            programs: &programs_map,
        };

        let auction = dot::program::Auction::load(&mut ctx.accounts.auction, &programs_map);
        let clock = &ctx.accounts.clock.clone();

        end_auction_handler(caller.clone(), auction.clone(), clock.clone());

        dot::program::Auction::store(auction);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (bid_amount : u64)]
    pub struct PlaceBid<'info> {
        #[account(mut)]
        pub bidder: Signer<'info>,
        #[account(mut)]
        #[doc = "CHECK: This account is unchecked."]
        pub previous_bidder: UncheckedAccount<'info>,
        #[account(mut)]
        pub bid_mint: Box<Account<'info, Mint>>,
        #[account(mut)]
        pub bidder_token_account: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub previous_bidder_token_account: Box<Account<'info, TokenAccount>>,
        #[account(mut)]
        pub auction: Box<Account<'info, dot::program::Auction>>,
        #[account(mut)]
        pub bid_escrow: Box<Account<'info, TokenAccount>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
        pub token_program: Program<'info, Token>,
    }

    pub fn place_bid(ctx: Context<PlaceBid>, bid_amount: u64) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "token_program",
            ctx.accounts.token_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let bidder = SeahorseSigner {
            account: &ctx.accounts.bidder,
            programs: &programs_map,
        };

        let previous_bidder = &ctx.accounts.previous_bidder.clone();
        let bid_mint = SeahorseAccount {
            account: &ctx.accounts.bid_mint,
            programs: &programs_map,
        };

        let bidder_token_account = SeahorseAccount {
            account: &ctx.accounts.bidder_token_account,
            programs: &programs_map,
        };

        let previous_bidder_token_account = SeahorseAccount {
            account: &ctx.accounts.previous_bidder_token_account,
            programs: &programs_map,
        };

        let auction = dot::program::Auction::load(&mut ctx.accounts.auction, &programs_map);
        let bid_escrow = SeahorseAccount {
            account: &ctx.accounts.bid_escrow,
            programs: &programs_map,
        };

        let clock = &ctx.accounts.clock.clone();

        place_bid_handler(
            bidder.clone(),
            previous_bidder.clone(),
            bid_mint.clone(),
            bidder_token_account.clone(),
            previous_bidder_token_account.clone(),
            auction.clone(),
            bid_escrow.clone(),
            clock.clone(),
            bid_amount,
        );

        dot::program::Auction::store(auction);

        return Ok(());
    }
}

