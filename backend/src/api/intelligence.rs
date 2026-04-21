use crate::{error::AppError, AppState};
use axum::{extract::State, Json};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct IntelligenceSummaryResponse {
    pub total_tokens: i64,
    pub low_risk_tokens: i64,
    pub medium_risk_tokens: i64,
    pub high_risk_tokens: i64,
    pub total_whale_alerts_24h: i64,
    pub latest_alpha_window_end: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn get_intelligence_summary(
    State(state): State<AppState>,
) -> Result<Json<IntelligenceSummaryResponse>, AppError> {
    let token_counts: (i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*)::bigint AS total,
            COUNT(*) FILTER (WHERE rs.composite_score BETWEEN 0 AND 30)::bigint AS low,
            COUNT(*) FILTER (WHERE rs.composite_score BETWEEN 31 AND 60)::bigint AS medium,
            COUNT(*) FILTER (WHERE rs.composite_score BETWEEN 61 AND 100)::bigint AS high
        FROM tokens t
        LEFT JOIN risk_scores rs ON rs.token_address = t.contract_address
        "#,
    )
    .fetch_one(&state.db)
    .await?;

    let whale_count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)::bigint
        FROM whale_alerts
        WHERE created_at >= NOW() - INTERVAL '24 hours'
        "#,
    )
    .fetch_one(&state.db)
    .await?;

    let latest_alpha: (Option<chrono::DateTime<chrono::Utc>>,) =
        sqlx::query_as("SELECT MAX(window_end) FROM alpha_rankings")
            .fetch_one(&state.db)
            .await?;

    Ok(Json(IntelligenceSummaryResponse {
        total_tokens: token_counts.0,
        low_risk_tokens: token_counts.1,
        medium_risk_tokens: token_counts.2,
        high_risk_tokens: token_counts.3,
        total_whale_alerts_24h: whale_count.0,
        latest_alpha_window_end: latest_alpha.0,
    }))
}
