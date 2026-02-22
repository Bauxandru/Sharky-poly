use anchor_lang::prelude::*;

/// Emitted cheaply via `emit!()` — indexed by the gRPC subscriber at near-zero cost.
/// These replace expensive `getProgramAccounts` polling.

#[event]
pub struct GlobalInitialized {
    pub admin: Pubkey,
    pub protocol_fee_bps: u16,
    pub reward_share_bps: u16,
}

#[event]
pub struct OrderBookCreated {
    pub orderbook: Pubkey,
    pub collection_mint: Pubkey,
    pub duration_seconds: u64,
    pub creator: Pubkey,
}

#[event]
pub struct LenderOrderPlaced {
    pub orderbook: Pubkey,
    pub order: Pubkey,
    pub lender: Pubkey,
    pub principal_lamports: u64,
    pub apr_bps: u64,
    pub expiry_ts: i64,
    pub nonce: u64,
}

#[event]
pub struct LenderOrderCancelled {
    pub orderbook: Pubkey,
    pub order: Pubkey,
    pub lender: Pubkey,
    pub principal_lamports: u64,
}

#[event]
pub struct BorrowerOrderPlaced {
    pub orderbook: Pubkey,
    pub order: Pubkey,
    pub borrower: Pubkey,
    pub nft_mint: Pubkey,
    pub principal_lamports: u64,
    pub max_apr_bps: u64,
    pub expiry_ts: i64,
}

#[event]
pub struct BorrowerOrderCancelled {
    pub orderbook: Pubkey,
    pub order: Pubkey,
    pub borrower: Pubkey,
    pub nft_mint: Pubkey,
}

#[event]
pub struct LoanCreated {
    pub loan: Pubkey,
    pub orderbook: Pubkey,
    pub lender: Pubkey,
    pub borrower: Pubkey,
    pub nft_mint: Pubkey,
    pub principal_lamports: u64,
    pub apr_bps: u64,
    pub duration_seconds: u64,
    pub start_ts: i64,
    pub end_ts: i64,
    pub repay_lamports: u64,
}

#[event]
pub struct LoanRepaid {
    pub loan: Pubkey,
    pub orderbook: Pubkey,
    pub lender: Pubkey,
    pub borrower: Pubkey,
    pub nft_mint: Pubkey,
    pub principal_lamports: u64,
    pub repay_lamports: u64,
    pub protocol_fee_lamports: u64,
    pub repaid_at: i64,
}

#[event]
pub struct LoanForeclosed {
    pub loan: Pubkey,
    pub orderbook: Pubkey,
    pub lender: Pubkey,
    pub borrower: Pubkey,
    pub nft_mint: Pubkey,
    pub principal_lamports: u64,
    pub foreclosed_at: i64,
}

#[event]
pub struct RewardEpochCreated {
    pub orderbook: Pubkey,
    pub epoch_id: u64,
    pub start_ts: i64,
    pub end_ts: i64,
    pub total_reward_lamports: u64,
}

#[event]
pub struct RewardClaimed {
    pub maker: Pubkey,
    pub orderbook: Pubkey,
    pub epoch_id: u64,
    pub reward_lamports: u64,
}
