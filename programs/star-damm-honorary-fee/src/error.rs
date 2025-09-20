use anchor_lang::prelude::*;

#[error_code]
pub enum HonoraryFeeError {
    #[msg("Invalid pool configuration - would accrue base token fees")]
    BaseFeesDetected,
    
    #[msg("Invalid token order - quote mint not identified")]
    InvalidTokenOrder,
    
    #[msg("24 hour cooldown not yet elapsed")]
    CooldownNotElapsed,
    
    #[msg("Daily cap exceeded")]
    DailyCapExceeded,
    
    #[msg("Amount below minimum payout threshold")]
    BelowMinPayout,
    
    #[msg("No locked tokens found for distribution")]
    NoLockedTokens,
    
    #[msg("Invalid Streamflow stream account")]
    InvalidStreamAccount,
    
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
    
    #[msg("Distribution already complete for this day")]
    DistributionComplete,
    
    #[msg("Invalid pagination cursor")]
    InvalidPaginationCursor,
    
    #[msg("Pool not initialized properly")]
    PoolNotInitialized,
    
    #[msg("Position not owned by program PDA")]
    InvalidPositionOwner,
    
    #[msg("Base token fees detected during claim - aborting distribution")]
    BaseFeesInClaim,
    
    #[msg("Invalid quote mint for this vault")]
    InvalidQuoteMint,
    
    #[msg("Treasury ATA not found or invalid")]
    InvalidTreasury,
}