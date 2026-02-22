use axum::{
    extract::{Path, State},
    Json,
    http::StatusCode,
};
use serde::Serialize;
use sqlx::FromRow;

use super::AppState;

#[derive(Serialize, FromRow)]
pub struct OrderBookRow {
    pub pubkey: String,
    pub collection_mint: String,
    pub duration_seconds: i64,
    pub best_lender_apr_bps: i64,
    pub best_borrower_max_apr_bps: i64,
    pub mid_apr_bps: i64,
    pub max_incentive_spread_bps: Option<i64>,
    pub open_lender_orders: i64,
    pub open_borrower_orders: i64,
    pub active_loans: i64,
    pub total_loans: i64,
    pub total_volume_lamports: i64,
    pub is_active: bool,
}

pub async fn list_orderbooks(
    State(state): State<AppState>,
) -> Result<Json<Vec<OrderBookRow>>, StatusCode> {
    let rows = sqlx::query_as::<_, OrderBookRow>(
        r#"
        SELECT pubkey, collection_mint, duration_seconds,
               best_lender_apr_bps, best_borrower_max_apr_bps, mid_apr_bps,
               max_incentive_spread_bps,
               open_lender_orders, open_borrower_orders,
               active_loans, total_loans, total_volume_lamports, is_active
        FROM orderbooks
        WHERE is_active = true
        ORDER BY total_volume_lamports DESC
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(rows))
}

pub async fn get_orderbook(
    State(state): State<AppState>,
    Path(pubkey): Path<String>,
) -> Result<Json<OrderBookRow>, StatusCode> {
    let row = sqlx::query_as::<_, OrderBookRow>(
        r#"
        SELECT pubkey, collection_mint, duration_seconds,
               best_lender_apr_bps, best_borrower_max_apr_bps, mid_apr_bps,
               max_incentive_spread_bps,
               open_lender_orders, open_borrower_orders,
               active_loans, total_loans, total_volume_lamports, is_active
        FROM orderbooks
        WHERE pubkey = $1
        "#,
    )
    .bind(pubkey)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(row))
}
