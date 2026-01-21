   Compiling seahorse-dev v0.1.1 (/Users/z80/dev/seahorse)
warning: unnecessary parentheses around match arm expression
   --> src/core/clean/mod.rs:374:51
    |
374 |            py::ExpressionType::Subscript { a, b } => (
    |  ____________________________________________________^
375 | |              validate_constant(&*a) && validate_constant(&*b)
    | | ____________^________________________________________________^
    | ||____________|
    |  |
376 |  |         ),
    |  |_________^
    |
    = note: `#[warn(unused_parens)]` (part of `#[warn(unused)]`) on by default
help: remove these parentheses
    |
374 -         py::ExpressionType::Subscript { a, b } => (
375 -             validate_constant(&*a) && validate_constant(&*b)
374 +         py::ExpressionType::Subscript { a, b } => validate_constant(&*a) && validate_constant(&*b) ,
    |

warning: unnecessary parentheses around match arm expression
   --> src/core/clean/mod.rs:377:51
    |
377 |            py::ExpressionType::Binop { a, b, .. } => (
    |  ____________________________________________________^
378 | |              validate_constant(&*a) && validate_constant(&*b)
    | | ____________^________________________________________________^
    | ||____________|
    |  |
379 |  |         ),
    |  |_________^
    |
help: remove these parentheses
    |
377 -         py::ExpressionType::Binop { a, b, .. } => (
378 -             validate_constant(&*a) && validate_constant(&*b)
377 +         py::ExpressionType::Binop { a, b, .. } => validate_constant(&*a) && validate_constant(&*b) ,
    |

warning: unused imports: `Python` and `pyth::ExprContext`
  --> src/core/compile/sign/mod.rs:16:15
   |
16 |     builtin::{pyth::ExprContext, Builtin, BuiltinSource, Python},
   |               ^^^^^^^^^^^^^^^^^                          ^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: unnecessary parentheses around `if` condition
    --> src/core/generate/mod.rs:1479:32
     |
1479 |     let maybe_pyth_import = if (features.contains(&Feature::Pyth)) {
     |                                ^                                 ^
     |
help: remove these parentheses
     |
1479 -     let maybe_pyth_import = if (features.contains(&Feature::Pyth)) {
1479 +     let maybe_pyth_import = if features.contains(&Feature::Pyth)  {
     |

warning: unreachable pattern
    --> src/core/compile/build/mod.rs:1267:25
     |
1267 |                         _ => {}
     |                         ^ no value can reach this
     |
note: multiple earlier patterns match some of the same values
    --> src/core/compile/build/mod.rs:1267:25
     |
1114 |                         NamespacedObject::Automatic(..) => {},
     |                         ------------------------------- matches some of the same values
1115 |                         NamespacedObject::Import(Located(_, ImportObj { path, is_builtin: true, .. })) => {
     |                         ------------------------------------------------------------------------------ matches some of the same values
...
1122 |                         NamespacedObject::Import(Located(_, ImportObj { mut path, is_builtin: false, .. })) => {
     |                         ----------------------------------------------------------------------------------- matches some of the same values
...
1155 |                         NamespacedObject::Item(item) => {
     |                         ---------------------------- matches some of the same values
...
1267 |                         _ => {}
     |                         ^ collectively making this unreachable
     = note: `#[warn(unreachable_patterns)]` (part of `#[warn(unused)]`) on by default

warning: unused variable: `abs`
    --> src/core/compile/build/mod.rs:1102:74
     |
1102 |             .map_with_path(|(mut contexts, (mut signatures, namespace)), abs| {
     |                                                                          ^^^ help: if this is intentional, prefix it with an underscore: `_abs`
     |
     = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default

warning: unused variable: `attr`
   --> src/core/compile/builtin/python.rs:662:27
    |
662 |     fn static_attr(&self, attr: &String) -> Option<Ty> {
    |                           ^^^^ help: if this is intentional, prefix it with an underscore: `_attr`

warning: unused variable: `path`
   --> src/core/compile/check/mod.rs:878:48
    |
878 |                         Located(_, ImportObj { path, import_type: ImportType::Symbol, .. }) => {
    |                                                ^^^^ help: try ignoring the field: `path: _`

warning: unused variable: `path`
   --> src/core/compile/check/mod.rs:883:48
    |
883 |                         Located(_, ImportObj { path, .. }) => {
    |                                                ^^^^ help: try ignoring the field: `path: _`

warning: unused variable: `target`
    --> src/core/compile/check/mod.rs:1153:27
     |
1153 |                     (Some(target), None) => {
     |                           ^^^^^^ help: if this is intentional, prefix it with an underscore: `_target`

warning: unused variable: `returns`
   --> src/core/compile/sign/mod.rs:131:46
    |
131 |             |(_, FunctionSignature { params, returns })| {
    |                                              ^^^^^^^ help: try ignoring the field: `returns: _`

warning: unused variable: `ty`
    --> src/core/generate/mod.rs:1360:54
     |
1360 |                 |(name, ContextAccount { account_ty, ty, .. })| {
     |                                                      ^^ help: try ignoring the field: `ty: _`

warning: variant `ReusedVarInTuple` is never constructed
  --> src/core/compile/check/mod.rs:26:5
   |
24 | enum Error {
   |      ----- variant in this enum
25 |     VarNotFound(String),
26 |     ReusedVarInTuple,
   |     ^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: method `get_undeclared` is never used
   --> src/core/compile/check/mod.rs:702:8
    |
550 | impl<'a> Context<'a> {
    | -------------------- method in this implementation
...
702 |     fn get_undeclared(&self, target: &Target, loc: &Location) -> CResult<Vec<String>> {
    |        ^^^^^^^^^^^^^^

warning: variant `CouldNotAddModule` is never constructed
  --> src/core/preprocess/mod.rs:20:5
   |
19 | enum Error {
   |      ----- variant in this enum
20 |     CouldNotAddModule,
   |     ^^^^^^^^^^^^^^^^^

warning: `seahorse-dev` (lib) generated 15 warnings (run `cargo fix --lib -p seahorse-dev` to apply 11 suggestions)
warning: unused import: `generate::GenerateOutput`
 --> src/bin/cli/compile.rs:3:41
  |
3 |     core::{compile as seahorse_compile, generate::GenerateOutput, Tree},
  |                                         ^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: unused import: `Table`
  --> src/bin/cli/init.rs:16:57
   |
16 | use toml_edit::{Document, Formatted, InlineTable, Item, Table, Value};
   |                                                         ^^^^^

warning: unused import: `BufWriter`
 --> src/bin/cli/update.rs:9:10
  |
9 |     io::{BufWriter, Write},
  |          ^^^^^^^^^

warning: unused variable: `args`
  --> src/bin/cli/update.rs:28:15
   |
28 | pub fn update(args: UpdateArgs) -> Result<(), Box<dyn Error>> {
   |               ^^^^ help: if this is intentional, prefix it with an underscore: `_args`
   |
   = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default

warning: `seahorse-dev` (bin "seahorse") generated 4 warnings (run `cargo fix --bin "seahorse" -p seahorse-dev` to apply 4 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.08s
warning: the following packages contain code that will be rejected by a future version of Rust: lalrpop v0.17.2
note: to see what the problems were, use the option `--future-incompat-report`, or run `cargo report future-incompatibilities --id 1`
     Running `target/debug/seahorse compile examples/events.py`
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
pub struct EventCounter {
    pub authority: Pubkey,
    pub event_count: u64,
    pub last_event_type: u8,
    pub bump: u8,
}

impl<'info, 'entrypoint> EventCounter {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedEventCounter<'info, 'entrypoint>> {
        let authority = account.authority.clone();
        let event_count = account.event_count;
        let last_event_type = account.last_event_type;
        let bump = account.bump;

        Mutable::new(LoadedEventCounter {
            __account__: account,
            __programs__: programs_map,
            authority,
            event_count,
            last_event_type,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedEventCounter>) {
        let mut loaded = loaded.borrow_mut();
        let authority = loaded.authority.clone();

        loaded.__account__.authority = authority;

        let event_count = loaded.event_count;

        loaded.__account__.event_count = event_count;

        let last_event_type = loaded.last_event_type;

        loaded.__account__.last_event_type = last_event_type;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedEventCounter<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, EventCounter>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub authority: Pubkey,
    pub event_count: u64,
    pub last_event_type: u8,
    pub bump: u8,
}

#[event]
pub struct NumericEvent {
    pub unsigned_small: u8,
    pub unsigned_medium: u32,
    pub unsigned_large: u64,
    pub signed_value: i64,
}

#[derive(Clone, Debug, Default)]
pub struct LoadedNumericEvent {
    pub unsigned_small: u8,
    pub unsigned_medium: u32,
    pub unsigned_large: u64,
    pub signed_value: i64,
}

impl Mutable<LoadedNumericEvent> {
    fn __emit__(&self) {
        let e = self.borrow();

        emit!(NumericEvent {
            unsigned_small: e.unsigned_small,
            unsigned_medium: e.unsigned_medium,
            unsigned_large: e.unsigned_large,
            signed_value: e.signed_value
        })
    }
}

impl LoadedNumericEvent {
    pub fn __new__(
        unsigned_small: u8,
        unsigned_medium: u32,
        unsigned_large: u64,
        signed_value: i64,
    ) -> Mutable<Self> {
        let obj = LoadedNumericEvent {
            unsigned_small,
            unsigned_medium,
            unsigned_large,
            signed_value,
        };

        return Mutable::new(obj);
    }
}

impl Loadable for NumericEvent {
    type Loaded = LoadedNumericEvent;

    fn load(stored: Self) -> Self::Loaded {
        Self::Loaded {
            unsigned_small: stored.unsigned_small,
            unsigned_medium: stored.unsigned_medium,
            unsigned_large: stored.unsigned_large,
            signed_value: stored.signed_value,
        }
    }

    fn store(loaded: Self::Loaded) -> Self {
        Self {
            unsigned_small: loaded.unsigned_small,
            unsigned_medium: loaded.unsigned_medium,
            unsigned_large: loaded.unsigned_large,
            signed_value: loaded.signed_value,
        }
    }
}

#[event]
pub struct SimpleEvent {
    pub value: u64,
    pub label: String,
}

#[derive(Clone, Debug, Default)]
pub struct LoadedSimpleEvent {
    pub value: u64,
    pub label: String,
}

impl Mutable<LoadedSimpleEvent> {
    fn __emit__(&self) {
        let e = self.borrow();

        emit!(SimpleEvent {
            value: e.value,
            label: e.label.clone()
        })
    }
}

impl LoadedSimpleEvent {
    pub fn __new__(value: u64, label: String) -> Mutable<Self> {
        let obj = LoadedSimpleEvent { value, label };

        return Mutable::new(obj);
    }
}

impl Loadable for SimpleEvent {
    type Loaded = LoadedSimpleEvent;

    fn load(stored: Self) -> Self::Loaded {
        Self::Loaded {
            value: stored.value,
            label: stored.label,
        }
    }

    fn store(loaded: Self::Loaded) -> Self {
        Self {
            value: loaded.value,
            label: loaded.label.clone(),
        }
    }
}

#[event]
pub struct StateChangeEvent {
    pub account: Pubkey,
    pub old_value: u64,
    pub new_value: u64,
    pub change_type: String,
}

#[derive(Clone, Debug, Default)]
pub struct LoadedStateChangeEvent {
    pub account: Pubkey,
    pub old_value: u64,
    pub new_value: u64,
    pub change_type: String,
}

impl Mutable<LoadedStateChangeEvent> {
    fn __emit__(&self) {
        let e = self.borrow();

        emit!(StateChangeEvent {
            account: e.account.clone(),
            old_value: e.old_value,
            new_value: e.new_value,
            change_type: e.change_type.clone()
        })
    }
}

impl LoadedStateChangeEvent {
    pub fn __new__(
        account: Pubkey,
        old_value: u64,
        new_value: u64,
        change_type: String,
    ) -> Mutable<Self> {
        let obj = LoadedStateChangeEvent {
            account,
            old_value,
            new_value,
            change_type,
        };

        return Mutable::new(obj);
    }
}

impl Loadable for StateChangeEvent {
    type Loaded = LoadedStateChangeEvent;

    fn load(stored: Self) -> Self::Loaded {
        Self::Loaded {
            account: stored.account,
            old_value: stored.old_value,
            new_value: stored.new_value,
            change_type: stored.change_type,
        }
    }

    fn store(loaded: Self::Loaded) -> Self {
        Self {
            account: loaded.account.clone(),
            old_value: loaded.old_value,
            new_value: loaded.new_value,
            change_type: loaded.change_type.clone(),
        }
    }
}

#[event]
pub struct TransferEvent {
    pub from_addr: Pubkey,
    pub to_addr: Pubkey,
    pub amount: u64,
    pub memo: String,
}

#[derive(Clone, Debug, Default)]
pub struct LoadedTransferEvent {
    pub from_addr: Pubkey,
    pub to_addr: Pubkey,
    pub amount: u64,
    pub memo: String,
}

impl Mutable<LoadedTransferEvent> {
    fn __emit__(&self) {
        let e = self.borrow();

        emit!(TransferEvent {
            from_addr: e.from_addr.clone(),
            to_addr: e.to_addr.clone(),
            amount: e.amount,
            memo: e.memo.clone()
        })
    }
}

impl LoadedTransferEvent {
    pub fn __new__(from_addr: Pubkey, to_addr: Pubkey, amount: u64, memo: String) -> Mutable<Self> {
        let obj = LoadedTransferEvent {
            from_addr,
            to_addr,
            amount,
            memo,
        };

        return Mutable::new(obj);
    }
}

impl Loadable for TransferEvent {
    type Loaded = LoadedTransferEvent;

    fn load(stored: Self) -> Self::Loaded {
        Self::Loaded {
            from_addr: stored.from_addr,
            to_addr: stored.to_addr,
            amount: stored.amount,
            memo: stored.memo,
        }
    }

    fn store(loaded: Self::Loaded) -> Self {
        Self {
            from_addr: loaded.from_addr.clone(),
            to_addr: loaded.to_addr.clone(),
            amount: loaded.amount,
            memo: loaded.memo.clone(),
        }
    }
}

#[event]
pub struct UserActionEvent {
    pub user: Pubkey,
    pub action_type: u8,
    pub timestamp: i64,
}

#[derive(Clone, Debug, Default)]
pub struct LoadedUserActionEvent {
    pub user: Pubkey,
    pub action_type: u8,
    pub timestamp: i64,
}

impl Mutable<LoadedUserActionEvent> {
    fn __emit__(&self) {
        let e = self.borrow();

        emit!(UserActionEvent {
            user: e.user.clone(),
            action_type: e.action_type,
            timestamp: e.timestamp
        })
    }
}

impl LoadedUserActionEvent {
    pub fn __new__(user: Pubkey, action_type: u8, timestamp: i64) -> Mutable<Self> {
        let obj = LoadedUserActionEvent {
            user,
            action_type,
            timestamp,
        };

        return Mutable::new(obj);
    }
}

impl Loadable for UserActionEvent {
    type Loaded = LoadedUserActionEvent;

    fn load(stored: Self) -> Self::Loaded {
        Self::Loaded {
            user: stored.user,
            action_type: stored.action_type,
            timestamp: stored.timestamp,
        }
    }

    fn store(loaded: Self::Loaded) -> Self {
        Self {
            user: loaded.user.clone(),
            action_type: loaded.action_type,
            timestamp: loaded.timestamp,
        }
    }
}

pub fn emit_multiple_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut counter: Mutable<LoadedEventCounter<'info, '_>>,
    mut count: u8,
) -> () {
    "\n    Emit multiple events in a single transaction.\n    Demonstrates batch event emission pattern.\n\n    Args:\n        authority: Account authority\n        counter: The event counter account\n        count: Number of events to emit (max 10 for compute limits)\n    " . to_string () ;

    if !(counter.borrow().authority == authority.key()) {
        panic!("Unauthorized");
    }

    if !(count <= 10) {
        panic!("Maximum 10 events per transaction");
    }

    let mut i = 0;

    while i < count {
        let mut event = <Loaded!(SimpleEvent)>::__new__(
            <u64 as TryFrom<_>>::try_from(i.clone()).unwrap(),
            "batch".to_string(),
        );

        event.__emit__();

        assign!(
            counter.borrow_mut().event_count,
            counter.borrow().event_count + 1
        );

        i = i + 1;
    }

    assign!(counter.borrow_mut().last_event_type, 6);

    solana_program::msg!("{}", format!("Emitted {} SimpleEvents", count));
}

pub fn emit_numeric_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut counter: Mutable<LoadedEventCounter<'info, '_>>,
    mut unsigned_small: u8,
    mut unsigned_medium: u32,
    mut unsigned_large: u64,
    mut signed_value: i64,
) -> () {
    "\n    Emit an event with various numeric types.\n\n    Args:\n        authority: Account authority\n        counter: The event counter account\n        unsigned_small: u8 value\n        unsigned_medium: u32 value\n        unsigned_large: u64 value\n        signed_value: i64 value\n    " . to_string () ;

    if !(counter.borrow().authority == authority.key()) {
        panic!("Unauthorized");
    }

    assign!(
        counter.borrow_mut().event_count,
        counter.borrow().event_count + 1
    );

    assign!(counter.borrow_mut().last_event_type, 3);

    let mut event = <Loaded!(NumericEvent)>::__new__(
        unsigned_small.clone(),
        unsigned_medium.clone(),
        unsigned_large.clone(),
        signed_value.clone(),
    );

    event.__emit__();

    solana_program::msg!("{}", "Emitted NumericEvent".to_string());
}

pub fn emit_simple_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut counter: Mutable<LoadedEventCounter<'info, '_>>,
    mut value: u64,
    mut label: String,
) -> () {
    "\n    Emit a simple event with basic data types.\n\n    Args:\n        authority: Account authority (must match counter.authority)\n        counter: The event counter account\n        value: Numeric value to include in event\n        label: String label to include in event\n    " . to_string () ;

    if !(counter.borrow().authority == authority.key()) {
        panic!("Unauthorized");
    }

    assign!(
        counter.borrow_mut().event_count,
        counter.borrow().event_count + 1
    );

    assign!(counter.borrow_mut().last_event_type, 1);

    let mut event = <Loaded!(SimpleEvent)>::__new__(value.clone(), label.clone());

    event.__emit__();

    solana_program::msg!("{}", "Emitted SimpleEvent".to_string());
}

pub fn emit_state_change_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut counter: Mutable<LoadedEventCounter<'info, '_>>,
    mut old_value: u64,
    mut new_value: u64,
    mut change_type: String,
) -> () {
    "\n    Emit a state change event (tracks before/after values).\n\n    Args:\n        authority: Account authority\n        counter: The event counter account\n        old_value: Previous value\n        new_value: New value after change\n        change_type: Type of change (e.g., \"increment\", \"set\", \"reset\")\n    " . to_string () ;

    if !(counter.borrow().authority == authority.key()) {
        panic!("Unauthorized");
    }

    assign!(
        counter.borrow_mut().event_count,
        counter.borrow().event_count + 1
    );

    assign!(counter.borrow_mut().last_event_type, 5);

    let mut event = <Loaded!(StateChangeEvent)>::__new__(
        counter.borrow().__account__.key(),
        old_value.clone(),
        new_value.clone(),
        change_type.clone(),
    );

    event.__emit__();

    solana_program::msg!("{}", "Emitted StateChangeEvent".to_string());
}

pub fn emit_transfer_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut counter: Mutable<LoadedEventCounter<'info, '_>>,
    mut to_addr: Pubkey,
    mut amount: u64,
    mut memo: String,
) -> () {
    "\n    Emit a transfer event (common pattern in token programs).\n\n    Args:\n        authority: Account authority (sender)\n        counter: The event counter account\n        to_addr: Recipient address\n        amount: Transfer amount\n        memo: Transfer memo/description\n    " . to_string () ;

    if !(counter.borrow().authority == authority.key()) {
        panic!("Unauthorized");
    }

    assign!(
        counter.borrow_mut().event_count,
        counter.borrow().event_count + 1
    );

    assign!(counter.borrow_mut().last_event_type, 4);

    let mut event = <Loaded!(TransferEvent)>::__new__(
        authority.key(),
        to_addr.clone(),
        amount.clone(),
        memo.clone(),
    );

    event.__emit__();

    solana_program::msg!("{}", "Emitted TransferEvent".to_string());
}

pub fn emit_user_action_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut counter: Mutable<LoadedEventCounter<'info, '_>>,
    mut action_type: u8,
    mut timestamp: i64,
) -> () {
    "\n    Emit a user action event (tracks who did what and when).\n\n    Args:\n        authority: Account authority (user performing action)\n        counter: The event counter account\n        action_type: Type of action being performed\n        timestamp: Unix timestamp of the action\n    " . to_string () ;

    if !(counter.borrow().authority == authority.key()) {
        panic!("Unauthorized");
    }

    assign!(
        counter.borrow_mut().event_count,
        counter.borrow().event_count + 1
    );

    assign!(counter.borrow_mut().last_event_type, 2);

    let mut event = <Loaded!(UserActionEvent)>::__new__(
        authority.key(),
        action_type.clone(),
        timestamp.clone(),
    );

    event.__emit__();

    solana_program::msg!("{}", "Emitted UserActionEvent".to_string());
}

pub fn get_counter_info_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut counter: Mutable<LoadedEventCounter<'info, '_>>,
) -> () {
    "\n    Get event counter info (read-only, for logging/debugging).\n\n    Args:\n        authority: Signer for instruction\n        counter: The event counter account\n    " . to_string () ;

    solana_program::msg!(
        "{}",
        format!("Event count: {}", counter.borrow().event_count)
    );

    solana_program::msg!(
        "{}",
        format!("Last event type: {}", counter.borrow().last_event_type)
    );

    solana_program::msg!("{}", format!("Authority: {:?}", counter.borrow().authority));
}

pub fn initialize_handler<'info>(
    mut authority: SeahorseSigner<'info, '_>,
    mut counter: Empty<Mutable<LoadedEventCounter<'info, '_>>>,
) -> () {
    "\n    Initialize the event counter account.\n\n    Args:\n        authority: Account authority who will emit events\n        counter: The counter account to initialize\n    " . to_string () ;

    let mut bump = counter.bump.unwrap();
    let mut counter = counter.account.clone();

    assign!(counter.borrow_mut().authority, authority.key());

    assign!(counter.borrow_mut().event_count, 0);

    assign!(counter.borrow_mut().last_event_type, 0);

    assign!(counter.borrow_mut().bump, bump);

    solana_program::msg!("{}", "Initialized event counter".to_string());
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

declare_id!("YqBL3cHjsojPxJuyLF6bcQSYJ59p9X5qePjhWkXCEGR");

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
mod events {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    # [instruction (count : u8)]
    pub struct EmitMultiple<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub counter: Box<Account<'info, dot::program::EventCounter>>,
    }

    pub fn emit_multiple(ctx: Context<EmitMultiple>, count: u8) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let counter = dot::program::EventCounter::load(&mut ctx.accounts.counter, &programs_map);

        emit_multiple_handler(authority.clone(), counter.clone(), count);

        dot::program::EventCounter::store(counter);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (unsigned_small : u8 , unsigned_medium : u32 , unsigned_large : u64 , signed_value : i64)]
    pub struct EmitNumeric<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub counter: Box<Account<'info, dot::program::EventCounter>>,
    }

    pub fn emit_numeric(
        ctx: Context<EmitNumeric>,
        unsigned_small: u8,
        unsigned_medium: u32,
        unsigned_large: u64,
        signed_value: i64,
    ) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let counter = dot::program::EventCounter::load(&mut ctx.accounts.counter, &programs_map);

        emit_numeric_handler(
            authority.clone(),
            counter.clone(),
            unsigned_small,
            unsigned_medium,
            unsigned_large,
            signed_value,
        );

        dot::program::EventCounter::store(counter);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (value : u64 , label : String)]
    pub struct EmitSimple<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub counter: Box<Account<'info, dot::program::EventCounter>>,
    }

    pub fn emit_simple(ctx: Context<EmitSimple>, value: u64, label: String) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let counter = dot::program::EventCounter::load(&mut ctx.accounts.counter, &programs_map);

        emit_simple_handler(authority.clone(), counter.clone(), value, label);

        dot::program::EventCounter::store(counter);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (old_value : u64 , new_value : u64 , change_type : String)]
    pub struct EmitStateChange<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub counter: Box<Account<'info, dot::program::EventCounter>>,
    }

    pub fn emit_state_change(
        ctx: Context<EmitStateChange>,
        old_value: u64,
        new_value: u64,
        change_type: String,
    ) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let counter = dot::program::EventCounter::load(&mut ctx.accounts.counter, &programs_map);

        emit_state_change_handler(
            authority.clone(),
            counter.clone(),
            old_value,
            new_value,
            change_type,
        );

        dot::program::EventCounter::store(counter);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (to_addr : Pubkey , amount : u64 , memo : String)]
    pub struct EmitTransfer<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub counter: Box<Account<'info, dot::program::EventCounter>>,
    }

    pub fn emit_transfer(
        ctx: Context<EmitTransfer>,
        to_addr: Pubkey,
        amount: u64,
        memo: String,
    ) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let counter = dot::program::EventCounter::load(&mut ctx.accounts.counter, &programs_map);

        emit_transfer_handler(authority.clone(), counter.clone(), to_addr, amount, memo);

        dot::program::EventCounter::store(counter);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (action_type : u8 , timestamp : i64)]
    pub struct EmitUserAction<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub counter: Box<Account<'info, dot::program::EventCounter>>,
    }

    pub fn emit_user_action(
        ctx: Context<EmitUserAction>,
        action_type: u8,
        timestamp: i64,
    ) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let counter = dot::program::EventCounter::load(&mut ctx.accounts.counter, &programs_map);

        emit_user_action_handler(authority.clone(), counter.clone(), action_type, timestamp);

        dot::program::EventCounter::store(counter);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct GetCounterInfo<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut)]
        pub counter: Box<Account<'info, dot::program::EventCounter>>,
    }

    pub fn get_counter_info(ctx: Context<GetCounterInfo>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let authority = SeahorseSigner {
            account: &ctx.accounts.authority,
            programs: &programs_map,
        };

        let counter = dot::program::EventCounter::load(&mut ctx.accounts.counter, &programs_map);

        get_counter_info_handler(authority.clone(), counter.clone());

        dot::program::EventCounter::store(counter);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct Initialize<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: EventCounter > () + 8 , payer = authority , seeds = ["counter" . as_bytes () . as_ref () , authority . key () . as_ref ()] , bump)]
        pub counter: Box<Account<'info, dot::program::EventCounter>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
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

        let counter = Empty {
            account: dot::program::EventCounter::load(&mut ctx.accounts.counter, &programs_map),
            bump: Some(ctx.bumps.counter),
        };

        initialize_handler(authority.clone(), counter.clone());

        dot::program::EventCounter::store(counter.account);

        return Ok(());
    }
}

