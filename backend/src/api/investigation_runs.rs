use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{error::AppError, AppState};

use super::investigation::InvestigationResponse;

#[derive(Debug, Clone, Serialize)]
pub struct InvestigationRunSummary {
    pub run_id: Uuid,
    pub token_address: String,
    pub trigger_type: String,
    pub status: String,
    pub current_stage: String,
    pub source_surface: String,
    pub current_read: Option<String>,
    pub confidence_label: Option<String>,
    pub investigation_score: Option<i32>,
    pub summary: Option<String>,
    pub signal_tag: Option<String>,
    pub status_reason: Option<String>,
    pub evidence_delta: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct InvestigationRunListResponse {
    pub data: Vec<InvestigationRunSummary>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct InvestigationRunTimelineEvent {
    pub key: String,
    pub label: String,
    pub detail: String,
    pub signal_tag: Option<String>,
    pub reason: Option<String>,
    pub evidence_delta: Option<String>,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InvestigationRunDetailResponse {
    pub run: InvestigationRunSummary,
    pub timeline: Vec<InvestigationRunTimelineEvent>,
    pub continuity_note: String,
}

#[derive(Debug, Deserialize)]
pub struct InvestigationRunListQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub status: Option<String>,
    pub trigger: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InvestigationRunStatusUpdateRequest {
    pub status: String,
    pub reason: Option<String>,
    pub evidence_delta: Option<String>,
}

#[derive(Debug, FromRow)]
struct InvestigationRunRow {
    id: Uuid,
    token_address: String,
    trigger_type: String,
    status: String,
    current_stage: String,
    source_surface: String,
    current_read: Option<String>,
    confidence_label: Option<String>,
    investigation_score: Option<i32>,
    summary: Option<String>,
    status_reason: Option<String>,
    evidence_delta: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
struct InvestigationRunEventRow {
    id: i64,
    event_key: String,
    label: String,
    detail: String,
    reason: Option<String>,
    evidence_delta: Option<String>,
    created_at: DateTime<Utc>,
}

fn default_limit() -> i64 {
    20
}

fn infer_signal_tag_from_event_key(value: &str) -> Option<&'static str> {
    if value.contains("multi_signal") {
        return Some("multi_signal");
    }
    if value.contains("source_degradation") {
        return Some("source_degradation");
    }
    if value.contains("builder_overlap") {
        return Some("builder_overlap");
    }
    if value.contains("linked_launch_overlap") {
        return Some("linked_launch_overlap");
    }
    if value.contains("whale_alert") {
        return Some("whale_alert");
    }
    if value.contains("wallet_concentration") {
        return Some("wallet_concentration");
    }
    if value.contains("activity") {
        return Some("activity");
    }
    None
}

fn infer_signal_tag_from_text(value: Option<&str>) -> Option<&'static str> {
    let text = value.unwrap_or_default().to_ascii_lowercase();
    if text.is_empty() {
        return None;
    }

    if text.contains("linked launch overlap cooled") {
        return Some("linked_launch_overlap");
    }
    if text.contains("source health degraded") || text.contains("source degradation") {
        return Some("source_degradation");
    }
    if text.contains("builder overlap cooled") {
        return Some("builder_overlap");
    }
    if text.contains("critical whale alert activity cooled") {
        return Some("whale_alert");
    }
    if text.contains("wallet concentration cooled") {
        return Some("wallet_concentration");
    }
    if text.contains("live activity cooled below the promotion threshold;") {
        return Some("activity");
    }

    let has_activity = text.contains("transactions");
    let has_builder_overlap = text.contains("builder overlap");
    let has_linked_launch_overlap = text.contains("linked launch overlap");
    let has_concentration = text.contains("wallet concentration");
    let has_whale = text.contains("critical whale alert");
    let active_count = [
        has_activity,
        has_builder_overlap,
        has_linked_launch_overlap,
        has_concentration,
        has_whale,
    ]
    .into_iter()
    .filter(|active| *active)
    .count();

    if active_count > 1 {
        return Some("multi_signal");
    }
    if has_whale {
        return Some("whale_alert");
    }
    if has_concentration {
        return Some("wallet_concentration");
    }
    if has_builder_overlap {
        return Some("builder_overlap");
    }
    if has_linked_launch_overlap {
        return Some("linked_launch_overlap");
    }
    if has_activity {
        return Some("activity");
    }
    None
}

fn infer_signal_tag_from_summary(
    status: &str,
    current_stage: &str,
    status_reason: Option<&str>,
    evidence_delta: Option<&str>,
) -> Option<&'static str> {
    let is_auto_escalation = status == "escalated" && current_stage == "auto_escalation";
    let is_auto_monitoring = status == "watching" && current_stage == "auto_monitoring";

    if !is_auto_escalation && !is_auto_monitoring {
        return None;
    }

    infer_signal_tag_from_text(status_reason).or_else(|| infer_signal_tag_from_text(evidence_delta))
}

fn map_row(row: InvestigationRunRow) -> InvestigationRunSummary {
    let signal_tag = infer_signal_tag_from_summary(
        &row.status,
        &row.current_stage,
        row.status_reason.as_deref(),
        row.evidence_delta.as_deref(),
    )
    .map(ToString::to_string);

    InvestigationRunSummary {
        run_id: row.id,
        token_address: row.token_address,
        trigger_type: row.trigger_type,
        status: row.status,
        current_stage: row.current_stage,
        source_surface: row.source_surface,
        current_read: row.current_read,
        confidence_label: row.confidence_label,
        investigation_score: row.investigation_score,
        summary: row.summary,
        signal_tag,
        status_reason: row.status_reason,
        evidence_delta: row.evidence_delta,
        created_at: row.created_at,
        updated_at: row.updated_at,
        started_at: row.started_at,
        completed_at: row.completed_at,
    }
}

fn map_event_row(row: InvestigationRunEventRow) -> InvestigationRunTimelineEvent {
    InvestigationRunTimelineEvent {
        key: format!("{}-{}", row.event_key, row.id),
        label: row.label,
        detail: row.detail,
        signal_tag: infer_signal_tag_from_event_key(&row.event_key).map(ToString::to_string),
        reason: row.reason,
        evidence_delta: row.evidence_delta,
        at: row.created_at,
    }
}

fn build_status_change_detail(
    status: &str,
    reason: Option<&str>,
    evidence_delta: Option<&str>,
) -> String {
    let mut parts = Vec::new();

    if let Some(reason) = reason.filter(|value| !value.trim().is_empty()) {
        parts.push(reason.trim().to_string());
    } else {
        parts.push(format!("Run moved into {}.", status));
    }

    if let Some(delta) = evidence_delta.filter(|value| !value.trim().is_empty()) {
        parts.push(format!("Evidence delta: {}", delta.trim()));
    }

    parts.join(" ")
}

fn build_timeline(summary: &InvestigationRunSummary) -> Vec<InvestigationRunTimelineEvent> {
    let mut events = Vec::new();

    events.push(InvestigationRunTimelineEvent {
        key: "created".to_string(),
        label: "Run created".to_string(),
        detail: format!(
            "Run entered the system from {} as a {} investigation for {}.",
            summary.source_surface, summary.trigger_type, summary.token_address
        ),
        signal_tag: None,
        reason: None,
        evidence_delta: None,
        at: summary.created_at,
    });

    if let Some(started_at) = summary.started_at {
        events.push(InvestigationRunTimelineEvent {
            key: "started".to_string(),
            label: "Investigation started".to_string(),
            detail: format!(
                "Run moved into the {} stage and began collecting the current token read.",
                summary.current_stage
            ),
            signal_tag: None,
            reason: None,
            evidence_delta: None,
            at: started_at,
        });
    }

    if let Some(completed_at) = summary.completed_at {
        events.push(InvestigationRunTimelineEvent {
            key: "completed".to_string(),
            label: "Run completed".to_string(),
            detail: format!(
                "Run completed with status {} and current read {}.",
                summary.status,
                summary
                    .current_read
                    .clone()
                    .unwrap_or_else(|| "n/a".to_string())
            ),
            signal_tag: None,
            reason: None,
            evidence_delta: None,
            at: completed_at,
        });
    }

    if summary.updated_at > summary.created_at {
        events.push(InvestigationRunTimelineEvent {
            key: "updated".to_string(),
            label: "Latest update".to_string(),
            detail: format!(
                "Run was last refreshed with confidence {} and score {}.",
                summary
                    .confidence_label
                    .clone()
                    .unwrap_or_else(|| "n/a".to_string()),
                summary
                    .investigation_score
                    .map(|value| format!("{value}/100"))
                    .unwrap_or_else(|| "n/a".to_string())
            ),
            signal_tag: None,
            reason: None,
            evidence_delta: None,
            at: summary.updated_at,
        });
    }

    if summary.status_reason.is_some() || summary.evidence_delta.is_some() {
        events.push(InvestigationRunTimelineEvent {
            key: "status_change".to_string(),
            label: "Run status change".to_string(),
            detail: build_status_change_detail(
                &summary.status,
                summary.status_reason.as_deref(),
                summary.evidence_delta.as_deref(),
            ),
            signal_tag: summary.signal_tag.clone(),
            reason: summary.status_reason.clone(),
            evidence_delta: summary.evidence_delta.clone(),
            at: summary.updated_at,
        });
    }

    events.sort_by_key(|event| event.at);
    events
}

pub(crate) async fn append_run_event(
    db: &sqlx::PgPool,
    run_id: Uuid,
    event_key: &str,
    label: &str,
    detail: &str,
    reason: Option<&str>,
    evidence_delta: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO investigation_run_events (
            run_id,
            event_key,
            label,
            detail,
            reason,
            evidence_delta
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(run_id)
    .bind(event_key)
    .bind(label)
    .bind(detail)
    .bind(reason)
    .bind(evidence_delta)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn persist_manual_investigation_run(
    db: &sqlx::PgPool,
    report: &InvestigationResponse,
) -> Result<InvestigationRunSummary, AppError> {
    let existing = sqlx::query_as::<_, InvestigationRunRow>(
        r#"
        SELECT
            id,
            token_address,
            trigger_type,
            status,
            current_stage,
            source_surface,
            current_read,
            confidence_label,
            investigation_score,
            summary,
            status_reason,
            evidence_delta,
            created_at,
            updated_at,
            started_at,
            completed_at
        FROM investigation_runs
        WHERE token_address = $1
          AND trigger_type = 'manual'
          AND source_surface = 'mia'
          AND created_at >= NOW() - INTERVAL '15 minutes'
        ORDER BY updated_at DESC, created_at DESC
        LIMIT 1
        "#,
    )
    .bind(&report.token_address)
    .fetch_optional(db)
    .await?;

    let agent_scorecard = report.internal.agent_scorecard.as_ref();
    let read_label = agent_scorecard
        .map(|scorecard| scorecard.label.clone())
        .or_else(|| report.analysis.label.clone())
        .or_else(|| Some(report.analysis.verdict.clone()));
    let confidence_label = Some(
        agent_scorecard
            .map(|scorecard| scorecard.confidence_label.clone())
            .unwrap_or_else(|| report.analysis.confidence.clone()),
    );
    let score = agent_scorecard
        .map(|scorecard| i32::from(scorecard.score))
        .or_else(|| report.analysis.score.map(i32::from));
    let summary = Some(
        agent_scorecard
            .map(|scorecard| scorecard.summary.clone())
            .unwrap_or_else(|| report.analysis.executive_summary.clone()),
    );

    let (row, created_new_run) = if let Some(run) = existing {
        (
            sqlx::query_as::<_, InvestigationRunRow>(
                r#"
            UPDATE investigation_runs
            SET
                status = 'completed',
                current_stage = 'investigation',
                current_read = $2,
                confidence_label = $3,
                investigation_score = $4,
                summary = $5,
                updated_at = NOW(),
                started_at = COALESCE(started_at, NOW()),
                completed_at = NOW()
            WHERE id = $1
            RETURNING
                id,
                token_address,
                trigger_type,
                status,
                current_stage,
                source_surface,
                current_read,
                confidence_label,
                investigation_score,
                summary,
                status_reason,
                evidence_delta,
                created_at,
                updated_at,
                started_at,
                completed_at
            "#,
            )
            .bind(run.id)
            .bind(read_label)
            .bind(confidence_label)
            .bind(score)
            .bind(summary)
            .fetch_one(db)
            .await?,
            false,
        )
    } else {
        let run_id = Uuid::new_v4();
        (
            sqlx::query_as::<_, InvestigationRunRow>(
            r#"
            INSERT INTO investigation_runs (
                id,
                token_address,
                trigger_type,
                status,
                current_stage,
                source_surface,
                current_read,
                confidence_label,
                investigation_score,
                summary,
                started_at,
                completed_at
            )
            VALUES ($1, $2, 'manual', 'completed', 'investigation', 'mia', $3, $4, $5, $6, NOW(), NOW())
            RETURNING
                id,
                token_address,
                trigger_type,
                status,
                current_stage,
                source_surface,
                current_read,
                confidence_label,
                investigation_score,
                summary,
                status_reason,
                evidence_delta,
                created_at,
                updated_at,
                started_at,
                completed_at
            "#,
        )
            .bind(run_id)
            .bind(&report.token_address)
            .bind(read_label)
            .bind(confidence_label)
            .bind(score)
            .bind(summary)
            .fetch_one(db)
            .await?,
            true,
        )
    };

    let summary = map_row(row);
    let completion_detail = format!(
        "Manual investigation completed with current read {} at {} confidence and score {}.",
        summary
            .current_read
            .clone()
            .unwrap_or_else(|| "n/a".to_string()),
        summary
            .confidence_label
            .clone()
            .unwrap_or_else(|| "n/a".to_string()),
        summary
            .investigation_score
            .map(|value| format!("{value}/100"))
            .unwrap_or_else(|| "n/a".to_string())
    );

    if created_new_run {
        append_run_event(
            db,
            summary.run_id,
            "run_created",
            "Run created",
            &format!(
                "Run entered the system from {} as a {} investigation for {}.",
                summary.source_surface, summary.trigger_type, summary.token_address
            ),
            None,
            None,
        )
        .await?;
    }

    append_run_event(
        db,
        summary.run_id,
        "manual_investigation_completed",
        "Manual investigation completed",
        &completion_detail,
        summary.summary.as_deref(),
        Some(&completion_detail),
    )
    .await?;

    Ok(summary)
}

pub async fn list_investigation_runs(
    State(state): State<AppState>,
    Query(params): Query<InvestigationRunListQuery>,
) -> Result<Json<InvestigationRunListResponse>, AppError> {
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);

    let rows = sqlx::query_as::<_, InvestigationRunRow>(
        r#"
        SELECT
            id,
            token_address,
            trigger_type,
            status,
            current_stage,
            source_surface,
            current_read,
            confidence_label,
            investigation_score,
            summary,
            status_reason,
            evidence_delta,
            created_at,
            updated_at,
            started_at,
            completed_at
        FROM investigation_runs
        WHERE ($1::text IS NULL OR status = $1)
          AND ($2::text IS NULL OR trigger_type = $2)
          AND ($3::text IS NULL OR token_address = $3)
        ORDER BY updated_at DESC, created_at DESC
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(params.status.as_deref())
    .bind(params.trigger.as_deref())
    .bind(params.token.as_deref())
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)::bigint
        FROM investigation_runs
        WHERE ($1::text IS NULL OR status = $1)
          AND ($2::text IS NULL OR trigger_type = $2)
          AND ($3::text IS NULL OR token_address = $3)
        "#,
    )
    .bind(params.status.as_deref())
    .bind(params.trigger.as_deref())
    .bind(params.token.as_deref())
    .fetch_one(&state.db)
    .await?;

    Ok(Json(InvestigationRunListResponse {
        data: rows.into_iter().map(map_row).collect(),
        total: total.0,
        limit,
        offset,
    }))
}

pub async fn get_investigation_run(
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
) -> Result<Json<InvestigationRunSummary>, AppError> {
    let row = sqlx::query_as::<_, InvestigationRunRow>(
        r#"
        SELECT
            id,
            token_address,
            trigger_type,
            status,
            current_stage,
            source_surface,
            current_read,
            confidence_label,
            investigation_score,
            summary,
            status_reason,
            evidence_delta,
            created_at,
            updated_at,
            started_at,
            completed_at
        FROM investigation_runs
        WHERE id = $1
        "#,
    )
    .bind(run_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Investigation run not found.".to_string()))?;

    Ok(Json(map_row(row)))
}

pub async fn get_investigation_run_detail(
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
) -> Result<Json<InvestigationRunDetailResponse>, AppError> {
    let detail = load_investigation_run_detail(&state.db, run_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Investigation run not found.".to_string()))?;

    Ok(Json(detail))
}

pub(crate) async fn load_investigation_run_detail(
    db: &sqlx::PgPool,
    run_id: Uuid,
) -> Result<Option<InvestigationRunDetailResponse>, AppError> {
    let row = sqlx::query_as::<_, InvestigationRunRow>(
        r#"
        SELECT
            id,
            token_address,
            trigger_type,
            status,
            current_stage,
            source_surface,
            current_read,
            confidence_label,
            investigation_score,
            summary,
            status_reason,
            evidence_delta,
            created_at,
            updated_at,
            started_at,
            completed_at
        FROM investigation_runs
        WHERE id = $1
        "#,
    )
    .bind(run_id)
    .fetch_optional(db)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let run = map_row(row);
    let event_rows = sqlx::query_as::<_, InvestigationRunEventRow>(
        r#"
        SELECT
            id,
            event_key,
            label,
            detail,
            reason,
            evidence_delta,
            created_at
        FROM investigation_run_events
        WHERE run_id = $1
        ORDER BY created_at ASC, id ASC
        "#,
    )
    .bind(run_id)
    .fetch_all(db)
    .await?;

    let timeline = if event_rows.is_empty() {
        build_timeline(&run)
    } else {
        event_rows.into_iter().map(map_event_row).collect()
    };
    let continuity_note = format!(
        "This run keeps continuity for {} through {} recorded events, is currently in the {} stage, and is marked {}.",
        run.token_address,
        timeline.len(),
        run.current_stage,
        run.status
    );

    Ok(Some(InvestigationRunDetailResponse {
        run,
        timeline,
        continuity_note,
    }))
}

pub async fn update_investigation_run_status(
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
    Json(payload): Json<InvestigationRunStatusUpdateRequest>,
) -> Result<Json<InvestigationRunSummary>, AppError> {
    let normalized_status = payload.status.trim().to_ascii_lowercase();
    let reason = payload
        .reason
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let evidence_delta = payload
        .evidence_delta
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let next_stage = match normalized_status.as_str() {
        "watching" => "monitoring",
        "escalated" => "escalated",
        "archived" => "archived",
        _ => {
            return Err(AppError::BadRequest(
                "status must be one of: watching, escalated, archived".to_string(),
            ))
        }
    };

    let row = sqlx::query_as::<_, InvestigationRunRow>(
        r#"
        UPDATE investigation_runs
        SET
            status = $2,
            current_stage = $3,
            status_reason = $4,
            evidence_delta = $5,
            updated_at = NOW()
        WHERE id = $1
        RETURNING
            id,
            token_address,
            trigger_type,
            status,
            current_stage,
            source_surface,
            current_read,
            confidence_label,
            investigation_score,
            summary,
            status_reason,
            evidence_delta,
            created_at,
            updated_at,
            started_at,
            completed_at
        "#,
    )
    .bind(run_id)
    .bind(&normalized_status)
    .bind(next_stage)
    .bind(reason.as_deref())
    .bind(evidence_delta.as_deref())
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Investigation run not found.".to_string()))?;

    let summary = map_row(row);
    let detail = build_status_change_detail(
        &summary.status,
        reason.as_deref(),
        evidence_delta.as_deref(),
    );

    append_run_event(
        &state.db,
        summary.run_id,
        "status_transition",
        "Run status change",
        &detail,
        reason.as_deref(),
        evidence_delta.as_deref(),
    )
    .await?;

    Ok(Json(summary))
}

#[cfg(test)]
mod tests {
    use super::{
        build_status_change_detail, infer_signal_tag_from_event_key, infer_signal_tag_from_summary,
    };

    #[test]
    fn status_change_detail_keeps_reason_and_delta() {
        let detail = build_status_change_detail(
            "watching",
            Some("Monitoring reason: whale concentration increased."),
            Some("Latest evidence delta: top holder share rose by 8%."),
        );

        assert!(detail.contains("Monitoring reason: whale concentration increased."));
        assert!(
            detail.contains("Evidence delta: Latest evidence delta: top holder share rose by 8%.")
        );
    }

    #[test]
    fn status_change_detail_falls_back_to_status_when_reason_missing() {
        let detail = build_status_change_detail("archived", None, None);
        assert_eq!(detail, "Run moved into archived.");
    }

    #[test]
    fn event_key_maps_to_signal_tag() {
        assert_eq!(
            infer_signal_tag_from_event_key("auto_escalation_triggered_wallet_concentration"),
            Some("wallet_concentration")
        );
        assert_eq!(
            infer_signal_tag_from_event_key("auto_escalation_triggered_builder_overlap"),
            Some("builder_overlap")
        );
        assert_eq!(
            infer_signal_tag_from_event_key("auto_escalation_triggered_linked_launch_overlap"),
            Some("linked_launch_overlap")
        );
        assert_eq!(
            infer_signal_tag_from_event_key("auto_monitoring_downgraded_linked_launch_overlap"),
            Some("linked_launch_overlap")
        );
        assert_eq!(
            infer_signal_tag_from_event_key("auto_monitoring_downgraded_source_degradation"),
            Some("source_degradation")
        );
        assert_eq!(
            infer_signal_tag_from_event_key("auto_escalation_triggered_whale_alert"),
            Some("whale_alert")
        );
    }

    #[test]
    fn summary_signal_tag_prefers_structured_auto_escalation_context() {
        assert_eq!(
            infer_signal_tag_from_summary(
                "escalated",
                "auto_escalation",
                Some("Auto escalation reason: wallet concentration is elevated at 97 while the run was in watching."),
                None
            ),
            Some("wallet_concentration")
        );
        assert_eq!(
            infer_signal_tag_from_summary(
                "escalated",
                "auto_escalation",
                Some("Auto escalation reason: builder overlap is live with 1 seller wallet later appearing as new deployers while the run was in watching."),
                None
            ),
            Some("builder_overlap")
        );
        assert_eq!(
            infer_signal_tag_from_summary(
                "escalated",
                "auto_escalation",
                Some("Auto escalation reason: linked launch overlap is live across 2 related launches while the run was in watching."),
                None
            ),
            Some("linked_launch_overlap")
        );
        assert_eq!(
            infer_signal_tag_from_summary(
                "watching",
                "auto_monitoring",
                Some(
                    "Auto monitoring downgrade reason: linked launch overlap cooled to zero related launches while other live promotion signals stayed muted; live activity cooled below the escalation threshold with 24 transactions against a 100 transaction trigger, wallet concentration at 41, 0 critical whale alerts, 0 builder-overlap seller wallets, and 0 linked launch overlaps still active while the run returned to watching."
                ),
                None
            ),
            Some("linked_launch_overlap")
        );
        assert_eq!(
            infer_signal_tag_from_summary(
                "watching",
                "auto_monitoring",
                Some(
                    "Auto monitoring downgrade reason: source health degraded while other live promotion signals stayed muted; live activity cooled below the escalation threshold with 24 transactions against a 100 transaction trigger, wallet concentration at 41, 0 critical whale alerts, 0 builder-overlap seller wallets, and 0 linked launch overlaps still active while the run returned to watching."
                ),
                None
            ),
            Some("source_degradation")
        );
        assert_eq!(
            infer_signal_tag_from_summary(
                "completed",
                "investigation",
                Some("Auto escalation reason: the launch stayed above 100 transactions."),
                None
            ),
            None
        );
    }
}
