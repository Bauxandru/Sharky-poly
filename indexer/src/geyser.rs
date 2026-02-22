/// Yellowstone gRPC subscriber.
///
/// Instead of polling `getProgramAccounts` (which costs $$$ at scale),
/// we maintain ONE streaming subscription that pushes every program
/// account change and transaction to us instantly.
///
/// This is Helius/Triton's "Geyser" plugin interface — the same data
/// that goes into Solana's mempool/validator state, streamed to us.

use anyhow::Result;
use sqlx::PgPool;
use tokio_stream::StreamExt;
use tracing::{debug, error, info, warn};

use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::prelude::{
    subscribe_update::UpdateOneof,
    CommitmentLevel, SubscribeRequest, SubscribeRequestFilterTransactions,
};
use std::collections::HashMap;

use crate::parser;
use crate::ws::WsBroadcaster;

pub async fn run_subscriber(
    endpoint: &str,
    api_key: &str,
    program_id: &str,
    pool: PgPool,
    ws_tx: WsBroadcaster,
) -> Result<()> {
    let mut client = GeyserGrpcClient::build_from_shared(endpoint.to_string())?
        .x_token(Some(api_key.to_string()))?
        .connect()
        .await?;

    info!("Connected to Yellowstone gRPC. Subscribing to program {}...", program_id);

    // Subscribe to:
    // 1. All transactions touching our program (for event/instruction parsing)
    // 2. All account updates for our program (for state sync)
    let mut tx_filter = HashMap::new();
    tx_filter.insert(
        "nft_lending_txs".to_string(),
        SubscribeRequestFilterTransactions {
            vote: Some(false),
            failed: Some(false),
            account_include: vec![program_id.to_string()],
            account_exclude: vec![],
            account_required: vec![],
            signature: None,
        },
    );

    let request = SubscribeRequest {
        transactions: tx_filter,
        commitment: Some(CommitmentLevel::Confirmed as i32),
        ..Default::default()
    };

    let (_, mut stream) = client.subscribe_with_request(Some(request)).await?;

    info!("Subscription active. Processing events...");

    while let Some(msg) = stream.next().await {
        match msg {
            Ok(update) => {
                match update.update_oneof {
                    Some(UpdateOneof::Transaction(tx_update)) => {
                        if let Err(e) = parser::process_transaction(
                            tx_update,
                            &pool,
                            &ws_tx,
                        )
                        .await
                        {
                            warn!("Error processing transaction: {}", e);
                        }
                    }
                    Some(UpdateOneof::Ping(_)) => {
                        debug!("gRPC ping received");
                    }
                    Some(UpdateOneof::Pong(_)) => {}
                    _ => {}
                }
            }
            Err(e) => {
                error!("gRPC stream error: {}", e);
                return Err(e.into());
            }
        }
    }

    Ok(())
}
