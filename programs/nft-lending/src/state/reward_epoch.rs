use anchor_lang::prelude::*;

/// A reward epoch defines a time window and reward pool for liquidity providers.
///
/// Seeds: ["reward_epoch", orderbook, epoch_id_le_bytes (8)]
#[account]
pub struct RewardEpoch {
    /// Orderbook this epoch applies to.
    pub orderbook: Pubkey,

    /// Monotonically increasing epoch identifier.
    pub epoch_id: u64,

    /// Start of the epoch (Unix timestamp).
    pub start_ts: i64,

    /// End of the epoch (Unix timestamp).
    pub end_ts: i64,

    /// Total SOL (lamports) in the reward pool for this epoch.
    pub total_reward_lamports: u64,

    /// Sum of all maker scores accumulated during the epoch.
    /// Used to compute each maker's share: reward_i = score_i / total_score * pool
    pub total_score: u64,

    /// Whether rewards have been finalized (epoch ended, scoring done).
    pub finalized: bool,

    /// PDA bump.
    pub bump: u8,
}

impl RewardEpoch {
    pub const SEED: &'static [u8] = b"reward_epoch";

    pub const SIZE: usize = 8   // discriminator
        + 32    // orderbook
        + 8     // epoch_id
        + 8     // start_ts
        + 8     // end_ts
        + 8     // total_reward_lamports
        + 8     // total_score
        + 1     // finalized
        + 1;    // bump

    pub fn is_active(&self, now: i64) -> bool {
        now >= self.start_ts && now < self.end_ts && !self.finalized
    }
}

/// Tracks a single maker's accumulated score for one epoch.
///
/// Seeds: ["maker_score", maker, orderbook, epoch_id_le_bytes (8)]
#[account]
pub struct MakerScore {
    /// The maker (lender or borrower who placed a qualifying order).
    pub maker: Pubkey,

    /// Orderbook.
    pub orderbook: Pubkey,

    /// Epoch this score belongs to.
    pub epoch_id: u64,

    /// Accumulated quadratic score (raw integer, scale = 1e6).
    /// Increased by the crank bot's off-chain score submission, or by
    /// on-chain sampling instructions.
    pub score: u64,

    /// True once rewards have been claimed.
    pub claimed: bool,

    /// PDA bump.
    pub bump: u8,
}

impl MakerScore {
    pub const SEED: &'static [u8] = b"maker_score";

    pub const SIZE: usize = 8   // discriminator
        + 32    // maker
        + 32    // orderbook
        + 8     // epoch_id
        + 8     // score
        + 1     // claimed
        + 1;    // bump
}
