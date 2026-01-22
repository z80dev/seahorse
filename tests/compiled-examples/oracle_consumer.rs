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

seahorse_const! { MAX_PRICE_AGE_SLOTS , 100 }

seahorse_const! { PRICE_SCALE , 1000000000 }

#[account]
#[derive(Debug)]
pub struct PriceConfig {
    pub authority: Pubkey,
    pub config_id: u64,
    pub feed_name: String,
    pub oracle_address: Pubkey,
    pub max_staleness_slots: u64,
    pub last_price: i64,
    pub last_confidence: u64,
    pub last_exponent: i32,
    pub last_record_slot: u64,
    pub update_count: u64,
    pub bump: u8,
}

impl<'info, 'entrypoint> PriceConfig {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedPriceConfig<'info, 'entrypoint>> {
        let authority = account.authority.clone();
        let config_id = account.config_id;
        let feed_name = account.feed_name.clone();
        let oracle_address = account.oracle_address.clone();
        let max_staleness_slots = account.max_staleness_slots;
        let last_price = account.last_price;
        let last_confidence = account.last_confidence;
        let last_exponent = account.last_exponent;
        let last_record_slot = account.last_record_slot;
        let update_count = account.update_count;
        let bump = account.bump;

        Mutable::new(LoadedPriceConfig {
            __account__: account,
            __programs__: programs_map,
            authority,
            config_id,
            feed_name,
            oracle_address,
            max_staleness_slots,
            last_price,
            last_confidence,
            last_exponent,
            last_record_slot,
            update_count,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedPriceConfig>) {
        let mut loaded = loaded.borrow_mut();
        let authority = loaded.authority.clone();

        loaded.__account__.authority = authority;

        let config_id = loaded.config_id;

        loaded.__account__.config_id = config_id;

        let feed_name = loaded.feed_name.clone();

        loaded.__account__.feed_name = feed_name;

        let oracle_address = loaded.oracle_address.clone();

        loaded.__account__.oracle_address = oracle_address;

        let max_staleness_slots = loaded.max_staleness_slots;

        loaded.__account__.max_staleness_slots = max_staleness_slots;

        let last_price = loaded.last_price;

        loaded.__account__.last_price = last_price;

        let last_confidence = loaded.last_confidence;

        loaded.__account__.last_confidence = last_confidence;

        let last_exponent = loaded.last_exponent;

        loaded.__account__.last_exponent = last_exponent;

        let last_record_slot = loaded.last_record_slot;

        loaded.__account__.last_record_slot = last_record_slot;

        let update_count = loaded.update_count;

        loaded.__account__.update_count = update_count;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedPriceConfig<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, PriceConfig>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub authority: Pubkey,
    pub config_id: u64,
    pub feed_name: String,
    pub oracle_address: Pubkey,
    pub max_staleness_slots: u64,
    pub last_price: i64,
    pub last_confidence: u64,
    pub last_exponent: i32,
    pub last_record_slot: u64,
    pub update_count: u64,
    pub bump: u8,
}

#[account]
#[derive(Debug)]
pub struct PriceThreshold {
    pub config: Pubkey,
    pub threshold_id: u64,
    pub trigger_above: bool,
    pub target_price_scaled: u64,
    pub is_triggered: bool,
    pub triggered_at_slot: u64,
    pub bump: u8,
}

impl<'info, 'entrypoint> PriceThreshold {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedPriceThreshold<'info, 'entrypoint>> {
        let config = account.config.clone();
        let threshold_id = account.threshold_id;
        let trigger_above = account.trigger_above.clone();
        let target_price_scaled = account.target_price_scaled;
        let is_triggered = account.is_triggered.clone();
        let triggered_at_slot = account.triggered_at_slot;
        let bump = account.bump;

        Mutable::new(LoadedPriceThreshold {
            __account__: account,
            __programs__: programs_map,
            config,
            threshold_id,
            trigger_above,
            target_price_scaled,
            is_triggered,
            triggered_at_slot,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedPriceThreshold>) {
        let mut loaded = loaded.borrow_mut();
        let config = loaded.config.clone();

        loaded.__account__.config = config;

        let threshold_id = loaded.threshold_id;

        loaded.__account__.threshold_id = threshold_id;

        let trigger_above = loaded.trigger_above.clone();

        loaded.__account__.trigger_above = trigger_above;

        let target_price_scaled = loaded.target_price_scaled;

        loaded.__account__.target_price_scaled = target_price_scaled;

        let is_triggered = loaded.is_triggered.clone();

        loaded.__account__.is_triggered = is_triggered;

        let triggered_at_slot = loaded.triggered_at_slot;

        loaded.__account__.triggered_at_slot = triggered_at_slot;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedPriceThreshold<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, PriceThreshold>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub config: Pubkey,
    pub threshold_id: u64,
    pub trigger_above: bool,
    pub target_price_scaled: u64,
    pub is_triggered: bool,
    pub triggered_at_slot: u64,
    pub bump: u8,
}

pub fn check_threshold_handler<'info>(
    mut price_config: Mutable<LoadedPriceConfig<'info, '_>>,
    mut price_threshold: Mutable<LoadedPriceThreshold<'info, '_>>,
    mut clock: Sysvar<'info, Clock>,
) -> () {
    "Check if price threshold has been crossed.".to_string();

    if !(price_threshold.borrow().config == price_config.borrow().__account__.key()) {
        panic!("Threshold does not belong to this config");
    }

    if price_threshold.borrow().is_triggered {
        solana_program::msg!("{}", "Threshold already triggered".to_string());

        return;
    }

    let mut raw_price = price_config.borrow().last_price;
    let mut exponent = price_config.borrow().last_exponent;

    if raw_price < 0 {
        solana_program::msg!(
            "{}",
            "Negative price not supported for threshold".to_string()
        );

        return;
    }

    let mut price_u64 = <u64 as TryFrom<_>>::try_from(raw_price.clone()).unwrap();
    let mut target = price_threshold.borrow().target_price_scaled;
    let mut trigger_above = price_threshold.borrow().trigger_above;
    let mut triggered = false;

    if trigger_above {
        if price_u64 >= target {
            triggered = true;
        }
    } else {
        if price_u64 <= target {
            triggered = true;
        }
    }

    if triggered {
        let mut current_slot = clock.slot;

        assign!(price_threshold.borrow_mut().is_triggered, true);

        assign!(price_threshold.borrow_mut().triggered_at_slot, current_slot);

        solana_program::msg!("{}", "Threshold triggered!".to_string());
    }
}

pub fn create_price_config_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut price_config: Empty<Mutable<LoadedPriceConfig<'info, '_>>>,
    mut config_id: u64,
    mut feed_name: String,
    mut oracle_address: Pubkey,
    mut max_staleness_slots: u64,
) -> () {
    "Create a new price feed configuration.".to_string();

    if !((feed_name.chars().count() as u64) <= 32) {
        panic!("Feed name too long (max 32 chars)");
    }

    if !(max_staleness_slots > 0) {
        panic!("Max staleness must be greater than zero");
    }

    let mut config_bump = price_config.bump.unwrap();
    let mut price_config = price_config.account.clone();

    assign!(price_config.borrow_mut().authority, authority.key());

    assign!(price_config.borrow_mut().config_id, config_id);

    assign!(price_config.borrow_mut().feed_name, feed_name.clone());

    assign!(price_config.borrow_mut().oracle_address, oracle_address);

    assign!(
        price_config.borrow_mut().max_staleness_slots,
        max_staleness_slots
    );

    assign!(price_config.borrow_mut().last_price, 0);

    assign!(price_config.borrow_mut().last_confidence, 0);

    assign!(price_config.borrow_mut().last_exponent, 0);

    assign!(price_config.borrow_mut().last_record_slot, 0);

    assign!(price_config.borrow_mut().update_count, 0);

    assign!(price_config.borrow_mut().bump, config_bump);
}

pub fn create_price_threshold_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut price_config: Mutable<LoadedPriceConfig<'info, '_>>,
    mut price_threshold: Empty<Mutable<LoadedPriceThreshold<'info, '_>>>,
    mut config_id: u64,
    mut threshold_id: u64,
    mut trigger_above: bool,
    mut target_price_scaled: u64,
) -> () {
    "Create a price threshold for monitoring.".to_string();

    if !(authority.key() == price_config.borrow().authority) {
        panic!("Unauthorized");
    }

    if !(config_id == price_config.borrow().config_id) {
        panic!("Config ID mismatch");
    }

    if !(target_price_scaled > 0) {
        panic!("Target price must be greater than zero");
    }

    let mut threshold_bump = price_threshold.bump.unwrap();
    let mut price_threshold = price_threshold.account.clone();

    assign!(
        price_threshold.borrow_mut().config,
        price_config.borrow().__account__.key()
    );

    assign!(price_threshold.borrow_mut().threshold_id, threshold_id);

    assign!(price_threshold.borrow_mut().trigger_above, trigger_above);

    assign!(
        price_threshold.borrow_mut().target_price_scaled,
        target_price_scaled
    );

    assign!(price_threshold.borrow_mut().is_triggered, false);

    assign!(price_threshold.borrow_mut().triggered_at_slot, 0);

    assign!(price_threshold.borrow_mut().bump, threshold_bump);
}

pub fn get_price_info_handler<'info>(
    mut price_config: Mutable<LoadedPriceConfig<'info, '_>>,
    mut clock: Sysvar<'info, Clock>,
) -> () {
    "Display current price information and staleness status.".to_string();

    let mut current_slot = clock.slot;
    let mut age_slots = current_slot - price_config.borrow().last_record_slot;
    let mut is_stale = age_slots > price_config.borrow().max_staleness_slots;

    solana_program::msg!("{}", price_config.borrow().last_price);

    solana_program::msg!("{}", price_config.borrow().last_confidence);

    solana_program::msg!("{}", price_config.borrow().last_exponent);

    solana_program::msg!("{}", age_slots);

    if is_stale {
        solana_program::msg!("{}", "Price is STALE".to_string());
    } else {
        solana_program::msg!("{}", "Price is FRESH".to_string());
    }
}

pub fn record_price_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut price_config: Mutable<LoadedPriceConfig<'info, '_>>,
    mut clock: Sysvar<'info, Clock>,
    mut oracle_price: i64,
    mut oracle_confidence: u64,
    mut oracle_exponent: i32,
    mut oracle_publish_slot: u64,
) -> () {
    "\n    Record a price from the oracle.\n\n    In production, this would read from the actual oracle account.\n    For testing, we pass price data as parameters.\n    " . to_string () ;

    if !(authority.key() == price_config.borrow().authority) {
        panic!("Unauthorized");
    }

    let mut current_slot = clock.slot;
    let mut price_age = current_slot - oracle_publish_slot;

    if !(price_age <= price_config.borrow().max_staleness_slots) {
        panic!("Price is stale");
    }

    assign!(price_config.borrow_mut().last_price, oracle_price);

    assign!(price_config.borrow_mut().last_confidence, oracle_confidence);

    assign!(price_config.borrow_mut().last_exponent, oracle_exponent);

    assign!(price_config.borrow_mut().last_record_slot, current_slot);

    assign!(
        price_config.borrow_mut().update_count,
        price_config.borrow().update_count + 1
    );

    solana_program::msg!("{}", oracle_price);

    solana_program::msg!("{}", oracle_exponent);
}

pub fn reset_threshold_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut price_config: Mutable<LoadedPriceConfig<'info, '_>>,
    mut price_threshold: Mutable<LoadedPriceThreshold<'info, '_>>,
) -> () {
    "Reset a triggered threshold (authority only).".to_string();

    if !(authority.key() == price_config.borrow().authority) {
        panic!("Unauthorized");
    }

    if !(price_threshold.borrow().config == price_config.borrow().__account__.key()) {
        panic!("Threshold does not belong to this config");
    }

    assign!(price_threshold.borrow_mut().is_triggered, false);

    assign!(price_threshold.borrow_mut().triggered_at_slot, 0);
}

