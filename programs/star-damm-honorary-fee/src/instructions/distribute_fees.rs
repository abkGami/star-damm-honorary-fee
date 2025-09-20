use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer},
};
use crate::{
    state::*,
    error::HonoraryFeeError,
    events::*,
    utils::MathUtil,
    validation::PoolValidator,
};

#[derive(Accounts)]
pub struct DistributeFees<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    
    /// The vault identifier
    /// CHECK: Used as seed for PDA derivation
    pub vault: UncheckedAccount<'info>,
    
    /// Policy state account
    #[account(
        seeds = [VAULT_SEED, vault.key().as_ref(), POLICY_SEED],
        bump = policy.bump
    )]
    pub policy: Account<'info, PolicyState>,
    
    /// Progress state account
    #[account(
        mut,
        seeds = [VAULT_SEED, vault.key().as_ref(), PROGRESS_SEED],
        bump = progress.bump
    )]
    pub progress: Account<'info, ProgressState>,
    
    /// PDA that owns the honorary position
    #[account(
        seeds = [VAULT_SEED, vault.key().as_ref(), INVESTOR_FEE_POS_OWNER_SEED],
        bump
    )]
    /// CHECK: This is a PDA
    pub position_owner_pda: UncheckedAccount<'info>,
    
    /// Honorary position account
    /// CHECK: Validated through cp-amm integration
    #[account(mut)]
    pub position: UncheckedAccount<'info>,
    
    /// Treasury account for holding claimed fees
    #[account(
        mut,
        associated_token::mint = policy.quote_mint,
        associated_token::authority = position_owner_pda,
    )]
    pub treasury: Account<'info, TokenAccount>,
    
    /// Creator's quote token account
    #[account(
        mut,
        constraint = creator_quote_ata.key() == policy.creator_quote_ata @ HonoraryFeeError::InvalidTreasury
    )]
    pub creator_quote_ata: Account<'info, TokenAccount>,
    
    /// cp-amm program
    /// CHECK: This is the cp-amm program ID
    pub cp_amm_program: UncheckedAccount<'info>,
    
    /// Streamflow program
    /// CHECK: This is the Streamflow program ID
    pub streamflow_program: UncheckedAccount<'info>,
    
    /// Token program
    pub token_program: Program<'info, Token>,
    
    /// System program
    pub system_program: Program<'info, System>,
    
    /// Clock sysvar
    pub clock: Sysvar<'info, Clock>,
}

// Additional accounts for investor distribution (passed as remaining accounts)
#[derive(Clone)]
pub struct InvestorDistributionAccount {
    /// Streamflow stream account
    pub stream_account: Pubkey,
    /// Investor's quote token account
    pub investor_quote_ata: Pubkey,
    /// Current locked amount (read from Streamflow)
    pub locked_amount: u64,
}

pub fn handler(ctx: Context<DistributeFees>, page_size: u64) -> Result<()> {
    let vault = ctx.accounts.vault.key();
    let current_ts = ctx.accounts.clock.unix_timestamp;
    let policy = &ctx.accounts.policy;
    let progress = &mut ctx.accounts.progress;
    
    // Check if this is the start of a new day
    let is_new_day = !progress.day_complete || 
        MathUtil::is_24h_elapsed(progress.last_distribution_ts, current_ts);
    
    // If it's a new day, we need to claim fees first
    if is_new_day {
        require!(
            MathUtil::is_24h_elapsed(progress.last_distribution_ts, current_ts),
            HonoraryFeeError::CooldownNotElapsed
        );
        
        // Start new day
        progress.last_distribution_ts = current_ts;
        progress.daily_distributed = 0;
        progress.pagination_cursor = 0;
        progress.day_complete = false;
        progress.daily_claimed_total = 0;
        
        // Claim fees from honorary position
        claim_fees_from_position(ctx.reborrow())?;
        
        msg!("Started new distribution day, claimed {} quote tokens", 
             progress.daily_claimed_total);
    }
    
    // Process investor distributions
    let (total_distributed, investors_processed) = process_investor_page(
        ctx.reborrow(),
        page_size,
    )?;
    
    // Update progress
    progress.daily_distributed = MathUtil::safe_add(
        progress.daily_distributed,
        total_distributed
    )?;
    
    // Emit page event
    emit!(InvestorPayoutPage {
        vault,
        page_start: progress.pagination_cursor,
        page_end: progress.pagination_cursor + investors_processed,
        total_distributed,
        investor_count: investors_processed,
        timestamp: current_ts,
    });
    
    // Update cursor
    progress.pagination_cursor = MathUtil::safe_add(
        progress.pagination_cursor,
        investors_processed
    )?;
    
    // Check if this was the final page of the day
    if progress.pagination_cursor >= ctx.remaining_accounts.len() as u64 {
        // Final page - distribute remainder to creator and close the day
        close_day_and_pay_creator(ctx, current_ts)?;
    }
    
    Ok(())
}

