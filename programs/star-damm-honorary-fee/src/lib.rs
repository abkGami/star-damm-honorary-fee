use anchor_lang::prelude::*;

mod state;
mod error;
mod events;
mod utils;
mod instructions;

pub use state::*;
pub use error::*;
pub use events::*;
pub use utils::*;
pub use instructions::*;

declare_id!("AQUVRgoaGsoy2uGnzkSDBoEVEJox2XT6Vna3Y9xKKwFZ");

#[program]
pub mod star_damm_honorary_fee {
    use super::*;

    /// Initialize the honorary fee position and policy
    pub fn initialize_honorary_position(
        ctx: Context<InitializeHonoraryPosition>,
        investor_fee_share_bps: u16,
        daily_cap: u64,
        min_payout_lamports: u64,
        total_investor_allocation: u64,
    ) -> Result<()> {
        instructions::initialize_handler(
            ctx,
            investor_fee_share_bps,
            daily_cap,
            min_payout_lamports,
            total_investor_allocation,
        )
    }

    /// Permissionless crank to claim and distribute fees (supports pagination)
    pub fn distribute_fees(
        ctx: Context<DistributeFees>,
        page_size: u32,
    ) -> Result<()> {
        instructions::distribute_handler(ctx, page_size)
    }
}
