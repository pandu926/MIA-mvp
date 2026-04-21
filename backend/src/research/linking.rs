use anyhow::Result;
use serde::Serialize;
use sqlx::{FromRow, PgPool};

use crate::indexer::deployer::get_deployer_profile;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum LinkConfidence {
    Low,
    Medium,
    High,
}

impl LinkConfidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkConfidence::Low => "low",
            LinkConfidence::Medium => "medium",
            LinkConfidence::High => "high",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkEvidenceSnapshot {
    pub probable_cluster_wallets: usize,
    pub potential_cluster_wallets: usize,
    pub repeated_wallet_count: usize,
    pub prior_deployer_launches: usize,
    pub deployer_rug_count: usize,
    pub honeypot_history: bool,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct LinkedTokenRef {
    pub contract_address: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub is_rug: bool,
    pub graduated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkedLaunchSummary {
    pub confidence: String,
    pub summary: String,
    pub evidence: Vec<String>,
    pub related_tokens: Vec<LinkedTokenRef>,
    pub repeated_wallets: Vec<String>,
}

pub fn score_link_confidence(snapshot: &LinkEvidenceSnapshot) -> LinkConfidence {
    let mut score = 0u8;

    if snapshot.probable_cluster_wallets >= 5 {
        score += 3;
    } else if snapshot.potential_cluster_wallets >= 3 {
        score += 1;
    }

    if snapshot.repeated_wallet_count >= 3 {
        score += 2;
    } else if snapshot.repeated_wallet_count >= 1 {
        score += 1;
    }

    if snapshot.prior_deployer_launches >= 3 {
        score += 2;
    } else if snapshot.prior_deployer_launches >= 1 {
        score += 1;
    }

    if snapshot.deployer_rug_count >= 2 {
        score += 1;
    }

    if snapshot.honeypot_history {
        score += 1;
    }

    match score {
        0..=1 => LinkConfidence::Low,
        2..=4 => LinkConfidence::Medium,
        _ => LinkConfidence::High,
    }
}

pub fn build_pattern_summary(
    snapshot: &LinkEvidenceSnapshot,
    confidence: LinkConfidence,
) -> String {
    let cluster_wallets = snapshot
        .probable_cluster_wallets
        .max(snapshot.potential_cluster_wallets);
    let cluster_phrase = if cluster_wallets >= 5 {
        format!("{cluster_wallets} early wallets moved inside a probable cluster")
    } else if cluster_wallets >= 3 {
        format!("{cluster_wallets} early wallets moved inside a potential cluster")
    } else {
        "no strong early-wallet cluster was recovered yet".to_string()
    };

    let repeat_phrase = if snapshot.repeated_wallet_count > 0 {
        format!(
            "{} wallet(s) reappeared across this deployer's other launches",
            snapshot.repeated_wallet_count
        )
    } else {
        "no repeated wallet overlap was recovered across prior launches".to_string()
    };

    let deployer_phrase = if snapshot.prior_deployer_launches > 0 {
        format!(
            "the deployer has {} prior launch(es) in MIA's dataset",
            snapshot.prior_deployer_launches
        )
    } else {
        "the deployer has no prior launch history in MIA's dataset".to_string()
    };

    let qualifier = match confidence {
        LinkConfidence::Low => "MIA sees a low-confidence likely linked launch cluster",
        LinkConfidence::Medium => "MIA sees a medium-confidence likely linked launch cluster",
        LinkConfidence::High => "MIA sees a high-confidence likely linked launch cluster",
    };

    format!(
        "{qualifier}: {cluster_phrase}; {repeat_phrase}; {deployer_phrase}. This is pattern detection for repeat launch pattern review, not an identity claim."
    )
}

pub async fn build_linked_launch_summary(
    pool: &PgPool,
    token_address: &str,
) -> Result<Option<LinkedLaunchSummary>> {
    let deployer_row: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT deployer_address
        FROM tokens
        WHERE LOWER(contract_address) = LOWER($1)
        LIMIT 1
        "#,
    )
    .bind(token_address)
    .fetch_optional(pool)
    .await?;

    let Some((deployer_address,)) = deployer_row else {
        return Ok(None);
    };

    let cluster_row: Option<(String, i64)> = sqlx::query_as(
        r#"
        SELECT confidence, COUNT(*)::bigint AS wallet_count
        FROM wallet_clusters
        WHERE LOWER(token_address) = LOWER($1)
        GROUP BY cluster_id, confidence
        ORDER BY wallet_count DESC
        LIMIT 1
        "#,
    )
    .bind(token_address)
    .fetch_optional(pool)
    .await?;

    let repeated_wallet_rows: Vec<(String, i64)> = sqlx::query_as(
        r#"
        SELECT current.wallet_address, COUNT(DISTINCT related.token_address)::bigint AS overlap_count
        FROM wallet_clusters current
        JOIN tokens base
          ON LOWER(base.contract_address) = LOWER(current.token_address)
        JOIN tokens related_tokens
          ON LOWER(related_tokens.deployer_address) = LOWER(base.deployer_address)
         AND LOWER(related_tokens.contract_address) <> LOWER(base.contract_address)
        JOIN wallet_clusters related
          ON LOWER(related.wallet_address) = LOWER(current.wallet_address)
         AND LOWER(related.token_address) = LOWER(related_tokens.contract_address)
        WHERE LOWER(current.token_address) = LOWER($1)
        GROUP BY current.wallet_address
        ORDER BY overlap_count DESC, current.wallet_address ASC
        LIMIT 5
        "#,
    )
    .bind(token_address)
    .fetch_all(pool)
    .await?;

    let related_tokens: Vec<LinkedTokenRef> = sqlx::query_as(
        r#"
        SELECT contract_address, symbol, name, is_rug, graduated
        FROM tokens
        WHERE LOWER(deployer_address) = LOWER($1)
          AND LOWER(contract_address) <> LOWER($2)
        ORDER BY deployed_at DESC
        LIMIT 3
        "#,
    )
    .bind(&deployer_address)
    .bind(token_address)
    .fetch_all(pool)
    .await?;

    let deployer_profile = get_deployer_profile(pool, &deployer_address).await?;
    let snapshot = LinkEvidenceSnapshot {
        probable_cluster_wallets: cluster_row
            .as_ref()
            .filter(|(confidence, _)| confidence.eq_ignore_ascii_case("probable"))
            .map(|(_, wallet_count)| *wallet_count as usize)
            .unwrap_or(0),
        potential_cluster_wallets: cluster_row
            .as_ref()
            .filter(|(confidence, _)| confidence.eq_ignore_ascii_case("potential"))
            .map(|(_, wallet_count)| *wallet_count as usize)
            .unwrap_or(0),
        repeated_wallet_count: repeated_wallet_rows.len(),
        prior_deployer_launches: related_tokens.len(),
        deployer_rug_count: deployer_profile
            .as_ref()
            .map(|profile| profile.rug_count.max(0) as usize)
            .unwrap_or(0),
        honeypot_history: deployer_profile
            .as_ref()
            .map(|profile| profile.honeypot_detected)
            .unwrap_or(false),
    };
    let confidence = score_link_confidence(&snapshot);

    let mut evidence = Vec::new();
    if snapshot.probable_cluster_wallets > 0 {
        evidence.push(format!(
            "Probable early-wallet cluster with {} wallet(s).",
            snapshot.probable_cluster_wallets
        ));
    } else if snapshot.potential_cluster_wallets > 0 {
        evidence.push(format!(
            "Potential early-wallet cluster with {} wallet(s).",
            snapshot.potential_cluster_wallets
        ));
    } else {
        evidence.push(
            "No strong early-wallet cluster has been persisted for this token yet.".to_string(),
        );
    }

    if snapshot.repeated_wallet_count > 0 {
        evidence.push(format!(
            "{} wallet(s) overlap with the same deployer's prior launches.",
            snapshot.repeated_wallet_count
        ));
    } else {
        evidence.push(
            "No repeated wallet overlap was recovered across this deployer's prior launches."
                .to_string(),
        );
    }

    if snapshot.prior_deployer_launches > 0 {
        evidence.push(format!(
            "Deployer has {} prior launch(es) recorded by MIA.",
            snapshot.prior_deployer_launches
        ));
    } else {
        evidence.push("Deployer has no prior launch history in MIA's dataset.".to_string());
    }

    if snapshot.deployer_rug_count > 0 || snapshot.honeypot_history {
        evidence.push(format!(
            "Historical deployer risk: {} rug flag(s), honeypot history = {}.",
            snapshot.deployer_rug_count, snapshot.honeypot_history
        ));
    }

    Ok(Some(LinkedLaunchSummary {
        confidence: confidence.as_str().to_string(),
        summary: build_pattern_summary(&snapshot, confidence),
        evidence,
        related_tokens,
        repeated_wallets: repeated_wallet_rows
            .into_iter()
            .map(|(wallet, _)| wallet)
            .collect(),
    }))
}

#[cfg(test)]
mod tests {
    use super::{
        build_pattern_summary, score_link_confidence, LinkConfidence, LinkEvidenceSnapshot,
    };

    #[test]
    fn low_signal_snapshot_scores_low_confidence() {
        let snapshot = LinkEvidenceSnapshot {
            probable_cluster_wallets: 0,
            potential_cluster_wallets: 2,
            repeated_wallet_count: 0,
            prior_deployer_launches: 0,
            deployer_rug_count: 0,
            honeypot_history: false,
        };

        assert_eq!(score_link_confidence(&snapshot), LinkConfidence::Low);
    }

    #[test]
    fn repeated_wallets_and_prior_launches_score_medium_confidence() {
        let snapshot = LinkEvidenceSnapshot {
            probable_cluster_wallets: 0,
            potential_cluster_wallets: 4,
            repeated_wallet_count: 2,
            prior_deployer_launches: 2,
            deployer_rug_count: 0,
            honeypot_history: false,
        };

        assert_eq!(score_link_confidence(&snapshot), LinkConfidence::Medium);
    }

    #[test]
    fn probable_cluster_with_repeat_history_scores_high_confidence() {
        let snapshot = LinkEvidenceSnapshot {
            probable_cluster_wallets: 6,
            potential_cluster_wallets: 0,
            repeated_wallet_count: 3,
            prior_deployer_launches: 4,
            deployer_rug_count: 2,
            honeypot_history: false,
        };

        assert_eq!(score_link_confidence(&snapshot), LinkConfidence::High);
    }

    #[test]
    fn summary_uses_legal_safe_wording() {
        let snapshot = LinkEvidenceSnapshot {
            probable_cluster_wallets: 5,
            potential_cluster_wallets: 0,
            repeated_wallet_count: 2,
            prior_deployer_launches: 3,
            deployer_rug_count: 1,
            honeypot_history: false,
        };

        let summary = build_pattern_summary(&snapshot, LinkConfidence::High);

        assert!(summary.contains("likely linked launch cluster"));
        assert!(summary.contains("repeat launch pattern"));
        assert!(!summary.contains("confirmed"));
        assert!(!summary.contains("scammer"));
    }
}
