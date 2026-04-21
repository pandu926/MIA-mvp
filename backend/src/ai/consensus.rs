use serde::{Deserialize, Serialize};

// ─── Types ────────────────────────────────────────────────────────────────────

/// Whether two LLM models agreed on their interpretation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConsensusStatus {
    /// Both models produced compatible sentiment
    Agreed,
    /// Models produced conflicting sentiment — flag as "Uncertain"
    Diverged,
    /// Only one model response was available
    SingleModel,
}

impl ConsensusStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConsensusStatus::Agreed => "agreed",
            ConsensusStatus::Diverged => "diverged",
            ConsensusStatus::SingleModel => "single_model",
        }
    }
}

/// Simple polarity bucket derived from keyword analysis.
#[derive(Debug, Clone, PartialEq)]
pub enum Sentiment {
    Positive,
    Neutral,
    Negative,
}

/// The final consensus result produced by comparing two LLM responses.
#[derive(Debug, Clone)]
pub struct ConsensusResult {
    pub status: ConsensusStatus,
    /// The final narrative text (prefixed with "[Uncertain] " on divergence).
    pub final_narrative: String,
    pub final_risk_interpretation: Option<String>,
}

// ─── Keyword lists ────────────────────────────────────────────────────────────

/// Words strongly associated with a positive/safe assessment.
const POSITIVE_KEYWORDS: &[&str] = &[
    "safe",
    "organic",
    "healthy",
    "trusted",
    "graduated",
    "strong",
    "legitimate",
    "authentic",
    "consistent",
    "low risk",
];

/// Words strongly associated with a negative/risky assessment.
const NEGATIVE_KEYWORDS: &[&str] = &[
    "suspicious",
    "risk",
    "rug",
    "honeypot",
    "caution",
    "warning",
    "dangerous",
    "coordinated",
    "concentrated",
    "concerning",
    "uncertain",
    "volatile",
    "sell pressure",
    "dump",
];

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Extract a coarse sentiment from LLM output text via keyword matching.
///
/// Returns `Neutral` when neither positive nor negative keywords dominate
/// (i.e., the counts are tied or the text is ambiguous). This keeps
/// the divergence rate low — only clear conflicts trigger "Uncertain".
pub fn extract_sentiment(text: &str) -> Sentiment {
    let lower = text.to_lowercase();

    let pos = POSITIVE_KEYWORDS
        .iter()
        .filter(|&&kw| lower.contains(kw))
        .count();
    let neg = NEGATIVE_KEYWORDS
        .iter()
        .filter(|&&kw| lower.contains(kw))
        .count();

    if pos > neg + 1 {
        Sentiment::Positive
    } else if neg > pos + 1 {
        Sentiment::Negative
    } else {
        Sentiment::Neutral
    }
}