/// Claim fees from the honorary position
fn claim_fees_from_position(ctx: Context<DistributeFees>) -> Result<()> {
    let vault_key = ctx.accounts.vault.key();
    let seeds = &[
        VAULT_SEED,
        vault_key.as_ref(),
        INVESTOR_FEE_POS_OWNER_SEED,
        &[ctx.bumps.position_owner_pda],
    ];
    let signer = &[&seeds[..]];
    
    // Get treasury balance before claim
    let treasury_before = ctx.accounts.treasury.amount;
    
    // Make CPI call to cp-amm to claim fees
    msg!("Claiming fees from honorary position");
    
    // Placeholder for actual cp-amm fee claiming CPI
    // In real implementation, this would be:
    /*
    let cpi_accounts = cp_amm::cpi::accounts::ClaimFees {
        position: ctx.accounts.position.to_account_info(),
        position_authority: ctx.accounts.position_owner_pda.to_account_info(),
        treasury_a: ctx.accounts.treasury.to_account_info(),
        treasury_b: ctx.accounts.treasury.to_account_info(), // or another account
        // ... other required accounts
    };
    
    let cpi_program = ctx.accounts.cp_amm_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    
    let claim_result = cp_amm::cpi::claim_fees(cpi_ctx)?;
    */
    
    // For now, simulate claiming some fees
    ctx.accounts.treasury.reload()?;
    let treasury_after = ctx.accounts.treasury.amount;
    let claimed_amount = treasury_after.saturating_sub(treasury_before);
    
    // Validate no base fees were claimed
    // In real implementation, we'd check the claim result for base token amounts
    let claimed_tokens = vec![
        (ctx.accounts.policy.quote_mint, claimed_amount),
        // Would also include base mint with amount 0 in real implementation
    ];
    
    // Get base mint from pool (placeholder)
    let base_mint = Pubkey::default(); // Would be extracted from pool
    
    PoolValidator::detect_base_fees_in_claim(
        &ctx.accounts.policy.quote_mint,
        &base_mint,
        &claimed_tokens,
    )?;
    
    // Update progress with claimed amount
    ctx.accounts.progress.daily_claimed_total = claimed_amount;
    
    // Emit claim event
    emit!(QuoteFeesClaimed {
        vault: ctx.accounts.vault.key(),
        amount_claimed: claimed_amount,
        quote_mint: ctx.accounts.policy.quote_mint,
        timestamp: ctx.accounts.clock.unix_timestamp,
    });
    
    Ok(())
}

/// Process a page of investor distributions
fn process_investor_page(
    ctx: Context<DistributeFees>,
    page_size: u64,
) -> Result<(u64, u64)> {
    let policy = &ctx.accounts.policy;
    let progress = &ctx.accounts.progress;
    let cursor = progress.pagination_cursor as usize;
    
    // Calculate available amount for this page
    let total_available = MathUtil::safe_add(
        progress.daily_claimed_total,
        progress.carry_over
    )?;
    let already_distributed = progress.daily_distributed;
    let remaining_for_distribution = MathUtil::safe_sub(total_available, already_distributed)?;
    
    // Get investor data from remaining accounts
    let investor_accounts = parse_investor_accounts(&ctx.remaining_accounts[cursor..])?;
    let page_end = (cursor + page_size as usize).min(investor_accounts.len());
    let investors_this_page = &investor_accounts[..page_end.saturating_sub(cursor)];
    
    // Calculate total locked amount for this page
    let total_locked_this_page: u64 = investors_this_page
        .iter()
        .map(|inv| inv.locked_amount)
        .sum();
    
    if total_locked_this_page == 0 {
        return Ok((0, investors_this_page.len() as u64));
    }
    
    // Calculate investor share based on locked percentage
    let total_locked_all = get_total_locked_amount(&investor_accounts)?;
    let eligible_share_bps = MathUtil::calculate_eligible_share_bps(
        total_locked_all,
        policy.total_investor_allocation,
        policy.investor_fee_share_bps,
    )?;
    
    // Calculate total investor allocation for this distribution
    let investor_total = MathUtil::safe_div(
        MathUtil::safe_mul(remaining_for_distribution, eligible_share_bps as u64)?,
        10000
    )?;
    
    // Apply daily cap if configured
    let capped_investor_total = if policy.daily_cap > 0 {
        investor_total.min(policy.daily_cap.saturating_sub(already_distributed))
    } else {
        investor_total
    };
    
    // Distribute to investors in this page
    let mut total_page_distribution = 0u64;
    
    for investor in investors_this_page {
        let (payout, _remainder) = MathUtil::calculate_proportional_payout(
            capped_investor_total,
            investor.locked_amount,
            total_locked_this_page,
        )?;
        
        // Apply minimum payout threshold
        if payout >= policy.min_payout_lamports {
            // Transfer tokens to investor
            transfer_to_investor(&ctx, investor, payout)?;
            total_page_distribution = MathUtil::safe_add(total_page_distribution, payout)?;
        }
    }
    
    Ok((total_page_distribution, investors_this_page.len() as u64))
}

