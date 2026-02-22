/// Parse Anchor program events from confirmed transactions.
///
/// Anchor emits events as base64-encoded borsh data in transaction logs
/// with the prefix "Program data: ". We decode these to get typed events.

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use borsh::BorshDeserialize;
use sha2::{Sha256, Digest};
use sqlx::PgPool;
use std::sync::OnceLock;
use tracing::{debug, warn};
use yellowstone_grpc_proto::prelude::SubscribeUpdateTransaction;

use crate::db;
use crate::ws::{WsBroadcaster, WsMessage};

// ─── Discriminators (computed once at startup) ────────────────────────────────

/// Compute an Anchor event discriminator: sha256("event:<EventName>")[0..8]
fn anchor_event_disc(event_name: &str) -> [u8; 8] {
    let preimage = format!("event:{}", event_name);
    let hash = Sha256::digest(preimage.as_bytes());
    hash[0..8].try_into().unwrap()
}

struct Discriminators {
    lender_order_placed:      [u8; 8],
    lender_order_cancelled:   [u8; 8],
    borrower_order_placed:    [u8; 8],
    borrower_order_cancelled: [u8; 8],
    loan_created:             [u8; 8],
    loan_repaid:              [u8; 8],
    loan_foreclosed:          [u8; 8],
    orderbook_created:        [u8; 8],
    reward_epoch_created:     [u8; 8],
    reward_claimed:           [u8; 8],
}

static DISCS: OnceLock<Discriminators> = OnceLock::new();

fn discs() -> &'static Discriminators {
    DISCS.get_or_init(|| Discriminators {
        lender_order_placed:      anchor_event_disc("LenderOrderPlaced"),
        lender_order_cancelled:   anchor_event_disc("LenderOrderCancelled"),
        borrower_order_placed:    anchor_event_disc("BorrowerOrderPlaced"),
        borrower_order_cancelled: anchor_event_disc("BorrowerOrderCancelled"),
        loan_created:             anchor_event_disc("LoanCreated"),
        loan_repaid:              anchor_event_disc("LoanRepaid"),
        loan_foreclosed:          anchor_event_disc("LoanForeclosed"),
        orderbook_created:        anchor_event_disc("OrderBookCreated"),
        reward_epoch_created:     anchor_event_disc("RewardEpochCreated"),
        reward_claimed:           anchor_event_disc("RewardClaimed"),
    })
}

// ─── Transaction processor ────────────────────────────────────────────────────

pub async fn process_transaction(
    tx_update: SubscribeUpdateTransaction,
    pool: &PgPool,
    ws_tx: &WsBroadcaster,
) -> Result<()> {
    let tx = tx_update.transaction.ok_or_else(|| anyhow!("no transaction"))?;
    let sig = bs58::encode(&tx.signature).into_string();
    let slot = tx_update.slot;

    let meta = tx.meta.ok_or_else(|| anyhow!("no meta"))?;
    if meta.err.is_some() {
        return Ok(());
    }

    for log in &meta.log_messages {
        if let Some(b64) = log.strip_prefix("Program data: ") {
            if let Ok(data) = BASE64.decode(b64) {
                if data.len() < 8 {
                    continue;
                }
                let disc: [u8; 8] = data[0..8].try_into().unwrap();
                let payload = &data[8..];
                if let Err(e) = dispatch_event(disc, payload, &sig, slot, pool, ws_tx).await {
                    warn!("Event dispatch error for {}: {}", sig, e);
                }
            }
        }
    }

    Ok(())
}

