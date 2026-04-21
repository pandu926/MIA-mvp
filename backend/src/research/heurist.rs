use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::Config;

#[derive(Debug, Clone, Serialize)]
pub struct HeuristAgentRequest {
    pub section_id: &'static str,
    pub title: &'static str,
    pub agent_id: &'static str,
    pub query: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HeuristAgentResult {
    pub section_id: String,
    pub title: String,
    pub agent_id: String,
    pub query: String,
    pub summary: String,
    pub raw_result: Value,
}

#[derive(Debug, Deserialize)]
struct HeuristMeshEnvelope {
    result: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct HeuristDossier {
    pub executive_summary: String,
    pub results: Vec<HeuristAgentResult>,
    pub citations: Vec<Value>,
    pub source_status: Value,
    pub raw_payload: Value,
}

pub fn build_mvp_agent_requests(
    token_address: &str,
    symbol_hint: &str,
) -> Vec<HeuristAgentRequest> {
    let symbol_hint = if symbol_hint.trim().is_empty() {
        token_address.to_string()
    } else {
        format!("{symbol_hint} ({token_address})")
    };

    vec![
        HeuristAgentRequest {
            section_id: "token_profile",
            title: "Token profile",
            agent_id: "TokenResolverAgent",
            query: format!(
                "Resolve token {symbol_hint}. Return the normalized token profile, socials, top DEX pools, and anything that helps identify the asset quickly in a live launch workflow."
            ),
        },
        HeuristAgentRequest {
            section_id: "market_trend",
            title: "Market and trend context",
            agent_id: "TrendingTokenAgent",
            query: format!(
                "Assess whether {symbol_hint} is trending. Summarize cross-source momentum using DexScreener, social chatter, and market context."
            ),
        },
        HeuristAgentRequest {
            section_id: "x_signal",
            title: "X signal",
            agent_id: "TwitterIntelligenceAgent",
            query: format!(
                "Find high-signal Twitter or X discussion about {symbol_hint}. Summarize narrative alignment, skepticism, key accounts, and any red flags."
            ),
        },
        HeuristAgentRequest {
            section_id: "web_digest",
            title: "Web digest",
            agent_id: "ExaSearchDigestAgent",
            query: format!(
                "Search the web for {symbol_hint}. Summarize website claims, launch notes, community footprint, known promoter context, and obvious risk signals with citations when possible."
            ),
        },
    ]
}

pub fn extract_summary_text(value: &Value) -> String {
    fn walk(value: &Value, out: &mut Vec<String>) {
        match value {
            Value::String(text) => {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    out.push(trimmed.to_string());
                }
            }
            Value::Array(items) => {
                for item in items {
                    walk(item, out);
                }
            }
            Value::Object(map) => {
                for (key, nested) in map {
                    if matches!(
                        key.as_str(),
                        "summary"
                            | "snippet"
                            | "content"
                            | "text"
                            | "analysis"
                            | "answer"
                            | "title"
                    ) {
                        walk(nested, out);
                    }
                }
                for nested in map.values() {
                    walk(nested, out);
                }
            }
            _ => {}
        }
    }

    let mut collected = Vec::new();
    walk(value, &mut collected);

    collected
        .into_iter()
        .filter(|item| !item.is_empty())
        .take(10)
        .collect::<Vec<_>>()
        .join(" ")
}

pub async fn run_mvp_dossier(
    config: &Config,
    token_address: &str,
    symbol_hint: &str,
) -> Result<HeuristDossier> {
    let api_key = config
        .heurist_api_key
        .as_ref()
        .context("HEURIST_API_KEY must be set to generate a Deep Research dossier")?;

    let client = Client::builder()
        .timeout(Duration::from_secs(25))
        .build()
        .context("failed to build Heurist HTTP client")?;

    let base_url = config.heurist_mesh_api_url.trim_end_matches('/');
    let endpoint = format!("{base_url}/mesh_request");
    let requests = build_mvp_agent_requests(token_address, symbol_hint);

    let mut results = Vec::new();

    for request in &requests {
        let response = client
            .post(&endpoint)
            .json(&json!({
                "api_key": api_key,
                "agent_id": request.agent_id,
                "input": {
                    "query": request.query,
                    "raw_data_only": false
                }
            }))
            .send()
            .await
            .with_context(|| format!("Heurist request failed for {}", request.agent_id))?;

        let status = response.status();
        let body = response.text().await.with_context(|| {
            format!("Heurist response body unavailable for {}", request.agent_id)
        })?;

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "Heurist agent {} returned {}: {}",
                request.agent_id,
                status,
                body
            ));
        }

        let envelope: HeuristMeshEnvelope = serde_json::from_str(&body).with_context(|| {
            format!(
                "Heurist response is not valid JSON for {}",
                request.agent_id
            )
        })?;
        let summary = extract_summary_text(&envelope.result);

        results.push(HeuristAgentResult {
            section_id: request.section_id.to_string(),
            title: request.title.to_string(),
            agent_id: request.agent_id.to_string(),
            query: request.query.clone(),
            summary,
            raw_result: envelope.result,
        });
    }

    let executive_summary = results
        .iter()
        .map(|item| format!("{}: {}", item.title, item.summary))
        .collect::<Vec<_>>()
        .join(" ");

    Ok(HeuristDossier {
        executive_summary,
        citations: Vec::new(),
        source_status: json!({
            "provider": "Heurist Mesh REST API",
            "agent_set": config.heurist_mesh_agent_set,
            "result_count": results.len(),
        }),
        raw_payload: json!({
            "provider": "Heurist Mesh REST API",
            "results": results,
        }),
        results,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{build_mvp_agent_requests, extract_summary_text};

    #[test]
    fn build_mvp_agent_requests_uses_expected_agent_pack() {
        let requests = build_mvp_agent_requests("0x123", "TEST");

        assert_eq!(requests.len(), 4);
        assert_eq!(requests[0].agent_id, "TokenResolverAgent");
        assert!(requests
            .iter()
            .any(|item| item.agent_id == "TwitterIntelligenceAgent"));
        assert!(requests.iter().any(|item| item.query.contains("0x123")));
    }

    #[test]
    fn extract_summary_text_flattens_nested_textual_result() {
        let value = json!({
            "summary": "Momentum is building",
            "sources": [
                { "title": "Thread", "snippet": "Smart money is watching this token." },
                { "title": "Digest", "content": "Dex liquidity and socials are accelerating." }
            ]
        });

        let summary = extract_summary_text(&value);

        assert!(summary.contains("Momentum is building"));
        assert!(summary.contains("Smart money is watching this token."));
        assert!(summary.contains("Dex liquidity and socials are accelerating."));
    }
}
