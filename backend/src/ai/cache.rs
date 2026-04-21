use anyhow::Result;
use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

/// The shape cached in Redis for a token's AI narrative.
/// Serialized as JSON string under the key `mia:narrative:{token_address}`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CachedNarrative {
    pub token_address: String,
    pub narrative_text: String,
    pub risk_interpretation: Option<String>,
    pub consensus_status: String, // "agreed" | "diverged" | "single_model"
    pub confidence: String,       // "high" | "medium" | "low"
    pub generated_at: DateTime<Utc>,
}

/// Format the Redis key for a token narrative.
pub fn narrative_cache_key(token_address: &str) -> String {
    format!("mia:narrative:{}", token_address.to_lowercase())
}

/// Attempt to load a cached narrative from Redis.
///
/// Returns `Ok(None)` on a cache miss or on Redis errors (best-effort —
/// callers should fall through to the LLM when this returns None).
pub async fn get_cached_narrative(
    redis: &mut redis::aio::ConnectionManager,
    token_address: &str,
) -> Result<Option<CachedNarrative>> {
    let key = narrative_cache_key(token_address);
    let raw: Option<String> = redis.get(&key).await.unwrap_or(None);

    match raw {
        None => Ok(None),
        Some(json) => {
            let narrative: CachedNarrative = serde_json::from_str(&json)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize cached narrative: {}", e))?;
            Ok(Some(narrative))
        }
    }
}

/// Store a narrative in Redis with the given TTL in seconds.
///
/// Errors are logged but NOT propagated — cache writes are best-effort.
/// The narrative is always persisted to PostgreSQL regardless.
pub async fn set_cached_narrative(
    redis: &mut redis::aio::ConnectionManager,
    token_address: &str,
    narrative: &CachedNarrative,
    ttl_secs: u64,
) -> Result<()> {
    let key = narrative_cache_key(token_address);
    let json = serde_json::to_string(narrative)
        .map_err(|e| anyhow::anyhow!("Failed to serialize narrative for cache: {}", e))?;

    redis
        .set_ex::<_, _, ()>(&key, json, ttl_secs)
        .await
        .map_err(|e| anyhow::anyhow!("Redis SET failed: {}", e))?;

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// TDD Tests — pure serialization logic, no live Redis needed
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn sample_narrative(token: &str) -> CachedNarrative {
        CachedNarrative {
            token_address: token.to_string(),
            narrative_text: "Organic growth with healthy liquidity.".to_string(),
            risk_interpretation: Some("Low risk based on deployer history.".to_string()),
            consensus_status: "agreed".to_string(),
            confidence: "high".to_string(),
            generated_at: Utc::now(),
        }
    }

    // ── cache key format ──────────────────────────────────────────────────────

    // RED → GREEN: key is lowercase and uses correct prefix
    #[test]
    fn cache_key_has_correct_prefix() {
        let key = narrative_cache_key("0xABC123");
        assert!(
            key.starts_with("mia:narrative:"),
            "Key must start with mia:narrative:"
        );
    }

    #[test]
    fn cache_key_lowercases_address() {
        let key = narrative_cache_key("0xABCDEF");
        assert_eq!(key, "mia:narrative:0xabcdef");
    }

    #[test]
    fn cache_key_format_is_consistent() {
        assert_eq!(narrative_cache_key("0xabc"), "mia:narrative:0xabc");
    }

    // ── CachedNarrative serialization roundtrip ───────────────────────────────

    // RED → GREEN: serialize and deserialize produces identical struct
    #[test]
    fn serialization_roundtrip_preserves_all_fields() {
        let original = sample_narrative("0xtoken1");
        let json = serde_json::to_string(&original).expect("serialize");
        let restored: CachedNarrative = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(restored.token_address, original.token_address);
        assert_eq!(restored.narrative_text, original.narrative_text);
        assert_eq!(restored.risk_interpretation, original.risk_interpretation);
        assert_eq!(restored.consensus_status, original.consensus_status);
        assert_eq!(restored.confidence, original.confidence);
    }

    // RED → GREEN: optional risk_interpretation serializes as null when None
    #[test]
    fn none_risk_interpretation_serializes_correctly() {
        let mut n = sample_narrative("0xt");
        n.risk_interpretation = None;
        let json = serde_json::to_string(&n).unwrap();
        let restored: CachedNarrative = serde_json::from_str(&json).unwrap();
        assert!(restored.risk_interpretation.is_none());
    }

    // RED → GREEN: invalid JSON deserialization returns error
    #[test]
    fn invalid_json_returns_error() {
        let bad = r#"{"not": "a narrative"}"#;
        // Missing required fields → deserialization should fail
        let result: Result<CachedNarrative, _> = serde_json::from_str(bad);
        assert!(result.is_err(), "Deserializing invalid JSON should fail");
    }

    // RED → GREEN: consensus_status field round-trips correctly
    #[test]
    fn consensus_status_field_round_trips() {
        for status in ["agreed", "diverged", "single_model"] {
            let mut n = sample_narrative("0xt");
            n.consensus_status = status.to_string();
            let json = serde_json::to_string(&n).unwrap();
            let restored: CachedNarrative = serde_json::from_str(&json).unwrap();
            assert_eq!(restored.consensus_status, status);
        }
    }

    // RED → GREEN: confidence field round-trips correctly
    #[test]
    fn confidence_field_round_trips() {
        for level in ["high", "medium", "low"] {
            let mut n = sample_narrative("0xt");
            n.confidence = level.to_string();
            let json = serde_json::to_string(&n).unwrap();
            let restored: CachedNarrative = serde_json::from_str(&json).unwrap();
            assert_eq!(restored.confidence, level);
        }
    }
}
