use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const CREATE_TOKEN_SELECTOR: &str = "0x519ebb10";
const BUY_TOKEN_SELECTOR: &str = "0x87f27655";
const SELL_TOKEN_SELECTOR: &str = "0xf464e7db";
const BUY_TOKEN_SELECTOR_LEGACY: &str = "0x06e7b98f";
const SELL_TOKEN_SELECTOR_LEGACY: &str = "0xedf9e251";

/// Represents a parsed token deployment event from Four.Meme factory contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDeployedEvent {
    pub contract_address: String,
    pub deployer_address: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub block_number: u64,
    pub tx_hash: String,
    pub timestamp: DateTime<Utc>,
}

/// Raw transaction data from BNB Chain block.
#[derive(Debug)]
pub struct RawTransaction {
    pub hash: String,
    pub from: String,
    pub to: Option<String>,
    pub input: String,
    pub value_bnb: f64,
    pub block_number: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeType {
    Buy,
    Sell,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTrade {
    pub token_address: String,
    pub trade_type: TradeType,
}

fn normalize_hex_input(input: &str) -> Option<String> {
    let lower = input.to_ascii_lowercase();
    if lower.starts_with("0x") && lower.len() >= 10 {
        Some(lower)
    } else {
        None
    }
}

pub fn is_create_token_call(input: &str) -> bool {
    normalize_hex_input(input)
        .map(|s| s.starts_with(CREATE_TOKEN_SELECTOR))
        .unwrap_or(false)
}

/// Extract first ABI argument as address from calldata.
/// Assumes standard ABI-encoded call: 4-byte selector + 32-byte first arg.
pub fn extract_first_arg_address(input: &str) -> Option<String> {
    let normalized = normalize_hex_input(input)?;
    // 0x + 8 selector + 64 first word
    if normalized.len() < 2 + 8 + 64 {
        return None;
    }
    let word = &normalized[10..74];
    let addr = &word[24..64];
    Some(format!("0x{}", addr))
}

/// Parse manager buy/sell calls and return token + side.
pub fn parse_token_trade(tx: &RawTransaction, manager_address: &str) -> Option<ParsedTrade> {
    let to = tx.to.as_deref()?;
    if !to.eq_ignore_ascii_case(manager_address) {
        return None;
    }

    let normalized = normalize_hex_input(&tx.input)?;
    let token_address = extract_first_arg_address(&normalized)?;

    if normalized.starts_with(BUY_TOKEN_SELECTOR)
        || normalized.starts_with(BUY_TOKEN_SELECTOR_LEGACY)
    {
        return Some(ParsedTrade {
            token_address,
            trade_type: TradeType::Buy,
        });
    }
    if normalized.starts_with(SELL_TOKEN_SELECTOR)
        || normalized.starts_with(SELL_TOKEN_SELECTOR_LEGACY)
    {
        return Some(ParsedTrade {
            token_address,
            trade_type: TradeType::Sell,
        });
    }

    None
}

/// Attempt to parse a raw transaction as a Four.Meme token deployment.
///
/// TODO: Replace with proper ABI-based event decoding once Four.Meme contract
/// ABI is confirmed. Current implementation uses heuristic detection based on
/// transaction recipient and input data patterns.
///
/// Four.Meme token manager (live): 0x5c952063c7fc8610FFDB798152D69F0B9550762b
/// Known event signature: TokenCreated(address,address,string,string,uint256)
pub fn parse_token_deployment(
    tx: &RawTransaction,
    factory_address: &str,
) -> Option<TokenDeployedEvent> {
    // Check if transaction targets the Four.Meme factory contract
    let to = tx.to.as_deref()?;

    if !to.eq_ignore_ascii_case(factory_address) {
        return None;
    }

    // The token manager handles create/buy/sell. We only accept explicit
    // createToken(...) calls (selector 0x519ebb10) as token deployments.
    if !is_create_token_call(&tx.input) {
        return None;
    }

    tracing::debug!(
        tx_hash = %tx.hash,
        deployer = %tx.from,
        "Detected Four.Meme createToken deployment transaction"
    );

    Some(TokenDeployedEvent {
        // contract_address filled in by listener from receipt logs
        contract_address: format!("pending:{}", &tx.hash[..10]),
        deployer_address: tx.from.clone(),
        name: None,
        symbol: None,
        block_number: tx.block_number,
        tx_hash: tx.hash.clone(),
        timestamp: tx.timestamp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    const FACTORY: &str = "0x5c952063c7fc8610FFDB798152D69F0B9550762b";
    const CREATE_TOKEN_INPUT: &str =
        "0x519ebb1000000000000000000000000000000000000000000000000000000000";
    const BUY_TOKEN_INPUT: &str =
        "0x87f276550000000000000000000000000000000000000000000000000000000000000000";

    fn make_tx(to: Option<&str>, input: &str) -> RawTransaction {
        RawTransaction {
            hash: "0xabcdef1234567890abcdef1234567890abcdef12".to_string(),
            from: "0xdeployer0000000000000000000000000000000a".to_string(),
            to: to.map(str::to_string),
            input: input.to_string(),
            value_bnb: 0.0,
            block_number: 42_000_000,
            timestamp: Utc::now(),
        }
    }

    // RED → GREEN: deployment targeting factory is detected
    #[test]
    fn detects_factory_transaction() {
        let tx = make_tx(Some(FACTORY), CREATE_TOKEN_INPUT);
        let event = parse_token_deployment(&tx, FACTORY);
        assert!(event.is_some(), "should detect factory transaction");
    }

    // RED → GREEN: wrong recipient returns None
    #[test]
    fn ignores_non_factory_transaction() {
        let tx = make_tx(
            Some("0xsomeothercontract00000000000000000000000"),
            CREATE_TOKEN_INPUT,
        );
        let event = parse_token_deployment(&tx, FACTORY);
        assert!(event.is_none(), "should ignore unrelated transaction");
    }

    // RED → GREEN: contract-create (no `to`) returns None
    #[test]
    fn ignores_contract_creation_transaction() {
        let tx = make_tx(None, CREATE_TOKEN_INPUT);
        let event = parse_token_deployment(&tx, FACTORY);
        assert!(event.is_none(), "should ignore contract creation tx");
    }

    // RED → GREEN: factory address matching is case-insensitive
    #[test]
    fn matches_factory_address_case_insensitively() {
        let lowercase_factory = FACTORY.to_lowercase();
        let tx = make_tx(Some(&lowercase_factory), CREATE_TOKEN_INPUT);
        let event = parse_token_deployment(&tx, FACTORY);
        assert!(event.is_some(), "factory match should be case-insensitive");
    }

    #[test]
    fn ignores_non_create_methods() {
        let tx = make_tx(Some(FACTORY), BUY_TOKEN_INPUT);
        let event = parse_token_deployment(&tx, FACTORY);
        assert!(
            event.is_none(),
            "buy/sell calls should not be treated as deployments"
        );
    }

    #[test]
    fn extracts_first_arg_address() {
        let input = "0x87f276550000000000000000000000005df95e82f8bc148a0777faa3f62667d84cb844440000000000000000000000000000000000000000000000000000000000000001";
        let got = extract_first_arg_address(input).unwrap();
        assert_eq!(got, "0x5df95e82f8bc148a0777faa3f62667d84cb84444");
    }

    #[test]
    fn parses_buy_trade_from_manager() {
        let input = "0x87f276550000000000000000000000005df95e82f8bc148a0777faa3f62667d84cb844440000000000000000000000000000000000000000000000000000000000000001";
        let tx = make_tx(Some(FACTORY), input);
        let trade = parse_token_trade(&tx, FACTORY).expect("buy trade should parse");
        assert_eq!(trade.trade_type, TradeType::Buy);
        assert_eq!(
            trade.token_address,
            "0x5df95e82f8bc148a0777faa3f62667d84cb84444"
        );
    }

    #[test]
    fn parses_sell_trade_from_manager() {
        let input = "0xf464e7db0000000000000000000000005df95e82f8bc148a0777faa3f62667d84cb844440000000000000000000000000000000000000000000000000000000000000001";
        let tx = make_tx(Some(FACTORY), input);
        let trade = parse_token_trade(&tx, FACTORY).expect("sell trade should parse");
        assert_eq!(trade.trade_type, TradeType::Sell);
    }

    // RED → GREEN: event preserves deployer and block metadata
    #[test]
    fn event_contains_correct_metadata() {
        let tx = make_tx(Some(FACTORY), CREATE_TOKEN_INPUT);
        let event = parse_token_deployment(&tx, FACTORY).unwrap();

        assert_eq!(event.deployer_address, tx.from);
        assert_eq!(event.block_number, tx.block_number);
        assert_eq!(event.tx_hash, tx.hash);
    }

    // RED → GREEN: pending contract address includes tx hash prefix
    #[test]
    fn pending_contract_address_includes_tx_prefix() {
        let tx = make_tx(Some(FACTORY), CREATE_TOKEN_INPUT);
        let event = parse_token_deployment(&tx, FACTORY).unwrap();

        assert!(
            event.contract_address.starts_with("pending:"),
            "placeholder address should use pending: prefix"
        );
        assert!(
            event.contract_address.contains(&tx.hash[..10]),
            "placeholder should include tx hash prefix"
        );
    }
}
