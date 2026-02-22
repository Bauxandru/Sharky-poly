use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade, Message}, State},
    response::IntoResponse,
};
use tracing::info;

use super::AppState;

/// WebSocket handler — clients subscribe here for real-time orderbook updates.
/// Each orderbook update, new order, match, or loan event is broadcast.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.ws_tx.subscribe();
    info!("WebSocket client connected");

    loop {
        tokio::select! {
            // Broadcast event to client
            Ok(msg) = rx.recv() => {
                if let Ok(json) = serde_json::to_string(&msg) {
                    if socket.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
            }
            // Handle client message (ping/pong or filter subscription)
            Some(Ok(msg)) = socket.recv() => {
                match msg {
                    Message::Close(_) => break,
                    Message::Ping(data) => {
                        let _ = socket.send(Message::Pong(data)).await;
                    }
                    _ => {}
                }
            }
            else => break,
        }
    }

    info!("WebSocket client disconnected");
}
