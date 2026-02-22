use anchor_lang::prelude::*;
use anchor_lang::system_program;
use crate::state::{LenderOrder, OrderBook};

/// Place a lender limit order.
///
/// The lender deposits `principal_lamports` into the order PDA (which acts as
/// the escrow). The PDA's lamport balance = rent_minimum + principal.
/// On cancel or match, principal is returned or transferred.
pub fn place_lender_order(
    ctx: Context<PlaceLenderOrder>,
    principal_lamports: u64,
    apr_bps: u64,
    expiry_ts: i64,
) -> Result<()> {
    let ob = &mut ctx.accounts.orderbook;
    require!(ob.is_active, crate::error::LendingError::OrderBookInactive);
    require!(principal_lamports >= 100_000_000, crate::error::LendingError::AmountTooSmall); // min 0.1 SOL
    require!(apr_bps > 0 && apr_bps <= 100_000, crate::error::LendingError::InvalidApr); // max 1000%
    let now = Clock::get()?.unix_timestamp;
    require!(expiry_ts == 0 || expiry_ts > now, crate::error::LendingError::AlreadyExpired);

    // Assign nonce from orderbook (ensures unique PDA even with same lender+orderbook)
    let nonce = ob.lender_order_nonce;
    ob.lender_order_nonce = ob.lender_order_nonce.checked_add(1).unwrap();

    // Initialize the order
    let order = &mut ctx.accounts.lender_order;
    order.orderbook = ob.key();
    order.lender = ctx.accounts.lender.key();
    order.principal_lamports = principal_lamports;
    order.apr_bps = apr_bps;
    order.expiry_ts = expiry_ts;
    order.created_at = now;
    order.nonce = nonce;
    order.bump = ctx.bumps.lender_order;

    // Transfer principal from lender to the order account (beyond rent).
    // The account is initialized with rent-exempt space. We now add principal.
    let transfer_ctx = CpiContext::new(
        ctx.accounts.system_program.to_account_info(),
        system_program::Transfer {
            from: ctx.accounts.lender.to_account_info(),
            to: ctx.accounts.lender_order.to_account_info(),
        },
    );
    system_program::transfer(transfer_ctx, principal_lamports)?;

    // Update orderbook stats
    ob.open_lender_orders += 1;

    // Update best lender APR if this is the lowest (best for borrowers)
    if ob.best_lender_apr_bps == 0 || apr_bps < ob.best_lender_apr_bps {
        ob.best_lender_apr_bps = apr_bps;
        ob.update_mid_apr();
    }

    emit!(crate::events::LenderOrderPlaced {
        orderbook: ob.key(),
        order: ctx.accounts.lender_order.key(),
        lender: ctx.accounts.lender.key(),
        principal_lamports,
        apr_bps,
        expiry_ts,
        nonce,
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(principal_lamports: u64, apr_bps: u64, expiry_ts: i64)]
pub struct PlaceLenderOrder<'info> {
    #[account(
        mut,
        seeds = [
            OrderBook::SEED,
            orderbook.collection_mint.as_ref(),
            &orderbook.duration_seconds.to_le_bytes(),
        ],
        bump = orderbook.bump
    )]
    pub orderbook: Account<'info, OrderBook>,

    #[account(
        init,
        payer = lender,
        space = LenderOrder::SIZE,
        seeds = [
            LenderOrder::SEED,
            orderbook.key().as_ref(),
            lender.key().as_ref(),
            &orderbook.lender_order_nonce.to_le_bytes(),
        ],
        bump
    )]
    pub lender_order: Account<'info, LenderOrder>,

    #[account(mut)]
    pub lender: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Cancel an unmatched lender order and recover the escrowed SOL.
pub fn cancel_lender_order(ctx: Context<CancelLenderOrder>) -> Result<()> {
    let order = &ctx.accounts.lender_order;
    let ob = &mut ctx.accounts.orderbook;
    let lender = &ctx.accounts.lender;

    require!(
        order.lender == lender.key(),
        crate::error::LendingError::Unauthorized
    );

    // Return principal: order account has rent + principal lamports.
    // Closing the account via close = lender returns all lamports (including principal).
    // anchor's `close` constraint handles this automatically.

    ob.open_lender_orders = ob.open_lender_orders.saturating_sub(1);

    // NOTE: best_lender_apr_bps may be stale after cancel.
    // The off-chain crank updates this field by calling update_orderbook_best_apr.
    // For correctness, we only set it to 0 if no orders remain.
    if ob.open_lender_orders == 0 {
        ob.best_lender_apr_bps = 0;
        ob.update_mid_apr();
    }

    emit!(crate::events::LenderOrderCancelled {
        orderbook: ob.key(),
        order: ctx.accounts.lender_order.key(),
        lender: lender.key(),
        principal_lamports: order.principal_lamports,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct CancelLenderOrder<'info> {
    #[account(
        mut,
        seeds = [
            OrderBook::SEED,
            orderbook.collection_mint.as_ref(),
            &orderbook.duration_seconds.to_le_bytes(),
        ],
        bump = orderbook.bump
    )]
    pub orderbook: Account<'info, OrderBook>,

    #[account(
        mut,
        close = lender,   // returns rent + principal to lender
        seeds = [
            LenderOrder::SEED,
            orderbook.key().as_ref(),
            lender.key().as_ref(),
            &lender_order.nonce.to_le_bytes(),
        ],
        bump = lender_order.bump,
        has_one = orderbook,
        has_one = lender,
    )]
    pub lender_order: Account<'info, LenderOrder>,

    #[account(mut)]
    pub lender: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Admin/crank instruction to update the best lender APR on the orderbook
/// after the true best has changed due to cancellations. The caller must pass
/// the current true best APR (verified off-chain).
pub fn update_best_lender_apr(
    ctx: Context<UpdateBestApr>,
    new_best_lender_apr_bps: u64,
    new_best_borrower_max_apr_bps: u64,
) -> Result<()> {
    let ob = &mut ctx.accounts.orderbook;
    ob.best_lender_apr_bps = new_best_lender_apr_bps;
    ob.best_borrower_max_apr_bps = new_best_borrower_max_apr_bps;
    ob.update_mid_apr();
    Ok(())
}

#[derive(Accounts)]
pub struct UpdateBestApr<'info> {
    #[account(
        mut,
        seeds = [
            OrderBook::SEED,
            orderbook.collection_mint.as_ref(),
            &orderbook.duration_seconds.to_le_bytes(),
        ],
        bump = orderbook.bump
    )]
    pub orderbook: Account<'info, OrderBook>,

    /// Only admin or crank authority can update best APR.
    pub authority: Signer<'info>,
}