/// Compare two LLM responses and determine whether they agree.
///
/// # Algorithm
/// 1. If `risk_response` is empty → `SingleModel` (no second opinion).
/// 2. Extract sentiment from each response.
/// 3. If both are `Neutral` → `Agreed` (nothing to conflict on).
/// 4. If one is strongly Positive and the other strongly Negative → `Diverged`.
/// 5. Otherwise → `Agreed`.
///
/// On `Diverged`, the final narrative is prefixed with "[Uncertain] " so the
/// frontend can show the amber badge.
pub fn check_consensus(narrative_response: &str, risk_response: &str) -> ConsensusResult {
    if risk_response.trim().is_empty() {
        return ConsensusResult {
            status: ConsensusStatus::SingleModel,
            final_narrative: narrative_response.to_string(),
            final_risk_interpretation: None,
        };
    }

    let narrative_sentiment = extract_sentiment(narrative_response);
    let risk_sentiment = extract_sentiment(risk_response);

    let diverged = matches!(
        (&narrative_sentiment, &risk_sentiment),
        (Sentiment::Positive, Sentiment::Negative) | (Sentiment::Negative, Sentiment::Positive)
    );

    if diverged {
        ConsensusResult {
            status: ConsensusStatus::Diverged,
            final_narrative: format!("[Uncertain] {}", narrative_response),
            final_risk_interpretation: Some(risk_response.to_string()),
        }
    } else {
        ConsensusResult {
            status: ConsensusStatus::Agreed,
            final_narrative: narrative_response.to_string(),
            final_risk_interpretation: Some(risk_response.to_string()),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TDD Tests — all pure functions, no external deps
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    // ── extract_sentiment ────────────────────────────────────────────────────

    // RED → GREEN: clearly positive text → Positive
    #[test]
    fn positive_keywords_yield_positive_sentiment() {
        let text =
            "This token shows organic growth with a safe, trusted deployer and healthy liquidity.";
        assert_eq!(extract_sentiment(text), Sentiment::Positive);
    }

    // RED → GREEN: clearly negative text → Negative
    #[test]
    fn negative_keywords_yield_negative_sentiment() {
        let text =
            "Suspicious coordinated buys. Honeypot contract detected. High rug risk, caution.";
        assert_eq!(extract_sentiment(text), Sentiment::Negative);
    }

    // RED → GREEN: neutral / balanced text → Neutral
    #[test]
    fn neutral_text_yields_neutral_sentiment() {
        let text = "Token was deployed 2 hours ago with 15 unique buyers and 3 BNB in volume.";
        assert_eq!(extract_sentiment(text), Sentiment::Neutral);
    }

    // RED → GREEN: empty text → Neutral (no keywords)
    #[test]
    fn empty_text_yields_neutral_sentiment() {
        assert_eq!(extract_sentiment(""), Sentiment::Neutral);
    }

    // RED → GREEN: single positive keyword → Neutral (not dominant enough, need >1 gap)
    #[test]
    fn single_positive_word_is_neutral() {
        // pos=1, neg=0 → pos > neg+1 is 1>1 → false → Neutral
        assert_eq!(
            extract_sentiment("This token looks organic."),
            Sentiment::Neutral
        );
    }

    // RED → GREEN: two positive vs zero negative → Positive
    #[test]
    fn two_positive_zero_negative_is_positive() {
        let text = "organic growth with healthy liquidity";
        assert_eq!(extract_sentiment(text), Sentiment::Positive);
    }

    // RED → GREEN: case-insensitive matching
    #[test]
    fn sentiment_is_case_insensitive() {
        let text = "HONEYPOT detected. HIGH RUG RISK. SUSPICIOUS activity.";
        assert_eq!(extract_sentiment(text), Sentiment::Negative);
    }

    // ── check_consensus ──────────────────────────────────────────────────────

    // RED → GREEN: empty risk_response → SingleModel
    #[test]
    fn empty_risk_response_is_single_model() {
        let result = check_consensus("Some narrative.", "");
        assert_eq!(result.status, ConsensusStatus::SingleModel);
        assert!(result.final_risk_interpretation.is_none());
    }

    // RED → GREEN: whitespace-only risk_response → SingleModel
    #[test]
    fn whitespace_risk_response_is_single_model() {
        let result = check_consensus("Some narrative.", "   \n  ");
        assert_eq!(result.status, ConsensusStatus::SingleModel);
    }

    // RED → GREEN: both positive → Agreed
    #[test]
    fn both_positive_responses_agree() {
        let narrative = "organic growth and healthy liquidity — safe deployer";
        let risk = "trusted deployer with consistent volume and low risk";
        let result = check_consensus(narrative, risk);
        assert_eq!(result.status, ConsensusStatus::Agreed);
        assert_eq!(result.final_narrative, narrative);
    }

    // RED → GREEN: both negative → Agreed (agreement on risk)
    #[test]
    fn both_negative_responses_agree() {
        let narrative = "suspicious coordinated buys, honeypot detected with high rug risk";
        let risk = "dangerous concentrated wallets, warning signs and dump pressure";
        let result = check_consensus(narrative, risk);
        assert_eq!(result.status, ConsensusStatus::Agreed);
    }

    // RED → GREEN: positive narrative vs negative risk → Diverged
    #[test]
    fn positive_narrative_negative_risk_diverges() {
        let narrative = "organic growth with healthy liquidity — safe trusted deployer";
        let risk = "honeypot detected, suspicious activity, rug risk warning concerning";
        let result = check_consensus(narrative, risk);
        assert_eq!(result.status, ConsensusStatus::Diverged);
    }

    // RED → GREEN: negative narrative vs positive risk → Diverged
    #[test]
    fn negative_narrative_positive_risk_diverges() {
        let narrative = "honeypot contract detected, suspicious coordinated dump pressure";
        let risk = "organic healthy safe trusted deployer with consistent growth";
        let result = check_consensus(narrative, risk);
        assert_eq!(result.status, ConsensusStatus::Diverged);
    }

    // RED → GREEN: diverged result prefixes narrative with [Uncertain]
    #[test]
    fn diverged_result_prefixes_narrative_with_uncertain() {
        let narrative = "organic safe healthy trusted liquidity";
        let risk = "honeypot rug suspicious caution warning dangerous";
        let result = check_consensus(narrative, risk);
        assert!(
            result.final_narrative.starts_with("[Uncertain] "),
            "Diverged narrative must start with [Uncertain]"
        );
    }

    // RED → GREEN: neutral vs neutral → Agreed
    #[test]
    fn both_neutral_responses_agree() {
        let narrative = "Token deployed 1 hour ago with 10 buyers.";
        let risk = "Composite score is 45 out of 100.";
        let result = check_consensus(narrative, risk);
        assert_eq!(result.status, ConsensusStatus::Agreed);
    }

    // RED → GREEN: agreed result does NOT prefix narrative
    #[test]
    fn agreed_result_preserves_original_narrative() {
        let narrative = "organic safe trusted healthy liquidity";
        let risk = "safe trusted organic consistent low risk";
        let result = check_consensus(narrative, risk);
        assert_eq!(result.final_narrative, narrative);
        assert!(!result.final_narrative.contains("[Uncertain]"));
    }

    // ── ConsensusStatus::as_str ──────────────────────────────────────────────

    #[test]
    fn consensus_status_as_str_values() {
        assert_eq!(ConsensusStatus::Agreed.as_str(), "agreed");
        assert_eq!(ConsensusStatus::Diverged.as_str(), "diverged");
        assert_eq!(ConsensusStatus::SingleModel.as_str(), "single_model");
    }
}
