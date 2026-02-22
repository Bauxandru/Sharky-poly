use axum::{
    extract::{Path, Query, State},
    Json,
    http::StatusCode,
};
use serde::{Serialize, Deserialize};
use sqlx::FromRow;

use super::AppState;

#[derive(Deserialize)]
pub struct OrdersQuery {
    /// Filter by status: open, filled, cancelled
    pub status: Option<String>,
    /// Max results
    pub limit: Option<i64>,
}

#[derive(Serialize, FromRow)]
pub struct LenderOrderRow {
    pub pubkey: String,
    pub orderbook: String,
    pub lender: String,
    pub principal_lamports: i64,
    pub apr_bps: i64,
    pub expiry_ts: i64,
    pub nonce: i64,
    pub status: String,
    pub slot: i64,
}

#[derive(Serialize, FromRow)]
pub struct BorrowerOrderRow {
    pub pubkey: String,
    pub orderbook: String,
    pub borrower: String,
    pub nft_mint: String,
    pub principal_lamports: i64,
    pub max_apr_bps: i64,
    pub expiry_ts: i64,
    pub status: String,
    pub slot: i64,
}

pub async fn list_lender_orders(
    State(state): State<AppState>,
    Path(orderbook): Path<String>,
    Query(q): Query<OrdersQuery>,
) -> Result<Json<Vec<LenderOrderRow>>, StatusCode> {
    let status = q.status.unwrap_or_else(|| "open".to_string());
    let limit = q.limit.unwrap_or(100).min(500);

    // Return lender orders sorted by APR ascending (best offer first)
    let rows = sqlx::query_as::<_, LenderOrderRow>(
        r#"
        SELECT pubkey, orderbook, lender, principal_lamports, apr_bps,
               expiry_ts, nonce, status, slot
        FROM lender_orders
        WHERE orderbook = $1 AND status = $2
        ORDER BY apr_bps ASC, placed_at ASC
        LIMIT $3
        "#,
    )
    .bind(orderbook)
    .bind(status)
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(rows))
}

pub async fn list_borrower_orders(
    State(state): State<AppState>,
    Path(orderbook): Path<String>,
    Query(q): Query<OrdersQuery>,
) -> Result<Json<Vec<BorrowerOrderRow>>, StatusCode> {
    let status = q.status.unwrap_or_else(|| "open".to_string());
    let limit = q.limit.unwrap_or(100).min(500);

    // Return borrower orders sorted by max_apr descending (best offer first)
    let rows = sqlx::query_as::<_, BorrowerOrderRow>(
        r#"
        SELECT pubkey, orderbook, borrower, nft_mint, principal_lamports,
               max_apr_bps, expiry_ts, status, slot
        FROM borrower_orders
        WHERE orderbook = $1 AND status = $2
        ORDER BY max_apr_bps DESC, placed_at ASC
        LIMIT $3
        "#,
    )
    .bind(orderbook)
    .bind(status)
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(rows))
}
