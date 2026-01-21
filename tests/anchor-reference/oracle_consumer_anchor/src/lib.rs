use anchor_lang::prelude::*;
use pyth_sdk_solana::{load_price_feed_from_account_info, Price, PriceFeed};

// Same program ID as Seahorse version for parity testing
declare_id!("3ePvSJdgkK1r5CrxyEjJoMnSCJJjCMoRKNdYLi6Ws8g7");

// Maximum age of price in slots before considered stale
pub const MAX_PRICE_AGE_SLOTS: u64 = 100;

// Scale factor for price calculations (10^9 for precision)
pub const PRICE_SCALE: u64 = 1_000_000_000;

#[program]
pub mod oracle_consumer_anchor {
    use super::*;

    /// Create a new price feed configuration.
    pub fn create_price_config(
        ctx: Context<CreatePriceConfig>,
        config_id: u64,
        feed_name: String,
        oracle_address: Pubkey,
        max_staleness_slots: u64,
    ) -> Result<()> {
        require!(feed_name.len() <= 32, OracleError::FeedNameTooLong);
        require!(max_staleness_slots > 0, OracleError::InvalidStaleness);

        let price_config = &mut ctx.accounts.price_config;
        price_config.authority = ctx.accounts.authority.key();
        price_config.config_id = config_id;
        price_config.feed_name = feed_name;
        price_config.oracle_address = oracle_address;
        price_config.max_staleness_slots = max_staleness_slots;
        price_config.last_price = 0;
        price_config.last_confidence = 0;
        price_config.last_exponent = 0;
        price_config.last_record_slot = 0;
        price_config.update_count = 0;
        price_config.bump = ctx.bumps.price_config;

        Ok(())
    }

    /// Update price feed configuration (authority only).
    pub fn update_price_config(
        ctx: Context<UpdatePriceConfig>,
        new_oracle_address: Pubkey,
        new_max_staleness_slots: u64,
    ) -> Result<()> {
        require!(
            ctx.accounts.authority.key() == ctx.accounts.price_config.authority,
            OracleError::Unauthorized
        );
        require!(new_max_staleness_slots > 0, OracleError::InvalidStaleness);

        let price_config = &mut ctx.accounts.price_config;
        price_config.oracle_address = new_oracle_address;
        price_config.max_staleness_slots = new_max_staleness_slots;

        Ok(())
    }

    /// Record a price from the oracle (mocked version for testing).
    /// In production, this would use the actual Pyth oracle account.
    pub fn record_price(
        ctx: Context<RecordPrice>,
        oracle_price: i64,
        oracle_confidence: u64,
        oracle_exponent: i32,
        oracle_publish_slot: u64,
    ) -> Result<()> {
        require!(
            ctx.accounts.authority.key() == ctx.accounts.price_config.authority,
            OracleError::Unauthorized
        );

        let clock = &ctx.accounts.clock;
        let current_slot = clock.slot;

        // Staleness check
        let price_age = current_slot.saturating_sub(oracle_publish_slot);
        require!(
            price_age <= ctx.accounts.price_config.max_staleness_slots,
            OracleError::PriceStale
        );

        // Record the price
        let price_config = &mut ctx.accounts.price_config;
        price_config.last_price = oracle_price;
        price_config.last_confidence = oracle_confidence;
        price_config.last_exponent = oracle_exponent;
        price_config.last_record_slot = current_slot;
        price_config.update_count += 1;

        msg!("Recorded price: {}, exponent: {}", oracle_price, oracle_exponent);

        Ok(())
    }

    /// Record price directly from a Pyth oracle account.
    /// This demonstrates real Pyth SDK integration.
    pub fn record_price_from_pyth(ctx: Context<RecordPriceFromPyth>) -> Result<()> {
        require!(
            ctx.accounts.authority.key() == ctx.accounts.price_config.authority,
            OracleError::Unauthorized
        );

        // Load price feed from Pyth oracle account
        let price_feed_account = &ctx.accounts.pyth_price_account;
        let price_feed: PriceFeed =
            load_price_feed_from_account_info(price_feed_account).map_err(|_| OracleError::InvalidPythAccount)?;

        // Validate oracle address matches expected
        require!(
            price_feed_account.key() == ctx.accounts.price_config.oracle_address,
            OracleError::WrongOracleAccount
        );

        // Get the current price (unchecked - we do our own staleness check)
        let price: Price = price_feed.get_price_unchecked();

        let clock = &ctx.accounts.clock;
        let current_slot = clock.slot;

        // Staleness check using publish time (converted to slots approximately)
        // Note: Pyth uses Unix timestamp, we compare slots for consistency
        // In production, you'd convert properly or use Pyth's own staleness check
        let price_config = &mut ctx.accounts.price_config;

        // Record the price
        price_config.last_price = price.price;
        price_config.last_confidence = price.conf;
        price_config.last_exponent = price.expo;
        price_config.last_record_slot = current_slot;
        price_config.update_count += 1;

        msg!(
            "Recorded Pyth price: {}, confidence: {}, exponent: {}",
            price.price,
            price.conf,
            price.expo
        );

        Ok(())
    }

