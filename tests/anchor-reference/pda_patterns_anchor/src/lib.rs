use anchor_lang::prelude::*;

declare_id!("DxpKiPEUYHy6NKsnEMRvRKtC3ncrtP8epGt5KKS59Xzh");

#[program]
pub mod pda_patterns_anchor {
    use super::*;

    /// Initialize a global config PDA with a single seed
    pub fn init_global_config(ctx: Context<InitGlobalConfig>) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.admin = ctx.accounts.admin.key();
        config.counter = 0;
        config.bump = ctx.bumps.config;
        Ok(())
    }

    /// Update the global config counter
    pub fn update_global_config(ctx: Context<UpdateGlobalConfig>) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.counter = config.counter.checked_add(1).ok_or(ErrorCode::Overflow)?;
        Ok(())
    }

    /// Initialize a user profile PDA with user-derived seeds
    pub fn init_user_profile(ctx: Context<InitUserProfile>, name: String) -> Result<()> {
        let profile = &mut ctx.accounts.profile;
        profile.owner = ctx.accounts.user.key();
        profile.name = name;
        profile.visits = 0;
        profile.bump = ctx.bumps.profile;
        Ok(())
    }

    /// Increment user profile visits
    pub fn visit_profile(ctx: Context<VisitProfile>) -> Result<()> {
        let profile = &mut ctx.accounts.profile;
        profile.visits = profile.visits.checked_add(1).ok_or(ErrorCode::Overflow)?;
        Ok(())
    }

    /// Initialize a data record with multiple seeds (category + id)
    pub fn init_data_record(
        ctx: Context<InitDataRecord>,
        category: String,
        record_id: u64,
        data: String,
    ) -> Result<()> {
        let record = &mut ctx.accounts.record;
        record.creator = ctx.accounts.creator.key();
        record.category = category;
        record.record_id = record_id;
        record.data = data;
        record.bump = ctx.bumps.record;
        Ok(())
    }

    /// Update a data record
    pub fn update_data_record(ctx: Context<UpdateDataRecord>, data: String) -> Result<()> {
        let record = &mut ctx.accounts.record;
        record.data = data;
        Ok(())
    }

    /// Initialize a vault PDA that can sign (demonstrates PDA signing capability)
    pub fn init_vault(ctx: Context<InitVault>, vault_id: u64) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.owner = ctx.accounts.owner.key();
        vault.vault_id = vault_id;
        vault.balance = 0;
        vault.bump = ctx.bumps.vault;
        Ok(())
    }

    /// Deposit to vault (demonstrates reading bump from account)
    pub fn deposit_to_vault(ctx: Context<DepositToVault>, vault_id: u64, amount: u64) -> Result<()> {
        // vault_id is used for PDA verification in accounts struct, not logic
        let _ = vault_id;
        let vault = &mut ctx.accounts.vault;
        vault.balance = vault.balance.checked_add(amount).ok_or(ErrorCode::Overflow)?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitGlobalConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + GlobalConfig::INIT_SPACE,
        seeds = [b"global_config"],
        bump
    )]
    pub config: Account<'info, GlobalConfig>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateGlobalConfig<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [b"global_config"],
        bump = config.bump,
        has_one = admin
    )]
    pub config: Account<'info, GlobalConfig>,
}

#[derive(Accounts)]
pub struct InitUserProfile<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        payer = user,
        space = 8 + UserProfile::INIT_SPACE,
        seeds = [b"user_profile", user.key().as_ref()],
        bump
    )]
    pub profile: Account<'info, UserProfile>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct VisitProfile<'info> {
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"user_profile", user.key().as_ref()],
        bump = profile.bump,
        has_one = owner @ ErrorCode::Unauthorized
    )]
    pub profile: Account<'info, UserProfile>,

    /// CHECK: The owner who can modify the profile
    pub owner: UncheckedAccount<'info>,
}

#[derive(Accounts)]
#[instruction(category: String, record_id: u64)]
pub struct InitDataRecord<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        init,
        payer = creator,
        space = 8 + DataRecord::INIT_SPACE,
        seeds = [b"data_record", category.as_bytes(), &record_id.to_le_bytes()],
        bump
    )]
    pub record: Account<'info, DataRecord>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(category: String, record_id: u64)]
pub struct UpdateDataRecord<'info> {
    pub creator: Signer<'info>,

    #[account(
        mut,
        seeds = [b"data_record", category.as_bytes(), &record_id.to_le_bytes()],
        bump = record.bump,
        has_one = creator @ ErrorCode::Unauthorized
    )]
    pub record: Account<'info, DataRecord>,
}

#[derive(Accounts)]
#[instruction(vault_id: u64)]
pub struct InitVault<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        space = 8 + Vault::INIT_SPACE,
        seeds = [b"vault", owner.key().as_ref(), &vault_id.to_le_bytes()],
        bump
    )]
    pub vault: Account<'info, Vault>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(vault_id: u64)]
pub struct DepositToVault<'info> {
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [b"vault", owner.key().as_ref(), &vault_id.to_le_bytes()],
        bump = vault.bump,
        has_one = owner @ ErrorCode::Unauthorized
    )]
    pub vault: Account<'info, Vault>,
}

#[account]
#[derive(InitSpace)]
pub struct GlobalConfig {
    pub admin: Pubkey,
    pub counter: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct UserProfile {
    pub owner: Pubkey,
    #[max_len(32)]
    pub name: String,
    pub visits: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct DataRecord {
    pub creator: Pubkey,
    #[max_len(32)]
    pub category: String,
    pub record_id: u64,
    #[max_len(256)]
    pub data: String,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub owner: Pubkey,
    pub vault_id: u64,
    pub balance: u64,
    pub bump: u8,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Operation would overflow")]
    Overflow,
    #[msg("Unauthorized access")]
    Unauthorized,
}
