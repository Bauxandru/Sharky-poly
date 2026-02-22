/// NFT Lending Indexer
///
/// Cost strategy:
/// - ONE gRPC subscription to Yellowstone (Helius/Triton) for all program events.
///   This replaces thousands of `getProgramAccounts` RPC polls per day.
/// - All state stored in PostgreSQL — frontend reads from DB, not RPC.
/// - Axum serves REST + WebSocket from the same process.
/// - Estimated infra cost: ~$80-150/month vs Sharky's $2000+/month.

use anyhow::Result;
use dotenvy::dotenv;
use std::env;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod db;
mod geyser;
mod parser;
mod rewards;
mod api;
mod ws;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let grpc_endpoint = env::var("GRPC_ENDPOINT")
        .expect("GRPC_ENDPOINT must be set (e.g. https://mainnet.helius-rpc.com:443)");
    let grpc_api_key = env::var("GRPC_API_KEY")
        .unwrap_or_default();
    let program_id = env::var("PROGRAM_ID")
        .unwrap_or_else(|_| "NLend1111111111111111111111111111111111111111".to_string());
    let api_port: u16 = env::var("API_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()?;

    info!("Connecting to database...");
    let pool = db::connect(&database_url).await?;
    db::run_migrations(&pool).await?;

    info!("Starting gRPC subscriber to {}", grpc_endpoint);
    let pool_clone = pool.clone();

    // WebSocket broadcaster for real-time UI updates
    let (ws_tx, _ws_rx) = tokio::sync::broadcast::channel::<ws::WsMessage>(1024);

    // Run gRPC indexer in background
    let ws_tx_clone = ws_tx.clone();
    let program_id_clone = program_id.clone();
    tokio::spawn(async move {
        loop {
            match geyser::run_subscriber(
                &grpc_endpoint,
                &grpc_api_key,
                &program_id_clone,
                pool_clone.clone(),
                ws_tx_clone.clone(),
            )
            .await
            {
                Ok(_) => info!("gRPC subscriber ended cleanly, restarting..."),
                Err(e) => tracing::error!("gRPC subscriber error: {}, restarting in 5s...", e),
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });

    // Run reward scorer in background (samples every 60 seconds)
    let pool_for_scorer = pool.clone();
    tokio::spawn(async move {
        rewards::scorer::run_scorer(pool_for_scorer).await;
    });

    // Start REST + WebSocket API
    info!("Starting API server on port {}", api_port);
    api::serve(pool, ws_tx, api_port).await?;

    Ok(())
}