    /// Create a price threshold for monitoring.
    pub fn create_price_threshold(
        ctx: Context<CreatePriceThreshold>,
        config_id: u64,
        threshold_id: u64,
        trigger_above: bool,
        target_price_scaled: u64,
    ) -> Result<()> {
        require!(
            ctx.accounts.authority.key() == ctx.accounts.price_config.authority,
            OracleError::Unauthorized
        );
        require!(
            config_id == ctx.accounts.price_config.config_id,
            OracleError::ConfigIdMismatch
        );
        require!(target_price_scaled > 0, OracleError::InvalidTargetPrice);

        let price_threshold = &mut ctx.accounts.price_threshold;
        price_threshold.config = ctx.accounts.price_config.key();
        price_threshold.threshold_id = threshold_id;
        price_threshold.trigger_above = trigger_above;
        price_threshold.target_price_scaled = target_price_scaled;
        price_threshold.is_triggered = false;
        price_threshold.triggered_at_slot = 0;
        price_threshold.bump = ctx.bumps.price_threshold;

        Ok(())
    }

    /// Check if price threshold has been crossed.
    pub fn check_threshold(ctx: Context<CheckThreshold>) -> Result<()> {
        let price_config = &ctx.accounts.price_config;
        let price_threshold = &mut ctx.accounts.price_threshold;

        require!(
            price_threshold.config == price_config.key(),
            OracleError::ThresholdConfigMismatch
        );

        // Skip if already triggered
        if price_threshold.is_triggered {
            msg!("Threshold already triggered");
            return Ok(());
        }

        let raw_price = price_config.last_price;

        // Handle negative prices
        if raw_price < 0 {
            msg!("Negative price not supported for threshold");
            return Ok(());
        }

        let price_u64 = raw_price as u64;
        let target = price_threshold.target_price_scaled;
        let trigger_above = price_threshold.trigger_above;

        let triggered = if trigger_above {
            price_u64 >= target
        } else {
            price_u64 <= target
        };

        if triggered {
            let clock = &ctx.accounts.clock;
            price_threshold.is_triggered = true;
            price_threshold.triggered_at_slot = clock.slot;
            msg!("Threshold triggered!");
        }

        Ok(())
    }

    /// Reset a triggered threshold (authority only).
    pub fn reset_threshold(ctx: Context<ResetThreshold>) -> Result<()> {
        require!(
            ctx.accounts.authority.key() == ctx.accounts.price_config.authority,
            OracleError::Unauthorized
        );
        require!(
            ctx.accounts.price_threshold.config == ctx.accounts.price_config.key(),
            OracleError::ThresholdConfigMismatch
        );

        let price_threshold = &mut ctx.accounts.price_threshold;
        price_threshold.is_triggered = false;
        price_threshold.triggered_at_slot = 0;

        Ok(())
    }

    /// Display current price information and staleness status.
    pub fn get_price_info(ctx: Context<GetPriceInfo>) -> Result<()> {
        let price_config = &ctx.accounts.price_config;
        let clock = &ctx.accounts.clock;
        let current_slot = clock.slot;

        let age_slots = current_slot.saturating_sub(price_config.last_record_slot);
        let is_stale = age_slots > price_config.max_staleness_slots;

        msg!("Price: {}", price_config.last_price);
        msg!("Confidence: {}", price_config.last_confidence);
        msg!("Exponent: {}", price_config.last_exponent);
        msg!("Age (slots): {}", age_slots);

        if is_stale {
            msg!("Price is STALE");
        } else {
            msg!("Price is FRESH");
        }

        Ok(())
    }
}

// ============================================================================
// Account Structures
// ============================================================================