async fn dispatch_event(
    discriminator: [u8; 8],
    data: &[u8],
    signature: &str,
    slot: u64,
    pool: &PgPool,
    ws_tx: &WsBroadcaster,
) -> Result<()> {
    let d = discs();

    if discriminator == d.orderbook_created {
        #[derive(BorshDeserialize)]
        struct E { orderbook: [u8; 32], collection_mint: [u8; 32], duration_seconds: u64, creator: [u8; 32] }
        let e = E::try_from_slice(data)?;
        db::upsert_orderbook(pool, &pk58(&e.orderbook), &pk58(&e.collection_mint), e.duration_seconds).await?;
        let _ = ws_tx.send(WsMessage::OrderBookUpdate { orderbook: pk58(&e.orderbook) });

    } else if discriminator == d.lender_order_placed {
        #[derive(BorshDeserialize)]
        struct E { orderbook: [u8; 32], order: [u8; 32], lender: [u8; 32], principal_lamports: u64, apr_bps: u64, expiry_ts: i64, nonce: u64 }
        let e = E::try_from_slice(data)?;
        db::upsert_lender_order(pool, &pk58(&e.order), &pk58(&e.orderbook), &pk58(&e.lender), e.principal_lamports, e.apr_bps, e.expiry_ts, e.nonce, signature, slot).await?;
        let _ = ws_tx.send(WsMessage::LenderOrderPlaced { orderbook: pk58(&e.orderbook), order: pk58(&e.order), apr_bps: e.apr_bps, principal_lamports: e.principal_lamports });

    } else if discriminator == d.lender_order_cancelled {
        #[derive(BorshDeserialize)]
        struct E { orderbook: [u8; 32], order: [u8; 32], lender: [u8; 32], principal_lamports: u64 }
        let e = E::try_from_slice(data)?;
        db::cancel_lender_order(pool, &pk58(&e.order)).await?;
        let _ = ws_tx.send(WsMessage::LenderOrderCancelled { orderbook: pk58(&e.orderbook), order: pk58(&e.order) });

    } else if discriminator == d.borrower_order_placed {
        #[derive(BorshDeserialize)]
        struct E { orderbook: [u8; 32], order: [u8; 32], borrower: [u8; 32], nft_mint: [u8; 32], principal_lamports: u64, max_apr_bps: u64, expiry_ts: i64 }
        let e = E::try_from_slice(data)?;
        db::upsert_borrower_order(pool, &pk58(&e.order), &pk58(&e.orderbook), &pk58(&e.borrower), &pk58(&e.nft_mint), e.principal_lamports, e.max_apr_bps, e.expiry_ts, signature, slot).await?;
        let _ = ws_tx.send(WsMessage::BorrowerOrderPlaced { orderbook: pk58(&e.orderbook), order: pk58(&e.order), max_apr_bps: e.max_apr_bps, principal_lamports: e.principal_lamports });

    } else if discriminator == d.borrower_order_cancelled {
        #[derive(BorshDeserialize)]
        struct E { orderbook: [u8; 32], order: [u8; 32], borrower: [u8; 32], nft_mint: [u8; 32] }
        let e = E::try_from_slice(data)?;
        db::cancel_borrower_order(pool, &pk58(&e.order)).await?;
        let _ = ws_tx.send(WsMessage::BorrowerOrderCancelled { orderbook: pk58(&e.orderbook), order: pk58(&e.order) });

    } else if discriminator == d.loan_created {
        #[derive(BorshDeserialize)]
        struct E { loan: [u8; 32], orderbook: [u8; 32], lender: [u8; 32], borrower: [u8; 32], nft_mint: [u8; 32], principal_lamports: u64, apr_bps: u64, duration_seconds: u64, start_ts: i64, end_ts: i64, repay_lamports: u64 }
        let e = E::try_from_slice(data)?;
        db::insert_loan(pool, &pk58(&e.loan), &pk58(&e.orderbook), &pk58(&e.lender), &pk58(&e.borrower), &pk58(&e.nft_mint), e.principal_lamports, e.apr_bps, e.duration_seconds, e.start_ts, e.end_ts, e.repay_lamports, signature).await?;
        let _ = ws_tx.send(WsMessage::LoanCreated { orderbook: pk58(&e.orderbook), loan: pk58(&e.loan), apr_bps: e.apr_bps, principal_lamports: e.principal_lamports });

    } else if discriminator == d.loan_repaid {
        #[derive(BorshDeserialize)]
        struct E { loan: [u8; 32], orderbook: [u8; 32], lender: [u8; 32], borrower: [u8; 32], nft_mint: [u8; 32], principal_lamports: u64, repay_lamports: u64, protocol_fee_lamports: u64, repaid_at: i64 }
        let e = E::try_from_slice(data)?;
        db::mark_loan_repaid(pool, &pk58(&e.loan), e.repaid_at).await?;

    } else if discriminator == d.loan_foreclosed {
        #[derive(BorshDeserialize)]
        struct E { loan: [u8; 32], orderbook: [u8; 32], lender: [u8; 32], borrower: [u8; 32], nft_mint: [u8; 32], principal_lamports: u64, foreclosed_at: i64 }
        let e = E::try_from_slice(data)?;
        db::mark_loan_foreclosed(pool, &pk58(&e.loan), e.foreclosed_at).await?;

    } else {
        debug!("Unknown event discriminator: {:?}", discriminator);
    }

    Ok(())
}

#[inline]
fn pk58(bytes: &[u8; 32]) -> String {
    bs58::encode(bytes).into_string()
}
