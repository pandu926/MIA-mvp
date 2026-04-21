use axum::{extract::State, Json};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::investigation_runs::append_run_event, error::AppError,
    research::auto_investigation::AutoInvestigationSettings, AppState,
};

#[derive(Debug, Serialize)]
pub struct InvestigationOpsSummary {
    pub runs: InvestigationOpsRunCounts,
    pub triggers: InvestigationOpsTriggerCounts,
    pub loop_health: InvestigationOpsLoopHealth,
    pub watchlist_items: i64,
    pub missions: InvestigationOpsMissionCounts,
    pub auto_investigation: InvestigationOpsAutoConfig,
    pub degradation_notes: Vec<InvestigationOpsDegradationNote>,
}

#[derive(Debug, Serialize)]
pub struct InvestigationOpsRunCounts {
    pub queued: i64,
    pub running: i64,
    pub watching: i64,
    pub escalated: i64,
    pub completed: i64,
    pub failed: i64,
    pub archived: i64,
}

#[derive(Debug, Serialize)]
pub struct InvestigationOpsTriggerCounts {
    pub manual: i64,
    pub auto: i64,
}

#[derive(Debug, Serialize)]
pub struct InvestigationOpsLoopHealth {
    pub auto_runs_24h: i64,
    pub retry_actions_24h: i64,
    pub recovery_actions_24h: i64,
    pub failure_rate_24h_pct: f64,
    pub average_completion_minutes_24h: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct InvestigationOpsMissionCounts {
    pub active: i64,
    pub paused: i64,
    pub archived: i64,
}

#[derive(Debug, Serialize)]
pub struct InvestigationOpsAutoConfig {
    pub enabled: bool,
    pub paused: bool,
    pub tx_threshold: i64,
    pub cooldown_mins: i64,
}

#[derive(Debug, Serialize)]
pub struct InvestigationOpsDegradationNote {
    pub code: String,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateInvestigationOpsControlRequest {
    pub auto_investigation_paused: bool,
}

#[derive(Debug, Deserialize)]
pub struct ArchiveStaleRunsRequest {
    pub stale_after_minutes: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ArchiveStaleRunsResponse {
    pub archived_count: usize,
    pub archived_run_ids: Vec<Uuid>,
    pub ops_summary: InvestigationOpsSummary,
}

#[derive(Debug, Serialize)]
pub struct RetryFailedRunsResponse {
    pub retried_count: usize,
    pub retried_run_ids: Vec<Uuid>,
    pub ops_summary: InvestigationOpsSummary,
}

#[derive(Debug, Deserialize)]
pub struct RecoverStaleRunningRunsRequest {
    pub stale_after_minutes: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct RecoverStaleRunningRunsResponse {
    pub recovered_count: usize,
    pub recovered_run_ids: Vec<Uuid>,
    pub ops_summary: InvestigationOpsSummary,
}

#[derive(Debug, sqlx::FromRow)]
struct RunCountsRow {
    queued: i64,
    running: i64,
    watching: i64,
    escalated: i64,
    completed: i64,
    failed: i64,
    archived: i64,
    manual: i64,
    auto: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct MissionCountsRow {
    active: i64,
    paused: i64,
    archived: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct LoopHealthRow {
    auto_runs_24h: i64,
    retry_actions_24h: i64,
    recovery_actions_24h: i64,
    failure_rate_24h_pct: f64,
    average_completion_minutes_24h: Option<f64>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct InvestigationOperatorControlsRow {
    pub auto_investigation_paused: bool,
}

pub async fn get_investigation_ops_summary(
    State(state): State<AppState>,
) -> Result<Json<InvestigationOpsSummary>, AppError> {
    let run_counts = sqlx::query_as::<_, RunCountsRow>(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE status = 'queued')::bigint AS queued,
            COUNT(*) FILTER (WHERE status = 'running')::bigint AS running,
            COUNT(*) FILTER (WHERE status = 'watching')::bigint AS watching,
            COUNT(*) FILTER (WHERE status = 'escalated')::bigint AS escalated,
            COUNT(*) FILTER (WHERE status = 'completed')::bigint AS completed,
            COUNT(*) FILTER (WHERE status = 'failed')::bigint AS failed,
            COUNT(*) FILTER (WHERE status = 'archived')::bigint AS archived,
            COUNT(*) FILTER (WHERE trigger_type = 'manual')::bigint AS manual,
            COUNT(*) FILTER (WHERE trigger_type = 'auto')::bigint AS auto
        FROM investigation_runs
        "#,
    )
    .fetch_one(&state.db)
    .await?;

    let watchlist_items =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::bigint FROM investigation_watchlist_items")
            .fetch_one(&state.db)
            .await?;

    let mission_counts = sqlx::query_as::<_, MissionCountsRow>(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE status = 'active')::bigint AS active,
            COUNT(*) FILTER (WHERE status = 'paused')::bigint AS paused,
            COUNT(*) FILTER (WHERE status = 'archived')::bigint AS archived
        FROM investigation_missions
        "#,
    )
    .fetch_one(&state.db)
    .await?;

    let loop_health = load_loop_health(&state.db).await?;
    let controls = load_operator_controls(&state.db).await?;
    let auto_settings = AutoInvestigationSettings::from_config(&state.config);
    let stale_running_runs = load_stale_running_count(&state.db, 30).await?;
    let degradation_notes =
        load_degradation_notes(&state.db, run_counts.failed, stale_running_runs).await?;

    Ok(Json(InvestigationOpsSummary {
        runs: InvestigationOpsRunCounts {
            queued: run_counts.queued,
            running: run_counts.running,
            watching: run_counts.watching,
            escalated: run_counts.escalated,
            completed: run_counts.completed,
            failed: run_counts.failed,
            archived: run_counts.archived,
        },
        triggers: InvestigationOpsTriggerCounts {
            manual: run_counts.manual,
            auto: run_counts.auto,
        },
        loop_health: InvestigationOpsLoopHealth {
            auto_runs_24h: loop_health.auto_runs_24h,
            retry_actions_24h: loop_health.retry_actions_24h,
            recovery_actions_24h: loop_health.recovery_actions_24h,
            failure_rate_24h_pct: loop_health.failure_rate_24h_pct,
            average_completion_minutes_24h: loop_health.average_completion_minutes_24h,
        },
        watchlist_items,
        missions: InvestigationOpsMissionCounts {
            active: mission_counts.active,
            paused: mission_counts.paused,
            archived: mission_counts.archived,
        },
        auto_investigation: InvestigationOpsAutoConfig::from_settings(
            &auto_settings,
            controls.auto_investigation_paused,
        ),
        degradation_notes,
    }))
}

impl InvestigationOpsAutoConfig {
    fn from_settings(settings: &AutoInvestigationSettings, paused: bool) -> Self {
        Self {
            enabled: settings.enabled,
            paused,
            tx_threshold: settings.tx_threshold,
            cooldown_mins: settings.cooldown_mins,
        }
    }
}

pub async fn update_investigation_ops_control(
    State(state): State<AppState>,
    Json(payload): Json<UpdateInvestigationOpsControlRequest>,
) -> Result<Json<InvestigationOpsSummary>, AppError> {
    sqlx::query(
        r#"
        INSERT INTO investigation_operator_controls (id, auto_investigation_paused, updated_at)
        VALUES (TRUE, $1, NOW())
        ON CONFLICT (id) DO UPDATE
        SET auto_investigation_paused = EXCLUDED.auto_investigation_paused,
            updated_at = NOW()
        "#,
    )
    .bind(payload.auto_investigation_paused)
    .execute(&state.db)
    .await?;

    get_investigation_ops_summary(State(state)).await
}

pub async fn archive_stale_investigation_runs(
    State(state): State<AppState>,
    Json(payload): Json<ArchiveStaleRunsRequest>,
) -> Result<Json<ArchiveStaleRunsResponse>, AppError> {
    let stale_after_minutes = payload.stale_after_minutes.unwrap_or(0).max(0);

    let run_ids = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id
        FROM investigation_runs
        WHERE status IN ('completed', 'failed')
          AND updated_at <= NOW() - make_interval(mins => $1::int)
        ORDER BY updated_at ASC, created_at ASC
        LIMIT 100
        "#,
    )
    .bind(stale_after_minutes as i32)
    .fetch_all(&state.db)
    .await?;

