use crate::{error::AppError, AppState};
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::Serialize;

// ─── Response shapes ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct DeployerResponse {
    pub address: String,
    pub total_tokens_deployed: i64,
    pub rug_count: i64,
    pub graduated_count: i64,
    pub honeypot_detected: bool,
    pub trust_grade: String,
    pub trust_label: String,
    pub first_seen_at: Option<DateTime<Utc>>,
    pub last_seen_at: Option<DateTime<Utc>>,
}

// ─── Handler ──────────────────────────────────────────────────────────────────

/// GET /api/v1/deployer/:address
pub async fn get_deployer(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<DeployerResponse>, AppError> {
    use crate::indexer::deployer::get_deployer_profile;

    let profile = get_deployer_profile(&state.db, &address).await?;

    match profile {
        None => Err(AppError::NotFound(format!(
            "Deployer {} not found",
            address
        ))),
        Some(p) => Ok(Json(DeployerResponse {
            address: p.address,
            total_tokens_deployed: p.total_tokens_deployed,
            rug_count: p.rug_count,
            graduated_count: p.graduated_count,
            honeypot_detected: p.honeypot_detected,
            trust_grade: p.trust_grade.as_str().to_string(),
            trust_label: p.trust_grade.label().to_string(),
            first_seen_at: p.first_seen_at,
            last_seen_at: p.last_seen_at,
        })),
    }
}
