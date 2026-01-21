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

seahorse_const! { MAX_OWNERS , 10 }

#[account]
#[derive(Debug)]
pub struct Approval {
    pub approver: Pubkey,
    pub transaction: Pubkey,
    pub approved_at_slot: u64,
    pub bump: u8,
}

impl<'info, 'entrypoint> Approval {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedApproval<'info, 'entrypoint>> {
        let approver = account.approver.clone();
        let transaction = account.transaction.clone();
        let approved_at_slot = account.approved_at_slot;
        let bump = account.bump;

        Mutable::new(LoadedApproval {
            __account__: account,
            __programs__: programs_map,
            approver,
            transaction,
            approved_at_slot,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedApproval>) {
        let mut loaded = loaded.borrow_mut();
        let approver = loaded.approver.clone();

        loaded.__account__.approver = approver;

        let transaction = loaded.transaction.clone();

        loaded.__account__.transaction = transaction;

        let approved_at_slot = loaded.approved_at_slot;

        loaded.__account__.approved_at_slot = approved_at_slot;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedApproval<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, Approval>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub approver: Pubkey,
    pub transaction: Pubkey,
    pub approved_at_slot: u64,
    pub bump: u8,
}

#[account]
#[derive(Debug)]
pub struct Multisig {
    pub multisig_id: u64,
    pub threshold: u8,
    pub owner_count: u8,
    pub owner_1: Pubkey,
    pub owner_2: Pubkey,
    pub owner_3: Pubkey,
    pub owner_4: Pubkey,
    pub owner_5: Pubkey,
    pub owner_6: Pubkey,
    pub owner_7: Pubkey,
    pub owner_8: Pubkey,
    pub owner_9: Pubkey,
    pub owner_10: Pubkey,
    pub next_tx_id: u64,
    pub bump: u8,
}

impl<'info, 'entrypoint> Multisig {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedMultisig<'info, 'entrypoint>> {
        let multisig_id = account.multisig_id;
        let threshold = account.threshold;
        let owner_count = account.owner_count;
        let owner_1 = account.owner_1.clone();
        let owner_2 = account.owner_2.clone();
        let owner_3 = account.owner_3.clone();
        let owner_4 = account.owner_4.clone();
        let owner_5 = account.owner_5.clone();
        let owner_6 = account.owner_6.clone();
        let owner_7 = account.owner_7.clone();
        let owner_8 = account.owner_8.clone();
        let owner_9 = account.owner_9.clone();
        let owner_10 = account.owner_10.clone();
        let next_tx_id = account.next_tx_id;
        let bump = account.bump;

        Mutable::new(LoadedMultisig {
            __account__: account,
            __programs__: programs_map,
            multisig_id,
            threshold,
            owner_count,
            owner_1,
            owner_2,
            owner_3,
            owner_4,
            owner_5,
            owner_6,
            owner_7,
            owner_8,
            owner_9,
            owner_10,
            next_tx_id,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedMultisig>) {
        let mut loaded = loaded.borrow_mut();
        let multisig_id = loaded.multisig_id;

        loaded.__account__.multisig_id = multisig_id;

        let threshold = loaded.threshold;

        loaded.__account__.threshold = threshold;

        let owner_count = loaded.owner_count;

        loaded.__account__.owner_count = owner_count;

        let owner_1 = loaded.owner_1.clone();

        loaded.__account__.owner_1 = owner_1;

        let owner_2 = loaded.owner_2.clone();

        loaded.__account__.owner_2 = owner_2;

        let owner_3 = loaded.owner_3.clone();

        loaded.__account__.owner_3 = owner_3;

        let owner_4 = loaded.owner_4.clone();

        loaded.__account__.owner_4 = owner_4;

        let owner_5 = loaded.owner_5.clone();

        loaded.__account__.owner_5 = owner_5;

        let owner_6 = loaded.owner_6.clone();

        loaded.__account__.owner_6 = owner_6;

        let owner_7 = loaded.owner_7.clone();

        loaded.__account__.owner_7 = owner_7;

        let owner_8 = loaded.owner_8.clone();

        loaded.__account__.owner_8 = owner_8;

        let owner_9 = loaded.owner_9.clone();

        loaded.__account__.owner_9 = owner_9;

        let owner_10 = loaded.owner_10.clone();

        loaded.__account__.owner_10 = owner_10;

        let next_tx_id = loaded.next_tx_id;

        loaded.__account__.next_tx_id = next_tx_id;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedMultisig<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, Multisig>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub multisig_id: u64,
    pub threshold: u8,
    pub owner_count: u8,
    pub owner_1: Pubkey,
    pub owner_2: Pubkey,
    pub owner_3: Pubkey,
    pub owner_4: Pubkey,
    pub owner_5: Pubkey,
    pub owner_6: Pubkey,
    pub owner_7: Pubkey,
    pub owner_8: Pubkey,
    pub owner_9: Pubkey,
    pub owner_10: Pubkey,
    pub next_tx_id: u64,
    pub bump: u8,
}

#[account]
#[derive(Debug)]
pub struct Transaction {
    pub multisig: Pubkey,
    pub tx_id: u64,
    pub proposer: Pubkey,
    pub recipient: Pubkey,
    pub amount: u64,
    pub approval_count: u8,
    pub is_executed: bool,
    pub bump: u8,
}

impl<'info, 'entrypoint> Transaction {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedTransaction<'info, 'entrypoint>> {
        let multisig = account.multisig.clone();
        let tx_id = account.tx_id;
        let proposer = account.proposer.clone();
        let recipient = account.recipient.clone();
        let amount = account.amount;
        let approval_count = account.approval_count;
        let is_executed = account.is_executed.clone();
        let bump = account.bump;

        Mutable::new(LoadedTransaction {
            __account__: account,
            __programs__: programs_map,
            multisig,
            tx_id,
            proposer,
            recipient,
            amount,
            approval_count,
            is_executed,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedTransaction>) {
        let mut loaded = loaded.borrow_mut();
        let multisig = loaded.multisig.clone();

        loaded.__account__.multisig = multisig;

        let tx_id = loaded.tx_id;

        loaded.__account__.tx_id = tx_id;

        let proposer = loaded.proposer.clone();

        loaded.__account__.proposer = proposer;

        let recipient = loaded.recipient.clone();

        loaded.__account__.recipient = recipient;

        let amount = loaded.amount;

        loaded.__account__.amount = amount;

        let approval_count = loaded.approval_count;

        loaded.__account__.approval_count = approval_count;

        let is_executed = loaded.is_executed.clone();

        loaded.__account__.is_executed = is_executed;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedTransaction<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, Transaction>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub multisig: Pubkey,
    pub tx_id: u64,
    pub proposer: Pubkey,
    pub recipient: Pubkey,
    pub amount: u64,
    pub approval_count: u8,
    pub is_executed: bool,
    pub bump: u8,
}

pub fn approve_handler<'info>(
    mut approver: SeahorseSigner<'info, '_>,
    mut multisig: Mutable<LoadedMultisig<'info, '_>>,
    mut transaction: Mutable<LoadedTransaction<'info, '_>>,
    mut approval: Empty<Mutable<LoadedApproval<'info, '_>>>,
    mut clock: Sysvar<'info, Clock>,
    mut tx_id: u64,
) -> () {
    if !is_owner(multisig.clone(), approver.key()) {
        panic!("Only owners can approve");
    }

    if !(transaction.borrow().multisig == multisig.borrow().__account__.key()) {
        panic!("Transaction does not belong to this multisig");
    }

    if !(transaction.borrow().tx_id == tx_id) {
        panic!("Transaction ID mismatch");
    }

    if !(!transaction.borrow().is_executed) {
        panic!("Transaction already executed");
    }

    let mut approval_bump = approval.bump.unwrap();
    let mut current_slot = clock.slot;
    let mut approval = approval.account.clone();

    assign!(approval.borrow_mut().approver, approver.key());

    assign!(
        approval.borrow_mut().transaction,
        transaction.borrow().__account__.key()
    );

    assign!(approval.borrow_mut().approved_at_slot, current_slot);

    assign!(approval.borrow_mut().bump, approval_bump);

    assign!(
        transaction.borrow_mut().approval_count,
        transaction.borrow().approval_count + 1
    );

    solana_program::msg!(
        "{}",
        format!(
            "Transaction {} approved by {:?}, count={}/{}",
            tx_id,
            approver.key(),
            transaction.borrow().approval_count,
            multisig.borrow().threshold
        )
    );
}

pub fn create_multisig_handler<'info>(
    mut creator: SeahorseSigner<'info, '_>,
    mut multisig: Empty<Mutable<LoadedMultisig<'info, '_>>>,
    mut multisig_id: u64,
    mut threshold: u8,
    mut owner_count: u8,
    mut owner_1: Pubkey,
    mut owner_2: Pubkey,
    mut owner_3: Pubkey,
) -> () {
    if !(threshold > 0) {
        panic!("Threshold must be greater than zero");
    }

    if !(owner_count >= threshold) {
        panic!("Owner count must be at least threshold");
    }

    if !(owner_count <= MAX_OWNERS!()) {
        panic!("Too many owners");
    }

    if !(owner_count >= 1) {
        panic!("Must have at least one owner");
    }

    if !(owner_count <= 3) {
        panic!("This instruction supports up to 3 owners");
    }

    let mut multisig_bump = multisig.bump.unwrap();
    let mut multisig = multisig.account.clone();

    assign!(multisig.borrow_mut().multisig_id, multisig_id);

    assign!(multisig.borrow_mut().threshold, threshold);

    assign!(multisig.borrow_mut().owner_count, owner_count);

    assign!(multisig.borrow_mut().next_tx_id, 1);

    assign!(multisig.borrow_mut().owner_1, owner_1);

    assign!(multisig.borrow_mut().owner_2, owner_2);

    assign!(multisig.borrow_mut().owner_3, owner_3);

    assign!(multisig.borrow_mut().owner_4, creator.key());

    assign!(multisig.borrow_mut().owner_5, creator.key());

    assign!(multisig.borrow_mut().owner_6, creator.key());

    assign!(multisig.borrow_mut().owner_7, creator.key());

    assign!(multisig.borrow_mut().owner_8, creator.key());

    assign!(multisig.borrow_mut().owner_9, creator.key());

    assign!(multisig.borrow_mut().owner_10, creator.key());

    assign!(multisig.borrow_mut().bump, multisig_bump);

    solana_program::msg!(
        "{}",
        format!(
            "Multisig created: id={}, threshold={}, owners={}",
            multisig_id, threshold, owner_count
        )
    );
}

pub fn execute_handler<'info>(
    mut executor: SeahorseSigner<'info, '_>,
    mut multisig: Mutable<LoadedMultisig<'info, '_>>,
    mut transaction: Mutable<LoadedTransaction<'info, '_>>,
    mut recipient: UncheckedAccount<'info>,
    mut tx_id: u64,
) -> () {
    if !is_owner(multisig.clone(), executor.key()) {
        panic!("Only owners can execute");
    }

    if !(transaction.borrow().multisig == multisig.borrow().__account__.key()) {
        panic!("Transaction does not belong to this multisig");
    }

    if !(transaction.borrow().tx_id == tx_id) {
        panic!("Transaction ID mismatch");
    }

    if !(!transaction.borrow().is_executed) {
        panic!("Transaction already executed");
    }

    if !(transaction.borrow().recipient == recipient.key()) {
        panic!("Recipient mismatch");
    }

    if !(transaction.borrow().approval_count >= multisig.borrow().threshold) {
        panic!("Threshold not met");
    }

    assign!(transaction.borrow_mut().is_executed, true);

    {
        let amount = transaction.borrow().amount.clone();

        **multisig
            .borrow()
            .__account__
            .to_account_info()
            .try_borrow_mut_lamports()
            .unwrap() -= amount;

        **recipient
            .clone()
            .to_account_info()
            .try_borrow_mut_lamports()
            .unwrap() += amount;
    };

    solana_program::msg!(
        "{}",
        format!(
            "Transaction {} executed: {} lamports to {:?}",
            tx_id,
            transaction.borrow().amount,
            recipient.key()
        )
    );
}

pub fn get_multisig_info_handler<'info>(mut multisig: Mutable<LoadedMultisig<'info, '_>>) -> () {
    solana_program::msg!("{}", format!("Multisig {}:", multisig.borrow().multisig_id));

    solana_program::msg!("{}", format!("Threshold: {}", multisig.borrow().threshold));

    solana_program::msg!(
        "{}",
        format!("Owner count: {}", multisig.borrow().owner_count)
    );

    solana_program::msg!(
        "{}",
        format!("Next tx ID: {}", multisig.borrow().next_tx_id)
    );
}

pub fn get_transaction_info_handler<'info>(
    mut transaction: Mutable<LoadedTransaction<'info, '_>>,
) -> () {
    solana_program::msg!("{}", format!("Transaction {}:", transaction.borrow().tx_id));

    solana_program::msg!(
        "{}",
        format!("Recipient: {:?}", transaction.borrow().recipient)
    );

    solana_program::msg!("{}", format!("Amount: {}", transaction.borrow().amount));

    solana_program::msg!(
        "{}",
        format!("Approvals: {}", transaction.borrow().approval_count)
    );

    solana_program::msg!(
        "{}",
        format!("Executed: {}", transaction.borrow().is_executed)
    );
}

pub fn is_owner<'info>(mut multisig: Mutable<LoadedMultisig<'info, '_>>, mut addr: Pubkey) -> bool {
    if addr == multisig.borrow().owner_1 {
        return true;
    }

    if addr == multisig.borrow().owner_2 {
        return true;
    }

    if addr == multisig.borrow().owner_3 {
        return true;
    }

    if addr == multisig.borrow().owner_4 {
        return true;
    }

    if addr == multisig.borrow().owner_5 {
        return true;
    }

    if addr == multisig.borrow().owner_6 {
        return true;
    }

    if addr == multisig.borrow().owner_7 {
        return true;
    }

    if addr == multisig.borrow().owner_8 {
        return true;
    }

    if addr == multisig.borrow().owner_9 {
        return true;
    }

    if addr == multisig.borrow().owner_10 {
        return true;
    }

    return false;
}

pub fn propose_transaction_handler<'info>(
    mut proposer: SeahorseSigner<'info, '_>,
    mut multisig: Mutable<LoadedMultisig<'info, '_>>,
    mut transaction: Empty<Mutable<LoadedTransaction<'info, '_>>>,
    mut tx_id: u64,
    mut recipient: Pubkey,
    mut amount: u64,
) -> () {
    if !is_owner(multisig.clone(), proposer.key()) {
        panic!("Only owners can propose transactions");
    }

    if !(tx_id == multisig.borrow().next_tx_id) {
        panic!("Invalid transaction ID");
    }

    assign!(
        multisig.borrow_mut().next_tx_id,
        multisig.borrow().next_tx_id + 1
    );

    if !(amount > 0) {
        panic!("Amount must be greater than zero");
    }

    let mut tx_bump = transaction.bump.unwrap();
    let mut transaction = transaction.account.clone();

    assign!(
        transaction.borrow_mut().multisig,
        multisig.borrow().__account__.key()
    );

    assign!(transaction.borrow_mut().tx_id, tx_id);

    assign!(transaction.borrow_mut().proposer, proposer.key());

    assign!(transaction.borrow_mut().recipient, recipient);

    assign!(transaction.borrow_mut().amount, amount);

    assign!(transaction.borrow_mut().approval_count, 0);

    assign!(transaction.borrow_mut().is_executed, false);

    assign!(transaction.borrow_mut().bump, tx_bump);

    solana_program::msg!(
        "{}",
        format!(
            "Transaction proposed: id={}, recipient={:?}, amount={}",
            tx_id, recipient, amount
        )
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

declare_id!("5h7CmLSs5q2ZfWFns5yUjN6BTYVxVxTNddJ2X1yHnWCd");

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
mod multisig {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    # [instruction (tx_id : u64)]
    pub struct Approve<'info> {
        #[account(mut)]
        pub approver: Signer<'info>,
        #[account(mut)]
        pub multisig: Box<Account<'info, dot::program::Multisig>>,
        #[account(mut)]
        pub transaction: Box<Account<'info, dot::program::Transaction>>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: Approval > () + 8 , payer = approver , seeds = ["approval" . as_bytes () . as_ref () , multisig . multisig_id . to_le_bytes () . as_ref () , tx_id . to_le_bytes () . as_ref () , approver . key () . as_ref ()] , bump)]
        pub approval: Box<Account<'info, dot::program::Approval>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn approve(ctx: Context<Approve>, tx_id: u64) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "system_program",
            ctx.accounts.system_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let approver = SeahorseSigner {
            account: &ctx.accounts.approver,
            programs: &programs_map,
        };

        let multisig = dot::program::Multisig::load(&mut ctx.accounts.multisig, &programs_map);
        let transaction =
            dot::program::Transaction::load(&mut ctx.accounts.transaction, &programs_map);

        let approval = Empty {
            account: dot::program::Approval::load(&mut ctx.accounts.approval, &programs_map),
            bump: Some(ctx.bumps.approval),
        };

        let clock = &ctx.accounts.clock.clone();

        approve_handler(
            approver.clone(),
            multisig.clone(),
            transaction.clone(),
            approval.clone(),
            clock.clone(),
            tx_id,
        );

        dot::program::Multisig::store(multisig);

        dot::program::Transaction::store(transaction);

        dot::program::Approval::store(approval.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (multisig_id : u64 , threshold : u8 , owner_count : u8 , owner_1 : Pubkey , owner_2 : Pubkey , owner_3 : Pubkey)]
    pub struct CreateMultisig<'info> {
        #[account(mut)]
        pub creator: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: Multisig > () + 8 , payer = creator , seeds = ["multisig" . as_bytes () . as_ref () , multisig_id . to_le_bytes () . as_ref ()] , bump)]
        pub multisig: Box<Account<'info, dot::program::Multisig>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn create_multisig(
        ctx: Context<CreateMultisig>,
        multisig_id: u64,
        threshold: u8,
        owner_count: u8,
        owner_1: Pubkey,
        owner_2: Pubkey,
        owner_3: Pubkey,
    ) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "system_program",
            ctx.accounts.system_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let creator = SeahorseSigner {
            account: &ctx.accounts.creator,
            programs: &programs_map,
        };

