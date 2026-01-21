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
pub struct ErrorDemo {
    pub authority: Pubkey,
    pub value: u64,
    pub operation_count: u64,
    pub is_active: bool,
    pub name: String,
    pub max_value: u64,
    pub bump: u8,
}

impl<'info, 'entrypoint> ErrorDemo {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedErrorDemo<'info, 'entrypoint>> {
        let authority = account.authority.clone();
        let value = account.value;
        let operation_count = account.operation_count;
        let is_active = account.is_active.clone();
        let name = account.name.clone();
        let max_value = account.max_value;
        let bump = account.bump;

        Mutable::new(LoadedErrorDemo {
            __account__: account,
            __programs__: programs_map,
            authority,
            value,
            operation_count,
            is_active,
            name,
            max_value,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedErrorDemo>) {
        let mut loaded = loaded.borrow_mut();
        let authority = loaded.authority.clone();

        loaded.__account__.authority = authority;

        let value = loaded.value;

        loaded.__account__.value = value;

        let operation_count = loaded.operation_count;

        loaded.__account__.operation_count = operation_count;

        let is_active = loaded.is_active.clone();

        loaded.__account__.is_active = is_active;

        let name = loaded.name.clone();

        loaded.__account__.name = name;

        let max_value = loaded.max_value;

        loaded.__account__.max_value = max_value;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedErrorDemo<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, ErrorDemo>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub authority: Pubkey,
    pub value: u64,
    pub operation_count: u64,
    pub is_active: bool,
    pub name: String,
    pub max_value: u64,
    pub bump: u8,
}

#[account]
#[derive(Debug)]
pub struct RoleRegistry {
    pub admin: Pubkey,
    pub operator: Pubkey,
    pub has_operator: bool,
    pub registry_id: u64,
    pub bump: u8,
}

impl<'info, 'entrypoint> RoleRegistry {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedRoleRegistry<'info, 'entrypoint>> {
        let admin = account.admin.clone();
        let operator = account.operator.clone();
        let has_operator = account.has_operator.clone();
        let registry_id = account.registry_id;
        let bump = account.bump;

        Mutable::new(LoadedRoleRegistry {
            __account__: account,
            __programs__: programs_map,
            admin,
            operator,
            has_operator,
            registry_id,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedRoleRegistry>) {
        let mut loaded = loaded.borrow_mut();
        let admin = loaded.admin.clone();

        loaded.__account__.admin = admin;

        let operator = loaded.operator.clone();

        loaded.__account__.operator = operator;

        let has_operator = loaded.has_operator.clone();

        loaded.__account__.has_operator = has_operator;

        let registry_id = loaded.registry_id;

        loaded.__account__.registry_id = registry_id;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedRoleRegistry<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, RoleRegistry>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub admin: Pubkey,
    pub operator: Pubkey,
    pub has_operator: bool,
    pub registry_id: u64,
    pub bump: u8,
}

pub fn admin_only_action_handler<'info>(
    mut caller: SeahorseSigner<'info, '_>,
    mut registry: Mutable<LoadedRoleRegistry<'info, '_>>,
) -> () {
    "Action that requires admin role.".to_string();

    if !(caller.key() == registry.borrow().admin) {
        panic!("Unauthorized: admin role required");
    }

    solana_program::msg!("{}", format!("Admin action executed by {:?}", caller.key()));
}

pub fn complex_validation_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut error_demo: Mutable<LoadedErrorDemo<'info, '_>>,
    mut new_value: u64,
    mut new_name: String,
    mut require_high_value: bool,
) -> () {
    "Demonstrate multiple validation checks in sequence.".to_string();

    if !(authority.key() == error_demo.borrow().authority) {
        panic!("Unauthorized: only authority can perform complex validation");
    }

    if !error_demo.borrow().is_active {
        panic!("Account is not active");
    }

    if !(new_value > 0) {
        panic!("Value must be greater than zero");
    }

    if !(new_value <= error_demo.borrow().max_value) {
        panic!("Value exceeds maximum allowed");
    }

    let mut half_max = error_demo.borrow().max_value / 2;

    if require_high_value {
        if !(new_value >= half_max) {
            panic!("Value must be at least half of max when high value required");
        }
    }

    if !((new_name.chars().count() as u64) > 0) {
        panic!("Name cannot be empty");
    }

    if !((new_name.chars().count() as u64) <= 32) {
        panic!("Name exceeds maximum length of 32 characters");
    }

    solana_program::msg!(
        "{}",
        format!(
            "Complex validation passed: value={}, name={}",
            new_value, new_name
        )
    );

    assign!(error_demo.borrow_mut().value, new_value);

    assign!(error_demo.borrow_mut().name, new_name);

    assign!(
        error_demo.borrow_mut().operation_count,
        error_demo.borrow().operation_count + 1
    );
}

pub fn deactivate_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut error_demo: Mutable<LoadedErrorDemo<'info, '_>>,
) -> () {
    "Deactivate the account (can only be done once when active).".to_string();

    if !(authority.key() == error_demo.borrow().authority) {
        panic!("Unauthorized: only authority can deactivate");
    }

    if !error_demo.borrow().is_active {
        panic!("Account is already deactivated");
    }

    assign!(error_demo.borrow_mut().is_active, false);

    assign!(
        error_demo.borrow_mut().operation_count,
        error_demo.borrow().operation_count + 1
    );

    solana_program::msg!("{}", "Account deactivated".to_string());
}

pub fn decrement_value_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut error_demo: Mutable<LoadedErrorDemo<'info, '_>>,
    mut amount: u64,
) -> () {
    "Decrement value with underflow prevention.".to_string();

    if !(authority.key() == error_demo.borrow().authority) {
        panic!("Unauthorized: only authority can decrement");
    }

    if !error_demo.borrow().is_active {
        panic!("Account is not active");
    }

    if !(amount > 0) {
        panic!("Decrement amount must be greater than zero");
    }

    if !(error_demo.borrow().value >= amount) {
        panic!("Underflow: value would go below zero");
    }

    assign!(
        error_demo.borrow_mut().value,
        error_demo.borrow().value - amount
    );

    assign!(
        error_demo.borrow_mut().operation_count,
        error_demo.borrow().operation_count + 1
    );

    solana_program::msg!(
        "{}",
        format!(
            "Value decremented by {} to {}",
            amount,
            error_demo.borrow().value
        )
    );
}

pub fn get_demo_info_handler<'info>(mut error_demo: Mutable<LoadedErrorDemo<'info, '_>>) -> () {
    "Get ErrorDemo info (read-only, for logging).".to_string();

