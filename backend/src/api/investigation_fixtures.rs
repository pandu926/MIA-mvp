use axum::{extract::State, Json};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{api::investigation_runs::append_run_event, error::AppError, AppState};

const FIXTURE_WALLET_CONCENTRATION_TOKEN: &str = "0x0000000000000000000000000000000000000c01";
const FIXTURE_WHALE_ALERT_TOKEN: &str = "0x0000000000000000000000000000000000000c02";
const FIXTURE_STALE_ACTIVITY_TOKEN: &str = "0x0000000000000000000000000000000000000c03";
const FIXTURE_BUILDER_OVERLAP_TOKEN: &str = "0x0000000000000000000000000000000000000c04";
const FIXTURE_BUILDER_OVERLAP_RELATED_TOKEN: &str = "0x0000000000000000000000000000000000000c05";
const FIXTURE_BUILDER_OVERLAP_SELLER: &str = "0x0000000000000000000000000000000000000d04";
const FIXTURE_LINKED_OVERLAP_TOKEN: &str = "0x0000000000000000000000000000000000000c06";
const FIXTURE_LINKED_OVERLAP_RELATED_TOKEN: &str = "0x0000000000000000000000000000000000000c07";
const FIXTURE_LINKED_OVERLAP_WALLET: &str = "0x0000000000000000000000000000000000000d05";
const FIXTURE_SOURCE_DEGRADATION_TOKEN: &str = "0x0000000000000000000000000000000000000c08";
const FIXTURE_FAILED_RUN_TOKEN: &str = "0x0000000000000000000000000000000000000c09";
const FIXTURE_STALE_RUNNING_TOKEN: &str = "0x0000000000000000000000000000000000000c0a";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InvestigationFixtureRequest {
    pub signal: String,
}

#[derive(Debug, Serialize)]
pub struct InvestigationFixtureResponse {
    pub token_address: String,
    pub run_id: Uuid,
    pub signal_tag: String,
    pub tx_count: i64,
}

pub async fn post_non_transaction_escalation_fixture(
    State(state): State<AppState>,
    Json(payload): Json<InvestigationFixtureRequest>,
) -> Result<Json<InvestigationFixtureResponse>, AppError> {
    if !state.config.investigation_fixture_api_enabled {
        return Err(AppError::FeatureDisabled(
            "Investigation fixture API is disabled in this environment.".to_string(),
        ));
    }

    let signal = match payload.signal.trim().to_ascii_lowercase().as_str() {
        "wallet_concentration" => FixtureSignal::WalletConcentration,
        "whale_alert" => FixtureSignal::WhaleAlert,
        "builder_overlap" => FixtureSignal::BuilderOverlap,
        "linked_launch_overlap" => FixtureSignal::LinkedLaunchOverlap,
        other => {
            return Err(AppError::BadRequest(format!(
                "Unsupported fixture signal `{other}`. Use wallet_concentration, whale_alert, builder_overlap, or linked_launch_overlap."
            )))
        }
    };

    let seeded = seed_non_transaction_escalation_fixture(&state.db, signal).await?;
    Ok(Json(seeded))
}

pub async fn post_monitoring_downgrade_fixture(
    State(state): State<AppState>,
    payload: Option<Json<InvestigationFixtureRequest>>,
) -> Result<Json<InvestigationFixtureResponse>, AppError> {
    if !state.config.investigation_fixture_api_enabled {
        return Err(AppError::FeatureDisabled(
            "Investigation fixture API is disabled in this environment.".to_string(),
        ));
    }

    let signal = match payload
        .as_ref()
        .map(|json| json.signal.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("linked_launch_overlap") => FixtureSignal::LinkedLaunchOverlap,
        Some("builder_overlap") => FixtureSignal::BuilderOverlap,
        Some("source_degradation") => FixtureSignal::SourceDegradation,
        Some("activity") | None => FixtureSignal::StaleActivity,
        Some(other) => {
            return Err(AppError::BadRequest(format!(
                "Unsupported monitoring downgrade fixture `{other}`. Use activity, builder_overlap, linked_launch_overlap, or source_degradation."
            )))
        }
    };

    let seeded = seed_monitoring_downgrade_fixture(&state.db, signal).await?;
    Ok(Json(seeded))
}

pub async fn post_failed_run_fixture(
    State(state): State<AppState>,
) -> Result<Json<InvestigationFixtureResponse>, AppError> {
    if !state.config.investigation_fixture_api_enabled {
        return Err(AppError::FeatureDisabled(
            "Investigation fixture API is disabled in this environment.".to_string(),
        ));
    }

    let seeded = seed_failed_run_fixture(&state.db).await?;
    Ok(Json(seeded))
}

