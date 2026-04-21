use crate::{error::AppError, AppState};
use axum::{extract::State, Json};
use chrono::{Duration, Utc};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub db: String,
    pub redis: String,
    pub indexer: IndexerStatus,
    pub alpha: AlphaStatus,
}

#[derive(Debug, Serialize)]
pub struct IndexerStatus {
    pub status: String,
    pub last_block: i64,
}

#[derive(Debug, Serialize)]
pub struct AlphaStatus {
    pub latest_window_end: Option<chrono::DateTime<Utc>>,
    pub rows_in_latest_window: i64,
    pub stale: bool,
}

pub async fn health_handler(
    State(state): State<AppState>,
) -> Result<Json<HealthResponse>, AppError> {
    // Check database connectivity
    let db_status = match sqlx::query("SELECT 1").fetch_one(&state.db).await {
        Ok(_) => "connected".to_string(),
        Err(e) => {
            tracing::warn!("Health check: database error: {}", e);
            format!("error: {}", e)
        }
    };

    // Check Redis connectivity
    let redis_status = {
        let mut conn = state.redis.clone();
        match redis::cmd("PING").query_async::<String>(&mut conn).await {
            Ok(_) => "connected".to_string(),
            Err(e) => {
                tracing::warn!("Health check: redis error: {}", e);
                format!("error: {}", e)
            }
        }
    };

    // Get indexer state
    let indexer = match sqlx::query_as::<_, (i64, String)>(
        "SELECT last_processed_block, indexer_status FROM indexer_state WHERE id = 1",
    )
    .fetch_one(&state.db)
    .await
    {
        Ok((last_block, status)) => IndexerStatus { status, last_block },
        Err(_) => IndexerStatus {
            status: "unknown".to_string(),
            last_block: 0,
        },
    };

    let latest_alpha_window: Option<(chrono::DateTime<Utc>,)> =
        sqlx::query_as("SELECT MAX(window_end) FROM alpha_rankings")
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten();

    let alpha = if let Some((window_end,)) = latest_alpha_window {
        let rows_in_latest_window: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM alpha_rankings WHERE window_end = $1")
                .bind(window_end)
                .fetch_one(&state.db)
                .await
                .unwrap_or(0);
        let stale = window_end < (Utc::now() - Duration::hours(2));
        AlphaStatus {
            latest_window_end: Some(window_end),
            rows_in_latest_window,
            stale,
        }
    } else {
        AlphaStatus {
            latest_window_end: None,
            rows_in_latest_window: 0,
            stale: true,
        }
    };

    let overall_status = if db_status == "connected" && redis_status == "connected" && !alpha.stale
    {
        "ok"
    } else {
        "degraded"
    };

    Ok(Json(HealthResponse {
        status: overall_status.to_string(),
        db: db_status,
        redis: redis_status,
        indexer,
        alpha,
    }))
}
