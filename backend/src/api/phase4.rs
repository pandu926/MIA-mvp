mod alpha;
mod deployers;
mod telegram;
mod types;
mod whales;

pub use alpha::{get_alpha_backtest, get_alpha_history, get_latest_alpha};
pub use deployers::get_deployer_tokens;
pub use telegram::{get_telegram_config, telegram_webhook, update_telegram_config};
pub use whales::{list_whale_alerts, whale_network, whale_stream};

#[cfg(test)]
mod tests {
    use super::types::evaluate_alpha_outcome;

    #[test]
    fn alpha_outcome_outperform_when_volume_expands_and_buys_dominate() {
        let (score, outcome, hit) = evaluate_alpha_outcome(1.0, 2.8, 16, 4);
        assert!(score >= 65.0, "score={score}");
        assert_eq!(outcome, "outperform");
        assert!(hit);
    }

    #[test]
    fn alpha_outcome_underperform_when_flow_is_weak() {
        let (score, outcome, hit) = evaluate_alpha_outcome(2.0, 0.2, 1, 6);
        assert!(score < 45.0, "score={score}");
        assert_eq!(outcome, "underperform");
        assert!(!hit);
    }
}
