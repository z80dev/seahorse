//! Error Handling Patterns - Anchor Reference
//!
//! Demonstrates custom error definitions and handling patterns:
//! - Custom error enum with error codes
//! - require! macro usage patterns
//! - Authorization errors
//! - Numeric validation (range checks, overflow prevention)
//! - String length validation
//! - State validation errors
//! - Role-based access control errors
//!
//! Key patterns:
//! - Anchor uses #[error_code] attribute for custom errors
//! - Each error variant has #[msg("...")] for human-readable messages
//! - require!(condition, ErrorEnum::Variant) for validation
//!
//! Program ID matches Seahorse version for parity testing.

use anchor_lang::prelude::*;

// Same program ID as Seahorse version for parity testing
declare_id!("ErrDemo111111111111111111111111111111111111");

#[program]
pub mod errors_anchor {
    use super::*;

    /// Initialize an ErrorDemo account with validation checks
    pub fn initialize(
        ctx: Context<Initialize>,
        max_value: u64,
        name: String,
    ) -> Result<()> {
        // Input validation errors
        require!(max_value > 0, ErrorsError::MaxValueMustBePositive);
        require!(max_value <= 1_000_000, ErrorsError::MaxValueExceedsLimit);
        require!(!name.is_empty(), ErrorsError::NameCannotBeEmpty);
        require!(name.len() <= 32, ErrorsError::NameTooLong);

        let error_demo = &mut ctx.accounts.error_demo;

        error_demo.authority = ctx.accounts.authority.key();
        error_demo.value = 0;
        error_demo.operation_count = 0;
        error_demo.is_active = true;
        error_demo.name = name;
        error_demo.max_value = max_value;
        error_demo.bump = ctx.bumps.error_demo;

        msg!(
            "ErrorDemo initialized: authority={}, max_value={}",
            ctx.accounts.authority.key(),
            max_value
        );

        Ok(())
    }

    /// Set value with authorization and range validation
    pub fn set_value(ctx: Context<SetValue>, new_value: u64) -> Result<()> {
        let error_demo = &mut ctx.accounts.error_demo;

        // Authorization error
        require!(
            ctx.accounts.authority.key() == error_demo.authority,
            ErrorsError::Unauthorized
        );

        // State validation error
        require!(error_demo.is_active, ErrorsError::AccountNotActive);

        // Range validation error
        require!(new_value <= error_demo.max_value, ErrorsError::ValueExceedsMax);

        error_demo.value = new_value;
        error_demo.operation_count += 1;

        msg!("Value set to {}", new_value);

        Ok(())
    }

    /// Increment value with overflow prevention
    pub fn increment_value(ctx: Context<IncrementValue>, amount: u64) -> Result<()> {
        let error_demo = &mut ctx.accounts.error_demo;

        // Authorization check
        require!(
            ctx.accounts.authority.key() == error_demo.authority,
            ErrorsError::Unauthorized
        );

        // State validation
        require!(error_demo.is_active, ErrorsError::AccountNotActive);

        // Amount validation
        require!(amount > 0, ErrorsError::AmountMustBePositive);

        // Overflow prevention
        let new_value = error_demo
            .value
            .checked_add(amount)
            .ok_or(ErrorsError::Overflow)?;
        require!(new_value <= error_demo.max_value, ErrorsError::ValueExceedsMax);

        error_demo.value = new_value;
        error_demo.operation_count += 1;

        msg!("Value incremented by {} to {}", amount, new_value);

        Ok(())
    }

    /// Decrement value with underflow prevention
    pub fn decrement_value(ctx: Context<DecrementValue>, amount: u64) -> Result<()> {
        let error_demo = &mut ctx.accounts.error_demo;

        // Authorization check
        require!(
            ctx.accounts.authority.key() == error_demo.authority,
            ErrorsError::Unauthorized
        );

        // State validation
        require!(error_demo.is_active, ErrorsError::AccountNotActive);

        // Amount validation
        require!(amount > 0, ErrorsError::AmountMustBePositive);

        // Underflow prevention
        require!(error_demo.value >= amount, ErrorsError::Underflow);

        error_demo.value = error_demo.value - amount;
        error_demo.operation_count += 1;

        msg!("Value decremented by {} to {}", amount, error_demo.value);

        Ok(())
    }