        let multisig = Empty {
            account: dot::program::Multisig::load(&mut ctx.accounts.multisig, &programs_map),
            bump: Some(ctx.bumps.multisig),
        };

        create_multisig_handler(
            creator.clone(),
            multisig.clone(),
            multisig_id,
            threshold,
            owner_count,
            owner_1,
            owner_2,
            owner_3,
        );

        dot::program::Multisig::store(multisig.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (tx_id : u64)]
    pub struct Execute<'info> {
        #[account(mut)]
        pub executor: Signer<'info>,
        #[account(mut)]
        pub multisig: Box<Account<'info, dot::program::Multisig>>,
        #[account(mut)]
        pub transaction: Box<Account<'info, dot::program::Transaction>>,
        #[account(mut)]
        #[doc = "CHECK: This account is unchecked."]
        pub recipient: UncheckedAccount<'info>,
    }

    pub fn execute(ctx: Context<Execute>, tx_id: u64) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let executor = SeahorseSigner {
            account: &ctx.accounts.executor,
            programs: &programs_map,
        };

        let multisig = dot::program::Multisig::load(&mut ctx.accounts.multisig, &programs_map);
        let transaction =
            dot::program::Transaction::load(&mut ctx.accounts.transaction, &programs_map);

        let recipient = &ctx.accounts.recipient.clone();

