use anchor_lang::prelude::*;

/// A borrower's limit order: "I want to borrow `principal_lamports` against
/// `nft_mint` at a maximum of `max_apr_bps` APR."
///
/// Seeds: ["borrower_order", orderbook, nft_mint]
///
/// One active borrower order is allowed per NFT mint at a time.
/// The NFT is frozen in the borrower's token account for the order's lifetime.
/// When matched, it stays frozen for the loan duration.
#[account]
pub struct BorrowerOrder {
    /// The orderbook this order belongs to.
    pub orderbook: Pubkey,

    /// Borrower's wallet (must own the NFT token account).
    pub borrower: Pubkey,

    /// The NFT mint being pledged as collateral.
    pub nft_mint: Pubkey,

    /// The borrower's token account holding the NFT.
    /// Stored so we can freeze/thaw it later without passing it each time.
    pub nft_token_account: Pubkey,

    /// Desired loan amount in lamports.
    pub principal_lamports: u64,

    /// Maximum APR the borrower is willing to pay, in basis points.
    pub max_apr_bps: u64,

    /// Order expiry timestamp (0 = GTC).
    pub expiry_ts: i64,

    /// When the order was placed.
    pub created_at: i64,

    /// PDA bump.
    pub bump: u8,
}

impl BorrowerOrder {
    pub const SEED: &'static [u8] = b"borrower_order";

    pub const SIZE: usize = 8   // discriminator
        + 32    // orderbook
        + 32    // borrower
        + 32    // nft_mint
        + 32    // nft_token_account
        + 8     // principal_lamports
        + 8     // max_apr_bps
        + 8     // expiry_ts
        + 8     // created_at
        + 1;    // bump

    pub fn is_expired(&self, now: i64) -> bool {
        self.expiry_ts != 0 && now >= self.expiry_ts
    }
}
