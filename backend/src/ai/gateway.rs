use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

// ─── Message types (OpenAI-compatible chat format) ────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }
}

// ─── Request / Response shapes ────────────────────────────────────────────────

/// Request payload sent to the LLM gateway (OpenAI-compatible).
/// The `model` field is used as a hint; `RollingGateway` may override it.
#[derive(Debug, Clone, Serialize)]
pub struct LlmRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f64,
    pub max_tokens: u32,
    pub reasoning: ReasoningConfig,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReasoningConfig {
    pub effort: &'static str,
}

/// Token usage counters returned by the gateway.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// A single choice in the chat completion response.
/// Unknown fields (e.g. `reasoning_content` from Qwen3 thinking models) are
/// silently ignored by serde's default behaviour.
#[derive(Debug, Clone, Deserialize)]
pub struct Choice {
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

/// Full response from the LLM gateway.
#[derive(Debug, Clone, Deserialize)]
pub struct LlmResponse {
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<TokenUsage>,
}

impl LlmResponse {
    /// Extract the text content from the first choice.
    pub fn content(&self) -> &str {
        self.choices
            .first()
            .map(|c| c.message.content.as_str())
            .unwrap_or("")
    }

    pub fn finish_reason(&self) -> Option<&str> {
        self.choices
            .first()
            .and_then(|choice| choice.finish_reason.as_deref())
    }

    pub fn prompt_tokens(&self) -> Option<u32> {
        self.usage.as_ref().map(|usage| usage.prompt_tokens)
    }

    pub fn completion_tokens(&self) -> Option<u32> {
        self.usage.as_ref().map(|usage| usage.completion_tokens)
    }
}

// ─── Gateway trait ────────────────────────────────────────────────────────────

/// Abstraction over the LLM gateway so the implementation can be swapped.
#[async_trait::async_trait]
pub trait LlmGateway: Send + Sync {
    async fn generate(&self, request: &LlmRequest) -> Result<LlmResponse>;
}

// ─── Low-level HTTP client ────────────────────────────────────────────────────

/// Raw OpenAI-compatible HTTP client.  Stateless — the model to use is
/// supplied per-call via `model_override`.
#[derive(Debug, Clone)]
pub struct LlmClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl LlmClient {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to build reqwest client");

        Self {
            base_url: base_url.into(),
            api_key: api_key.into(),
            client,
        }
    }

    /// Send a single completion request with an explicit model name.
    /// Returns `(status_code, body_bytes)` without consuming the response.
    pub async fn post_completion(
        &self,
        model: &str,
        request: &LlmRequest,
    ) -> Result<reqwest::Response> {
        // Build a patched request with the chosen model
        let patched = LlmRequest {
            model: model.to_string(),
            ..request.clone()
        };

        let url = format!("{}/v1/chat/completions", self.base_url);
        self.client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&patched)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("LLM HTTP request failed: {}", e))
    }
}

// ─── Rolling Gateway ──────────────────────────────────────────────────────────

/// Gateway that cycles through a prioritised model list on 429 (rate-limit).
///
/// On each `generate` call:
/// 1. Starts with the model at `current_idx`.
/// 2. On HTTP 429 → advances `current_idx`, waits 1 s, retries with next model.
/// 3. Repeats until a model succeeds or the entire pool is exhausted.
/// 4. Non-429 HTTP errors abort immediately (no rolling).
///
/// `current_idx` is shared atomically so state is preserved across concurrent
/// calls, making the roll sticky: once model 0 is rate-limited, subsequent
/// calls start from model 1.
#[derive(Debug, Clone)]
pub struct RollingGateway {
    inner: LlmClient,
    models: Vec<String>,
    current_idx: Arc<AtomicUsize>,
}

impl RollingGateway {
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        models: Vec<String>,
    ) -> Self {
        assert!(
            !models.is_empty(),
            "RollingGateway requires at least one model"
        );
        Self {
            inner: LlmClient::new(base_url, api_key),
            models,
            current_idx: Arc::new(AtomicUsize::new(0)),
        }
    }

    #[cfg(test)]
    pub fn current_model(&self) -> &str {
        let idx = self.current_idx.load(Ordering::Relaxed) % self.models.len();
        &self.models[idx]
    }

    /// Advance to the next model in the pool (wraps around).
    fn advance(&self) -> usize {
        let next = (self.current_idx.load(Ordering::Relaxed) + 1) % self.models.len();
        self.current_idx.store(next, Ordering::Relaxed);
        next
    }
}

#[async_trait::async_trait]
impl LlmGateway for RollingGateway {
    async fn generate(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let n = self.models.len();
        let start = self.current_idx.load(Ordering::Relaxed) % n;

        for attempt in 0..n {
            let idx = (start + attempt) % n;
            let model = &self.models[idx];

            tracing::debug!(model = %model, attempt = attempt + 1, "Sending LLM request");

            let resp = self.inner.post_completion(model, request).await?;
            let status = resp.status();

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let next = self.advance();
                tracing::warn!(
                    model = %model,
                    next_model = %self.models[next % n],
                    "Rate limited (429) — rolling to next model"
                );
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                continue;
            }

            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(anyhow::anyhow!(
                    "LLM gateway error {} on {}: {}",
                    status,
                    model,
                    body
                ));
            }

            let llm_resp: LlmResponse = resp.json().await.map_err(|e| {
                anyhow::anyhow!("Failed to parse LLM response from {}: {}", model, e)
            })?;

