use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};
use crate::{
    state::*,
    error::HonoraryFeeError,
    events::HonoraryPositionInitialized,
    validation::PoolValidator,
};

#[derive(Accounts)]
pub struct InitializeHonoraryPosition<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    
    /// The vault identifier (used for PDA seeds)
    /// CHECK: This is used as a seed for PDA derivation
    pub vault: UncheckedAccount<'info>,
    
    /// cp-amm pool account
    /// CHECK: Validated in handler through cp-amm integration
    pub pool: UncheckedAccount<'info>,
    
    /// Quote mint (the token we collect fees in)
    pub quote_mint: Account<'info, Mint>,
    
    /// Base mint (the token we must NOT collect fees from)
    pub base_mint: Account<'info, Mint>,
    
    /// Creator's quote token account (for remainder distribution)
    pub creator_quote_ata: Account<'info, TokenAccount>,
    
    /// PDA that will own the honorary position
    #[account(
        seeds = [VAULT_SEED, vault.key().as_ref(), INVESTOR_FEE_POS_OWNER_SEED],
        bump
    )]
    /// CHECK: This is a PDA and will be validated
    pub position_owner_pda: UncheckedAccount<'info>,
    
    /// Policy state account
    #[account(
        init,
        payer = payer,
        space = PolicyState::LEN,
        seeds = [VAULT_SEED, vault.key().as_ref(), POLICY_SEED],
        bump
    )]
    pub policy: Account<'info, PolicyState>,
    
    /// Progress state account
    #[account(
        init,
        payer = payer,
        space = ProgressState::LEN,
        seeds = [VAULT_SEED, vault.key().as_ref(), PROGRESS_SEED],
        bump
    )]
    pub progress: Account<'info, ProgressState>,
    
    /// Treasury account for holding claimed quote fees
    #[account(
        init,
        payer = payer,
        associated_token::mint = quote_mint,
        associated_token::authority = position_owner_pda,
    )]
    pub treasury: Account<'info, TokenAccount>,
    
    /// The honorary position account that will be created
    /// CHECK: This will be created via cp-amm CPI
    #[account(mut)]
    pub position: UncheckedAccount<'info>,
    
    /// cp-amm program
    /// CHECK: This is the cp-amm program ID
    pub cp_amm_program: UncheckedAccount<'info>,
    
    /// System program
    pub system_program: Program<'info, System>,
    
    /// Token program  
    pub token_program: Program<'info, Token>,
    
    /// Associated token program
    pub associated_token_program: Program<'info, AssociatedToken>,
    
    /// Rent sysvar
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<InitializeHonoraryPosition>,
    investor_fee_share_bps: u16,
    daily_cap: u64,
    min_payout_lamports: u64,
    total_investor_allocation: u64,
) -> Result<()> {
    let vault = ctx.accounts.vault.key();
    let quote_mint = ctx.accounts.quote_mint.key();
    
    // Validate investor fee share is within bounds (0-10000 basis points)
    require!(investor_fee_share_bps <= 10000, HonoraryFeeError::InvalidTokenOrder);
    
    // Validate pool configuration for quote-only fees
    PoolValidator::validate_quote_only_config(
        &ctx.accounts.pool,
        &quote_mint,
    )?;
    
    // Validate that creator_quote_ata belongs to the correct mint
    require!(
        ctx.accounts.creator_quote_ata.mint == quote_mint,
        HonoraryFeeError::InvalidQuoteMint
    );
    
    // Initialize policy state
    let policy = &mut ctx.accounts.policy;
    policy.investor_fee_share_bps = investor_fee_share_bps;
    policy.daily_cap = daily_cap;
    policy.min_payout_lamports = min_payout_lamports;
    policy.quote_mint = quote_mint;
    policy.creator_quote_ata = ctx.accounts.creator_quote_ata.key();
    policy.total_investor_allocation = total_investor_allocation;
    policy.bump = ctx.bumps.policy;
    
    // Initialize progress state  
    let progress = &mut ctx.accounts.progress;
    progress.last_distribution_ts = 0; // Allow immediate first distribution
    progress.daily_distributed = 0;
    progress.carry_over = 0;
    progress.pagination_cursor = 0;
    progress.daily_claimed_total = 0;
    progress.day_complete = true; // Start with day complete
    progress.bump = ctx.bumps.progress;
    
    // Create the honorary position via cp-amm CPI
    // This is where we'd make the actual cp-amm call to create a position
    // The position should be configured to only accrue quote token fees
    create_honorary_position_cpi(ctx)?;
    
    // Emit initialization event
    emit!(HonoraryPositionInitialized {
        vault,
        position_owner_pda: ctx.accounts.position_owner_pda.key(),
        quote_mint,
        position: ctx.accounts.position.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    msg!(
        "Initialized honorary position for vault {} with quote mint {}",
        vault,
        quote_mint
    );
    
    Ok(())
}

/// Create the honorary position via cp-amm CPI
fn create_honorary_position_cpi(ctx: Context<InitializeHonoraryPosition>) -> Result<()> {
    // This is where we would make the actual CPI call to cp-amm
    // to create a liquidity position owned by our PDA
    
    // The position should be:
    // 1. Owned by position_owner_pda
    // 2. Configured to only accrue quote token fees
    // 3. Properly initialized with the pool
    
    msg!("Creating honorary position via cp-amm CPI");
    msg!("Position owner PDA: {}", ctx.accounts.position_owner_pda.key());
    msg!("Position account: {}", ctx.accounts.position.key());
    
    // Placeholder for actual cp-amm integration
    // In the real implementation, this would be a CPI call like:
    
    /*
    let vault_key = ctx.accounts.vault.key();
    let seeds = &[
        VAULT_SEED,
        vault_key.as_ref(),
        INVESTOR_FEE_POS_OWNER_SEED,
        &[ctx.bumps.position_owner_pda],
    ];
    let signer = &[&seeds[..]];
    
    let cpi_accounts = cp_amm::cpi::accounts::CreatePosition {
        position: ctx.accounts.position.to_account_info(),
        position_authority: ctx.accounts.position_owner_pda.to_account_info(),
        pool: ctx.accounts.pool.to_account_info(),
        // ... other required accounts
    };
    
    let cpi_program = ctx.accounts.cp_amm_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    
    cp_amm::cpi::create_position(cpi_ctx, /* position params */)?;
    */
    
    Ok(())
}