export interface OrderBook {
  pubkey: string;
  collection_mint: string;
  duration_seconds: number;
  best_lender_apr_bps: number;
  best_borrower_max_apr_bps: number;
  mid_apr_bps: number;
  max_incentive_spread_bps: number | null;
  open_lender_orders: number;
  open_borrower_orders: number;
  active_loans: number;
  total_loans: number;
  total_volume_lamports: number;
  is_active: boolean;
}

export interface LenderOrder {
  pubkey: string;
  orderbook: string;
  lender: string;
  principal_lamports: number;
  apr_bps: number;
  expiry_ts: number;
  nonce: number;
  status: "open" | "filled" | "cancelled";
  slot: number;
}

export interface BorrowerOrder {
  pubkey: string;
  orderbook: string;
  borrower: string;
  nft_mint: string;
  principal_lamports: number;
  max_apr_bps: number;
  expiry_ts: number;
  status: "open" | "filled" | "cancelled";
  slot: number;
}

export interface Loan {
  pubkey: string;
  orderbook: string;
  lender: string;
  borrower: string;
  nft_mint: string;
  principal_lamports: number;
  apr_bps: number;
  duration_seconds: number;
  start_ts: number;
  end_ts: number;
  repay_lamports: number;
  state: "active" | "repaid" | "foreclosed";
  signature: string;
}

export interface MakerScore {
  maker: string;
  orderbook: string;
  epoch_id: number;
  score: number;
  claimed: boolean;
  estimated_reward_lamports: number | null;
}
