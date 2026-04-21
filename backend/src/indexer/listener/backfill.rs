use super::*;

impl BlockListener {
    pub(super) async fn backfill_recent_deployments(&self, target_tokens: i64) -> Result<()> {
        use alloy::{
            providers::{Provider, ProviderBuilder, WsConnect},
            rpc::types::BlockNumberOrTag,
        };

        let (existing,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tokens")
            .fetch_one(&self.db_pool)
            .await?;
        if existing >= target_tokens {
            tracing::info!(
                existing,
                target_tokens,
                "Backfill skipped: token count already sufficient"
            );
            return Ok(());
        }

        let needed = target_tokens - existing;
        tracing::info!(
            existing,
            needed,
            "Starting startup backfill for recent deployments"
        );

        let mut failures = Vec::new();
        let mut provider = None;
        for rpc_url in &self.config.bnb_rpc_ws_urls {
            let ws = WsConnect::new(rpc_url);
            match ProviderBuilder::new().on_ws(ws).await {
                Ok(connected) => {
                    provider = Some(connected);
                    break;
                }
                Err(error) => {
                    tracing::warn!(rpc_url = %rpc_url, error = %error, "Backfill WebSocket connect failed");
                    failures.push(format!("{rpc_url}: {error}"));
                }
            }
        }
        let provider = provider.ok_or_else(|| {
            anyhow::anyhow!(
                "Backfill WebSocket connect failed for all configured RPCs: {}",
                failures.join(" | ")
            )
        })?;

        let latest = provider
            .get_block_number()
            .await
            .map_err(|e| anyhow::anyhow!("Backfill get_block_number failed: {}", e))?;

        let mut found = 0i64;
        let mut scanned = 0u64;
        let start_block = latest.saturating_sub(BACKFILL_SCAN_WINDOW_BLOCKS);

        for block_number in (start_block..=latest).rev() {
            let full = match provider
                .get_block_by_number(BlockNumberOrTag::Number(block_number), true)
                .await
            {
                Ok(block) => block,
                Err(e) => {
                    let message = e.to_string();
                    if message.contains("error code -32005") || message.contains("RPS limit") {
                        tokio::time::sleep(Duration::from_millis(350)).await;
                        continue;
                    }
                    return Err(anyhow::anyhow!(
                        "Backfill get_block_by_number({}) failed: {}",
                        block_number,
                        e
                    ));
                }
            };

            let Some(block) = full else {
                continue;
            };

            scanned += 1;
            let timestamp = DateTime::from_timestamp(block.header.timestamp as i64, 0)
                .unwrap_or_else(chrono::Utc::now);

            let txs = match block.transactions {
                alloy::rpc::types::BlockTransactions::Full(txs) => txs,
                _ => continue,
            };

            for tx in txs {
                let raw = RawTransaction {
                    hash: tx.hash.to_string(),
                    from: tx.from.to_string(),
                    to: tx.to.map(|a| a.to_string()),
                    input: tx.input.to_string(),
                    value_bnb: wei_to_bnb(tx.value),
                    block_number,
                    timestamp,
                };

                if let Some(mut event) =
                    parse_token_deployment(&raw, &self.config.four_meme_contract_address)
                {
                    if let Ok(Some(receipt)) = provider.get_transaction_receipt(tx.hash).await {
                        if let Ok(json) = serde_json::to_value(receipt) {
                            if let Some(decoded) = decode_from_receipt_json(
                                &json,
                                &self.config.four_meme_contract_address,
                            ) {
                                event.contract_address = decoded.token_address;
                                event.name = decoded.name;
                                event.symbol = decoded.symbol;
                            }
                        }
                    }
                    self.save_event(&event).await?;
                    found += 1;
                    if found >= needed {
                        tracing::info!(found, scanned, "Backfill complete");
                        return Ok(());
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(75)).await;
        }

        tracing::warn!(found, scanned, "Backfill ended before reaching target");
        Ok(())
    }

    pub(super) async fn backfill_missing_risk_scores(&self, batch_size: i64) -> Result<()> {
        let mut total_backfilled = 0u64;
        let limit = batch_size.clamp(1, 5_000);

        loop {
            let rows: Vec<(String,)> = sqlx::query_as(
                r#"
                SELECT t.contract_address
                FROM tokens t
                LEFT JOIN risk_scores rs ON rs.token_address = t.contract_address
                WHERE rs.token_address IS NULL
                ORDER BY t.deployed_at DESC
                LIMIT $1
                "#,
            )
            .bind(limit)
            .fetch_all(&self.db_pool)
            .await?;

            if rows.is_empty() {
                break;
            }

            let batch_len = rows.len() as u64;
            for (token_address,) in rows {
                self.update_risk_score(&token_address).await;
            }
            total_backfilled += batch_len;
        }

        if total_backfilled > 0 {
            tracing::info!(count = total_backfilled, "Backfilled missing risk scores");
        }

        Ok(())
    }

    pub(super) async fn backfill_whale_alerts(&self, batch_size: i64) -> Result<()> {
        let threshold = self.config.whale_alert_threshold_bnb;
        let rows: Vec<(String, String, String, f64, DateTime<chrono::Utc>)> = sqlx::query_as(
            r#"
            SELECT tt.token_address,
                   tt.wallet_address,
                   tt.tx_hash,
                   tt.amount_bnb::double precision,
                   tt.created_at
            FROM token_transactions tt
            LEFT JOIN whale_alerts wa ON wa.tx_hash = tt.tx_hash
            WHERE tt.tx_type = 'buy'
              AND tt.amount_bnb >= $1
              AND wa.tx_hash IS NULL
            ORDER BY tt.created_at DESC
            LIMIT $2
            "#,
        )
        .bind(threshold)
        .bind(batch_size.clamp(1, 10_000))
        .fetch_all(&self.db_pool)
        .await?;

        if rows.is_empty() {
            return Ok(());
        }

        let mut inserted = 0usize;
        for (token_address, wallet_address, tx_hash, amount_bnb, created_at) in rows {
            if upsert_whale_alert(
                &self.db_pool,
                &token_address,
                &wallet_address,
                &tx_hash,
                amount_bnb,
                threshold,
                created_at,
            )
            .await?
            .is_some()
            {
                inserted += 1;
            }
        }

        if inserted > 0 {
            tracing::info!(count = inserted, "Backfilled whale alerts");
        }

        Ok(())
    }
}
