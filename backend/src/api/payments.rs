use axum::{extract::State, Json};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{config::Config, error::AppError, AppState};

#[derive(Debug, Deserialize)]
pub struct X402VerifyRequest {
    pub payment_payload: Option<Value>,
    pub payment_requirements: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct X402VerifyResponse {
    pub enabled: bool,
    pub accepted: bool,
    pub provider: String,
    pub network: String,
    pub scheme: String,
    pub facilitator_url: String,
    pub price_usdc_cents: u32,
    pub status: String,
    pub message: String,
    pub raw: Option<Value>,
}

fn stable_payment_context(
    resource: &str,
    config: &Config,
    pay_to: &str,
    asset_address: &str,
) -> Value {
    let seed = format!(
        "{}|{}|{}|{}|{}",
        resource, config.x402_network, config.x402_scheme, pay_to, asset_address
    );

    let mut payment_id_bytes = [0u8; 16];
    let mut nonce_bytes = [0u8; 16];
    for (index, byte) in seed.bytes().enumerate() {
        let rotated = byte.rotate_left((index % 8) as u32);
        payment_id_bytes[index % 16] ^= rotated;
        nonce_bytes[(index * 7) % 16] =
            nonce_bytes[(index * 7) % 16].wrapping_add(rotated ^ ((index as u8).wrapping_mul(31)));
    }

    let payment_id = format!(
        "0x{}",
        payment_id_bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    let nonce = u128::from_be_bytes(nonce_bytes).to_string();

    json!({
        "paymentPermitContext": {
            "meta": {
                "kind": "PAYMENT_ONLY",
                "paymentId": payment_id,
                "nonce": nonce,
                "validAfter": 0,
                "validBefore": 4102444800u64
            }
        }
    })
}

fn payment_extra(config: &Config) -> Value {
    let mut extra = json!({
        "name": "USDT",
        "version": "2"
    });

    if config.x402_scheme == "exact_permit" && config.x402_network.starts_with("eip155:") {
        if let (Some(fee_to), Some(caller)) =
            (config.x402_fee_to.as_deref(), config.x402_caller.as_deref())
        {
            extra["fee"] = json!({
                "facilitatorId": config
                    .x402_facilitator_id
                    .clone()
                    .unwrap_or_else(|| "https://facilitator.bankofai.io".to_string()),
                "feeTo": fee_to,
                "feeAmount": config.x402_fee_amount,
                "caller": caller
            });
        }
    }

    extra
}

pub fn build_payment_requirements(
    resource: &str,
    description: &str,
    config: &Config,
    pay_to: &str,
    asset_address: &str,
) -> Value {
    json!({
        "x402Version": 2,
        "resource": {
            "url": resource,
            "description": description,
            "mimeType": "application/json"
        },
        "accepts": [{
            "scheme": config.x402_scheme,
            "network": config.x402_network,
            "amount": config.x402_price_usdc_cents.to_string(),
            "payTo": pay_to,
            "maxTimeoutSeconds": config.x402_max_timeout_secs,
            "asset": asset_address,
            "extra": payment_extra(config)
        }],
        "extensions": stable_payment_context(resource, config, pay_to, asset_address),
        "error": "Payment required"
    })
}

pub fn build_payment_required_header_value(
    resource: &str,
    description: &str,
    config: &Config,
    pay_to: &str,
    asset_address: &str,
) -> anyhow::Result<String> {
    let payload = build_payment_requirements(resource, description, config, pay_to, asset_address);
    let encoded = STANDARD.encode(serde_json::to_vec(&payload)?);
    Ok(encoded)
}

pub fn decode_payment_signature_header(encoded: &str) -> anyhow::Result<Value> {
    let bytes = STANDARD.decode(encoded.trim())?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub fn build_facilitator_headers(config: &Config) -> anyhow::Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    if let Some(api_key) = config.x402_facilitator_api_key.as_deref() {
        headers.insert("X-API-KEY", HeaderValue::from_str(api_key)?);
    }
    Ok(headers)
}

fn facilitator_payment_requirements(payment_requirements: &Value) -> Value {
    payment_requirements
        .get("accepts")
        .and_then(Value::as_array)
        .and_then(|accepts| accepts.first())
        .cloned()
        .unwrap_or_else(|| payment_requirements.clone())
}

pub async fn verify_with_facilitator(
    config: &Config,
    payment_payload: &Value,
    payment_requirements: &Value,
) -> anyhow::Result<Value> {
    let facilitator = config
        .x402_facilitator_url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("X402_FACILITATOR_URL is not configured"))?;
    let endpoint = format!("{}/verify", facilitator.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let response = client
        .post(&endpoint)
        .headers(build_facilitator_headers(config)?)
        .json(&json!({
            "paymentPayload": payment_payload,
            "paymentRequirements": facilitator_payment_requirements(payment_requirements),
        }))
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "facilitator verify returned {}: {}",
            status,
            body
        ));
    }
    Ok(serde_json::from_str(&body)?)
}

