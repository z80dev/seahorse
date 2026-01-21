use anchor_lang::prelude::*;

// Use same program ID as Seahorse version for parity testing
declare_id!("F4fSecp12t3QtQaXTUWrjxLec1wQXJ31iQzuE2WjPsoB");

/// Anchor reference implementation for realloc and close account patterns.
///
/// Demonstrates:
/// 1. Account initialization with explicit space
/// 2. Growing accounts with realloc (increase space)
/// 3. Shrinking accounts with realloc (decrease space)
/// 4. Closing accounts with rent return
/// 5. Zero-copy operations for large data
#[program]
pub mod realloc_close_anchor {
    use super::*;

    /// Initialize a dynamic data account with initial content
    pub fn initialize_data(
        ctx: Context<InitializeData>,
        data_id: u64,
        initial_content: String,
    ) -> Result<()> {
        let data = &mut ctx.accounts.data;
        data.owner = ctx.accounts.owner.key();
        data.data_id = data_id;
        data.content = initial_content;
        data.version = 1;
        data.bump = ctx.bumps.data;

        msg!("Initialized data account with id: {}", data_id);
        msg!("Content length: {} bytes", data.content.len());
        Ok(())
    }

    /// Grow the account to accommodate more content
    /// Uses realloc with zero_init = true to clear new space
    pub fn grow_account(
        ctx: Context<GrowAccount>,
        _data_id: u64,
        new_content: String,
        additional_space: u32,
    ) -> Result<()> {
        let data = &mut ctx.accounts.data;

        // Verify owner
        require!(
            data.owner == ctx.accounts.owner.key(),
            ReallocCloseError::Unauthorized
        );

        // Update content
        data.content = new_content;
        data.version += 1;

        msg!("Grew account by {} bytes", additional_space);
        msg!("New content length: {} bytes", data.content.len());
        msg!("Version: {}", data.version);
        Ok(())
    }

    /// Shrink the account to reclaim lamports
    /// Uses realloc with zero_init = false (more efficient for shrinking)
    pub fn shrink_account(
        ctx: Context<ShrinkAccount>,
        _data_id: u64,
        new_content: String,
    ) -> Result<()> {
        let data = &mut ctx.accounts.data;

        // Verify owner
        require!(
            data.owner == ctx.accounts.owner.key(),
            ReallocCloseError::Unauthorized
        );

        // Verify new content fits in smaller space
        require!(
            new_content.len() <= data.content.len(),
            ReallocCloseError::ContentTooLarge
        );

        // Update content
        data.content = new_content;
        data.version += 1;

        msg!("Shrunk account");
        msg!("New content length: {} bytes", data.content.len());
        msg!("Version: {}", data.version);
        Ok(())
    }

    /// Close the account and return lamports to owner
    /// Uses Anchor's close constraint for automatic rent return
    pub fn close_data_account(
        ctx: Context<CloseDataAccount>,
        _data_id: u64,
    ) -> Result<()> {
        msg!("Closing data account, returning lamports to owner");
        msg!("Final version was: {}", ctx.accounts.data.version);
        Ok(())
    }

    /// Initialize a record account that can track size changes
    pub fn create_record(
        ctx: Context<CreateRecord>,
        record_id: u64,
    ) -> Result<()> {
        let record = &mut ctx.accounts.record;
        record.owner = ctx.accounts.owner.key();
        record.record_id = record_id;
        record.current_size = 0;
        record.max_size_reached = 0;
        record.resize_count = 0;
        record.is_active = true;
        record.bump = ctx.bumps.record;

        msg!("Created record with id: {}", record_id);
        Ok(())
    }

    /// Track a resize operation (for parity with Seahorse's tracking pattern)
    pub fn record_resize(
        ctx: Context<RecordResize>,
        _record_id: u64,
        new_size: u64,
        is_grow: bool,
    ) -> Result<()> {
        let record = &mut ctx.accounts.record;

        // Verify owner
        require!(
            record.owner == ctx.accounts.owner.key(),
            ReallocCloseError::Unauthorized
        );

        // Verify record is active
        require!(record.is_active, ReallocCloseError::RecordInactive);

        // Update tracking
        record.current_size = new_size;
        record.resize_count += 1;

        if new_size > record.max_size_reached {
            record.max_size_reached = new_size;
        }

        msg!(
            "Recorded resize: {} to {} bytes",
            if is_grow { "grow" } else { "shrink" },
            new_size
        );
        msg!("Total resizes: {}", record.resize_count);
        Ok(())
    }

    /// Close a record (mark as inactive and clear data)
    pub fn close_record(
        ctx: Context<CloseRecord>,
        _record_id: u64,
    ) -> Result<()> {
        msg!("Closing record, returning lamports to owner");
        msg!("Final resize count: {}", ctx.accounts.record.resize_count);
        Ok(())
    }

