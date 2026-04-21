use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgPool};

const HORIZONS: [i16; 3] = [1, 6, 24];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternAnalog {
    pub token_address: String,
    pub window_end: String,
    pub match_score: f64,
    pub match_label: String,
    pub outcome_class: String,
    pub rationale: String,
    pub notable_differences: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PatternPrediction {
    pub horizon_hours: i16,
    pub model_version: String,
    pub match_label: String,
    pub outcome_class: String,
    pub score: f64,
    pub confidence: f64,
    pub anomaly_score: Option<f64>,
    pub expected_path_summary: String,
    pub rationale: String,
    pub analogs: Vec<PatternAnalog>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PatternEngineSummary {
    pub summary: String,
    pub evidence: Vec<String>,
    pub model_version: String,
    pub horizons: Vec<PatternPrediction>,
}

#[derive(Debug, FromRow)]
struct PatternPredictionRow {
    horizon_hours: i16,
    model_version: String,
    match_label: String,
    outcome_class: String,
    score: f64,
    confidence: f64,
    anomaly_score: Option<f64>,
    expected_path_summary: String,
    rationale: String,
    analogs: Value,
    created_at: DateTime<Utc>,
}

fn format_signal_line(prediction: &PatternPrediction) -> String {
    let mut line = format!(
        "{}H pattern read: {} / {} at {:.0}% confidence.",
        prediction.horizon_hours,
        prediction.match_label.replace('_', " "),
        prediction.outcome_class.replace('_', " "),
        prediction.confidence * 100.0
    );
    if let Some(anomaly_score) = prediction.anomaly_score {
        line.push_str(&format!(" Anomaly guard: {:.2}.", anomaly_score));
    }
    line
}

fn summarize_horizon(prediction: &PatternPrediction) -> String {
    format!(
        "{}H leans {} with {:.0}% confidence; {}",
        prediction.horizon_hours,
        prediction.match_label.replace('_', " "),
        prediction.confidence * 100.0,
        prediction.expected_path_summary
    )
}

fn normalize_analogs(raw: Value) -> Vec<PatternAnalog> {
    serde_json::from_value(raw).unwrap_or_default()
}

pub(crate) fn build_pattern_engine_summary(
    model_version: &str,
    mut horizons: Vec<PatternPrediction>,
) -> Option<PatternEngineSummary> {
    if horizons.is_empty() {
        return None;
    }

    horizons.sort_by_key(|item| item.horizon_hours);

    let mut evidence = horizons.iter().map(format_signal_line).collect::<Vec<_>>();
    for prediction in &horizons {
        if let Some(top_analog) = prediction.analogs.first() {
            evidence.push(format!(
                "{}H top analog: {} scored {:.0}% and previously resolved as {}.",
                prediction.horizon_hours,
                top_analog.token_address,
                top_analog.match_score * 100.0,
                top_analog.outcome_class.replace('_', " ")
            ));
        }
    }

    let summary = format!(
        "Historical pattern context compared this token against MIA's indexed launch history across {} using model {}. Use this as supporting context alongside the live market, wallet, deployer, and linked-pattern layers.",
        horizons
            .iter()
            .map(summarize_horizon)
            .collect::<Vec<_>>()
            .join(" "),
        model_version
    );

    Some(PatternEngineSummary {
        summary,
        evidence,
        model_version: model_version.to_string(),
        horizons,
    })
}

pub async fn load_latest_pattern_engine_summary(
    db: &PgPool,
    token_address: &str,
) -> Result<Option<PatternEngineSummary>> {
    let active_model: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT model_version
        FROM ml_pattern_model_registry
        WHERE is_active = true
        ORDER BY updated_at DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(db)
    .await?;

    let Some((model_version,)) = active_model else {
        return Ok(None);
    };

    let rows: Vec<PatternPredictionRow> = sqlx::query_as(
        r#"
        SELECT DISTINCT ON (horizon_hours)
            horizon_hours,
            model_version,
            match_label,
            outcome_class,
            score::double precision,
            confidence::double precision,
            anomaly_score::double precision,
            expected_path_summary,
            rationale,
            analogs,
            created_at
        FROM ml_pattern_predictions
        WHERE LOWER(token_address) = LOWER($1)
          AND model_version = $2
          AND horizon_hours = ANY($3)
        ORDER BY horizon_hours, window_end DESC, created_at DESC
        "#,
    )
    .bind(token_address)
    .bind(&model_version)
    .bind(&HORIZONS)
    .fetch_all(db)
    .await?;

    if rows.is_empty() {
        return Ok(None);
    }

    let predictions = rows
        .into_iter()
        .map(|row| PatternPrediction {
            horizon_hours: row.horizon_hours,
            model_version: row.model_version,
            match_label: row.match_label,
            outcome_class: row.outcome_class,
            score: row.score,
            confidence: row.confidence,
            anomaly_score: row.anomaly_score,
            expected_path_summary: row.expected_path_summary,
            rationale: row.rationale,
            analogs: normalize_analogs(row.analogs),
            created_at: row.created_at,
        })
        .collect::<Vec<_>>();

    Ok(build_pattern_engine_summary(&model_version, predictions))
}

#[cfg(test)]
mod tests {
    use super::{build_pattern_engine_summary, PatternAnalog, PatternPrediction};
    use chrono::Utc;

    #[test]
    fn summary_uses_all_horizons_and_top_analogs() {
        let summary = build_pattern_engine_summary(
            "pattern-v1",
            vec![
                PatternPrediction {
                    horizon_hours: 24,
                    model_version: "pattern-v1".to_string(),
                    match_label: "pump_and_fade".to_string(),
                    outcome_class: "pump_and_fade".to_string(),
                    score: 0.61,
                    confidence: 0.71,
                    anomaly_score: Some(0.42),
                    expected_path_summary: "late strength is likely to fade.".to_string(),
                    rationale: "buyers weaken into distribution.".to_string(),
                    analogs: vec![PatternAnalog {
                        token_address: "0x24".to_string(),
                        window_end: "2026-04-19T00:00:00Z".to_string(),
                        match_score: 0.83,
                        match_label: "pump_and_fade".to_string(),
                        outcome_class: "pump_and_fade".to_string(),
                        rationale: "similar fade profile".to_string(),
                        notable_differences: vec!["broader wallets".to_string()],
                    }],
                    created_at: Utc::now(),
                },
                PatternPrediction {
                    horizon_hours: 1,
                    model_version: "pattern-v1".to_string(),
                    match_label: "thin_momentum".to_string(),
                    outcome_class: "thin_momentum".to_string(),
                    score: 0.57,
                    confidence: 0.68,
                    anomaly_score: None,
                    expected_path_summary: "early participation is narrow.".to_string(),
                    rationale: "wallet breadth is thin.".to_string(),
                    analogs: vec![PatternAnalog {
                        token_address: "0x01".to_string(),
                        window_end: "2026-04-19T00:00:00Z".to_string(),
                        match_score: 0.81,
                        match_label: "thin_momentum".to_string(),
                        outcome_class: "thin_momentum".to_string(),
                        rationale: "similar breadth".to_string(),
                        notable_differences: vec![],
                    }],
                    created_at: Utc::now(),
                },
                PatternPrediction {
                    horizon_hours: 6,
                    model_version: "pattern-v1".to_string(),
                    match_label: "healthy_rotation".to_string(),
                    outcome_class: "healthy_rotation".to_string(),
                    score: 0.63,
                    confidence: 0.73,
                    anomaly_score: Some(0.18),
                    expected_path_summary: "mid-session flow can stay tradeable.".to_string(),
                    rationale: "rotation remains balanced.".to_string(),
                    analogs: vec![PatternAnalog {
                        token_address: "0x06".to_string(),
                        window_end: "2026-04-19T00:00:00Z".to_string(),
                        match_score: 0.79,
                        match_label: "healthy_rotation".to_string(),
                        outcome_class: "healthy_rotation".to_string(),
                        rationale: "balanced flow".to_string(),
                        notable_differences: vec![],
                    }],
                    created_at: Utc::now(),
                },
            ],
        )
        .expect("summary");

        assert_eq!(summary.horizons.len(), 3);
        assert!(summary.summary.contains("1H leans thin momentum"));
        assert!(summary.summary.contains("6H leans healthy rotation"));
        assert!(summary.summary.contains("24H leans pump and fade"));
        assert!(summary.evidence.iter().any(|line| line.contains("0x24")));
    }
}
