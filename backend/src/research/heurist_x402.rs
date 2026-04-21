use std::{path::PathBuf, process::Stdio};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::process::Command;

use crate::config::Config;

use super::heurist::{extract_summary_text, HeuristAgentResult, HeuristDossier};

const UPSTREAM_NETWORK: &str = "base";
const UPSTREAM_ASSET: &str = "USDC";
const BASE_USDC_ADDRESS: &str = "0x833589fcdd6edb6e08f4c7c32d4f71b54bda02913";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeuristPaymentTrace {
    pub provider: String,
    pub agent_id: String,
    pub tool_name: String,
    pub network: String,
    pub asset: String,
    pub amount_units: String,
    pub amount_display: String,
    pub cost_cents: u32,
    pub payment_tx: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PaidHeuristResult {
    pub result: HeuristAgentResult,
    pub payment: HeuristPaymentTrace,
    pub endpoint: String,
    pub payer: String,
}

#[derive(Debug, Clone)]
struct PaidHeuristRequest {
    section_id: &'static str,
    title: &'static str,
    agent_id: &'static str,
    tool_name: &'static str,
    query: String,
    payload: Value,
}

#[derive(Debug, Deserialize)]
struct ScriptPaymentRequirement {
    #[serde(rename = "maxAmountRequired")]
    max_amount_required: Option<String>,
    network: Option<String>,
    asset: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ScriptPaymentResponse {
    success: bool,
    transaction: Option<String>,
    network: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ScriptEnvelope {
    ok: bool,
    status: u16,
    payer: String,
    endpoint: String,
    payment_requirement: Option<ScriptPaymentRequirement>,
    payment_response: Option<ScriptPaymentResponse>,
    body_json: Option<Value>,
    body_text: String,
}

fn script_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join("heurist_x402_call.cjs")
}

fn build_query(token_address: &str, symbol_hint: &str) -> String {
    let trimmed = symbol_hint.trim();
    if trimmed.is_empty() {
        token_address.to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_asset_label(asset: &str) -> String {
    if asset.eq_ignore_ascii_case("USDC") || asset.eq_ignore_ascii_case(BASE_USDC_ADDRESS) {
        UPSTREAM_ASSET.to_string()
    } else {
        asset.to_string()
    }
}

fn display_amount(amount_units: &str, asset: &str) -> String {
    if asset.eq_ignore_ascii_case(UPSTREAM_ASSET) {
        match amount_units.parse::<u64>() {
            Ok(raw) => format!("{:.6} {}", raw as f64 / 1_000_000.0, asset),
            Err(_) => format!("{amount_units} {asset}"),
        }
    } else {
        format!("{amount_units} {asset}")
    }
}

fn infer_cost_cents(amount_units: &str, asset: &str) -> u32 {
    if !asset.eq_ignore_ascii_case(UPSTREAM_ASSET) {
        return 0;
    }

    let raw = amount_units.parse::<u64>().unwrap_or(0);
    ((raw as f64 / 1_000_000.0) * 100.0).round() as u32
}

fn build_paid_requests(token_address: &str, symbol_hint: &str) -> Vec<PaidHeuristRequest> {
    let query = build_query(token_address, symbol_hint);
    let web_query = format!("{query} {token_address} token launch footprint");

    vec![
        PaidHeuristRequest {
            section_id: "token_profile",
            title: "Paid token profile",
            agent_id: "TokenResolverAgent",
            tool_name: "token_search",
            query: token_address.to_string(),
            payload: json!({
                "query": token_address,
                "limit": 1,
            }),
        },
        PaidHeuristRequest {
            section_id: "market_trend",
            title: "Paid market context",
            agent_id: "TrendingTokenAgent",
            tool_name: "get_market_summary",
            query: "Latest crypto market summary".to_string(),
            payload: json!({}),
        },
        PaidHeuristRequest {
            section_id: "x_signal",
            title: "Paid X signal",
            agent_id: "TwitterIntelligenceAgent",
            tool_name: "twitter_search",
            query: query.clone(),
            payload: json!({
                "queries": [query],
                "limit": 10,
            }),
        },
        PaidHeuristRequest {
            section_id: "web_digest",
            title: "Paid web digest",
            agent_id: "ExaSearchDigestAgent",
            tool_name: "exa_web_search",
            query: web_query.clone(),
            payload: json!({
                "search_term": web_query,
                "time_filter": "past_month",
                "limit": 6,
            }),
        },
    ]
}

fn build_payment_trace(
    request: &PaidHeuristRequest,
    envelope: &ScriptEnvelope,
) -> Result<HeuristPaymentTrace> {
    let payment = envelope
        .payment_response
        .as_ref()
        .context("paid Heurist call returned without a payment response")?;
    if !payment.success {
        return Err(anyhow::anyhow!(
            "Heurist payment header returned success=false"
        ));
    }

    let requirement = envelope
        .payment_requirement
        .as_ref()
        .context("paid Heurist call returned without a payment requirement")?;
    let amount_units = requirement
        .max_amount_required
        .clone()
        .unwrap_or_else(|| "0".to_string());
    let asset = normalize_asset_label(
        &requirement
            .asset
            .clone()
            .unwrap_or_else(|| UPSTREAM_ASSET.to_string()),
    );
    let amount_display = display_amount(&amount_units, &asset);
    let cost_cents = infer_cost_cents(&amount_units, &asset);

    Ok(HeuristPaymentTrace {
        provider: "heurist_mesh_x402".to_string(),
        agent_id: request.agent_id.to_string(),
        tool_name: request.tool_name.to_string(),
        network: payment
            .network
            .clone()
            .or_else(|| requirement.network.clone())
            .unwrap_or_else(|| UPSTREAM_NETWORK.to_string()),
        asset,
        amount_units,
        amount_display,
        cost_cents,
        payment_tx: payment.transaction.clone(),
    })
}

fn build_agent_result(request: &PaidHeuristRequest, value: Value) -> HeuristAgentResult {
    HeuristAgentResult {
        section_id: request.section_id.to_string(),
        title: request.title.to_string(),
        agent_id: request.agent_id.to_string(),
        query: request.query.clone(),
        summary: extract_summary_text(&value),
        raw_result: value,
    }
}

fn build_source_status(results: &[PaidHeuristResult], agent_set: &str) -> Value {
    json!({
        "provider": "Heurist Mesh x402",
        "agent_set": agent_set,
        "result_count": results.len(),
        "payments": results.iter().map(|item| json!({
            "agent_id": item.payment.agent_id,
            "tool_name": item.payment.tool_name,
            "amount_display": item.payment.amount_display,
            "amount_units": item.payment.amount_units,
            "asset": item.payment.asset,
            "network": item.payment.network,
            "payment_tx": item.payment.payment_tx,
            "endpoint": item.endpoint,
            "payer": item.payer,
        })).collect::<Vec<_>>(),
    })
}

fn build_executive_summary(results: &[PaidHeuristResult]) -> String {
    results
        .iter()
        .map(|item| format!("{}: {}", item.result.title, item.result.summary))
        .collect::<Vec<_>>()
        .join(" ")
}

async fn run_paid_request(
    config: &Config,
    request: &PaidHeuristRequest,
) -> Result<PaidHeuristResult> {
    let wallet_dir = config
        .heurist_x402_wallet_dir
        .as_ref()
        .context("HEURIST_X402_WALLET_DIR or AGENT_WALLET_DIR must be set")?;
    let wallet_password = config
        .heurist_x402_wallet_password
        .as_ref()
        .context("HEURIST_X402_WALLET_PASSWORD or AGENT_WALLET_PASSWORD must be set")?;

    let output = Command::new("node")
        .arg(script_path())
        .arg(request.agent_id)
        .arg(request.tool_name)
        .arg(request.payload.to_string())
        .env("HEURIST_X402_BASE_URL", config.heurist_mesh_api_url.clone())
        .env("HEURIST_X402_WALLET_DIR", wallet_dir)
        .env(
            "HEURIST_X402_WALLET_ID",
            config.heurist_x402_wallet_id.clone(),
        )
        .env("HEURIST_X402_WALLET_PASSWORD", wallet_password)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("failed to launch Heurist x402 helper")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Err(anyhow::anyhow!(
            "Heurist x402 helper failed: {}",
            if !stderr.is_empty() { stderr } else { stdout }
        ));
    }

    let stdout =
        String::from_utf8(output.stdout).context("Heurist x402 helper returned invalid UTF-8")?;
    let envelope: ScriptEnvelope = serde_json::from_str(&stdout)
        .with_context(|| format!("Heurist x402 helper returned invalid JSON: {stdout}"))?;

    if !envelope.ok || envelope.status != 200 {
        return Err(anyhow::anyhow!(
            "Heurist x402 helper returned non-success status {}: {}",
            envelope.status,
            envelope.body_text
        ));
    }

    let body_json = envelope
        .body_json
        .clone()
        .context("Heurist x402 helper returned an empty JSON body")?;

    Ok(PaidHeuristResult {
        result: build_agent_result(request, body_json),
        payment: build_payment_trace(request, &envelope)?,
        endpoint: envelope.endpoint,
        payer: envelope.payer,
    })
}

pub async fn run_paid_mvp_dossier(
    config: &Config,
    token_address: &str,
    symbol_hint: &str,
) -> Result<(HeuristDossier, Vec<HeuristPaymentTrace>)> {
    let requests = build_paid_requests(token_address, symbol_hint);
    let mut results = Vec::with_capacity(requests.len());

    for request in &requests {
        results.push(run_paid_request(config, request).await?);
    }

    let payment_traces = results
        .iter()
        .map(|item| item.payment.clone())
        .collect::<Vec<_>>();

    let dossier = HeuristDossier {
        executive_summary: build_executive_summary(&results),
        results: results.iter().map(|item| item.result.clone()).collect(),
        citations: Vec::new(),
        source_status: build_source_status(&results, &config.heurist_mesh_agent_set),
        raw_payload: json!({
            "provider": "Heurist Mesh x402",
            "results": results.iter().map(|item| item.result.raw_result.clone()).collect::<Vec<_>>(),
        }),
    };

    Ok((dossier, payment_traces))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        build_agent_result, build_query, display_amount, infer_cost_cents, normalize_asset_label,
        BASE_USDC_ADDRESS,
    };

    #[test]
    fn build_query_prefers_symbol_hint_when_available() {
        assert_eq!(build_query("0x123", "PEPE"), "PEPE");
        assert_eq!(build_query("0x123", "   "), "0x123");
    }

    #[test]
    fn build_agent_result_extracts_summary_from_body() {
        let result = build_agent_result(
            &super::PaidHeuristRequest {
                section_id: "token_profile",
                title: "Paid token profile",
                agent_id: "TokenResolverAgent",
                tool_name: "token_search",
                query: "PEPE".to_string(),
                payload: json!({}),
            },
            json!({
                "result": {
                    "data": {
                        "results": [
                            {
                                "name": "Pepe",
                                "summary": "Large-cap meme token with active DEX liquidity."
                            }
                        ]
                    }
                }
            }),
        );

        assert_eq!(result.section_id, "token_profile");
        assert!(result.summary.contains("Large-cap meme token"));
    }

    #[test]
    fn payment_display_for_usdc_keeps_six_decimals() {
        assert_eq!(display_amount("2000", "USDC"), "0.002000 USDC");
        assert_eq!(infer_cost_cents("10000", "USDC"), 1);
        assert_eq!(normalize_asset_label(BASE_USDC_ADDRESS), "USDC");
    }
}
