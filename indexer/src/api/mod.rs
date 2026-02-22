use anyhow::Result;
use axum::{
    Router,
    routing::get,
};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::ws::WsBroadcaster;

pub mod orderbooks;
pub mod orders;
pub mod loans;
pub mod rewards;
pub mod websocket;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub ws_tx: WsBroadcaster,
}

pub async fn serve(pool: PgPool, ws_tx: WsBroadcaster, port: u16) -> Result<()> {
    let state = AppState { pool, ws_tx };

    let app = Router::new()
        // ─── REST endpoints ───────────────────────────────────────────────
        .route("/api/orderbooks", get(orderbooks::list_orderbooks))
        .route("/api/orderbooks/:pubkey", get(orderbooks::get_orderbook))
        .route("/api/orderbooks/:pubkey/lender-orders", get(orders::list_lender_orders))
        .route("/api/orderbooks/:pubkey/borrower-orders", get(orders::list_borrower_orders))
        .route("/api/orderbooks/:pubkey/loans", get(loans::list_loans))
        .route("/api/orderbooks/:pubkey/rewards/:epoch_id", get(rewards::get_epoch))
        .route("/api/loans/:pubkey", get(loans::get_loan))
        .route("/api/makers/:wallet/scores", get(rewards::get_maker_scores))
        // ─── WebSocket ────────────────────────────────────────────────────
        .route("/ws", get(websocket::ws_handler))
        // ─── Health ───────────────────────────────────────────────────────
        .route("/health", get(|| async { "ok" }))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("API listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
