use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Direction of a token trade.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TxType {
    Buy,
    Sell,
}

impl TxType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TxType::Buy => "buy",
            TxType::Sell => "sell",
        }
    }
}

/// A parsed buy or sell transaction for a specific token.
#[derive(Debug, Clone)]
pub struct ParsedTransaction {
    pub wallet_address: String,
    pub tx_type: TxType,
    pub amount_bnb: f64,
    pub tx_hash: String,
}

/// Accumulated metrics for a token, built by replaying its transactions.
/// Immutable pattern: `accumulate` returns a new value, never mutates self.
#[derive(Debug, Default, Clone)]
pub struct TokenMetrics {
    pub holder_count: u64,
    pub buy_count: u64,
    pub sell_count: u64,
    pub volume_bnb: f64,
    // Internal set for unique wallet tracking — not serialized
    unique_wallets: HashSet<String>,
}

impl TokenMetrics {
    pub fn new() -> Self {
        Self::default()
    }
}

// ─── Phase 2 heuristic: known sell function selectors on PancakeSwap/router ───
// swapExactTokensForETH     = 0x18cbafe5
// swapExactTokensForETHFee  = 0x791ac947
const SELL_SELECTORS: &[[u8; 4]] = &[[0x18, 0xcb, 0xaf, 0xe5], [0x79, 0x1a, 0xc9, 0x47]];

/// Classify a raw transaction as a token buy, sell, or unrelated.
///
/// Heuristic rules (Phase 2):
/// - tx to token contract with BNB value > 0 → Buy
/// - tx input data starts with a known sell selector → Sell
/// - otherwise → None (ignore)
pub fn classify_transaction(
    value_bnb: f64,
    input_data: &[u8],
    to_address: Option<&str>,
    token_address: &str,
) -> Option<TxType> {
    let to = to_address?;

    if to.eq_ignore_ascii_case(token_address) && value_bnb > 0.0 {
        return Some(TxType::Buy);
    }

    if input_data.len() >= 4 {
        let selector: [u8; 4] = [input_data[0], input_data[1], input_data[2], input_data[3]];
        if SELL_SELECTORS.contains(&selector) {
            return Some(TxType::Sell);
        }
    }

    None
}

/// Accumulate a transaction into token metrics, returning new metrics.
/// Immutable: the original `metrics` is unchanged.
pub fn accumulate_metrics(metrics: &TokenMetrics, tx: &ParsedTransaction) -> TokenMetrics {
    let mut updated = metrics.clone();

    match tx.tx_type {
        TxType::Buy => updated.buy_count += 1,
        TxType::Sell => updated.sell_count += 1,
    }

    updated.volume_bnb += tx.amount_bnb;
    updated.unique_wallets.insert(tx.wallet_address.clone());
    updated.holder_count = updated.unique_wallets.len() as u64;

    updated
}

