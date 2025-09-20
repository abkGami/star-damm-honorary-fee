use anchor_lang::prelude::*;
use crate::{
    error::HonoraryFeeError,
    utils::MathUtil,
};

/// Enhanced mathematical utilities with comprehensive error handling
pub struct EnhancedMathUtil;

impl EnhancedMathUtil {
    /// Calculate proportional distribution with dust handling and carry forward
    /// Returns (individual_payouts, total_paid, dust_remainder)
    pub fn calculate_proportional_distribution_with_dust(
        total_amount: u64,
        weights: &[u64],
        min_payout_threshold: u64,
    ) -> Result<(Vec<u64>, u64, u64)> {
        if weights.is_empty() {
            return Ok((Vec::new(), 0, total_amount));
        }
        
        let total_weight: u64 = weights.iter().sum();
        if total_weight == 0 {
            return Ok((vec![0; weights.len()], 0, total_amount));
        }
        
        let mut payouts = Vec::with_capacity(weights.len());
        let mut total_paid = 0u64;
        let mut dust_accumulator = 0u64;
        
        for &weight in weights {
            // Calculate proportional amount
            let raw_payout = MathUtil::safe_div(
                MathUtil::safe_mul(total_amount, weight)?,
                total_weight
            )?;
            
            // Apply minimum threshold
            let actual_payout = if raw_payout >= min_payout_threshold {
                raw_payout
            } else {
                // Below threshold - accumulate as dust
                dust_accumulator = MathUtil::safe_add(dust_accumulator, raw_payout)?;
                0
            };
            
            payouts.push(actual_payout);
            total_paid = MathUtil::safe_add(total_paid, actual_payout)?;
        }
        
        // Calculate total dust (includes sub-threshold amounts + rounding remainder)
        let total_dust = MathUtil::safe_add(
            dust_accumulator,
            MathUtil::safe_sub(total_amount, total_paid)?
        )?;
        
        Ok((payouts, total_paid, total_dust))
    }
    
    /// Apply daily cap with proper remainder calculation
    pub fn apply_daily_cap(
        requested_amount: u64,
        daily_cap: u64,
        already_distributed: u64,
    ) -> Result<(u64, u64)> {
        if daily_cap == 0 {
            return Ok((requested_amount, 0));
        }
        
        let remaining_cap = daily_cap.saturating_sub(already_distributed);
        let capped_amount = requested_amount.min(remaining_cap);
        let excess = MathUtil::safe_sub(requested_amount, capped_amount)?;
        
        Ok((capped_amount, excess))
    }
    
    /// Validate distribution invariants
    pub fn validate_distribution_invariants(
        total_claimed: u64,
        total_distributed: u64,
        carry_over: u64,
        creator_remainder: u64,
    ) -> Result<()> {
        let total_accounted = MathUtil::safe_add(
            MathUtil::safe_add(total_distributed, carry_over)?,
            creator_remainder
        )?;
        
        require!(
            total_accounted <= total_claimed,
            HonoraryFeeError::ArithmeticOverflow
        );
        
        Ok(())
    }
}

/// Comprehensive validation utilities
pub struct ValidationUtil;

impl ValidationUtil {
    /// Validate investor account format and accessibility
    pub fn validate_investor_accounts(remaining_accounts: &[AccountInfo]) -> Result<()> {
        // Must have even number of accounts (stream + ATA pairs)
        require!(
            remaining_accounts.len() % 2 == 0,
            HonoraryFeeError::InvalidStreamAccount
        );
        
        // Validate each pair
        for chunk in remaining_accounts.chunks(2) {
            let stream_account = &chunk[0];
            let investor_ata = &chunk[1];
            
            // Basic account validation
            require!(
                stream_account.data_is_empty() == false,
                HonoraryFeeError::InvalidStreamAccount
            );
            
            require!(
                investor_ata.data_is_empty() == false,
                HonoraryFeeError::InvalidTreasury
            );
            
            msg!("Validated investor pair: stream={}, ata={}", 
                 stream_account.key, investor_ata.key);
        }
        
        Ok(())
    }
    
    /// Validate timing constraints
    pub fn validate_timing_constraints(
        last_distribution_ts: i64,
        current_ts: i64,
        day_complete: bool,
    ) -> Result<bool> {
        let is_new_day = !day_complete || 
            MathUtil::is_24h_elapsed(last_distribution_ts, current_ts);
        
        if is_new_day && !MathUtil::is_24h_elapsed(last_distribution_ts, current_ts) {
            return Err(error!(HonoraryFeeError::CooldownNotElapsed));
        }
        
        Ok(is_new_day)
    }
    
    /// Validate pagination cursor
    pub fn validate_pagination_cursor(
        cursor: u64,
        total_accounts: usize,
        page_size: u64,
    ) -> Result<()> {
        require!(
            cursor < total_accounts as u64,
            HonoraryFeeError::InvalidPaginationCursor
        );
        
        require!(
            page_size > 0 && page_size <= 100, // Reasonable limits
            HonoraryFeeError::InvalidPaginationCursor
        );
        
        Ok(())
    }
}

/// Safe account parsing utilities
pub struct AccountParser;

impl AccountParser {
    /// Safely parse Streamflow stream account
    pub fn parse_streamflow_stream(account_info: &AccountInfo) -> Result<StreamflowStreamData> {
        require!(
            account_info.data_len() >= StreamflowStreamData::MIN_LEN,
            HonoraryFeeError::InvalidStreamAccount
        );
        
        let data = account_info.try_borrow_data()?;
        
        // Parse based on Streamflow's actual format
        // This is a placeholder - would need actual Streamflow parsing
        let locked_amount = Self::extract_locked_amount(&data)?;
        let recipient = Self::extract_recipient(&data)?;
        
        Ok(StreamflowStreamData {
            locked_amount,
            recipient,
            stream_pubkey: account_info.key(),
        })
    }
    
    fn extract_locked_amount(data: &[u8]) -> Result<u64> {
        // Placeholder for actual Streamflow parsing logic
        if data.len() < 8 {
            return Err(error!(HonoraryFeeError::InvalidStreamAccount));
        }
        
        // Would parse actual locked amount from Streamflow format
        Ok(1000000) // Mock value
    }
    
    fn extract_recipient(data: &[u8]) -> Result<Pubkey> {
        // Placeholder for actual recipient extraction
        if data.len() < 40 {
            return Err(error!(HonoraryFeeError::InvalidStreamAccount));
        }
        
        // Would extract actual recipient pubkey
        Ok(Pubkey::default()) // Mock value
    }
}

/// Streamflow stream data structure
#[derive(Clone, Debug)]
pub struct StreamflowStreamData {
    pub locked_amount: u64,
    pub recipient: Pubkey,
    pub stream_pubkey: Pubkey,
}

impl StreamflowStreamData {
    pub const MIN_LEN: usize = 100; // Minimum expected size for Streamflow stream
}