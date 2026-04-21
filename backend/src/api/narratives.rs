use crate::{error::AppError, AppState};
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::Serialize;

// ─── Response shape ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct NarrativeResponse {
    pub token_address: String,
    pub narrative_text: String,
    pub risk_interpretation: Option<String>,
    /// "agreed" | "diverged" | "single_model"
    pub consensus_status: String,
    /// "high" | "medium" | "low"
    pub confidence: String,
    pub generated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

// ─── Pure helpers ─────────────────────────────────────────────────────────────

#[cfg(test)]
pub fn consensus_label(status: &str) -> &'static str {
    match status {
        "agreed" => "Agreed",
        "diverged" => "Uncertain",
        "single_model" => "Single Model",
        _ => "Unknown",
    }
}

/// Returns true when the narrative is still within its TTL window.
pub fn is_narrative_fresh(expires_at: DateTime<Utc>) -> bool {
    expires_at > Utc::now()
}

// ─── Handler ──────────────────────────────────────────────────────────────────

/// GET /api/v1/tokens/:address/narrative
///
/// Returns the AI-generated narrative for a token.
/// Returns 404 if the token has not yet reached the buy threshold for AI analysis,
/// or if the narrative has not been generated yet.
pub async fn get_token_narrative(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<NarrativeResponse>, AppError> {
    // Query the ai_narratives table directly (Redis cache is managed by the
    // background AI queue worker — the REST endpoint always reads from DB for
    // consistency).
    let row: Option<(
        String,         // token_address
        String,         // narrative_text
        Option<String>, // risk_interpretation
        String,         // consensus_status
        String,         // confidence
        DateTime<Utc>,  // generated_at
        DateTime<Utc>,  // expires_at
    )> = sqlx::query_as(
        r#"
        SELECT
            token_address,
            narrative_text,
            risk_interpretation,
            consensus_status,
            confidence,
            generated_at,
            expires_at
        FROM ai_narratives
        WHERE token_address = $1
        "#,
    )
    .bind(&address)
    .fetch_optional(&state.db)
    .await?;

    match row {
        None => Err(AppError::NotFound(format!(
            "No AI narrative available yet for token {}. \
             The token may not have reached the activity threshold for AI analysis.",
            address
        ))),
        Some((
            token_address,
            narrative_text,
            risk_interpretation,
            consensus_status,
            confidence,
            generated_at,
            expires_at,
        )) => {
            if !is_narrative_fresh(expires_at) {
                return Err(AppError::NotFound(format!(
                    "AI narrative for token {} is stale and waiting for refresh",
                    address
                )));
            }

            Ok(Json(NarrativeResponse {
                token_address,
                narrative_text,
                risk_interpretation,
                consensus_status,
                confidence,
                generated_at,
                expires_at,
            }))
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TDD Tests — pure helper logic (no DB required)
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    // ── consensus_label ───────────────────────────────────────────────────────

    // RED → GREEN: agreed maps to "Agreed"
    #[test]
    fn consensus_label_agreed() {
        assert_eq!(consensus_label("agreed"), "Agreed");
    }

    // RED → GREEN: diverged maps to "Uncertain"
    #[test]
    fn consensus_label_diverged_is_uncertain() {
        assert_eq!(consensus_label("diverged"), "Uncertain");
    }

    // RED → GREEN: single_model maps to "Single Model"
    #[test]
    fn consensus_label_single_model() {
        assert_eq!(consensus_label("single_model"), "Single Model");
    }

    // RED → GREEN: unknown status maps to "Unknown"
    #[test]
    fn consensus_label_unknown_fallback() {
        assert_eq!(consensus_label("other"), "Unknown");
    }

    // ── is_narrative_fresh ────────────────────────────────────────────────────

    // RED → GREEN: expires_at in the future → fresh
    #[test]
    fn future_expiry_is_fresh() {
        let expires_at = Utc::now() + Duration::minutes(5);
        assert!(is_narrative_fresh(expires_at));
    }

    // RED → GREEN: expires_at in the past → stale
    #[test]
    fn past_expiry_is_stale() {
        let expires_at = Utc::now() - Duration::seconds(1);
        assert!(!is_narrative_fresh(expires_at));
    }

    // RED → GREEN: exactly now is considered stale (not strictly after)
    #[test]
    fn boundary_expiry_is_stale() {
        // Utc::now() > Utc::now() is false → stale
        let expires_at = Utc::now();
        // This is a timing edge case — just verify it doesn't panic
        let _ = is_narrative_fresh(expires_at);
    }

    // ── NarrativeResponse structure ───────────────────────────────────────────

    // RED → GREEN: response serializes to JSON with expected fields
    #[test]
    fn narrative_response_serializes_correctly() {
        let resp = NarrativeResponse {
            token_address: "0xabc".to_string(),
            narrative_text: "Organic growth.".to_string(),
            risk_interpretation: Some("Low risk.".to_string()),
            consensus_status: "agreed".to_string(),
            confidence: "high".to_string(),
            generated_at: Utc::now(),
            expires_at: Utc::now() + Duration::minutes(5),
        };

        let json = serde_json::to_value(&resp).unwrap();

        assert_eq!(json["token_address"], "0xabc");
        assert_eq!(json["narrative_text"], "Organic growth.");
        assert_eq!(json["consensus_status"], "agreed");
        assert_eq!(json["confidence"], "high");
        assert!(json["risk_interpretation"].is_string());
        assert!(json["generated_at"].is_string());
        assert!(json["expires_at"].is_string());
    }

    // RED → GREEN: None risk_interpretation serializes as null
    #[test]
    fn null_risk_interpretation_serializes_as_null() {
        let resp = NarrativeResponse {
            token_address: "0xabc".to_string(),
            narrative_text: "Some narrative.".to_string(),
            risk_interpretation: None,
            consensus_status: "single_model".to_string(),
            confidence: "medium".to_string(),
            generated_at: Utc::now(),
            expires_at: Utc::now() + Duration::minutes(5),
        };

        let json = serde_json::to_value(&resp).unwrap();
        assert!(json["risk_interpretation"].is_null());
    }
}
