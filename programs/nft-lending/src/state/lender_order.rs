use anchor_lang::prelude::*;

/// A lender's limit order: "I will lend `principal_lamports` against
/// [collection] at exactly `apr_bps` APR for `duration_seconds`."
///
/// Seeds: ["lender_order", orderbook, lender, nonce_le_bytes (8)]
///
/// The principal SOL is held in this account as lamports above the
/// minimum rent (i.e. the account itself acts as escrow — no separate vault).
/// On cancel, excess lamports above rent are returned. On match, the
/// instruction transfers them to the borrower.
#[account]
pub struct LenderOrder {
    /// The orderbook this order belongs to.
    pub orderbook: Pubkey,

    /// The lender's wallet.
    pub lender: Pubkey,

    /// Amount to lend, in lamports.
    pub principal_lamports: u64,

    /// Offered APR in basis points (e.g. 2000 = 20%).
    /// The lender earns this if matched.
    pub apr_bps: u64,

    /// Unix timestamp after which this order auto-expires (GTC if 0).
    pub expiry_ts: i64,

    /// When this order was placed.
    pub created_at: i64,

    /// Monotonically increasing nonce per lender per orderbook.
    /// Part of the PDA seed so a lender can have multiple orders.
    pub nonce: u64,

    /// PDA bump.
    pub bump: u8,
}

impl LenderOrder {
    pub const SEED: &'static [u8] = b"lender_order";

    pub const SIZE: usize = 8   // discriminator
        + 32    // orderbook
        + 32    // lender
        + 8     // principal_lamports
        + 8     // apr_bps
        + 8     // expiry_ts
        + 8     // created_at
        + 8     // nonce
        + 1;    // bump

    /// Total lamports this account must hold: rent-exempt minimum + principal.
    pub fn required_lamports(rent_minimum: u64, principal: u64) -> u64 {
        rent_minimum + principal
    }

    /// True if this order has expired.
    pub fn is_expired(&self, now: i64) -> bool {
        self.expiry_ts != 0 && now >= self.expiry_ts
    }
}