    /// Update name with string validation
    pub fn update_name(ctx: Context<UpdateName>, new_name: String) -> Result<()> {
        let error_demo = &mut ctx.accounts.error_demo;

        // Authorization check
        require!(
            ctx.accounts.authority.key() == error_demo.authority,
            ErrorsError::Unauthorized
        );

        // State validation
        require!(error_demo.is_active, ErrorsError::AccountNotActive);

        // String validation
        require!(!new_name.is_empty(), ErrorsError::NameCannotBeEmpty);
        require!(new_name.len() <= 32, ErrorsError::NameTooLong);

        error_demo.name = new_name.clone();
        error_demo.operation_count += 1;

        msg!("Name updated to: {}", new_name);

        Ok(())
    }

    /// Deactivate the account (can only be done once when active)
    pub fn deactivate(ctx: Context<Deactivate>) -> Result<()> {
        let error_demo = &mut ctx.accounts.error_demo;

        // Authorization check
        require!(
            ctx.accounts.authority.key() == error_demo.authority,
            ErrorsError::Unauthorized
        );

        // State transition validation
        require!(error_demo.is_active, ErrorsError::AlreadyDeactivated);

        error_demo.is_active = false;
        error_demo.operation_count += 1;

        msg!("Account deactivated");

        Ok(())
    }

    /// Reactivate the account (can only be done when inactive)
    pub fn reactivate(ctx: Context<Reactivate>) -> Result<()> {
        let error_demo = &mut ctx.accounts.error_demo;

        // Authorization check
        require!(
            ctx.accounts.authority.key() == error_demo.authority,
            ErrorsError::Unauthorized
        );

        // State transition validation
        require!(!error_demo.is_active, ErrorsError::AlreadyActive);

        error_demo.is_active = true;
        error_demo.operation_count += 1;

        msg!("Account reactivated");

        Ok(())
    }

    /// Initialize a role registry with admin
    pub fn initialize_role_registry(
        ctx: Context<InitializeRoleRegistry>,
        registry_id: u64,
    ) -> Result<()> {
        let registry = &mut ctx.accounts.registry;

        registry.admin = ctx.accounts.admin.key();
        registry.operator = ctx.accounts.admin.key(); // Default operator to admin
        registry.has_operator = false;
        registry.registry_id = registry_id;
        registry.bump = ctx.bumps.registry;

        msg!(
            "RoleRegistry {} initialized with admin {}",
            registry_id,
            ctx.accounts.admin.key()
        );

        Ok(())
    }

    /// Set operator role (admin only)
    pub fn set_operator(ctx: Context<SetOperator>, new_operator: Pubkey) -> Result<()> {
        let registry = &mut ctx.accounts.registry;

        // Admin-only check
        require!(
            ctx.accounts.admin.key() == registry.admin,
            ErrorsError::AdminRequired
        );

        registry.operator = new_operator;
        registry.has_operator = true;

        msg!("Operator set to {}", new_operator);

        Ok(())
    }

    /// Action that requires admin role
    pub fn admin_only_action(ctx: Context<AdminOnlyAction>) -> Result<()> {
        let registry = &ctx.accounts.registry;

        // Admin-only check
        require!(
            ctx.accounts.caller.key() == registry.admin,
            ErrorsError::AdminRequired
        );

        msg!("Admin action executed by {}", ctx.accounts.caller.key());

        Ok(())
    }

    /// Action that requires operator or admin role
    pub fn operator_action(ctx: Context<OperatorAction>) -> Result<()> {
        let registry = &ctx.accounts.registry;

        // Must have operator set
        require!(registry.has_operator, ErrorsError::NoOperatorSet);

        // Operator or admin check
        let is_admin = ctx.accounts.caller.key() == registry.admin;
        let is_operator = ctx.accounts.caller.key() == registry.operator;
        require!(
            is_admin || is_operator,
            ErrorsError::OperatorOrAdminRequired
        );

        msg!("Operator action executed by {}", ctx.accounts.caller.key());

        Ok(())
    }

