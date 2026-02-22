use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum LoanState {
    /// Loan is live, awaiting repayment.
    Active,
    /// Borrower repaid. Closed account, but stored for history via event.
    Repaid,
    /// Lender foreclosed after expiry. NFT transferred to lender.
    Foreclosed,
}

/// An active loan created when a lender order matches a borrower order.
///
/// Seeds: ["loan", lender_order, borrower_order]
///
/// The loan account is closed (rent returned) after repayment or foreclosure.
#[account]
pub struct Loan {
    /// The orderbook this loan came from.
    pub orderbook: Pubkey,

    /// Lender's wallet.
    pub lender: Pubkey,

    /// Borrower's wallet.
    pub borrower: Pubkey,

    /// NFT mint used as collateral.
    pub nft_mint: Pubkey,

    /// Borrower's token account holding the frozen NFT.
    pub nft_token_account: Pubkey,

    /// Lender order that was matched (for PDA derivation history).
    pub lender_order: Pubkey,

    /// Borrower order that was matched.
    pub borrower_order: Pubkey,

    /// Principal amount sent to borrower in lamports.
    pub principal_lamports: u64,

    /// APR at which the loan was matched, in basis points.
    /// (The lender's APR, since lender is maker.)
    pub apr_bps: u64,

    /// Loan duration in seconds (copied from orderbook at match time).
    pub duration_seconds: u64,

    /// Unix timestamp when the loan started.
    pub start_ts: i64,

    /// Unix timestamp when the loan expires (start_ts + duration_seconds).
    pub end_ts: i64,

    /// Total amount the borrower must repay (principal + interest + protocol fee).
    pub repay_lamports: u64,

    /// Protocol fee component of repay_lamports (goes to reward pool / treasury).
    pub protocol_fee_lamports: u64,

    /// Current loan state.
    pub state: LoanState,

    /// PDA bump.
    pub bump: u8,
}

impl Loan {
    pub const SEED: &'static [u8] = b"loan";

    pub const SIZE: usize = 8   // discriminator
        + 32    // orderbook
        + 32    // lender
        + 32    // borrower
        + 32    // nft_mint
        + 32    // nft_token_account
        + 32    // lender_order
        + 32    // borrower_order
        + 8     // principal_lamports
        + 8     // apr_bps
        + 8     // duration_seconds
        + 8     // start_ts
        + 8     // end_ts
        + 8     // repay_lamports
        + 8     // protocol_fee_lamports
        + 1     // state
        + 1;    // bump

    /// Compute interest + fee given principal, APR in bps, duration in seconds.
    /// Interest = principal * apr_bps / 10_000 * (duration / seconds_in_year)
    pub fn compute_repay(
        principal: u64,
        apr_bps: u64,
        duration_seconds: u64,
        protocol_fee_bps: u16,
    ) -> (u64, u64) {
        const SECONDS_PER_YEAR: u64 = 31_536_000;
        // Use u128 to avoid overflow on large amounts
        let interest = (principal as u128)
            .saturating_mul(apr_bps as u128)
            .saturating_mul(duration_seconds as u128)
            / (10_000u128 * SECONDS_PER_YEAR as u128);

        let protocol_fee = (interest as u128)
            .saturating_mul(protocol_fee_bps as u128)
            / 10_000u128;

        let repay = principal as u128 + interest;
        (repay as u64, protocol_fee as u64)
    }

    pub fn is_expired(&self, now: i64) -> bool {
        now >= self.end_ts
    }
}