    for run_id in &run_ids {
        let reason = format!(
            "Operator cleanup archived this terminal run after the configured stale window of {} minute(s).",
            stale_after_minutes
        );
        let delta = "Cleanup policy moved a completed or failed run into archived so the active inbox stays readable.";

        sqlx::query(
            r#"
            UPDATE investigation_runs
            SET
                status = 'archived',
                current_stage = 'archived',
                status_reason = $2,
                evidence_delta = $3,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(run_id)
        .bind(&reason)
        .bind(delta)
        .execute(&state.db)
        .await?;

        append_run_event(
            &state.db,
            *run_id,
            "operator_cleanup_archived",
            "Operator cleanup archive",
            &format!("{reason} Evidence delta: {delta}"),
            Some(&reason),
            Some(delta),
        )
        .await?;
    }

    let summary = get_investigation_ops_summary(State(state.clone())).await?.0;

    Ok(Json(ArchiveStaleRunsResponse {
        archived_count: run_ids.len(),
        archived_run_ids: run_ids,
        ops_summary: summary,
    }))
}

pub async fn retry_failed_investigation_runs(
    State(state): State<AppState>,
) -> Result<Json<RetryFailedRunsResponse>, AppError> {
    let run_ids = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id
        FROM investigation_runs
        WHERE status = 'failed'
        ORDER BY updated_at ASC, created_at ASC
        LIMIT 100
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    for run_id in &run_ids {
        let reason =
            "Operator retried this failed run from the runs inbox so it can re-enter the investigation queue.";
        let delta =
            "Retry control moved the run from failed into queued for another investigation pass.";

        sqlx::query(
            r#"
            UPDATE investigation_runs
            SET
                status = 'queued',
                current_stage = 'retry_queued',
                status_reason = $2,
                evidence_delta = $3,
                updated_at = NOW(),
                completed_at = NULL
            WHERE id = $1
            "#,
        )
        .bind(run_id)
        .bind(reason)
        .bind(delta)
        .execute(&state.db)
        .await?;

        append_run_event(
            &state.db,
            *run_id,
            "operator_retry_queued",
            "Operator retry queued",
            &format!("{reason} Evidence delta: {delta}"),
            Some(reason),
            Some(delta),
        )
        .await?;
    }

    let summary = get_investigation_ops_summary(State(state.clone())).await?.0;

    Ok(Json(RetryFailedRunsResponse {
        retried_count: run_ids.len(),
        retried_run_ids: run_ids,
        ops_summary: summary,
    }))
}

pub async fn recover_stale_running_investigation_runs(
    State(state): State<AppState>,
    Json(payload): Json<RecoverStaleRunningRunsRequest>,
) -> Result<Json<RecoverStaleRunningRunsResponse>, AppError> {
    let stale_after_minutes = payload.stale_after_minutes.unwrap_or(30).max(0);

    let run_ids = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id
        FROM investigation_runs
        WHERE status = 'running'
          AND updated_at <= NOW() - make_interval(mins => $1::int)
        ORDER BY updated_at ASC, created_at ASC
        LIMIT 100
        "#,
    )
    .bind(stale_after_minutes as i32)
    .fetch_all(&state.db)
    .await?;

