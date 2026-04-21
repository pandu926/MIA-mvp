use serde::{Deserialize, Serialize};

use crate::ai::gateway::ChatMessage;

// ─── Input data structure ─────────────────────────────────────────────────────

/// All structured data used to build AI prompts for a token.
/// Every field comes from the database — no inference, no speculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativePromptData {
    pub token_address: String,
    pub token_name: Option<String>,
    pub token_symbol: Option<String>,

    pub deployer_address: String,
    pub deployer_trust_grade: String, // "A" | "B" | "C" | "D" | "F"
    pub deployer_rug_count: i64,
    pub deployer_graduated_count: i64,

    pub holder_count: i32,
    pub buy_count: i32,
    pub sell_count: i32,
    pub volume_bnb: f64,

    pub composite_risk_score: u8,
    pub risk_category: String, // "low" | "medium" | "high"

    /// Top-10 wallet concentration as percentage (0.0–100.0)
    pub top_holder_concentration_pct: Option<f64>,

    /// Hours since token was deployed
    pub hours_since_deploy: f64,

    pub honeypot_detected: bool,
    pub is_rug: bool,
    pub graduated: bool,
}

// ─── Confidence ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl Confidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Confidence::High => "high",
            Confidence::Medium => "medium",
            Confidence::Low => "low",
        }
    }
}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Count how many of the optional/derived fields are populated.
fn populated_field_count(data: &NarrativePromptData) -> usize {
    let mut count = 0;
    if data.token_name.is_some() {
        count += 1;
    }
    if data.token_symbol.is_some() {
        count += 1;
    }
    if data.top_holder_concentration_pct.is_some() {
        count += 1;
    }
    if data.holder_count > 0 {
        count += 1;
    }
    if data.buy_count > 0 {
        count += 1;
    }
    if data.volume_bnb > 0.0 {
        count += 1;
    }
    if data.deployer_rug_count > 0 || data.deployer_graduated_count > 0 {
        count += 1;
    }
    count
}

/// Determine AI confidence level based on data completeness.
///
/// Rules:
/// - Low:    fewer than 3 optional fields populated
/// - Medium: 3–4 fields, or deployer history unknown (grade B with 0 history)
/// - High:   5+ fields populated and deployer history is known
pub fn determine_confidence(data: &NarrativePromptData) -> Confidence {
    let count = populated_field_count(data);

    if count < 3 {
        return Confidence::Low;
    }

    // Medium if deployer is brand-new (B grade, no rugs, no grads = unknown history)
    let deployer_unknown = data.deployer_trust_grade == "B"
        && data.deployer_rug_count == 0
        && data.deployer_graduated_count == 0;

    if count < 5 || deployer_unknown {
        return Confidence::Medium;
    }

    Confidence::High
}

/// Build the narrative prompt messages for GPT-4o.
///
/// System prompt strictly constrains hallucination by requiring the model
/// to use ONLY the provided structured data.
pub fn build_narrative_prompt(data: &NarrativePromptData) -> Vec<ChatMessage> {
    let system = "You are MIA, the Memecoin Intelligence Aggregator for Four.Meme (BNB Chain). \
        Generate a 2-3 sentence plain-language narrative summary of the given token. \
        RULES: ONLY use facts present in the structured data below. \
        Do NOT infer, speculate, or extrapolate beyond what the data shows. \
        Do NOT reference price, future potential, or investment advice. \
        Write in present tense, objective tone.";

    let data_json = serde_json::to_string_pretty(data).unwrap_or_else(|_| format!("{:?}", data));

    let user = format!(
        "Generate a narrative summary for this token.\n\nToken data:\n```json\n{}\n```",
        data_json
    );

    vec![ChatMessage::system(system), ChatMessage::user(user)]
}

/// Build the risk interpretation prompt for Claude.
///
/// Focused on explaining what the composite risk score means to a buyer.
pub fn build_risk_interpretation_prompt(data: &NarrativePromptData) -> Vec<ChatMessage> {
    let system = "You are MIA's risk analyst for Four.Meme (BNB Chain). \
        Interpret the risk signals for a token in 1-2 sentences for a potential buyer. \
        RULES: ONLY use facts present in the structured data below. \
        Be direct and specific about which signals are concerning or reassuring. \
        Do NOT give investment advice. Do NOT speculate.";

    let data_json = serde_json::to_string_pretty(data).unwrap_or_else(|_| format!("{:?}", data));

    let user = format!(
        "Interpret the risk score of {}/100 ({}) for this token.\n\nToken data:\n```json\n{}\n```",
        data.composite_risk_score, data.risk_category, data_json
    );

    vec![ChatMessage::system(system), ChatMessage::user(user)]
}

