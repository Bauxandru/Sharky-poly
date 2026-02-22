use anchor_lang::prelude::*;

/// Per-collection, per-duration orderbook.
/// Seeds: ["orderbook", collection_mint, duration_seconds_le_bytes (8)]
///
/// One orderbook exists per (collection, duration) pair.
/// Lenders and borrowers both post orders here.
/// Off-chain indexer maintains the sorted order list from on-chain events.
#[account]
pub struct OrderBook {
    /// NFT collection verified creator / verified collection mint.
    /// This is the on-chain identity of the NFT collection.
    pub collection_mint: Pubkey,

    /// Fixed loan duration in seconds for all loans on this orderbook.
    /// Common values: 604800 (7d), 1209600 (14d), 2592000 (30d)
    pub duration_seconds: u64,

    /// Best (lowest) APR currently offered by lenders, in basis points.
    /// Updated on every place/cancel/match. 0 if no lender orders.
    pub best_lender_apr_bps: u64,

    /// Best (highest) max-APR currently offered by borrowers, in basis points.
    /// Updated on every place/cancel/match. 0 if no borrower orders.
    pub best_borrower_max_apr_bps: u64,

    /// Mid-APR used for reward scoring = (best_lender + best_borrower) / 2.
    /// Updated whenever best_lender or best_borrower changes.
    pub mid_apr_bps: u64,

    /// Maximum spread from mid-APR eligible for rewards (in bps).
    /// Default: 500 (5% APR). Admin-configurable per orderbook.
    pub max_incentive_spread_bps: u64,

    /// Total active lender orders (unmatched).
    pub open_lender_orders: u64,

    /// Total active borrower orders (unmatched).
    pub open_borrower_orders: u64,

    /// Total loans currently active on this orderbook.
    pub active_loans: u64,

    /// Cumulative loans originated.
    pub total_loans: u64,

    /// Cumulative volume in lamports.
    pub total_volume_lamports: u64,

    /// Nonce for lender orders (monotonically increasing).
    /// Used as part of lender order PDA seeds to allow multiple orders.
    pub lender_order_nonce: u64,

    /// Is this orderbook active? Admin can pause/unpause.
    pub is_active: bool,

    /// Bump for PDA derivation.
    pub bump: u8,
}

impl OrderBook {
    pub const SEED: &'static [u8] = b"orderbook";

    pub const SIZE: usize = 8   // discriminator
        + 32    // collection_mint
        + 8     // duration_seconds
        + 8     // best_lender_apr_bps
        + 8     // best_borrower_max_apr_bps
        + 8     // mid_apr_bps
        + 8     // max_incentive_spread_bps
        + 8     // open_lender_orders
        + 8     // open_borrower_orders
        + 8     // active_loans
        + 8     // total_loans
        + 8     // total_volume_lamports
        + 8     // lender_order_nonce
        + 1     // is_active
        + 1;    // bump

    /// Recompute mid APR from current best lender/borrower APRs.
    pub fn update_mid_apr(&mut self) {
        if self.best_lender_apr_bps > 0 && self.best_borrower_max_apr_bps > 0 {
            self.mid_apr_bps = (self.best_lender_apr_bps + self.best_borrower_max_apr_bps) / 2;
        } else if self.best_lender_apr_bps > 0 {
            self.mid_apr_bps = self.best_lender_apr_bps;
        } else {
            self.mid_apr_bps = self.best_borrower_max_apr_bps;
        }
    }

    /// True if the orderbook has a matchable pair of orders.
    pub fn is_matchable(&self) -> bool {
        self.best_lender_apr_bps > 0
            && self.best_borrower_max_apr_bps > 0
            && self.best_lender_apr_bps <= self.best_borrower_max_apr_bps
    }
}