    solana_program::msg!(
        "{}",
        format!("Authority: {:?}", error_demo.borrow().authority)
    );

    solana_program::msg!("{}", format!("Value: {}", error_demo.borrow().value));

    solana_program::msg!(
        "{}",
        format!("Max Value: {}", error_demo.borrow().max_value)
    );

    solana_program::msg!("{}", format!("Name: {}", error_demo.borrow().name));

    solana_program::msg!(
        "{}",
        format!("Is Active: {}", error_demo.borrow().is_active)
    );

    solana_program::msg!(
        "{}",
        format!("Operation Count: {}", error_demo.borrow().operation_count)
    );
}

pub fn increment_value_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut error_demo: Mutable<LoadedErrorDemo<'info, '_>>,
    mut amount: u64,
) -> () {
    "Increment value with overflow prevention.".to_string();

    if !(authority.key() == error_demo.borrow().authority) {
        panic!("Unauthorized: only authority can increment");
    }

    if !error_demo.borrow().is_active {
        panic!("Account is not active");
    }

    if !(amount > 0) {
        panic!("Increment amount must be greater than zero");
    }

    let mut new_value = error_demo.borrow().value + amount;

    if !(new_value >= error_demo.borrow().value) {
        panic!("Overflow: increment would overflow");
    }

    if !(new_value <= error_demo.borrow().max_value) {
        panic!("Value would exceed maximum allowed");
    }

    assign!(error_demo.borrow_mut().value, new_value);

    assign!(
        error_demo.borrow_mut().operation_count,
        error_demo.borrow().operation_count + 1
    );

    solana_program::msg!(
        "{}",
        format!("Value incremented by {} to {}", amount, new_value)
    );
}

