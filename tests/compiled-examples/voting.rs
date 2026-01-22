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
pub struct Proposal {
    pub creator: Pubkey,
    pub proposal_id: u64,
    pub title: String,
    pub description: String,
    pub yes_votes: u64,
    pub no_votes: u64,
    pub start_slot: u64,
    pub end_slot: u64,
    pub is_finalized: bool,
    pub bump: u8,
}

impl<'info, 'entrypoint> Proposal {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedProposal<'info, 'entrypoint>> {
        let creator = account.creator.clone();
        let proposal_id = account.proposal_id;
        let title = account.title.clone();
        let description = account.description.clone();
        let yes_votes = account.yes_votes;
        let no_votes = account.no_votes;
        let start_slot = account.start_slot;
        let end_slot = account.end_slot;
        let is_finalized = account.is_finalized.clone();
        let bump = account.bump;

        Mutable::new(LoadedProposal {
            __account__: account,
            __programs__: programs_map,
            creator,
            proposal_id,
            title,
            description,
            yes_votes,
            no_votes,
            start_slot,
            end_slot,
            is_finalized,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedProposal>) {
        let mut loaded = loaded.borrow_mut();
        let creator = loaded.creator.clone();

        loaded.__account__.creator = creator;

        let proposal_id = loaded.proposal_id;

        loaded.__account__.proposal_id = proposal_id;

        let title = loaded.title.clone();

        loaded.__account__.title = title;

        let description = loaded.description.clone();

        loaded.__account__.description = description;

        let yes_votes = loaded.yes_votes;

        loaded.__account__.yes_votes = yes_votes;

        let no_votes = loaded.no_votes;

        loaded.__account__.no_votes = no_votes;

        let start_slot = loaded.start_slot;

        loaded.__account__.start_slot = start_slot;

        let end_slot = loaded.end_slot;

        loaded.__account__.end_slot = end_slot;

        let is_finalized = loaded.is_finalized.clone();

        loaded.__account__.is_finalized = is_finalized;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedProposal<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, Proposal>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub creator: Pubkey,
    pub proposal_id: u64,
    pub title: String,
    pub description: String,
    pub yes_votes: u64,
    pub no_votes: u64,
    pub start_slot: u64,
    pub end_slot: u64,
    pub is_finalized: bool,
    pub bump: u8,
}

#[account]
#[derive(Debug)]
pub struct VoterRecord {
    pub voter: Pubkey,
    pub proposal: Pubkey,
    pub vote_yes: bool,
    pub voted_at_slot: u64,
    pub bump: u8,
}

impl<'info, 'entrypoint> VoterRecord {
    pub fn load(
        account: &'entrypoint mut Box<Account<'info, Self>>,
        programs_map: &'entrypoint ProgramsMap<'info>,
    ) -> Mutable<LoadedVoterRecord<'info, 'entrypoint>> {
        let voter = account.voter.clone();
        let proposal = account.proposal.clone();
        let vote_yes = account.vote_yes.clone();
        let voted_at_slot = account.voted_at_slot;
        let bump = account.bump;

        Mutable::new(LoadedVoterRecord {
            __account__: account,
            __programs__: programs_map,
            voter,
            proposal,
            vote_yes,
            voted_at_slot,
            bump,
        })
    }

    pub fn store(loaded: Mutable<LoadedVoterRecord>) {
        let mut loaded = loaded.borrow_mut();
        let voter = loaded.voter.clone();

        loaded.__account__.voter = voter;

        let proposal = loaded.proposal.clone();

        loaded.__account__.proposal = proposal;

        let vote_yes = loaded.vote_yes.clone();

        loaded.__account__.vote_yes = vote_yes;

        let voted_at_slot = loaded.voted_at_slot;

        loaded.__account__.voted_at_slot = voted_at_slot;

        let bump = loaded.bump;

        loaded.__account__.bump = bump;
    }
}

#[derive(Debug)]
pub struct LoadedVoterRecord<'info, 'entrypoint> {
    pub __account__: &'entrypoint mut Box<Account<'info, VoterRecord>>,
    pub __programs__: &'entrypoint ProgramsMap<'info>,
    pub voter: Pubkey,
    pub proposal: Pubkey,
    pub vote_yes: bool,
    pub voted_at_slot: u64,
    pub bump: u8,
}

pub fn cast_vote_handler<'info>(
    mut voter: SeahorseSigner<'info, '_>,
    mut proposal: Mutable<LoadedProposal<'info, '_>>,
    mut voter_record: Empty<Mutable<LoadedVoterRecord<'info, '_>>>,
    mut clock: Sysvar<'info, Clock>,
    mut proposal_id: u64,
    mut vote_yes: bool,
) -> () {
    let mut current_slot = clock.slot;

    if !(proposal.borrow().proposal_id == proposal_id) {
        panic!("Proposal ID mismatch");
    }

    if !(current_slot < proposal.borrow().end_slot) {
        panic!("Voting period has ended");
    }

    if !(!proposal.borrow().is_finalized) {
        panic!("Proposal has already been finalized");
    }

    let mut record_bump = voter_record.bump.unwrap();
    let mut voter_record = voter_record.account.clone();

    assign!(voter_record.borrow_mut().voter, voter.key());

    assign!(
        voter_record.borrow_mut().proposal,
        proposal.borrow().__account__.key()
    );

    assign!(voter_record.borrow_mut().vote_yes, vote_yes);

    assign!(voter_record.borrow_mut().voted_at_slot, current_slot);

    assign!(voter_record.borrow_mut().bump, record_bump);

    if vote_yes {
        assign!(
            proposal.borrow_mut().yes_votes,
            proposal.borrow().yes_votes + 1
        );
    } else {
        assign!(
            proposal.borrow_mut().no_votes,
            proposal.borrow().no_votes + 1
        );
    }

    solana_program::msg!(
        "{}",
        format!(
            "Vote cast: voter={:?}, proposal={}, vote_yes={}",
            voter.key(),
            proposal.borrow().proposal_id,
            vote_yes
        )
    );
}

pub fn create_proposal_handler<'info>(
    mut creator: SeahorseSigner<'info, '_>,
    mut proposal: Empty<Mutable<LoadedProposal<'info, '_>>>,
    mut clock: Sysvar<'info, Clock>,
    mut proposal_id: u64,
    mut title: String,
    mut description: String,
    mut voting_duration_slots: u64,
) -> () {
    if !((title.chars().count() as u64) <= 64) {
        panic!("Title exceeds maximum length of 64 characters");
    }

    if !((description.chars().count() as u64) <= 256) {
        panic!("Description exceeds maximum length of 256 characters");
    }

    if !(voting_duration_slots > 0) {
        panic!("Voting duration must be greater than zero");
    }

    let mut proposal_bump = proposal.bump.unwrap();
    let mut current_slot = clock.slot;
    let mut proposal = proposal.account.clone();

    assign!(proposal.borrow_mut().creator, creator.key());

    assign!(proposal.borrow_mut().proposal_id, proposal_id);

    assign!(proposal.borrow_mut().title, title.clone());

    assign!(proposal.borrow_mut().description, description.clone());

    assign!(proposal.borrow_mut().yes_votes, 0);

    assign!(proposal.borrow_mut().no_votes, 0);

    assign!(proposal.borrow_mut().start_slot, current_slot);

    assign!(
        proposal.borrow_mut().end_slot,
        current_slot + voting_duration_slots
    );

    assign!(proposal.borrow_mut().is_finalized, false);

    assign!(proposal.borrow_mut().bump, proposal_bump);

    solana_program::msg!(
        "{}",
        format!("Proposal {} created by {:?}", proposal_id, creator.key())
    );
}

pub fn finalize_proposal_handler<'info>(
    mut finalizer: SeahorseSigner<'info, '_>,
    mut proposal: Mutable<LoadedProposal<'info, '_>>,
    mut clock: Sysvar<'info, Clock>,
) -> () {
    let mut current_slot = clock.slot;

    if !(current_slot >= proposal.borrow().end_slot) {
        panic!("Voting period has not ended yet");
    }

    if !(!proposal.borrow().is_finalized) {
        panic!("Proposal has already been finalized");
    }

    assign!(proposal.borrow_mut().is_finalized, true);

    if proposal.borrow().yes_votes > proposal.borrow().no_votes {
        solana_program::msg!(
            "{}",
            format!(
                "Proposal {} finalized: PASSED (yes: {}, no: {})",
                proposal.borrow().proposal_id,
                proposal.borrow().yes_votes,
                proposal.borrow().no_votes
            )
        );
    } else {
        if proposal.borrow().no_votes > proposal.borrow().yes_votes {
            solana_program::msg!(
                "{}",
                format!(
                    "Proposal {} finalized: REJECTED (yes: {}, no: {})",
                    proposal.borrow().proposal_id,
                    proposal.borrow().yes_votes,
                    proposal.borrow().no_votes
                )
            );
        } else {
            solana_program::msg!(
                "{}",
                format!(
                    "Proposal {} finalized: TIE (yes: {}, no: {})",
                    proposal.borrow().proposal_id,
                    proposal.borrow().yes_votes,
                    proposal.borrow().no_votes
                )
            );
        }
    }
}

