use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

// ─── Message types ────────────────────────────────────────────────────────────

/// All messages the WebSocket server can push to connected clients.
/// The `type` field is added automatically by serde's `tag` attribute
/// so the frontend can switch on `msg.type`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    /// A new or updated token with its latest on-chain metrics.
    TokenUpdate {
        token_address: String,
        name: Option<String>,
        symbol: Option<String>,
        deployer_address: String,
        buy_count: i32,
        sell_count: i32,
        volume_bnb: f64,
        composite_score: Option<i16>,
        risk_category: Option<String>,
        deployed_at: String,
    },
    /// AI narrative ready for a token.
    NarrativeUpdate {
        token_address: String,
        narrative_text: String,
        risk_interpretation: Option<String>,
        consensus_status: String,
        confidence: String,
    },
    /// Server-sent keepalive.
    Ping,
    /// Client pong response (also used server → client for symmetry).
    Pong,
}

// ─── Broadcast hub ────────────────────────────────────────────────────────────

/// Shared state that manages all active WebSocket connections.
///
/// Each subscriber gets a unique `Uuid` and an unbounded receiver.
/// The hub broadcasts by cloning and sending to every live sender.
/// Dead senders (client disconnected) are cleaned up lazily on the next
/// broadcast.
#[derive(Debug, Clone)]
pub struct WsBroadcastHub {
    clients: Arc<RwLock<HashMap<Uuid, mpsc::UnboundedSender<WsMessage>>>>,
}

impl Default for WsBroadcastHub {
    fn default() -> Self {
        Self::new()
    }
}

impl WsBroadcastHub {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new WebSocket client.
    ///
    /// Returns a `(client_id, receiver)` pair. The handler task reads from the
    /// receiver and writes frames to the WebSocket connection. The id is used
    /// to unsubscribe on disconnect.
    pub async fn subscribe(&self) -> (Uuid, mpsc::UnboundedReceiver<WsMessage>) {
        let id = Uuid::new_v4();
        let (tx, rx) = mpsc::unbounded_channel();
        self.clients.write().await.insert(id, tx);
        (id, rx)
    }

    /// Remove a client from the hub (called on disconnect).
    pub async fn unsubscribe(&self, id: Uuid) {
        self.clients.write().await.remove(&id);
    }

    /// Send `message` to every connected client.
    ///
    /// Clients whose channels have been dropped (disconnected without cleanup)
    /// are silently removed from the registry.
    pub async fn broadcast(&self, message: WsMessage) {
        let clients = self.clients.read().await;
        let mut dead: Vec<Uuid> = Vec::new();

        for (id, tx) in clients.iter() {
            if tx.send(message.clone()).is_err() {
                dead.push(*id);
            }
        }
        drop(clients);

        if !dead.is_empty() {
            let mut clients = self.clients.write().await;
            for id in dead {
                clients.remove(&id);
                tracing::debug!(client_id = %id, "Cleaned up dead WS client");
            }
        }
    }