pub fn initialize_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut error_demo: Empty<Mutable<LoadedErrorDemo<'info, '_>>>,
    mut max_value: u64,
    mut name: String,
) -> () {
    "Initialize an ErrorDemo account with validation checks.".to_string();

    if !(max_value > 0) {
        panic!("Max value must be greater than zero");
    }

    if !(max_value <= 1000000) {
        panic!("Max value exceeds maximum allowed (1,000,000)");
    }

    if !((name.chars().count() as u64) > 0) {
        panic!("Name cannot be empty");
    }

    if !((name.chars().count() as u64) <= 32) {
        panic!("Name exceeds maximum length of 32 characters");
    }

    let mut demo_bump = error_demo.bump.unwrap();
    let mut error_demo = error_demo.account.clone();

    assign!(error_demo.borrow_mut().authority, authority.key());

    assign!(error_demo.borrow_mut().value, 0);

    assign!(error_demo.borrow_mut().operation_count, 0);

    assign!(error_demo.borrow_mut().is_active, true);

    assign!(error_demo.borrow_mut().name, name);

    assign!(error_demo.borrow_mut().max_value, max_value);

    assign!(error_demo.borrow_mut().bump, demo_bump);

    solana_program::msg!(
        "{}",
        format!(
            "ErrorDemo initialized: authority={:?}, max_value={}",
            authority.key(),
            max_value
        )
    );
}

pub fn initialize_role_registry_handler<'info>(
    mut admin: SeahorseSigner<'info, '_>,
    mut registry: Empty<Mutable<LoadedRoleRegistry<'info, '_>>>,
    mut registry_id: u64,
) -> () {
    "Initialize a role registry with admin.".to_string();

    let mut registry_bump = registry.bump.unwrap();
    let mut registry = registry.account.clone();

    assign!(registry.borrow_mut().admin, admin.key());

    assign!(registry.borrow_mut().operator, admin.key());

    assign!(registry.borrow_mut().has_operator, false);

    assign!(registry.borrow_mut().registry_id, registry_id);

    assign!(registry.borrow_mut().bump, registry_bump);

    solana_program::msg!(
        "{}",
        format!(
            "RoleRegistry {} initialized with admin {:?}",
            registry_id,
            admin.key()
        )
    );
}

pub fn operator_action_handler<'info>(
    mut caller: SeahorseSigner<'info, '_>,
    mut registry: Mutable<LoadedRoleRegistry<'info, '_>>,
) -> () {
    "Action that requires operator or admin role.".to_string();

    if !registry.borrow().has_operator {
        panic!("No operator has been set");
    }

    let mut is_admin = caller.key() == registry.borrow().admin;
    let mut is_operator = caller.key() == registry.borrow().operator;

    if !(is_admin || is_operator) {
        panic!("Unauthorized: operator or admin role required");
    }

    solana_program::msg!(
        "{}",
        format!("Operator action executed by {:?}", caller.key())
    );
}

pub fn reactivate_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut error_demo: Mutable<LoadedErrorDemo<'info, '_>>,
) -> () {
    "Reactivate the account (can only be done when inactive).".to_string();

    if !(authority.key() == error_demo.borrow().authority) {
        panic!("Unauthorized: only authority can reactivate");
    }

    if !(!error_demo.borrow().is_active) {
        panic!("Account is already active");
    }

    assign!(error_demo.borrow_mut().is_active, true);

    assign!(
        error_demo.borrow_mut().operation_count,
        error_demo.borrow().operation_count + 1
    );

    solana_program::msg!("{}", "Account reactivated".to_string());
}

pub fn set_operator_handler<'info>(
    mut admin: SeahorseSigner<'info, '_>,
    mut registry: Mutable<LoadedRoleRegistry<'info, '_>>,
    mut new_operator: Pubkey,
) -> () {
    "Set operator role (admin only).".to_string();

    if !(admin.key() == registry.borrow().admin) {
        panic!("Unauthorized: only admin can set operator");
    }

    assign!(registry.borrow_mut().operator, new_operator);

    assign!(registry.borrow_mut().has_operator, true);

    solana_program::msg!("{}", format!("Operator set to {:?}", new_operator));
}

pub fn set_value_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut error_demo: Mutable<LoadedErrorDemo<'info, '_>>,
    mut new_value: u64,
) -> () {
    "Set value with authorization and range validation.".to_string();

    if !(authority.key() == error_demo.borrow().authority) {
        panic!("Unauthorized: only authority can set value");
    }

    if !error_demo.borrow().is_active {
        panic!("Account is not active");
    }

    if !(new_value <= error_demo.borrow().max_value) {
        panic!("Value exceeds maximum allowed");
    }

    assign!(error_demo.borrow_mut().value, new_value);

    assign!(
        error_demo.borrow_mut().operation_count,
        error_demo.borrow().operation_count + 1
    );

    solana_program::msg!("{}", format!("Value set to {}", new_value));
}

