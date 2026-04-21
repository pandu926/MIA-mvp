use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::{
    ai::{
        cache::{get_cached_narrative, set_cached_narrative, CachedNarrative},
        consensus::check_consensus,
        gateway::{LlmGateway, LlmRequest, RollingGateway},
        prompts::{
            build_narrative_prompt, build_risk_interpretation_prompt, determine_confidence,
            NarrativePromptData,
        },
        queue::AiJob,
    },
    config::Config,
    ws::hub::{WsBroadcastHub, WsMessage},
};

const NARRATIVE_MAX_TOKENS: u32 = 512;
const RISK_MAX_TOKENS: u32 = 256;
const TEMPERATURE: f64 = 0.3;

/// Background worker that processes AI analysis jobs from the queue.
///
/// For each job:
/// 1. Checks Redis cache — skips if a fresh narrative exists.
/// 2. Fires two LLM calls concurrently (narrative + risk interpretation).
/// 3. Runs consensus checker.
/// 4. Caches result in Redis and persists to PostgreSQL.
/// 5. Broadcasts `NarrativeUpdate` to all WebSocket clients.
pub struct AiWorker {
    rx: mpsc::Receiver<AiJob>,
    gateway: RollingGateway,
    redis: redis::aio::ConnectionManager,
    db: PgPool,
    config: Arc<Config>,
    ws_hub: WsBroadcastHub,
}

impl AiWorker {
    pub fn new(
        rx: mpsc::Receiver<AiJob>,
        redis: redis::aio::ConnectionManager,
        db: PgPool,
        config: Arc<Config>,
        ws_hub: WsBroadcastHub,
    ) -> Self {
        let gateway = RollingGateway::new(
            &config.llm_api_url,
            &config.llm_api_key,
            config.llm_models.clone(),
        );
        Self {
            rx,
            gateway,
            redis,
            db,
            config,
            ws_hub,
        }
    }

    /// Process jobs until the sender is dropped (shutdown).
    pub async fn run(mut self) {
        tracing::info!("AiWorker started");
        while let Some(job) = self.rx.recv().await {
            if let Err(e) = self.process_job(job).await {
                tracing::error!("AI job failed: {}", e);
            }
        }
        tracing::info!("AiWorker shutting down — queue closed");
    }

    async fn process_job(&mut self, job: AiJob) -> Result<()> {
        let addr = &job.token_address;

        // 1. Cache check — skip if still fresh
        if let Some(cached) = get_cached_narrative(&mut self.redis, addr).await? {
            tracing::debug!(token = %addr, "Narrative cache hit — skipping LLM");
            // Still broadcast so late-connecting WS clients get current narrative
            self.broadcast_narrative(&cached).await;
            return Ok(());
        }

        tracing::info!(token = %addr, "Starting AI analysis");

        // 2. Build prompts
        let narrative_msgs = build_narrative_prompt(&job.prompt_data);
        let risk_msgs = build_risk_interpretation_prompt(&job.prompt_data);

        // Model name is a no-op hint — RollingGateway overrides it from its pool.
        let narrative_req = LlmRequest {
            model: String::new(),
            messages: narrative_msgs,
            temperature: TEMPERATURE,
            max_tokens: NARRATIVE_MAX_TOKENS,
            reasoning: crate::ai::gateway::ReasoningConfig { effort: "high" },
        };
        let risk_req = LlmRequest {
            model: String::new(),
            messages: risk_msgs,
            temperature: TEMPERATURE,
            max_tokens: RISK_MAX_TOKENS,
            reasoning: crate::ai::gateway::ReasoningConfig { effort: "high" },
        };

        // 3. Fire both LLM calls concurrently
        let (narrative_result, risk_result) = tokio::join!(
            self.gateway.generate(&narrative_req),
            self.gateway.generate(&risk_req)
        );

        let narrative_text = match narrative_result {
            Ok(resp) => resp.content().to_string(),
            Err(e) => {
                tracing::error!(token = %addr, "Narrative LLM call failed: {}", e);
                return Err(e);
            }
        };

        let risk_text = match risk_result {
            Ok(resp) => resp.content().to_string(),
            Err(e) => {
                // Risk interpretation is non-critical — proceed without it
                tracing::warn!(token = %addr, "Risk LLM call failed: {}", e);
                String::new()
            }
        };

        // 4. Consensus + confidence
        let consensus = check_consensus(&narrative_text, &risk_text);
        let confidence = determine_confidence(&job.prompt_data);

        // 5. Build cached narrative
        let cached = CachedNarrative {
            token_address: addr.clone(),
            narrative_text: consensus.final_narrative.clone(),
            risk_interpretation: consensus.final_risk_interpretation.clone(),
            consensus_status: consensus.status.as_str().to_string(),
            confidence: confidence.as_str().to_string(),
            generated_at: Utc::now(),
        };

        // 6. Write to Redis cache (best-effort)
        if let Err(e) = set_cached_narrative(
            &mut self.redis,
            addr,
            &cached,
            self.config.ai_cache_ttl_secs,
        )
        .await
        {
            tracing::warn!(token = %addr, "Redis cache write failed: {}", e);
        }

        // 7. Persist to PostgreSQL
        self.persist_narrative(&cached, &narrative_text, &risk_text, &job.prompt_data)
            .await?;

        // 8. Broadcast to WebSocket clients
        self.broadcast_narrative(&cached).await;

        tracing::info!(
            token = %addr,
            consensus = %cached.consensus_status,
            confidence = %cached.confidence,
            "AI narrative generated and broadcast"
        );

        Ok(())
    }

    async fn persist_narrative(
        &self,
        cached: &CachedNarrative,
        model_a_raw: &str,
        model_b_raw: &str,
        prompt_data: &NarrativePromptData,
    ) -> Result<()> {
        let ttl_secs = self.config.ai_cache_ttl_secs as i64;
        let data_basis = serde_json::to_value(prompt_data)
            .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));

        sqlx::query(
            r#"
            INSERT INTO ai_narratives
                (token_address, narrative_text, risk_interpretation,
                 consensus_status, confidence, data_basis,
                 model_a_response, model_b_response,
                 generated_at, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(),
                    NOW() + ($9 * INTERVAL '1 second'))
            ON CONFLICT (token_address) DO UPDATE SET
                narrative_text      = EXCLUDED.narrative_text,
                risk_interpretation = EXCLUDED.risk_interpretation,
                consensus_status    = EXCLUDED.consensus_status,
                confidence          = EXCLUDED.confidence,
                data_basis          = EXCLUDED.data_basis,
                model_a_response    = EXCLUDED.model_a_response,
                model_b_response    = EXCLUDED.model_b_response,
                generated_at        = EXCLUDED.generated_at,
                expires_at          = EXCLUDED.expires_at
            "#,
        )
        .bind(&cached.token_address)
        .bind(&cached.narrative_text)
        .bind(&cached.risk_interpretation)
        .bind(&cached.consensus_status)
        .bind(&cached.confidence)
        .bind(data_basis)
        .bind(model_a_raw)
        .bind(model_b_raw)
        .bind(ttl_secs)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    async fn broadcast_narrative(&self, cached: &CachedNarrative) {
        let msg = WsMessage::NarrativeUpdate {
            token_address: cached.token_address.clone(),
            narrative_text: cached.narrative_text.clone(),
            risk_interpretation: cached.risk_interpretation.clone(),
            consensus_status: cached.consensus_status.clone(),
            confidence: cached.confidence.clone(),
        };
        self.ws_hub.broadcast(msg).await;
    }
}
