use crate::{config::MlRolloutMode, error::AppError, AppState};
use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize)]
pub struct MlHealthResponse {
    pub status: String,
    pub rollout_mode: MlRolloutMode,
    pub model_version: String,
    pub min_confidence: f64,
    pub shadow_enabled: bool,
    pub predictions_24h: i64,
    pub last_prediction_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct MlEvalQuery {
    #[serde(default = "default_hours")]
    pub hours: i64,
}

fn default_hours() -> i64 {
    24
}

#[derive(Debug, Serialize)]
pub struct MlAlphaEvalResponse {
    pub hours: i64,
    pub evaluated_pairs: i64,
    pub legacy_hit_rate: f64,
    pub ml_hit_rate: f64,
    pub uplift_pct_points: f64,
}

#[derive(Debug, Serialize)]
pub struct MlDecisionResponse {
    pub hours: i64,
    pub evaluated_pairs: i64,
    pub legacy_hit_rate: f64,
    pub ml_hit_rate: f64,
    pub uplift_pct_points: f64,
    pub recommendation: String,
    pub recommended_mode: String,
    pub rationale: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct MlModelRegistryRow {
    pub model_version: String,
    pub rollout_mode: String,
    pub is_active: bool,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ActivateModelRequest {
    pub model_version: String,
    pub rollout_mode: Option<String>,
    #[serde(default = "default_true")]
    pub activate: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct ActivateModelResponse {
    pub model_version: String,
    pub rollout_mode: String,
    pub is_active: bool,
    pub updated_at: DateTime<Utc>,
}

pub async fn get_ml_health(
    State(state): State<AppState>,
) -> Result<Json<MlHealthResponse>, AppError> {
    let predictions_24h: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::bigint FROM ml_alpha_predictions WHERE created_at >= NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(&state.db)
    .await?;

    let last_prediction_at: (Option<DateTime<Utc>>,) =
        sqlx::query_as("SELECT MAX(created_at) FROM ml_alpha_predictions")
            .fetch_one(&state.db)
            .await?;

    let rollout_mode = state.config.ml_rollout_mode;
    let shadow_enabled = matches!(rollout_mode, MlRolloutMode::Shadow | MlRolloutMode::Hybrid);

    Ok(Json(MlHealthResponse {
        status: "ok".to_string(),
        rollout_mode,
        model_version: state.config.ml_model_version.clone(),
        min_confidence: state.config.ml_min_confidence,
        shadow_enabled,
        predictions_24h: predictions_24h.0,
        last_prediction_at: last_prediction_at.0,
    }))
}

pub async fn get_ml_alpha_eval(
    State(state): State<AppState>,
    Query(params): Query<MlEvalQuery>,
) -> Result<Json<MlAlphaEvalResponse>, AppError> {
    let hours = params.hours.clamp(1, 336);
    let (evaluated, legacy, ml) = eval_aggregate(&state.db, hours).await?;

    Ok(Json(MlAlphaEvalResponse {
        hours,
        evaluated_pairs: evaluated,
        legacy_hit_rate: round2(legacy),
        ml_hit_rate: round2(ml),
        uplift_pct_points: round2(ml - legacy),
    }))
}

pub async fn get_ml_decision(
    State(state): State<AppState>,
    Query(params): Query<MlEvalQuery>,
) -> Result<Json<MlDecisionResponse>, AppError> {
    let hours = params.hours.clamp(24, 336);
    let (evaluated, legacy, ml) = eval_aggregate(&state.db, hours).await?;
    let uplift = ml - legacy;
    let current_mode = state.config.ml_rollout_mode;

    let (recommendation, recommended_mode, rationale) = if evaluated < 50 {
        (
            "hold_shadow",
            "shadow",
            format!(
                "Need more evaluated pairs before cutover ({} < 50). Keep shadow collection running.",
                evaluated
            ),
        )
    } else if matches!(current_mode, MlRolloutMode::Hybrid) && uplift >= 12.0 && evaluated >= 100 {
        (
            "promote_to_ml",
            "ml",
            format!(
                "Hybrid stage is stable with strong uplift (+{:.2}pp, n={}). Promote to full ML mode.",
                uplift, evaluated
            ),
        )
    } else if uplift >= 10.0 {
        (
            "promote_to_hybrid",
            "hybrid",
            format!(
                "Uplift is strong (+{:.2}pp). Move to hybrid for controlled production exposure.",
                uplift
            ),
        )
    } else if uplift <= 0.0 {
        (
            "stay_shadow",
            "shadow",
            format!(
                "No positive uplift ({:.2}pp). Keep shadow mode and retrain/tune before cutover.",
                uplift
            ),
        )
    } else {
        (
            "extended_shadow",
            "shadow",
            format!(
                "Uplift is positive but below threshold ({:.2}pp < 10pp). Continue shadow evaluation.",
                uplift
            ),
        )
    };

    Ok(Json(MlDecisionResponse {
        hours,
        evaluated_pairs: evaluated,
        legacy_hit_rate: round2(legacy),
        ml_hit_rate: round2(ml),
        uplift_pct_points: round2(uplift),
        recommendation: recommendation.to_string(),
        recommended_mode: recommended_mode.to_string(),
        rationale,
    }))
}

pub async fn list_models(
    State(state): State<AppState>,
) -> Result<Json<Vec<MlModelRegistryRow>>, AppError> {
    let rows: Vec<MlModelRegistryRow> = sqlx::query_as(
        r#"
        SELECT model_version, rollout_mode, is_active, metadata, created_at, updated_at
        FROM ml_model_registry
        ORDER BY is_active DESC, updated_at DESC
        "#,
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

pub async fn activate_model(
    State(state): State<AppState>,
    Json(payload): Json<ActivateModelRequest>,
) -> Result<Json<ActivateModelResponse>, AppError> {
    let rollout_mode = payload
        .rollout_mode
        .as_deref()
        .and_then(parse_rollout_mode)
        .unwrap_or(state.config.ml_rollout_mode);

    let rollout_mode_sql = match rollout_mode {
        MlRolloutMode::Legacy => "legacy",
        MlRolloutMode::Shadow => "shadow",
        MlRolloutMode::Ml => "ml",
        MlRolloutMode::Hybrid => "hybrid",
    };

    let mut tx = state.db.begin().await?;
    if payload.activate {
        sqlx::query("UPDATE ml_model_registry SET is_active = false WHERE is_active = true")
            .execute(&mut *tx)
            .await?;
    }
    let row: (String, String, bool, DateTime<Utc>) = sqlx::query_as(
        r#"
        INSERT INTO ml_model_registry (model_version, rollout_mode, is_active, metadata, updated_at)
        VALUES ($1, $2, $3, '{}'::jsonb, NOW())
        ON CONFLICT (model_version) DO UPDATE SET
            rollout_mode = EXCLUDED.rollout_mode,
            is_active = EXCLUDED.is_active,
            updated_at = NOW()
        RETURNING model_version, rollout_mode, is_active, updated_at
        "#,
    )
    .bind(&payload.model_version)
    .bind(rollout_mode_sql)
    .bind(payload.activate)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(ActivateModelResponse {
        model_version: row.0,
        rollout_mode: row.1,
        is_active: row.2,
        updated_at: row.3,
    }))
}

fn parse_rollout_mode(raw: &str) -> Option<MlRolloutMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "legacy" => Some(MlRolloutMode::Legacy),
        "shadow" => Some(MlRolloutMode::Shadow),
        "ml" => Some(MlRolloutMode::Ml),
        "hybrid" => Some(MlRolloutMode::Hybrid),
        _ => None,
    }
}

async fn eval_aggregate(db: &sqlx::PgPool, hours: i64) -> Result<(i64, f64, f64), AppError> {
    const EVAL_TOP_K: i64 = 3;
    let since = Utc::now() - Duration::hours(hours);
    type EvalAgg = (i64, Option<f64>, Option<f64>);
    let eval: EvalAgg = sqlx::query_as(
        r#"
        WITH base AS (
            SELECT
                p.window_end,
                p.token_address,
                p.score_source,
                p.score::double precision AS score,
                p.realized_hit_1h::int AS hit
            FROM ml_alpha_predictions p
            WHERE p.window_end >= $1
              AND p.realized_hit_1h IS NOT NULL
              AND p.score_source IN ('legacy', 'ml', 'shadow_ml_stub')
        ),
        legacy_ranked AS (
            SELECT
                b.window_end,
                b.hit,
                ROW_NUMBER() OVER (
                    PARTITION BY b.window_end
                    ORDER BY b.score DESC, b.token_address ASC
                ) AS rn
            FROM base b
            WHERE b.score_source = 'legacy'
        ),
        ml_dedup AS (
            SELECT DISTINCT ON (b.window_end, b.token_address)
                b.window_end,
                b.token_address,
                b.score,
                b.hit
            FROM base b
            WHERE b.score_source IN ('ml', 'shadow_ml_stub')
            ORDER BY
                b.window_end,
                b.token_address,
                CASE WHEN b.score_source = 'ml' THEN 0 ELSE 1 END,
                b.score DESC
        ),
        ml_ranked AS (
            SELECT
                m.window_end,
                m.hit,
                ROW_NUMBER() OVER (
                    PARTITION BY m.window_end
                    ORDER BY m.score DESC, m.token_address ASC
                ) AS rn
            FROM ml_dedup m
        ),
        legacy_top AS (
            SELECT
                lr.window_end,
                AVG(lr.hit::double precision) * 100 AS legacy_hit_rate
            FROM legacy_ranked lr
            WHERE lr.rn <= $2
            GROUP BY lr.window_end
        ),
        ml_top AS (
            SELECT
                mr.window_end,
                AVG(mr.hit::double precision) * 100 AS ml_hit_rate
            FROM ml_ranked mr
            WHERE mr.rn <= $2
            GROUP BY mr.window_end
        )
        SELECT
            COUNT(*)::bigint AS evaluated,
            AVG(lt.legacy_hit_rate) AS legacy_hit_rate,
            AVG(mt.ml_hit_rate) AS ml_hit_rate
        FROM legacy_top lt
        JOIN ml_top mt
          ON mt.window_end = lt.window_end
        "#,
    )
    .bind(since)
    .bind(EVAL_TOP_K)
    .fetch_one(db)
    .await?;
    Ok((eval.0, eval.1.unwrap_or(0.0), eval.2.unwrap_or(0.0)))
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
