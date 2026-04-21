use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    api::investigation_runs::InvestigationRunSummary,
    api::verdict::VerdictResponse,
    research::{
        decision_scorecard::DecisionScorecard,
        launch_intelligence::{
            DeployerMemorySummary, OperatorFamilySummary, WalletStructureSummary,
        },
    },
};

#[derive(Debug, Serialize)]
pub struct InvestigationResponse {
    pub token_address: String,
    pub generated_at: DateTime<Utc>,
    pub active_run: Option<InvestigationRunSummary>,
    pub deep_research: InvestigationDeepResearchState,
    pub internal: PublicInternalEvidence,
    pub contract_intelligence: ContractIntelligence,
    pub market_intelligence: MarketIntelligence,
    pub analysis: InvestigationAnalysis,
    pub source_status: SourceStatus,
}

#[derive(Debug, Deserialize)]
pub struct AskMiaRequest {
    pub question: String,
    pub run_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct AskMiaAnswer {
    pub short_answer: String,
    pub why: String,
    pub evidence: Vec<String>,
    pub next_move: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AskMiaTraceStep {
    pub tool: String,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AskMiaRunContextEvent {
    pub label: String,
    pub detail: String,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AskMiaRunContext {
    pub run_id: Uuid,
    pub status: String,
    pub current_stage: String,
    pub continuity_note: String,
    pub latest_reason: Option<String>,
    pub latest_evidence_delta: Option<String>,
    pub recent_events: Vec<AskMiaRunContextEvent>,
}

#[derive(Debug, Serialize)]
pub struct AskMiaResponse {
    pub token_address: String,
    pub question: String,
    pub generated_at: DateTime<Utc>,
    pub mode: String,
    pub provider: String,
    pub grounded_layers: Vec<String>,
    pub tool_trace: Vec<String>,
    pub analysis_trace: Vec<AskMiaTraceStep>,
    pub run_context: Option<AskMiaRunContext>,
    pub answer: AskMiaAnswer,
    pub fallback_used: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct InvestigationDeepResearchState {
    pub report_cached: bool,
    pub report_generated_at: Option<DateTime<Utc>>,
    pub auto_threshold_met: bool,
    pub auto_threshold_tx_count: i64,
    pub auto_requested: bool,
    pub ai_score_enabled: bool,
    pub ai_score_gate_tx_count: i64,
    pub score_enriched: bool,
}

#[derive(Debug, Serialize)]
pub struct InternalEvidence {
    pub token: TokenSnapshot,
    pub risk: Option<RiskSnapshot>,
    pub agent_scorecard: Option<AgentScorecard>,
    pub verdict: VerdictResponse,
    pub narrative_cache: Option<NarrativeCacheSnapshot>,
    pub deployer: Option<DeployerSnapshot>,
    pub deployer_recent_tokens: Vec<DeployerTokenSnapshot>,
    pub recent_transactions: Vec<TransactionSnapshot>,
    pub whale_activity_24h: WhaleActivitySnapshot,
    pub alpha_context: Option<AlphaContextSnapshot>,
    pub wallet_structure: WalletStructureSummary,
    pub deployer_memory: Option<DeployerMemorySummary>,
    pub operator_family: OperatorFamilySummary,
    pub decision_scorecard: DecisionScorecard,
}

#[derive(Debug, Serialize)]
pub struct PublicInternalEvidence {
    pub token: TokenSnapshot,
    pub risk: Option<RiskSnapshot>,
    pub agent_scorecard: Option<AgentScorecard>,
    pub deployer: Option<DeployerSnapshot>,
    pub deployer_recent_tokens: Vec<DeployerTokenSnapshot>,
    pub recent_transactions: Vec<TransactionSnapshot>,
    pub whale_activity_24h: WhaleActivitySnapshot,
    pub alpha_context: Option<AlphaContextSnapshot>,
    pub wallet_structure: WalletStructureSummary,
    pub deployer_memory: Option<DeployerMemorySummary>,
    pub operator_family: OperatorFamilySummary,
}

impl From<InternalEvidence> for PublicInternalEvidence {
    fn from(value: InternalEvidence) -> Self {
        Self {
            token: value.token,
            risk: value.risk,
            agent_scorecard: value.agent_scorecard,
            deployer: value.deployer,
            deployer_recent_tokens: value.deployer_recent_tokens,
            recent_transactions: value.recent_transactions,
            whale_activity_24h: value.whale_activity_24h,
            alpha_context: value.alpha_context,
            wallet_structure: value.wallet_structure,
            deployer_memory: value.deployer_memory,
            operator_family: value.operator_family,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentScorecard {
    pub score: i16,
    pub label: String,
    pub confidence_label: String,
    pub headline: String,
    pub summary: String,
    pub primary_reason: String,
    pub primary_risk: String,
    pub supporting_points: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenSnapshot {
    pub contract_address: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub deployer_address: String,
    pub deployed_at: DateTime<Utc>,
    pub block_number: i64,
    pub tx_hash: String,
    pub initial_liquidity_bnb: Option<f64>,
    pub participant_wallet_count: i32,
    pub holder_count: i32,
    pub buy_count: i32,
    pub sell_count: i32,
    pub volume_bnb: f64,
    pub is_rug: bool,
    pub graduated: bool,
    pub honeypot_detected: bool,
}

#[derive(Debug, Serialize)]
pub struct RiskSnapshot {
    pub composite_score: i16,
    pub risk_category: String,
    pub deployer_history_score: Option<i16>,
    pub liquidity_lock_score: Option<i16>,
    pub wallet_concentration_score: Option<i16>,
    pub buy_sell_velocity_score: Option<i16>,
    pub contract_audit_score: Option<i16>,
    pub social_authenticity_score: Option<i16>,
    pub volume_consistency_score: Option<i16>,
    pub computed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct NarrativeCacheSnapshot {
    pub narrative_text: String,
    pub risk_interpretation: Option<String>,
    pub consensus_status: String,
    pub confidence: String,
    pub generated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct DeployerSnapshot {
    pub address: String,
    pub total_tokens_deployed: i64,
    pub rug_count: i64,
    pub graduated_count: i64,
    pub honeypot_detected: bool,
    pub trust_grade: String,
    pub trust_label: String,
    pub first_seen_at: Option<DateTime<Utc>>,
    pub last_seen_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct DeployerTokenSnapshot {
    pub contract_address: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub deployed_at: DateTime<Utc>,
    pub buy_count: i32,
    pub sell_count: i32,
    pub volume_bnb: f64,
    pub composite_score: Option<i16>,
    pub risk_category: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TransactionSnapshot {
    pub wallet_address: String,
    pub tx_hash: String,
    pub tx_type: String,
    pub amount_bnb: f64,
    pub block_number: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct WhaleActivitySnapshot {
    pub watch_alerts: usize,
    pub critical_alerts: usize,
    pub latest_levels: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AlphaContextSnapshot {
    pub rank: i16,
    pub alpha_score: f64,
    pub rationale: String,
    pub window_end: DateTime<Utc>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct ContractIntelligence {
    pub provider: String,
    pub available: bool,
    pub source_verified: bool,
    pub contract_name: Option<String>,
    pub compiler_version: Option<String>,
    pub optimization_used: Option<bool>,
    pub optimization_runs: Option<i64>,
    pub proxy: Option<bool>,
    pub implementation: Option<String>,
    pub token_type: Option<String>,
    pub total_supply: Option<String>,
    pub total_supply_raw: Option<String>,
    pub decimals: Option<u32>,
    pub indexed_holder_count: Option<u64>,
    pub holder_count: Option<u64>,
    pub description: Option<String>,
    pub website: Option<String>,
    pub twitter: Option<String>,
    pub telegram: Option<String>,
    pub discord: Option<String>,
    pub owner_holding_pct: Option<f64>,
    pub owner_in_top_holders: bool,
    pub holder_supply: Option<HolderSupplySnapshot>,
    pub holder_change: Option<HolderChangeSnapshot>,
    pub holder_distribution: Option<HolderDistributionSnapshot>,
    pub holders_by_acquisition: Option<HolderAcquisitionSnapshot>,
    pub top_holders: Vec<HolderSnapshot>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HolderSnapshot {
    pub address: String,
    pub quantity: String,
    pub quantity_raw: String,
    pub ownership_pct: Option<f64>,
    pub is_owner: bool,
    pub address_type: Option<String>,
    pub owner_label: Option<String>,
    pub entity: Option<String>,
    pub is_contract: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HolderSupplyBandSnapshot {
    pub supply: Option<String>,
    pub supply_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HolderSupplySnapshot {
    pub top10: HolderSupplyBandSnapshot,
    pub top25: HolderSupplyBandSnapshot,
    pub top50: HolderSupplyBandSnapshot,
    pub top100: HolderSupplyBandSnapshot,
}

#[derive(Debug, Clone, Serialize)]
pub struct HolderChangeWindowSnapshot {
    pub change: Option<i64>,
    pub change_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HolderChangeSnapshot {
    pub one_hour: HolderChangeWindowSnapshot,
    pub twenty_four_hours: HolderChangeWindowSnapshot,
    pub seven_days: HolderChangeWindowSnapshot,
}

#[derive(Debug, Clone, Serialize)]
pub struct HolderDistributionSnapshot {
    pub whales: Option<u64>,
    pub sharks: Option<u64>,
    pub dolphins: Option<u64>,
    pub fish: Option<u64>,
    pub octopus: Option<u64>,
    pub crabs: Option<u64>,
    pub shrimps: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HolderAcquisitionSnapshot {
    pub swap: Option<u64>,
    pub transfer: Option<u64>,
    pub airdrop: Option<u64>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct MarketIntelligence {
    pub provider: String,
    pub available: bool,
    pub x_summary: Option<String>,
    pub web_summary: Option<String>,
    pub active_event: Option<String>,
    pub narrative_alignment: Option<String>,
    pub excitement_score: Option<i16>,
    pub risk_flags: Vec<String>,
    pub sources: Vec<InvestigationSource>,
    pub raw_summary: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct InvestigationSource {
    pub title: String,
    pub url: String,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct InvestigationAnalysis {
    pub provider: String,
    pub score: Option<i16>,
    pub label: Option<String>,
    pub verdict: String,
    pub conviction: String,
    pub confidence: String,
    pub executive_summary: String,
    pub primary_reason: String,
    pub primary_risk: String,
    pub supporting_points: Vec<String>,
    pub thesis: Vec<String>,
    pub risks: Vec<String>,
    pub next_actions: Vec<String>,
    pub raw: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SourceStatus {
    pub bscscan_configured: bool,
    pub market_provider: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EtherscanEnvelope<T> {
    pub status: String,
    pub message: String,
    pub result: T,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EtherscanTokenInfoRow {
    #[serde(rename = "tokenName")]
    pub token_name: Option<String>,
    pub symbol: Option<String>,
    #[serde(rename = "divisor")]
    pub divisor: Option<String>,
    #[serde(rename = "tokenType")]
    pub token_type: Option<String>,
    #[serde(rename = "totalSupply")]
    pub total_supply: Option<String>,
    pub description: Option<String>,
    pub website: Option<String>,
    pub twitter: Option<String>,
    pub telegram: Option<String>,
    pub discord: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EtherscanSourceCodeRow {
    #[serde(rename = "ContractName")]
    pub contract_name: Option<String>,
    #[serde(rename = "CompilerVersion")]
    pub compiler_version: Option<String>,
    #[serde(rename = "OptimizationUsed")]
    pub optimization_used: Option<String>,
    #[serde(rename = "Runs")]
    pub runs: Option<String>,
    #[serde(rename = "Proxy")]
    pub proxy: Option<String>,
    #[serde(rename = "Implementation")]
    pub implementation: Option<String>,
    #[serde(rename = "SourceCode")]
    pub source_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MoralisWalletTokenBalancesResponse {
    pub result: Vec<MoralisWalletTokenBalanceRow>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MoralisWalletTokenBalanceRow {
    pub token_address: String,
    pub balance: String,
    pub total_supply: Option<String>,
    pub total_supply_formatted: Option<String>,
    pub percentage_relative_to_total_supply: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MoralisTokenOwnersResponse {
    pub result: Vec<MoralisTokenOwnerRow>,
    pub total_supply: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MoralisTokenOwnerRow {
    pub owner_address: String,
    pub owner_address_label: Option<String>,
    pub balance: String,
    pub balance_formatted: Option<String>,
    pub is_contract: Option<bool>,
    pub percentage_relative_to_total_supply: Option<f64>,
    pub entity: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MoralisTokenHolderStatsResponse {
    #[serde(rename = "totalHolders")]
    pub total_holders: Option<Value>,
    #[serde(rename = "holderSupply")]
    pub holder_supply: MoralisHolderSupply,
    #[serde(rename = "holderChange")]
    pub holder_change: MoralisHolderChange,
    #[serde(rename = "holdersByAcquisition")]
    pub holders_by_acquisition: MoralisHolderAcquisition,
    #[serde(rename = "holderDistribution")]
    pub holder_distribution: MoralisHolderDistribution,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MoralisSupplyBand {
    pub supply: Option<Value>,
    #[serde(rename = "supplyPercent")]
    pub supply_percent: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MoralisHolderSupply {
    pub top10: MoralisSupplyBand,
    pub top25: MoralisSupplyBand,
    pub top50: MoralisSupplyBand,
    pub top100: MoralisSupplyBand,
    #[serde(rename = "top250")]
    pub _top250: Option<MoralisSupplyBand>,
    #[serde(rename = "top500")]
    pub _top500: Option<MoralisSupplyBand>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MoralisChangeWindow {
    pub change: Option<Value>,
    #[serde(rename = "changePercent")]
    pub change_percent: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MoralisHolderChange {
    #[serde(rename = "5min")]
    pub five_min: Option<MoralisChangeWindow>,
    #[serde(rename = "10min")]
    pub ten_min: Option<MoralisChangeWindow>,
    #[serde(rename = "1h")]
    pub one_hour: Option<MoralisChangeWindow>,
    #[serde(rename = "6h")]
    pub six_hours: Option<MoralisChangeWindow>,
    #[serde(rename = "24h")]
    pub twenty_four_hours: Option<MoralisChangeWindow>,
    #[serde(rename = "3d")]
    pub three_days: Option<MoralisChangeWindow>,
    #[serde(rename = "7d")]
    pub seven_days: Option<MoralisChangeWindow>,
    #[serde(rename = "30d")]
    pub thirty_days: Option<MoralisChangeWindow>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MoralisHolderAcquisition {
    pub swap: Option<Value>,
    pub transfer: Option<Value>,
    pub airdrop: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MoralisHolderDistribution {
    pub whales: Option<Value>,
    pub sharks: Option<Value>,
    pub dolphins: Option<Value>,
    pub fish: Option<Value>,
    pub octopus: Option<Value>,
    pub crabs: Option<Value>,
    pub shrimps: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AnalysisPayload {
    pub score: i16,
    pub label: String,
    pub verdict: String,
    pub conviction: Value,
    pub confidence: Value,
    pub executive_summary: String,
    pub primary_reason: String,
    pub primary_risk: String,
    pub supporting_points: Vec<String>,
    pub thesis: Vec<String>,
    pub risks: Vec<String>,
    pub next_actions: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AskMiaPayload {
    pub short_answer: String,
    pub why: String,
    pub evidence: Vec<String>,
    pub next_move: String,
}

#[cfg(test)]
mod tests {
    use super::MoralisTokenHolderStatsResponse;

    #[test]
    fn parses_moralis_holder_stats_with_extra_windows_and_bands() {
        let raw = r#"{
            "totalHolders": 4,
            "holdersByAcquisition": { "swap": 1, "transfer": 3, "airdrop": 0 },
            "holderChange": {
                "5min": { "change": 0, "changePercent": 0 },
                "1h": { "change": 0, "changePercent": 0 },
                "6h": { "change": 0, "changePercent": 0 },
                "24h": { "change": 4, "changePercent": 100 },
                "3d": { "change": 4, "changePercent": 100 },
                "7d": { "change": 4, "changePercent": 100 },
                "30d": { "change": 4, "changePercent": 100 }
            },
            "holderSupply": {
                "top10": { "supply": "1000000000", "supplyPercent": 100 },
                "top25": { "supply": "1000000000", "supplyPercent": 100 },
                "top50": { "supply": "1000000000", "supplyPercent": 100 },
                "top100": { "supply": "1000000000", "supplyPercent": 100 },
                "top250": { "supply": "1000000000", "supplyPercent": 100 },
                "top500": { "supply": "1000000000", "supplyPercent": 100 }
            },
            "holderDistribution": {
                "whales": 3,
                "sharks": 0,
                "dolphins": 1,
                "fish": 0,
                "octopus": 0,
                "crabs": 0,
                "shrimps": 0
            }
        }"#;

        let parsed: MoralisTokenHolderStatsResponse = serde_json::from_str(raw).unwrap();
        assert!(parsed.holder_change.five_min.is_some());
        assert!(parsed.holder_supply._top250.is_some());
        assert!(parsed.holder_supply._top500.is_some());
    }
}
