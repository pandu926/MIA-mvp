use serde::{Deserialize, Serialize};

/// Input signals for composite risk score computation.
/// All values are in [0, 100] where 100 = most risky.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskSignals {
    pub deployer_history: u8,     // 25%
    pub liquidity_lock: u8,       // 20%
    pub wallet_concentration: u8, // 20%
    pub buy_sell_velocity: u8,    // 15%
    pub contract_audit: u8,       // 10%
    pub social_authenticity: u8,  // 5%
    pub volume_consistency: u8,   // 5%
}

/// Signal weights — must sum to 1.0.
const WEIGHTS: [f64; 7] = [0.25, 0.20, 0.20, 0.15, 0.10, 0.05, 0.05];

/// Human-readable risk category derived from composite score.
#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskCategory {
    Low,    // 0–30  🟢
    Medium, // 31–60 🟡
    High,   // 61–100 🔴
}

#[cfg(test)]
impl RiskCategory {
    pub fn emoji(&self) -> &'static str {
        match self {
            RiskCategory::Low => "🟢",
            RiskCategory::Medium => "🟡",
            RiskCategory::High => "🔴",
        }
    }
}

/// Compute the weighted composite risk score from all 7 signals.
///
/// Returns a value in [0, 100]:
///   0–30  = Low risk (green)
///   31–60 = Medium risk (yellow)
///   61–100 = High risk (red)
pub fn compute_composite_score(signals: &RiskSignals) -> u8 {
    let scores = [
        signals.deployer_history as f64,
        signals.liquidity_lock as f64,
        signals.wallet_concentration as f64,
        signals.buy_sell_velocity as f64,
        signals.contract_audit as f64,
        signals.social_authenticity as f64,
        signals.volume_consistency as f64,
    ];

    let weighted: f64 = scores
        .iter()
        .zip(WEIGHTS.iter())
        .map(|(score, weight)| score * weight)
        .sum();

    weighted.clamp(0.0, 100.0) as u8
}

