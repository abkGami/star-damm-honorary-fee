use anchor_lang::prelude::*;

/// State structure for the policy configuration
#[account]
pub struct PolicyState {
    /// Fee share for investors in basis points (0-10000)
    pub investor_fee_share_bps: u16,
    
    /// Optional daily cap in quote tokens (0 = no cap)  
    pub daily_cap: u64,
    
    /// Minimum payout threshold in lamports
    pub min_payout_lamports: u64,
    
    /// The quote mint for this policy
    pub quote_mint: Pubkey,
    
    /// Creator's quote ATA for remainder distribution
    pub creator_quote_ata: Pubkey,
    
    /// Total investor allocation minted at TGE (Y0)
    pub total_investor_allocation: u64,
    
    /// Bump for PDA derivation
    pub bump: u8,
}

impl PolicyState {
    pub const LEN: usize = 8 + // discriminator
        2 +    // investor_fee_share_bps
        8 +    // daily_cap
        8 +    // min_payout_lamports
        32 +   // quote_mint
        32 +   // creator_quote_ata
        8 +    // total_investor_allocation
        1;     // bump
}

/// State structure for tracking distribution progress
#[account]
pub struct ProgressState {
    /// Last distribution timestamp
    pub last_distribution_ts: i64,
    
    /// Total distributed today
    pub daily_distributed: u64,
    
    /// Carry-over amount from previous distributions
    pub carry_over: u64,
    
    /// Current pagination cursor
    pub pagination_cursor: u64,
    
    /// Current day's total claimed fees before distribution
    pub daily_claimed_total: u64,
    
    /// Whether the current day's distribution is complete
    pub day_complete: bool,
    
    /// Bump for PDA derivation
    pub bump: u8,
}

impl ProgressState {
    pub const LEN: usize = 8 + // discriminator
        8 +    // last_distribution_ts
        8 +    // daily_distributed
        8 +    // carry_over
        8 +    // pagination_cursor
        8 +    // daily_claimed_total
        1 +    // day_complete
        1;     // bump
}

/// Seeds for PDA derivation
pub const VAULT_SEED: &[u8] = b"star_vault";
pub const INVESTOR_FEE_POS_OWNER_SEED: &[u8] = b"investor_fee_pos_owner";
pub const POLICY_SEED: &[u8] = b"policy";
pub const PROGRESS_SEED: &[u8] = b"progress";
pub const TREASURY_SEED: &[u8] = b"treasury";

/// Helper functions for PDA derivation
pub fn get_investor_fee_position_owner_pda(
    vault: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[VAULT_SEED, vault.as_ref(), INVESTOR_FEE_POS_OWNER_SEED],
        program_id,
    )
}

pub fn get_policy_pda(
    vault: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[VAULT_SEED, vault.as_ref(), POLICY_SEED],
        program_id,
    )
}

pub fn get_progress_pda(
    vault: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[VAULT_SEED, vault.as_ref(), PROGRESS_SEED],
        program_id,
    )
}

pub fn get_treasury_pda(
    vault: &Pubkey,
    quote_mint: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[VAULT_SEED, vault.as_ref(), TREASURY_SEED, quote_mint.as_ref()],
        program_id,
    )
}