use super::types::{
    round2, LimitQuery, WhaleAlertResponse, WhaleNetworkEdge, WhaleNetworkMetrics,
    WhaleNetworkNode, WhaleNetworkQuery, WhaleNetworkResponse, WhaleStreamQuery,
    WhaleStreamResponse,
};
use crate::{error::AppError, AppState};
use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

/// GET /api/v1/whales?limit=20
pub async fn list_whale_alerts(
    State(state): State<AppState>,
    Query(params): Query<LimitQuery>,
) -> Result<Json<Vec<WhaleAlertResponse>>, AppError> {
    let limit = params.limit.clamp(1, 200);
    let rows: Vec<(String, String, String, f64, f64, String, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT token_address, wallet_address, tx_hash, amount_bnb::double precision,
               threshold_bnb::double precision, alert_level, created_at
        FROM whale_alerts
        ORDER BY created_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(
                |(
                    token_address,
                    wallet_address,
                    tx_hash,
                    amount_bnb,
                    threshold_bnb,
                    alert_level,
                    created_at,
                )| WhaleAlertResponse {
                    token_address,
                    wallet_address,
                    tx_hash,
                    amount_bnb,
                    threshold_bnb,
                    alert_level,
                    created_at,
                },
            )
            .collect(),
    ))
}

/// GET /api/v1/whales/stream?limit=20&offset=0&min_amount=0.5&level=watch&token=0x...
pub async fn whale_stream(
    State(state): State<AppState>,
    Query(params): Query<WhaleStreamQuery>,
) -> Result<Json<WhaleStreamResponse>, AppError> {
    let limit = params.limit.clamp(1, 200);
    let offset = params.offset.max(0);
    let min_amount = params.min_amount.max(0.0);

    let level = match params.level.as_deref() {
        Some("watch") | Some("critical") => params.level,
        _ => None,
    };

    let token = params
        .token
        .as_ref()
        .map(|token| token.trim().to_lowercase())
        .filter(|token| !token.is_empty());

    let rows: Vec<(String, String, String, f64, f64, String, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT token_address, wallet_address, tx_hash, amount_bnb::double precision,
               threshold_bnb::double precision, alert_level, created_at
        FROM whale_alerts
        WHERE amount_bnb >= $1
          AND ($2::text IS NULL OR alert_level = $2)
          AND ($3::text IS NULL OR LOWER(token_address) = $3)
        ORDER BY created_at DESC
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(min_amount)
    .bind(level.as_deref())
    .bind(token.as_deref())
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)::bigint
        FROM whale_alerts
        WHERE amount_bnb >= $1
          AND ($2::text IS NULL OR alert_level = $2)
          AND ($3::text IS NULL OR LOWER(token_address) = $3)
        "#,
    )
    .bind(min_amount)
    .bind(level.as_deref())
    .bind(token.as_deref())
    .fetch_one(&state.db)
    .await?;

    let data = rows
        .into_iter()
        .map(
            |(
                token_address,
                wallet_address,
                tx_hash,
                amount_bnb,
                threshold_bnb,
                alert_level,
                created_at,
            )| WhaleAlertResponse {
                token_address,
                wallet_address,
                tx_hash,
                amount_bnb,
                threshold_bnb,
                alert_level,
                created_at,
            },
        )
        .collect();

    Ok(Json(WhaleStreamResponse {
        data,
        total: total.0,
        limit,
        offset,
    }))
}

