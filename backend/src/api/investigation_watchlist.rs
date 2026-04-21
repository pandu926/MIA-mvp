use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{error::AppError, AppState};

#[derive(Debug, Clone, Serialize)]
pub struct InvestigationWatchlistItem {
    pub item_id: Uuid,
    pub entity_kind: String,
    pub entity_key: String,
    pub label: String,
    pub source_run_id: Option<Uuid>,
    pub linked_runs_count: i64,
    pub latest_run_id: Option<Uuid>,
    pub latest_run_status: Option<String>,
    pub latest_run_token_address: Option<String>,
    pub latest_run_updated_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct InvestigationWatchlistListResponse {
    pub data: Vec<InvestigationWatchlistItem>,
    pub total: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateInvestigationWatchlistItemRequest {
    pub entity_kind: String,
    pub entity_key: String,
    pub label: Option<String>,
    pub source_run_id: Option<Uuid>,
}

#[derive(Debug, FromRow)]
struct InvestigationWatchlistRow {
    id: Uuid,
    entity_kind: String,
    entity_key: String,
    label: String,
    source_run_id: Option<Uuid>,
    linked_runs_count: i64,
    latest_run_id: Option<Uuid>,
    latest_run_status: Option<String>,
    latest_run_token_address: Option<String>,
    latest_run_updated_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

fn validate_entity_kind(value: &str) -> Result<&'static str, AppError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "token" => Ok("token"),
        "builder" => Ok("builder"),
        _ => Err(AppError::BadRequest(
            "entity_kind must be one of: token, builder".to_string(),
        )),
    }
}

fn map_row(row: InvestigationWatchlistRow) -> InvestigationWatchlistItem {
    InvestigationWatchlistItem {
        item_id: row.id,
        entity_kind: row.entity_kind,
        entity_key: row.entity_key,
        label: row.label,
        source_run_id: row.source_run_id,
        linked_runs_count: row.linked_runs_count,
        latest_run_id: row.latest_run_id,
        latest_run_status: row.latest_run_status,
        latest_run_token_address: row.latest_run_token_address,
        latest_run_updated_at: row.latest_run_updated_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

pub async fn list_investigation_watchlist(
    State(state): State<AppState>,
) -> Result<Json<InvestigationWatchlistListResponse>, AppError> {
    let rows = sqlx::query_as::<_, InvestigationWatchlistRow>(
        r#"
        SELECT
            w.id,
            w.entity_kind,
            w.entity_key,
            w.label,
            w.source_run_id,
            CASE
                WHEN w.entity_kind = 'token' THEN COALESCE((
                    SELECT COUNT(*)::bigint
                    FROM investigation_runs ir
                    WHERE LOWER(ir.token_address) = LOWER(w.entity_key)
                ), 0)
                WHEN w.entity_kind = 'builder' THEN COALESCE((
                    SELECT COUNT(*)::bigint
                    FROM investigation_runs ir
                    JOIN tokens t ON LOWER(t.contract_address) = LOWER(ir.token_address)
                    WHERE LOWER(t.deployer_address) = LOWER(w.entity_key)
                ), 0)
                ELSE 0::bigint
            END AS linked_runs_count,
            CASE
                WHEN w.entity_kind = 'token' THEN (
                    SELECT ir.id
                    FROM investigation_runs ir
                    WHERE LOWER(ir.token_address) = LOWER(w.entity_key)
                    ORDER BY ir.updated_at DESC, ir.created_at DESC
                    LIMIT 1
                )
                WHEN w.entity_kind = 'builder' THEN (
                    SELECT ir.id
                    FROM investigation_runs ir
                    JOIN tokens t ON LOWER(t.contract_address) = LOWER(ir.token_address)
                    WHERE LOWER(t.deployer_address) = LOWER(w.entity_key)
                    ORDER BY ir.updated_at DESC, ir.created_at DESC
                    LIMIT 1
                )
                ELSE NULL
            END AS latest_run_id,
            CASE
                WHEN w.entity_kind = 'token' THEN (
                    SELECT ir.status
                    FROM investigation_runs ir
                    WHERE LOWER(ir.token_address) = LOWER(w.entity_key)
                    ORDER BY ir.updated_at DESC, ir.created_at DESC
                    LIMIT 1
                )
                WHEN w.entity_kind = 'builder' THEN (
                    SELECT ir.status
                    FROM investigation_runs ir
                    JOIN tokens t ON LOWER(t.contract_address) = LOWER(ir.token_address)
                    WHERE LOWER(t.deployer_address) = LOWER(w.entity_key)
                    ORDER BY ir.updated_at DESC, ir.created_at DESC
                    LIMIT 1
                )
                ELSE NULL
            END AS latest_run_status,
            CASE
                WHEN w.entity_kind = 'token' THEN (
                    SELECT ir.token_address
                    FROM investigation_runs ir
                    WHERE LOWER(ir.token_address) = LOWER(w.entity_key)
                    ORDER BY ir.updated_at DESC, ir.created_at DESC
                    LIMIT 1
                )
                WHEN w.entity_kind = 'builder' THEN (
                    SELECT ir.token_address
                    FROM investigation_runs ir
                    JOIN tokens t ON LOWER(t.contract_address) = LOWER(ir.token_address)
                    WHERE LOWER(t.deployer_address) = LOWER(w.entity_key)
                    ORDER BY ir.updated_at DESC, ir.created_at DESC
                    LIMIT 1
                )
                ELSE NULL
            END AS latest_run_token_address,
            CASE
                WHEN w.entity_kind = 'token' THEN (
                    SELECT ir.updated_at
                    FROM investigation_runs ir
                    WHERE LOWER(ir.token_address) = LOWER(w.entity_key)
                    ORDER BY ir.updated_at DESC, ir.created_at DESC
                    LIMIT 1
                )
                WHEN w.entity_kind = 'builder' THEN (
                    SELECT ir.updated_at
                    FROM investigation_runs ir
                    JOIN tokens t ON LOWER(t.contract_address) = LOWER(ir.token_address)
                    WHERE LOWER(t.deployer_address) = LOWER(w.entity_key)
                    ORDER BY ir.updated_at DESC, ir.created_at DESC
                    LIMIT 1
                )
                ELSE NULL
            END AS latest_run_updated_at,
            w.created_at,
            w.updated_at
        FROM investigation_watchlist_items w
        ORDER BY w.updated_at DESC, w.created_at DESC
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(InvestigationWatchlistListResponse {
        total: rows.len() as i64,
        data: rows.into_iter().map(map_row).collect(),
    }))
}

pub async fn create_investigation_watchlist_item(
    State(state): State<AppState>,
    Json(payload): Json<CreateInvestigationWatchlistItemRequest>,
) -> Result<Json<InvestigationWatchlistItem>, AppError> {
    let entity_kind = validate_entity_kind(&payload.entity_kind)?;
    let entity_key = payload.entity_key.trim().to_string();
    if entity_key.is_empty() {
        return Err(AppError::BadRequest("entity_key is required.".to_string()));
    }

    let label = payload
        .label
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| entity_key.clone());

    let row = sqlx::query_as::<_, InvestigationWatchlistRow>(
        r#"
        INSERT INTO investigation_watchlist_items (
            id,
            entity_kind,
            entity_key,
            label,
            source_run_id
        )
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (entity_kind, entity_key) DO UPDATE
        SET
            label = EXCLUDED.label,
            source_run_id = COALESCE(EXCLUDED.source_run_id, investigation_watchlist_items.source_run_id),
            updated_at = NOW()
        RETURNING
            id,
            entity_kind,
            entity_key,
            label,
            source_run_id,
            0::bigint AS linked_runs_count,
            NULL::uuid AS latest_run_id,
            NULL::varchar AS latest_run_status,
            NULL::varchar AS latest_run_token_address,
            NULL::timestamptz AS latest_run_updated_at,
            created_at,
            updated_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(entity_kind)
    .bind(&entity_key)
    .bind(&label)
    .bind(payload.source_run_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(map_row(row)))
}

pub async fn delete_investigation_watchlist_item(
    State(state): State<AppState>,
    Path(item_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let deleted = sqlx::query(
        r#"
        DELETE FROM investigation_watchlist_items
        WHERE id = $1
        "#,
    )
    .bind(item_id)
    .execute(&state.db)
    .await?;

    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound("Watchlist item not found.".to_string()));
    }

    Ok(Json(serde_json::json!({
        "deleted": true,
        "item_id": item_id,
    })))
}

#[cfg(test)]
mod tests {
    use super::validate_entity_kind;

    #[test]
    fn validate_entity_kind_accepts_supported_values() {
        assert_eq!(validate_entity_kind("token").unwrap(), "token");
        assert_eq!(validate_entity_kind("builder").unwrap(), "builder");
    }

    #[test]
    fn validate_entity_kind_rejects_unknown_values() {
        assert!(validate_entity_kind("cluster").is_err());
    }
}