// ─────────────────────────────────────────────────────────────────────────────
// TDD Tests — all pure functions, no external deps
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn full_data() -> NarrativePromptData {
        NarrativePromptData {
            token_address: "0xabc123".to_string(),
            token_name: Some("PepeCoin".to_string()),
            token_symbol: Some("PEPE".to_string()),
            deployer_address: "0xdeployer".to_string(),
            deployer_trust_grade: "A".to_string(),
            deployer_rug_count: 0,
            deployer_graduated_count: 3,
            holder_count: 120,
            buy_count: 85,
            sell_count: 15,
            volume_bnb: 12.5,
            composite_risk_score: 22,
            risk_category: "low".to_string(),
            top_holder_concentration_pct: Some(35.0),
            hours_since_deploy: 4.5,
            honeypot_detected: false,
            is_rug: false,
            graduated: false,
        }
    }

    fn sparse_data() -> NarrativePromptData {
        NarrativePromptData {
            token_address: "0xnew".to_string(),
            token_name: None,
            token_symbol: None,
            deployer_address: "0xfresh".to_string(),
            deployer_trust_grade: "B".to_string(),
            deployer_rug_count: 0,
            deployer_graduated_count: 0,
            holder_count: 0,
            buy_count: 0,
            sell_count: 0,
            volume_bnb: 0.0,
            composite_risk_score: 50,
            risk_category: "medium".to_string(),
            top_holder_concentration_pct: None,
            hours_since_deploy: 0.1,
            honeypot_detected: false,
            is_rug: false,
            graduated: false,
        }
    }

    // ── determine_confidence ─────────────────────────────────────────────────

    // RED → GREEN: all data present → High confidence
    #[test]
    fn full_data_yields_high_confidence() {
        let confidence = determine_confidence(&full_data());
        assert_eq!(confidence, Confidence::High);
    }

    // RED → GREEN: no optional fields → Low confidence
    #[test]
    fn sparse_data_yields_low_confidence() {
        let confidence = determine_confidence(&sparse_data());
        assert_eq!(confidence, Confidence::Low);
    }

    // RED → GREEN: partial data with unknown deployer → Medium
    #[test]
    fn partial_data_unknown_deployer_is_medium() {
        let mut data = full_data();
        data.deployer_trust_grade = "B".to_string();
        data.deployer_rug_count = 0;
        data.deployer_graduated_count = 0;
        let confidence = determine_confidence(&data);
        assert_eq!(confidence, Confidence::Medium);
    }

    // RED → GREEN: fewer than 3 populated fields → Low regardless of deployer
    #[test]
    fn two_fields_is_low_confidence() {
        let mut data = sparse_data();
        // populate just 2 optional fields
        data.token_name = Some("X".to_string());
        data.token_symbol = Some("X".to_string());
        let confidence = determine_confidence(&data);
        assert_eq!(confidence, Confidence::Low);
    }

    // RED → GREEN: confidence as_str
    #[test]
    fn confidence_as_str_values() {
        assert_eq!(Confidence::High.as_str(), "high");
        assert_eq!(Confidence::Medium.as_str(), "medium");
        assert_eq!(Confidence::Low.as_str(), "low");
    }

    // ── build_narrative_prompt ───────────────────────────────────────────────

    // RED → GREEN: returns exactly 2 messages (system + user)
    #[test]
    fn narrative_prompt_has_two_messages() {
        let msgs = build_narrative_prompt(&full_data());
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "system");
        assert_eq!(msgs[1].role, "user");
    }

    // RED → GREEN: system prompt contains hallucination constraint
    #[test]
    fn narrative_system_prompt_contains_only_use_facts() {
        let msgs = build_narrative_prompt(&full_data());
        assert!(
            msgs[0].content.contains("ONLY use facts"),
            "System prompt must contain 'ONLY use facts'"
        );
    }

    // RED → GREEN: user message contains token address
    #[test]
    fn narrative_user_message_contains_token_address() {
        let msgs = build_narrative_prompt(&full_data());
        assert!(
            msgs[1].content.contains("0xabc123"),
            "User message must include token address"
        );
    }

    // RED → GREEN: user message contains risk score
    #[test]
    fn narrative_user_message_contains_risk_score() {
        let msgs = build_narrative_prompt(&full_data());
        assert!(
            msgs[1].content.contains("22"),
            "User message must include composite_risk_score"
        );
    }

    // RED → GREEN: handles None fields (no panic on sparse data)
    #[test]
    fn narrative_prompt_handles_sparse_data_without_panic() {
        let msgs = build_narrative_prompt(&sparse_data());
        assert_eq!(msgs.len(), 2);
        assert!(!msgs[1].content.is_empty());
    }

    // ── build_risk_interpretation_prompt ────────────────────────────────────

    // RED → GREEN: returns exactly 2 messages
    #[test]
    fn risk_prompt_has_two_messages() {
        let msgs = build_risk_interpretation_prompt(&full_data());
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "system");
        assert_eq!(msgs[1].role, "user");
    }

    // RED → GREEN: system prompt mentions risk
    #[test]
    fn risk_system_prompt_mentions_risk_signals() {
        let msgs = build_risk_interpretation_prompt(&full_data());
        assert!(
            msgs[0].content.contains("risk"),
            "Risk system prompt should mention 'risk'"
        );
    }

    // RED → GREEN: user message includes score and category
    #[test]
    fn risk_user_message_includes_score_and_category() {
        let msgs = build_risk_interpretation_prompt(&full_data());
        assert!(
            msgs[1].content.contains("22"),
            "User message should include composite_risk_score"
        );
        assert!(
            msgs[1].content.contains("low"),
            "User message should include risk_category"
        );
    }

    // RED → GREEN: no investment advice in system prompt
    #[test]
    fn risk_system_prompt_prohibits_investment_advice() {
        let msgs = build_risk_interpretation_prompt(&full_data());
        assert!(
            msgs[0].content.contains("Do NOT give investment advice"),
            "System prompt must prohibit investment advice"
        );
    }

    // RED → GREEN: honeypot flag visible in user message
    #[test]
    fn narrative_includes_honeypot_flag_in_user_message() {
        let mut data = full_data();
        data.honeypot_detected = true;
        let msgs = build_narrative_prompt(&data);
        assert!(
            msgs[1].content.contains("honeypot_detected"),
            "honeypot_detected field must appear in user message"
        );
    }
}
