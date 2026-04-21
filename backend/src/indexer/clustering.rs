use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Known exchange hot-wallet addresses excluded from clustering.
/// These wallets appear in many tokens' early transactions but are NOT coordinated pumpers.
const EXCHANGE_ALLOWLIST: &[&str] = &[
    "0x3fc91a3afd70395cd496c647d5a6cc9d4b2b7fad", // Uniswap Universal Router
    "0x13f4ea83d0bd40e75c8222255bc855a974568dd4", // PancakeSwap Router v3
    "0x10ed43c718714eb63d5aa57b78b54704e256024e", // PancakeSwap Router v2
];

const MIN_CLUSTER_SIZE: usize = 3;

/// A single wallet's first buy activity for a token.
#[derive(Debug, Clone)]
pub struct WalletActivity {
    pub wallet_address: String,
    pub first_buy_at: DateTime<Utc>,
}

/// Confidence level for a detected co-moving wallet cluster.
#[derive(Debug, Clone, PartialEq)]
pub enum ClusterConfidence {
    /// 3–4 wallets co-moving within window
    Potential,
    /// 5+ wallets co-moving within window
    Probable,
}

impl ClusterConfidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClusterConfidence::Potential => "potential",
            ClusterConfidence::Probable => "probable",
        }
    }
}

/// A detected group of potentially coordinated wallets.
#[derive(Debug, Clone)]
pub struct WalletCluster {
    pub cluster_id: Uuid,
    pub members: Vec<String>,
    pub confidence: ClusterConfidence,
}

/// Pure function: group wallet activities into co-moving clusters.
///
/// Algorithm:
/// 1. Exclude known exchange wallets
/// 2. Sort by `first_buy_at`
/// 3. Sliding window: any wallet whose first buy is within `window_secs` of
///    the cluster's earliest buy is added to the cluster
/// 4. Only emit clusters with `>= min_cluster_size` members
///
/// This is conservative: false positives are worse than false negatives
/// for user trust. Label is "Potentially related", never "Confirmed".
pub fn cluster_wallets(
    activities: &[WalletActivity],
    window_secs: u64,
    min_cluster_size: usize,
) -> Vec<WalletCluster> {
    // Filter out known exchange wallets
    let filtered: Vec<&WalletActivity> = activities
        .iter()
        .filter(|a| {
            !EXCHANGE_ALLOWLIST
                .iter()
                .any(|ex| ex.eq_ignore_ascii_case(&a.wallet_address))
        })
        .collect();

    if filtered.len() < min_cluster_size {
        return vec![];
    }

    // Sort by first buy timestamp (ascending)
    let mut sorted = filtered.clone();
    sorted.sort_by_key(|a| a.first_buy_at);

    let window = chrono::Duration::seconds(window_secs as i64);
    let mut clusters: Vec<WalletCluster> = vec![];
    let mut used: Vec<bool> = vec![false; sorted.len()];

    for i in 0..sorted.len() {
        if used[i] {
            continue;
        }

        let anchor = sorted[i].first_buy_at;
        let mut members = vec![sorted[i].wallet_address.clone()];
        used[i] = true;

        for j in (i + 1)..sorted.len() {
            if used[j] {
                continue;
            }
            if sorted[j].first_buy_at - anchor <= window {
                members.push(sorted[j].wallet_address.clone());
                used[j] = true;
            }
        }

        if members.len() >= min_cluster_size {
            let confidence = if members.len() >= 5 {
                ClusterConfidence::Probable
            } else {
                ClusterConfidence::Potential
            };

            clusters.push(WalletCluster {
                cluster_id: Uuid::new_v4(),
                members,
                confidence,
            });
        }
    }

    clusters
}

/// Fetch wallet activities from DB and run clustering for a token.
pub async fn detect_clusters(
    pool: &PgPool,
    token_address: &str,
    window_secs: u64,
) -> Result<Vec<WalletCluster>> {
    // Fetch first buy time per wallet for this token within the first 10 minutes
    let rows: Vec<(String, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT wallet_address, MIN(created_at) AS first_buy_at
        FROM token_transactions
        WHERE token_address = $1
          AND tx_type = 'buy'
          AND created_at <= (
              SELECT MIN(created_at) + INTERVAL '10 minutes'
              FROM token_transactions
              WHERE token_address = $1 AND tx_type = 'buy'
          )
        GROUP BY wallet_address
        "#,
    )
    .bind(token_address)
    .fetch_all(pool)
    .await?;

    let activities: Vec<WalletActivity> = rows
        .into_iter()
        .map(|(wallet_address, first_buy_at)| WalletActivity {
            wallet_address,
            first_buy_at,
        })
        .collect();

    Ok(cluster_wallets(&activities, window_secs, MIN_CLUSTER_SIZE))
}

