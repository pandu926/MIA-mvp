mod backfill;
mod maintenance;

use crate::{
    ai::{
        prompts::NarrativePromptData,
        queue::{check_eligibility, AiJob},
    },
    config::Config,
    indexer::{
        clustering::{detect_clusters, save_clusters},
        parser::{
            parse_token_deployment, parse_token_trade, RawTransaction, TokenDeployedEvent,
            TradeType,
        },
    },
    phase4::{
        telegram::{send_telegram_message, TelegramConfig},
        whale::upsert_whale_alert,
    },
    risk::{scorer as risk_scorer, signals as risk_signals},
    ws::hub::{WsBroadcastHub, WsMessage},
};
use alloy::primitives::U256;
use anyhow::Result;
use chrono::DateTime;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

const BASE_BACKOFF_SECS: u64 = 1;
const MAX_BACKOFF_SECS: u64 = 60;
const BACKFILL_TARGET_TOKENS: i64 = 10;
const BACKFILL_SCAN_WINDOW_BLOCKS: u64 = 5_000;
const RISK_BACKFILL_BATCH_SIZE: i64 = 500;
const WHALE_BACKFILL_BATCH_SIZE: i64 = 1_000;
const TOKEN_CREATED_TOPIC0: &str =
    "0x396d5e902b675b032348d3d2e9517ee8f0c4a926603fbc075d3d282ff00cad20";

fn wei_to_bnb(value: U256) -> f64 {
    let wei = value.to_string().parse::<f64>().unwrap_or(0.0);
    wei / 1_000_000_000_000_000_000f64
}

struct DecodedTokenCreated {
    token_address: String,
    name: Option<String>,
    symbol: Option<String>,
}

fn decode_token_created_log(data_hex: &str) -> Option<DecodedTokenCreated> {
    let trimmed = data_hex
        .strip_prefix("0x")
        .unwrap_or(data_hex)
        .to_ascii_lowercase();
    if trimmed.len() < 320 {
        return None;
    }

    let token_word = &trimmed[64..128];
    let token_address = format!("0x{}", &token_word[24..64]);
    let name = decode_abi_string_at_offset(&trimmed, 3);
    let symbol = decode_abi_string_at_offset(&trimmed, 4);

    Some(DecodedTokenCreated {
        token_address,
        name,
        symbol,
    })
}

fn decode_abi_string_at_offset(hex: &str, word_index: usize) -> Option<String> {
    let offset_hex = hex.get(word_index * 64..(word_index + 1) * 64)?;
    let offset_bytes = usize::from_str_radix(offset_hex, 16).ok()?;
    let offset_hex_pos = offset_bytes * 2;

    let len_hex = hex.get(offset_hex_pos..offset_hex_pos + 64)?;
    let str_len = usize::from_str_radix(len_hex, 16).ok()?;

    if str_len == 0 || str_len > 256 {
        return None;
    }

    let data_start = offset_hex_pos + 64;
    let str_hex = hex.get(data_start..data_start + str_len * 2)?;
    let bytes: Vec<u8> = (0..str_hex.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(&str_hex[i..i + 2], 16).ok())
        .collect();

    String::from_utf8(bytes)
        .ok()
        .filter(|s| !s.trim().is_empty())
}

fn decode_from_receipt_json(
    receipt: &serde_json::Value,
    manager_address: &str,
) -> Option<DecodedTokenCreated> {
    let logs = receipt
        .pointer("/inner/logs")
        .or_else(|| receipt.get("logs"))
        .and_then(|v| v.as_array())?;

    for log in logs {
        let address = log
            .get("address")
            .and_then(|v| v.as_str())
            .map(|s| s.to_ascii_lowercase())?;
        if address != manager_address.to_ascii_lowercase() {
            continue;
        }

        let topic0 = log
            .get("topics")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .map(|s| s.to_ascii_lowercase());
        if topic0.as_deref() != Some(TOKEN_CREATED_TOPIC0) {
            continue;
        }

        let data_hex = log
            .get("data")
            .and_then(|v| v.as_str())
            .or_else(|| log.pointer("/data/data").and_then(|v| v.as_str()))?;

        if let Some(decoded) = decode_token_created_log(data_hex) {
            return Some(decoded);
        }
    }
    None
}

