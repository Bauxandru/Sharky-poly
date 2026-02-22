use anchor_lang::prelude::*;
use crate::state::{GlobalState, LenderOrder, BorrowerOrder, Loan, LoanState, OrderBook};

/// Match a lender order with a borrower order to create a loan.
///
/// This instruction is **permissionless** — anyone can call it (the crank bot
/// does so, but so can users). The instruction validates:
///   1. Both orders belong to the same orderbook.
///   2. lender_apr ≤ borrower_max_apr (match condition).
///   3. Both orders are not expired.
///   4. Principal amounts are compatible.
///
/// On successful match:
///   - A Loan account is created.
///   - The lender order account is closed (principal moves to borrower).
///   - The borrower order account is closed (NFT stays frozen in borrower wallet).
///   - Orderbook stats are updated.
pub fn match_orders(ctx: Context<MatchOrders>) -> Result<()> {
    let lender_order = &ctx.accounts.lender_order;
    let borrower_order = &ctx.accounts.borrower_order;
    let ob = &mut ctx.accounts.orderbook;
    let now = Clock::get()?.unix_timestamp;

    // --- Validation ---
    require!(
        lender_order.orderbook == ob.key(),
        crate::error::LendingError::OrderBookMismatch
    );
    require!(
        borrower_order.orderbook == ob.key(),
        crate::error::LendingError::OrderBookMismatch
    );
    require!(!lender_order.is_expired(now), crate::error::LendingError::OrderExpired);
    require!(!borrower_order.is_expired(now), crate::error::LendingError::OrderExpired);

    // Core match condition: lender is willing to lend at an APR that the borrower accepts
    require!(
        lender_order.apr_bps <= borrower_order.max_apr_bps,
        crate::error::LendingError::OrdersNotMatchable
    );

    // Principal must match exactly (both orders specify same amount).
    // The off-chain matcher ensures this before calling.
    require!(
        lender_order.principal_lamports == borrower_order.principal_lamports,
        crate::error::LendingError::PrincipalMismatch
    );

    let principal = lender_order.principal_lamports;
    let apr_bps = lender_order.apr_bps; // Lender is the maker; their APR sets the rate
    let duration = ob.duration_seconds;
    let global = &ctx.accounts.global_state;

    // Compute repay amount
    let (repay_lamports, protocol_fee_lamports) =
        Loan::compute_repay(principal, apr_bps, duration, global.protocol_fee_bps);

    // --- Create Loan ---
    let loan = &mut ctx.accounts.loan;
    loan.orderbook = ob.key();
    loan.lender = lender_order.lender;
    loan.borrower = borrower_order.borrower;
    loan.nft_mint = borrower_order.nft_mint;
    loan.nft_token_account = borrower_order.nft_token_account;
    loan.lender_order = ctx.accounts.lender_order.key();
    loan.borrower_order = ctx.accounts.borrower_order.key();
    loan.principal_lamports = principal;
    loan.apr_bps = apr_bps;
    loan.duration_seconds = duration;
    loan.start_ts = now;
    loan.end_ts = now + duration as i64;
    loan.repay_lamports = repay_lamports;
    loan.protocol_fee_lamports = protocol_fee_lamports;
    loan.state = LoanState::Active;
    loan.bump = ctx.bumps.loan;

    // --- Transfer principal from lender order escrow → borrower ---
    // The lender_order account holds rent + principal in its lamports.
    // We transfer only the principal to the borrower; the rent goes back to lender
    // when the account is closed via `close = lender_order_refund_target`.
    //
    // Anchor's `close` constraint on lender_order will return all remaining lamports
    // (rent) to the lender's refund account. We need to manually move the principal
    // out before that happens.
    //
    // Strategy: do a raw lamport transfer from lender_order to borrower.
    {
        let lender_order_info = ctx.accounts.lender_order.to_account_info();
        let borrower_info = ctx.accounts.borrower_wallet.to_account_info();
        **lender_order_info.try_borrow_mut_lamports()? -= principal;
        **borrower_info.try_borrow_mut_lamports()? += principal;
    }

    // Update orderbook stats
    ob.open_lender_orders = ob.open_lender_orders.saturating_sub(1);
    ob.open_borrower_orders = ob.open_borrower_orders.saturating_sub(1);
    ob.active_loans += 1;
    ob.total_loans += 1;
    ob.total_volume_lamports = ob.total_volume_lamports.saturating_add(principal);

    // Update global volume
    ctx.accounts.global_state.total_volume_lamports =
        ctx.accounts.global_state.total_volume_lamports.saturating_add(principal);

    emit!(crate::events::LoanCreated {
        loan: loan.key(),
        orderbook: ob.key(),
        lender: loan.lender,
        borrower: loan.borrower,
        nft_mint: loan.nft_mint,
        principal_lamports: principal,
        apr_bps,
        duration_seconds: duration,
        start_ts: now,
        end_ts: loan.end_ts,
        repay_lamports,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct MatchOrders<'info> {
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
        seeds = [GlobalState::SEED],
        bump = global_state.bump
    )]
    pub global_state: Account<'info, GlobalState>,

    /// The lender order being matched. Closed after match (rent → lender).
    #[account(
        mut,
        close = lender_refund,
        seeds = [
            LenderOrder::SEED,
            orderbook.key().as_ref(),
            lender_order.lender.as_ref(),
            &lender_order.nonce.to_le_bytes(),
        ],
        bump = lender_order.bump,
        has_one = orderbook,
    )]
    pub lender_order: Account<'info, LenderOrder>,

    /// The borrower order being matched. Closed after match (rent → borrower).
    #[account(
        mut,
        close = borrower_refund,
        seeds = [
            BorrowerOrder::SEED,
            orderbook.key().as_ref(),
            borrower_order.nft_mint.as_ref(),
        ],
        bump = borrower_order.bump,
        has_one = orderbook,
    )]
    pub borrower_order: Account<'info, BorrowerOrder>,

    /// The loan account created by this match.
    #[account(
        init,
        payer = crank,
        space = Loan::SIZE,
        seeds = [
            Loan::SEED,
            lender_order.key().as_ref(),
            borrower_order.key().as_ref(),
        ],
        bump
    )]
    pub loan: Account<'info, Loan>,

    /// Lender's wallet (receives principal escrow rent refund from closed order).
    /// CHECK: This is just a lamport recipient, verified by has_one on lender_order.
    #[account(mut, address = lender_order.lender)]
    pub lender_refund: UncheckedAccount<'info>,

    /// Borrower's wallet (receives principal loan funds).
    /// CHECK: lamport recipient, verified by has_one on borrower_order.
    #[account(mut, address = borrower_order.borrower)]
    pub borrower_wallet: UncheckedAccount<'info>,

    /// Borrower's wallet refund for closed borrower_order rent.
    /// CHECK: same as borrower_wallet.
    #[account(mut, address = borrower_order.borrower)]
    pub borrower_refund: UncheckedAccount<'info>,

    /// The crank/caller pays for Loan account creation.
    #[account(mut)]
    pub crank: Signer<'info>,

    pub system_program: Program<'info, System>,
}
