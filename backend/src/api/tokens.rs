use crate::{error::AppError, AppState};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ─── Query params ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TokenListQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    /// Optional filter: "high" | "medium" | "low"
    pub risk: Option<String>,
    /// Optional free-text query against symbol/name/address/deployer.
    pub q: Option<String>,
    /// Minimum token volume (BNB).
    pub min_liquidity: Option<f64>,
    /// Sort mode: newest | volume | risk | activity
    pub sort: Option<String>,
    /// Optional activity window in hours when sort=activity.
    pub window_hours: Option<i64>,
    /// Filter tokens that are eligible for AI scoring.
    pub ai_scored: Option<bool>,
    /// Filter tokens that already have a deep research report.
    pub deep_research: Option<bool>,
}

fn default_limit() -> i64 {
    20
}

// ─── Response shapes ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct TokenListResponse {
    pub data: Vec<TokenSummary>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize)]
pub struct TokenSummary {
    pub contract_address: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub deployer_address: String,
    pub deployed_at: DateTime<Utc>,
    pub block_number: i64,
    pub buy_count: i32,
    pub sell_count: i32,
    pub total_tx: i32,
    pub volume_bnb: f64,
    pub composite_score: Option<i16>,
    pub risk_category: Option<String>,
    pub ai_scored: bool,
    pub deep_researched: bool,
    pub watching_for: String,
    pub window_hours: Option<i64>,
    pub window_volume_bnb: Option<f64>,
    pub window_buy_count: Option<i64>,
    pub window_sell_count: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct TokenDetail {
    pub contract_address: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub deployer_address: String,
    pub deployed_at: DateTime<Utc>,
    pub block_number: i64,
    pub tx_hash: String,
    pub initial_liquidity_bnb: Option<f64>,
    pub holder_count: i32,
    pub buy_count: i32,
    pub sell_count: i32,
    pub volume_bnb: f64,
    pub is_rug: bool,
    pub graduated: bool,
    pub honeypot_detected: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct RiskDetail {
    pub token_address: String,
    pub composite_score: i16,
    pub risk_category: String,
    pub deployer_history_score: Option<i16>,
    pub liquidity_lock_score: Option<i16>,
    pub wallet_concentration_score: Option<i16>,
    pub buy_sell_velocity_score: Option<i16>,
    pub contract_audit_score: Option<i16>,
    pub social_authenticity_score: Option<i16>,
    pub volume_consistency_score: Option<i16>,
    pub computed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct TransactionSummary {
    pub tx_hash: String,
    pub wallet_address: String,
    pub tx_type: String,
    pub amount_bnb: f64,
    pub block_number: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct TransactionListResponse {
    pub data: Vec<TransactionSummary>,
    pub total: i64,
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn risk_category_from_score(score: i16) -> &'static str {
    if score <= 30 {
        "low"
    } else if score <= 60 {
        "medium"
    } else {
        "high"
    }
}

fn build_token_watching_for(
    total_tx: i32,
    ai_scored: bool,
    deep_researched: bool,
    ai_score_gate: i32,
    deep_research_threshold: i64,
) -> String {
    if !ai_scored {
        return format!(
            "Watching for more than {} total transactions so MIA can unlock a live AI score.",
            ai_score_gate
        );
    }

    if deep_researched {
        return "Watching for live holder, builder, and flow changes after deep research.".to_string();
    }

    if i64::from(total_tx) >= deep_research_threshold {
        return format!(
            "Watching for the deep research report after activity cleared the {} transaction threshold.",
            deep_research_threshold
        );
    }

    format!(
        "Watching for {} total transactions so deep research can auto-start.",
        deep_research_threshold
    )
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

/// GET /api/v1/tokens
pub async fn list_tokens(
    State(state): State<AppState>,
    Query(params): Query<TokenListQuery>,
) -> Result<Json<TokenListResponse>, AppError> {
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);
    let q = params
        .q
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{}%", s.to_lowercase()));
    let min_liquidity = params.min_liquidity.unwrap_or(0.0).max(0.0);
    let sort_mode = params.sort.as_deref().unwrap_or("newest");
    let window_hours = params.window_hours.unwrap_or(24).clamp(1, 24 * 7);
    let ai_score_gate = state.config.ai_score_min_tx_count as i32;

    // Build score filter if requested
    let (score_min, score_max): (i16, i16) = match params.risk.as_deref() {
        Some("low") => (0, 30),
        Some("medium") => (31, 60),
        Some("high") => (61, 100),
        _ => (0, 100),
    };

    let order_clause = match sort_mode {
        "volume" => "t.volume_bnb DESC, t.deployed_at DESC",
        "risk" => "COALESCE(rs.composite_score, -1) DESC, t.deployed_at DESC",
        "tx" => "(t.buy_count + t.sell_count) DESC, t.deployed_at DESC",
        "activity" => {
            "COALESCE(act.window_volume_bnb, 0) DESC, COALESCE(act.window_buy_count, 0) DESC, t.deployed_at DESC"
        }
        _ => "t.deployed_at DESC",
    };

    let query = format!(
        r#"
        SELECT
            t.contract_address,
            t.name,
            t.symbol,
            t.deployer_address,
            t.deployed_at,
            t.block_number,
            t.buy_count,
            t.sell_count,
            (t.buy_count + t.sell_count) AS total_tx,
            t.volume_bnb::double precision,
            rs.composite_score,
            (dr.token_address IS NOT NULL) AS deep_researched,
            ((t.buy_count + t.sell_count) > $10 OR dr.token_address IS NOT NULL) AS ai_scored,
            act.window_volume_bnb,
            act.window_buy_count,
            act.window_sell_count
        FROM tokens t
        LEFT JOIN risk_scores rs ON rs.token_address = t.contract_address
        LEFT JOIN (
            SELECT DISTINCT token_address
            FROM deep_research_reports
        ) dr ON dr.token_address = t.contract_address
        LEFT JOIN (
            SELECT
                token_address,
                SUM(amount_bnb)::double precision AS window_volume_bnb,
                COUNT(*) FILTER (WHERE tx_type = 'buy')::bigint AS window_buy_count,
                COUNT(*) FILTER (WHERE tx_type = 'sell')::bigint AS window_sell_count
            FROM token_transactions
            WHERE created_at >= NOW() - make_interval(hours => $7::int)
            GROUP BY token_address
        ) act ON act.token_address = t.contract_address
        WHERE (rs.composite_score IS NULL OR rs.composite_score BETWEEN $1 AND $2)
          AND t.volume_bnb::double precision >= $3
          AND (
            $4::text IS NULL
            OR LOWER(COALESCE(t.symbol, '')) LIKE $4
            OR LOWER(COALESCE(t.name, '')) LIKE $4
            OR LOWER(t.contract_address) LIKE $4
            OR LOWER(t.deployer_address) LIKE $4
          )
          AND (
            $8::bool IS NULL
            OR ($8 = true AND dr.token_address IS NOT NULL)
            OR ($8 = false AND dr.token_address IS NULL)
          )
          AND (
            $9::bool IS NULL
            OR ($9 = true AND ((t.buy_count + t.sell_count) > $10 OR dr.token_address IS NOT NULL))
            OR ($9 = false AND ((t.buy_count + t.sell_count) <= $10 AND dr.token_address IS NULL))
          )
        ORDER BY {}
        LIMIT $5 OFFSET $6
        "#,
        order_clause
    );

    let rows: Vec<(
        String,
        Option<String>,
        Option<String>,
        String,
        DateTime<Utc>,
        i64,
        i32,
        i32,
        i32,
        f64,
        Option<i16>,
        bool,
        bool,
        Option<f64>,
        Option<i64>,
        Option<i64>,
    )> = sqlx::query_as(&query)
        .bind(score_min)
        .bind(score_max)
        .bind(min_liquidity)
        .bind(q.as_deref())
        .bind(limit)
        .bind(offset)
        .bind(window_hours as i32)
        .bind(params.deep_research)
        .bind(params.ai_scored)
        .bind(ai_score_gate)
        .fetch_all(&state.db)
        .await?;

    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM tokens t
        LEFT JOIN risk_scores rs ON rs.token_address = t.contract_address
        LEFT JOIN (
            SELECT DISTINCT token_address
            FROM deep_research_reports
        ) dr ON dr.token_address = t.contract_address
        WHERE (rs.composite_score IS NULL OR rs.composite_score BETWEEN $1 AND $2)
          AND t.volume_bnb::double precision >= $3
          AND (
            $4::text IS NULL
            OR LOWER(COALESCE(t.symbol, '')) LIKE $4
            OR LOWER(COALESCE(t.name, '')) LIKE $4
            OR LOWER(t.contract_address) LIKE $4
            OR LOWER(t.deployer_address) LIKE $4
          )
          AND (
            $5::bool IS NULL
            OR ($5 = true AND dr.token_address IS NOT NULL)
            OR ($5 = false AND dr.token_address IS NULL)
          )
          AND (
            $6::bool IS NULL
            OR ($6 = true AND ((t.buy_count + t.sell_count) > $7 OR dr.token_address IS NOT NULL))
            OR ($6 = false AND ((t.buy_count + t.sell_count) <= $7 AND dr.token_address IS NULL))
          )
        "#,
    )
    .bind(score_min)
    .bind(score_max)
    .bind(min_liquidity)
    .bind(q.as_deref())
    .bind(params.deep_research)
    .bind(params.ai_scored)
    .bind(ai_score_gate)
    .fetch_one(&state.db)
    .await?;

    let data = rows
        .into_iter()
        .map(
            |(
                contract_address,
                name,
                symbol,
                deployer_address,
                deployed_at,
                block_number,
                buy_count,
                sell_count,
                total_tx,
                volume_bnb,
                composite_score,
                deep_researched,
                ai_scored,
                window_volume_bnb,
                window_buy_count,
                window_sell_count,
            )| {
                let risk_category =
                    composite_score.map(|s| risk_category_from_score(s).to_string());
                TokenSummary {
                    contract_address,
                    name,
                    symbol,
                    deployer_address,
                    deployed_at,
                    block_number,
                    buy_count,
                    sell_count,
                    total_tx,
                    volume_bnb,
                    composite_score,
                    risk_category,
                    ai_scored,
                    deep_researched,
                    watching_for: build_token_watching_for(
                        total_tx,
                        ai_scored,
                        deep_researched,
                        ai_score_gate,
                        state.config.auto_deep_research_tx_threshold,
                    ),
                    window_hours: (sort_mode == "activity").then_some(window_hours),
                    window_volume_bnb,
                    window_buy_count,
                    window_sell_count,
                }
            },
        )
        .collect();

    Ok(Json(TokenListResponse {
        data,
        total: total.0,
        limit,
        offset,
    }))
}

/// GET /api/v1/tokens/:address
pub async fn get_token(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<TokenDetail>, AppError> {
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
        DateTime<Utc>,
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
            honeypot_detected,
            created_at
        FROM tokens
        WHERE contract_address = $1
        "#,
    )
    .bind(&address)
    .fetch_optional(&state.db)
    .await?;

    match row {
        None => Err(AppError::NotFound(format!("Token {} not found", address))),
        Some((
            contract_address,
            name,
            symbol,
            deployer_address,
            deployed_at,
            block_number,
            tx_hash,
            initial_liquidity_bnb,
            holder_count,
            buy_count,
            sell_count,
            volume_bnb,
            is_rug,
            graduated,
            honeypot_detected,
            created_at,
        )) => Ok(Json(TokenDetail {
            contract_address,
            name,
            symbol,
            deployer_address,
            deployed_at,
            block_number,
            tx_hash,
            initial_liquidity_bnb,
            holder_count,
            buy_count,
            sell_count,
            volume_bnb,
            is_rug,
            graduated,
            honeypot_detected,
            created_at,
        })),
    }
}

/// GET /api/v1/tokens/:address/risk
pub async fn get_token_risk(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<RiskDetail>, AppError> {
    let row: Option<(
        String,
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
        FROM risk_scores
        WHERE token_address = $1
        "#,
    )
    .bind(&address)
    .fetch_optional(&state.db)
    .await?;

    match row {
        None => Err(AppError::NotFound(format!(
            "Risk score for token {} not found",
            address
        ))),
        Some((
            token_address,
            composite_score,
            deployer_history_score,
            liquidity_lock_score,
            wallet_concentration_score,
            buy_sell_velocity_score,
            contract_audit_score,
            social_authenticity_score,
            volume_consistency_score,
            computed_at,
        )) => Ok(Json(RiskDetail {
            risk_category: risk_category_from_score(composite_score).to_string(),
            token_address,
            composite_score,
            deployer_history_score,
            liquidity_lock_score,
            wallet_concentration_score,
            buy_sell_velocity_score,
            contract_audit_score,
            social_authenticity_score,
            volume_consistency_score,
            computed_at,
        })),
    }
}

/// GET /api/v1/tokens/:address/transactions
pub async fn get_token_transactions(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<TransactionListResponse>, AppError> {
    let rows: Vec<(String, String, String, f64, i64, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT
            tx_hash,
            wallet_address,
            tx_type,
            amount_bnb::double precision,
            block_number,
            created_at
        FROM token_transactions
        WHERE token_address = $1
        ORDER BY created_at DESC
        LIMIT 100
        "#,
    )
    .bind(&address)
    .fetch_all(&state.db)
    .await?;

    let total: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM token_transactions WHERE token_address = $1")
            .bind(&address)
            .fetch_one(&state.db)
            .await?;

    let data = rows
        .into_iter()
        .map(
            |(tx_hash, wallet_address, tx_type, amount_bnb, block_number, created_at)| {
                TransactionSummary {
                    tx_hash,
                    wallet_address,
                    tx_type,
                    amount_bnb,
                    block_number,
                    created_at,
                }
            },
        )
        .collect();

    Ok(Json(TransactionListResponse {
        data,
        total: total.0,
    }))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — pure logic, no DB required
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risk_category_low_boundary() {
        assert_eq!(risk_category_from_score(0), "low");
        assert_eq!(risk_category_from_score(30), "low");
    }

    #[test]
    fn risk_category_medium_boundary() {
        assert_eq!(risk_category_from_score(31), "medium");
        assert_eq!(risk_category_from_score(60), "medium");
    }

    #[test]
    fn risk_category_high_boundary() {
        assert_eq!(risk_category_from_score(61), "high");
        assert_eq!(risk_category_from_score(100), "high");
    }

    #[test]
    fn default_limit_is_twenty() {
        assert_eq!(default_limit(), 20);
    }

    #[test]
    fn token_watching_for_explains_score_unlock_before_ai_gate() {
        let watching_for = build_token_watching_for(18, false, false, 50, 500);
        assert!(watching_for.contains("50"));
        assert!(watching_for.contains("AI score"));
    }

    #[test]
    fn token_watching_for_explains_deep_research_after_score_unlock() {
        let watching_for = build_token_watching_for(120, true, false, 50, 500);
        assert!(watching_for.contains("500"));
        assert!(watching_for.contains("deep research"));
    }

    #[test]
    fn token_watching_for_mentions_post_research_monitoring() {
        let watching_for = build_token_watching_for(800, true, true, 50, 500);
        assert!(watching_for.contains("after deep research"));
    }
}
