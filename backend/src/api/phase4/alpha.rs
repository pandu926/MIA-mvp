use super::types::{
    evaluate_alpha_outcome, round2, AlphaBacktestQuery, AlphaBacktestResponse,
    AlphaBacktestRowResponse, AlphaHistoryQuery, AlphaRowResponse, LimitQuery,
};
use crate::{config::MlRolloutMode, error::AppError, AppState};
use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, Duration, Utc};
use std::{cmp::Ordering, collections::HashMap};

/// GET /api/v1/alpha/latest
pub async fn get_latest_alpha(
    State(state): State<AppState>,
    Query(params): Query<LimitQuery>,
) -> Result<Json<Vec<AlphaRowResponse>>, AppError> {
    let limit = params.limit.clamp(1, 50);
    let latest_window: (Option<DateTime<Utc>>,) =
        sqlx::query_as("SELECT MAX(window_end) FROM alpha_rankings")
            .fetch_one(&state.db)
            .await?;

    let Some(window_end) = latest_window.0 else {
        return Ok(Json(vec![]));
    };

    let rows: Vec<(DateTime<Utc>, DateTime<Utc>, i16, String, f64, String)> = sqlx::query_as(
        r#"
        SELECT window_start, window_end, rank, token_address,
               alpha_score::double precision, rationale
        FROM alpha_rankings
        WHERE window_end = $1
        ORDER BY rank ASC
        LIMIT $2
        "#,
    )
    .bind(window_end)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    let mut out: Vec<AlphaRowResponse> = rows
        .into_iter()
        .map(
            |(window_start, window_end, rank, token_address, alpha_score, rationale)| {
                AlphaRowResponse {
                    window_start,
                    window_end,
                    rank,
                    token_address,
                    alpha_score,
                    rationale,
                    score_source: "legacy".to_string(),
                    model_version: state.config.ml_model_version.clone(),
                    score_confidence: None,
                    explanations: vec![
                        "Risk-adjusted momentum composite".to_string(),
                        "Legacy formula retained for backward compatibility".to_string(),
                    ],
                }
            },
        )
        .collect();

    if matches!(
        state.config.ml_rollout_mode,
        MlRolloutMode::Ml | MlRolloutMode::Hybrid
    ) {
        let ml_rows: Vec<(String, f64, f64)> = sqlx::query_as(
            r#"
            SELECT token_address,
                   score::double precision,
                   confidence::double precision
            FROM ml_alpha_predictions
            WHERE window_end = $1
              AND score_source = 'ml'
              AND model_version = $2
            "#,
        )
        .bind(window_end)
        .bind(&state.config.ml_model_version)
        .fetch_all(&state.db)
        .await?;

        let ml_map: HashMap<String, (f64, f64)> = ml_rows
            .into_iter()
            .map(|(address, score, confidence)| (address.to_lowercase(), (score, confidence)))
            .collect();

        for row in &mut out {
            if let Some((ml_score, confidence)) = ml_map.get(&row.token_address.to_lowercase()) {
                match state.config.ml_rollout_mode {
                    MlRolloutMode::Ml => {
                        row.alpha_score = round2(*ml_score);
                        row.score_source = "ml".to_string();
                        row.score_confidence = Some(round2(*confidence * 100.0));
                        row.explanations = vec![
                            "Score produced by active ML model".to_string(),
                            "Ranked by calibrated hit probability".to_string(),
                        ];
                    }
                    MlRolloutMode::Hybrid => {
                        let blended = (row.alpha_score * 0.5) + (*ml_score * 0.5);
                        row.alpha_score = round2(blended);
                        row.score_source = "hybrid".to_string();
                        row.score_confidence = Some(round2(*confidence * 100.0));
                        row.explanations = vec![
                            "Hybrid blend of legacy and ML score (50/50)".to_string(),
                            "Fallback-safe transition mode".to_string(),
                        ];
                    }
                    MlRolloutMode::Shadow | MlRolloutMode::Legacy => {}
                }
            }
        }

        out.sort_by(|left, right| {
            right
                .alpha_score
                .partial_cmp(&left.alpha_score)
                .unwrap_or(Ordering::Equal)
        });
        for (idx, row) in out.iter_mut().enumerate() {
            row.rank = (idx + 1) as i16;
        }
    }

    Ok(Json(out.into_iter().take(limit as usize).collect()))
}

/// GET /api/v1/alpha/history?hours=24&limit=100
pub async fn get_alpha_history(
    State(state): State<AppState>,
    Query(params): Query<AlphaHistoryQuery>,
) -> Result<Json<Vec<AlphaRowResponse>>, AppError> {
    let since = Utc::now() - Duration::hours(params.hours.clamp(1, 168));
    let limit = params.limit.clamp(1, 500);
    let rows: Vec<(DateTime<Utc>, DateTime<Utc>, i16, String, f64, String)> = sqlx::query_as(
        r#"
        SELECT window_start, window_end, rank, token_address,
               alpha_score::double precision, rationale
        FROM alpha_rankings
        WHERE window_end >= $1
        ORDER BY window_end DESC, rank ASC
        LIMIT $2
        "#,
    )
    .bind(since)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(
                |(window_start, window_end, rank, token_address, alpha_score, rationale)| {
                    AlphaRowResponse {
                        window_start,
                        window_end,
                        rank,
                        token_address,
                        alpha_score,
                        rationale,
                        score_source: "legacy".to_string(),
                        model_version: state.config.ml_model_version.clone(),
                        score_confidence: None,
                        explanations: vec![
                            "Risk-adjusted momentum composite".to_string(),
                            "Legacy formula retained for backward compatibility".to_string(),
                        ],
                    }
                },
            )
            .collect(),
    ))
}