pub async fn settle_with_facilitator(
    config: &Config,
    payment_payload: &Value,
    payment_requirements: &Value,
) -> anyhow::Result<Value> {
    let facilitator = config
        .x402_facilitator_url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("X402_FACILITATOR_URL is not configured"))?;
    let endpoint = format!("{}/settle", facilitator.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let response = client
        .post(&endpoint)
        .headers(build_facilitator_headers(config)?)
        .json(&json!({
            "paymentPayload": payment_payload,
            "paymentRequirements": facilitator_payment_requirements(payment_requirements),
        }))
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "facilitator settle returned {}: {}",
            status,
            body
        ));
    }
    Ok(serde_json::from_str(&body)?)
}

pub fn verification_success(value: &Value) -> bool {
    value
        .get("isValid")
        .and_then(Value::as_bool)
        .or_else(|| value.get("valid").and_then(Value::as_bool))
        .or_else(|| value.get("is_valid").and_then(Value::as_bool))
        .or_else(|| value.get("success").and_then(Value::as_bool))
        .unwrap_or(false)
}

pub fn settlement_success(value: &Value) -> bool {
    value
        .get("success")
        .and_then(Value::as_bool)
        .or_else(|| value.get("valid").and_then(Value::as_bool))
        .unwrap_or(false)
}

pub fn build_x402_verify_contract(config: &Config) -> X402VerifyResponse {
    X402VerifyResponse {
        enabled: config.x402_enabled,
        accepted: false,
        provider: "MIA x402 via BANK OF AI".to_string(),
        network: config.x402_network.clone(),
        scheme: config.x402_scheme.clone(),
        facilitator_url: config
            .x402_facilitator_url
            .clone()
            .unwrap_or_else(|| "not_configured".to_string()),
        price_usdc_cents: config.x402_price_usdc_cents,
        status: "scaffolded".to_string(),
        message: "MIA charges users through x402 and verifies or settles with the BANK OF AI facilitator.".to_string(),
        raw: None,
    }
}

pub async fn verify_x402_payment(
    State(state): State<AppState>,
    Json(request): Json<X402VerifyRequest>,
) -> Result<Json<X402VerifyResponse>, AppError> {
    if !state.config.x402_enabled {
        return Err(AppError::FeatureDisabled(
            "x402 verification is disabled on this deployment.".to_string(),
        ));
    }

    let Some(payment_payload) = request.payment_payload else {
        return Ok(Json(build_x402_verify_contract(&state.config)));
    };
    let Some(payment_requirements) = request.payment_requirements else {
        return Ok(Json(build_x402_verify_contract(&state.config)));
    };

    let verify_response =
        verify_with_facilitator(&state.config, &payment_payload, &payment_requirements)
            .await
            .map_err(|err| AppError::PaymentRequired(format!("x402 verification failed: {err}")))?;

    Ok(Json(X402VerifyResponse {
        enabled: true,
        accepted: verification_success(&verify_response),
        provider: "MIA x402 via BANK OF AI".to_string(),
        network: state.config.x402_network.clone(),
        scheme: state.config.x402_scheme.clone(),
        facilitator_url: state
            .config
            .x402_facilitator_url
            .clone()
            .unwrap_or_else(|| "not_configured".to_string()),
        price_usdc_cents: state.config.x402_price_usdc_cents,
        status: if verification_success(&verify_response) {
            "verified".to_string()
        } else {
            "rejected".to_string()
        },
        message: "Facilitator verification completed.".to_string(),
        raw: Some(verify_response),
    }))
}

#[cfg(test)]
mod tests {
    use super::{
        build_facilitator_headers, build_payment_required_header_value, build_x402_verify_contract,
        decode_payment_signature_header,
    };
    use crate::config::{Config, DeepResearchProvider, DeepResearchUnlockModel, MlRolloutMode};
    use base64::Engine as _;
    use serde_json::json;