    /// Get info about a data account (read-only)
    pub fn get_data_info(
        ctx: Context<GetDataInfo>,
        _data_id: u64,
    ) -> Result<()> {
        let data = &ctx.accounts.data;
        msg!("Data ID: {}", data.data_id);
        msg!("Owner: {}", data.owner);
        msg!("Content length: {}", data.content.len());
        msg!("Version: {}", data.version);
        Ok(())
    }

    /// Get info about a record account (read-only)
    pub fn get_record_info(
        ctx: Context<GetRecordInfo>,
        _record_id: u64,
    ) -> Result<()> {
        let record = &ctx.accounts.record;
        msg!("Record ID: {}", record.record_id);
        msg!("Owner: {}", record.owner);
        msg!("Current size: {}", record.current_size);
        msg!("Max size reached: {}", record.max_size_reached);
        msg!("Resize count: {}", record.resize_count);
        msg!("Is active: {}", record.is_active);
        Ok(())
    }
}

// ============================================================================
// Account Structures
// ============================================================================

/// Dynamic data account that can be resized
#[account]
pub struct DynamicData {
    pub owner: Pubkey,
    pub data_id: u64,
    pub content: String,  // Variable length - requires realloc for changes
    pub version: u64,
    pub bump: u8,
}

impl DynamicData {
    /// Base space without string content
    pub const BASE_SPACE: usize = 8 + // discriminator
        32 + // owner
        8 +  // data_id
        4 +  // string length prefix
        8 +  // version
        1;   // bump

    /// Calculate total space for given content length
    pub fn space(content_len: usize) -> usize {
        Self::BASE_SPACE + content_len
    }
}

/// Record account for tracking resize operations
#[account]
pub struct ResizeRecord {
    pub owner: Pubkey,
    pub record_id: u64,
    pub current_size: u64,
    pub max_size_reached: u64,
    pub resize_count: u64,
    pub is_active: bool,
    pub bump: u8,
}

impl ResizeRecord {
    pub const SPACE: usize = 8 + // discriminator
        32 + // owner
        8 +  // record_id
        8 +  // current_size
        8 +  // max_size_reached
        8 +  // resize_count
        1 +  // is_active
        1;   // bump
}

// ============================================================================
// Instruction Contexts
// ============================================================================

#[derive(Accounts)]
#[instruction(data_id: u64, initial_content: String)]
pub struct InitializeData<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        space = DynamicData::space(initial_content.len()),
        seeds = [b"data", data_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub data: Account<'info, DynamicData>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(data_id: u64, new_content: String, additional_space: u32)]
pub struct GrowAccount<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        realloc = DynamicData::space(new_content.len()) + additional_space as usize,
        realloc::payer = owner,
        realloc::zero = true,
        seeds = [b"data", data_id.to_le_bytes().as_ref()],
        bump = data.bump,
    )]
    pub data: Account<'info, DynamicData>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(data_id: u64, new_content: String)]
pub struct ShrinkAccount<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        realloc = DynamicData::space(new_content.len()),
        realloc::payer = owner,
        realloc::zero = false,
        seeds = [b"data", data_id.to_le_bytes().as_ref()],
        bump = data.bump,
    )]
    pub data: Account<'info, DynamicData>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(data_id: u64)]
pub struct CloseDataAccount<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        close = owner,
        has_one = owner,
        seeds = [b"data", data_id.to_le_bytes().as_ref()],
        bump = data.bump,
    )]
    pub data: Account<'info, DynamicData>,
}

#[derive(Accounts)]
#[instruction(record_id: u64)]
pub struct CreateRecord<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        space = ResizeRecord::SPACE,
        seeds = [b"record", record_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub record: Account<'info, ResizeRecord>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(record_id: u64)]
pub struct RecordResize<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [b"record", record_id.to_le_bytes().as_ref()],
        bump = record.bump,
    )]
    pub record: Account<'info, ResizeRecord>,
}

#[derive(Accounts)]
#[instruction(record_id: u64)]
pub struct CloseRecord<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        close = owner,
        has_one = owner,
        seeds = [b"record", record_id.to_le_bytes().as_ref()],
        bump = record.bump,
    )]
    pub record: Account<'info, ResizeRecord>,
}

#[derive(Accounts)]
#[instruction(data_id: u64)]
pub struct GetDataInfo<'info> {
    pub owner: Signer<'info>,

    #[account(
        seeds = [b"data", data_id.to_le_bytes().as_ref()],
        bump = data.bump,
    )]
    pub data: Account<'info, DynamicData>,
}

#[derive(Accounts)]
#[instruction(record_id: u64)]
pub struct GetRecordInfo<'info> {
    pub owner: Signer<'info>,

    #[account(
        seeds = [b"record", record_id.to_le_bytes().as_ref()],
        bump = record.bump,
    )]
    pub record: Account<'info, ResizeRecord>,
}

// ============================================================================
// Errors
// ============================================================================

#[error_code]
pub enum ReallocCloseError {
    #[msg("Only the owner can perform this action")]
    Unauthorized,

    #[msg("New content is too large for the requested space")]
    ContentTooLarge,

    #[msg("Record is inactive")]
    RecordInactive,
}
