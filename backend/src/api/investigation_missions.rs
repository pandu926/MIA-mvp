use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{error::AppError, AppState};

#[derive(Debug, Serialize, Clone)]
pub struct InvestigationMission {
    pub mission_id: Uuid,
    pub mission_type: String,
    pub status: String,
    pub entity_kind: Option<String>,
    pub entity_key: Option<String>,
    pub label: String,
    pub note: Option<String>,
    pub source_watchlist_item_id: Option<Uuid>,
    pub source_run_id: Option<Uuid>,
    pub linked_runs_count: i64,
    pub latest_run_id: Option<Uuid>,
    pub latest_run_status: Option<String>,
    pub latest_run_updated_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct InvestigationMissionListResponse {
    pub data: Vec<InvestigationMission>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInvestigationMissionRequest {
    pub mission_type: String,
    pub entity_kind: Option<String>,
    pub entity_key: Option<String>,
    pub label: Option<String>,
    pub note: Option<String>,
    pub source_watchlist_item_id: Option<Uuid>,
    pub source_run_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateInvestigationMissionRequest {
    pub status: String,
}

#[derive(Debug, sqlx::FromRow)]
struct InvestigationMissionRow {
    mission_id: Uuid,
    mission_type: String,
    status: String,
    entity_kind: Option<String>,
    entity_key: Option<String>,
    label: String,
    note: Option<String>,
    source_watchlist_item_id: Option<Uuid>,
    source_run_id: Option<Uuid>,
    linked_runs_count: i64,
    latest_run_id: Option<Uuid>,
    latest_run_status: Option<String>,
    latest_run_updated_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

fn validate_mission_type(value: &str) -> Result<&str, AppError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "watch_hot_launches" => Ok("watch_hot_launches"),
        "watch_builder_cluster" => Ok("watch_builder_cluster"),
        "watch_suspicious_recurrence" => Ok("watch_suspicious_recurrence"),
        "watch_proof_qualified_launches" => Ok("watch_proof_qualified_launches"),
        other => Err(AppError::BadRequest(format!(
            "Unsupported mission type `{other}`. Use watch_hot_launches, watch_builder_cluster, watch_suspicious_recurrence, or watch_proof_qualified_launches."
        ))),
    }
}

fn validate_entity_kind(value: Option<&str>) -> Result<Option<&str>, AppError> {
    match value.map(|raw| raw.trim().to_ascii_lowercase()) {
        Some(kind) if kind == "token" => Ok(Some("token")),
        Some(kind) if kind == "builder" => Ok(Some("builder")),
        Some(kind) if kind.is_empty() => Ok(None),
        Some(other) => Err(AppError::BadRequest(format!(
            "Unsupported mission entity kind `{other}`. Use token or builder."
        ))),
        None => Ok(None),
    }
}

fn validate_status(value: &str) -> Result<&str, AppError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "active" => Ok("active"),
        "paused" => Ok("paused"),
        "archived" => Ok("archived"),
        other => Err(AppError::BadRequest(format!(
            "Unsupported mission status `{other}`. Use active, paused, or archived."
        ))),
    }
}

fn default_label(
    mission_type: &str,
    entity_kind: Option<&str>,
    entity_key: Option<&str>,
) -> String {
    let mission_label = match mission_type {
        "watch_builder_cluster" => "Watch builder cluster",
        "watch_suspicious_recurrence" => "Watch suspicious recurrence",
        "watch_proof_qualified_launches" => "Watch proof-qualified launches",
        _ => "Watch hot launches",
    };

    match (entity_kind, entity_key) {
        (Some("builder"), Some(key)) => format!("{mission_label}: {key}"),
        (Some("token"), Some(key)) => format!("{mission_label}: {key}"),
        _ => mission_label.to_string(),
    }
}

