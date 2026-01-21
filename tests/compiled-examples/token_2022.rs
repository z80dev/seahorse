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

seahorse_const! { MAX_FEE_BASIS_POINTS , 10000 }

#[account]
#[derive(Debug)]
pub struct FeeConfig {
    pub mint: Pubkey,
    pub authority: Pubkey,
    pub decimals: u8,
    pub transfer_fee_bps: u16,
    pub max_fee: u64,
    pub total_fees_collected: u64,
    pub bump: u8,
}

impl<'info, 'entrypoint> FeeConfig {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedFeeConfig<'info, 'entrypoint>> {
        let mint = account.mint.clone();
        let authority = account.authority.clone();
        let decimals = account.decimals;
        let transfer_fee_bps = account.transfer_fee_bps;
        let max_fee = account.max_fee;
        let total_fees_collected = account.total_fees_collected;
        let bump = account.bump;

        Mutable::new(LoadedFeeConfig {
            __account__: account,
            __programs__: programs_map,
            mint,
            authority,
            decimals,
            transfer_fee_bps,
            max_fee,
            total_fees_collected,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedFeeConfig>) {
        let mut loaded = loaded.borrow_mut();
        let mint = loaded.mint.clone();

        loaded.__account__.mint = mint;

        let authority = loaded.authority.clone();

        loaded.__account__.authority = authority;

        let decimals = loaded.decimals;

        loaded.__account__.decimals = decimals;

        let transfer_fee_bps = loaded.transfer_fee_bps;

        loaded.__account__.transfer_fee_bps = transfer_fee_bps;

        let max_fee = loaded.max_fee;

        loaded.__account__.max_fee = max_fee;

        let total_fees_collected = loaded.total_fees_collected;

        loaded.__account__.total_fees_collected = total_fees_collected;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedFeeConfig<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, FeeConfig>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub mint: Pubkey,
    pub authority: Pubkey,
    pub decimals: u8,
    pub transfer_fee_bps: u16,
    pub max_fee: u64,
    pub total_fees_collected: u64,
    pub bump: u8,
}

pub fn calculate_transfer_fee_handler<'info>(
    mut fee_config: Mutable<LoadedFeeConfig<'info, '_>>,
    mut amount: u64,
) -> u64 {
    "\n    Calculate the transfer fee for a given amount.\n\n    This demonstrates the fee calculation logic used by Token-2022:\n    fee = min(amount * fee_bps / 10000, max_fee)\n    " . to_string () ;

    let mut fee = (amount
        * <u64 as TryFrom<_>>::try_from(fee_config.borrow().transfer_fee_bps.clone()).unwrap())
        / 10000;

    if fee > fee_config.borrow().max_fee {
        fee = fee_config.borrow().max_fee;
    }

    solana_program::msg!("{} {}", "Amount:".to_string(), amount);

    solana_program::msg!(
        "{} {} {}",
        "Fee rate:".to_string(),
        fee_config.borrow().transfer_fee_bps,
        "bps".to_string()
    );

    solana_program::msg!("{} {}", "Calculated fee:".to_string(), fee);

    return fee;
}

pub fn get_fee_info_handler<'info>(mut fee_config: Mutable<LoadedFeeConfig<'info, '_>>) -> () {
    "\n    Display the current fee configuration.\n    ".to_string();

    solana_program::msg!("{} {:?}", "Mint:".to_string(), fee_config.borrow().mint);

    solana_program::msg!(
        "{} {:?}",
        "Authority:".to_string(),
        fee_config.borrow().authority
    );

    solana_program::msg!(
        "{} {}",
        "Decimals:".to_string(),
        fee_config.borrow().decimals
    );

    solana_program::msg!(
        "{} {} {}",
        "Transfer fee:".to_string(),
        fee_config.borrow().transfer_fee_bps,
        "bps".to_string()
    );

    solana_program::msg!("{} {}", "Max fee:".to_string(), fee_config.borrow().max_fee);

    solana_program::msg!(
        "{} {}",
        "Total collected:".to_string(),
        fee_config.borrow().total_fees_collected
    );
}

pub fn initialize_fee_config_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut fee_config: Empty<Mutable<LoadedFeeConfig<'info, '_>>>,
    mut mint: UncheckedAccount<'info>,
    mut decimals: u8,
    mut transfer_fee_bps: u16,
    mut max_fee: u64,
) -> () {
    "\n    Initialize a fee configuration for an existing Token-2022 mint.\n\n    The mint must be created externally with the transfer fee extension enabled.\n    This instruction stores the fee configuration in a program-owned account\n    for tracking and validation purposes.\n\n    NOTE: In a full implementation, this would also call Token-2022 to set up\n    the transfer fee extension. Seahorse doesn't support this natively.\n    " . to_string () ;

    if !(transfer_fee_bps <= MAX_FEE_BASIS_POINTS!()) {
        panic!("Fee exceeds maximum (10000 bps)");
    }

    let mut config_bump = fee_config.bump.unwrap();
    let mut fee_config = fee_config.account.clone();

    assign!(fee_config.borrow_mut().mint, mint.key());

    assign!(fee_config.borrow_mut().authority, authority.key());

    assign!(fee_config.borrow_mut().decimals, decimals);

    assign!(fee_config.borrow_mut().transfer_fee_bps, transfer_fee_bps);

    assign!(fee_config.borrow_mut().max_fee, max_fee);

    assign!(fee_config.borrow_mut().total_fees_collected, 0);

    assign!(fee_config.borrow_mut().bump, config_bump);

    solana_program::msg!(
        "{} {:?}",
        "Initialized fee config for mint:".to_string(),
        mint.key()
    );

    solana_program::msg!(
        "{} {} {}",
        "Transfer fee:".to_string(),
        transfer_fee_bps,
        "bps".to_string()
    );

    solana_program::msg!("{} {}", "Max fee:".to_string(), max_fee);
}

