use axum::{
    extract::{Path, Query, State},
    Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::AppState;

#[derive(Deserialize)]
pub struct LoansQuery {
    pub state: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Serialize, FromRow)]
pub struct LoanRow {
    pub pubkey: String,
    pub orderbook: String,
    pub lender: String,
    pub borrower: String,
    pub nft_mint: String,
    pub principal_lamports: i64,
    pub apr_bps: i64,
    pub duration_seconds: i64,
    pub start_ts: i64,
    pub end_ts: i64,
    pub repay_lamports: i64,
    pub state: String,
    pub signature: String,
}

pub async fn list_loans(
    State(state): State<AppState>,
    Path(orderbook): Path<String>,
    Query(q): Query<LoansQuery>,
) -> Result<Json<Vec<LoanRow>>, StatusCode> {
    let loan_state = q.state.unwrap_or_else(|| "active".to_string());
    let limit = q.limit.unwrap_or(50).min(200);

    let rows = sqlx::query_as::<_, LoanRow>(
        r#"
        SELECT pubkey, orderbook, lender, borrower, nft_mint,
               principal_lamports, apr_bps, duration_seconds,
               start_ts, end_ts, repay_lamports, state, signature
        FROM loans
        WHERE orderbook = $1 AND state = $2
        ORDER BY start_ts DESC
        LIMIT $3
        "#,
    )
    .bind(orderbook)
    .bind(loan_state)
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(rows))
}

pub async fn get_loan(
    State(state): State<AppState>,
    Path(pubkey): Path<String>,
) -> Result<Json<LoanRow>, StatusCode> {
    let row = sqlx::query_as::<_, LoanRow>(
        r#"
        SELECT pubkey, orderbook, lender, borrower, nft_mint,
               principal_lamports, apr_bps, duration_seconds,
               start_ts, end_ts, repay_lamports, state, signature
        FROM loans WHERE pubkey = $1
        "#,
    )
    .bind(pubkey)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(row))
}
