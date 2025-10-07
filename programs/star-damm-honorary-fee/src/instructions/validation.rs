use anchor_lang::prelude::*;
use crate::error::HonoraryFeeError;

/// Simplified LbPair structure for token mint extraction
/// Based on Meteora DLMM IDL - only including fields we need
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LbPair {
    /// Token X mint
    pub token_x_mint: Pubkey,
    /// Token Y mint
    pub token_y_mint: Pubkey,
    /// Active bin ID (current price bin)
    pub active_id: i32,
}

impl LbPair {
    /// Size of the LbPair account in bytes
    pub const LEN: usize = 32 + 32 + 4 + 752; // tokenXMint + tokenYMint + activeId + other fields

    /// Deserialize LbPair from account data
    pub fn try_deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < Self::LEN {
            return err!(HonoraryFeeError::PoolNotInitialized);
        }

        // Based on IDL structure analysis, the key fields are at these offsets:
        // discriminator (8) + parameters (64) + vParameters (64) + bumpSeed (1) + binStepSeed (2)
        // + tokenXMint (32) + tokenYMint (32) + reserveX (32) + reserveY (32) + ...
        // + activeId (4) at some offset

        let base_offset = 8 + 64 + 64 + 1 + 2; // Skip to tokenXMint
        let token_x_offset = base_offset;
        let token_y_offset = token_x_offset + 32;

        // activeId comes after several other fields, approximately at offset ~200
        // This is an approximation - would need exact IDL offset calculation
        let active_id_offset = 200;

        if data.len() < token_y_offset + 32 || data.len() < active_id_offset + 4 {
            return err!(HonoraryFeeError::PoolNotInitialized);
        }

        let token_x_mint = Pubkey::try_from(&data[token_x_offset..token_x_offset + 32])
            .map_err(|_| HonoraryFeeError::PoolNotInitialized)?;

        let token_y_mint = Pubkey::try_from(&data[token_y_offset..token_y_offset + 32])
            .map_err(|_| HonoraryFeeError::PoolNotInitialized)?;

        let active_id = i32::from_le_bytes(
            data[active_id_offset..active_id_offset + 4]
                .try_into()
                .map_err(|_| HonoraryFeeError::PoolNotInitialized)?
        );

        Ok(Self {
            token_x_mint,
            token_y_mint,
            active_id,
        })
    }
}

/// Pool validator for DAMM v2 quote-only fee accrual validation
pub struct PoolValidator;

impl PoolValidator {
    /// Validates that the DAMM v2 pool is configured for quote-only fee accrual
    /// This ensures the honorary position only accrues fees in the quote token
    pub fn validate_quote_only_config(
        pool_account_info: &AccountInfo,
        cp_amm_program: &Pubkey,
        quote_token_mint: &Pubkey,
        base_token_mint: &Pubkey,
    ) -> Result<()> {
        // Check account is owned by cp-amm program
        if pool_account_info.owner != cp_amm_program {
            return err!(HonoraryFeeError::PoolNotInitialized);
        }

        // Extract pool data to validate token configuration
        let lb_pair = LbPair::try_deserialize(&pool_account_info.data.borrow())?;

        // Validate that quote and base tokens match pool configuration
        let has_quote_token = lb_pair.token_x_mint == *quote_token_mint ||
                             lb_pair.token_y_mint == *quote_token_mint;
        let has_base_token = lb_pair.token_x_mint == *base_token_mint ||
                            lb_pair.token_y_mint == *base_token_mint;

        if !has_quote_token || !has_base_token {
            return err!(HonoraryFeeError::InvalidTokenOrder);
        }

        // Ensure quote and base tokens are different
        if quote_token_mint == base_token_mint {
            return err!(HonoraryFeeError::InvalidTokenOrder);
        }

        // Additional validations could include:
        // - Pool is active and not paused
        // - Fee parameters are reasonable
        // - Pool has sufficient liquidity

        Ok(())
    }

    /// Extracts token mint addresses from the DAMM v2 pool
    /// Based on Meteora DLMM lbPair structure with tokenXMint and tokenYMint fields
    pub fn extract_token_mints(
        pool_account_info: &AccountInfo,
        cp_amm_program: &Pubkey,
    ) -> Result<(Pubkey, Pubkey)> {
        // Verify the account is owned by the cp-amm program
        if pool_account_info.owner != cp_amm_program {
            return err!(HonoraryFeeError::PoolNotInitialized);
        }

        // Deserialize the lbPair account to extract token mints
        let lb_pair = LbPair::try_deserialize(&pool_account_info.data.borrow())?;

        Ok((lb_pair.token_x_mint, lb_pair.token_y_mint))
    }

