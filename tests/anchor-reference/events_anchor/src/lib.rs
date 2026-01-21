use anchor_lang::prelude::*;

declare_id!("CCLFgYt5eEtrMU9mrHWPQ6FHn88GFosEnyj1HCDhMD5U");

// ============================================================================
// Event Definitions
// ============================================================================

// Simple event with basic primitives
#[event]
pub struct SimpleEvent {
    pub value: u64,
    pub label: String,
}

// Event with Pubkey (common pattern for tracking user actions)
#[event]
pub struct UserActionEvent {
    pub user: Pubkey,
    pub action_type: u8,
    pub timestamp: i64,
}

// Event with multiple numeric types
#[event]
pub struct NumericEvent {
    pub unsigned_small: u8,
    pub unsigned_medium: u32,
    pub unsigned_large: u64,
    pub signed_value: i64,
}

// Event with complex data (array simulation via fixed values)
#[event]
pub struct TransferEvent {
    pub from: Pubkey,
    pub to: Pubkey,
    pub amount: u64,
    pub memo: String,
}

// Event for tracking state changes
#[event]
pub struct StateChangeEvent {
    pub account: Pubkey,
    pub old_value: u64,
    pub new_value: u64,
    pub change_type: String,
}

// ============================================================================
// Account Structures
// ============================================================================

#[account]
#[derive(InitSpace)]
pub struct EventCounter {
    pub authority: Pubkey,
    pub event_count: u64,
    pub last_event_type: u8,
    pub bump: u8,
}

// ============================================================================
// Program Instructions
// ============================================================================

#[program]
pub mod events_anchor {
    use super::*;

    /// Initialize the event counter account
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.authority = ctx.accounts.authority.key();
        counter.event_count = 0;
        counter.last_event_type = 0;
        counter.bump = ctx.bumps.counter;

        msg!("Initialized event counter");
        Ok(())
    }

    /// Emit a simple event with basic data types
    pub fn emit_simple(
        ctx: Context<EmitEvent>,
        value: u64,
        label: String,
    ) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.event_count += 1;
        counter.last_event_type = 1; // SimpleEvent type

        emit!(SimpleEvent { value, label });

        msg!("Emitted SimpleEvent");
        Ok(())
    }

    /// Emit a user action event (tracks who did what)
    pub fn emit_user_action(
        ctx: Context<EmitEvent>,
        action_type: u8,
        timestamp: i64,
    ) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.event_count += 1;
        counter.last_event_type = 2; // UserActionEvent type

        emit!(UserActionEvent {
            user: ctx.accounts.authority.key(),
            action_type,
            timestamp,
        });

        msg!("Emitted UserActionEvent");
        Ok(())
    }

    /// Emit an event with various numeric types
    pub fn emit_numeric(
        ctx: Context<EmitEvent>,
        unsigned_small: u8,
        unsigned_medium: u32,
        unsigned_large: u64,
        signed_value: i64,
    ) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.event_count += 1;
        counter.last_event_type = 3; // NumericEvent type

        emit!(NumericEvent {
            unsigned_small,
            unsigned_medium,
            unsigned_large,
            signed_value,
        });

        msg!("Emitted NumericEvent");
        Ok(())
    }

    /// Emit a transfer event (common in token programs)
    pub fn emit_transfer(
        ctx: Context<EmitEvent>,
        to: Pubkey,
        amount: u64,
        memo: String,
    ) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.event_count += 1;
        counter.last_event_type = 4; // TransferEvent type

        emit!(TransferEvent {
            from: ctx.accounts.authority.key(),
            to,
            amount,
            memo,
        });

        msg!("Emitted TransferEvent");
        Ok(())
    }

    /// Emit a state change event and update counter
    pub fn emit_state_change(
        ctx: Context<EmitEvent>,
        old_value: u64,
        new_value: u64,
        change_type: String,
    ) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.event_count += 1;
        counter.last_event_type = 5; // StateChangeEvent type

        emit!(StateChangeEvent {
            account: ctx.accounts.counter.key(),
            old_value,
            new_value,
            change_type,
        });

        msg!("Emitted StateChangeEvent");
        Ok(())
    }

    /// Emit multiple events in a single transaction
    pub fn emit_multiple(
        ctx: Context<EmitEvent>,
        count: u8,
    ) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        let user = ctx.accounts.authority.key();

        for i in 0..count {
            emit!(SimpleEvent {
                value: i as u64,
                label: format!("Event {}", i),
            });
            counter.event_count += 1;
        }

        counter.last_event_type = 6; // Multiple events

        msg!("Emitted {} SimpleEvents", count);
        Ok(())
    }

    /// Get event counter info (read-only)
    pub fn get_counter_info(ctx: Context<GetCounterInfo>) -> Result<()> {
        let counter = &ctx.accounts.counter;
        msg!("Event count: {}", counter.event_count);
        msg!("Last event type: {}", counter.last_event_type);
        msg!("Authority: {}", counter.authority);
        Ok(())
    }
}

// ============================================================================
// Account Contexts
// ============================================================================

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + EventCounter::INIT_SPACE,
        seeds = [b"counter", authority.key().as_ref()],
        bump
    )]
    pub counter: Account<'info, EventCounter>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct EmitEvent<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"counter", authority.key().as_ref()],
        bump = counter.bump,
        has_one = authority
    )]
    pub counter: Account<'info, EventCounter>,
}

#[derive(Accounts)]
pub struct GetCounterInfo<'info> {
    pub authority: Signer<'info>,

    #[account(
        seeds = [b"counter", authority.key().as_ref()],
        bump = counter.bump,
    )]
    pub counter: Account<'info, EventCounter>,
}
