use anchor_lang::prelude::*;

pub mod error;
pub mod events;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("625a3ioDN2HBSGZwpRFKGWcAVSc2XbySBRubDcMKxVjo");

#[program]
pub mod nft_lending {
    use super::*;

    // ─── Global ───────────────────────────────────────────────────────────────

    /// Initialize the protocol global state. Call once after deployment.
    pub fn initialize_global(
        ctx: Context<InitializeGlobal>,
        protocol_fee_bps: u16,
        reward_share_bps: u16,
    ) -> Result<()> {
        instructions::initialize_global(ctx, protocol_fee_bps, reward_share_bps)
    }

    /// Update protocol parameters (admin only).
    pub fn update_global(
        ctx: Context<UpdateGlobal>,
        protocol_fee_bps: Option<u16>,
        reward_share_bps: Option<u16>,
    ) -> Result<()> {
        instructions::update_global(ctx, protocol_fee_bps, reward_share_bps)
    }

    // ─── Orderbook ────────────────────────────────────────────────────────────

    /// Create a new orderbook for a (collection, duration) pair. Permissionless.
    pub fn initialize_orderbook(
        ctx: Context<InitializeOrderBook>,
        duration_seconds: u64,
        max_incentive_spread_bps: u64,
    ) -> Result<()> {
        instructions::initialize_orderbook(ctx, duration_seconds, max_incentive_spread_bps)
    }

    // ─── Lender Orders ────────────────────────────────────────────────────────

    /// Place a lender limit order. Deposits principal SOL into order escrow.
    pub fn place_lender_order(
        ctx: Context<PlaceLenderOrder>,
        principal_lamports: u64,
        apr_bps: u64,
        expiry_ts: i64,
    ) -> Result<()> {
        instructions::place_lender_order(ctx, principal_lamports, apr_bps, expiry_ts)
    }

    /// Cancel an unmatched lender order and recover escrowed SOL.
    pub fn cancel_lender_order(ctx: Context<CancelLenderOrder>) -> Result<()> {
        instructions::cancel_lender_order(ctx)
    }

    /// Crank: update the best APR fields on the orderbook after order changes.
    pub fn update_best_lender_apr(
        ctx: Context<UpdateBestApr>,
        new_best_lender_apr_bps: u64,
        new_best_borrower_max_apr_bps: u64,
    ) -> Result<()> {
        instructions::update_best_lender_apr(
            ctx,
            new_best_lender_apr_bps,
            new_best_borrower_max_apr_bps,
        )
    }

    // ─── Borrower Orders ─────────────────────────────────────────────────────

    /// Place a borrower limit order. Freezes NFT in borrower's wallet.
    pub fn place_borrower_order(
        ctx: Context<PlaceBorrowerOrder>,
        principal_lamports: u64,
        max_apr_bps: u64,
        expiry_ts: i64,
    ) -> Result<()> {
        instructions::place_borrower_order(ctx, principal_lamports, max_apr_bps, expiry_ts)
    }

    /// Cancel a borrower order. Unfreezes NFT.
    pub fn cancel_borrower_order(ctx: Context<CancelBorrowerOrder>) -> Result<()> {
        instructions::cancel_borrower_order(ctx)
    }

    // ─── Matching ─────────────────────────────────────────────────────────────

    /// Match a lender order with a borrower order. Permissionless (crank).
    pub fn match_orders(ctx: Context<MatchOrders>) -> Result<()> {
        instructions::match_orders(ctx)
    }

    // ─── Loan Lifecycle ───────────────────────────────────────────────────────

    /// Borrower repays loan. NFT is unfrozen, loan account closed.
    pub fn repay_loan(ctx: Context<RepayLoan>) -> Result<()> {
        instructions::repay_loan(ctx)
    }

    /// Lender forecloses expired loan. NFT transferred to lender.
    pub fn foreclose_loan(ctx: Context<ForeCloseLoan>) -> Result<()> {
        instructions::foreclose_loan(ctx)
    }

    // ─── Rewards ──────────────────────────────────────────────────────────────

    /// Initialize a reward epoch (admin). Funds the reward pool.
    pub fn initialize_reward_epoch(
        ctx: Context<InitializeRewardEpoch>,
        epoch_id: u64,
        start_ts: i64,
        end_ts: i64,
        total_reward_lamports: u64,
    ) -> Result<()> {
        instructions::initialize_reward_epoch(
            ctx,
            epoch_id,
            start_ts,
            end_ts,
            total_reward_lamports,
        )
    }

    /// Record a maker's score contribution for a sampling period (trusted crank).
    pub fn record_maker_score(
        ctx: Context<RecordMakerScore>,
        score_delta: u64,
    ) -> Result<()> {
        instructions::record_maker_score(ctx, score_delta)
    }

    /// Finalize a reward epoch after it ends (admin).
    pub fn finalize_reward_epoch(ctx: Context<FinalizeRewardEpoch>) -> Result<()> {
        instructions::finalize_reward_epoch(ctx)
    }

    /// Claim accumulated liquidity rewards for a finalized epoch.
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        instructions::claim_rewards(ctx)
    }
}