/// GET /api/v1/alpha/backtest?hours=24&limit=120
pub async fn get_alpha_backtest(
    State(state): State<AppState>,
    Query(params): Query<AlphaBacktestQuery>,
) -> Result<Json<AlphaBacktestResponse>, AppError> {
    let since = Utc::now() - Duration::hours(params.hours.clamp(1, 168));
    let limit = params.limit.clamp(1, 300);

    type BacktestRaw = (
        DateTime<Utc>,
        i16,
        String,
        f64,
        f64,
        f64,
        i64,
        i64,
        f64,
        i64,
        i64,
    );

    let rows: Vec<BacktestRaw> = sqlx::query_as(
        r#"
        SELECT
            ar.window_end,
            ar.rank,
            ar.token_address,
            ar.alpha_score::double precision,
            COALESCE(base.volume, 0)::double precision AS baseline_volume_1h,
            COALESCE(f1.volume, 0)::double precision AS future_volume_1h,
            COALESCE(f1.buys, 0)::bigint AS future_buy_count_1h,
            COALESCE(f1.sells, 0)::bigint AS future_sell_count_1h,
            COALESCE(f6.volume, 0)::double precision AS future_volume_6h,
            COALESCE(f6.buys, 0)::bigint AS future_buy_count_6h,
            COALESCE(f6.sells, 0)::bigint AS future_sell_count_6h
        FROM alpha_rankings ar
        LEFT JOIN LATERAL (
            SELECT SUM(tt.amount_bnb) AS volume
            FROM token_transactions tt
            WHERE tt.token_address = ar.token_address
              AND tt.created_at > ar.window_end - INTERVAL '1 hour'
              AND tt.created_at <= ar.window_end
        ) base ON TRUE
        LEFT JOIN LATERAL (
            SELECT
                SUM(tt.amount_bnb) AS volume,
                COUNT(*) FILTER (WHERE tt.tx_type = 'buy') AS buys,
                COUNT(*) FILTER (WHERE tt.tx_type = 'sell') AS sells
            FROM token_transactions tt
            WHERE tt.token_address = ar.token_address
              AND tt.created_at > ar.window_end
              AND tt.created_at <= ar.window_end + INTERVAL '1 hour'
        ) f1 ON TRUE
        LEFT JOIN LATERAL (
            SELECT
                SUM(tt.amount_bnb) AS volume,
                COUNT(*) FILTER (WHERE tt.tx_type = 'buy') AS buys,
                COUNT(*) FILTER (WHERE tt.tx_type = 'sell') AS sells
            FROM token_transactions tt
            WHERE tt.token_address = ar.token_address
              AND tt.created_at > ar.window_end
              AND tt.created_at <= ar.window_end + INTERVAL '6 hour'
        ) f6 ON TRUE
        WHERE ar.window_end >= $1
        ORDER BY ar.window_end DESC, ar.rank ASC
        LIMIT $2
        "#,
    )
    .bind(since)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    let mut hit_1h = 0usize;
    let mut hit_6h = 0usize;
    let mut sum_1h = 0.0;
    let mut sum_6h = 0.0;
    let mut out_rows = Vec::with_capacity(rows.len());

    for (
        window_end,
        rank,
        token_address,
        alpha_score,
        baseline_volume_1h,
        future_volume_1h,
        future_buy_count_1h,
        future_sell_count_1h,
        future_volume_6h,
        future_buy_count_6h,
        future_sell_count_6h,
    ) in rows
    {
        let (score_1h, outcome_1h, is_hit_1h) = evaluate_alpha_outcome(
            baseline_volume_1h,
            future_volume_1h,
            future_buy_count_1h,
            future_sell_count_1h,
        );
        let (score_6h, outcome_6h, is_hit_6h) = evaluate_alpha_outcome(
            baseline_volume_1h,
            future_volume_6h,
            future_buy_count_6h,
            future_sell_count_6h,
        );

        let _ = sqlx::query(
            r#"
            UPDATE ml_alpha_predictions
            SET realized_hit_1h = $3,
                realized_score_1h = $4,
                realized_at = NOW()
            WHERE window_end = $1
              AND token_address = $2
            "#,
        )
        .bind(window_end)
        .bind(&token_address)
        .bind(is_hit_1h)
        .bind(score_1h)
        .execute(&state.db)
        .await;

        if is_hit_1h {
            hit_1h += 1;
        }
        if is_hit_6h {
            hit_6h += 1;
        }
        sum_1h += score_1h;
        sum_6h += score_6h;

        out_rows.push(AlphaBacktestRowResponse {
            window_end,
            rank,
            token_address,
            alpha_score: round2(alpha_score),
            baseline_volume_1h: round2(baseline_volume_1h),
            future_volume_1h: round2(future_volume_1h),
            future_buy_count_1h,
            future_sell_count_1h,
            score_1h,
            outcome_1h: outcome_1h.to_string(),
            future_volume_6h: round2(future_volume_6h),
            future_buy_count_6h,
            future_sell_count_6h,
            score_6h,
            outcome_6h: outcome_6h.to_string(),
        });
    }

    let evaluated = out_rows.len();
    let denom = evaluated.max(1) as f64;

    Ok(Json(AlphaBacktestResponse {
        evaluated,
        hit_rate_1h: round2((hit_1h as f64 / denom) * 100.0),
        hit_rate_6h: round2((hit_6h as f64 / denom) * 100.0),
        average_score_1h: round2(sum_1h / denom),
        average_score_6h: round2(sum_6h / denom),
        rows: out_rows,
    }))
}
