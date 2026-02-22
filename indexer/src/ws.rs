/// WebSocket broadcaster for real-time orderbook updates to the frontend.

use serde::Serialize;
use tokio::sync::broadcast;

pub type WsBroadcaster = broadcast::Sender<WsMessage>;

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    OrderBookUpdate {
        orderbook: String,
    },
    LenderOrderPlaced {
        orderbook: String,
        order: String,
        apr_bps: u64,
        principal_lamports: u64,
    },
    LenderOrderCancelled {
        orderbook: String,
        order: String,
    },
    BorrowerOrderPlaced {
        orderbook: String,
        order: String,
        max_apr_bps: u64,
        principal_lamports: u64,
    },
    BorrowerOrderCancelled {
        orderbook: String,
        order: String,
    },
    LoanCreated {
        orderbook: String,
        loan: String,
        apr_bps: u64,
        principal_lamports: u64,
    },
}
