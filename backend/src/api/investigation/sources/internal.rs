use chrono::{DateTime, Duration, Utc};

use crate::{error::AppError, AppState};

use super::super::types::{
    AlphaContextSnapshot, DeployerTokenSnapshot, NarrativeCacheSnapshot, RiskSnapshot,
    TokenSnapshot, TransactionSnapshot, WhaleActivitySnapshot,
};
use super::helpers::risk_category;

pub(crate) async fn fetch_token_snapshot(
    state: &AppState,
    address: &str,
) -> Result<TokenSnapshot, AppError> {
    let row: Option<(
        String,
        Option<String>,
        Option<String>,
        String,
        DateTime<Utc>,
        i64,
        String,
        Option<f64>,
        i32,
        i32,
        i32,
        f64,
        bool,
        bool,
        bool,
    )> = sqlx::query_as(
        r#"
        SELECT
            contract_address,
            name,
            symbol,
            deployer_address,
            deployed_at,
            block_number,
            tx_hash,
            initial_liquidity_bnb::double precision,
            holder_count,
            buy_count,
            sell_count,
            volume_bnb::double precision,
            is_rug,
            graduated,
            honeypot_detected
        FROM tokens
        WHERE contract_address = $1
        "#,
    )
    .bind(address)
    .fetch_optional(&state.db)
    .await?;

    let row = row.ok_or_else(|| AppError::NotFound(format!("Token {} not found", address)))?;

    Ok(TokenSnapshot {
        contract_address: row.0,
        name: row.1,
        symbol: row.2,
        deployer_address: row.3,
        deployed_at: row.4,
        block_number: row.5,
        tx_hash: row.6,
        initial_liquidity_bnb: row.7,
        participant_wallet_count: row.8,
        holder_count: row.8,
        buy_count: row.9,
        sell_count: row.10,
        volume_bnb: row.11,
        is_rug: row.12,
        graduated: row.13,
        honeypot_detected: row.14,
    })
}

pub(super) async fn fetch_risk_snapshot(
    state: &AppState,
    address: &str,
) -> Result<Option<RiskSnapshot>, AppError> {
    let row: Option<(
        i16,
        Option<i16>,
        Option<i16>,
        Option<i16>,
        Option<i16>,
        Option<i16>,
        Option<i16>,
        Option<i16>,
        DateTime<Utc>,
    )> = sqlx::query_as(
        r#"
        SELECT
            composite_score,
            deployer_history_score,
            liquidity_lock_score,
            wallet_concentration_score,
            buy_sell_velocity_score,
            contract_audit_score,
            social_authenticity_score,
            volume_consistency_score,
            computed_at
        FROM risk_scores
        WHERE token_address = $1
        "#,
    )
    .bind(address)
    .fetch_optional(&state.db)
    .await?;

    Ok(row.map(|value| RiskSnapshot {
        composite_score: value.0,
        risk_category: risk_category(value.0),
        deployer_history_score: value.1,
        liquidity_lock_score: value.2,
        wallet_concentration_score: value.3,
        buy_sell_velocity_score: value.4,
        contract_audit_score: value.5,
        social_authenticity_score: value.6,
        volume_consistency_score: value.7,
        computed_at: value.8,
    }))
}

pub(super) async fn fetch_narrative_cache(
    state: &AppState,
    address: &str,
) -> Result<Option<NarrativeCacheSnapshot>, AppError> {
    let row: Option<(
        String,
        Option<String>,
        String,
        String,
        DateTime<Utc>,
        DateTime<Utc>,
    )> = sqlx::query_as(
        r#"
        SELECT
            narrative_text,
            risk_interpretation,
            consensus_status,
            confidence,
            generated_at,
            expires_at
        FROM ai_narratives
        WHERE token_address = $1
        "#,
    )
    .bind(address)
    .fetch_optional(&state.db)
    .await?;

    Ok(row.map(|value| NarrativeCacheSnapshot {
        narrative_text: value.0,
        risk_interpretation: value.1,
        consensus_status: value.2,
        confidence: value.3,
        generated_at: value.4,
        expires_at: value.5,
    }))
}

pub(super) async fn fetch_recent_transactions(
    state: &AppState,
    address: &str,
    limit: i64,
) -> Result<Vec<TransactionSnapshot>, AppError> {
    let rows: Vec<(String, String, String, f64, i64, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT wallet_address, tx_hash, tx_type, amount_bnb::double precision, block_number, created_at
        FROM token_transactions
        WHERE token_address = $1
        ORDER BY created_at DESC
        LIMIT $2
        "#,
    )
    .bind(address)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| TransactionSnapshot {
            wallet_address: row.0,
            tx_hash: row.1,
            tx_type: row.2,
            amount_bnb: row.3,
            block_number: row.4,
            created_at: row.5,
        })
        .collect())
}

pub(super) async fn fetch_whale_activity(
    state: &AppState,
    address: &str,
) -> Result<WhaleActivitySnapshot, AppError> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT alert_level
        FROM whale_alerts
        WHERE token_address = $1
          AND created_at >= $2
        ORDER BY created_at DESC
        LIMIT 10
        "#,
    )
    .bind(address)
    .bind(Utc::now() - Duration::hours(24))
    .fetch_all(&state.db)
    .await?;

    let latest_levels: Vec<String> = rows.iter().map(|row| row.0.clone()).collect();
    let watch_alerts = latest_levels
        .iter()
        .filter(|level| level.as_str() == "watch")
        .count();
    let critical_alerts = latest_levels
        .iter()
        .filter(|level| level.as_str() == "critical")
        .count();

    Ok(WhaleActivitySnapshot {
        watch_alerts,
        critical_alerts,
        latest_levels,
    })
}

pub(super) async fn fetch_alpha_context(
    state: &AppState,
    address: &str,
) -> Result<Option<AlphaContextSnapshot>, AppError> {
    let row: Option<(i16, f64, String, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT rank, alpha_score::double precision, rationale, window_end
        FROM alpha_rankings
        WHERE token_address = $1
          AND window_end >= $2
        ORDER BY window_end DESC, rank ASC
        LIMIT 1
        "#,
    )
    .bind(address)
    .bind(Utc::now() - Duration::hours(24))
    .fetch_optional(&state.db)
    .await?;

    Ok(row.map(|value| AlphaContextSnapshot {
        rank: value.0,
        alpha_score: value.1,
        rationale: value.2,
        window_end: value.3,
    }))
}

pub(super) async fn fetch_deployer_recent_tokens(
    state: &AppState,
    deployer_address: &str,
    limit: i64,
) -> Result<Vec<DeployerTokenSnapshot>, AppError> {
    let rows: Vec<(
        String,
        Option<String>,
        Option<String>,
        DateTime<Utc>,
        i32,
        i32,
        f64,
        Option<i16>,
    )> = sqlx::query_as(
        r#"
        SELECT
            t.contract_address,
            t.name,
            t.symbol,
            t.deployed_at,
            t.buy_count,
            t.sell_count,
            t.volume_bnb::double precision,
            rs.composite_score
        FROM tokens t
        LEFT JOIN risk_scores rs ON rs.token_address = t.contract_address
        WHERE t.deployer_address = $1
        ORDER BY t.deployed_at DESC
        LIMIT $2
        "#,
    )
    .bind(deployer_address)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|value| DeployerTokenSnapshot {
            contract_address: value.0,
            name: value.1,
            symbol: value.2,
            deployed_at: value.3,
            buy_count: value.4,
            sell_count: value.5,
            volume_bnb: value.6,
            composite_score: value.7,
            risk_category: value.7.map(risk_category),
        })
        .collect())
}
