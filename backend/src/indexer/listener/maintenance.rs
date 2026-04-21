use super::*;

impl BlockListener {
    pub(super) async fn refresh_token_metrics(&self, token_address: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE tokens t
            SET
                buy_count = agg.buy_count,
                sell_count = agg.sell_count,
                volume_bnb = agg.volume_bnb,
                holder_count = agg.holder_count,
                updated_at = NOW()
            FROM (
                SELECT
                    COUNT(*) FILTER (WHERE tx_type = 'buy')::int AS buy_count,
                    COUNT(*) FILTER (WHERE tx_type = 'sell')::int AS sell_count,
                    COALESCE(SUM(amount_bnb), 0)::numeric AS volume_bnb,
                    COUNT(DISTINCT wallet_address)::int AS holder_count
                FROM token_transactions
                WHERE token_address = $1
            ) AS agg
            WHERE t.contract_address = $1
            "#,
        )
        .bind(token_address)
        .execute(&self.db_pool)
        .await?;
        Ok(())
    }

    pub(super) async fn get_deployer_for_token(&self, token_address: &str) -> Result<String> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT deployer_address FROM tokens WHERE contract_address = $1")
                .bind(token_address)
                .fetch_optional(&self.db_pool)
                .await?;

        Ok(row
            .map(|(d,)| d)
            .unwrap_or_else(|| "0x0000000000000000000000000000000000000000".to_string()))
    }

    pub(super) async fn broadcast_token_snapshot(&self, token_address: &str) -> Result<()> {
        let row: Option<(String, DateTime<chrono::Utc>, i32, i32, f64, Option<i16>)> =
            sqlx::query_as(
                r#"
                SELECT t.deployer_address, t.deployed_at, t.buy_count, t.sell_count,
                       t.volume_bnb::double precision, rs.composite_score
                FROM tokens t
                LEFT JOIN risk_scores rs ON rs.token_address = t.contract_address
                WHERE t.contract_address = $1
                "#,
            )
            .bind(token_address)
            .fetch_optional(&self.db_pool)
            .await?;

        let Some((
            deployer_address,
            deployed_at,
            buy_count,
            sell_count,
            volume_bnb,
            composite_score,
        )) = row
        else {
            return Ok(());
        };

        let risk_category = composite_score.map(|s| {
            match s {
                0..=33 => "low",
                34..=66 => "medium",
                _ => "high",
            }
            .to_string()
        });

        self.ws_hub
            .broadcast(WsMessage::TokenUpdate {
                token_address: token_address.to_string(),
                name: None,
                symbol: None,
                deployer_address,
                buy_count,
                sell_count,
                volume_bnb,
                composite_score,
                risk_category,
                deployed_at: deployed_at.to_rfc3339(),
            })
            .await;
        Ok(())
    }

    pub(super) async fn update_risk_score(&self, token_address: &str) {
        let row: Option<(i32, i32, i32, bool, String)> = sqlx::query_as(
            "SELECT buy_count, sell_count, holder_count, honeypot_detected, deployer_address FROM tokens WHERE contract_address = $1",
        )
        .bind(token_address)
        .fetch_optional(&self.db_pool)
        .await
        .ok()
        .flatten();

        let Some((buy_count, sell_count, holder_count, honeypot_detected, deployer_address)) = row
        else {
            return;
        };

        let deployer_history: Option<(i64, i64, i64)> = sqlx::query_as(
            r#"
            SELECT COUNT(*),
                   SUM(CASE WHEN is_rug    THEN 1 ELSE 0 END),
                   SUM(CASE WHEN graduated THEN 1 ELSE 0 END)
            FROM tokens WHERE deployer_address = $1
            "#,
        )
        .bind(&deployer_address)
        .fetch_optional(&self.db_pool)
        .await
        .ok()
        .flatten();

        let (total, rug_count, grad_count) = deployer_history.unwrap_or((1, 0, 0));
        let deployer_history_score = if total <= 1 && rug_count == 0 && grad_count == 0 {
            35
        } else {
            risk_signals::deployer_history_score(rug_count as u32, grad_count as u32)
        };

        let total_tx = buy_count.saturating_add(sell_count);
        let top_holder_concentration_pct = self
            .compute_top_holder_concentration_pct(token_address)
            .await
            .or_else(|| {
                if holder_count <= 3 && total_tx >= 10 {
                    Some(95.0)
                } else if holder_count <= 10 && total_tx >= 20 {
                    Some(80.0)
                } else {
                    None
                }
            })
            .unwrap_or(50.0);
        let recent_volumes = self.recent_volume_buckets(token_address, 60, 30).await;
        let liquidity_locked_pct = 0.0;

        let wallet_concentration_score =
            risk_signals::wallet_concentration_score(top_holder_concentration_pct);
        let buy_sell_velocity_score =
            risk_signals::buy_sell_velocity_score(buy_count as u64, sell_count as u64);
        let contract_audit_score =
            risk_signals::contract_audit_score(honeypot_detected, false, false);
        let volume_consistency_score = risk_signals::volume_consistency_score(&recent_volumes);

        let signals = risk_scorer::RiskSignals {
            deployer_history: deployer_history_score,
            liquidity_lock: risk_signals::liquidity_lock_score(liquidity_locked_pct),
            wallet_concentration: wallet_concentration_score,
            buy_sell_velocity: buy_sell_velocity_score,
            contract_audit: contract_audit_score,
            social_authenticity: risk_signals::social_authenticity_score(None),
            volume_consistency: volume_consistency_score,
        };
        let mut composite = risk_scorer::compute_composite_score(&signals);
        if honeypot_detected {
            composite = composite.max(90);
        }
        if wallet_concentration_score >= 85 && buy_sell_velocity_score >= 60 {
            composite = composite.max(72);
        }
        if wallet_concentration_score >= 90 && volume_consistency_score >= 70 {
            composite = composite.max(75);
        }
        if deployer_history_score >= 70 && buy_sell_velocity_score >= 70 {
            composite = composite.max(80);
        }
        let composite = composite as i16;

        let result = sqlx::query(
            r#"
            INSERT INTO risk_scores
                (token_address, composite_score, deployer_history_score, liquidity_lock_score,
                 wallet_concentration_score, buy_sell_velocity_score, contract_audit_score,
                 social_authenticity_score, volume_consistency_score)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (token_address) DO UPDATE SET
                composite_score            = EXCLUDED.composite_score,
                deployer_history_score     = EXCLUDED.deployer_history_score,
                liquidity_lock_score       = EXCLUDED.liquidity_lock_score,
                wallet_concentration_score = EXCLUDED.wallet_concentration_score,
                buy_sell_velocity_score    = EXCLUDED.buy_sell_velocity_score,
                contract_audit_score       = EXCLUDED.contract_audit_score,
                social_authenticity_score  = EXCLUDED.social_authenticity_score,
                volume_consistency_score   = EXCLUDED.volume_consistency_score,
                computed_at                = NOW()
            "#,
        )
        .bind(token_address)
        .bind(composite)
        .bind(deployer_history_score as i16)
        .bind(signals.liquidity_lock as i16)
        .bind(wallet_concentration_score as i16)
        .bind(buy_sell_velocity_score as i16)
        .bind(contract_audit_score as i16)
        .bind(signals.social_authenticity as i16)
        .bind(volume_consistency_score as i16)
        .execute(&self.db_pool)
        .await;

        if let Err(e) = result {
            tracing::warn!(token = %token_address, "Failed to upsert risk_score: {}", e);
        }
    }

    pub(super) async fn compute_top_holder_concentration_pct(
        &self,
        token_address: &str,
    ) -> Option<f64> {
        let row: Option<(f64, f64)> = sqlx::query_as(
            r#"
            WITH wallet_buy AS (
                SELECT wallet_address, SUM(amount_bnb)::double precision AS buy_volume
                FROM token_transactions
                WHERE token_address = $1
                  AND tx_type = 'buy'
                  AND amount_bnb > 0
                GROUP BY wallet_address
            ),
            top_wallets AS (
                SELECT buy_volume
                FROM wallet_buy
                ORDER BY buy_volume DESC
                LIMIT 10
            )
            SELECT
                COALESCE((SELECT SUM(buy_volume) FROM top_wallets), 0)::double precision AS top_10_volume,
                COALESCE((SELECT SUM(buy_volume) FROM wallet_buy), 0)::double precision AS total_volume
            "#,
        )
        .bind(token_address)
        .fetch_optional(&self.db_pool)
        .await
        .ok()
        .flatten();

        let Some((top_10_volume, total_volume)) = row else {
            return None;
        };
        if total_volume <= 0.0 {
            return None;
        }
        Some(((top_10_volume / total_volume) * 100.0).clamp(0.0, 100.0))
    }

    pub(super) async fn recent_volume_buckets(
        &self,
        token_address: &str,
        minutes: i64,
        max_points: i64,
    ) -> Vec<f64> {
        let rows: Vec<(f64,)> = sqlx::query_as(
            r#"
            SELECT SUM(amount_bnb)::double precision AS bucket_volume
            FROM token_transactions
            WHERE token_address = $1
              AND created_at >= NOW() - ($2 * INTERVAL '1 minute')
            GROUP BY date_trunc('minute', created_at)
            ORDER BY date_trunc('minute', created_at) DESC
            LIMIT $3
            "#,
        )
        .bind(token_address)
        .bind(minutes.max(1))
        .bind(max_points.clamp(3, 120))
        .fetch_all(&self.db_pool)
        .await
        .unwrap_or_default();

        rows.into_iter().map(|(v,)| v).collect()
    }

    pub(super) async fn update_indexer_state(&self, block_number: u64) -> Result<()> {
        sqlx::query(
            "UPDATE indexer_state SET last_processed_block = $1, updated_at = NOW() WHERE id = 1",
        )
        .bind(block_number as i64)
        .execute(&self.db_pool)
        .await?;
        Ok(())
    }

    pub(super) async fn update_indexer_status(&self, status: &str) -> Result<()> {
        sqlx::query(
            "UPDATE indexer_state SET indexer_status = $1, updated_at = NOW() WHERE id = 1",
        )
        .bind(status)
        .execute(&self.db_pool)
        .await?;
        Ok(())
    }

    pub(super) async fn get_last_processed_block(&self) -> Result<i64> {
        let (block,): (i64,) =
            sqlx::query_as("SELECT last_processed_block FROM indexer_state WHERE id = 1")
                .fetch_one(&self.db_pool)
                .await?;
        Ok(block)
    }
}
