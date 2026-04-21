use crate::{
    config::MlRolloutMode,
    phase4::telegram::{send_telegram_message, TelegramConfig},
};
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use serde::Serialize;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::time::{interval, Duration as TokioDuration};

#[derive(Debug, Clone, Serialize)]
pub struct AlphaEntry {
    pub rank: i16,
    pub token_address: String,
    pub alpha_score: f64,
    pub rationale: String,
}

#[derive(Clone)]
pub struct AlphaScheduler {
    pub db: PgPool,
    pub refresh_secs: u64,
    pub top_k: i64,
    pub telegram_cfg: TelegramConfig,
    pub ml_rollout_mode: MlRolloutMode,
    pub ml_model_version: String,
    pub ml_min_confidence: f64,
    pub http: Client,
}

impl AlphaScheduler {
    pub fn new(
        db: PgPool,
        refresh_secs: u64,
        top_k: i64,
        telegram_cfg: TelegramConfig,
        ml_rollout_mode: MlRolloutMode,
        ml_model_version: String,
        ml_min_confidence: f64,
    ) -> Self {
        Self {
            db,
            refresh_secs: refresh_secs.max(60),
            top_k: top_k.clamp(1, 50),
            telegram_cfg,
            ml_rollout_mode,
            ml_model_version,
            ml_min_confidence: ml_min_confidence.clamp(0.0, 1.0),
            http: Client::new(),
        }
    }

    pub async fn run(self: Arc<Self>) {
        // Run once on startup so alpha endpoint has immediate data.
        if let Err(e) = self.refresh_once().await {
            tracing::warn!(error = %e, "Initial alpha refresh failed");
        }

        let mut ticker = interval(TokioDuration::from_secs(self.refresh_secs));
        loop {
            ticker.tick().await;
            if let Err(e) = self.refresh_once().await {
                tracing::warn!(error = %e, "Alpha refresh failed");
            }
        }
    }

    pub async fn refresh_once(&self) -> Result<()> {
        let window_end = Utc::now();
        let window_start = window_end - Duration::hours(1);
        let rows: Vec<(String, f64, i32, i32, f64, f64)> = sqlx::query_as(
            r#"
            SELECT
                t.contract_address,
                (
                    ((100 - COALESCE(rs.composite_score, 50))::double precision * 0.50) +
                    (LEAST(t.buy_count, 300)::double precision * 0.30) +
                    (LEAST(t.volume_bnb::double precision, 50)::double precision * 0.20)
                ) AS alpha_score,
                t.buy_count,
                t.sell_count,
                t.volume_bnb::double precision,
                COALESCE(rs.composite_score, 50)::double precision AS risk_score
            FROM tokens t
            LEFT JOIN risk_scores rs ON rs.token_address = t.contract_address
            WHERE t.updated_at >= $1
              AND t.buy_count >= 3
              AND (t.buy_count + t.sell_count) >= 5
              AND t.volume_bnb > 0
            ORDER BY alpha_score DESC
            LIMIT $2
            "#,
        )
        .bind(window_start)
        .bind(self.top_k)
        .fetch_all(&self.db)
        .await?;

        sqlx::query("DELETE FROM alpha_rankings WHERE window_end = $1")
            .bind(window_end)
            .execute(&self.db)
            .await?;

        let mut ranked: Vec<AlphaEntry> = Vec::with_capacity(rows.len());
        for (idx, (token_address, alpha_score, buy_count, sell_count, volume_bnb, risk_score)) in
            rows.into_iter().enumerate()
        {
            let rank = (idx + 1) as i16;
            let risk_component = (100.0 - risk_score) * 0.50;
            let buy_component = (buy_count.min(300) as f64) * 0.30;
            let volume_component = volume_bnb.min(50.0) * 0.20;
            let rationale = format!(
                "Risk component: {:.2}, Buy momentum: {:.2}, Volume support: {:.2} | Buys: {buy_count}, Sells: {sell_count}, Volume: {:.3} BNB, Risk: {:.0}/100",
                risk_component,
                buy_component,
                volume_component,
                volume_bnb,
                risk_score
            );
            sqlx::query(
                r#"
                INSERT INTO alpha_rankings (window_start, window_end, rank, token_address, alpha_score, rationale)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(window_start)
            .bind(window_end)
            .bind(rank)
            .bind(&token_address)
            .bind(alpha_score)
            .bind(&rationale)
            .execute(&self.db)
            .await?;

            let legacy_score = clamp_score(alpha_score);
            let legacy_confidence = confidence_from_score(legacy_score).max(self.ml_min_confidence);
            upsert_prediction(
                &self.db,
                window_end,
                &token_address,
                "legacy",
                &self.ml_model_version,
                self.ml_rollout_mode,
                legacy_score,
                legacy_confidence,
            )
            .await?;

            if matches!(
                self.ml_rollout_mode,
                MlRolloutMode::Shadow | MlRolloutMode::Hybrid
            ) {
                let shadow_score =
                    shadow_ml_stub_score(buy_count, sell_count, volume_bnb, risk_score);
                let shadow_confidence =
                    confidence_from_score(shadow_score).max(self.ml_min_confidence);
                upsert_prediction(
                    &self.db,
                    window_end,
                    &token_address,
                    "shadow_ml_stub",
                    &self.ml_model_version,
                    self.ml_rollout_mode,
                    shadow_score,
                    shadow_confidence,
                )
                .await?;
            }

            ranked.push(AlphaEntry {
                rank,
                token_address,
                alpha_score,
                rationale,
            });
        }

        if !ranked.is_empty() {
            let digest = format_alpha_digest(window_end, &ranked);
            send_telegram_message(
                &self.http,
                &self.db,
                &self.telegram_cfg,
                "alpha_hourly_digest",
                digest,
            )
            .await?;
        }

        Ok(())
    }
}

async fn upsert_prediction(
    db: &PgPool,
    window_end: DateTime<Utc>,
    token_address: &str,
    score_source: &str,
    model_version: &str,
    rollout_mode: MlRolloutMode,
    score: f64,
    confidence: f64,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO ml_alpha_predictions
            (window_end, token_address, score_source, model_version, rollout_mode, score, confidence)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (window_end, token_address, score_source) DO UPDATE
            SET model_version = EXCLUDED.model_version,
                rollout_mode = EXCLUDED.rollout_mode,
                score = EXCLUDED.score,
                confidence = EXCLUDED.confidence
        "#,
    )
    .bind(window_end)
    .bind(token_address)
    .bind(score_source)
    .bind(model_version)
    .bind(match rollout_mode {
        MlRolloutMode::Legacy => "legacy",
        MlRolloutMode::Shadow => "shadow",
        MlRolloutMode::Ml => "ml",
        MlRolloutMode::Hybrid => "hybrid",
    })
    .bind(score)
    .bind(confidence)
    .execute(db)
    .await?;
    Ok(())
}