pub fn update_price_config_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut price_config: Mutable<LoadedPriceConfig<'info, '_>>,
    mut new_oracle_address: Pubkey,
    mut new_max_staleness_slots: u64,
) -> () {
    "Update price feed configuration (authority only).".to_string();

    if !(authority.key() == price_config.borrow().authority) {
        panic!("Unauthorized");
    }

    if !(new_max_staleness_slots > 0) {
        panic!("Max staleness must be greater than zero");
    }

    assign!(price_config.borrow_mut().oracle_address, new_oracle_address);

    assign!(
        price_config.borrow_mut().max_staleness_slots,
        new_max_staleness_slots
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

declare_id!("3ePvSJdgkK1r5CrxyEjJoMnSCJJjCMoRKNdYLi6Ws8g7");

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
mod oracle_consumer {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    pub struct CheckThreshold<'info> {
        #[account(mut)]
        pub price_config: Box<Account<'info, dot::program::PriceConfig>>,
        #[account(mut)]
        pub price_threshold: Box<Account<'info, dot::program::PriceThreshold>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
    }

    pub fn check_threshold(ctx: Context<CheckThreshold>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let price_config =
            dot::program::PriceConfig::load(&mut ctx.accounts.price_config, &programs_map);

        let price_threshold =
            dot::program::PriceThreshold::load(&mut ctx.accounts.price_threshold, &programs_map);

        let clock = &ctx.accounts.clock.clone();

        check_threshold_handler(price_config.clone(), price_threshold.clone(), clock.clone());

        dot::program::PriceConfig::store(price_config);

        dot::program::PriceThreshold::store(price_threshold);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (config_id : u64 , feed_name : String , oracle_address : Pubkey , max_staleness_slots : u64)]
    pub struct CreatePriceConfig<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: PriceConfig > () + 8 + (50 as usize) , payer = authority , seeds = ["price_config" . as_bytes () . as_ref () , config_id . to_le_bytes () . as_ref ()] , bump)]
        pub price_config: Box<Account<'info, dot::program::PriceConfig>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn create_price_config(
        ctx: Context<CreatePriceConfig>,
        config_id: u64,
        feed_name: String,
        oracle_address: Pubkey,
        max_staleness_slots: u64,
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

        let price_config = Empty {
            account: dot::program::PriceConfig::load(&mut ctx.accounts.price_config, &programs_map),
            bump: Some(ctx.bumps.price_config),
        };

        create_price_config_handler(
            authority.clone(),
            price_config.clone(),
            config_id,
            feed_name,
            oracle_address,
            max_staleness_slots,
        );

        dot::program::PriceConfig::store(price_config.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (config_id : u64 , threshold_id : u64 , trigger_above : bool , target_price_scaled : u64)]
    pub struct CreatePriceThreshold<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub price_config: Box<Account<'info, dot::program::PriceConfig>>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: PriceThreshold > () + 8 , payer = authority , seeds = ["threshold" . as_bytes () . as_ref () , config_id . to_le_bytes () . as_ref () , threshold_id . to_le_bytes () . as_ref ()] , bump)]
        pub price_threshold: Box<Account<'info, dot::program::PriceThreshold>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn create_price_threshold(
        ctx: Context<CreatePriceThreshold>,
        config_id: u64,
        threshold_id: u64,
        trigger_above: bool,
        target_price_scaled: u64,
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

        let price_config =
            dot::program::PriceConfig::load(&mut ctx.accounts.price_config, &programs_map);

        let price_threshold = Empty {
            account: dot::program::PriceThreshold::load(
                &mut ctx.accounts.price_threshold,
                &programs_map,
            ),
            bump: Some(ctx.bumps.price_threshold),
        };

        create_price_threshold_handler(
            authority.clone(),
            price_config.clone(),
            price_threshold.clone(),
            config_id,
            threshold_id,
            trigger_above,
            target_price_scaled,
        );

        dot::program::PriceConfig::store(price_config);

        dot::program::PriceThreshold::store(price_threshold.account);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct GetPriceInfo<'info> {
        #[account(mut)]
        pub price_config: Box<Account<'info, dot::program::PriceConfig>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
    }

    pub fn get_price_info(ctx: Context<GetPriceInfo>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let price_config =
            dot::program::PriceConfig::load(&mut ctx.accounts.price_config, &programs_map);

        let clock = &ctx.accounts.clock.clone();

        get_price_info_handler(price_config.clone(), clock.clone());

        dot::program::PriceConfig::store(price_config);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (oracle_price : i64 , oracle_confidence : u64 , oracle_exponent : i32 , oracle_publish_slot : u64)]
    pub struct RecordPrice<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub price_config: Box<Account<'info, dot::program::PriceConfig>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
    }

    pub fn record_price(
        ctx: Context<RecordPrice>,
        oracle_price: i64,
        oracle_confidence: u64,
        oracle_exponent: i32,
        oracle_publish_slot: u64,
    ) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let price_config =
            dot::program::PriceConfig::load(&mut ctx.accounts.price_config, &programs_map);

        let clock = &ctx.accounts.clock.clone();

        record_price_handler(
            authority.clone(),
            price_config.clone(),
            clock.clone(),
            oracle_price,
            oracle_confidence,
            oracle_exponent,
            oracle_publish_slot,
        );

        dot::program::PriceConfig::store(price_config);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct ResetThreshold<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub price_config: Box<Account<'info, dot::program::PriceConfig>>,
        #[account(mut)]
        pub price_threshold: Box<Account<'info, dot::program::PriceThreshold>>,
    }

    pub fn reset_threshold(ctx: Context<ResetThreshold>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let price_config =
            dot::program::PriceConfig::load(&mut ctx.accounts.price_config, &programs_map);

        let price_threshold =
            dot::program::PriceThreshold::load(&mut ctx.accounts.price_threshold, &programs_map);

        reset_threshold_handler(
            authority.clone(),
            price_config.clone(),
            price_threshold.clone(),
        );

        dot::program::PriceConfig::store(price_config);

        dot::program::PriceThreshold::store(price_threshold);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (new_oracle_address : Pubkey , new_max_staleness_slots : u64)]
    pub struct UpdatePriceConfig<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub price_config: Box<Account<'info, dot::program::PriceConfig>>,
    }

    pub fn update_price_config(
        ctx: Context<UpdatePriceConfig>,
        new_oracle_address: Pubkey,
        new_max_staleness_slots: u64,
    ) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let price_config =
            dot::program::PriceConfig::load(&mut ctx.accounts.price_config, &programs_map);

        update_price_config_handler(
            authority.clone(),
            price_config.clone(),
            new_oracle_address,
            new_max_staleness_slots,
        );

        dot::program::PriceConfig::store(price_config);

        return Ok(());
    }
}