pub async fn post_stale_running_fixture(
    State(state): State<AppState>,
) -> Result<Json<InvestigationFixtureResponse>, AppError> {
    if !state.config.investigation_fixture_api_enabled {
        return Err(AppError::FeatureDisabled(
            "Investigation fixture API is disabled in this environment.".to_string(),
        ));
    }

    let seeded = seed_stale_running_fixture(&state.db).await?;
    Ok(Json(seeded))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FixtureSignal {
    WalletConcentration,
    WhaleAlert,
    BuilderOverlap,
    LinkedLaunchOverlap,
    SourceDegradation,
    StaleActivity,
}

impl FixtureSignal {
    fn token_address(self) -> &'static str {
        match self {
            Self::WalletConcentration => FIXTURE_WALLET_CONCENTRATION_TOKEN,
            Self::WhaleAlert => FIXTURE_WHALE_ALERT_TOKEN,
            Self::BuilderOverlap => FIXTURE_BUILDER_OVERLAP_TOKEN,
            Self::LinkedLaunchOverlap => FIXTURE_LINKED_OVERLAP_TOKEN,
            Self::SourceDegradation => FIXTURE_SOURCE_DEGRADATION_TOKEN,
            Self::StaleActivity => FIXTURE_STALE_ACTIVITY_TOKEN,
        }
    }

    fn signal_tag(self) -> &'static str {
        match self {
            Self::WalletConcentration => "wallet_concentration",
            Self::WhaleAlert => "whale_alert",
            Self::BuilderOverlap => "builder_overlap",
            Self::LinkedLaunchOverlap => "linked_launch_overlap",
            Self::SourceDegradation => "source_degradation",
            Self::StaleActivity => "activity",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::WalletConcentration => "Fixture Concentration Token",
            Self::WhaleAlert => "Fixture Whale Token",
            Self::BuilderOverlap => "Fixture Builder Token",
            Self::LinkedLaunchOverlap => "Fixture Linked Launch Token",
            Self::SourceDegradation => "Fixture Source Degradation Token",
            Self::StaleActivity => "Fixture Cooling Token",
        }
    }

    fn symbol(self) -> &'static str {
        match self {
            Self::WalletConcentration => "FIXCON",
            Self::WhaleAlert => "FIXWHALE",
            Self::BuilderOverlap => "FIXBLDR",
            Self::LinkedLaunchOverlap => "FIXLINK",
            Self::SourceDegradation => "FIXSRC",
            Self::StaleActivity => "FIXCOOL",
        }
    }
}