    for run_id in &run_ids {
        let reason = format!(
            "Operator recovered this stale running run after it exceeded the {} minute recovery window.",
            stale_after_minutes
        );
        let delta =
            "Recovery control moved the run from running back into queued so the investigation can resume safely.";

        sqlx::query(
            r#"
            UPDATE investigation_runs
            SET
                status = 'queued',
                current_stage = 'recovery_queued',
                status_reason = $2,
                evidence_delta = $3,
                updated_at = NOW(),
                completed_at = NULL
            WHERE id = $1
            "#,
        )
        .bind(run_id)
        .bind(&reason)
        .bind(delta)
        .execute(&state.db)
        .await?;

        append_run_event(
            &state.db,
            *run_id,
            "operator_recovery_queued",
            "Operator recovery queued",
            &format!("{reason} Evidence delta: {delta}"),
            Some(&reason),
            Some(delta),
        )
        .await?;
    }

    let summary = get_investigation_ops_summary(State(state.clone())).await?.0;

    Ok(Json(RecoverStaleRunningRunsResponse {
        recovered_count: run_ids.len(),
        recovered_run_ids: run_ids,
        ops_summary: summary,
    }))
}

async fn load_loop_health(db: &sqlx::PgPool) -> Result<LoopHealthRow, AppError> {
    let row = sqlx::query_as::<_, LoopHealthRow>(
        r#"
        SELECT
            (
                SELECT COUNT(*)::bigint
                FROM investigation_runs
                WHERE trigger_type = 'auto'
                  AND created_at >= NOW() - INTERVAL '24 hours'
            ) AS auto_runs_24h,
            (
                SELECT COUNT(*)::bigint
                FROM investigation_run_events
                WHERE event_key = 'operator_retry_queued'
                  AND created_at >= NOW() - INTERVAL '24 hours'
            ) AS retry_actions_24h,
            (
                SELECT COUNT(*)::bigint
                FROM investigation_run_events
                WHERE event_key = 'operator_recovery_queued'
                  AND created_at >= NOW() - INTERVAL '24 hours'
            ) AS recovery_actions_24h,
            (
                SELECT COALESCE(
                    ROUND(
                        (
                            COUNT(*) FILTER (WHERE status = 'failed')::numeric
                            / NULLIF(COUNT(*) FILTER (WHERE status IN ('completed', 'failed')), 0)::numeric
                        ) * 100,
                        1
                    )::float8,
                    0::float8
                )
                FROM investigation_runs
                WHERE updated_at >= NOW() - INTERVAL '24 hours'
                  AND status IN ('completed', 'failed')
            ) AS failure_rate_24h_pct,
            (
                SELECT ROUND(
                    AVG(EXTRACT(EPOCH FROM (completed_at - COALESCE(started_at, created_at))) / 60.0)::numeric,
                    1
                )::float8
                FROM investigation_runs
                WHERE completed_at IS NOT NULL
                  AND completed_at >= NOW() - INTERVAL '24 hours'
            ) AS average_completion_minutes_24h
        "#,
    )
    .fetch_one(db)
    .await?;

    Ok(row)
}

pub async fn load_operator_controls(
    db: &sqlx::PgPool,
) -> Result<InvestigationOperatorControlsRow, AppError> {
    let controls = sqlx::query_as::<_, InvestigationOperatorControlsRow>(
        r#"
        SELECT auto_investigation_paused
        FROM investigation_operator_controls
        WHERE id = TRUE
        "#,
    )
    .fetch_optional(db)
    .await?;

    Ok(controls.unwrap_or(InvestigationOperatorControlsRow {
        auto_investigation_paused: false,
    }))
}

async fn load_degradation_notes(
    db: &sqlx::PgPool,
    failed_runs: i64,
    stale_running_runs: i64,
) -> Result<Vec<InvestigationOpsDegradationNote>, AppError> {
    let mut notes = Vec::new();

    let source_degradation_runs = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)::bigint
        FROM investigation_runs
        WHERE status = 'watching'
          AND (
            LOWER(COALESCE(status_reason, '')) LIKE '%source health degraded%'
            OR LOWER(COALESCE(status_reason, '')) LIKE '%source degradation%'
          )
        "#,
    )
    .fetch_one(db)
    .await?;