    /// Demonstrate multiple validation checks in sequence
    pub fn complex_validation(
        ctx: Context<ComplexValidation>,
        new_value: u64,
        new_name: String,
        require_high_value: bool,
    ) -> Result<()> {
        let error_demo = &mut ctx.accounts.error_demo;

        // Authorization (first check)
        require!(
            ctx.accounts.authority.key() == error_demo.authority,
            ErrorsError::Unauthorized
        );

        // State check (second check)
        require!(error_demo.is_active, ErrorsError::AccountNotActive);

        // Numeric validations
        require!(new_value > 0, ErrorsError::ValueMustBePositive);
        require!(new_value <= error_demo.max_value, ErrorsError::ValueExceedsMax);

        // Conditional validation
        if require_high_value {
            require!(
                new_value >= error_demo.max_value / 2,
                ErrorsError::ValueTooLowForHighRequirement
            );
        }

        // String validations
        require!(!new_name.is_empty(), ErrorsError::NameCannotBeEmpty);
        require!(new_name.len() <= 32, ErrorsError::NameTooLong);

        // Apply changes
        error_demo.value = new_value;
        error_demo.name = new_name.clone();
        error_demo.operation_count += 1;

        msg!(
            "Complex validation passed: value={}, name={}",
            new_value,
            new_name
        );

        Ok(())
    }

    /// Get ErrorDemo info (read-only, for logging)
    pub fn get_demo_info(ctx: Context<GetDemoInfo>) -> Result<()> {
        let error_demo = &ctx.accounts.error_demo;

        msg!("Authority: {}", error_demo.authority);
        msg!("Value: {}", error_demo.value);
        msg!("Max Value: {}", error_demo.max_value);
        msg!("Name: {}", error_demo.name);
        msg!("Is Active: {}", error_demo.is_active);
        msg!("Operation Count: {}", error_demo.operation_count);

        Ok(())
    }
}

// =============================================================================
// ACCOUNT STRUCTURES
// =============================================================================

#[account]
pub struct ErrorDemo {
    /// Authority who can modify this account
    pub authority: Pubkey,
    /// Current value for numeric tests
    pub value: u64,
    /// Counter for operations
    pub operation_count: u64,
    /// Status flag
    pub is_active: bool,
    /// Name field for string validation
    pub name: String,
    /// Maximum allowed value (for range checking)
    pub max_value: u64,
    /// PDA bump seed
    pub bump: u8,
}

#[account]
pub struct RoleRegistry {
    /// Account authority (admin)
    pub admin: Pubkey,
    /// Registered operator (secondary role)
    pub operator: Pubkey,
    /// Whether operator is set
    pub has_operator: bool,
    /// Role registry ID
    pub registry_id: u64,
    /// PDA bump seed
    pub bump: u8,
}

// =============================================================================
// INSTRUCTION CONTEXTS
// =============================================================================

#[derive(Accounts)]
#[instruction(max_value: u64, name: String)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 8 + 8 + 1 + (4 + 32) + 8 + 1 + 100, // discriminator + authority + value + op_count + is_active + name + max_value + bump + padding
        seeds = [b"error_demo", authority.key().as_ref()],
        bump
    )]
    pub error_demo: Account<'info, ErrorDemo>,

    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetValue<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"error_demo", error_demo.authority.as_ref()],
        bump = error_demo.bump
    )]
    pub error_demo: Account<'info, ErrorDemo>,
}

#[derive(Accounts)]
pub struct IncrementValue<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"error_demo", error_demo.authority.as_ref()],
        bump = error_demo.bump
    )]
    pub error_demo: Account<'info, ErrorDemo>,
}