async fn seed_non_transaction_escalation_fixture(
    db: &sqlx::PgPool,
    signal: FixtureSignal,
) -> Result<InvestigationFixtureResponse, AppError> {
    let token_address = signal.token_address().to_string();
    let run_id = Uuid::new_v4();
    let now = Utc::now();
    let deployed_at = now - Duration::minutes(20);
    let tx_count = 24_i64;
    let buy_count = 16_i32;
    let sell_count = 8_i32;
    let volume_bnb = 1.35_f64;
    let block_number = 9_000_000_i64;
    let token_tx_hash = fixture_hash(&format!("{}-token", signal.signal_tag()));
    let whale_tx_hash = fixture_hash(&format!("{}-whale", signal.signal_tag()));
    let builder_related_tx_hash = fixture_hash(&format!("{}-related-token", signal.signal_tag()));

    sqlx::query("DELETE FROM investigation_runs WHERE token_address = $1")
        .bind(&token_address)
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM whale_alerts WHERE token_address = $1")
        .bind(&token_address)
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM deep_research_reports WHERE token_address = $1")
        .bind(&token_address)
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM token_transactions WHERE token_address = $1")
        .bind(&token_address)
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM wallet_clusters WHERE token_address = $1")
        .bind(&token_address)
        .execute(db)
        .await?;
    if signal == FixtureSignal::BuilderOverlap {
        sqlx::query("DELETE FROM tokens WHERE contract_address = $1")
            .bind(FIXTURE_BUILDER_OVERLAP_RELATED_TOKEN)
            .execute(db)
            .await?;
    }
    if signal == FixtureSignal::LinkedLaunchOverlap {
        sqlx::query("DELETE FROM tokens WHERE contract_address = $1")
            .bind(FIXTURE_LINKED_OVERLAP_RELATED_TOKEN)
            .execute(db)
            .await?;
        sqlx::query("DELETE FROM wallet_clusters WHERE token_address = $1")
            .bind(FIXTURE_LINKED_OVERLAP_RELATED_TOKEN)
            .execute(db)
            .await?;
    }

    sqlx::query(
        r#"
        INSERT INTO tokens (
            contract_address,
            deployer_address,
            name,
            symbol,
            deployed_at,
            block_number,
            tx_hash,
            initial_liquidity_bnb,
            holder_count,
            buy_count,
            sell_count,
            volume_bnb,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW())
        ON CONFLICT (contract_address) DO UPDATE
        SET
            deployer_address = EXCLUDED.deployer_address,
            name = EXCLUDED.name,
            symbol = EXCLUDED.symbol,
            deployed_at = EXCLUDED.deployed_at,
            block_number = EXCLUDED.block_number,
            tx_hash = EXCLUDED.tx_hash,
            initial_liquidity_bnb = EXCLUDED.initial_liquidity_bnb,
            holder_count = EXCLUDED.holder_count,
            buy_count = EXCLUDED.buy_count,
            sell_count = EXCLUDED.sell_count,
            volume_bnb = EXCLUDED.volume_bnb,
            updated_at = NOW()
        "#,
    )
    .bind(&token_address)
    .bind("0x0000000000000000000000000000000000000d01")
    .bind(signal.display_name())
    .bind(signal.symbol())
    .bind(deployed_at)
    .bind(block_number)
    .bind(token_tx_hash)
    .bind(0.75_f64)
    .bind(11_i32)
    .bind(buy_count)
    .bind(sell_count)
    .bind(volume_bnb)
    .execute(db)
    .await?;

    if signal == FixtureSignal::BuilderOverlap {
        sqlx::query(
            r#"
            INSERT INTO tokens (
                contract_address,
                deployer_address,
                name,
                symbol,
                deployed_at,
                block_number,
                tx_hash,
                initial_liquidity_bnb,
                holder_count,
                buy_count,
                sell_count,
                volume_bnb,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW())
            ON CONFLICT (contract_address) DO UPDATE
            SET
                deployer_address = EXCLUDED.deployer_address,
                name = EXCLUDED.name,
                symbol = EXCLUDED.symbol,
                deployed_at = EXCLUDED.deployed_at,
                block_number = EXCLUDED.block_number,
                tx_hash = EXCLUDED.tx_hash,
                initial_liquidity_bnb = EXCLUDED.initial_liquidity_bnb,
                holder_count = EXCLUDED.holder_count,
                buy_count = EXCLUDED.buy_count,
                sell_count = EXCLUDED.sell_count,
                volume_bnb = EXCLUDED.volume_bnb,
                updated_at = NOW()
            "#,
        )
        .bind(FIXTURE_BUILDER_OVERLAP_RELATED_TOKEN)
        .bind(FIXTURE_BUILDER_OVERLAP_SELLER)
        .bind("Fixture Related Builder Token")
        .bind("FIXREL")
        .bind(now - Duration::minutes(5))
        .bind(block_number + 1)
        .bind(builder_related_tx_hash)
        .bind(0.22_f64)
        .bind(3_i32)
        .bind(1_i32)
        .bind(0_i32)
        .bind(0.22_f64)
        .execute(db)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO token_transactions (
                token_address,
                wallet_address,
                tx_type,
                amount_bnb,
                tx_hash,
                block_number,
                created_at
            )
            VALUES ($1, $2, 'sell', $3, $4, $5, NOW())
            ON CONFLICT (tx_hash) DO UPDATE
            SET
                token_address = EXCLUDED.token_address,
                wallet_address = EXCLUDED.wallet_address,
                tx_type = EXCLUDED.tx_type,
                amount_bnb = EXCLUDED.amount_bnb,
                block_number = EXCLUDED.block_number,
                created_at = EXCLUDED.created_at
            "#,
        )
        .bind(&token_address)
        .bind(FIXTURE_BUILDER_OVERLAP_SELLER)
        .bind(0.0_f64)
        .bind(fixture_hash("builder-overlap-sell"))
        .bind(block_number + 2)
        .execute(db)
        .await?;
    }

    if signal == FixtureSignal::LinkedLaunchOverlap {
        sqlx::query(
            r#"
            INSERT INTO tokens (
                contract_address,
                deployer_address,
                name,
                symbol,
                deployed_at,
                block_number,
                tx_hash,
                initial_liquidity_bnb,
                holder_count,
                buy_count,
                sell_count,
                volume_bnb,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW())
            ON CONFLICT (contract_address) DO UPDATE
            SET
                deployer_address = EXCLUDED.deployer_address,
                name = EXCLUDED.name,
                symbol = EXCLUDED.symbol,
                deployed_at = EXCLUDED.deployed_at,
                block_number = EXCLUDED.block_number,
                tx_hash = EXCLUDED.tx_hash,
                initial_liquidity_bnb = EXCLUDED.initial_liquidity_bnb,
                holder_count = EXCLUDED.holder_count,
                buy_count = EXCLUDED.buy_count,
                sell_count = EXCLUDED.sell_count,
                volume_bnb = EXCLUDED.volume_bnb,
                updated_at = NOW()
            "#,
        )
        .bind(FIXTURE_LINKED_OVERLAP_RELATED_TOKEN)
        .bind("0x0000000000000000000000000000000000000d06")
        .bind("Fixture Linked Related Token")
        .bind("FIXLINKR")
        .bind(now - Duration::minutes(4))
        .bind(block_number + 3)
        .bind(fixture_hash("linked-overlap-related-token"))
        .bind(0.19_f64)
        .bind(4_i32)
        .bind(2_i32)
        .bind(0_i32)
        .bind(0.19_f64)
        .execute(db)
        .await?;

        let shared_cluster_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO wallet_clusters (token_address, wallet_address, cluster_id, confidence)
            VALUES
                ($1, $2, $3, 'potential'),
                ($4, $2, $3, 'potential')
            ON CONFLICT (token_address, wallet_address) DO UPDATE
            SET cluster_id = EXCLUDED.cluster_id,
                confidence = EXCLUDED.confidence
            "#,
        )
        .bind(&token_address)
        .bind(FIXTURE_LINKED_OVERLAP_WALLET)
        .bind(shared_cluster_id)
        .bind(FIXTURE_LINKED_OVERLAP_RELATED_TOKEN)
        .execute(db)
        .await?;
    }

    let wallet_concentration = match signal {
        FixtureSignal::WalletConcentration => 97_i16,
        FixtureSignal::WhaleAlert => 42_i16,
        FixtureSignal::BuilderOverlap => 41_i16,
        FixtureSignal::LinkedLaunchOverlap => 41_i16,
        FixtureSignal::SourceDegradation => 41_i16,
        FixtureSignal::StaleActivity => 41_i16,
    };

    sqlx::query(
        r#"
        INSERT INTO risk_scores (
            token_address,
            composite_score,
            deployer_history_score,
            liquidity_lock_score,
            wallet_concentration_score,
            buy_sell_velocity_score,
            contract_audit_score,
            social_authenticity_score,
            volume_consistency_score,
            computed_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
        ON CONFLICT (token_address) DO UPDATE
        SET
            composite_score = EXCLUDED.composite_score,
            deployer_history_score = EXCLUDED.deployer_history_score,
            liquidity_lock_score = EXCLUDED.liquidity_lock_score,
            wallet_concentration_score = EXCLUDED.wallet_concentration_score,
            buy_sell_velocity_score = EXCLUDED.buy_sell_velocity_score,
            contract_audit_score = EXCLUDED.contract_audit_score,
            social_authenticity_score = EXCLUDED.social_authenticity_score,
            volume_consistency_score = EXCLUDED.volume_consistency_score,
            computed_at = NOW()
        "#,
    )
    .bind(&token_address)
    .bind(58_i16)
    .bind(40_i16)
    .bind(72_i16)
    .bind(wallet_concentration)
    .bind(46_i16)
    .bind(61_i16)
    .bind(38_i16)
    .bind(49_i16)
    .execute(db)
    .await?;

    if signal == FixtureSignal::WhaleAlert {
        sqlx::query(
            r#"
            INSERT INTO whale_alerts (
                token_address,
                wallet_address,
                tx_hash,
                amount_bnb,
                threshold_bnb,
                alert_level,
                created_at
            )
            VALUES ($1, $2, $3, $4, $5, 'critical', NOW())
            "#,
        )
        .bind(&token_address)
        .bind("0x0000000000000000000000000000000000000d02")
        .bind(whale_tx_hash)
        .bind(3.4_f64)
        .bind(0.5_f64)
        .execute(db)
        .await?;
    }

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
            summary,
            created_at,
            updated_at,
            started_at,
            completed_at
        )
        VALUES (
            $1,
            $2,
            'manual',
            'watching',
            'investigation',
            'mia',
            'Fixture monitoring',
            'fixture',
            $3,
            $4,
            NOW(),
            NOW(),
            NOW(),
            NOW()
        )
        "#,
    )
    .bind(run_id)
    .bind(&token_address)
    .bind(63_i32)
    .bind(format!(
        "Fixture seeded for {} non-transaction escalation proof.",
        signal.signal_tag()
    ))
    .execute(db)
    .await?;

    append_run_event(
        db,
        run_id,
        "run_created",
        "Run created",
        "Fixture run created for non-transaction escalation proof.",
        None,
        None,
    )
    .await?;

    append_run_event(
        db,
        run_id,
        "status_transition_fixture_watch",
        "Fixture watching state",
        &format!(
            "Fixture seeded in watching so auto scan can promote it via {}.",
            signal.signal_tag()
        ),
        Some("Fixture watching state ready for auto escalation."),
        Some(match signal {
            FixtureSignal::WalletConcentration => {
                "Wallet concentration was seeded above threshold while transaction count stayed below auto-run threshold."
            }
            FixtureSignal::WhaleAlert => {
                "Critical whale activity was seeded while transaction count stayed below auto-run threshold."
            }
            FixtureSignal::BuilderOverlap => {
                "Builder overlap was seeded by making a seller wallet later appear as a deployer on another launch while transaction count stayed below auto-run threshold."
            }
            FixtureSignal::LinkedLaunchOverlap => {
                "Linked launch overlap was seeded by placing the same clustered wallet across two launches while transaction count stayed below auto-run threshold."
            }
            FixtureSignal::SourceDegradation => {
                "Source degradation was seeded by attaching a degraded report snapshot while transaction count stayed below auto-run threshold."
            }
            FixtureSignal::StaleActivity => {
                "Live activity was intentionally cooled below every escalation threshold."
            }
        }),
    )
    .await?;

    Ok(InvestigationFixtureResponse {
        token_address,
        run_id,
        signal_tag: signal.signal_tag().to_string(),
        tx_count,
    })
}