pub struct BlockListener {
    config: Arc<Config>,
    db_pool: PgPool,
    ai_queue_tx: mpsc::Sender<AiJob>,
    ws_hub: WsBroadcastHub,
}

impl BlockListener {
    pub fn new(
        config: Arc<Config>,
        db_pool: PgPool,
        ai_queue_tx: mpsc::Sender<AiJob>,
        ws_hub: WsBroadcastHub,
    ) -> Self {
        Self {
            config,
            db_pool,
            ai_queue_tx,
            ws_hub,
        }
    }

    pub async fn start(&self, cancel: CancellationToken) -> Result<()> {
        tracing::info!("BlockListener starting up");

        let last_block = self.get_last_processed_block().await?;
        tracing::info!(
            last_processed_block = last_block,
            "Resuming from last processed block"
        );

        self.update_indexer_status("running").await?;
        if self.config.indexer_deployment_backfill_enabled {
            if let Err(e) = self
                .backfill_recent_deployments(BACKFILL_TARGET_TOKENS)
                .await
            {
                tracing::warn!(error = %e, "Backfill failed (continuing with live stream)");
            }
        } else {
            tracing::info!("Startup deployment backfill disabled by config");
        }
        if let Err(e) = self
            .backfill_missing_risk_scores(RISK_BACKFILL_BATCH_SIZE)
            .await
        {
            tracing::warn!(
                error = %e,
                "Risk-score backfill failed (continuing with live stream)"
            );
        }
        if let Err(e) = self.backfill_whale_alerts(WHALE_BACKFILL_BATCH_SIZE).await {
            tracing::warn!(
                error = %e,
                "Whale-alert backfill failed (continuing with live stream)"
            );
        }

        let mut retry_count = 0u32;

        loop {
            if cancel.is_cancelled() {
                tracing::info!("BlockListener shutting down (cancellation)");
                self.update_indexer_status("idle").await.ok();
                break;
            }

            match self
                .run_listener_loop(&cancel, retry_count as usize)
                .await
            {
                Ok(()) => {
                    tracing::info!("BlockListener loop exited cleanly");
                    break;
                }
                Err(e) => {
                    retry_count += 1;
                    let backoff =
                        (BASE_BACKOFF_SECS * (1u64 << retry_count.min(5))).min(MAX_BACKOFF_SECS);
                    self.update_indexer_status("degraded").await.ok();
                    tracing::warn!(
                        error = %e,
                        retry = retry_count,
                        backoff_secs = backoff,
                        "BlockListener error — reconnecting with next available RPC"
                    );

                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_secs(backoff)) => {}
                        _ = cancel.cancelled() => {
                            self.update_indexer_status("idle").await.ok();
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn run_listener_loop(
        &self,
        cancel: &CancellationToken,
        start_offset: usize,
    ) -> Result<()> {
        use alloy::{
            providers::{Provider, ProviderBuilder, WsConnect},
            rpc::types::BlockNumberOrTag,
        };
        use futures_util::StreamExt;

        let mut failures = Vec::new();
        let mut provider = None;
        let rpc_urls = &self.config.bnb_rpc_ws_urls;
        for idx in 0..rpc_urls.len() {
            let rpc_url = &rpc_urls[(start_offset + idx) % rpc_urls.len()];
            tracing::info!(rpc_url = %rpc_url, "Connecting to BNB Chain");
            let ws = WsConnect::new(rpc_url);
            match ProviderBuilder::new().on_ws(ws).await {
                Ok(connected) => {
                    provider = Some(connected);
                    break;
                }
                Err(error) => {
                    tracing::warn!(rpc_url = %rpc_url, error = %error, "BNB Chain WebSocket connect failed");
                    failures.push(format!("{rpc_url}: {error}"));
                }
            }
        }
        let provider = provider.ok_or_else(|| {
            anyhow::anyhow!(
                "WebSocket connect failed for all configured RPCs: {}",
                failures.join(" | ")
            )
        })?;

        self.update_indexer_status("running").await.ok();

        tracing::info!("Connected — subscribing to new block headers");

        let sub = provider
            .subscribe_blocks()
            .await
            .map_err(|e| anyhow::anyhow!("Block subscription failed: {}", e))?;

        let mut stream = sub.into_stream();

        loop {
            tokio::select! {
                item = stream.next() => {
                    let Some(header_block) = item else {
                        tracing::warn!("Block subscription stream closed");
                        return Err(anyhow::anyhow!("Block stream terminated"));
                    };

                    let block_number = header_block.header.number;
                    tracing::debug!(block = block_number, "New block header received");

                    let full = provider
                        .get_block_by_number(BlockNumberOrTag::Number(block_number), true)
                        .await
                        .map_err(|e| anyhow::anyhow!("get_block_by_number({}) failed: {}", block_number, e))?;

                    let Some(block) = full else {
                        tracing::debug!(block = block_number, "Block not found (re-org?)");
                        continue;
                    };

                    let timestamp = DateTime::from_timestamp(block.header.timestamp as i64, 0)
                        .unwrap_or_else(chrono::Utc::now);

                    let txs = match block.transactions {
                        alloy::rpc::types::BlockTransactions::Full(txs) => txs,
                        _ => {
                            self.update_indexer_state(block_number).await?;
                            continue;
                        }
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
                            if let Err(e) = self.save_event(&event).await {
                                tracing::error!(error = %e, "Failed to save deployment event");
                            }
                        }

                        if let Some(trade) =
                            parse_token_trade(&raw, &self.config.four_meme_contract_address)
                        {
                            if let Err(e) =
                                self.save_trade(&raw, &trade.token_address, trade.trade_type).await
                            {
                                tracing::warn!(
                                    error = %e,
                                    token = %trade.token_address,
                                    "Failed to persist token trade"
                                );
                            }
                        }
                    }

                    self.update_indexer_state(block_number).await?;
                }
                _ = cancel.cancelled() => {
                    tracing::info!("Listener loop cancelled by shutdown signal");
                    return Ok(());
                }
            }
        }
    }

    async fn save_trade(
        &self,
        tx: &RawTransaction,
        token_address: &str,
        trade_type: TradeType,
    ) -> Result<()> {
        let amount_bnb = match trade_type {
            TradeType::Buy => tx.value_bnb,
            TradeType::Sell => 0.0,
        };

        let tx_type_str = match trade_type {
            TradeType::Buy => "buy",
            TradeType::Sell => "sell",
        };

        let inserted = sqlx::query(
            r#"
            INSERT INTO token_transactions
                (token_address, wallet_address, tx_type, amount_bnb, tx_hash, block_number, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (tx_hash) DO NOTHING
            "#,
        )
        .bind(token_address)
        .bind(&tx.from)
        .bind(tx_type_str)
        .bind(amount_bnb)
        .bind(&tx.hash)
        .bind(tx.block_number as i64)
        .bind(tx.timestamp)
        .execute(&self.db_pool)
        .await;

        let inserted = match inserted {
            Ok(result) => result.rows_affected() > 0,
            Err(e) => {
                tracing::debug!(token = %token_address, error = %e, "Skipping trade insert");
                return Ok(());
            }
        };
        if !inserted {
            return Ok(());
        }

        self.refresh_token_metrics(token_address).await?;
        self.update_risk_score(token_address).await;
        self.broadcast_token_snapshot(token_address).await?;

        if matches!(trade_type, TradeType::Buy) {
            self.maybe_emit_whale_alert(tx, token_address).await?;
            let deployer = self.get_deployer_for_token(token_address).await?;
            self.maybe_enqueue_ai_job(token_address, &deployer).await;
        }

        Ok(())
    }

    async fn maybe_emit_whale_alert(&self, tx: &RawTransaction, token_address: &str) -> Result<()> {
        let threshold = self.config.whale_alert_threshold_bnb;
        let Some(alert) = upsert_whale_alert(
            &self.db_pool,
            token_address,
            &tx.from,
            &tx.hash,
            tx.value_bnb,
            threshold,
            tx.timestamp,
        )
        .await?
        else {
            return Ok(());
        };

        let text = format!(
            "*Whale {} Alert*\nToken: `{}`\nWallet: `{}`\nAmount: {:.4} BNB\nTx: `{}`",
            alert.alert_level.to_uppercase(),
            alert.token_address,
            alert.wallet_address,
            alert.amount_bnb,
            alert.tx_hash
        );

        let cfg = TelegramConfig {
            bot_token: self.config.telegram_bot_token.clone(),
            chat_id: self.config.telegram_chat_id.clone(),
        };

        send_telegram_message(
            &reqwest::Client::new(),
            &self.db_pool,
            &cfg,
            "whale_alert",
            text,
        )
        .await?;

        Ok(())
    }

    async fn save_event(&self, event: &TokenDeployedEvent) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO blockchain_events
                (block_number, tx_hash, contract_address, event_name, raw_data)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(event.block_number as i64)
        .bind(&event.tx_hash)
        .bind(&event.contract_address)
        .bind("TokenDeployed")
        .bind(serde_json::to_value(event)?)
        .execute(&self.db_pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO tokens
                (contract_address, deployer_address, name, symbol, deployed_at, block_number, tx_hash)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (contract_address) DO UPDATE
                SET name   = COALESCE(EXCLUDED.name,   tokens.name),
                    symbol = COALESCE(EXCLUDED.symbol, tokens.symbol)
            "#,
        )
        .bind(&event.contract_address)
        .bind(&event.deployer_address)
        .bind(&event.name)
        .bind(&event.symbol)
        .bind(event.timestamp)
        .bind(event.block_number as i64)
        .bind(&event.tx_hash)
        .execute(&self.db_pool)
        .await?;

        tracing::info!(
            contract = %event.contract_address,
            deployer = %event.deployer_address,
            block = event.block_number,
            "Token deployment event persisted"
        );

        self.broadcast_token_update(event).await;
        self.update_risk_score(&event.contract_address).await;
        let _ = self.broadcast_token_snapshot(&event.contract_address).await;
        self.maybe_enqueue_ai_job(&event.contract_address, &event.deployer_address)
            .await;

        Ok(())
    }

    async fn broadcast_token_update(&self, event: &TokenDeployedEvent) {
        let msg = WsMessage::TokenUpdate {
            token_address: event.contract_address.clone(),
            name: None,
            symbol: None,
            deployer_address: event.deployer_address.clone(),
            buy_count: 0,
            sell_count: 0,
            volume_bnb: 0.0,
            composite_score: None,
            risk_category: None,
            deployed_at: event.timestamp.to_rfc3339(),
        };
        self.ws_hub.broadcast(msg).await;
    }

    async fn maybe_enqueue_ai_job(&self, token_address: &str, deployer_address: &str) {
        let has_fresh: Option<(bool,)> = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM ai_narratives WHERE token_address = $1 AND expires_at > NOW())",
        )
        .bind(token_address)
        .fetch_optional(&self.db_pool)
        .await
        .ok()
        .flatten();

        if has_fresh.map(|(v,)| v).unwrap_or(false) {
            return;
        }

        let threshold = self.config.ai_buy_threshold;
        let window = self.config.ai_threshold_window_secs;

        match check_eligibility(&self.db_pool, token_address, threshold, window).await {
            Ok(true) => {
                {
                    let pool = self.db_pool.clone();
                    let addr = token_address.to_string();
                    tokio::spawn(async move {
                        match detect_clusters(&pool, &addr, 60).await {
                            Ok(clusters) if !clusters.is_empty() => {
                                if let Err(e) = save_clusters(&pool, &addr, &clusters).await {
                                    tracing::warn!(token = %addr, "save_clusters failed: {}", e);
                                } else {
                                    tracing::info!(
                                        token = %addr,
                                        count = clusters.len(),
                                        "Wallet clusters saved"
                                    );
                                }
                            }
                            Ok(_) => {}
                            Err(e) => {
                                tracing::warn!(token = %addr, "detect_clusters failed: {}", e)
                            }
                        }
                    });
                }

                let snapshot: Option<(
                    Option<String>,
                    Option<String>,
                    i32,
                    i32,
                    i32,
                    f64,
                    bool,
                    bool,
                    bool,
                    chrono::DateTime<chrono::Utc>,
                )> = sqlx::query_as(
                    r#"
                    SELECT name, symbol, holder_count, buy_count, sell_count,
                           volume_bnb::double precision,
                           is_rug, graduated, honeypot_detected, deployed_at
                    FROM tokens
                    WHERE contract_address = $1
                    "#,
                )
                .bind(token_address)
                .fetch_optional(&self.db_pool)
                .await
                .ok()
                .flatten();

                let (
                    token_name,
                    token_symbol,
                    holder_count,
                    buy_count,
                    sell_count,
                    volume_bnb,
                    is_rug,
                    graduated,
                    honeypot_detected,
                    deployed_at,
                ) = snapshot.unwrap_or((
                    None,
                    None,
                    0,
                    0,
                    0,
                    0.0,
                    false,
                    false,
                    false,
                    chrono::Utc::now(),
                ));

                let hours_since_deploy =
                    (chrono::Utc::now() - deployed_at).num_seconds() as f64 / 3600.0;

                let deployer_history: Option<(i64, i64, i64)> = sqlx::query_as(
                    r#"
                    SELECT
                        COUNT(*)                                          AS total,
                        SUM(CASE WHEN is_rug       THEN 1 ELSE 0 END)   AS rug_count,
                        SUM(CASE WHEN graduated    THEN 1 ELSE 0 END)   AS graduated_count
                    FROM tokens
                    WHERE deployer_address = $1
                    "#,
                )
                .bind(deployer_address)
                .fetch_optional(&self.db_pool)
                .await
                .ok()
                .flatten();

                let (deployer_total, deployer_rug_count, deployer_graduated_count) =
                    deployer_history.unwrap_or((1, 0, 0));

                let rug_rate_pct = if deployer_total > 0 {
                    deployer_rug_count as f64 / deployer_total as f64 * 100.0
                } else {
                    0.0
                };
                let deployer_trust_grade = match rug_rate_pct as u32 {
                    0 if deployer_graduated_count >= 2 => "A",
                    0 => "B",
                    1..=10 => "C",
                    11..=30 => "D",
                    _ => "F",
                }
                .to_string();

                let mut risk_score: i32 = 50;
                risk_score += match rug_rate_pct as u32 {
                    0 => -10,
                    1..=10 => 5,
                    11..=30 => 20,
                    _ => 40,
                };
                if buy_count > 0 && sell_count > (buy_count as f64 * 1.5) as i32 {
                    risk_score += 15;
                }
                if honeypot_detected {
                    risk_score += 30;
                }
                if graduated {
                    risk_score -= 20;
                }
                let composite_risk_score = risk_score.clamp(0, 100) as u8;

                let risk_category = match composite_risk_score {
                    0..=33 => "low",
                    34..=66 => "medium",
                    _ => "high",
                }
                .to_string();

                let prompt_data = NarrativePromptData {
                    token_address: token_address.to_string(),
                    token_name,
                    token_symbol,
                    deployer_address: deployer_address.to_string(),
                    deployer_trust_grade,
                    deployer_rug_count,
                    deployer_graduated_count,
                    holder_count,
                    buy_count,
                    sell_count,
                    volume_bnb,
                    composite_risk_score,
                    risk_category,
                    top_holder_concentration_pct: self
                        .compute_top_holder_concentration_pct(token_address)
                        .await,
                    hours_since_deploy,
                    honeypot_detected,
                    is_rug,
                    graduated,
                };

                let job = AiJob {
                    token_address: token_address.to_string(),
                    prompt_data,
                };

                if let Err(e) = self.ai_queue_tx.try_send(job) {
                    tracing::warn!(token = %token_address, "AI queue full or closed: {}", e);
                } else {
                    tracing::info!(token = %token_address, "AI job enqueued");
                }
            }
            Ok(false) => {}
            Err(e) => tracing::warn!(token = %token_address, "Eligibility check failed: {}", e),
        }
    }
}