/// GET /api/v1/whales/network?hours=24&min_amount=0.5&level=critical
pub async fn whale_network(
    State(state): State<AppState>,
    Query(params): Query<WhaleNetworkQuery>,
) -> Result<Json<WhaleNetworkResponse>, AppError> {
    let hours = params.hours.clamp(1, 168);
    let since = Utc::now() - Duration::hours(hours);
    let min_amount = params.min_amount.max(0.0);
    let level = match params.level.as_deref() {
        Some("watch") | Some("critical") => params.level,
        _ => None,
    };

    let rows: Vec<(String, String, i64, f64, i64, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT
            wallet_address,
            token_address,
            COUNT(*)::bigint AS tx_count,
            COALESCE(SUM(amount_bnb), 0)::double precision AS total_volume_bnb,
            COUNT(*) FILTER (WHERE alert_level = 'critical')::bigint AS critical_count,
            MAX(created_at) AS last_tx_at
        FROM whale_alerts
        WHERE created_at >= $1
          AND amount_bnb >= $2
          AND ($3::text IS NULL OR alert_level = $3)
        GROUP BY wallet_address, token_address
        ORDER BY total_volume_bnb DESC, tx_count DESC
        LIMIT 300
        "#,
    )
    .bind(since)
    .bind(min_amount)
    .bind(level.as_deref())
    .fetch_all(&state.db)
    .await?;

    let mut node_map: HashMap<String, WhaleNetworkNode> = HashMap::new();
    let mut edges: Vec<WhaleNetworkEdge> = Vec::with_capacity(rows.len());
    let mut latest_updated_at: Option<DateTime<Utc>> = None;
    let mut total_volume = 0.0;
    let mut critical_edges = 0usize;

    for (wallet, token, tx_count, volume_bnb, critical_count, last_tx_at) in rows {
        let wallet_id = format!("wallet:{wallet}");
        let token_id = format!("token:{token}");

        let wallet_entry = node_map
            .entry(wallet_id.clone())
            .or_insert(WhaleNetworkNode {
                id: wallet_id.clone(),
                label: wallet.clone(),
                node_type: "wallet".to_string(),
                wallet_address: Some(wallet.clone()),
                token_address: None,
                total_volume_bnb: 0.0,
                tx_count: 0,
                critical_count: 0,
                last_seen_at: last_tx_at,
            });
        wallet_entry.total_volume_bnb += volume_bnb;
        wallet_entry.tx_count += tx_count;
        wallet_entry.critical_count += critical_count;
        if last_tx_at > wallet_entry.last_seen_at {
            wallet_entry.last_seen_at = last_tx_at;
        }

        let token_entry = node_map
            .entry(token_id.clone())
            .or_insert(WhaleNetworkNode {
                id: token_id.clone(),
                label: token.clone(),
                node_type: "token".to_string(),
                wallet_address: None,
                token_address: Some(token.clone()),
                total_volume_bnb: 0.0,
                tx_count: 0,
                critical_count: 0,
                last_seen_at: last_tx_at,
            });
        token_entry.total_volume_bnb += volume_bnb;
        token_entry.tx_count += tx_count;
        token_entry.critical_count += critical_count;
        if last_tx_at > token_entry.last_seen_at {
            token_entry.last_seen_at = last_tx_at;
        }

        edges.push(WhaleNetworkEdge {
            source: wallet_id,
            target: token_id,
            tx_count,
            total_volume_bnb: volume_bnb,
            last_tx_at,
        });

        total_volume += volume_bnb;
        if critical_count > 0 {
            critical_edges += 1;
        }
        latest_updated_at =
            Some(latest_updated_at.map_or(last_tx_at, |value| value.max(last_tx_at)));
    }

    let mut nodes: Vec<WhaleNetworkNode> = node_map.into_values().collect();
    nodes.sort_by(|left, right| right.total_volume_bnb.total_cmp(&left.total_volume_bnb));
    edges.sort_by(|left, right| right.total_volume_bnb.total_cmp(&left.total_volume_bnb));

    Ok(Json(WhaleNetworkResponse {
        metrics: WhaleNetworkMetrics {
            total_nodes: nodes.len(),
            total_edges: edges.len(),
            total_volume_bnb: round2(total_volume),
            critical_edges,
        },
        nodes,
        edges,
        latest_updated_at,
    }))
}