async fn seed_monitoring_downgrade_fixture(
    db: &sqlx::PgPool,
    signal: FixtureSignal,
) -> Result<InvestigationFixtureResponse, AppError> {
    let token_address = signal.token_address().to_string();
    let run_id = Uuid::new_v4();
    let now = Utc::now();
    let deployed_at = now - Duration::minutes(25);
    let tx_count = 24_i64;
    let buy_count = 15_i32;
    let sell_count = 9_i32;
    let volume_bnb = 1.18_f64;
    let block_number = 9_000_100_i64;
    let token_tx_hash = fixture_hash("stale-activity-token");

    sqlx::query("DELETE FROM investigation_runs WHERE token_address = $1")
        .bind(&token_address)
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM whale_alerts WHERE token_address = $1")
        .bind(&token_address)
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM token_transactions WHERE token_address = $1")
        .bind(&token_address)
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM wallet_clusters WHERE token_address = $1")
        .bind(&token_address)
        .execute(db)
        .await?;
    if signal == FixtureSignal::BuilderOverlap {
        sqlx::query("DELETE FROM tokens WHERE contract_address = $1")
            .bind(FIXTURE_BUILDER_OVERLAP_RELATED_TOKEN)
            .execute(db)
            .await?;
    }
    if signal == FixtureSignal::LinkedLaunchOverlap {
        sqlx::query("DELETE FROM tokens WHERE contract_address = $1")
            .bind(FIXTURE_LINKED_OVERLAP_RELATED_TOKEN)
            .execute(db)
            .await?;
        sqlx::query("DELETE FROM wallet_clusters WHERE token_address = $1")
            .bind(FIXTURE_LINKED_OVERLAP_RELATED_TOKEN)
            .execute(db)
            .await?;
    }

    sqlx::query(
        r#"
        INSERT INTO tokens (
            contract_address,
            deployer_address,
            name,
            symbol,
            deployed_at,
            block_number,
            tx_hash,
            initial_liquidity_bnb,
            holder_count,
            buy_count,
            sell_count,
            volume_bnb,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW())
        ON CONFLICT (contract_address) DO UPDATE
        SET
            deployer_address = EXCLUDED.deployer_address,
            name = EXCLUDED.name,
            symbol = EXCLUDED.symbol,
            deployed_at = EXCLUDED.deployed_at,
            block_number = EXCLUDED.block_number,
            tx_hash = EXCLUDED.tx_hash,
            initial_liquidity_bnb = EXCLUDED.initial_liquidity_bnb,
            holder_count = EXCLUDED.holder_count,
            buy_count = EXCLUDED.buy_count,
            sell_count = EXCLUDED.sell_count,
            volume_bnb = EXCLUDED.volume_bnb,
            updated_at = NOW()
        "#,
    )
    .bind(&token_address)
    .bind("0x0000000000000000000000000000000000000d03")
    .bind(signal.display_name())
    .bind(signal.symbol())
    .bind(deployed_at)
    .bind(block_number)
    .bind(token_tx_hash)
    .bind(0.62_f64)
    .bind(9_i32)
    .bind(buy_count)
    .bind(sell_count)
    .bind(volume_bnb)
    .execute(db)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO risk_scores (
            token_address,
            composite_score,
            deployer_history_score,
            liquidity_lock_score,
            wallet_concentration_score,
            buy_sell_velocity_score,
            contract_audit_score,
            social_authenticity_score,
            volume_consistency_score,
            computed_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
        ON CONFLICT (token_address) DO UPDATE
        SET
            composite_score = EXCLUDED.composite_score,
            deployer_history_score = EXCLUDED.deployer_history_score,
            liquidity_lock_score = EXCLUDED.liquidity_lock_score,
            wallet_concentration_score = EXCLUDED.wallet_concentration_score,
            buy_sell_velocity_score = EXCLUDED.buy_sell_velocity_score,
            contract_audit_score = EXCLUDED.contract_audit_score,
            social_authenticity_score = EXCLUDED.social_authenticity_score,
            volume_consistency_score = EXCLUDED.volume_consistency_score,
            computed_at = NOW()
        "#,
    )
    .bind(&token_address)
    .bind(44_i16)
    .bind(37_i16)
    .bind(68_i16)
    .bind(41_i16)
    .bind(39_i16)
    .bind(57_i16)
    .bind(33_i16)
    .bind(43_i16)
    .execute(db)
    .await?;

    if signal == FixtureSignal::SourceDegradation {
        sqlx::query(
            r#"
            INSERT INTO deep_research_reports (
                token_address,
                provider_path,
                executive_summary,
                sections,
                citations,
                source_status,
                raw_payload,
                updated_at
            )
            VALUES (
                $1,
                'fixtures/source-degradation',
                'Fixture report with degraded source health.',
                '[]'::jsonb,
                '[]'::jsonb,
                $2::jsonb,
                '{}'::jsonb,
                NOW()
            )
            ON CONFLICT (token_address, provider_path) DO UPDATE
            SET
                executive_summary = EXCLUDED.executive_summary,
                sections = EXCLUDED.sections,
                citations = EXCLUDED.citations,
                source_status = EXCLUDED.source_status,
                raw_payload = EXCLUDED.raw_payload,
                updated_at = NOW()
            "#,
        )
        .bind(&token_address)
        .bind(
            serde_json::json!({
                "dexscreener": { "status": "degraded" },
                "heurist_mesh": { "status": "ready" }
            })
            .to_string(),
        )
        .execute(db)
        .await?;
    }

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
            summary,
            status_reason,
            evidence_delta,
            created_at,
            updated_at,
            started_at,
            completed_at
        )
        VALUES (
            $1,
            $2,
            'auto',
            'escalated',
            'auto_escalation',
            'system',
            'Fixture escalated monitoring',
            'fixture',
            $3,
            $4,
            $5,
            $6,
            NOW(),
            NOW(),
            NOW(),
            NOW()
        )
        "#,
    )
    .bind(run_id)
    .bind(&token_address)
    .bind(66_i32)
    .bind("Fixture seeded for monitoring downgrade proof.")
    .bind(match signal {
        FixtureSignal::SourceDegradation => {
            "Auto escalation reason: source verification stayed healthy while the run was in watching."
        }
        FixtureSignal::BuilderOverlap => {
            "Auto escalation reason: builder overlap is live with 1 seller wallet later appearing as new deployers while the run was in watching."
        }
        FixtureSignal::LinkedLaunchOverlap => {
            "Auto escalation reason: linked launch overlap is live across 2 related launches while the run was in watching."
        }
        _ => "Auto escalation reason: the launch stayed above 100 transactions and is now at 124 transactions while the run was in watching.",
    })
    .bind(match signal {
        FixtureSignal::SourceDegradation => {
            "Fixture started from an escalated source-health state but the latest report now shows degraded sources and empty citations."
        }
        FixtureSignal::BuilderOverlap => {
            "Fixture started from an escalated builder-overlap state but current live overlap signals are intentionally zero."
        }
        FixtureSignal::LinkedLaunchOverlap => {
            "Fixture started from an escalated linked-launch-overlap state but current live overlap signals are intentionally zero."
        }
        _ => "Fixture started from an escalated state but current live signals are intentionally below the promotion thresholds.",
    })
    .execute(db)
    .await?;

    append_run_event(
        db,
        run_id,
        "run_created",
        "Run created",
        "Fixture run created for monitoring downgrade proof.",
        None,
        None,
    )
    .await?;

    append_run_event(
        db,
        run_id,
        match signal {
            FixtureSignal::SourceDegradation => "auto_escalation_triggered_source_degradation",
            FixtureSignal::BuilderOverlap => "auto_escalation_triggered_builder_overlap",
            FixtureSignal::LinkedLaunchOverlap => "auto_escalation_triggered_linked_launch_overlap",
            _ => "auto_escalation_triggered_activity",
        },
        match signal {
            FixtureSignal::SourceDegradation => "Auto escalation: source degradation",
            FixtureSignal::BuilderOverlap => "Auto escalation: builder overlap",
            FixtureSignal::LinkedLaunchOverlap => "Auto escalation: linked launch overlap",
            _ => "Auto escalation: activity",
        },
        match signal {
            FixtureSignal::SourceDegradation => {
                "Fixture seeded as an escalated run so auto monitoring can downgrade it after source health degrades."
            }
            FixtureSignal::BuilderOverlap => {
                "Fixture seeded as an escalated run so auto monitoring can downgrade it after builder overlap cools."
            }
            FixtureSignal::LinkedLaunchOverlap => {
                "Fixture seeded as an escalated run so auto monitoring can downgrade it after linked launch overlap cools."
            }
            _ => "Fixture seeded as an escalated run so auto monitoring can downgrade it after signals cool.",
        },
        Some(match signal {
            FixtureSignal::SourceDegradation => {
                "Fixture escalated state ready for source-degradation monitoring downgrade."
            }
            FixtureSignal::BuilderOverlap => {
                "Fixture escalated state ready for builder-overlap monitoring downgrade."
            }
            FixtureSignal::LinkedLaunchOverlap => {
                "Fixture escalated state ready for linked-launch-overlap monitoring downgrade."
            }
            _ => "Fixture escalated state ready for monitoring downgrade.",
        }),
        Some(match signal {
            FixtureSignal::SourceDegradation => {
                "Source health is now intentionally degraded and citations are empty while the run stays below the automatic escalation thresholds."
            }
            FixtureSignal::BuilderOverlap => {
                "Builder overlap is now intentionally zero while the run stays below the automatic escalation thresholds."
            }
            FixtureSignal::LinkedLaunchOverlap => {
                "Linked launch overlap is now intentionally zero while the run stays below the automatic escalation thresholds."
            }
            _ => "Live activity is now intentionally below the automatic escalation thresholds.",
        }),
    )
    .await?;

    Ok(InvestigationFixtureResponse {
        token_address,
        run_id,
        signal_tag: signal.signal_tag().to_string(),
        tx_count,
    })
}

