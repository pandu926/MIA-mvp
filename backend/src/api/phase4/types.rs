use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

#[derive(Debug, Deserialize)]
pub struct WhaleStreamQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    #[serde(default = "default_min_amount")]
    pub min_amount: f64,
    pub level: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WhaleNetworkQuery {
    #[serde(default = "default_hours")]
    pub hours: i64,
    #[serde(default = "default_min_amount")]
    pub min_amount: f64,
    pub level: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramConfigUpdateRequest {
    pub enabled: bool,
    pub chat_id: Option<String>,
    pub threshold_bnb: f64,
    pub alpha_digest_enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct AlphaHistoryQuery {
    #[serde(default = "default_hours")]
    pub hours: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

#[derive(Debug, Deserialize)]
pub struct AlphaBacktestQuery {
    #[serde(default = "default_hours")]
    pub hours: i64,
    #[serde(default = "default_backtest_limit")]
    pub limit: i64,
}

#[derive(Debug, Serialize)]
pub struct WhaleAlertResponse {
    pub token_address: String,
    pub wallet_address: String,
    pub tx_hash: String,
    pub amount_bnb: f64,
    pub threshold_bnb: f64,
    pub alert_level: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct WhaleStreamResponse {
    pub data: Vec<WhaleAlertResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct WhaleNetworkNode {
    pub id: String,
    pub label: String,
    pub node_type: String,
    pub wallet_address: Option<String>,
    pub token_address: Option<String>,
    pub total_volume_bnb: f64,
    pub tx_count: i64,
    pub critical_count: i64,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct WhaleNetworkEdge {
    pub source: String,
    pub target: String,
    pub tx_count: i64,
    pub total_volume_bnb: f64,
    pub last_tx_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct WhaleNetworkMetrics {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub total_volume_bnb: f64,
    pub critical_edges: usize,
}

#[derive(Debug, Serialize)]
pub struct WhaleNetworkResponse {
    pub nodes: Vec<WhaleNetworkNode>,
    pub edges: Vec<WhaleNetworkEdge>,
    pub metrics: WhaleNetworkMetrics,
    pub latest_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct TelegramConfigResponse {
    pub enabled: bool,
    pub chat_id: Option<String>,
    pub threshold_bnb: f64,
    pub alpha_digest_enabled: bool,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct AlphaRowResponse {
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub rank: i16,
    pub token_address: String,
    pub alpha_score: f64,
    pub rationale: String,
    pub score_source: String,
    pub model_version: String,
    pub score_confidence: Option<f64>,
    pub explanations: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DeployerTokenResponse {
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
pub struct AlphaBacktestRowResponse {
    pub window_end: DateTime<Utc>,
    pub rank: i16,
    pub token_address: String,
    pub alpha_score: f64,
    pub baseline_volume_1h: f64,
    pub future_volume_1h: f64,
    pub future_buy_count_1h: i64,
    pub future_sell_count_1h: i64,
    pub score_1h: f64,
    pub outcome_1h: String,
    pub future_volume_6h: f64,
    pub future_buy_count_6h: i64,
    pub future_sell_count_6h: i64,
    pub score_6h: f64,
    pub outcome_6h: String,
}

#[derive(Debug, Serialize)]
pub struct AlphaBacktestResponse {
    pub evaluated: usize,
    pub hit_rate_1h: f64,
    pub hit_rate_6h: f64,
    pub average_score_1h: f64,
    pub average_score_6h: f64,
    pub rows: Vec<AlphaBacktestRowResponse>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramWebhookUpdate {
    pub message: Option<TelegramMessage>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramMessage {
    pub chat: TelegramChat,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramChat {
    pub id: i64,
}

fn default_limit() -> i64 {
    20
}

fn default_min_amount() -> f64 {
    0.5
}

fn default_hours() -> i64 {
    24
}

fn default_backtest_limit() -> i64 {
    120
}

pub(super) fn risk_category(score: i16) -> &'static str {
    if score <= 30 {
        "low"
    } else if score <= 60 {
        "medium"
    } else {
        "high"
    }
}

pub(super) fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

pub(super) fn evaluate_alpha_outcome(
    baseline_volume_1h: f64,
    future_volume: f64,
    future_buys: i64,
    future_sells: i64,
) -> (f64, &'static str, bool) {
    let baseline = baseline_volume_1h.max(0.05);
    let volume_ratio = (future_volume / baseline).clamp(0.0, 3.0);
    let volume_component = (volume_ratio / 3.0) * 55.0;

    let total_flow = future_buys + future_sells;
    let buy_share = if total_flow > 0 {
        future_buys as f64 / total_flow as f64
    } else {
        0.5
    };
    let flow_component = buy_share * 45.0;

    let mut score = volume_component + flow_component;
    if total_flow < 3 {
        score *= 0.75;
    }

    let outcome = if score >= 65.0 {
        "outperform"
    } else if score >= 45.0 {
        "neutral"
    } else {
        "underperform"
    };

    (round2(score), outcome, outcome == "outperform")
}
