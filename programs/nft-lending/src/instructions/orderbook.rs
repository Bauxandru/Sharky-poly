use anchor_lang::prelude::*;
use crate::state::{GlobalState, OrderBook};

/// Create a new orderbook for a specific (collection, duration) pair.
/// This is permissionless — anyone can create a new orderbook.
pub fn initialize_orderbook(
    ctx: Context<InitializeOrderBook>,
    duration_seconds: u64,
    max_incentive_spread_bps: u64,
) -> Result<()> {
    require!(duration_seconds > 0, crate::error::LendingError::InvalidParam);
    // Valid durations: 1 day minimum, 365 days maximum
    require!(
        duration_seconds >= 86_400 && duration_seconds <= 31_536_000,
        crate::error::LendingError::InvalidDuration
    );
    require!(
        max_incentive_spread_bps > 0 && max_incentive_spread_bps <= 5_000,
        crate::error::LendingError::InvalidParam
    );

    let ob = &mut ctx.accounts.orderbook;
    ob.collection_mint = ctx.accounts.collection_mint.key();
    ob.duration_seconds = duration_seconds;
    ob.best_lender_apr_bps = 0;
    ob.best_borrower_max_apr_bps = 0;
    ob.mid_apr_bps = 0;
    ob.max_incentive_spread_bps = max_incentive_spread_bps;
    ob.open_lender_orders = 0;
    ob.open_borrower_orders = 0;
    ob.active_loans = 0;
    ob.total_loans = 0;
    ob.total_volume_lamports = 0;
    ob.lender_order_nonce = 0;
    ob.is_active = true;
    ob.bump = ctx.bumps.orderbook;

    // Increment global orderbook counter
    ctx.accounts.global_state.total_orderbooks += 1;

    emit!(crate::events::OrderBookCreated {
        orderbook: ob.key(),
        collection_mint: ob.collection_mint,
        duration_seconds,
        creator: ctx.accounts.payer.key(),
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(duration_seconds: u64)]
pub struct InitializeOrderBook<'info> {
    #[account(
        init,
        payer = payer,
        space = OrderBook::SIZE,
        seeds = [
            OrderBook::SEED,
            collection_mint.key().as_ref(),
            &duration_seconds.to_le_bytes(),
        ],
        bump
    )]
    pub orderbook: Account<'info, OrderBook>,

    /// CHECK: We only store this pubkey as the collection identifier.
    /// Validation (verified collection) is enforced on borrow order placement.
    pub collection_mint: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [GlobalState::SEED],
        bump = global_state.bump
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