pub fn record_fee_collection_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut fee_config: Mutable<LoadedFeeConfig<'info, '_>>,
    mut amount_collected: u64,
) -> () {
    "\n    Record fees collected from Token-2022 transfers.\n\n    This is called after withdrawing withheld fees from Token-2022\n    to update the tracking in our fee config account.\n    " . to_string () ;

    if !(authority.key() == fee_config.borrow().authority) {
        panic!("Unauthorized");
    }

    assign!(
        fee_config.borrow_mut().total_fees_collected,
        fee_config.borrow().total_fees_collected + amount_collected
    );

    solana_program::msg!(
        "{} {}",
        "Recorded fee collection:".to_string(),
        amount_collected
    );

    solana_program::msg!(
        "{} {}",
        "Total fees collected:".to_string(),
        fee_config.borrow().total_fees_collected
    );
}

pub fn update_fee_rate_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut fee_config: Mutable<LoadedFeeConfig<'info, '_>>,
    mut new_transfer_fee_bps: u16,
    mut new_max_fee: u64,
) -> () {
    "\n    Update the transfer fee rate.\n\n    NOTE: In a full implementation, this would also call Token-2022's\n    set_transfer_fee instruction. Seahorse doesn't support this natively.\n    " . to_string () ;

    if !(authority.key() == fee_config.borrow().authority) {
        panic!("Unauthorized");
    }

    if !(new_transfer_fee_bps <= MAX_FEE_BASIS_POINTS!()) {
        panic!("Fee exceeds maximum (10000 bps)");
    }

    assign!(
        fee_config.borrow_mut().transfer_fee_bps,
        new_transfer_fee_bps
    );

    assign!(fee_config.borrow_mut().max_fee, new_max_fee);

    solana_program::msg!(
        "{} {} {}",
        "Updated fee rate to:".to_string(),
        new_transfer_fee_bps,
        "bps".to_string()
    );

    solana_program::msg!("{} {}", "New max fee:".to_string(), new_max_fee);
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

declare_id!("T2Ex5VzVZYxLYtDmJq5DEmUKDEcPnP2zGxqHKbv2Tbn");

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
mod token_2022 {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    # [instruction (amount : u64)]
    pub struct CalculateTransferFee<'info> {
        #[account(mut)]
        pub fee_config: Box<Account<'info, dot::program::FeeConfig>>,
    }

    pub fn calculate_transfer_fee(ctx: Context<CalculateTransferFee>, amount: u64) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let fee_config = dot::program::FeeConfig::load(&mut ctx.accounts.fee_config, &programs_map);

        calculate_transfer_fee_handler(fee_config.clone(), amount);

        dot::program::FeeConfig::store(fee_config);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct GetFeeInfo<'info> {
        #[account(mut)]
        pub fee_config: Box<Account<'info, dot::program::FeeConfig>>,
    }

    pub fn get_fee_info(ctx: Context<GetFeeInfo>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let fee_config = dot::program::FeeConfig::load(&mut ctx.accounts.fee_config, &programs_map);

        get_fee_info_handler(fee_config.clone());

        dot::program::FeeConfig::store(fee_config);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (decimals : u8 , transfer_fee_bps : u16 , max_fee : u64)]
    pub struct InitializeFeeConfig<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: FeeConfig > () + 8 , payer = authority , seeds = ["fee_config" . as_bytes () . as_ref () , authority . key () . as_ref ()] , bump)]
        pub fee_config: Box<Account<'info, dot::program::FeeConfig>>,
        #[account(mut)]
        #[doc = "CHECK: This account is unchecked."]
        pub mint: UncheckedAccount<'info>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn initialize_fee_config(
        ctx: Context<InitializeFeeConfig>,
        decimals: u8,
        transfer_fee_bps: u16,
        max_fee: u64,
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

        let fee_config = Empty {
            account: dot::program::FeeConfig::load(&mut ctx.accounts.fee_config, &programs_map),
            bump: Some(ctx.bumps.fee_config),
        };

        let mint = &ctx.accounts.mint.clone();

        initialize_fee_config_handler(
            authority.clone(),
            fee_config.clone(),
            mint.clone(),
            decimals,
            transfer_fee_bps,
            max_fee,
        );

        dot::program::FeeConfig::store(fee_config.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (amount_collected : u64)]
    pub struct RecordFeeCollection<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub fee_config: Box<Account<'info, dot::program::FeeConfig>>,
    }

    pub fn record_fee_collection(
        ctx: Context<RecordFeeCollection>,
        amount_collected: u64,
    ) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let fee_config = dot::program::FeeConfig::load(&mut ctx.accounts.fee_config, &programs_map);

        record_fee_collection_handler(authority.clone(), fee_config.clone(), amount_collected);

        dot::program::FeeConfig::store(fee_config);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (new_transfer_fee_bps : u16 , new_max_fee : u64)]
    pub struct UpdateFeeRate<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub fee_config: Box<Account<'info, dot::program::FeeConfig>>,
    }

    pub fn update_fee_rate(
        ctx: Context<UpdateFeeRate>,
        new_transfer_fee_bps: u16,
        new_max_fee: u64,
    ) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let fee_config = dot::program::FeeConfig::load(&mut ctx.accounts.fee_config, &programs_map);

        update_fee_rate_handler(
            authority.clone(),
            fee_config.clone(),
            new_transfer_fee_bps,
            new_max_fee,
        );

        dot::program::FeeConfig::store(fee_config);

        return Ok(());
    }
}

