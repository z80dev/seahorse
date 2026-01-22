use anchor_lang::prelude::*;

// Use same program ID as Seahorse version for parity tests
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod init_patterns_anchor {
    use super::*;

    /// Initialize a simple data account with primitive types
    /// Demonstrates basic space calculation for primitives
    pub fn init_simple(ctx: Context<InitSimple>, value_u8: u8, value_u64: u64) -> Result<()> {
        let data = &mut ctx.accounts.data;
        data.owner = ctx.accounts.owner.key();
        data.value_u8 = value_u8;
        data.value_u16 = 0;
        data.value_u32 = 0;
        data.value_u64 = value_u64;
        data.value_i64 = 0;
        data.bump = ctx.bumps.data;
        Ok(())
    }

    /// Update simple data values
    pub fn update_simple(
        ctx: Context<UpdateSimple>,
        value_u8: u8,
        value_u16: u16,
        value_u32: u32,
        value_u64: u64,
        value_i64: i64,
    ) -> Result<()> {
        let data = &mut ctx.accounts.data;
        data.value_u8 = value_u8;
        data.value_u16 = value_u16;
        data.value_u32 = value_u32;
        data.value_u64 = value_u64;
        data.value_i64 = value_i64;
        Ok(())
    }

    /// Initialize a data account with dynamic string
    /// Demonstrates padding for variable-length data
    pub fn init_with_string(ctx: Context<InitWithString>, name: String, description: String) -> Result<()> {
        let data = &mut ctx.accounts.data;
        data.owner = ctx.accounts.owner.key();
        data.name = name;
        data.description = description;
        data.bump = ctx.bumps.data;
        Ok(())
    }

    /// Update string data
    pub fn update_string(ctx: Context<UpdateString>, name: String, description: String) -> Result<()> {
        let data = &mut ctx.accounts.data;
        data.name = name;
        data.description = description;
        Ok(())
    }

    /// Initialize a data account with fixed-size array
    /// Demonstrates array space calculation
    pub fn init_with_array(ctx: Context<InitWithArray>) -> Result<()> {
        let data = &mut ctx.accounts.data;
        data.owner = ctx.accounts.owner.key();
        data.values = [0u64; 4];
        data.flags = [false; 8];
        data.bump = ctx.bumps.data;
        Ok(())
    }

    /// Update array values
    pub fn update_array(ctx: Context<UpdateArray>, values: [u64; 4], flags: [bool; 8]) -> Result<()> {
        let data = &mut ctx.accounts.data;
        data.values = values;
        data.flags = flags;
        Ok(())
    }

    /// Initialize a complex nested account
    /// Demonstrates nested struct space calculation
    pub fn init_complex(ctx: Context<InitComplex>, initial_count: u64) -> Result<()> {
        let data = &mut ctx.accounts.data;
        data.owner = ctx.accounts.owner.key();
        data.stats = Stats {
            count: initial_count,
            total: 0,
            average: 0,
        };
        data.is_active = true;
        data.bump = ctx.bumps.data;
        Ok(())
    }

    /// Update complex data stats
    pub fn update_complex(ctx: Context<UpdateComplex>, count: u64, total: u64) -> Result<()> {
        let data = &mut ctx.accounts.data;
        data.stats.count = count;
        data.stats.total = total;
        // Calculate average (safe division)
        data.stats.average = if count > 0 { total / count } else { 0 };
        Ok(())
    }

    /// Toggle complex data active state
    pub fn toggle_active(ctx: Context<UpdateComplex>) -> Result<()> {
        let data = &mut ctx.accounts.data;
        data.is_active = !data.is_active;
        Ok(())
    }

    /// Close an account and reclaim lamports
    /// Demonstrates proper account closure with rent return
    pub fn close_simple(ctx: Context<CloseSimple>) -> Result<()> {
        // Anchor's close constraint handles everything
        msg!("Closing simple data account, returning lamports to owner");
        Ok(())
    }
}

// ============================================
// Account contexts
// ============================================

#[derive(Accounts)]
pub struct InitSimple<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        // Space: discriminator(8) + owner(32) + u8(1) + u16(2) + u32(4) + u64(8) + i64(8) + bump(1) = 64
        space = 8 + SimpleData::INIT_SPACE,
        seeds = [b"simple", owner.key().as_ref()],
        bump
    )]
    pub data: Account<'info, SimpleData>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateSimple<'info> {
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [b"simple", owner.key().as_ref()],
        bump = data.bump,
        has_one = owner @ ErrorCode::Unauthorized
    )]
    pub data: Account<'info, SimpleData>,
}