pub async fn list_investigation_missions(
    State(state): State<AppState>,
) -> Result<Json<InvestigationMissionListResponse>, AppError> {
    let rows = sqlx::query_as::<_, InvestigationMissionRow>(
        r#"
        SELECT
            m.id AS mission_id,
            m.mission_type,
            m.status,
            m.entity_kind,
            m.entity_key,
            m.label,
            m.note,
            m.source_watchlist_item_id,
            m.source_run_id,
            CASE
                WHEN m.entity_kind = 'token' THEN COALESCE((
                    SELECT COUNT(*)::bigint
                    FROM investigation_runs ir
                    WHERE LOWER(ir.token_address) = LOWER(m.entity_key)
                ), 0)
                WHEN m.entity_kind = 'builder' THEN COALESCE((
                    SELECT COUNT(*)::bigint
                    FROM investigation_runs ir
                    JOIN tokens t ON LOWER(t.contract_address) = LOWER(ir.token_address)
                    WHERE LOWER(t.deployer_address) = LOWER(m.entity_key)
                ), 0)
                ELSE 0::bigint
            END AS linked_runs_count,
            CASE
                WHEN m.entity_kind = 'token' THEN (
                    SELECT ir.id
                    FROM investigation_runs ir
                    WHERE LOWER(ir.token_address) = LOWER(m.entity_key)
                    ORDER BY ir.updated_at DESC, ir.created_at DESC
                    LIMIT 1
                )
                WHEN m.entity_kind = 'builder' THEN (
                    SELECT ir.id
                    FROM investigation_runs ir
                    JOIN tokens t ON LOWER(t.contract_address) = LOWER(ir.token_address)
                    WHERE LOWER(t.deployer_address) = LOWER(m.entity_key)
                    ORDER BY ir.updated_at DESC, ir.created_at DESC
                    LIMIT 1
                )
                ELSE NULL
            END AS latest_run_id,
            CASE
                WHEN m.entity_kind = 'token' THEN (
                    SELECT ir.status
                    FROM investigation_runs ir
                    WHERE LOWER(ir.token_address) = LOWER(m.entity_key)
                    ORDER BY ir.updated_at DESC, ir.created_at DESC
                    LIMIT 1
                )
                WHEN m.entity_kind = 'builder' THEN (
                    SELECT ir.status
                    FROM investigation_runs ir
                    JOIN tokens t ON LOWER(t.contract_address) = LOWER(ir.token_address)
                    WHERE LOWER(t.deployer_address) = LOWER(m.entity_key)
                    ORDER BY ir.updated_at DESC, ir.created_at DESC
                    LIMIT 1
                )
                ELSE NULL
            END AS latest_run_status,
            CASE
                WHEN m.entity_kind = 'token' THEN (
                    SELECT ir.updated_at
                    FROM investigation_runs ir
                    WHERE LOWER(ir.token_address) = LOWER(m.entity_key)
                    ORDER BY ir.updated_at DESC, ir.created_at DESC
                    LIMIT 1
                )
                WHEN m.entity_kind = 'builder' THEN (
                    SELECT ir.updated_at
                    FROM investigation_runs ir
                    JOIN tokens t ON LOWER(t.contract_address) = LOWER(ir.token_address)
                    WHERE LOWER(t.deployer_address) = LOWER(m.entity_key)
                    ORDER BY ir.updated_at DESC, ir.created_at DESC
                    LIMIT 1
                )
                ELSE NULL
            END AS latest_run_updated_at,
            m.created_at,
            m.updated_at
        FROM investigation_missions m
        ORDER BY
            CASE m.status WHEN 'active' THEN 0 WHEN 'paused' THEN 1 ELSE 2 END,
            m.updated_at DESC,
            m.created_at DESC
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(InvestigationMissionListResponse {
        data: rows
            .into_iter()
            .map(|row| InvestigationMission {
                mission_id: row.mission_id,
                mission_type: row.mission_type,
                status: row.status,
                entity_kind: row.entity_kind,
                entity_key: row.entity_key,
                label: row.label,
                note: row.note,
                source_watchlist_item_id: row.source_watchlist_item_id,
                source_run_id: row.source_run_id,
                linked_runs_count: row.linked_runs_count,
                latest_run_id: row.latest_run_id,
                latest_run_status: row.latest_run_status,
                latest_run_updated_at: row.latest_run_updated_at,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect(),
    }))
}

pub async fn create_investigation_mission(
    State(state): State<AppState>,
    Json(payload): Json<CreateInvestigationMissionRequest>,
) -> Result<Json<InvestigationMission>, AppError> {
    let mission_type = validate_mission_type(&payload.mission_type)?;
    let entity_kind = validate_entity_kind(payload.entity_kind.as_deref())?;
    let entity_key = payload
        .entity_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    if entity_kind.is_some() && entity_key.is_none() {
        return Err(AppError::BadRequest(
            "Mission entity_key is required when entity_kind is provided.".to_string(),
        ));
    }

    let label = payload
        .label
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| default_label(mission_type, entity_kind, entity_key.as_deref()));

    let row = sqlx::query_as::<_, InvestigationMissionRow>(
        r#"
        INSERT INTO investigation_missions (
            id,
            mission_type,
            status,
            entity_kind,
            entity_key,
            label,
            note,
            source_watchlist_item_id,
            source_run_id,
            created_at,
            updated_at
        )
        VALUES ($1, $2, 'active', $3, $4, $5, $6, $7, $8, NOW(), NOW())
        RETURNING
            id AS mission_id,
            mission_type,
            status,
            entity_kind,
            entity_key,
            label,
            note,
            source_watchlist_item_id,
            source_run_id,
            0::bigint AS linked_runs_count,
            NULL::uuid AS latest_run_id,
            NULL::varchar AS latest_run_status,
            NULL::timestamptz AS latest_run_updated_at,
            created_at,
            updated_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(mission_type)
    .bind(entity_kind)
    .bind(entity_key)
    .bind(label)
    .bind(
        payload
            .note
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty()),
    )
    .bind(payload.source_watchlist_item_id)
    .bind(payload.source_run_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(InvestigationMission {
        mission_id: row.mission_id,
        mission_type: row.mission_type,
        status: row.status,
        entity_kind: row.entity_kind,
        entity_key: row.entity_key,
        label: row.label,
        note: row.note,
        source_watchlist_item_id: row.source_watchlist_item_id,
        source_run_id: row.source_run_id,
        linked_runs_count: row.linked_runs_count,
        latest_run_id: row.latest_run_id,
        latest_run_status: row.latest_run_status,
        latest_run_updated_at: row.latest_run_updated_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }))
}

pub async fn update_investigation_mission(
    State(state): State<AppState>,
    Path(mission_id): Path<Uuid>,
    Json(payload): Json<UpdateInvestigationMissionRequest>,
) -> Result<Json<InvestigationMission>, AppError> {
    let status = validate_status(&payload.status)?;

    let row = sqlx::query_as::<_, InvestigationMissionRow>(
        r#"
        UPDATE investigation_missions
        SET status = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING
            id AS mission_id,
            mission_type,
            status,
            entity_kind,
            entity_key,
            label,
            note,
            source_watchlist_item_id,
            source_run_id,
            0::bigint AS linked_runs_count,
            NULL::uuid AS latest_run_id,
            NULL::varchar AS latest_run_status,
            NULL::timestamptz AS latest_run_updated_at,
            created_at,
            updated_at
        "#,
    )
    .bind(mission_id)
    .bind(status)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Mission not found".to_string()))?;

    Ok(Json(InvestigationMission {
        mission_id: row.mission_id,
        mission_type: row.mission_type,
        status: row.status,
        entity_kind: row.entity_kind,
        entity_key: row.entity_key,
        label: row.label,
        note: row.note,
        source_watchlist_item_id: row.source_watchlist_item_id,
        source_run_id: row.source_run_id,
        linked_runs_count: row.linked_runs_count,
        latest_run_id: row.latest_run_id,
        latest_run_status: row.latest_run_status,
        latest_run_updated_at: row.latest_run_updated_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }))
}

#[cfg(test)]
mod tests {
    use super::{validate_entity_kind, validate_mission_type, validate_status};

    #[test]
    fn mission_type_accepts_supported_values() {
        assert_eq!(
            validate_mission_type("watch_hot_launches").unwrap(),
            "watch_hot_launches"
        );
        assert_eq!(
            validate_mission_type("watch_builder_cluster").unwrap(),
            "watch_builder_cluster"
        );
        assert_eq!(
            validate_mission_type("watch_suspicious_recurrence").unwrap(),
            "watch_suspicious_recurrence"
        );
        assert_eq!(
            validate_mission_type("watch_proof_qualified_launches").unwrap(),
            "watch_proof_qualified_launches"
        );
    }

    #[test]
    fn mission_status_accepts_supported_values() {
        assert_eq!(validate_status("active").unwrap(), "active");
        assert_eq!(validate_status("paused").unwrap(), "paused");
        assert_eq!(validate_status("archived").unwrap(), "archived");
    }

    #[test]
    fn entity_kind_accepts_supported_values() {
        assert_eq!(validate_entity_kind(Some("token")).unwrap(), Some("token"));
        assert_eq!(
            validate_entity_kind(Some("builder")).unwrap(),
            Some("builder")
        );
        assert_eq!(validate_entity_kind(None).unwrap(), None);
    }
}