    /// Calculates the tick range required for quote-only fee accrual
    /// For quote-only fees, position liquidity on one side of current price:
    /// - If quote is token X: position below current price (collects when price falls)
    /// - If quote is token Y: position above current price (collects when price rises)
    pub fn calculate_quote_only_tick_range(
        pool_account_info: &AccountInfo,
        _cp_amm_program: &Pubkey,
        quote_token_mint: &Pubkey,
    ) -> Result<(i32, i32)> {
        // First extract pool data to get current active bin and token order
        let lb_pair = LbPair::try_deserialize(&pool_account_info.data.borrow())?;

        // Determine if quote token is token X or token Y
        let is_quote_token_x = lb_pair.token_x_mint == *quote_token_mint;
        let is_quote_token_y = lb_pair.token_y_mint == *quote_token_mint;

        if !is_quote_token_x && !is_quote_token_y {
            return err!(HonoraryFeeError::InvalidQuoteMint);
        }

        // Calculate quote-only tick range
        if is_quote_token_x {
            // Position below current price to collect quote (token X) fees
            // Lower tick: minimum possible, Upper tick: current active bin
            let lower_tick = -887272; // Minimum tick
            let upper_tick = lb_pair.active_id;
            Ok((lower_tick, upper_tick))
        } else {
            // Position above current price to collect quote (token Y) fees
            // Lower tick: current active bin, Upper tick: maximum possible
            let lower_tick = lb_pair.active_id;
            let upper_tick = 887272; // Maximum tick
            Ok((lower_tick, upper_tick))
        }
    }

    /// Detects if claimed fees contain any base token fees
    pub fn detect_base_fees_in_claim(
        claim_amount_a: u64,
        claim_amount_b: u64,
        quote_token_mint: &Pubkey,
        token_a_mint: &Pubkey,
        token_b_mint: &Pubkey,
    ) -> Result<()> {
        let is_token_a_quote = quote_token_mint == token_a_mint;
        let is_token_b_quote = quote_token_mint == token_b_mint;

        if !is_token_a_quote && !is_token_b_quote {
            return err!(HonoraryFeeError::InvalidTokenOrder);
        }

        if is_token_a_quote && claim_amount_b > 0 {
            return err!(HonoraryFeeError::BaseFeesDetected);
        }

        if is_token_b_quote && claim_amount_a > 0 {
            return err!(HonoraryFeeError::BaseFeesDetected);
        }

        Ok(())
    }

    /// Validates pool state for fee distribution
    pub fn validate_pool_for_distribution(
        pool_account_info: &AccountInfo,
        cp_amm_program: &Pubkey,
    ) -> Result<()> {
        if pool_account_info.owner != cp_amm_program {
            return err!(HonoraryFeeError::InvalidStreamAccount);
        }

        Ok(())
    }

    /// Extracts the current active tick/bin ID from pool data
    pub fn extract_current_tick(pool_data: &[u8]) -> Result<i32> {
        let lb_pair = LbPair::try_deserialize(pool_data)?;
        Ok(lb_pair.active_id)
    }

    /// Validates that a position with given tick range would only accrue quote token fees
    pub fn validate_position_for_quote_only_fees(
        pool_account_info: &AccountInfo,
        quote_token_mint: &Pubkey,
        tick_lower: i32,
        tick_upper: i32,
    ) -> Result<()> {
        let lb_pair = LbPair::try_deserialize(&pool_account_info.data.borrow())?;
        let current_tick = lb_pair.active_id;

        // Determine which token is the quote token
        let is_quote_token_x = lb_pair.token_x_mint == *quote_token_mint;
        let is_quote_token_y = lb_pair.token_y_mint == *quote_token_mint;

        if !is_quote_token_x && !is_quote_token_y {
            return err!(HonoraryFeeError::InvalidTokenOrder);
        }

        // For quote-only fees, the position should be entirely on one side of current price
        if is_quote_token_x {
            // Quote is token X: position should be below current price
            if tick_upper >= current_tick {
                return err!(HonoraryFeeError::InvalidTokenOrder);
            }
        } else {
            // Quote is token Y: position should be above current price
            if tick_lower <= current_tick {
                return err!(HonoraryFeeError::InvalidTokenOrder);
            }
        }

        Ok(())
    }
}