#[derive(Accounts)]
pub struct InitWithString<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        // Space: discriminator(8) + pubkey(32) + name(4+32) + description(4+128) + bump(1) = 209
        space = 8 + StringData::INIT_SPACE,
        seeds = [b"string_data", owner.key().as_ref()],
        bump
    )]
    pub data: Account<'info, StringData>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateString<'info> {
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [b"string_data", owner.key().as_ref()],
        bump = data.bump,
        has_one = owner @ ErrorCode::Unauthorized
    )]
    pub data: Account<'info, StringData>,
}

#[derive(Accounts)]
pub struct InitWithArray<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        // Space: discriminator(8) + pubkey(32) + [u64; 4](32) + [bool; 8](8) + bump(1) = 81
        space = 8 + ArrayData::INIT_SPACE,
        seeds = [b"array_data", owner.key().as_ref()],
        bump
    )]
    pub data: Account<'info, ArrayData>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateArray<'info> {
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [b"array_data", owner.key().as_ref()],
        bump = data.bump,
        has_one = owner @ ErrorCode::Unauthorized
    )]
    pub data: Account<'info, ArrayData>,
}

#[derive(Accounts)]
pub struct InitComplex<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        // Space: discriminator(8) + pubkey(32) + Stats(24) + bool(1) + bump(1) = 66
        space = 8 + ComplexData::INIT_SPACE,
        seeds = [b"complex", owner.key().as_ref()],
        bump
    )]
    pub data: Account<'info, ComplexData>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateComplex<'info> {
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [b"complex", owner.key().as_ref()],
        bump = data.bump,
        has_one = owner @ ErrorCode::Unauthorized
    )]
    pub data: Account<'info, ComplexData>,
}

#[derive(Accounts)]
pub struct CloseSimple<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [b"simple", owner.key().as_ref()],
        bump = data.bump,
        has_one = owner @ ErrorCode::Unauthorized,
        close = owner  // Return lamports to owner
    )]
    pub data: Account<'info, SimpleData>,
}

// ============================================
// Account structs with space calculations
// ============================================

/// Simple data with various primitive types
/// Space: 32 + 1 + 2 + 4 + 8 + 8 + 1 = 56 bytes (+ 8 discriminator = 64)
#[account]
#[derive(InitSpace)]
pub struct SimpleData {
    pub owner: Pubkey,  // 32 bytes
    pub value_u8: u8,   // 1 byte
    pub value_u16: u16, // 2 bytes
    pub value_u32: u32, // 4 bytes
    pub value_u64: u64, // 8 bytes
    pub value_i64: i64, // 8 bytes
    pub bump: u8,       // 1 byte
}

/// String data demonstrating variable-length field sizing
/// Space: 32 + (4+32) + (4+128) + 1 = 201 bytes (+ 8 discriminator = 209)
#[account]
#[derive(InitSpace)]
pub struct StringData {
    pub owner: Pubkey,           // 32 bytes
    #[max_len(32)]
    pub name: String,            // 4 (length) + 32 (max chars) = 36 bytes
    #[max_len(128)]
    pub description: String,     // 4 (length) + 128 (max chars) = 132 bytes
    pub bump: u8,                // 1 byte
}

/// Array data demonstrating fixed-size array sizing
/// Space: 32 + (8*4) + (1*8) + 1 = 73 bytes (+ 8 discriminator = 81)
#[account]
#[derive(InitSpace)]
pub struct ArrayData {
    pub owner: Pubkey,       // 32 bytes
    pub values: [u64; 4],    // 8 * 4 = 32 bytes
    pub flags: [bool; 8],    // 1 * 8 = 8 bytes
    pub bump: u8,            // 1 byte
}

/// Nested struct for complex data
/// Space: 8 + 8 + 8 = 24 bytes
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, InitSpace)]
pub struct Stats {
    pub count: u64,   // 8 bytes
    pub total: u64,   // 8 bytes
    pub average: u64, // 8 bytes
}

/// Complex data with nested struct
/// Space: 32 + 24 + 1 + 1 = 58 bytes (+ 8 discriminator = 66)
#[account]
#[derive(InitSpace)]
pub struct ComplexData {
    pub owner: Pubkey,   // 32 bytes
    pub stats: Stats,    // 24 bytes
    pub is_active: bool, // 1 byte
    pub bump: u8,        // 1 byte
}

// ============================================
// Error codes
// ============================================

#[error_code]
pub enum ErrorCode {
    #[msg("Unauthorized access")]
    Unauthorized,
}
