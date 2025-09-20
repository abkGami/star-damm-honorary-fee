use anchor_lang::prelude::*;

/// Event emitted when honorary position is initialized
#[event]
pub struct HonoraryPositionInitialized {
    pub vault: Pubkey,
    pub position_owner_pda: Pubkey,
    pub quote_mint: Pubkey,
    pub position: Pubkey,
    pub timestamp: i64,
}

/// Event emitted when quote fees are claimed from the honorary position
#[event]
pub struct QuoteFeesClaimed {
    pub vault: Pubkey,
    pub amount_claimed: u64,
    pub quote_mint: Pubkey,
    pub timestamp: i64,
}

/// Event emitted for each investor payout page
#[event] 
pub struct InvestorPayoutPage {
    pub vault: Pubkey,
    pub page_start: u64,
    pub page_end: u64,
    pub total_distributed: u64,
    pub investor_count: u64,
    pub timestamp: i64,
}

/// Event emitted when creator gets remainder payout at day close
#[event]
pub struct CreatorPayoutDayClosed {
    pub vault: Pubkey,
    pub creator_amount: u64,
    pub total_claimed_today: u64,
    pub total_distributed_to_investors: u64,
    pub timestamp: i64,
}