/// Parse investor account data from remaining accounts
fn parse_investor_accounts(
    remaining_accounts: &[AccountInfo]
) -> Result<Vec<InvestorDistributionAccount>> {
    let mut investors = Vec::new();
    
    // Each investor needs 2 accounts: stream + ATA
    for chunk in remaining_accounts.chunks(2) {
        if chunk.len() < 2 {
            break;
        }
        
        let stream_account = chunk[0].key();
        let investor_quote_ata = chunk[1].key();
        
        // Read locked amount from Streamflow stream
        let locked_amount = read_locked_amount_from_stream(&chunk[0])?;
        
        investors.push(InvestorDistributionAccount {
            stream_account,
            investor_quote_ata,
            locked_amount,
        });
    }
    
    Ok(investors)
}

/// Read locked amount from a Streamflow stream account
fn read_locked_amount_from_stream(stream_account: &AccountInfo) -> Result<u64> {
    // Parse Streamflow stream account to get remaining locked tokens
    // This would integrate with the Streamflow program
    
    msg!("Reading locked amount from stream: {}", stream_account.key);
    
    // Placeholder - would parse actual Streamflow stream data
    // For testing, return a mock value
    Ok(1000000) // 1M tokens locked
}

/// Get total locked amount across all investors
fn get_total_locked_amount(investors: &[InvestorDistributionAccount]) -> Result<u64> {
    let mut total = 0u64;
    for investor in investors {
        total = MathUtil::safe_add(total, investor.locked_amount)?;
    }
    Ok(total)
}

/// Transfer tokens to an investor
fn transfer_to_investor(
    ctx: &Context<DistributeFees>,
    investor: &InvestorDistributionAccount,
    amount: u64,
) -> Result<()> {
    let vault_key = ctx.accounts.vault.key();
    let seeds = &[
        VAULT_SEED,
        vault_key.as_ref(),
        INVESTOR_FEE_POS_OWNER_SEED,
        &[ctx.bumps.position_owner_pda],
    ];
    let signer = &[&seeds[..]];
    
    // Find the investor's ATA in remaining accounts
    let investor_ata_info = ctx.remaining_accounts
        .iter()
        .find(|acc| acc.key == &investor.investor_quote_ata)
        .ok_or(HonoraryFeeError::InvalidTreasury)?;
    
    let cpi_accounts = Transfer {
        from: ctx.accounts.treasury.to_account_info(),
        to: investor_ata_info.clone(),
        authority: ctx.accounts.position_owner_pda.to_account_info(),
    };
    
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    
    token::transfer(cpi_ctx, amount)?;
    
    msg!("Transferred {} tokens to investor {}", amount, investor.investor_quote_ata);
    
    Ok(())
}

/// Close the day and pay remainder to creator
fn close_day_and_pay_creator(
    ctx: Context<DistributeFees>,
    current_ts: i64,
) -> Result<()> {
    let vault = ctx.accounts.vault.key();
    let progress = &mut ctx.accounts.progress;
    
    // Calculate remainder for creator
    let total_available = MathUtil::safe_add(
        progress.daily_claimed_total,
        progress.carry_over
    )?;
    let creator_amount = MathUtil::safe_sub(total_available, progress.daily_distributed)?;
    
    if creator_amount > 0 {
        // Transfer remainder to creator
        let vault_key = ctx.accounts.vault.key();
        let seeds = &[
            VAULT_SEED,
            vault_key.as_ref(),
            INVESTOR_FEE_POS_OWNER_SEED,
            &[ctx.bumps.position_owner_pda],
        ];
        let signer = &[&seeds[..]];
        
        let cpi_accounts = Transfer {
            from: ctx.accounts.treasury.to_account_info(),
            to: ctx.accounts.creator_quote_ata.to_account_info(),
            authority: ctx.accounts.position_owner_pda.to_account_info(),
        };
        
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        
        token::transfer(cpi_ctx, creator_amount)?;
    }
    
    // Mark day as complete
    progress.day_complete = true;
    progress.carry_over = 0; // Reset carry over
    
    // Emit creator payout event
    emit!(CreatorPayoutDayClosed {
        vault,
        creator_amount,
        total_claimed_today: progress.daily_claimed_total,
        total_distributed_to_investors: progress.daily_distributed,
        timestamp: current_ts,
    });
    
    msg!("Day complete - paid {} to creator", creator_amount);
    
    Ok(())
}