use serde::Serialize;

/// Ordered list of GPT models to try (primary first).
/// Used as the default when `LLM_MODELS` env var is not set.
pub(crate) const DEFAULT_LLM_MODELS: &str =
    "gpt-5.4,gpt-5.2,gpt-5.4-mini";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MlRolloutMode {
    Legacy,
    Shadow,
    Ml,
    Hybrid,
}

impl MlRolloutMode {
    pub(crate) fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "legacy" => Some(Self::Legacy),
            "shadow" => Some(Self::Shadow),
            "ml" => Some(Self::Ml),
            "hybrid" => Some(Self::Hybrid),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeepResearchProvider {
    HeuristMeshX402,
    NativeXApi,
}

impl DeepResearchProvider {
    pub(crate) fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "heurist_mesh_x402" => Some(Self::HeuristMeshX402),
            "native_x_api" => Some(Self::NativeXApi),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HeuristMeshX402 => "heurist_mesh_x402",
            Self::NativeXApi => "native_x_api",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeepResearchUnlockModel {
    UnlockThisReport,
    DayPass,
}

impl DeepResearchUnlockModel {
    pub(crate) fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "unlock_this_report" => Some(Self::UnlockThisReport),
            "day_pass" => Some(Self::DayPass),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UnlockThisReport => "unlock_this_report",
            Self::DayPass => "day_pass",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub bnb_rpc_ws_url: String,
    pub bnb_rpc_ws_urls: Vec<String>,
    pub four_meme_contract_address: String,
    pub app_base_url: String,
    pub allowed_origins: Vec<String>,
    pub log_level: String,
    pub server_port: u16,
    pub llm_api_url: String,
    pub llm_api_key: String,
    pub llm_models: Vec<String>,
    pub ai_cache_ttl_secs: u64,
    pub ai_buy_threshold: u64,
    pub ai_threshold_window_secs: u64,
    pub whale_alert_threshold_bnb: f64,
    pub alpha_refresh_secs: u64,
    pub alpha_top_k: i64,
    pub indexer_deployment_backfill_enabled: bool,
    pub auto_investigation_enabled: bool,
    pub auto_investigation_interval_secs: u64,
    pub auto_investigation_tx_threshold: i64,
    pub auto_investigation_cooldown_mins: i64,
    pub auto_investigation_max_runs_per_scan: i64,
    pub ai_score_min_tx_count: i64,
    pub auto_deep_research_tx_threshold: i64,
    pub investigation_fixture_api_enabled: bool,
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub ml_rollout_mode: MlRolloutMode,
    pub ml_model_version: String,
    pub ml_min_confidence: f64,
    pub moralis_api_key: Option<String>,
    pub moralis_api_url: String,
    pub bscscan_api_key: Option<String>,
    pub bscscan_api_url: String,
    pub bscscan_chain_id: u64,
    pub deep_research_enabled: bool,
    pub ask_mia_function_calling_enabled: bool,
    pub deep_research_provider: DeepResearchProvider,
    pub deep_research_unlock_model: DeepResearchUnlockModel,
    pub x402_enabled: bool,
    pub x402_facilitator_url: Option<String>,
    pub x402_facilitator_api_key: Option<String>,
    pub x402_pay_to: Option<String>,
    pub x402_asset_address: Option<String>,
    pub x402_network: String,
    pub x402_scheme: String,
    pub x402_facilitator_id: Option<String>,
    pub x402_fee_to: Option<String>,
    pub x402_caller: Option<String>,
    pub x402_fee_amount: String,
    pub x402_price_usdc_cents: u32,
    pub x402_max_timeout_secs: u32,
    pub heurist_mesh_api_url: String,
    pub heurist_mesh_agent_set: String,
    pub heurist_api_key: Option<String>,
    pub heurist_x402_wallet_dir: Option<String>,
    pub heurist_x402_wallet_id: String,
    pub heurist_x402_wallet_password: Option<String>,
}
