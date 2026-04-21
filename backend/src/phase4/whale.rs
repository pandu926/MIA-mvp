use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize)]
pub struct WhaleAlert {
    pub token_address: String,
    pub wallet_address: String,
    pub tx_hash: String,
    pub amount_bnb: f64,
    pub threshold_bnb: f64,
    /// "watch" for >= configured threshold, "critical" for >= 1 BNB.
    pub alert_level: String,
    pub created_at: DateTime<Utc>,
}

pub fn classify_whale_trade(amount_bnb: f64, threshold_bnb: f64) -> Option<&'static str> {
    if amount_bnb < threshold_bnb {
        return None;
    }
    if amount_bnb >= 1.0 {
        Some("critical")
    } else {
        Some("watch")
    }
}

pub async fn upsert_whale_alert(
    db: &PgPool,
    token_address: &str,
    wallet_address: &str,
    tx_hash: &str,
    amount_bnb: f64,
    threshold_bnb: f64,
    created_at: DateTime<Utc>,
) -> Result<Option<WhaleAlert>> {
    let Some(alert_level) = classify_whale_trade(amount_bnb, threshold_bnb) else {
        return Ok(None);
    };

    let inserted = sqlx::query(
        r#"
        INSERT INTO whale_alerts
            (token_address, wallet_address, tx_hash, amount_bnb, threshold_bnb, alert_level, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (tx_hash) DO NOTHING
        "#,
    )
    .bind(token_address)
    .bind(wallet_address)
    .bind(tx_hash)
    .bind(amount_bnb)
    .bind(threshold_bnb)
    .bind(alert_level)
    .bind(created_at)
    .execute(db)
    .await?;

    if inserted.rows_affected() == 0 {
        return Ok(None);
    }

    Ok(Some(WhaleAlert {
        token_address: token_address.to_string(),
        wallet_address: wallet_address.to_string(),
        tx_hash: tx_hash.to_string(),
        amount_bnb,
        threshold_bnb,
        alert_level: alert_level.to_string(),
        created_at,
    }))
}

#[cfg(test)]
mod tests {
    use super::classify_whale_trade;

    #[test]
    fn classify_below_threshold_is_none() {
        assert_eq!(classify_whale_trade(0.49, 0.5), None);
    }

    #[test]
    fn classify_between_threshold_and_one_is_watch() {
        assert_eq!(classify_whale_trade(0.6, 0.5), Some("watch"));
    }

    #[test]
    fn classify_one_or_more_is_critical() {
        assert_eq!(classify_whale_trade(1.0, 0.5), Some("critical"));
        assert_eq!(classify_whale_trade(2.5, 0.5), Some("critical"));
    }
}