pub fn get_proposal_info_handler<'info>(mut proposal: Mutable<LoadedProposal<'info, '_>>) -> () {
    solana_program::msg!(
        "{}",
        format!(
            "Proposal {}: {}",
            proposal.borrow().proposal_id,
            proposal.borrow().title
        )
    );

    solana_program::msg!(
        "{}",
        format!("Description: {}", proposal.borrow().description)
    );

    solana_program::msg!(
        "{}",
        format!(
            "Votes: yes={}, no={}",
            proposal.borrow().yes_votes,
            proposal.borrow().no_votes
        )
    );

    solana_program::msg!(
        "{}",
        format!(
            "Slots: start={}, end={}",
            proposal.borrow().start_slot,
            proposal.borrow().end_slot
        )
    );

    solana_program::msg!(
        "{}",
        format!("Finalized: {}", proposal.borrow().is_finalized)
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

declare_id!("VoteSimp1e111111111111111111111111111111111");

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
mod voting {
    use super::*;
    use seahorse_util::*;
    use std::collections::HashMap;

    #[derive(Accounts)]
    # [instruction (proposal_id : u64 , vote_yes : bool)]
    pub struct CastVote<'info> {
        #[account(mut)]
        pub voter: Signer<'info>,
        #[account(mut)]
        pub proposal: Box<Account<'info, dot::program::Proposal>>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: VoterRecord > () + 8 , payer = voter , seeds = ["voter_record" . as_bytes () . as_ref () , proposal_id . to_le_bytes () . as_ref () , voter . key () . as_ref ()] , bump)]
        pub voter_record: Box<Account<'info, dot::program::VoterRecord>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn cast_vote(ctx: Context<CastVote>, proposal_id: u64, vote_yes: bool) -> Result<()> {
        let mut programs = HashMap::new();

        programs.insert(
            "system_program",
            ctx.accounts.system_program.to_account_info(),
        );

        let programs_map = ProgramsMap(programs);
        let voter = SeahorseSigner {
            account: &ctx.accounts.voter,
            programs: &programs_map,
        };

        let proposal = dot::program::Proposal::load(&mut ctx.accounts.proposal, &programs_map);
        let voter_record = Empty {
            account: dot::program::VoterRecord::load(&mut ctx.accounts.voter_record, &programs_map),
            bump: Some(ctx.bumps.voter_record),
        };

        let clock = &ctx.accounts.clock.clone();

        cast_vote_handler(
            voter.clone(),
            proposal.clone(),
            voter_record.clone(),
            clock.clone(),
            proposal_id,
            vote_yes,
        );

        dot::program::Proposal::store(proposal);

        dot::program::VoterRecord::store(voter_record.account);

        return Ok(());
    }

    #[derive(Accounts)]
    # [instruction (proposal_id : u64 , title : String , description : String , voting_duration_slots : u64)]
    pub struct CreateProposal<'info> {
        #[account(mut)]
        pub creator: Signer<'info>,
        # [account (init , space = std :: mem :: size_of :: < dot :: program :: Proposal > () + 8 + (400 as usize) , payer = creator , seeds = ["proposal" . as_bytes () . as_ref () , proposal_id . to_le_bytes () . as_ref ()] , bump)]
        pub proposal: Box<Account<'info, dot::program::Proposal>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
        pub rent: Sysvar<'info, Rent>,
        pub system_program: Program<'info, System>,
    }

    pub fn create_proposal(
        ctx: Context<CreateProposal>,
        proposal_id: u64,
        title: String,
        description: String,
        voting_duration_slots: u64,
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

        let proposal = Empty {
            account: dot::program::Proposal::load(&mut ctx.accounts.proposal, &programs_map),
            bump: Some(ctx.bumps.proposal),
        };

        let clock = &ctx.accounts.clock.clone();

        create_proposal_handler(
            creator.clone(),
            proposal.clone(),
            clock.clone(),
            proposal_id,
            title,
            description,
            voting_duration_slots,
        );

        dot::program::Proposal::store(proposal.account);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct FinalizeProposal<'info> {
        #[account(mut)]
        pub finalizer: Signer<'info>,
        #[account(mut)]
        pub proposal: Box<Account<'info, dot::program::Proposal>>,
        #[account()]
        pub clock: Sysvar<'info, Clock>,
    }

    pub fn finalize_proposal(ctx: Context<FinalizeProposal>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let finalizer = SeahorseSigner {
            account: &ctx.accounts.finalizer,
            programs: &programs_map,
        };

        let proposal = dot::program::Proposal::load(&mut ctx.accounts.proposal, &programs_map);
        let clock = &ctx.accounts.clock.clone();

        finalize_proposal_handler(finalizer.clone(), proposal.clone(), clock.clone());

        dot::program::Proposal::store(proposal);

        return Ok(());
    }

    #[derive(Accounts)]
    pub struct GetProposalInfo<'info> {
        #[account(mut)]
        pub proposal: Box<Account<'info, dot::program::Proposal>>,
    }

    pub fn get_proposal_info(ctx: Context<GetProposalInfo>) -> Result<()> {
        let mut programs = HashMap::new();
        let programs_map = ProgramsMap(programs);
        let proposal = dot::program::Proposal::load(&mut ctx.accounts.proposal, &programs_map);

        get_proposal_info_handler(proposal.clone());

        dot::program::Proposal::store(proposal);

        return Ok(());
    }
}

