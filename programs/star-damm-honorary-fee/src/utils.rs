use anchor_lang::prelude::*;

/// Mathematical utilities with overflow protection
pub struct MathUtil;

impl MathUtil {
    /// Safe multiplication with overflow check
    pub fn safe_mul(a: u64, b: u64) -> Result<u64> {
        a.checked_mul(b).ok_or(error!(crate::error::HonoraryFeeError::ArithmeticOverflow))
    }
    
    /// Safe division with overflow check
    pub fn safe_div(a: u64, b: u64) -> Result<u64> {
        if b == 0 {
            return Err(error!(crate::error::HonoraryFeeError::ArithmeticOverflow));
        }
        Ok(a / b)
    }
    
    /// Safe addition with overflow check
    pub fn safe_add(a: u64, b: u64) -> Result<u64> {
        a.checked_add(b).ok_or(error!(crate::error::HonoraryFeeError::ArithmeticOverflow))
    }
    
    /// Safe subtraction with underflow check
    pub fn safe_sub(a: u64, b: u64) -> Result<u64> {
        a.checked_sub(b).ok_or(error!(crate::error::HonoraryFeeError::ArithmeticOverflow))
    }
    
    /// Calculate proportional distribution using floor division
    /// Returns (payout_amount, remainder)
    pub fn calculate_proportional_payout(
        total_amount: u64,
        weight: u64,
        total_weight: u64,
    ) -> Result<(u64, u64)> {
        if total_weight == 0 {
            return Ok((0, total_amount));
        }
        
        let payout = Self::safe_div(Self::safe_mul(total_amount, weight)?, total_weight)?;
        let remainder = Self::safe_sub(total_amount, payout)?;
        
        Ok((payout, remainder))
    }
    
    /// Calculate eligible investor share based on locked percentage
    /// Returns basis points (0-10000)
    pub fn calculate_eligible_share_bps(
        locked_total: u64,
        total_allocation: u64,
        max_investor_share_bps: u16,
    ) -> Result<u16> {
        if total_allocation == 0 {
            return Ok(0);
        }
        
        // f_locked(t) = locked_total(t) / Y0
        let f_locked_bps = Self::safe_div(
            Self::safe_mul(locked_total, 10000)?,
            total_allocation
        )? as u16;
        
        // Take the minimum of investor_fee_share_bps and floor(f_locked(t) * 10000)
        Ok(f_locked_bps.min(max_investor_share_bps))
    }
    
    /// Check if 24 hours have passed since last distribution
    pub fn is_24h_elapsed(last_ts: i64, current_ts: i64) -> bool {
        current_ts >= last_ts + 86400 // 86400 seconds = 24 hours
    }
}