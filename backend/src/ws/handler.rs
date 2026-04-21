use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};

use crate::AppState;

/// WebSocket upgrade handler — registered at `GET /ws`.
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Manage a single WebSocket connection lifecycle.
///
/// Spawns a write task that forwards hub messages to the client.
/// The main task reads incoming frames to detect disconnects and client pings.
async fn handle_socket(socket: WebSocket, state: AppState) {
    let (id, mut rx) = state.ws_hub.subscribe().await;
    tracing::debug!(client_id = %id, "WebSocket client connected");

    let (mut sender, mut receiver) = socket.split();

    // Write task: hub messages → WebSocket frames
    let write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let json = match serde_json::to_string(&msg) {
                Ok(j) => j,
                Err(e) => {
                    tracing::error!("WS serialize error: {}", e);
                    continue;
                }
            };
            if sender.send(Message::Text(json.into())).await.is_err() {
                // Client disconnected mid-stream
                break;
            }
        }
    });

    // Read task: handle client-initiated close / ignore other frames
    loop {
        match receiver.next().await {
            Some(Ok(Message::Close(_))) | None => break,
            Some(Ok(Message::Ping(_))) => {
                // Axum's WebSocket layer auto-responds to pings with pongs,
                // so we just need to keep the read loop alive.
            }
            Some(Ok(_)) => {} // ignore binary, text from client
            Some(Err(e)) => {
                tracing::warn!(client_id = %id, error = %e, "WS receive error");
                break;
            }
        }
    }

    write_task.abort();
    state.ws_hub.unsubscribe(id).await;
    tracing::debug!(client_id = %id, "WebSocket client disconnected");
}