    if source_degradation_runs > 0 {
        notes.push(InvestigationOpsDegradationNote {
            code: "source_degradation".to_string(),
            level: "warn".to_string(),
            message: format!(
                "{} monitoring run{} cooled because source health degraded. Re-check provider and citation quality before treating those runs as current.",
                source_degradation_runs,
                if source_degradation_runs == 1 { "" } else { "s" }
            ),
        });
    }

    let degraded_report_sources = sqlx::query_scalar::<_, i64>(
        r#"
        WITH latest_reports AS (
            SELECT DISTINCT ON (token_address)
                token_address,
                source_status,
                citations,
                updated_at
            FROM deep_research_reports
            ORDER BY token_address, updated_at DESC
        )
        SELECT COUNT(*)::bigint
        FROM latest_reports
        WHERE COALESCE(source_status #>> '{dexscreener,status}', '') = 'degraded'
        "#,
    )
    .fetch_one(db)
    .await?;

    if degraded_report_sources > 0 {
        notes.push(InvestigationOpsDegradationNote {
            code: "degraded_report_sources".to_string(),
            level: "warn".to_string(),
            message: format!(
                "{} latest report snapshot{} show degraded source providers. Treat evidence freshness cautiously until source health recovers.",
                degraded_report_sources,
                if degraded_report_sources == 1 { "" } else { "s" }
            ),
        });
    }

    let empty_citation_reports = sqlx::query_scalar::<_, i64>(
        r#"
        WITH latest_reports AS (
            SELECT DISTINCT ON (token_address)
                token_address,
                citations,
                updated_at
            FROM deep_research_reports
            ORDER BY token_address, updated_at DESC
        )
        SELECT COUNT(*)::bigint
        FROM latest_reports
        WHERE jsonb_typeof(citations) = 'array'
          AND jsonb_array_length(citations) = 0
        "#,
    )
    .fetch_one(db)
    .await?;

    if empty_citation_reports > 0 {
        notes.push(InvestigationOpsDegradationNote {
            code: "empty_citations".to_string(),
            level: "warn".to_string(),
            message: format!(
                "{} latest report snapshot{} have empty citations. Evidence quality is weaker than a fully sourced report.",
                empty_citation_reports,
                if empty_citation_reports == 1 { "" } else { "s" }
            ),
        });
    }

    let latest_alpha_window: Option<(chrono::DateTime<Utc>,)> =
        sqlx::query_as("SELECT MAX(window_end) FROM alpha_rankings")
            .fetch_optional(db)
            .await
            .ok()
            .flatten();

    let alpha_stale = latest_alpha_window
        .map(|(window_end,)| window_end < (Utc::now() - Duration::hours(2)))
        .unwrap_or(true);

    if alpha_stale {
        notes.push(InvestigationOpsDegradationNote {
            code: "alpha_stale".to_string(),
            level: "warn".to_string(),
            message:
                "Alpha windows are stale right now, so ranking surfaces may lag behind the live investigation loop."
                    .to_string(),
        });
    }

    if failed_runs > 0 {
        notes.push(InvestigationOpsDegradationNote {
            code: "failed_runs".to_string(),
            level: "warn".to_string(),
            message: format!(
                "{} failed run{} need operator review or retry before the operating surface is fully clean.",
                failed_runs,
                if failed_runs == 1 { "" } else { "s" }
            ),
        });
    }

    if stale_running_runs > 0 {
        notes.push(InvestigationOpsDegradationNote {
            code: "stale_running".to_string(),
            level: "warn".to_string(),
            message: format!(
                "{} running run{} exceeded the recovery window and may need operator recovery.",
                stale_running_runs,
                if stale_running_runs == 1 { "" } else { "s" }
            ),
        });
    }

    Ok(notes)
}

async fn load_stale_running_count(
    db: &sqlx::PgPool,
    stale_after_minutes: i64,
) -> Result<i64, AppError> {
    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)::bigint
        FROM investigation_runs
        WHERE status = 'running'
          AND updated_at <= NOW() - make_interval(mins => $1::int)
        "#,
    )
    .bind(stale_after_minutes as i32)
    .fetch_one(db)
    .await?;

    Ok(count)
}