    fn fixture_config() -> Config {
        Config {
            database_url: "postgres://mia:pass@localhost/mia_db".to_string(),
            redis_url: "redis://localhost:6379".to_string(),
            bnb_rpc_ws_url: "wss://example.com/ws".to_string(),
            bnb_rpc_ws_urls: vec!["wss://example.com/ws".to_string()],
            four_meme_contract_address: "0xabc".to_string(),
            app_base_url: "https://mia.example.com".to_string(),
            allowed_origins: vec!["https://mia.example.com".to_string()],
            log_level: "info".to_string(),
            server_port: 8080,
            llm_api_url: "https://llm.example".to_string(),
            llm_api_key: "sk-test".to_string(),
            llm_models: vec!["gpt-5.4".to_string()],
            ai_cache_ttl_secs: 300,
            ai_buy_threshold: 10,
            ai_threshold_window_secs: 600,
            whale_alert_threshold_bnb: 0.5,
            alpha_refresh_secs: 3600,
            alpha_top_k: 10,
            indexer_deployment_backfill_enabled: false,
            auto_investigation_enabled: true,
            auto_investigation_interval_secs: 300,
            auto_investigation_tx_threshold: 100,
            auto_investigation_cooldown_mins: 240,
            auto_investigation_max_runs_per_scan: 3,
            ai_score_min_tx_count: 50,
            auto_deep_research_tx_threshold: 500,
            investigation_fixture_api_enabled: false,
            telegram_bot_token: None,
            telegram_chat_id: None,
            ml_rollout_mode: MlRolloutMode::Shadow,
            ml_model_version: "shadow".to_string(),
            ml_min_confidence: 0.55,
            moralis_api_key: None,
            moralis_api_url: "https://deep-index.moralis.io/api/v2.2".to_string(),
            bscscan_api_key: None,
            bscscan_api_url: "https://api.etherscan.io/v2/api".to_string(),
            bscscan_chain_id: 56,
            deep_research_enabled: true,
            ask_mia_function_calling_enabled: false,
            deep_research_provider: DeepResearchProvider::HeuristMeshX402,
            deep_research_unlock_model: DeepResearchUnlockModel::UnlockThisReport,
            x402_enabled: true,
            x402_facilitator_url: Some("https://facilitator.bankofai.io".to_string()),
            x402_facilitator_api_key: Some("facilitator-key".to_string()),
            x402_pay_to: Some("0xabc123abc123abc123abc123abc123abc123abcd".to_string()),
            x402_asset_address: Some("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".to_string()),
            x402_network: "eip155:56".to_string(),
            x402_scheme: "exact_permit".to_string(),
            x402_facilitator_id: Some("https://facilitator.bankofai.io".to_string()),
            x402_fee_to: Some("0xfee0000000000000000000000000000000000001".to_string()),
            x402_caller: Some("0xfee0000000000000000000000000000000000001".to_string()),
            x402_fee_amount: "0".to_string(),
            x402_price_usdc_cents: 50,
            x402_max_timeout_secs: 60,
            heurist_mesh_api_url: "https://mesh.heurist.xyz".to_string(),
            heurist_mesh_agent_set: "deep_research".to_string(),
            heurist_api_key: Some("heurist-key".to_string()),
            heurist_x402_wallet_dir: Some("/tmp/mia-agent-wallet".to_string()),
            heurist_x402_wallet_id: "mia-base-upstream".to_string(),
            heurist_x402_wallet_password: Some("wallet-pass".to_string()),
        }
    }

    #[test]
    fn x402_verify_contract_reports_scaffolded_state() {
        let response = build_x402_verify_contract(&fixture_config());

        assert!(response.enabled);
        assert!(!response.accepted);
        assert_eq!(response.provider, "MIA x402 via BANK OF AI");
        assert_eq!(response.network, "eip155:56");
        assert_eq!(response.scheme, "exact_permit");
        assert_eq!(response.status, "scaffolded");
    }

    #[test]
    fn payment_required_header_contains_report_unlock_requirements() {
        let encoded = build_payment_required_header_value(
            "https://mia.example.com/api/v1/tokens/0x123/deep-research",
            "Unlock MIA Deep Research report",
            &fixture_config(),
            "0xabc123abc123abc123abc123abc123abc123abcd",
            "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        )
        .expect("header should encode");

        let decoded = decode_payment_signature_header(&encoded).expect("header should decode");

        assert_eq!(decoded["x402Version"], 2);
        assert_eq!(decoded["accepts"][0]["scheme"], "exact_permit");
        assert_eq!(decoded["accepts"][0]["network"], "eip155:56");
        assert_eq!(
            decoded["resource"]["description"],
            "Unlock MIA Deep Research report"
        );
        assert_eq!(
            decoded["accepts"][0]["extra"]["fee"]["caller"],
            "0xfee0000000000000000000000000000000000001"
        );
    }

    #[test]
    fn facilitator_headers_include_bankofai_api_key_when_present() {
        let headers = build_facilitator_headers(&fixture_config()).expect("headers should build");

        assert_eq!(
            headers
                .get("X-API-KEY")
                .and_then(|value| value.to_str().ok()),
            Some("facilitator-key")
        );
    }

    #[test]
    fn decode_payment_signature_header_round_trips_json_payload() {
        let payload = json!({
            "x402Version": 1,
            "accepts": [{
                "scheme": "exact_permit",
                "network": "eip155:56"
            }]
        });
        let encoded = base64::engine::general_purpose::STANDARD
            .encode(serde_json::to_vec(&payload).expect("json encoding should succeed"));

        let decoded = decode_payment_signature_header(&encoded).expect("decode should succeed");

        assert_eq!(decoded["x402Version"], 1);
        assert!(decoded["accepts"].is_array());
    }
}