            tracing::debug!(
                model = %model,
                tokens = ?llm_resp.usage.as_ref().map(|usage| usage.total_tokens),
                prompt_tokens = ?llm_resp.prompt_tokens(),
                completion_tokens = ?llm_resp.completion_tokens(),
                finish_reason = ?llm_resp.finish_reason(),
                "LLM response received"
            );
            return Ok(llm_resp);
        }

        Err(anyhow::anyhow!(
            "All {} models in the pool are rate-limited. Try again later.",
            n
        ))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    // ── ChatMessage ───────────────────────────────────────────────────────────

    #[test]
    fn chat_message_system_role() {
        let msg = ChatMessage::system("You are a helpful assistant.");
        assert_eq!(msg.role, "system");
        assert_eq!(msg.content, "You are a helpful assistant.");
    }

    #[test]
    fn chat_message_user_role() {
        let msg = ChatMessage::user("Analyze this token.");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Analyze this token.");
    }

    // ── LlmRequest serialization ──────────────────────────────────────────────

    #[test]
    fn llm_request_serializes_correctly() {
        let req = LlmRequest {
            model: "gpt-5.4".to_string(),
            messages: vec![
                ChatMessage::system("You are MIA."),
                ChatMessage::user("Analyze token 0xABC."),
            ],
            temperature: 0.3,
            max_tokens: 256,
            reasoning: ReasoningConfig { effort: "high" },
        };

        let json = serde_json::to_value(&req).unwrap();

        assert_eq!(json["model"], "gpt-5.4");
        assert_eq!(json["temperature"], 0.3);
        assert_eq!(json["max_tokens"], 256);
        assert_eq!(json["messages"][0]["role"], "system");
        assert_eq!(json["messages"][1]["role"], "user");
    }

    #[test]
    fn llm_request_messages_field_present() {
        let req = LlmRequest {
            model: "gpt-5.2".to_string(),
            messages: vec![ChatMessage::user("Hello")],
            temperature: 0.0,
            max_tokens: 100,
            reasoning: ReasoningConfig { effort: "high" },
        };

        let json = serde_json::to_value(&req).unwrap();
        assert!(json["messages"].is_array());
        assert_eq!(json["messages"].as_array().unwrap().len(), 1);
    }

    // ── LlmResponse deserialization ───────────────────────────────────────────

    #[test]
    fn llm_response_deserializes_correctly() {
        let raw = r#"{
            "model": "gpt-5.4",
            "choices": [
                {
                    "message": { "role": "assistant", "content": "This token looks risky." },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 20,
                "total_tokens": 120
            }
        }"#;

        let resp: LlmResponse = serde_json::from_str(raw).unwrap();

        assert_eq!(resp.model, "gpt-5.4");
        assert_eq!(resp.content(), "This token looks risky.");
        assert_eq!(resp.usage.as_ref().unwrap().total_tokens, 120);
    }

    #[test]
    fn llm_response_content_empty_on_no_choices() {
        let raw = r#"{"model": "gpt-5.4", "choices": [], "usage": null}"#;
        let resp: LlmResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(resp.content(), "");
    }

    #[test]
    fn llm_response_usage_optional() {
        let raw = r#"{
            "model": "gpt-5.4",
            "choices": [{"message": {"role": "assistant", "content": "ok"}, "finish_reason": null}],
            "usage": null
        }"#;
        let resp: LlmResponse = serde_json::from_str(raw).unwrap();
        assert!(resp.usage.is_none());
        assert_eq!(resp.content(), "ok");
    }

    /// Some GPT-compatible providers include `reasoning_content` in the message object.
    /// serde should silently ignore it and still parse `content` correctly.
    #[test]
    fn llm_response_ignores_reasoning_content_field() {
        let raw = r#"{
            "model": "gpt-5.4",
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Looks organic.",
                    "reasoning_content": "Step 1: check deployer history..."
                },
                "finish_reason": "stop"
            }],
            "usage": null
        }"#;
        let resp: LlmResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(resp.content(), "Looks organic.");
    }

    // ── LlmClient construction ────────────────────────────────────────────────

    #[test]
    fn llm_client_stores_base_url_and_key() {
        let client = LlmClient::new(
            "http://127.0.0.1:8317",
            "sk-test-123",
        );
        assert_eq!(
            client.base_url,
            "http://127.0.0.1:8317"
        );
        assert_eq!(client.api_key, "sk-test-123");
    }

    // ── RollingGateway ────────────────────────────────────────────────────────

    #[test]
    fn rolling_gateway_current_model_is_first() {
        let gw = RollingGateway::new(
            "https://example.com",
            "key",
            vec!["model-a".into(), "model-b".into(), "model-c".into()],
        );
        assert_eq!(gw.current_model(), "model-a");
    }

    #[test]
    fn rolling_gateway_advance_wraps_around() {
        let gw = RollingGateway::new(
            "https://example.com",
            "key",
            vec!["model-a".into(), "model-b".into(), "model-c".into()],
        );

        let n1 = gw.advance(); // 1
        assert_eq!(n1, 1);
        assert_eq!(gw.current_model(), "model-b");

        let n2 = gw.advance(); // 2
        assert_eq!(n2, 2);
        assert_eq!(gw.current_model(), "model-c");

        let n3 = gw.advance(); // wraps → 0
        assert_eq!(n3, 0);
        assert_eq!(gw.current_model(), "model-a");
    }

    #[test]
    fn rolling_gateway_clone_shares_index() {
        let gw = RollingGateway::new(
            "https://example.com",
            "key",
            vec!["model-a".into(), "model-b".into()],
        );
        let gw2 = gw.clone();
        gw.advance(); // index → 1 on original
                      // clone shares Arc<AtomicUsize>, so both see the update
        assert_eq!(gw2.current_model(), "model-b");
    }

    #[test]
    #[should_panic(expected = "at least one model")]
    fn rolling_gateway_panics_on_empty_models() {
        RollingGateway::new("https://example.com", "key", vec![]);
    }
}
