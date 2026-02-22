use anchor_lang::prelude::*;

/// Global protocol configuration — single PDA for the whole protocol.
/// Seeds: ["global_state"]
#[account]
pub struct GlobalState {
    /// Admin who can set params and initialize reward epochs.
    pub admin: Pubkey,
    /// Protocol fee in basis points (e.g. 50 = 0.5%).
    /// Taken from borrower interest on each repayment.
    pub protocol_fee_bps: u16,
    /// Fraction of collected fees sent to the reward pool (in bps).
    /// e.g. 10_000 = 100% of fees go to rewards
    pub reward_share_bps: u16,
    /// Treasury wallet that receives the non-reward portion of fees.
    pub treasury: Pubkey,
    /// Reward pool vault (SOL held here for paying out liquidity rewards).
    pub reward_pool_vault: Pubkey,
    /// Total number of orderbooks created (used to emit stats).
    pub total_orderbooks: u64,
    /// Total cumulative loan volume (in lamports).
    pub total_volume_lamports: u64,
    /// Bump seed for the PDA.
    pub bump: u8,
}

impl GlobalState {
    pub const SEED: &'static [u8] = b"global_state";
    /// 8 (discriminator) + layout
    pub const SIZE: usize = 8 + 32 + 2 + 2 + 32 + 32 + 8 + 8 + 1;
}
