use crate::{error::AppError, AppState};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct WalletIntelQuery {
    #[serde(default = "default_hours")]
    pub hours: i64,
}

fn default_hours() -> i64 {
    24
}

#[derive(Debug, Serialize)]
pub struct WalletTokenBreakdown {
    pub token_address: String,
    pub tx_count: i64,
    pub volume_bnb: f64,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct WalletIntelResponse {
    pub wallet_address: String,
    pub total_whale_txs: i64,
    pub total_volume_bnb: f64,
    pub watch_alerts: i64,
    pub critical_alerts: i64,
    pub last_seen_at: DateTime<Utc>,
    pub top_tokens: Vec<WalletTokenBreakdown>,
}

/// GET /api/v1/wallets/:address/intel?hours=24
pub async fn get_wallet_intel(
    State(state): State<AppState>,
    Path(address): Path<String>,
    Query(params): Query<WalletIntelQuery>,
) -> Result<Json<WalletIntelResponse>, AppError> {
    let hours = params.hours.clamp(1, 168);
    let since = Utc::now() - Duration::hours(hours);

    let summary: Option<(i64, f64, i64, i64, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT
            COUNT(*)::bigint AS total_whale_txs,
            COALESCE(SUM(amount_bnb), 0)::double precision AS total_volume_bnb,
            COUNT(*) FILTER (WHERE alert_level = 'watch')::bigint AS watch_alerts,
            COUNT(*) FILTER (WHERE alert_level = 'critical')::bigint AS critical_alerts,
            MAX(created_at) AS last_seen_at
        FROM whale_alerts
        WHERE LOWER(wallet_address) = LOWER($1)
          AND created_at >= $2
        "#,
    )
    .bind(&address)
    .bind(since)
    .fetch_optional(&state.db)
    .await?;

    let Some((total_whale_txs, total_volume_bnb, watch_alerts, critical_alerts, last_seen_at)) =
        summary
    else {
        return Err(AppError::NotFound(format!(
            "Wallet {} has no whale intel in last {}h",
            address, hours
        )));
    };

    if total_whale_txs == 0 {
        return Err(AppError::NotFound(format!(
            "Wallet {} has no whale intel in last {}h",
            address, hours
        )));
    }

    let rows: Vec<(String, i64, f64, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT
            token_address,
            COUNT(*)::bigint AS tx_count,
            COALESCE(SUM(amount_bnb), 0)::double precision AS volume_bnb,
            MAX(created_at) AS last_seen_at
        FROM whale_alerts
        WHERE LOWER(wallet_address) = LOWER($1)
          AND created_at >= $2
        GROUP BY token_address
        ORDER BY volume_bnb DESC, tx_count DESC
        LIMIT 8
        "#,
    )
    .bind(&address)
    .bind(since)
    .fetch_all(&state.db)
    .await?;

    let top_tokens = rows
        .into_iter()
        .map(
            |(token_address, tx_count, volume_bnb, last_seen_at)| WalletTokenBreakdown {
                token_address,
                tx_count,
                volume_bnb,
                last_seen_at,
            },
        )
        .collect();

    Ok(Json(WalletIntelResponse {
        wallet_address: address,
        total_whale_txs,
        total_volume_bnb,
        watch_alerts,
        critical_alerts,
        last_seen_at,
        top_tokens,
    }))
}