// ─────────────────────────────────────────────────────────────────────────────
// TDD: Tests written before implementation above was finalized (RED → GREEN)
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "0xTokenAddress0000000000000000000000000001";
    const WALLET_A: &str = "0xWalletA000000000000000000000000000000A";
    const WALLET_B: &str = "0xWalletB000000000000000000000000000000B";

    fn make_tx(wallet: &str, tx_type: TxType) -> ParsedTransaction {
        ParsedTransaction {
            wallet_address: wallet.to_string(),
            tx_type,
            amount_bnb: 1.0,
            tx_hash: format!("0xhash_{}", wallet),
        }
    }

    // ── classify_transaction ─────────────────────────────────────────────────

    // RED → GREEN: tx with BNB to token address = Buy
    #[test]
    fn buy_detected_when_bnb_sent_to_token() {
        let result = classify_transaction(0.5, &[], Some(TOKEN), TOKEN);
        assert_eq!(result, Some(TxType::Buy));
    }

    // RED → GREEN: tx to token with zero BNB = not classified
    #[test]
    fn no_classification_when_zero_bnb_to_token() {
        let result = classify_transaction(0.0, &[], Some(TOKEN), TOKEN);
        assert!(result.is_none());
    }

    // RED → GREEN: tx with sell selector = Sell
    #[test]
    fn sell_detected_by_function_selector() {
        let selector = [0x18u8, 0xcb, 0xaf, 0xe5]; // swapExactTokensForETH
        let result = classify_transaction(0.0, &selector, Some("0xRouter"), TOKEN);
        assert_eq!(result, Some(TxType::Sell));
    }

    // RED → GREEN: second sell selector also works
    #[test]
    fn sell_detected_by_fee_selector() {
        let selector = [0x79u8, 0x1a, 0xc9, 0x47]; // swapExactTokensForETHFee
        let result = classify_transaction(0.0, &selector, Some("0xRouter"), TOKEN);
        assert_eq!(result, Some(TxType::Sell));
    }

    // RED → GREEN: no `to` address = None (contract creation)
    #[test]
    fn no_classification_without_recipient() {
        let result = classify_transaction(1.0, &[], None, TOKEN);
        assert!(result.is_none());
    }

    // RED → GREEN: unrelated address, no selector = None
    #[test]
    fn no_classification_for_unrelated_tx() {
        let result = classify_transaction(1.0, &[0x01, 0x02, 0x03, 0x04], Some("0xother"), TOKEN);
        assert!(result.is_none());
    }

    // RED → GREEN: token address match is case-insensitive
    #[test]
    fn buy_detection_is_case_insensitive() {
        let lower = TOKEN.to_lowercase();
        let result = classify_transaction(1.0, &[], Some(&lower), TOKEN);
        assert_eq!(result, Some(TxType::Buy));
    }

    // ── accumulate_metrics ───────────────────────────────────────────────────

    // RED → GREEN: buy increments buy_count and holder_count
    #[test]
    fn buy_increments_buy_count_and_holder_count() {
        let metrics = TokenMetrics::new();
        let tx = make_tx(WALLET_A, TxType::Buy);
        let updated = accumulate_metrics(&metrics, &tx);

        assert_eq!(updated.buy_count, 1);
        assert_eq!(updated.sell_count, 0);
        assert_eq!(updated.holder_count, 1);
        assert_eq!(updated.volume_bnb, 1.0);
    }

    // RED → GREEN: sell increments sell_count
    #[test]
    fn sell_increments_sell_count() {
        let metrics = TokenMetrics::new();
        let tx = make_tx(WALLET_A, TxType::Sell);
        let updated = accumulate_metrics(&metrics, &tx);

        assert_eq!(updated.sell_count, 1);
        assert_eq!(updated.buy_count, 0);
    }

    // RED → GREEN: same wallet buying twice = holder_count stays 1 (unique)
    #[test]
    fn same_wallet_does_not_double_count_holders() {
        let metrics = TokenMetrics::new();
        let tx1 = make_tx(WALLET_A, TxType::Buy);
        let tx2 = make_tx(WALLET_A, TxType::Buy);

        let updated = accumulate_metrics(&accumulate_metrics(&metrics, &tx1), &tx2);

        assert_eq!(updated.holder_count, 1);
        assert_eq!(updated.buy_count, 2);
    }

    // RED → GREEN: two different wallets = holder_count is 2
    #[test]
    fn two_wallets_gives_holder_count_of_two() {
        let metrics = TokenMetrics::new();
        let tx_a = make_tx(WALLET_A, TxType::Buy);
        let tx_b = make_tx(WALLET_B, TxType::Buy);

        let updated = accumulate_metrics(&accumulate_metrics(&metrics, &tx_a), &tx_b);

        assert_eq!(updated.holder_count, 2);
    }

    // RED → GREEN: volumes accumulate correctly
    #[test]
    fn volume_accumulates_across_transactions() {
        let mut metrics = TokenMetrics::new();
        for i in 1..=5 {
            let mut tx = make_tx(WALLET_A, TxType::Buy);
            tx.amount_bnb = i as f64;
            tx.tx_hash = format!("0xhash_{}", i);
            tx.wallet_address = format!("0xwallet_{}", i);
            metrics = accumulate_metrics(&metrics, &tx);
        }

        assert!((metrics.volume_bnb - 15.0).abs() < 1e-9); // 1+2+3+4+5
    }

    // RED → GREEN: accumulate is immutable — original unchanged
    #[test]
    fn accumulate_does_not_mutate_original() {
        let original = TokenMetrics::new();
        let tx = make_tx(WALLET_A, TxType::Buy);
        let _updated = accumulate_metrics(&original, &tx);

        assert_eq!(original.buy_count, 0);
        assert_eq!(original.holder_count, 0);
    }

    // ── TxType helpers ───────────────────────────────────────────────────────

    #[test]
    fn tx_type_as_str_returns_correct_values() {
        assert_eq!(TxType::Buy.as_str(), "buy");
        assert_eq!(TxType::Sell.as_str(), "sell");
    }
}