        execute_handler(
            executor.clone(),
            multisig.clone(),
            transaction.clone(),
            recipient.clone(),
            tx_id,
        );

        dot::program::Multisig::store(multisig);

        dot::program::Transaction::store(transaction);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct GetMultisigInfo<'info> {
        #[account(mut)]
        pub multisig: Box<Account<'info, dot::program::Multisig>>,
    }

    pub fn get_multisig_info(ctx: Context<GetMultisigInfo>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let multisig = dot::program::Multisig::load(&mut ctx.accounts.multisig, &programs_map);

        get_multisig_info_handler(multisig.clone());

        dot::program::Multisig::store(multisig);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct GetTransactionInfo<'info> {
        #[account(mut)]
        pub transaction: Box<Account<'info, dot::program::Transaction>>,
    }

    pub fn get_transaction_info(ctx: Context<GetTransactionInfo>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let transaction =
            dot::program::Transaction::load(&mut ctx.accounts.transaction, &programs_map);

        get_transaction_info_handler(transaction.clone());

        dot::program::Transaction::store(transaction);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (tx_id : u64 , recipient : Pubkey , amount : u64)]
    pub struct ProposeTransaction<'info> {
        #[account(mut)]
        pub proposer: Signer<'info>,
        #[account(mut)]
        pub multisig: Box<Account<'info, dot::program::Multisig>>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: Transaction > () + 8 , payer = proposer , seeds = ["transaction" . as_bytes () . as_ref () , multisig . multisig_id . to_le_bytes () . as_ref () , tx_id . to_le_bytes () . as_ref ()] , bump)]
        pub transaction: Box<Account<'info, dot::program::Transaction>>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn propose_transaction(
        ctx: Context<ProposeTransaction>,
        tx_id: u64,
        recipient: Pubkey,
        amount: u64,
    ) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "system_program",
            ctx.accounts.system_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let proposer = SeahorseSigner {
            account: &ctx.accounts.proposer,
            programs: &programs_map,
        };

        let multisig = dot::program::Multisig::load(&mut ctx.accounts.multisig, &programs_map);
        let transaction = Empty {
            account: dot::program::Transaction::load(&mut ctx.accounts.transaction, &programs_map),
            bump: Some(ctx.bumps.transaction),
        };

        propose_transaction_handler(
            proposer.clone(),
            multisig.clone(),
            transaction.clone(),
            tx_id,
            recipient,
            amount,
        );

        dot::program::Multisig::store(multisig);

        dot::program::Transaction::store(transaction.account);

        return Ok(());
    }
}