#[derive(Accounts)]
pub struct DecrementValue<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"error_demo", error_demo.authority.as_ref()],
        bump = error_demo.bump
    )]
    pub error_demo: Account<'info, ErrorDemo>,
}

#[derive(Accounts)]
pub struct UpdateName<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"error_demo", error_demo.authority.as_ref()],
        bump = error_demo.bump
    )]
    pub error_demo: Account<'info, ErrorDemo>,
}

#[derive(Accounts)]
pub struct Deactivate<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"error_demo", error_demo.authority.as_ref()],
        bump = error_demo.bump
    )]
    pub error_demo: Account<'info, ErrorDemo>,
}

#[derive(Accounts)]
pub struct Reactivate<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"error_demo", error_demo.authority.as_ref()],
        bump = error_demo.bump
    )]
    pub error_demo: Account<'info, ErrorDemo>,
}

#[derive(Accounts)]
#[instruction(registry_id: u64)]
pub struct InitializeRoleRegistry<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + 32 + 32 + 1 + 8 + 1, // discriminator + admin + operator + has_operator + registry_id + bump
        seeds = [b"role_registry", registry_id.to_le_bytes().as_ref()],
        bump
    )]
    pub registry: Account<'info, RoleRegistry>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetOperator<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [b"role_registry", registry.registry_id.to_le_bytes().as_ref()],
        bump = registry.bump
    )]
    pub registry: Account<'info, RoleRegistry>,
}

#[derive(Accounts)]
pub struct AdminOnlyAction<'info> {
    pub caller: Signer<'info>,

    #[account(
        seeds = [b"role_registry", registry.registry_id.to_le_bytes().as_ref()],
        bump = registry.bump
    )]
    pub registry: Account<'info, RoleRegistry>,
}

#[derive(Accounts)]
pub struct OperatorAction<'info> {
    pub caller: Signer<'info>,

    #[account(
        seeds = [b"role_registry", registry.registry_id.to_le_bytes().as_ref()],
        bump = registry.bump
    )]
    pub registry: Account<'info, RoleRegistry>,
}

#[derive(Accounts)]
pub struct ComplexValidation<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"error_demo", error_demo.authority.as_ref()],
        bump = error_demo.bump
    )]
    pub error_demo: Account<'info, ErrorDemo>,
}

#[derive(Accounts)]
pub struct GetDemoInfo<'info> {
    pub caller: Signer<'info>,

    pub error_demo: Account<'info, ErrorDemo>,
}

// =============================================================================
// ERROR CODES
// =============================================================================

#[error_code]
pub enum ErrorsError {
    // Initialization errors
    #[msg("Max value must be greater than zero")]
    MaxValueMustBePositive,
    #[msg("Max value exceeds maximum allowed (1,000,000)")]
    MaxValueExceedsLimit,

    // String validation errors
    #[msg("Name cannot be empty")]
    NameCannotBeEmpty,
    #[msg("Name exceeds maximum length of 32 characters")]
    NameTooLong,

    // Authorization errors
    #[msg("Unauthorized: only authority can perform this action")]
    Unauthorized,
    #[msg("Unauthorized: admin role required")]
    AdminRequired,
    #[msg("Unauthorized: operator or admin role required")]
    OperatorOrAdminRequired,
    #[msg("No operator has been set")]
    NoOperatorSet,

    // State validation errors
    #[msg("Account is not active")]
    AccountNotActive,
    #[msg("Account is already deactivated")]
    AlreadyDeactivated,
    #[msg("Account is already active")]
    AlreadyActive,

    // Numeric validation errors
    #[msg("Value exceeds maximum allowed")]
    ValueExceedsMax,
    #[msg("Amount must be greater than zero")]
    AmountMustBePositive,
    #[msg("Value must be greater than zero")]
    ValueMustBePositive,
    #[msg("Overflow: operation would overflow")]
    Overflow,
    #[msg("Underflow: value would go below zero")]
    Underflow,

    // Conditional validation errors
    #[msg("Value must be at least half of max when high value required")]
    ValueTooLowForHighRequirement,
}