async fn seed_failed_run_fixture(
    db: &sqlx::PgPool,
) -> Result<InvestigationFixtureResponse, AppError> {
    let token_address = FIXTURE_FAILED_RUN_TOKEN.to_string();
    let run_id = Uuid::new_v4();
    let now = Utc::now();
    let deployed_at = now - Duration::minutes(18);
    let token_tx_hash = fixture_hash("failed-run-token");

    sqlx::query("DELETE FROM investigation_runs WHERE token_address = $1")
        .bind(&token_address)
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM tokens WHERE contract_address = $1")
        .bind(&token_address)
        .execute(db)
        .await?;

    sqlx::query(
        r#"
        INSERT INTO tokens (
            contract_address,
            deployer_address,
            name,
            symbol,
            deployed_at,
            block_number,
            tx_hash,
            initial_liquidity_bnb,
            holder_count,
            buy_count,
            sell_count,
            volume_bnb,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW())
        "#,
    )
    .bind(&token_address)
    .bind("0x0000000000000000000000000000000000000d09")
    .bind("Fixture Failed Run Token")
    .bind("FIXFAIL")
    .bind(deployed_at)
    .bind(9_000_200_i64)
    .bind(token_tx_hash)
    .bind(0.55_f64)
    .bind(7_i32)
    .bind(9_i32)
    .bind(5_i32)
    .bind(0.92_f64)
    .execute(db)
    .await?;

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
            summary,
            status_reason,
            evidence_delta,
            created_at,
            updated_at,
            started_at,
            completed_at
        )
        VALUES (
            $1,
            $2,
            'manual',
            'failed',
            'failed',
            'mia',
            'Fixture failed investigation',
            'fixture',
            $3,
            $4,
            $5,
            $6,
            NOW(),
            NOW(),
            NOW(),
            NOW()
        )
        "#,
    )
    .bind(run_id)
    .bind(&token_address)
    .bind(28_i32)
    .bind("Fixture failed run seeded for operator retry proof.")
    .bind("Fixture failure reason: upstream enrichment timed out during investigation.")
    .bind("Fixture evidence delta: the run is intentionally left failed so operator retry can move it back into queue.")
    .execute(db)
    .await?;

    append_run_event(
        db,
        run_id,
        "run_created",
        "Run created",
        "Fixture run created for failed-run retry proof.",
        None,
        None,
    )
    .await?;

    append_run_event(
        db,
        run_id,
        "fixture_failed_run",
        "Fixture failed run",
        "Fixture was seeded directly into failed so operator retry can re-queue it.",
        Some("Fixture failure reason: upstream enrichment timed out during investigation."),
        Some("Fixture evidence delta: the run is intentionally left failed so operator retry can move it back into queue."),
    )
    .await?;

    Ok(InvestigationFixtureResponse {
        token_address,
        run_id,
        signal_tag: "failed_run".to_string(),
        tx_count: 14,
    })
}

