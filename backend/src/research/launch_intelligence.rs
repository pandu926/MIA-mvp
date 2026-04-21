use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgPool};

use crate::{
    api::investigation::TokenSnapshot,
    indexer::deployer::{get_deployer_profile, TrustGrade},
};

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct DeployerLaunchRef {
    pub contract_address: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub is_rug: bool,
    pub graduated: bool,
    pub deployed_at: DateTime<Utc>,
    pub buy_count: i32,
    pub sell_count: i32,
    pub volume_bnb: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeployerMemorySummary {
    pub summary: String,
    pub evidence: Vec<String>,
    pub trust_grade: String,
    pub trust_label: String,
    pub total_launches: i64,
    pub rug_count: i64,
    pub graduated_count: i64,
    pub honeypot_history: bool,
    pub first_seen_at: Option<DateTime<Utc>>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub recent_launches: Vec<DeployerLaunchRef>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
struct WalletFlowRow {
    wallet_address: String,
    net_flow_bnb: f64,
    txn_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct WalletStructureSummary {
    pub summary: String,
    pub evidence: Vec<String>,
    pub active_wallet_count: i64,
    pub participant_wallet_count: i32,
    pub holder_count: i32,
    pub probable_cluster_wallets: i64,
    pub potential_cluster_wallets: i64,
    pub repeated_wallet_count: i64,
    pub top_flow_wallets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct OperatorFamilyLaunchRef {
    pub contract_address: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub deployer_address: String,
    pub deployed_at: DateTime<Utc>,
    pub is_rug: bool,
    pub graduated: bool,
    pub overlap_wallets: i64,
}

#[derive(Debug, Clone, Serialize, FromRow)]
struct OperatorWalletOverlapRow {
    wallet_address: String,
    overlap_launches: i64,
    overlap_deployers: i64,
}

#[derive(Debug, Clone, Serialize, FromRow)]
struct SellerMigrationRow {
    wallet_address: String,
    future_launches: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorFamilySnapshot {
    pub probable_cluster_wallets: usize,
    pub potential_cluster_wallets: usize,
    pub repeated_wallet_count: usize,
    pub related_launch_count: usize,
    pub related_deployer_count: usize,
    pub seller_to_new_builder_count: usize,
    pub seller_reentry_wallet_count: usize,
    pub deployer_rug_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct OperatorFamilySummary {
    pub confidence: String,
    pub summary: String,
    pub evidence: Vec<String>,
    pub safety_score: i16,
    pub signal_score: i16,
    pub related_launch_count: i64,
    pub related_deployer_count: i64,
    pub repeated_wallet_count: i64,
    pub seller_to_new_builder_count: i64,
    pub seller_reentry_wallet_count: i64,
    pub probable_cluster_wallets: i64,
    pub potential_cluster_wallets: i64,
    pub repeated_wallets: Vec<String>,
    pub migrated_wallets: Vec<String>,
    pub related_launches: Vec<OperatorFamilyLaunchRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorFamilyConfidence {
    Low,
    Medium,
    High,
}

impl OperatorFamilyConfidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

fn trust_label(grade: &TrustGrade) -> &'static str {
    grade.label()
}

pub fn score_operator_family_confidence(
    snapshot: &OperatorFamilySnapshot,
) -> OperatorFamilyConfidence {
    let mut points = 0u8;

    if snapshot.probable_cluster_wallets >= 5 {
        points += 3;
    } else if snapshot.potential_cluster_wallets >= 3 {
        points += 1;
    }

    if snapshot.repeated_wallet_count >= 4 {
        points += 2;
    } else if snapshot.repeated_wallet_count >= 2 {
        points += 1;
    }

    if snapshot.related_launch_count >= 4 {
        points += 2;
    } else if snapshot.related_launch_count >= 2 {
        points += 1;
    }

    if snapshot.related_deployer_count >= 2 {
        points += 2;
    } else if snapshot.related_deployer_count >= 1 {
        points += 1;
    }

    if snapshot.seller_to_new_builder_count >= 1 {
        points += 2;
    }

    if snapshot.seller_reentry_wallet_count >= 2 {
        points += 2;
    } else if snapshot.seller_reentry_wallet_count >= 1 {
        points += 1;
    }

    if snapshot.deployer_rug_count >= 1 {
        points += 1;
    }

    match points {
        0..=2 => OperatorFamilyConfidence::Low,
        3..=5 => OperatorFamilyConfidence::Medium,
        _ => OperatorFamilyConfidence::High,
    }
}

pub fn build_operator_family_summary_text(
    snapshot: &OperatorFamilySnapshot,
    confidence: OperatorFamilyConfidence,
) -> String {
    let intro = match confidence {
        OperatorFamilyConfidence::Low => {
            "No strong cross-launch operator-family pattern has been recovered yet"
        }
        OperatorFamilyConfidence::Medium => {
            "MIA sees a medium-confidence likely coordinated operator-family pattern"
        }
        OperatorFamilyConfidence::High => {
            "MIA sees a high-confidence likely coordinated operator-family pattern"
        }
    };

    format!(
        "{intro}: {} repeated wallet(s) connect to {} related launch(es) across {} deployer wallet(s), while {} seller wallet(s) later reappear in new launches and {} seller wallet(s) later become deployer wallets. This is pattern-risk detection, not an identity claim.",
        snapshot.repeated_wallet_count,
        snapshot.related_launch_count,
        snapshot.related_deployer_count,
        snapshot.seller_reentry_wallet_count,
        snapshot.seller_to_new_builder_count,
    )
}

fn operator_family_signal_score(snapshot: &OperatorFamilySnapshot) -> i16 {
    let mut risk_score = 0.0;
    risk_score += (snapshot.probable_cluster_wallets.min(8) as f64) * 8.0;
    risk_score += (snapshot.potential_cluster_wallets.min(8) as f64) * 3.0;
    risk_score += (snapshot.repeated_wallet_count.min(8) as f64) * 7.0;
    risk_score += (snapshot.related_launch_count.min(8) as f64) * 5.5;
    risk_score += (snapshot.related_deployer_count.min(6) as f64) * 8.0;
    risk_score += (snapshot.seller_to_new_builder_count.min(4) as f64) * 12.0;
    risk_score += (snapshot.seller_reentry_wallet_count.min(6) as f64) * 7.0;
    risk_score += (snapshot.deployer_rug_count.min(4) as f64) * 4.0;
    risk_score.round().clamp(0.0, 100.0) as i16
}

pub async fn build_deployer_memory_summary(
    pool: &PgPool,
    token: &TokenSnapshot,
) -> Result<Option<DeployerMemorySummary>> {
    let Some(profile) = get_deployer_profile(pool, &token.deployer_address).await? else {
        return Ok(None);
    };

    let recent_launches: Vec<DeployerLaunchRef> = sqlx::query_as(
        r#"
        SELECT
            contract_address,
            symbol,
            name,
            is_rug,
            graduated,
            deployed_at,
            buy_count,
            sell_count,
            volume_bnb::double precision AS volume_bnb
        FROM tokens
        WHERE deployer_address = $1
          AND LOWER(contract_address) <> LOWER($2)
        ORDER BY deployed_at DESC
        LIMIT 4
        "#,
    )
    .bind(&token.deployer_address)
    .bind(&token.contract_address)
    .fetch_all(pool)
    .await?;

    let trust_grade = profile.trust_grade.as_str().to_string();
    let trust_label = trust_label(&profile.trust_grade).to_string();

    let summary = if profile.total_tokens_deployed <= 1 {
        format!(
            "This deployer looks new in MIA's dataset with a {}-grade profile and no meaningful launch history yet.",
            trust_grade
        )
    } else if profile.rug_count > 0 {
        format!(
            "This deployer carries a {}-grade history across {} launches, including {} rug-flagged launch(es).",
            trust_grade, profile.total_tokens_deployed, profile.rug_count
        )
    } else if profile.graduated_count > 0 {
        format!(
            "This deployer carries a {}-grade history across {} launches, with {} graduated launch(es) and no recorded rugs.",
            trust_grade, profile.total_tokens_deployed, profile.graduated_count
        )
    } else {
        format!(
            "This deployer has a {}-grade history across {} launches, but the track record is still neutral rather than trusted.",
            trust_grade, profile.total_tokens_deployed
        )
    };

    let mut evidence = vec![
        format!("Trust grade: {} ({trust_label}).", trust_grade),
        format!(
            "Total recorded launches: {}.",
            profile.total_tokens_deployed
        ),
        format!("Graduated launches: {}.", profile.graduated_count),
        format!("Rug-flagged launches: {}.", profile.rug_count),
        format!("Honeypot history present: {}.", profile.honeypot_detected),
    ];
    if let Some(first_seen_at) = profile.first_seen_at {
        evidence.push(format!(
            "First seen in dataset: {}.",
            first_seen_at.to_rfc3339()
        ));
    }
    if let Some(last_seen_at) = profile.last_seen_at {
        evidence.push(format!(
            "Last seen in dataset: {}.",
            last_seen_at.to_rfc3339()
        ));
    }
    if recent_launches.is_empty() {
        evidence.push(
            "No prior launches are attached for this deployer beyond the current token."
                .to_string(),
        );
    } else {
        evidence.push(format!(
            "{} recent launch(es) are attached to this deployer for review.",
            recent_launches.len()
        ));
    }

    Ok(Some(DeployerMemorySummary {
        summary,
        evidence,
        trust_grade,
        trust_label,
        total_launches: profile.total_tokens_deployed,
        rug_count: profile.rug_count,
        graduated_count: profile.graduated_count,
        honeypot_history: profile.honeypot_detected,
        first_seen_at: profile.first_seen_at,
        last_seen_at: profile.last_seen_at,
        recent_launches,
    }))
}

pub async fn build_wallet_structure_summary(
    pool: &PgPool,
    token: &TokenSnapshot,
) -> Result<WalletStructureSummary> {
    let active_wallet_row: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT COUNT(DISTINCT wallet_address)::bigint
        FROM token_transactions
        WHERE LOWER(token_address) = LOWER($1)
        "#,
    )
    .bind(&token.contract_address)
    .fetch_optional(pool)
    .await?;

    let cluster_counts_row: Option<(i64, i64)> = sqlx::query_as(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE confidence = 'probable')::bigint AS probable_wallets,
            COUNT(*) FILTER (WHERE confidence = 'potential')::bigint AS potential_wallets
        FROM wallet_clusters
        WHERE LOWER(token_address) = LOWER($1)
        "#,
    )
    .bind(&token.contract_address)
    .fetch_optional(pool)
    .await?;

    let repeated_wallet_row: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT COUNT(*)::bigint
        FROM (
            SELECT current.wallet_address
            FROM wallet_clusters current
            JOIN tokens related_tokens
              ON LOWER(related_tokens.deployer_address) = LOWER($2)
             AND LOWER(related_tokens.contract_address) <> LOWER($1)
            JOIN wallet_clusters related
              ON LOWER(related.wallet_address) = LOWER(current.wallet_address)
             AND LOWER(related.token_address) = LOWER(related_tokens.contract_address)
            WHERE LOWER(current.token_address) = LOWER($1)
            GROUP BY current.wallet_address
        ) overlap_wallets
        "#,
    )
    .bind(&token.contract_address)
    .bind(&token.deployer_address)
    .fetch_optional(pool)
    .await?;

    let top_flow_rows: Vec<WalletFlowRow> = sqlx::query_as(
        r#"
        SELECT
            wallet_address,
            COALESCE(
                SUM(
                    CASE
                        WHEN tx_type = 'buy' THEN amount_bnb
                        ELSE -amount_bnb
                    END
                ),
                0
            )::double precision AS net_flow_bnb,
            COUNT(*)::bigint AS txn_count
        FROM token_transactions
        WHERE LOWER(token_address) = LOWER($1)
        GROUP BY wallet_address
        ORDER BY ABS(
            COALESCE(
                SUM(
                    CASE
                        WHEN tx_type = 'buy' THEN amount_bnb
                        ELSE -amount_bnb
                    END
                ),
                0
            )::double precision
        ) DESC
        LIMIT 5
        "#,
    )
    .bind(&token.contract_address)
    .fetch_all(pool)
    .await?;

    let active_wallet_count = active_wallet_row.map(|row| row.0).unwrap_or(0);
    let (probable_cluster_wallets, potential_cluster_wallets) =
        cluster_counts_row.unwrap_or((0, 0));
    let repeated_wallet_count = repeated_wallet_row.map(|row| row.0).unwrap_or(0);

    let concentration_label = if probable_cluster_wallets >= 5 || repeated_wallet_count >= 3 {
        "coordinated wallet behavior is materially visible"
    } else if active_wallet_count <= 10 && token.volume_bnb >= 5.0 {
        "trading participation looks concentrated for the observed volume"
    } else if active_wallet_count >= 30 && probable_cluster_wallets == 0 {
        "wallet participation looks broader and less obviously coordinated"
    } else {
        "wallet structure looks mixed and still needs caution"
    };

    let summary = format!(
        "Wallet structure shows {}: {} active wallet(s), {} indexed holder(s), {} probable cluster wallet(s), {} potential cluster wallet(s), and {} wallet(s) that reappear across this deployer's other launches.",
        concentration_label,
        active_wallet_count,
        token.holder_count,
        probable_cluster_wallets,
        potential_cluster_wallets,
        repeated_wallet_count
    );

    let mut evidence = vec![
        format!(
            "Active wallets in transaction history: {}.",
            active_wallet_count
        ),
        format!("Indexed holder count: {}.", token.holder_count),
        format!("Probable cluster wallets: {}.", probable_cluster_wallets),
        format!("Potential cluster wallets: {}.", potential_cluster_wallets),
        format!(
            "Repeated wallets across this deployer's other launches: {}.",
            repeated_wallet_count
        ),
        format!("Observed token volume: {:.2} BNB.", token.volume_bnb),
    ];

    let top_flow_wallets = top_flow_rows
        .iter()
        .map(|row| {
            format!(
                "{} | net flow {:.3} BNB | {} tx",
                row.wallet_address, row.net_flow_bnb, row.txn_count
            )
        })
        .collect::<Vec<_>>();

    if top_flow_wallets.is_empty() {
        evidence
            .push("No dominant flow wallets were recovered from transaction history.".to_string());
    } else {
        evidence.push(format!(
            "{} high-impact wallet(s) were ranked by net flow.",
            top_flow_wallets.len()
        ));
    }

    Ok(WalletStructureSummary {
        summary,
        evidence,
        active_wallet_count,
        participant_wallet_count: token.participant_wallet_count,
        holder_count: token.holder_count,
        probable_cluster_wallets,
        potential_cluster_wallets,
        repeated_wallet_count,
        top_flow_wallets,
    })
}

pub async fn build_operator_family_summary(
    pool: &PgPool,
    token: &TokenSnapshot,
) -> Result<OperatorFamilySummary> {
    let cluster_counts_row: Option<(i64, i64)> = sqlx::query_as(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE confidence = 'probable')::bigint AS probable_wallets,
            COUNT(*) FILTER (WHERE confidence = 'potential')::bigint AS potential_wallets
        FROM wallet_clusters
        WHERE LOWER(token_address) = LOWER($1)
        "#,
    )
    .bind(&token.contract_address)
    .fetch_optional(pool)
    .await?;

    let repeated_wallet_rows: Vec<OperatorWalletOverlapRow> = sqlx::query_as(
        r#"
        SELECT
            current.wallet_address,
            COUNT(DISTINCT related.token_address)::bigint AS overlap_launches,
            COUNT(DISTINCT related_tokens.deployer_address)::bigint AS overlap_deployers
        FROM wallet_clusters current
        JOIN wallet_clusters related
          ON LOWER(related.wallet_address) = LOWER(current.wallet_address)
         AND LOWER(related.token_address) <> LOWER(current.token_address)
        JOIN tokens related_tokens
          ON LOWER(related_tokens.contract_address) = LOWER(related.token_address)
        WHERE LOWER(current.token_address) = LOWER($1)
        GROUP BY current.wallet_address
        ORDER BY overlap_launches DESC, overlap_deployers DESC, current.wallet_address ASC
        LIMIT 8
        "#,
    )
    .bind(&token.contract_address)
    .fetch_all(pool)
    .await?;

    let related_launches: Vec<OperatorFamilyLaunchRef> = sqlx::query_as(
        r#"
        SELECT
            related.token_address AS contract_address,
            related_tokens.symbol,
            related_tokens.name,
            related_tokens.deployer_address,
            related_tokens.deployed_at,
            related_tokens.is_rug,
            related_tokens.graduated,
            COUNT(DISTINCT current.wallet_address)::bigint AS overlap_wallets
        FROM wallet_clusters current
        JOIN wallet_clusters related
          ON LOWER(related.wallet_address) = LOWER(current.wallet_address)
         AND LOWER(related.token_address) <> LOWER(current.token_address)
        JOIN tokens related_tokens
          ON LOWER(related_tokens.contract_address) = LOWER(related.token_address)
        WHERE LOWER(current.token_address) = LOWER($1)
        GROUP BY
            related.token_address,
            related_tokens.symbol,
            related_tokens.name,
            related_tokens.deployer_address,
            related_tokens.deployed_at,
            related_tokens.is_rug,
            related_tokens.graduated
        ORDER BY overlap_wallets DESC, related_tokens.deployed_at DESC
        LIMIT 5
        "#,
    )
    .bind(&token.contract_address)
    .fetch_all(pool)
    .await?;

    let seller_to_new_builder_rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT DISTINCT tx.wallet_address
        FROM token_transactions tx
        JOIN tokens related_tokens
          ON LOWER(related_tokens.deployer_address) = LOWER(tx.wallet_address)
         AND LOWER(related_tokens.contract_address) <> LOWER($1)
         AND related_tokens.deployed_at >= $2
        WHERE LOWER(tx.token_address) = LOWER($1)
          AND tx.tx_type = 'sell'
        ORDER BY tx.wallet_address ASC
        LIMIT 5
        "#,
    )
    .bind(&token.contract_address)
    .bind(token.deployed_at)
    .fetch_all(pool)
    .await?;

    let seller_reentry_rows: Vec<SellerMigrationRow> = sqlx::query_as(
        r#"
        SELECT
            tx.wallet_address,
            COUNT(DISTINCT related.token_address)::bigint AS future_launches
        FROM token_transactions tx
        JOIN wallet_clusters related
          ON LOWER(related.wallet_address) = LOWER(tx.wallet_address)
         AND LOWER(related.token_address) <> LOWER($1)
        JOIN tokens related_tokens
          ON LOWER(related_tokens.contract_address) = LOWER(related.token_address)
         AND related_tokens.deployed_at >= $2
        WHERE LOWER(tx.token_address) = LOWER($1)
          AND tx.tx_type = 'sell'
        GROUP BY tx.wallet_address
        ORDER BY future_launches DESC, tx.wallet_address ASC
        LIMIT 5
        "#,
    )
    .bind(&token.contract_address)
    .bind(token.deployed_at)
    .fetch_all(pool)
    .await?;

    let deployer_profile = get_deployer_profile(pool, &token.deployer_address).await?;
    let (probable_cluster_wallets, potential_cluster_wallets) =
        cluster_counts_row.unwrap_or((0, 0));
    let related_deployer_count = related_launches
        .iter()
        .map(|launch| launch.deployer_address.to_ascii_lowercase())
        .collect::<std::collections::BTreeSet<_>>()
        .len() as i64;

    let snapshot = OperatorFamilySnapshot {
        probable_cluster_wallets: probable_cluster_wallets.max(0) as usize,
        potential_cluster_wallets: potential_cluster_wallets.max(0) as usize,
        repeated_wallet_count: repeated_wallet_rows.len(),
        related_launch_count: related_launches.len(),
        related_deployer_count: related_deployer_count.max(0) as usize,
        seller_to_new_builder_count: seller_to_new_builder_rows.len(),
        seller_reentry_wallet_count: seller_reentry_rows.len(),
        deployer_rug_count: deployer_profile
            .as_ref()
            .map(|profile| profile.rug_count.max(0) as usize)
            .unwrap_or(0),
    };

    let confidence = score_operator_family_confidence(&snapshot);
    let signal_score = operator_family_signal_score(&snapshot);
    let safety_score = (100 - signal_score).clamp(0, 100);

    let repeated_wallets = repeated_wallet_rows
        .iter()
        .map(|row| {
            format!(
                "{} | {} linked launch(es) | {} deployer wallet(s)",
                row.wallet_address, row.overlap_launches, row.overlap_deployers
            )
        })
        .collect::<Vec<_>>();

    let migrated_wallets = seller_reentry_rows
        .iter()
        .map(|row| {
            format!(
                "{} | {} later launch(es)",
                row.wallet_address, row.future_launches
            )
        })
        .collect::<Vec<_>>();

    let mut evidence = vec![
        format!(
            "Probable cluster wallets on this launch: {}.",
            probable_cluster_wallets
        ),
        format!(
            "Potential cluster wallets on this launch: {}.",
            potential_cluster_wallets
        ),
        format!(
            "Repeated early wallets across other launches: {}.",
            repeated_wallet_rows.len()
        ),
        format!(
            "Related launches connected by wallet overlap: {}.",
            related_launches.len()
        ),
        format!(
            "Related deployer wallets connected by overlap: {}.",
            related_deployer_count
        ),
        format!(
            "Seller wallets that later became deployer wallets: {}.",
            seller_to_new_builder_rows.len()
        ),
        format!(
            "Seller wallets that later re-entered other launches: {}.",
            seller_reentry_rows.len()
        ),
    ];
    if let Some(profile) = &deployer_profile {
        evidence.push(format!(
            "Current deployer has {} recorded rug-flagged launch(es).",
            profile.rug_count
        ));
    }

    if related_launches.is_empty() {
        evidence.push(
            "No strong cross-launch overlap has been recovered from current indexed wallet-cluster data."
                .to_string(),
        );
    }

    Ok(OperatorFamilySummary {
        confidence: confidence.as_str().to_string(),
        summary: build_operator_family_summary_text(&snapshot, confidence),
        evidence,
        safety_score,
        signal_score,
        related_launch_count: related_launches.len() as i64,
        related_deployer_count,
        repeated_wallet_count: repeated_wallet_rows.len() as i64,
        seller_to_new_builder_count: seller_to_new_builder_rows.len() as i64,
        seller_reentry_wallet_count: seller_reentry_rows.len() as i64,
        probable_cluster_wallets,
        potential_cluster_wallets,
        repeated_wallets,
        migrated_wallets,
        related_launches,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        build_operator_family_summary_text, score_operator_family_confidence,
        OperatorFamilyConfidence, OperatorFamilySnapshot,
    };

    #[test]
    fn low_signal_snapshot_scores_low_confidence() {
        let snapshot = OperatorFamilySnapshot {
            probable_cluster_wallets: 0,
            potential_cluster_wallets: 1,
            repeated_wallet_count: 1,
            related_launch_count: 1,
            related_deployer_count: 0,
            seller_to_new_builder_count: 0,
            seller_reentry_wallet_count: 0,
            deployer_rug_count: 0,
        };

        assert_eq!(
            score_operator_family_confidence(&snapshot),
            OperatorFamilyConfidence::Low
        );
    }

    #[test]
    fn repeated_wallets_across_multiple_deployers_score_high() {
        let snapshot = OperatorFamilySnapshot {
            probable_cluster_wallets: 6,
            potential_cluster_wallets: 0,
            repeated_wallet_count: 4,
            related_launch_count: 5,
            related_deployer_count: 3,
            seller_to_new_builder_count: 1,
            seller_reentry_wallet_count: 2,
            deployer_rug_count: 1,
        };

        assert_eq!(
            score_operator_family_confidence(&snapshot),
            OperatorFamilyConfidence::High
        );
        let summary = build_operator_family_summary_text(&snapshot, OperatorFamilyConfidence::High);
        assert!(summary.contains("operator-family"));
        assert!(summary.contains("pattern-risk detection"));
    }
}
