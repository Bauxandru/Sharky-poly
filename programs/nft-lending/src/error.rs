use anchor_lang::prelude::*;

#[error_code]
pub enum LendingError {
    #[msg("Unauthorized: signer does not have permission")]
    Unauthorized,
    #[msg("OrderBook is currently inactive")]
    OrderBookInactive,
    #[msg("Amount is below the minimum allowed")]
    AmountTooSmall,
    #[msg("APR value is out of valid range")]
    InvalidApr,
    #[msg("Order has already expired")]
    AlreadyExpired,
    #[msg("Invalid loan duration")]
    InvalidDuration,
    #[msg("Invalid parameter")]
    InvalidParam,
    #[msg("Protocol fee exceeds maximum allowed")]
    FeeTooHigh,
    #[msg("Order belongs to a different orderbook")]
    OrderBookMismatch,
    #[msg("Order has expired and can no longer be matched")]
    OrderExpired,
    #[msg("Lender APR is higher than borrower max APR — orders cannot match")]
    OrdersNotMatchable,
    #[msg("Lender and borrower principal amounts do not match")]
    PrincipalMismatch,
    #[msg("Loan is not in Active state")]
    LoanNotActive,
    #[msg("Loan has not yet expired")]
    LoanNotExpired,
    #[msg("Reward epoch is not currently active")]
    EpochNotActive,
    #[msg("Reward epoch has not ended yet")]
    EpochNotEnded,
    #[msg("Reward epoch is not finalized")]
    EpochNotFinalized,
    #[msg("Rewards have already been claimed for this epoch")]
    AlreadyClaimed,
    #[msg("Maker has no accumulated score")]
    NoScore,
    #[msg("Reward amount is below the minimum payout threshold")]
    RewardTooSmall,
    #[msg("NFT does not belong to the expected collection")]
    WrongCollection,
    #[msg("NFT collection is not verified in Metaplex metadata")]
    CollectionNotVerified,
    #[msg("NFT has no collection in its metadata")]
    NoCollection,
    #[msg("Invalid Metaplex metadata account")]
    InvalidMetadata,
    #[msg("Token account does not hold exactly 1 NFT token")]
    InvalidNft,
}