    /// Current number of connected clients.
    pub async fn client_count(&self) -> usize {
        self.clients.read().await.len()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TDD Tests
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn sample_token_update() -> WsMessage {
        WsMessage::TokenUpdate {
            token_address: "0xabc".to_string(),
            name: Some("PepeCoin".to_string()),
            symbol: Some("PEPE".to_string()),
            deployer_address: "0xdeployer".to_string(),
            buy_count: 10,
            sell_count: 2,
            volume_bnb: 1.5,
            composite_score: Some(35),
            risk_category: Some("medium".to_string()),
            deployed_at: "2026-04-11T10:00:00Z".to_string(),
        }
    }

    // ── WsMessage serialization ───────────────────────────────────────────────

    // RED → GREEN: Ping serializes with type = "ping"
    #[test]
    fn ping_serializes_with_type_field() {
        let json = serde_json::to_string(&WsMessage::Ping).unwrap();
        assert_eq!(json, r#"{"type":"ping"}"#);
    }

    // RED → GREEN: Pong serializes with type = "pong"
    #[test]
    fn pong_serializes_with_type_field() {
        let json = serde_json::to_string(&WsMessage::Pong).unwrap();
        assert_eq!(json, r#"{"type":"pong"}"#);
    }

    // RED → GREEN: TokenUpdate includes type = "token_update"
    #[test]
    fn token_update_serializes_with_correct_type() {
        let json = serde_json::to_value(sample_token_update()).unwrap();
        assert_eq!(json["type"], "token_update");
        assert_eq!(json["token_address"], "0xabc");
        assert_eq!(json["buy_count"], 10);
    }

    // RED → GREEN: NarrativeUpdate includes type = "narrative_update"
    #[test]
    fn narrative_update_serializes_with_correct_type() {
        let msg = WsMessage::NarrativeUpdate {
            token_address: "0xabc".to_string(),
            narrative_text: "Organic growth.".to_string(),
            risk_interpretation: Some("Low risk.".to_string()),
            consensus_status: "agreed".to_string(),
            confidence: "high".to_string(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "narrative_update");
        assert_eq!(json["consensus_status"], "agreed");
    }

    // RED → GREEN: None fields in TokenUpdate serialize as null (not omitted)
    #[test]
    fn token_update_none_fields_serialize_as_null() {
        let msg = WsMessage::TokenUpdate {
            token_address: "0xabc".to_string(),
            name: None,
            symbol: None,
            deployer_address: "0xd".to_string(),
            buy_count: 0,
            sell_count: 0,
            volume_bnb: 0.0,
            composite_score: None,
            risk_category: None,
            deployed_at: "2026-04-11T00:00:00Z".to_string(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert!(json["name"].is_null());
        assert!(json["composite_score"].is_null());
    }

    // ── WsBroadcastHub ────────────────────────────────────────────────────────

    // RED → GREEN: new hub has 0 clients
    #[tokio::test]
    async fn new_hub_has_no_clients() {
        let hub = WsBroadcastHub::new();
        assert_eq!(hub.client_count().await, 0);
    }

    // RED → GREEN: subscribe adds a client
    #[tokio::test]
    async fn subscribe_increments_client_count() {
        let hub = WsBroadcastHub::new();
        let (_id, _rx) = hub.subscribe().await;
        assert_eq!(hub.client_count().await, 1);
    }

    // RED → GREEN: unsubscribe removes the client
    #[tokio::test]
    async fn unsubscribe_decrements_client_count() {
        let hub = WsBroadcastHub::new();
        let (id, _rx) = hub.subscribe().await;
        hub.unsubscribe(id).await;
        assert_eq!(hub.client_count().await, 0);
    }

    // RED → GREEN: subscribe returns unique IDs
    #[tokio::test]
    async fn subscribe_returns_unique_ids() {
        let hub = WsBroadcastHub::new();
        let (id1, _rx1) = hub.subscribe().await;
        let (id2, _rx2) = hub.subscribe().await;
        assert_ne!(id1, id2);
    }

    // RED → GREEN: broadcast delivers message to subscriber
    #[tokio::test]
    async fn broadcast_delivers_message_to_subscriber() {
        let hub = WsBroadcastHub::new();
        let (_id, mut rx) = hub.subscribe().await;

        hub.broadcast(WsMessage::Ping).await;

        let msg = rx.recv().await.expect("should receive message");
        assert!(matches!(msg, WsMessage::Ping));
    }

    // RED → GREEN: broadcast delivers to all subscribers
    #[tokio::test]
    async fn broadcast_delivers_to_all_subscribers() {
        let hub = WsBroadcastHub::new();
        let (_id1, mut rx1) = hub.subscribe().await;
        let (_id2, mut rx2) = hub.subscribe().await;

        hub.broadcast(sample_token_update()).await;

        assert!(rx1.recv().await.is_some());
        assert!(rx2.recv().await.is_some());
    }

    // RED → GREEN: broadcast cleans up dead senders without panicking
    #[tokio::test]
    async fn broadcast_cleans_up_dropped_receiver() {
        let hub = WsBroadcastHub::new();
        let (id, rx) = hub.subscribe().await;
        // Drop the receiver — sender will get an error on next send
        drop(rx);

        // Should not panic
        hub.broadcast(WsMessage::Ping).await;

        // Client should be cleaned up
        assert_eq!(hub.client_count().await, 0);

        // Calling unsubscribe on already-cleaned-up id should be safe
        hub.unsubscribe(id).await;
    }

    // RED → GREEN: multiple broadcasts reach subscriber in order
    #[tokio::test]
    async fn multiple_broadcasts_arrive_in_order() {
        let hub = WsBroadcastHub::new();
        let (_id, mut rx) = hub.subscribe().await;

        hub.broadcast(WsMessage::Ping).await;
        hub.broadcast(WsMessage::Pong).await;

        assert!(matches!(rx.recv().await.unwrap(), WsMessage::Ping));
        assert!(matches!(rx.recv().await.unwrap(), WsMessage::Pong));
    }
}
