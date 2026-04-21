use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use tokio::time::{interval, MissedTickBehavior};
use uuid::Uuid;

use crate::{
    api::{investigation_ops::load_operator_controls, investigation_runs::append_run_event},
    config::Config,
    error::AppError,
};

#[derive(Debug, Clone)]
pub struct AutoInvestigationSettings {
    pub enabled: bool,
    pub interval_secs: u64,
    pub tx_threshold: i64,
    pub cooldown_mins: i64,
    pub max_runs_per_scan: i64,
}

impl AutoInvestigationSettings {
    pub fn from_config(config: &Config) -> Self {
        Self {
            enabled: config.auto_investigation_enabled,
            interval_secs: config.auto_investigation_interval_secs.max(30),
            tx_threshold: config.auto_investigation_tx_threshold.max(1),
            cooldown_mins: config.auto_investigation_cooldown_mins.max(1),
            max_runs_per_scan: config.auto_investigation_max_runs_per_scan.clamp(1, 25),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AutoInvestigationQueuedRun {
    pub run_id: Uuid,
    pub token_address: String,
    pub tx_count: i64,
    pub priority_score: i32,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct AutoInvestigationEscalatedRun {
    pub run_id: Uuid,
    pub token_address: String,
    pub tx_count: i64,
    pub signal_tag: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct AutoInvestigationDowngradedRun {
    pub run_id: Uuid,
    pub token_address: String,
    pub tx_count: i64,
    pub signal_tag: Option<String>,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct AutoInvestigationSkippedToken {
    pub token_address: String,
    pub tx_count: i64,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct AutoInvestigationScanResponse {
    pub enabled: bool,
    pub paused: bool,
    pub tx_threshold: i64,
    pub cooldown_mins: i64,
    pub matched_candidates: usize,
    pub escalated_runs: Vec<AutoInvestigationEscalatedRun>,
    pub downgraded_runs: Vec<AutoInvestigationDowngradedRun>,
    pub created_runs: Vec<AutoInvestigationQueuedRun>,
    pub skipped_tokens: Vec<AutoInvestigationSkippedToken>,
}

#[derive(Debug, FromRow)]
struct AutoInvestigationCandidate {
    contract_address: String,
    tx_count: i64,
    volume_bnb: f64,
    deployed_at: DateTime<Utc>,
    composite_score: Option<i16>,
    wallet_concentration_score: Option<i16>,
    critical_whale_alerts: i64,
    seller_to_new_builder_count: i64,
    related_launch_overlap_count: i64,
}

#[derive(Debug, FromRow)]
struct WatchingRunCandidate {
    id: Uuid,
}

#[derive(Debug, FromRow)]
struct StaleEscalatedRunCandidate {
    id: Uuid,
    contract_address: String,
    status_reason: Option<String>,
    tx_count: i64,
    volume_bnb: f64,
    wallet_concentration_score: Option<i16>,
    critical_whale_alerts: i64,
    seller_to_new_builder_count: i64,
    related_launch_overlap_count: i64,
    source_degraded: bool,
}

pub async fn run_auto_investigation_scan(
    db: &sqlx::PgPool,
    settings: &AutoInvestigationSettings,
) -> Result<AutoInvestigationScanResponse, AppError> {
    if !settings.enabled {
        return Ok(AutoInvestigationScanResponse {
            enabled: false,
            paused: false,
            tx_threshold: settings.tx_threshold,
            cooldown_mins: settings.cooldown_mins,
            matched_candidates: 0,
            escalated_runs: Vec::new(),
            downgraded_runs: Vec::new(),
            created_runs: Vec::new(),
            skipped_tokens: Vec::new(),
        });
    }

    let controls = load_operator_controls(db).await?;
    if controls.auto_investigation_paused {
        return Ok(AutoInvestigationScanResponse {
            enabled: true,
            paused: true,
            tx_threshold: settings.tx_threshold,
            cooldown_mins: settings.cooldown_mins,
            matched_candidates: 0,
            escalated_runs: Vec::new(),
            downgraded_runs: Vec::new(),
            created_runs: Vec::new(),
            skipped_tokens: Vec::new(),
        });
    }

    let base_candidates = sqlx::query_as::<_, AutoInvestigationCandidate>(
        r#"
        SELECT
            t.contract_address,
            (t.buy_count + t.sell_count)::bigint AS tx_count,
            t.volume_bnb::double precision AS volume_bnb,
            t.deployed_at,
            rs.composite_score,
            rs.wallet_concentration_score,
            COALESCE((
                SELECT COUNT(*)::bigint
                FROM whale_alerts wa
                WHERE wa.token_address = t.contract_address
                  AND wa.alert_level = 'critical'
                  AND wa.created_at >= NOW() - INTERVAL '6 hours'
            ), 0) AS critical_whale_alerts,
            0::bigint AS seller_to_new_builder_count,
            0::bigint AS related_launch_overlap_count
        FROM tokens t
        LEFT JOIN risk_scores rs ON rs.token_address = t.contract_address
        WHERE (
                (t.buy_count + t.sell_count) >= $1
                OR COALESCE(rs.wallet_concentration_score, 0) >= 85
                OR COALESCE((
                    SELECT COUNT(*)::bigint
                    FROM whale_alerts wa
                    WHERE wa.token_address = t.contract_address
                      AND wa.alert_level = 'critical'
                      AND wa.created_at >= NOW() - INTERVAL '6 hours'
                ), 0) > 0
              )
          AND t.deployed_at >= NOW() - INTERVAL '24 hours'
        ORDER BY
            GREATEST(
                (t.buy_count + t.sell_count)::bigint,
                COALESCE(rs.wallet_concentration_score, 0)::bigint,
                COALESCE((
                    SELECT COUNT(*)::bigint * 100
                    FROM whale_alerts wa
                    WHERE wa.token_address = t.contract_address
                      AND wa.alert_level = 'critical'
                      AND wa.created_at >= NOW() - INTERVAL '6 hours'
                ), 0)
            ) DESC,
            t.volume_bnb DESC,
            t.deployed_at DESC
        "#,
    )
    .bind(settings.tx_threshold)
    .fetch_all(db)
    .await?;

    let builder_overlap_candidates = sqlx::query_as::<_, AutoInvestigationCandidate>(
        r#"
        SELECT
            t.contract_address,
            (t.buy_count + t.sell_count)::bigint AS tx_count,
            t.volume_bnb::double precision AS volume_bnb,
            t.deployed_at,
            rs.composite_score,
            rs.wallet_concentration_score,
            COALESCE((
                SELECT COUNT(*)::bigint
                FROM whale_alerts wa
                WHERE wa.token_address = t.contract_address
                  AND wa.alert_level = 'critical'
                  AND wa.created_at >= NOW() - INTERVAL '6 hours'
            ), 0) AS critical_whale_alerts,
            COUNT(DISTINCT tx.wallet_address)::bigint AS seller_to_new_builder_count,
            0::bigint AS related_launch_overlap_count
        FROM tokens t
        JOIN token_transactions tx
          ON LOWER(tx.token_address) = LOWER(t.contract_address)
         AND tx.tx_type = 'sell'
        JOIN tokens related_tokens
          ON LOWER(related_tokens.deployer_address) = LOWER(tx.wallet_address)
         AND LOWER(related_tokens.contract_address) <> LOWER(t.contract_address)
         AND related_tokens.deployed_at >= t.deployed_at
        LEFT JOIN risk_scores rs ON rs.token_address = t.contract_address
        WHERE t.deployed_at >= NOW() - INTERVAL '24 hours'
        GROUP BY
            t.contract_address,
            t.buy_count,
            t.sell_count,
            t.volume_bnb,
            t.deployed_at,
            rs.composite_score,
            rs.wallet_concentration_score
        ORDER BY seller_to_new_builder_count DESC, t.volume_bnb DESC, t.deployed_at DESC
        "#,
    )
    .fetch_all(db)
    .await?;

    let linked_launch_overlap_candidates = sqlx::query_as::<_, AutoInvestigationCandidate>(
        r#"
        SELECT
            t.contract_address,
            (t.buy_count + t.sell_count)::bigint AS tx_count,
            t.volume_bnb::double precision AS volume_bnb,
            t.deployed_at,
            rs.composite_score,
            rs.wallet_concentration_score,
            COALESCE((
                SELECT COUNT(*)::bigint
                FROM whale_alerts wa
                WHERE wa.token_address = t.contract_address
                  AND wa.alert_level = 'critical'
                  AND wa.created_at >= NOW() - INTERVAL '6 hours'
            ), 0) AS critical_whale_alerts,
            0::bigint AS seller_to_new_builder_count,
            COUNT(DISTINCT related.token_address)::bigint AS related_launch_overlap_count
        FROM tokens t
        JOIN wallet_clusters current
          ON LOWER(current.token_address) = LOWER(t.contract_address)
        JOIN wallet_clusters related
          ON LOWER(related.wallet_address) = LOWER(current.wallet_address)
         AND LOWER(related.token_address) <> LOWER(current.token_address)
        LEFT JOIN risk_scores rs ON rs.token_address = t.contract_address
        WHERE t.deployed_at >= NOW() - INTERVAL '24 hours'
        GROUP BY
            t.contract_address,
            t.buy_count,
            t.sell_count,
            t.volume_bnb,
            t.deployed_at,
            rs.composite_score,
            rs.wallet_concentration_score
        ORDER BY related_launch_overlap_count DESC, t.volume_bnb DESC, t.deployed_at DESC
        "#,
    )
    .fetch_all(db)
    .await?;

    let mut candidate_map: HashMap<String, AutoInvestigationCandidate> = HashMap::new();
    for candidate in base_candidates
        .into_iter()
        .chain(builder_overlap_candidates.into_iter())
        .chain(linked_launch_overlap_candidates.into_iter())
    {
        candidate_map
            .entry(candidate.contract_address.clone())
            .and_modify(|existing| {
                existing.tx_count = existing.tx_count.max(candidate.tx_count);
                existing.volume_bnb = existing.volume_bnb.max(candidate.volume_bnb);
                existing.deployed_at = existing.deployed_at.max(candidate.deployed_at);
                existing.composite_score = existing.composite_score.or(candidate.composite_score);
                existing.wallet_concentration_score = existing
                    .wallet_concentration_score
                    .or(candidate.wallet_concentration_score);
                existing.critical_whale_alerts = existing
                    .critical_whale_alerts
                    .max(candidate.critical_whale_alerts);
                existing.seller_to_new_builder_count = existing
                    .seller_to_new_builder_count
                    .max(candidate.seller_to_new_builder_count);
                existing.related_launch_overlap_count = existing
                    .related_launch_overlap_count
                    .max(candidate.related_launch_overlap_count);
            })
            .or_insert(candidate);
    }

    let mut candidates: Vec<AutoInvestigationCandidate> = candidate_map.into_values().collect();
    candidates.sort_by(|left, right| {
        let left_rank = std::cmp::max(
            std::cmp::max(
                left.tx_count,
                left.wallet_concentration_score.unwrap_or(0) as i64,
            ),
            std::cmp::max(
                std::cmp::max(
                    left.critical_whale_alerts * 100,
                    left.seller_to_new_builder_count * 100,
                ),
                left.related_launch_overlap_count * 100,
            ),
        );
        let right_rank = std::cmp::max(
            std::cmp::max(
                right.tx_count,
                right.wallet_concentration_score.unwrap_or(0) as i64,
            ),
            std::cmp::max(
                std::cmp::max(
                    right.critical_whale_alerts * 100,
                    right.seller_to_new_builder_count * 100,
                ),
                right.related_launch_overlap_count * 100,
            ),
        );

        right_rank
            .cmp(&left_rank)
            .then_with(|| {
                right
                    .volume_bnb
                    .partial_cmp(&left.volume_bnb)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| right.deployed_at.cmp(&left.deployed_at))
    });

    let mut created_runs = Vec::new();
    let mut escalated_runs = Vec::new();
    let mut downgraded_runs = Vec::new();
    let mut skipped_tokens = Vec::new();
    let candidate_addresses: HashSet<String> = candidates
        .iter()
        .map(|candidate| candidate.contract_address.clone())
        .collect();

    for candidate in &candidates {
        let watching_run = sqlx::query_as::<_, WatchingRunCandidate>(
            r#"
            SELECT id
            FROM investigation_runs
            WHERE token_address = $1
              AND status = 'watching'
            ORDER BY updated_at DESC, created_at DESC
            LIMIT 1
            "#,
        )
        .bind(&candidate.contract_address)
        .fetch_optional(db)
        .await?;

        if let Some(run) = watching_run {
            let signal_tag = auto_escalation_signal_tag(
                candidate.tx_count,
                settings.tx_threshold,
                candidate.wallet_concentration_score,
                candidate.critical_whale_alerts,
                candidate.seller_to_new_builder_count,
                candidate.related_launch_overlap_count,
            );
            let reason = build_auto_escalation_reason_with_signals(
                candidate.tx_count,
                settings.tx_threshold,
                candidate.wallet_concentration_score,
                candidate.critical_whale_alerts,
                candidate.seller_to_new_builder_count,
                candidate.related_launch_overlap_count,
            );
            let evidence_delta = format!(
                "Auto escalation delta: {} transactions, {:.2} BNB volume, wallet concentration score {}, {} critical whale alerts, {} builder-overlap seller wallet(s), and {} linked launch overlap(s) are still live while the run was in watching.",
                candidate.tx_count,
                candidate.volume_bnb,
                candidate
                    .wallet_concentration_score
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "n/a".to_string()),
                candidate.critical_whale_alerts,
                candidate.seller_to_new_builder_count,
                candidate.related_launch_overlap_count
            );

            sqlx::query(
                r#"
                UPDATE investigation_runs
                SET
                    status = 'escalated',
                    current_stage = 'auto_escalation',
                    status_reason = $2,
                    evidence_delta = $3,
                    updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(run.id)
            .bind(&reason)
            .bind(&evidence_delta)
            .execute(db)
            .await?;

            append_run_event(
                db,
                run.id,
                &format!("auto_escalation_triggered_{signal_tag}"),
                auto_escalation_signal_label(signal_tag),
                &format!("{reason} Evidence delta: {evidence_delta}"),
                Some(&reason),
                Some(&evidence_delta),
            )
            .await?;

            escalated_runs.push(AutoInvestigationEscalatedRun {
                run_id: run.id,
                token_address: candidate.contract_address.clone(),
                tx_count: candidate.tx_count,
                signal_tag: signal_tag.to_string(),
                reason,
            });
            continue;
        }

        let recent_auto_run_exists: Option<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT id
            FROM investigation_runs
            WHERE token_address = $1
              AND trigger_type = 'auto'
              AND created_at >= NOW() - make_interval(mins => $2::int)
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(&candidate.contract_address)
        .bind(settings.cooldown_mins as i32)
        .fetch_optional(db)
        .await?;

        if recent_auto_run_exists.is_some() {
            skipped_tokens.push(AutoInvestigationSkippedToken {
                token_address: candidate.contract_address.clone(),
                tx_count: candidate.tx_count,
                reason: format!(
                    "Skipped because an auto run already exists within the last {} minutes.",
                    settings.cooldown_mins
                ),
            });
            continue;
        }

        if created_runs.len() >= settings.max_runs_per_scan as usize {
            skipped_tokens.push(AutoInvestigationSkippedToken {
                token_address: candidate.contract_address.clone(),
                tx_count: candidate.tx_count,
                reason: format!(
                    "Skipped because the scan already created {} new auto runs in this pass.",
                    settings.max_runs_per_scan
                ),
            });
            continue;
        }

        let priority_score = compute_priority_score(
            candidate.tx_count,
            candidate.volume_bnb,
            candidate.composite_score,
        );
        let reason = build_auto_reason(
            candidate.tx_count,
            candidate.volume_bnb,
            candidate.deployed_at,
            settings.tx_threshold,
        );
        let run_id = Uuid::new_v4();

        sqlx::query(
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
                summary
            )
            VALUES ($1, $2, 'auto', 'queued', 'auto_triage', 'system', 'Auto queued', 'system', $3, $4)
            "#,
        )
        .bind(run_id)
        .bind(&candidate.contract_address)
        .bind(priority_score)
        .bind(&reason)
        .execute(db)
        .await?;

        append_run_event(
            db,
            run_id,
            "run_created",
            "Run created",
            &format!(
                "Run entered the system from system as an auto investigation for {}.",
                candidate.contract_address
            ),
            None,
            None,
        )
        .await?;

        let queued_delta = format!(
            "Auto triage priority {} from {} transactions and {:.2} BNB volume.",
            priority_score, candidate.tx_count, candidate.volume_bnb
        );

        append_run_event(
            db,
            run_id,
            "auto_triage_queued",
            "Auto triage queued",
            &reason,
            Some(&reason),
            Some(&queued_delta),
        )
        .await?;

        created_runs.push(AutoInvestigationQueuedRun {
            run_id,
            token_address: candidate.contract_address.clone(),
            tx_count: candidate.tx_count,
            priority_score,
            reason,
        });
    }

    let stale_escalated_runs = sqlx::query_as::<_, StaleEscalatedRunCandidate>(
        r#"
        SELECT
            ir.id,
            t.contract_address,
            ir.status_reason,
            (t.buy_count + t.sell_count)::bigint AS tx_count,
            t.volume_bnb::double precision AS volume_bnb,
            rs.wallet_concentration_score,
            COALESCE((
                SELECT COUNT(*)::bigint
                FROM whale_alerts wa
                WHERE wa.token_address = t.contract_address
                  AND wa.alert_level = 'critical'
                  AND wa.created_at >= NOW() - INTERVAL '6 hours'
            ), 0) AS critical_whale_alerts,
            COALESCE((
                SELECT COUNT(DISTINCT tx.wallet_address)::bigint
                FROM token_transactions tx
                JOIN tokens related_tokens
                  ON LOWER(related_tokens.deployer_address) = LOWER(tx.wallet_address)
                 AND LOWER(related_tokens.contract_address) <> LOWER(t.contract_address)
                 AND related_tokens.deployed_at >= t.deployed_at
                WHERE LOWER(tx.token_address) = LOWER(t.contract_address)
                  AND tx.tx_type = 'sell'
            ), 0) AS seller_to_new_builder_count,
            COALESCE((
                SELECT COUNT(DISTINCT related.token_address)::bigint
                FROM wallet_clusters current
                JOIN wallet_clusters related
                  ON LOWER(related.wallet_address) = LOWER(current.wallet_address)
                 AND LOWER(related.token_address) <> LOWER(current.token_address)
                WHERE LOWER(current.token_address) = LOWER(t.contract_address)
            ), 0) AS related_launch_overlap_count,
            COALESCE((
                SELECT (
                    COALESCE(report.source_status #>> '{dexscreener,status}', '') = 'degraded'
                    OR COALESCE(jsonb_array_length(report.citations), 0) = 0
                )
                FROM deep_research_reports report
                WHERE LOWER(report.token_address) = LOWER(t.contract_address)
                ORDER BY report.updated_at DESC, report.created_at DESC
                LIMIT 1
            ), false) AS source_degraded
        FROM investigation_runs ir
        INNER JOIN tokens t ON t.contract_address = ir.token_address
        LEFT JOIN risk_scores rs ON rs.token_address = t.contract_address
        WHERE ir.status = 'escalated'
          AND ir.current_stage = 'auto_escalation'
          AND t.deployed_at >= NOW() - INTERVAL '24 hours'
        ORDER BY ir.updated_at DESC, ir.created_at DESC
        "#,
    )
    .fetch_all(db)
    .await?;

    for run in stale_escalated_runs {
        if candidate_addresses.contains(&run.contract_address) {
            continue;
        }

        let downgrade_signal_tag = auto_downgrade_signal_tag(
            run.status_reason.as_deref(),
            run.tx_count,
            settings.tx_threshold,
            run.wallet_concentration_score,
            run.critical_whale_alerts,
            run.seller_to_new_builder_count,
            run.related_launch_overlap_count,
            run.source_degraded,
        );
        let reason = build_auto_downgrade_reason(
            downgrade_signal_tag,
            run.tx_count,
            settings.tx_threshold,
            run.wallet_concentration_score,
            run.critical_whale_alerts,
            run.seller_to_new_builder_count,
            run.related_launch_overlap_count,
        );
        let evidence_delta = format!(
            "Auto monitoring delta: live activity cooled to {} transactions with {:.2} BNB volume, wallet concentration score {}, {} critical whale alerts, {} builder-overlap seller wallet(s), and {} linked launch overlap(s), so the run returned to watching.",
            run.tx_count,
            run.volume_bnb,
            run.wallet_concentration_score
                .map(|value| value.to_string())
                .unwrap_or_else(|| "n/a".to_string()),
            run.critical_whale_alerts,
            run.seller_to_new_builder_count,
            run.related_launch_overlap_count
        );

        sqlx::query(
            r#"
            UPDATE investigation_runs
            SET
                status = 'watching',
                current_stage = 'auto_monitoring',
                status_reason = $2,
                evidence_delta = $3,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(run.id)
        .bind(&reason)
        .bind(&evidence_delta)
        .execute(db)
        .await?;

        append_run_event(
            db,
            run.id,
            auto_monitoring_downgrade_event_key(downgrade_signal_tag),
            auto_monitoring_downgrade_label(downgrade_signal_tag),
            &format!("{reason} Evidence delta: {evidence_delta}"),
            Some(&reason),
            Some(&evidence_delta),
        )
        .await?;

        downgraded_runs.push(AutoInvestigationDowngradedRun {
            run_id: run.id,
            token_address: run.contract_address,
            tx_count: run.tx_count,
            signal_tag: downgrade_signal_tag.map(ToString::to_string),
            reason,
        });
    }

    Ok(AutoInvestigationScanResponse {
        enabled: true,
        paused: false,
        tx_threshold: settings.tx_threshold,
        cooldown_mins: settings.cooldown_mins,
        matched_candidates: candidates.len(),
        escalated_runs,
        downgraded_runs,
        created_runs,
        skipped_tokens,
    })
}

pub async fn run_auto_investigation_scheduler(
    db: sqlx::PgPool,
    settings: AutoInvestigationSettings,
) {
    if !settings.enabled {
        tracing::info!("Auto investigation scheduler disabled");
        return;
    }

    if let Err(error) = run_auto_investigation_scan(&db, &settings).await {
        tracing::error!(?error, "Auto investigation startup scan failed");
    }

    let mut ticker = interval(Duration::from_secs(settings.interval_secs));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        ticker.tick().await;
        match run_auto_investigation_scan(&db, &settings).await {
            Ok(result) => {
                if !result.created_runs.is_empty()
                    || !result.escalated_runs.is_empty()
                    || !result.downgraded_runs.is_empty()
                {
                    tracing::info!(
                        created_runs = result.created_runs.len(),
                        escalated_runs = result.escalated_runs.len(),
                        downgraded_runs = result.downgraded_runs.len(),
                        matched_candidates = result.matched_candidates,
                        "Auto investigation scan changed monitoring state"
                    );
                }
            }
            Err(error) => tracing::error!(?error, "Auto investigation scan failed"),
        }
    }
}

fn compute_priority_score(tx_count: i64, volume_bnb: f64, composite_score: Option<i16>) -> i32 {
    let tx_component = (tx_count.min(180) as f64 / 180.0) * 55.0;
    let volume_component = volume_bnb.min(25.0) / 25.0 * 25.0;
    let risk_component = composite_score
        .map(|score| ((100 - score as i32).clamp(0, 100) as f64 / 100.0) * 20.0)
        .unwrap_or(10.0);

    (tx_component + volume_component + risk_component)
        .round()
        .clamp(0.0, 100.0) as i32
}

fn build_auto_reason(
    tx_count: i64,
    volume_bnb: f64,
    deployed_at: DateTime<Utc>,
    tx_threshold: i64,
) -> String {
    format!(
        "Auto-queued because this launch crossed {} transactions with {} total transactions and {:.2} BNB volume since {}.",
        tx_threshold,
        tx_count,
        volume_bnb,
        deployed_at.format("%Y-%m-%d %H:%M UTC")
    )
}

fn auto_escalation_signal_tag(
    tx_count: i64,
    tx_threshold: i64,
    wallet_concentration_score: Option<i16>,
    critical_whale_alerts: i64,
    seller_to_new_builder_count: i64,
    related_launch_overlap_count: i64,
) -> &'static str {
    let tx_signal = tx_count >= tx_threshold;
    let concentration_signal = wallet_concentration_score.is_some_and(|score| score >= 85);
    let whale_signal = critical_whale_alerts > 0;
    let builder_overlap_signal = seller_to_new_builder_count > 0;
    let linked_launch_overlap_signal = related_launch_overlap_count > 0;
    let active_signals = [
        tx_signal,
        concentration_signal,
        whale_signal,
        builder_overlap_signal,
        linked_launch_overlap_signal,
    ]
    .into_iter()
    .filter(|active| *active)
    .count();

    if active_signals > 1 {
        return "multi_signal";
    }
    if whale_signal {
        return "whale_alert";
    }
    if concentration_signal {
        return "wallet_concentration";
    }
    if builder_overlap_signal {
        return "builder_overlap";
    }
    if linked_launch_overlap_signal {
        return "linked_launch_overlap";
    }
    "activity"
}

fn auto_escalation_signal_label(signal_tag: &str) -> &'static str {
    match signal_tag {
        "multi_signal" => "Auto escalation: multi-signal",
        "whale_alert" => "Auto escalation: whale alert",
        "wallet_concentration" => "Auto escalation: wallet concentration",
        "builder_overlap" => "Auto escalation: builder overlap",
        "linked_launch_overlap" => "Auto escalation: linked launch overlap",
        _ => "Auto escalation: activity",
    }
}

fn build_auto_escalation_reason_with_signals(
    tx_count: i64,
    tx_threshold: i64,
    wallet_concentration_score: Option<i16>,
    critical_whale_alerts: i64,
    seller_to_new_builder_count: i64,
    related_launch_overlap_count: i64,
) -> String {
    let mut signals = Vec::new();

    if tx_count >= tx_threshold {
        signals.push(format!(
            "the launch stayed above {} transactions and is now at {} transactions",
            tx_threshold, tx_count
        ));
    }

    if let Some(score) = wallet_concentration_score.filter(|score| *score >= 85) {
        signals.push(format!("wallet concentration is elevated at {}", score));
    }

    if critical_whale_alerts > 0 {
        signals.push(format!(
            "{} critical whale alert{} fired in the last 6 hours",
            critical_whale_alerts,
            if critical_whale_alerts == 1 { "" } else { "s" }
        ));
    }

    if seller_to_new_builder_count > 0 {
        signals.push(format!(
            "builder overlap is live with {} seller wallet{} later appearing as new deployers",
            seller_to_new_builder_count,
            if seller_to_new_builder_count == 1 {
                ""
            } else {
                "s"
            }
        ));
    }

    if related_launch_overlap_count > 0 {
        signals.push(format!(
            "linked launch overlap is live across {} related launch{}",
            related_launch_overlap_count,
            if related_launch_overlap_count == 1 {
                ""
            } else {
                "es"
            }
        ));
    }

    if signals.is_empty() {
        signals.push(format!(
            "the run still matches the automatic monitoring criteria at {} transactions",
            tx_count
        ));
    }

    format!(
        "Auto escalation reason: {} while the run was in watching.",
        signals.join("; ")
    )
}

fn build_auto_downgrade_reason(
    downgrade_signal_tag: Option<&str>,
    tx_count: i64,
    tx_threshold: i64,
    wallet_concentration_score: Option<i16>,
    critical_whale_alerts: i64,
    seller_to_new_builder_count: i64,
    related_launch_overlap_count: i64,
) -> String {
    let cooled_prefix = match downgrade_signal_tag {
        Some("source_degradation") => {
            "source health degraded while other live promotion signals stayed muted; "
        }
        Some("linked_launch_overlap") => {
            "linked launch overlap cooled to zero related launches while other live promotion signals stayed muted; "
        }
        Some("builder_overlap") => {
            "builder overlap cooled to zero seller-to-builder rotations while other live promotion signals stayed muted; "
        }
        Some("whale_alert") => {
            "critical whale alert activity cooled to zero while other live promotion signals stayed muted; "
        }
        Some("wallet_concentration") => {
            "wallet concentration cooled below the promotion threshold while other live promotion signals stayed muted; "
        }
        Some("activity") => {
            "live activity cooled below the promotion threshold; "
        }
        _ => "",
    };

    format!(
        "Auto monitoring downgrade reason: {cooled_prefix}live activity cooled below the escalation threshold with {} transactions against a {} transaction trigger, wallet concentration at {}, {} critical whale alerts, {} builder-overlap seller wallets, and {} linked launch overlaps still active while the run returned to watching.",
        tx_count,
        tx_threshold,
        wallet_concentration_score
            .map(|value| value.to_string())
            .unwrap_or_else(|| "n/a".to_string()),
        critical_whale_alerts,
        seller_to_new_builder_count,
        related_launch_overlap_count
    )
}

fn auto_monitoring_downgrade_event_key(signal_tag: Option<&str>) -> &'static str {
    match signal_tag {
        Some("multi_signal") => "auto_monitoring_downgraded_multi_signal",
        Some("source_degradation") => "auto_monitoring_downgraded_source_degradation",
        Some("builder_overlap") => "auto_monitoring_downgraded_builder_overlap",
        Some("linked_launch_overlap") => "auto_monitoring_downgraded_linked_launch_overlap",
        Some("whale_alert") => "auto_monitoring_downgraded_whale_alert",
        Some("wallet_concentration") => "auto_monitoring_downgraded_wallet_concentration",
        Some("activity") => "auto_monitoring_downgraded_activity",
        _ => "auto_monitoring_downgraded",
    }
}

fn auto_monitoring_downgrade_label(signal_tag: Option<&str>) -> &'static str {
    match signal_tag {
        Some("multi_signal") => "Auto monitoring downgrade: multi-signal cooled",
        Some("source_degradation") => "Auto monitoring downgrade: source degradation",
        Some("builder_overlap") => "Auto monitoring downgrade: builder overlap cooled",
        Some("linked_launch_overlap") => "Auto monitoring downgrade: linked launch overlap cooled",
        Some("whale_alert") => "Auto monitoring downgrade: whale alert cooled",
        Some("wallet_concentration") => "Auto monitoring downgrade: wallet concentration cooled",
        Some("activity") => "Auto monitoring downgrade: activity cooled",
        _ => "Auto monitoring downgrade",
    }
}

fn infer_signal_from_reason(reason: Option<&str>) -> Option<&'static str> {
    let text = reason.unwrap_or_default().to_ascii_lowercase();
    if text.is_empty() {
        return None;
    }

    let has_activity = text.contains("transactions");
    let has_builder_overlap = text.contains("builder overlap");
    let has_linked_launch_overlap = text.contains("linked launch overlap");
    let has_concentration = text.contains("wallet concentration");
    let has_whale = text.contains("critical whale alert");
    let has_source_degradation =
        text.contains("source health") || text.contains("source verification");
    let active_count = [
        has_activity,
        has_builder_overlap,
        has_linked_launch_overlap,
        has_concentration,
        has_whale,
        has_source_degradation,
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
    if has_source_degradation {
        return Some("source_degradation");
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

fn auto_downgrade_signal_tag(
    previous_status_reason: Option<&str>,
    tx_count: i64,
    tx_threshold: i64,
    wallet_concentration_score: Option<i16>,
    critical_whale_alerts: i64,
    seller_to_new_builder_count: i64,
    related_launch_overlap_count: i64,
    source_degraded: bool,
) -> Option<&'static str> {
    match infer_signal_from_reason(previous_status_reason) {
        Some("source_degradation") if source_degraded => Some("source_degradation"),
        Some("linked_launch_overlap") if related_launch_overlap_count == 0 => {
            Some("linked_launch_overlap")
        }
        Some("builder_overlap") if seller_to_new_builder_count == 0 => Some("builder_overlap"),
        Some("whale_alert") if critical_whale_alerts == 0 => Some("whale_alert"),
        Some("wallet_concentration")
            if !wallet_concentration_score.is_some_and(|score| score >= 85) =>
        {
            Some("wallet_concentration")
        }
        Some("activity") if tx_count < tx_threshold => Some("activity"),
        _ if source_degraded => Some("source_degradation"),
        Some("multi_signal") => Some("multi_signal"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_score_rewards_higher_activity() {
        let lower = compute_priority_score(100, 1.2, Some(60));
        let higher = compute_priority_score(180, 12.5, Some(45));
        assert!(higher > lower);
        assert!(higher <= 100);
    }

    #[test]
    fn auto_escalation_reason_mentions_threshold_and_live_activity() {
        let reason = build_auto_escalation_reason_with_signals(148, 100, None, 0, 0, 0);
        assert!(reason.contains("100"));
        assert!(reason.contains("148"));
        assert!(reason.contains("watching"));
        assert_eq!(
            auto_escalation_signal_tag(148, 100, None, 0, 0, 0),
            "activity"
        );
    }

    #[test]
    fn auto_escalation_reason_includes_concentration_and_whales_when_present() {
        let reason = build_auto_escalation_reason_with_signals(92, 100, Some(91), 2, 0, 0);
        assert!(reason.contains("wallet concentration is elevated at 91"));
        assert!(reason.contains("2 critical whale alerts"));
        assert!(reason.contains("watching"));
        assert_eq!(
            auto_escalation_signal_tag(92, 100, Some(91), 2, 0, 0),
            "multi_signal"
        );
    }

    #[test]
    fn auto_escalation_signal_tag_prefers_single_signal_variants() {
        assert_eq!(
            auto_escalation_signal_tag(80, 100, Some(90), 0, 0, 0),
            "wallet_concentration"
        );
        assert_eq!(
            auto_escalation_signal_tag(60, 100, None, 1, 0, 0),
            "whale_alert"
        );
        assert_eq!(
            auto_escalation_signal_tag(60, 100, None, 0, 1, 0),
            "builder_overlap"
        );
        assert_eq!(
            auto_escalation_signal_tag(60, 100, None, 0, 0, 2),
            "linked_launch_overlap"
        );
        assert_eq!(
            auto_escalation_signal_label("wallet_concentration"),
            "Auto escalation: wallet concentration"
        );
    }

    #[test]
    fn auto_escalation_reason_mentions_builder_overlap_when_present() {
        let reason = build_auto_escalation_reason_with_signals(24, 100, None, 0, 2, 0);
        assert!(reason.contains("builder overlap is live"));
        assert!(reason.contains("2 seller wallets"));
        assert_eq!(
            auto_escalation_signal_tag(24, 100, None, 0, 2, 0),
            "builder_overlap"
        );
    }

    #[test]
    fn auto_escalation_reason_mentions_linked_launch_overlap_when_present() {
        let reason = build_auto_escalation_reason_with_signals(24, 100, None, 0, 0, 3);
        assert!(reason.contains("linked launch overlap is live"));
        assert!(reason.contains("3 related launches"));
        assert_eq!(
            auto_escalation_signal_tag(24, 100, None, 0, 0, 3),
            "linked_launch_overlap"
        );
    }

    #[test]
    fn auto_reason_mentions_threshold_and_volume() {
        let deployed_at = DateTime::parse_from_rfc3339("2026-04-20T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let reason = build_auto_reason(124, 17.53, deployed_at, 100);
        assert!(reason.contains("crossed 100 transactions"));
        assert!(reason.contains("124 total transactions"));
        assert!(reason.contains("17.53 BNB"));
    }

    #[test]
    fn auto_downgrade_reason_mentions_cooled_signals() {
        let reason =
            build_auto_downgrade_reason(Some("linked_launch_overlap"), 24, 100, Some(42), 0, 1, 2);
        assert!(reason.contains("24 transactions"));
        assert!(reason.contains("100 transaction trigger"));
        assert!(reason.contains("linked launch overlap cooled"));
        assert!(reason.contains("wallet concentration at 42"));
        assert!(reason.contains("0 critical whale alerts"));
        assert!(reason.contains("1 builder-overlap seller wallets"));
        assert!(reason.contains("2 linked launch overlaps"));
        assert!(reason.contains("returned to watching"));
    }

    #[test]
    fn auto_downgrade_signal_tag_prefers_previous_overlap_signal_when_it_cools_to_zero() {
        let tag = auto_downgrade_signal_tag(
            Some(
                "Auto escalation reason: linked launch overlap is live across 2 related launches while the run was in watching.",
            ),
            24,
            100,
            Some(41),
            0,
            0,
            0,
            false,
        );
        assert_eq!(tag, Some("linked_launch_overlap"));
        assert_eq!(
            auto_monitoring_downgrade_event_key(tag),
            "auto_monitoring_downgraded_linked_launch_overlap"
        );
    }

    #[test]
    fn auto_downgrade_signal_tag_can_mark_source_degradation() {
        let tag = auto_downgrade_signal_tag(
            Some("Auto escalation reason: source verification stayed healthy while the run was in watching."),
            24,
            100,
            Some(41),
            0,
            0,
            0,
            true,
        );
        assert_eq!(tag, Some("source_degradation"));
        assert_eq!(
            auto_monitoring_downgrade_event_key(tag),
            "auto_monitoring_downgraded_source_degradation"
        );
        assert_eq!(
            auto_monitoring_downgrade_label(tag),
            "Auto monitoring downgrade: source degradation"
        );
    }
}