pub fn update_name_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut error_demo: Mutable<LoadedErrorDemo<'info, '_>>,
    mut new_name: String,
) -> () {
    "Update name with string validation.".to_string();

    if !(authority.key() == error_demo.borrow().authority) {
        panic!("Unauthorized: only authority can update name");
    }

    if !error_demo.borrow().is_active {
        panic!("Account is not active");
    }

    if !((new_name.chars().count() as u64) > 0) {
        panic!("Name cannot be empty");
    }

    if !((new_name.chars().count() as u64) <= 32) {
        panic!("Name exceeds maximum length of 32 characters");
    }

    solana_program::msg!("{}", format!("Name updated to: {}", new_name));

    assign!(error_demo.borrow_mut().name, new_name);

    assign!(
        error_demo.borrow_mut().operation_count,
        error_demo.borrow().operation_count + 1
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

declare_id!("ErrDemo111111111111111111111111111111111111");

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
mod errors {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    pub struct AdminOnlyAction<'info> {
        #[account(mut)]
        pub caller: Signer<'info>,
        #[account(mut)]
        pub registry: Box<Account<'info, dot::program::RoleRegistry>>,
    }

    pub fn admin_only_action(ctx: Context<AdminOnlyAction>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let caller = SeahorseSigner {
            account: &ctx.accounts.caller,
            programs: &programs_map,
        };

        let registry = dot::program::RoleRegistry::load(&mut ctx.accounts.registry, &programs_map);

        admin_only_action_handler(caller.clone(), registry.clone());

        dot::program::RoleRegistry::store(registry);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (new_value : u64 , new_name : String , require_high_value : bool)]
    pub struct ComplexValidation<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub error_demo: Box<Account<'info, dot::program::ErrorDemo>>,
    }

    pub fn complex_validation(
        ctx: Context<ComplexValidation>,
        new_value: u64,
        new_name: String,
        require_high_value: bool,
    ) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let error_demo = dot::program::ErrorDemo::load(&mut ctx.accounts.error_demo, &programs_map);

        complex_validation_handler(
            authority.clone(),
            error_demo.clone(),
            new_value,
            new_name,
            require_high_value,
        );

        dot::program::ErrorDemo::store(error_demo);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct Deactivate<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub error_demo: Box<Account<'info, dot::program::ErrorDemo>>,
    }

    pub fn deactivate(ctx: Context<Deactivate>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let error_demo = dot::program::ErrorDemo::load(&mut ctx.accounts.error_demo, &programs_map);

        deactivate_handler(authority.clone(), error_demo.clone());

        dot::program::ErrorDemo::store(error_demo);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (amount : u64)]
    pub struct DecrementValue<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub error_demo: Box<Account<'info, dot::program::ErrorDemo>>,
    }

    pub fn decrement_value(ctx: Context<DecrementValue>, amount: u64) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let error_demo = dot::program::ErrorDemo::load(&mut ctx.accounts.error_demo, &programs_map);

        decrement_value_handler(authority.clone(), error_demo.clone(), amount);

        dot::program::ErrorDemo::store(error_demo);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct GetDemoInfo<'info> {
        #[account(mut)]
        pub error_demo: Box<Account<'info, dot::program::ErrorDemo>>,
    }

    pub fn get_demo_info(ctx: Context<GetDemoInfo>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let error_demo = dot::program::ErrorDemo::load(&mut ctx.accounts.error_demo, &programs_map);

        get_demo_info_handler(error_demo.clone());

        dot::program::ErrorDemo::store(error_demo);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (amount : u64)]
    pub struct IncrementValue<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub error_demo: Box<Account<'info, dot::program::ErrorDemo>>,
    }

    pub fn increment_value(ctx: Context<IncrementValue>, amount: u64) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let error_demo = dot::program::ErrorDemo::load(&mut ctx.accounts.error_demo, &programs_map);

        increment_value_handler(authority.clone(), error_demo.clone(), amount);

        dot::program::ErrorDemo::store(error_demo);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (max_value : u64 , name : String)]
    pub struct Initialize<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: ErrorDemo > () + 8 + (100 as usize) , payer = authority , seeds = ["error_demo" . as_bytes () . as_ref () , authority . key () . as_ref ()] , bump)]
        pub error_demo: Box<Account<'info, dot::program::ErrorDemo>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn initialize(ctx: Context<Initialize>, max_value: u64, name: String) -> Result<()> {
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

        let error_demo = Empty {
            account: dot::program::ErrorDemo::load(&mut ctx.accounts.error_demo, &programs_map),
            bump: Some(ctx.bumps.error_demo),
        };

        initialize_handler(authority.clone(), error_demo.clone(), max_value, name);

        dot::program::ErrorDemo::store(error_demo.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (registry_id : u64)]
    pub struct InitializeRoleRegistry<'info> {
        #[account(mut)]
        pub admin: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: RoleRegistry > () + 8 , payer = admin , seeds = ["role_registry" . as_bytes () . as_ref () , registry_id . to_le_bytes () . as_ref ()] , bump)]
        pub registry: Box<Account<'info, dot::program::RoleRegistry>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn initialize_role_registry(
        ctx: Context<InitializeRoleRegistry>,
        registry_id: u64,
    ) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "system_program",
            ctx.accounts.system_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let admin = SeahorseSigner {
            account: &ctx.accounts.admin,
            programs: &programs_map,
        };

        let registry = Empty {
            account: dot::program::RoleRegistry::load(&mut ctx.accounts.registry, &programs_map),
            bump: Some(ctx.bumps.registry),
        };

        initialize_role_registry_handler(admin.clone(), registry.clone(), registry_id);

        dot::program::RoleRegistry::store(registry.account);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct OperatorAction<'info> {
        #[account(mut)]
        pub caller: Signer<'info>,
        #[account(mut)]
        pub registry: Box<Account<'info, dot::program::RoleRegistry>>,
    }

    pub fn operator_action(ctx: Context<OperatorAction>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let caller = SeahorseSigner {
            account: &ctx.accounts.caller,
            programs: &programs_map,
        };

        let registry = dot::program::RoleRegistry::load(&mut ctx.accounts.registry, &programs_map);

        operator_action_handler(caller.clone(), registry.clone());

        dot::program::RoleRegistry::store(registry);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct Reactivate<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub error_demo: Box<Account<'info, dot::program::ErrorDemo>>,
    }

    pub fn reactivate(ctx: Context<Reactivate>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let error_demo = dot::program::ErrorDemo::load(&mut ctx.accounts.error_demo, &programs_map);

        reactivate_handler(authority.clone(), error_demo.clone());

        dot::program::ErrorDemo::store(error_demo);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (new_operator : Pubkey)]
    pub struct SetOperator<'info> {
        #[account(mut)]
        pub admin: Signer<'info>,
        #[account(mut)]
        pub registry: Box<Account<'info, dot::program::RoleRegistry>>,
    }

    pub fn set_operator(ctx: Context<SetOperator>, new_operator: Pubkey) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let admin = SeahorseSigner {
            account: &ctx.accounts.admin,
            programs: &programs_map,
        };

        let registry = dot::program::RoleRegistry::load(&mut ctx.accounts.registry, &programs_map);

        set_operator_handler(admin.clone(), registry.clone(), new_operator);

        dot::program::RoleRegistry::store(registry);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (new_value : u64)]
    pub struct SetValue<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub error_demo: Box<Account<'info, dot::program::ErrorDemo>>,
    }

    pub fn set_value(ctx: Context<SetValue>, new_value: u64) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let error_demo = dot::program::ErrorDemo::load(&mut ctx.accounts.error_demo, &programs_map);

        set_value_handler(authority.clone(), error_demo.clone(), new_value);

        dot::program::ErrorDemo::store(error_demo);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (new_name : String)]
    pub struct UpdateName<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub error_demo: Box<Account<'info, dot::program::ErrorDemo>>,
    }

    pub fn update_name(ctx: Context<UpdateName>, new_name: String) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let error_demo = dot::program::ErrorDemo::load(&mut ctx.accounts.error_demo, &programs_map);

        update_name_handler(authority.clone(), error_demo.clone(), new_name);

        dot::program::ErrorDemo::store(error_demo);

        return Ok(());
    }
}