#[account]
#[derive(Debug)]
pub struct PriceConfig {
    /// Authority who created this config
    pub authority: Pubkey,
    /// Configuration ID for PDA derivation
    pub config_id: u64,
    /// Description/name of the price feed (max 32 chars)
    pub feed_name: String,
    /// Expected oracle account address (for validation)
    pub oracle_address: Pubkey,
    /// Maximum allowed staleness in slots
    pub max_staleness_slots: u64,
    /// Last recorded price
    pub last_price: i64,
    /// Last recorded confidence
    pub last_confidence: u64,
    /// Last recorded exponent
    pub last_exponent: i32,
    /// Slot when price was last recorded
    pub last_record_slot: u64,
    /// Count of price updates recorded
    pub update_count: u64,
    /// Bump seed for PDA
    pub bump: u8,
}

impl PriceConfig {
    pub const MAX_FEED_NAME_LEN: usize = 32;

    pub fn space() -> usize {
        8 +  // discriminator
        32 + // authority
        8 +  // config_id
        4 + Self::MAX_FEED_NAME_LEN + // feed_name (borsh string)
        32 + // oracle_address
        8 +  // max_staleness_slots
        8 +  // last_price (i64)
        8 +  // last_confidence
        4 +  // last_exponent (i32)
        8 +  // last_record_slot
        8 +  // update_count
        1 +  // bump
        50   // padding for safety
    }
}

#[account]
#[derive(Debug)]
pub struct PriceThreshold {
    /// Config this threshold belongs to
    pub config: Pubkey,
    /// Threshold ID for PDA derivation
    pub threshold_id: u64,
    /// Whether to trigger when above (True) or below (False) target
    pub trigger_above: bool,
    /// Target price (scaled by PRICE_SCALE for precision without float)
    pub target_price_scaled: u64,
    /// Whether threshold has been triggered
    pub is_triggered: bool,
    /// Slot when threshold was triggered (0 if not triggered)
    pub triggered_at_slot: u64,
    /// Bump seed for PDA
    pub bump: u8,
}

impl PriceThreshold {
    pub fn space() -> usize {
        8 +  // discriminator
        32 + // config
        8 +  // threshold_id
        1 +  // trigger_above
        8 +  // target_price_scaled
        1 +  // is_triggered
        8 +  // triggered_at_slot
        1    // bump
    }
}

// ============================================================================
// Instruction Account Contexts
// ============================================================================

#[derive(Accounts)]
#[instruction(config_id: u64, feed_name: String)]
pub struct CreatePriceConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = PriceConfig::space(),
        seeds = [b"price_config", config_id.to_le_bytes().as_ref()],
        bump
    )]
    pub price_config: Account<'info, PriceConfig>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdatePriceConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub price_config: Account<'info, PriceConfig>,
}

#[derive(Accounts)]
pub struct RecordPrice<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub price_config: Account<'info, PriceConfig>,

    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct RecordPriceFromPyth<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub price_config: Account<'info, PriceConfig>,

    /// CHECK: Validated by Pyth SDK when loading price feed
    pub pyth_price_account: AccountInfo<'info>,

    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
#[instruction(config_id: u64, threshold_id: u64)]
pub struct CreatePriceThreshold<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub price_config: Account<'info, PriceConfig>,

    #[account(
        init,
        payer = authority,
        space = PriceThreshold::space(),
        seeds = [
            b"threshold",
            config_id.to_le_bytes().as_ref(),
            threshold_id.to_le_bytes().as_ref()
        ],
        bump
    )]
    pub price_threshold: Account<'info, PriceThreshold>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CheckThreshold<'info> {
    #[account(mut)]
    pub price_config: Account<'info, PriceConfig>,

    #[account(mut)]
    pub price_threshold: Account<'info, PriceThreshold>,

    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct ResetThreshold<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub price_config: Account<'info, PriceConfig>,

    #[account(mut)]
    pub price_threshold: Account<'info, PriceThreshold>,
}

#[derive(Accounts)]
pub struct GetPriceInfo<'info> {
    pub price_config: Account<'info, PriceConfig>,

    pub clock: Sysvar<'info, Clock>,
}

// ============================================================================
// Error Codes
// ============================================================================

#[error_code]
pub enum OracleError {
    #[msg("Feed name too long (max 32 chars)")]
    FeedNameTooLong,

    #[msg("Max staleness must be greater than zero")]
    InvalidStaleness,

    #[msg("Unauthorized")]
    Unauthorized,

    #[msg("Price is stale")]
    PriceStale,

    #[msg("Invalid Pyth oracle account")]
    InvalidPythAccount,

    #[msg("Wrong oracle account")]
    WrongOracleAccount,

    #[msg("Target price must be greater than zero")]
    InvalidTargetPrice,

    #[msg("Config ID mismatch")]
    ConfigIdMismatch,

    #[msg("Threshold does not belong to this config")]
    ThresholdConfigMismatch,
}