/// Categorize a score into Low / Medium / High.
#[cfg(test)]
pub fn categorize_score(score: u8) -> RiskCategory {
    match score {
        0..=30 => RiskCategory::Low,
        31..=60 => RiskCategory::Medium,
        _ => RiskCategory::High,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TDD Tests
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn all_zero() -> RiskSignals {
        RiskSignals {
            deployer_history: 0,
            liquidity_lock: 0,
            wallet_concentration: 0,
            buy_sell_velocity: 0,
            contract_audit: 0,
            social_authenticity: 0,
            volume_consistency: 0,
        }
    }

    fn all_hundred() -> RiskSignals {
        RiskSignals {
            deployer_history: 100,
            liquidity_lock: 100,
            wallet_concentration: 100,
            buy_sell_velocity: 100,
            contract_audit: 100,
            social_authenticity: 100,
            volume_consistency: 100,
        }
    }

    // RED → GREEN: all signals zero = composite score zero
    #[test]
    fn all_zero_signals_give_zero_composite() {
        assert_eq!(compute_composite_score(&all_zero()), 0);
    }

    // RED → GREEN: all signals 100 = composite score 100
    #[test]
    fn all_hundred_signals_give_hundred_composite() {
        assert_eq!(compute_composite_score(&all_hundred()), 100);
    }

    // RED → GREEN: composite is a weighted sum (verify with specific values)
    #[test]
    fn composite_is_correct_weighted_sum() {
        // Only deployer_history = 100 (weight 0.25) → expected = 25
        let signals = RiskSignals {
            deployer_history: 100,
            ..all_zero()
        };
        assert_eq!(compute_composite_score(&signals), 25);
    }

    // RED → GREEN: liquidity_lock weight = 20% → score 100 → composite 20
    #[test]
    fn liquidity_lock_contributes_twenty_percent() {
        let signals = RiskSignals {
            liquidity_lock: 100,
            ..all_zero()
        };
        assert_eq!(compute_composite_score(&signals), 20);
    }

    // RED → GREEN: contract_audit = 100 (honeypot) → composite 10
    #[test]
    fn contract_audit_contributes_ten_percent() {
        let signals = RiskSignals {
            contract_audit: 100,
            ..all_zero()
        };
        assert_eq!(compute_composite_score(&signals), 10);
    }

    // RED → GREEN: WEIGHTS sum to 1.0 (internal invariant check)
    #[test]
    fn weights_sum_to_one() {
        let sum: f64 = WEIGHTS.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-10,
            "Weights must sum to 1.0, got {}",
            sum
        );
    }

    // RED → GREEN: composite score never exceeds 100
    #[test]
    fn composite_never_exceeds_100() {
        assert!(compute_composite_score(&all_hundred()) <= 100);
    }

    // RED → GREEN: composite score never goes below 0
    #[test]
    fn composite_never_below_zero() {
        assert_eq!(compute_composite_score(&all_zero()), 0);
    }

    // ── categorize_score ─────────────────────────────────────────────────────

    // RED → GREEN: score 0 = Low
    #[test]
    fn score_zero_is_low_risk() {
        assert_eq!(categorize_score(0), RiskCategory::Low);
    }

    // RED → GREEN: score 30 = Low (boundary)
    #[test]
    fn score_thirty_is_low_risk() {
        assert_eq!(categorize_score(30), RiskCategory::Low);
    }

    // RED → GREEN: score 31 = Medium (boundary)
    #[test]
    fn score_thirty_one_is_medium_risk() {
        assert_eq!(categorize_score(31), RiskCategory::Medium);
    }

    // RED → GREEN: score 60 = Medium (upper boundary)
    #[test]
    fn score_sixty_is_medium_risk() {
        assert_eq!(categorize_score(60), RiskCategory::Medium);
    }

    // RED → GREEN: score 61 = High (boundary)
    #[test]
    fn score_sixty_one_is_high_risk() {
        assert_eq!(categorize_score(61), RiskCategory::High);
    }

    // RED → GREEN: score 100 = High
    #[test]
    fn score_hundred_is_high_risk() {
        assert_eq!(categorize_score(100), RiskCategory::High);
    }

    // ── emoji ────────────────────────────────────────────────────────────────

    #[test]
    fn risk_category_emojis_are_correct() {
        assert_eq!(RiskCategory::Low.emoji(), "🟢");
        assert_eq!(RiskCategory::Medium.emoji(), "🟡");
        assert_eq!(RiskCategory::High.emoji(), "🔴");
    }

    // ── realistic scenario ───────────────────────────────────────────────────

    // RED → GREEN: rug token (honeypot + concentrated + no liquidity lock) = High risk
    #[test]
    fn rug_profile_scores_high_risk() {
        let signals = RiskSignals {
            deployer_history: 100, // serial rugger
            liquidity_lock: 100,   // 0% locked
            wallet_concentration: 90,
            buy_sell_velocity: 70,
            contract_audit: 100, // honeypot
            social_authenticity: 80,
            volume_consistency: 90,
        };
        let score = compute_composite_score(&signals);
        assert!(score > 60, "Rug profile should be High risk, got {}", score);
        assert_eq!(categorize_score(score), RiskCategory::High);
    }

    // RED → GREEN: safe token = Low risk
    #[test]
    fn safe_token_scores_low_risk() {
        let signals = RiskSignals {
            deployer_history: 0, // trusted deployer
            liquidity_lock: 0,   // 100% locked
            wallet_concentration: 10,
            buy_sell_velocity: 10, // mostly buys
            contract_audit: 0,     // clean contract
            social_authenticity: 20,
            volume_consistency: 15,
        };
        let score = compute_composite_score(&signals);
        assert!(
            score <= 30,
            "Safe profile should be Low risk, got {}",
            score
        );
        assert_eq!(categorize_score(score), RiskCategory::Low);
    }
}