/// Persist detected clusters to the database.
pub async fn save_clusters(
    pool: &PgPool,
    token_address: &str,
    clusters: &[WalletCluster],
) -> Result<()> {
    for cluster in clusters {
        for wallet in &cluster.members {
            sqlx::query(
                r#"
                INSERT INTO wallet_clusters (token_address, wallet_address, cluster_id, confidence)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (token_address, wallet_address) DO UPDATE
                    SET cluster_id = EXCLUDED.cluster_id,
                        confidence = EXCLUDED.confidence
                "#,
            )
            .bind(token_address)
            .bind(wallet)
            .bind(cluster.cluster_id)
            .bind(cluster.confidence.as_str())
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// TDD Tests — all tests on `cluster_wallets` (pure function, no DB needed)
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn make_activity(wallet: &str, offset_secs: i64) -> WalletActivity {
        WalletActivity {
            wallet_address: wallet.to_string(),
            first_buy_at: Utc::now() + Duration::seconds(offset_secs),
        }
    }

    // RED → GREEN: fewer than min_cluster_size wallets → no clusters
    #[test]
    fn two_wallets_do_not_form_cluster() {
        let activities = vec![make_activity("0xA", 0), make_activity("0xB", 10)];
        let clusters = cluster_wallets(&activities, 60, 3);
        assert!(clusters.is_empty(), "2 wallets should not form a cluster");
    }

    // RED → GREEN: 3 wallets within window → 1 cluster (Potential)
    #[test]
    fn three_wallets_within_window_form_potential_cluster() {
        let activities = vec![
            make_activity("0xA", 0),
            make_activity("0xB", 20),
            make_activity("0xC", 40),
        ];
        let clusters = cluster_wallets(&activities, 60, 3);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].members.len(), 3);
        assert_eq!(clusters[0].confidence, ClusterConfidence::Potential);
    }

    // RED → GREEN: 5+ wallets within window → Probable confidence
    #[test]
    fn five_wallets_within_window_are_probable() {
        let activities = vec![
            make_activity("0xA", 0),
            make_activity("0xB", 10),
            make_activity("0xC", 20),
            make_activity("0xD", 30),
            make_activity("0xE", 40),
        ];
        let clusters = cluster_wallets(&activities, 60, 3);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].confidence, ClusterConfidence::Probable);
    }

    // RED → GREEN: wallets spread across two separate windows → two clusters
    #[test]
    fn wallets_in_different_windows_form_separate_clusters() {
        let activities = vec![
            make_activity("0xA", 0),
            make_activity("0xB", 30),
            make_activity("0xC", 50),  // cluster 1
            make_activity("0xD", 300), // cluster 2 starts
            make_activity("0xE", 320),
            make_activity("0xF", 340),
        ];
        let clusters = cluster_wallets(&activities, 60, 3);
        assert_eq!(clusters.len(), 2, "Should produce 2 separate clusters");
    }

    // RED → GREEN: exchange wallet is excluded from clusters
    #[test]
    fn exchange_wallet_excluded_from_clustering() {
        let activities = vec![
            make_activity(EXCHANGE_ALLOWLIST[1], 0), // PancakeSwap Router — excluded
            make_activity("0xB", 10),
            make_activity("0xC", 20),
            make_activity("0xD", 30),
        ];
        let clusters = cluster_wallets(&activities, 60, 3);
        // Without exchange wallet we still have 3 → 1 cluster, exchange not in members
        if !clusters.is_empty() {
            for c in &clusters {
                assert!(
                    !c.members
                        .iter()
                        .any(|m| m.eq_ignore_ascii_case(EXCHANGE_ALLOWLIST[1])),
                    "Exchange wallet must not appear in cluster members"
                );
            }
        }
    }

    // RED → GREEN: each cluster gets a unique ID
    #[test]
    fn clusters_have_unique_ids() {
        let activities = vec![
            make_activity("0xA", 0),
            make_activity("0xB", 10),
            make_activity("0xC", 20),
            make_activity("0xD", 300),
            make_activity("0xE", 310),
            make_activity("0xF", 320),
        ];
        let clusters = cluster_wallets(&activities, 60, 3);
        assert_eq!(clusters.len(), 2);
        assert_ne!(
            clusters[0].cluster_id, clusters[1].cluster_id,
            "Each cluster must have a unique ID"
        );
    }

    // RED → GREEN: empty input → no clusters
    #[test]
    fn empty_activities_returns_no_clusters() {
        let clusters = cluster_wallets(&[], 60, 3);
        assert!(clusters.is_empty());
    }

    // RED → GREEN: wallets just outside window → not clustered together
    #[test]
    fn wallet_outside_window_not_clustered() {
        let activities = vec![
            make_activity("0xA", 0),
            make_activity("0xB", 30),
            make_activity("0xC", 61), // just outside 60s window
        ];
        let clusters = cluster_wallets(&activities, 60, 3);
        assert!(
            clusters.is_empty(),
            "0xC is outside window — no cluster of 3"
        );
    }

    // RED → GREEN: confidence string representation
    #[test]
    fn confidence_as_str_values() {
        assert_eq!(ClusterConfidence::Potential.as_str(), "potential");
        assert_eq!(ClusterConfidence::Probable.as_str(), "probable");
    }
}