async fn seed_stale_running_fixture(
    db: &sqlx::PgPool,
) -> Result<InvestigationFixtureResponse, AppError> {
    let token_address = FIXTURE_STALE_RUNNING_TOKEN.to_string();
    let run_id = Uuid::new_v4();
    let now = Utc::now();
    let started_at = now - Duration::minutes(45);
    let token_tx_hash = fixture_hash("stale-running-token");

    sqlx::query("DELETE FROM investigation_runs WHERE token_address = $1")
        .bind(&token_address)
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM tokens WHERE contract_address = $1")
        .bind(&token_address)
        .execute(db)
        .await?;

    sqlx::query(
        r#"
        INSERT INTO tokens (
            contract_address,
            deployer_address,
            name,
            symbol,
            deployed_at,
            block_number,
            tx_hash,
            initial_liquidity_bnb,
            holder_count,
            buy_count,
            sell_count,
            volume_bnb,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW())
        "#,
    )
    .bind(&token_address)
    .bind("0x0000000000000000000000000000000000000d0a")
    .bind("Fixture Stale Running Token")
    .bind("FIXRUN")
    .bind(now - Duration::minutes(55))
    .bind(9_000_210_i64)
    .bind(token_tx_hash)
    .bind(0.48_f64)
    .bind(6_i32)
    .bind(7_i32)
    .bind(3_i32)
    .bind(0.74_f64)
    .execute(db)
    .await?;

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
            summary,
            status_reason,
            evidence_delta,
            created_at,
            updated_at,
            started_at,
            completed_at
        )
        VALUES (
            $1,
            $2,
            'manual',
            'running',
            'investigation',
            'mia',
            'Fixture running investigation',
            'fixture',
            $3,
            $4,
            $5,
            $6,
            $7,
            $7,
            $7,
            NULL
        )
        "#,
    )
    .bind(run_id)
    .bind(&token_address)
    .bind(31_i32)
    .bind("Fixture stale running run seeded for operator recovery proof.")
    .bind("Fixture running reason: this investigation has been left in running past the recovery window.")
    .bind("Fixture evidence delta: the run is intentionally stale so operator recovery can move it back into queue.")
    .bind(started_at)
    .execute(db)
    .await?;

    append_run_event(
        db,
        run_id,
        "run_created",
        "Run created",
        "Fixture run created for stale-running recovery proof.",
        None,
        None,
    )
    .await?;

    append_run_event(
        db,
        run_id,
        "fixture_stale_running",
        "Fixture stale running",
        "Fixture was seeded directly into a stale running state so operator recovery can re-queue it.",
        Some("Fixture running reason: this investigation has been left in running past the recovery window."),
        Some("Fixture evidence delta: the run is intentionally stale so operator recovery can move it back into queue."),
    )
    .await?;

    Ok(InvestigationFixtureResponse {
        token_address,
        run_id,
        signal_tag: "stale_running".to_string(),
        tx_count: 11,
    })
}

fn fixture_hash(_seed: &str) -> String {
    let first = Uuid::new_v4().simple().to_string();
    let second = Uuid::new_v4().simple().to_string();
    let digest = format!("{first}{second}");
    let prefixed = format!("0x{digest}");
    prefixed.chars().take(66).collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_hash_looks_like_transaction_hash() {
        let value = fixture_hash("wallet_concentration-token");
        assert!(value.starts_with("0x"));
        assert_eq!(value.len(), 66);
    }

    #[test]
    fn fixture_signal_metadata_is_stable() {
        assert_eq!(
            FixtureSignal::WalletConcentration.signal_tag(),
            "wallet_concentration"
        );
        assert_eq!(FixtureSignal::WhaleAlert.signal_tag(), "whale_alert");
        assert_eq!(
            FixtureSignal::BuilderOverlap.signal_tag(),
            "builder_overlap"
        );
        assert_eq!(
            FixtureSignal::LinkedLaunchOverlap.signal_tag(),
            "linked_launch_overlap"
        );
        assert_eq!(
            FixtureSignal::SourceDegradation.signal_tag(),
            "source_degradation"
        );
        assert_eq!(FixtureSignal::StaleActivity.signal_tag(), "activity");
    }
}
