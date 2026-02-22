use axum::{
    extract::{Path, State},
    Json,
    http::StatusCode,
};
use serde::Serialize;
use sqlx::FromRow;

use super::AppState;

#[derive(Serialize, FromRow)]
pub struct RewardEpochRow {
    pub orderbook: String,
    pub epoch_id: i64,
    pub start_ts: i64,
    pub end_ts: i64,
    pub total_reward_lamports: i64,
    pub total_score: i64,
    pub finalized: bool,
}

#[derive(Serialize, FromRow)]
pub struct MakerScoreRow {
    pub maker: String,
    pub orderbook: String,
    pub epoch_id: i64,
    pub score: i64,
    pub claimed: bool,
    /// Estimated reward in lamports (score / total_score * reward_pool)
    pub estimated_reward_lamports: Option<i64>,
}

pub async fn get_epoch(
    State(state): State<AppState>,
    Path((orderbook, epoch_id)): Path<(String, i64)>,
) -> Result<Json<RewardEpochRow>, StatusCode> {
    let row = sqlx::query_as::<_, RewardEpochRow>(
        r#"
        SELECT orderbook, epoch_id, start_ts, end_ts,
               total_reward_lamports, total_score, finalized
        FROM reward_epochs
        WHERE orderbook = $1 AND epoch_id = $2
        "#,
    )
    .bind(orderbook)
    .bind(epoch_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(row))
}

pub async fn get_maker_scores(
    State(state): State<AppState>,
    Path(wallet): Path<String>,
) -> Result<Json<Vec<MakerScoreRow>>, StatusCode> {
    let rows = sqlx::query_as::<_, MakerScoreRow>(
        r#"
        SELECT ms.maker, ms.orderbook, ms.epoch_id, ms.score, ms.claimed,
               CASE WHEN re.total_score > 0
                    THEN ms.score * re.total_reward_lamports / re.total_score
                    ELSE 0
               END as estimated_reward_lamports
        FROM maker_scores ms
        JOIN reward_epochs re
          ON re.orderbook = ms.orderbook AND re.epoch_id = ms.epoch_id
        WHERE ms.maker = $1
        ORDER BY ms.epoch_id DESC
        LIMIT 50
        "#,
    )
    .bind(wallet)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(rows))
}
