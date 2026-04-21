use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::config::{Config, DeepResearchProvider, DeepResearchUnlockModel};

#[derive(Debug, Serialize)]
pub struct DeepResearchSection {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub stage: String,
}

#[derive(Debug, Serialize)]
pub struct SybilPolicyDescriptor {
    pub wording: String,
    pub confidence_model: String,
    pub promise: String,
}

#[derive(Debug, Serialize)]
pub struct DeepResearchPreviewResponse {
    pub token_address: String,
    pub enabled: bool,
    pub provider_path: String,
    pub unlock_model: String,
    pub unlock_cta: String,
    pub payment_network: String,
    pub price_usdc_cents: u32,
    pub sections: Vec<DeepResearchSection>,
    pub sybil_policy: SybilPolicyDescriptor,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DeepResearchStatusResponse {
    pub token_address: String,
    pub premium_state: String,
    pub provider_path: String,
    pub unlock_model: String,
    pub x402_enabled: bool,
    pub native_x_api_reserved: bool,
    pub report_cached: bool,
    pub has_active_entitlement: bool,
    pub entitlement_expires_at: Option<DateTime<Utc>>,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DeepResearchReportSectionResponse {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub stage: String,
    pub source_agent: Option<String>,
    pub confidence: Option<String>,
    pub provider: Option<String>,
    pub source_url: Option<String>,
    pub observed_at: Option<String>,
    pub fallback_note: Option<String>,
    pub evidence: Option<Vec<String>>,
    pub related_tokens: Option<Vec<serde_json::Value>>,
    pub repeated_wallets: Option<Vec<String>>,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct DeepResearchEntitlementResponse {
    pub access_token: String,
    pub kind: String,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct DeepResearchReportResponse {
    pub token_address: String,
    pub provider_path: String,
    pub status: String,
    pub executive_summary: String,
    pub sections: Vec<DeepResearchReportSectionResponse>,
    pub citations: Vec<serde_json::Value>,
    pub source_status: serde_json::Value,
    pub generated_at: DateTime<Utc>,
    pub entitlement: Option<DeepResearchEntitlementResponse>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeepResearchRunStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeepResearchRunStage {
    Plan,
    GatherInternal,
    GatherExternal,
    Synthesize,
    Finalize,
}

#[derive(Debug, Serialize)]
pub struct DeepResearchRunResponse {
    pub run_id: Uuid,
    pub token_address: String,
    pub provider_path: String,
    pub status: DeepResearchRunStatus,
    pub current_phase: DeepResearchRunStage,
    pub budget_usage_cents: u32,
    pub paid_calls_count: u32,
    pub report_ready: bool,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct DeepResearchRunStepResponse {
    pub id: i64,
    pub step_key: String,
    pub title: String,
    pub status: DeepResearchRunStatus,
    pub agent_name: Option<String>,
    pub tool_name: Option<String>,
    pub summary: Option<String>,
    pub evidence: Vec<String>,
    pub cost_cents: u32,
    pub payment_tx: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct DeepResearchToolCallResponse {
    pub id: i64,
    pub step_key: String,
    pub tool_name: String,
    pub provider: Option<String>,
    pub status: DeepResearchRunStatus,
    pub summary: Option<String>,
    pub evidence: Vec<String>,
    pub latency_ms: Option<u32>,
    pub cost_cents: u32,
    pub payment_tx: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct DeepResearchPaymentLedgerResponse {
    pub id: i64,
    pub tool_call_id: i64,
    pub provider: String,
    pub network: String,
    pub asset: String,
    pub amount_units: String,
    pub amount_display: String,
    pub tx_hash: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct DeepResearchRunTraceResponse {
    pub run_id: Uuid,
    pub token_address: String,
    pub provider_path: String,
    pub status: DeepResearchRunStatus,
    pub current_phase: DeepResearchRunStage,
    pub budget_usage_cents: u32,
    pub paid_calls_count: u32,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub steps: Vec<DeepResearchRunStepResponse>,
    pub tool_calls: Vec<DeepResearchToolCallResponse>,
    pub payment_ledger: Vec<DeepResearchPaymentLedgerResponse>,
}

pub(crate) fn deep_research_provider_label(provider: DeepResearchProvider) -> &'static str {
    match provider {
        DeepResearchProvider::HeuristMeshX402 => {
            "MIA launch intelligence + optional narrative enrichment"
        }
        DeepResearchProvider::NativeXApi => "MIA launch intelligence",
    }
}

pub(crate) fn deep_research_resource_path(address: &str) -> String {
    format!("/api/v1/tokens/{address}/deep-research")
}

pub(crate) fn deep_research_runs_path(address: &str) -> String {
    format!("/api/v1/tokens/{address}/deep-research/runs")
}

pub(crate) fn deep_research_runs_resource_url_for_base(base_url: &str, address: &str) -> String {
    format!(
        "{}{}",
        base_url.trim_end_matches('/'),
        deep_research_runs_path(address)
    )
}

pub(crate) fn deep_research_unlock_resource_path(address: &str) -> String {
    format!("/api/v1/tokens/{address}/deep-research/unlock")
}

pub(crate) fn deep_research_resource_url_for_base(base_url: &str, address: &str) -> String {
    format!(
        "{}{}",
        base_url.trim_end_matches('/'),
        deep_research_resource_path(address)
    )
}

pub(crate) fn deep_research_unlock_resource_url_for_base(base_url: &str, address: &str) -> String {
    format!(
        "{}{}",
        base_url.trim_end_matches('/'),
        deep_research_unlock_resource_path(address)
    )
}

pub(crate) fn resource_url(config: &Config, token_address: &str) -> String {
    deep_research_resource_url_for_base(&config.app_base_url, token_address)
}

pub(crate) fn unlock_resource_url(config: &Config, token_address: &str) -> String {
    deep_research_unlock_resource_url_for_base(&config.app_base_url, token_address)
}

pub(crate) fn runs_resource_url(config: &Config, token_address: &str) -> String {
    deep_research_runs_resource_url_for_base(&config.app_base_url, token_address)
}

pub(crate) fn payment_description() -> &'static str {
    "Unlock MIA Deep Research report"
}

fn build_stage_sections() -> Vec<DeepResearchSection> {
    vec![
        DeepResearchSection {
            id: "dex-market".to_string(),
            title: "Dex market structure".to_string(),
            summary: "Attach pair quality, liquidity, transaction windows, and market-structure interpretation to the premium report.".to_string(),
            stage: "mvp".to_string(),
        },
        DeepResearchSection {
            id: "wallet-structure".to_string(),
            title: "Wallet structure".to_string(),
            summary: "Inspect active wallets, cluster concentration, repeated wallets, and internal flow concentration from MIA's indexed data.".to_string(),
            stage: "mvp".to_string(),
        },
        DeepResearchSection {
            id: "deployer-memory".to_string(),
            title: "Deployer memory".to_string(),
            summary: "Review the deployer's launch history, trust grade, rugs, graduates, and recent launch pattern inside MIA.".to_string(),
            stage: "mvp".to_string(),
        },
        DeepResearchSection {
            id: "linked-launches".to_string(),
            title: "Linked launch cluster".to_string(),
            summary: "Show likely linked deployer, wallet, and relaunch patterns with evidence and confidence.".to_string(),
            stage: "mvp".to_string(),
        },
        DeepResearchSection {
            id: "pattern-match-engine".to_string(),
            title: "Historical pattern context".to_string(),
            summary: "Compare the token against MIA's indexed launch history across 1H, 6H, and 24H with model scoring, analog retrieval, and anomaly checks. This layer is supporting context, not the final call.".to_string(),
            stage: "supporting".to_string(),
        },
        DeepResearchSection {
            id: "optional-narrative".to_string(),
            title: "Optional narrative enrichment".to_string(),
            summary: "Attach X and web context when the upstream narrative lane is available, without blocking the premium report.".to_string(),
            stage: "optional".to_string(),
        },
    ]
}

pub fn build_deep_research_preview(
    token_address: &str,
    config: &Config,
) -> DeepResearchPreviewResponse {
    DeepResearchPreviewResponse {
        token_address: token_address.to_string(),
        enabled: config.deep_research_enabled,
        provider_path: deep_research_provider_label(config.deep_research_provider).to_string(),
        unlock_model: config.deep_research_unlock_model.as_str().to_string(),
        unlock_cta: match config.deep_research_unlock_model {
            DeepResearchUnlockModel::UnlockThisReport => "Unlock this report".to_string(),
            DeepResearchUnlockModel::DayPass => "Unlock 24-hour pass".to_string(),
        },
        payment_network: config.x402_network.clone(),
        price_usdc_cents: config.x402_price_usdc_cents,
        sections: build_stage_sections(),
        sybil_policy: SybilPolicyDescriptor {
            wording: "Pattern detection, not identity certainty".to_string(),
            confidence_model: "low|medium|high".to_string(),
            promise: "Flag likely linked launch behavior without claiming a definitive identity match.".to_string(),
        },
        notes: vec![
            "The free workflow remains available before any premium unlock.".to_string(),
            "Users pay MIA through x402 on the configured seller network.".to_string(),
            "Premium is centered on launch intelligence: market structure, wallet structure, deployer memory, and linked launch evidence.".to_string(),
            "Deep Research is an evidence workspace. MIA surfaces and explains the data, while the final judgment stays with the user.".to_string(),
            "Narrative enrichment is optional and never blocks the premium report.".to_string(),
        ],
    }
}