fn clamp_score(score: f64) -> f64 {
    score.clamp(0.0, 100.0)
}

fn confidence_from_score(score: f64) -> f64 {
    (0.5 + ((score - 50.0).abs() / 100.0)).clamp(0.0, 1.0)
}

fn shadow_ml_stub_score(buy_count: i32, sell_count: i32, volume_bnb: f64, risk_score: f64) -> f64 {
    let total_flow = (buy_count + sell_count).max(0) as f64;
    let buy_share = if total_flow > 0.0 {
        (buy_count.max(0) as f64 / total_flow) * 100.0
    } else {
        50.0
    };
    let volume_strength = (volume_bnb.clamp(0.0, 50.0) / 50.0) * 100.0;
    let risk_safety = 100.0 - risk_score.clamp(0.0, 100.0);
    clamp_score((risk_safety * 0.45) + (buy_share * 0.30) + (volume_strength * 0.25))
}

fn format_alpha_digest(window_end: DateTime<Utc>, entries: &[AlphaEntry]) -> String {
    let mut lines = vec![format!(
        "*MIA Hourly Alpha — {} UTC*",
        window_end.format("%Y-%m-%d %H:%M")
    )];
    for e in entries {
        lines.push(format!(
            "{}. `{}` — score {:.2}",
            e.rank, e.token_address, e.alpha_score
        ));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{format_alpha_digest, AlphaEntry};
    use chrono::TimeZone;

    #[test]
    fn digest_contains_rank_and_token() {
        let ts = chrono::Utc.with_ymd_and_hms(2026, 1, 5, 10, 0, 0).unwrap();
        let rows = vec![AlphaEntry {
            rank: 1,
            token_address: "0xabc".to_string(),
            alpha_score: 95.3,
            rationale: "test".to_string(),
        }];
        let text = format_alpha_digest(ts, &rows);
        assert!(text.contains("MIA Hourly Alpha"));
        assert!(text.contains("1. `0xabc`"));
    }
}
