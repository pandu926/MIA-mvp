use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{TimeZone, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const DEXSCREENER_SEARCH_URL: &str = "https://api.dexscreener.com/latest/dex/search";

#[derive(Debug, Clone, Serialize)]
pub struct DexScreenerContext {
    pub provider: String,
    pub summary: String,
    pub source_url: Option<String>,
    pub observed_at: Option<String>,
    pub fallback_note: Option<String>,
    pub pair_address: Option<String>,
    pub dex_id: Option<String>,
    pub pair_label: Option<String>,
    pub base_symbol: Option<String>,
    pub quote_symbol: Option<String>,
    pub price_usd: Option<String>,
    pub liquidity_usd: Option<f64>,
    pub fdv: Option<f64>,
    pub market_cap: Option<f64>,
    pub volume_usd: DexMetricWindow,
    pub price_change_pct: DexMetricWindow,
    pub txns: DexTxnWindowSummary,
    pub pair_created_at: Option<String>,
    pub age_label: Option<String>,
    pub market_structure_label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DexMetricWindow {
    pub m5: Option<f64>,
    pub h1: Option<f64>,
    pub h6: Option<f64>,
    pub h24: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DexTxnWindow {
    pub buys: Option<u64>,
    pub sells: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DexTxnWindowSummary {
    pub m5: DexTxnWindow,
    pub h1: DexTxnWindow,
    pub h6: DexTxnWindow,
    pub h24: DexTxnWindow,
}

#[derive(Debug, Deserialize)]
struct DexSearchEnvelope {
    pairs: Vec<DexPair>,
}

#[derive(Debug, Deserialize)]
struct DexPair {
    #[serde(rename = "chainId")]
    chain_id: String,
    #[serde(rename = "dexId")]
    dex_id: Option<String>,
    url: Option<String>,
    #[serde(rename = "pairCreatedAt")]
    pair_created_at: Option<i64>,
    #[serde(rename = "pairAddress")]
    pair_address: Option<String>,
    #[serde(rename = "baseToken")]
    base_token: DexTokenRef,
    #[serde(rename = "quoteToken")]
    quote_token: DexTokenRef,
    #[serde(rename = "priceUsd")]
    price_usd: Option<String>,
    #[serde(rename = "fdv")]
    fdv: Option<f64>,
    #[serde(rename = "marketCap")]
    market_cap: Option<f64>,
    liquidity: Option<DexLiquidity>,
    volume: Option<DexWindowStats>,
    txns: Option<DexTxnsEnvelope>,
    #[serde(rename = "priceChange")]
    price_change: Option<DexWindowStats>,
}

#[derive(Debug, Deserialize)]
struct DexTokenRef {
    address: String,
    symbol: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DexLiquidity {
    usd: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
struct DexWindowStats {
    h24: Option<f64>,
    h6: Option<f64>,
    h1: Option<f64>,
    m5: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct DexTxnsEnvelope {
    h24: Option<DexTxnCounts>,
    h6: Option<DexTxnCounts>,
    h1: Option<DexTxnCounts>,
    m5: Option<DexTxnCounts>,
}

#[derive(Debug, Deserialize)]
struct DexTxnCounts {
    buys: Option<u64>,
    sells: Option<u64>,
}

fn token_label(token: &DexTokenRef) -> String {
    token
        .symbol
        .clone()
        .or(token.name.clone())
        .unwrap_or_else(|| token.address.clone())
}

fn metric_window(stats: Option<&DexWindowStats>) -> DexMetricWindow {
    DexMetricWindow {
        m5: stats.and_then(|value| value.m5),
        h1: stats.and_then(|value| value.h1),
        h6: stats.and_then(|value| value.h6),
        h24: stats.and_then(|value| value.h24),
    }
}

fn txn_window(counts: Option<&DexTxnCounts>) -> DexTxnWindow {
    DexTxnWindow {
        buys: counts.and_then(|value| value.buys),
        sells: counts.and_then(|value| value.sells),
    }
}

fn txn_summary(txns: Option<&DexTxnsEnvelope>) -> DexTxnWindowSummary {
    DexTxnWindowSummary {
        m5: txn_window(txns.and_then(|value| value.m5.as_ref())),
        h1: txn_window(txns.and_then(|value| value.h1.as_ref())),
        h6: txn_window(txns.and_then(|value| value.h6.as_ref())),
        h24: txn_window(txns.and_then(|value| value.h24.as_ref())),
    }
}

fn format_usd_compact(value: f64) -> String {
    if value >= 1_000_000_000.0 {
        format!("${:.2}B", value / 1_000_000_000.0)
    } else if value >= 1_000_000.0 {
        format!("${:.2}M", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("${:.2}K", value / 1_000.0)
    } else {
        format!("${value:.2}")
    }
}

fn format_percent(value: f64) -> String {
    if value >= 0.0 {
        format!("+{value:.2}%")
    } else {
        format!("{value:.2}%")
    }
}

fn format_millis_timestamp(value: Option<i64>) -> Option<String> {
    value.and_then(|millis| {
        Utc.timestamp_millis_opt(millis)
            .single()
            .map(|dt| dt.to_rfc3339())
    })
}

fn age_label(value: Option<i64>) -> Option<String> {
    let created_at = value.and_then(|millis| Utc.timestamp_millis_opt(millis).single())?;
    let age = Utc::now().signed_duration_since(created_at);

    if age.num_days() > 0 {
        Some(format!("{}d old", age.num_days()))
    } else if age.num_hours() > 0 {
        Some(format!("{}h old", age.num_hours()))
    } else if age.num_minutes() > 0 {
        Some(format!("{}m old", age.num_minutes()))
    } else {
        Some("freshly created".to_string())
    }
}

fn market_structure_label(pair: &DexPair) -> String {
    let liquidity = pair
        .liquidity
        .as_ref()
        .and_then(|value| value.usd)
        .unwrap_or_default();
    let h24_volume = pair
        .volume
        .as_ref()
        .and_then(|value| value.h24)
        .unwrap_or_default();
    let h24_change = pair
        .price_change
        .as_ref()
        .and_then(|value| value.h24.or(value.h6).or(value.h1).or(value.m5))
        .unwrap_or_default();
    let h24_buys = pair
        .txns
        .as_ref()
        .and_then(|value| value.h24.as_ref())
        .and_then(|value| value.buys)
        .unwrap_or_default();
    let h24_sells = pair
        .txns
        .as_ref()
        .and_then(|value| value.h24.as_ref())
        .and_then(|value| value.sells)
        .unwrap_or_default();

    if liquidity >= 100_000.0 && h24_volume >= 100_000.0 && h24_buys > h24_sells {
        "liquid with active buy-side participation".to_string()
    } else if liquidity < 10_000.0 {
        "thin liquidity and fragile exit conditions".to_string()
    } else if h24_volume < 5_000.0 && h24_buys + h24_sells <= 5 {
        "low-turnover pair with weak trading interest".to_string()
    } else if h24_change <= -20.0 {
        "heavy drawdown with stressed momentum".to_string()
    } else if h24_change >= 20.0 {
        "high-volatility breakout conditions".to_string()
    } else {
        "tradable but mixed market structure".to_string()
    }
}

fn summarize_pair(pair: &DexPair) -> DexScreenerContext {
    let symbol = token_label(&pair.base_token);
    let quote_symbol = token_label(&pair.quote_token);
    let price = pair
        .price_usd
        .as_deref()
        .map(|value| format!("price ${value}"))
        .unwrap_or_else(|| "price unavailable".to_string());
    let liquidity = pair
        .liquidity
        .as_ref()
        .and_then(|value| value.usd)
        .map(format_usd_compact)
        .map(|value| format!("liquidity {value}"))
        .unwrap_or_else(|| "liquidity unavailable".to_string());
    let recent_volume = pair
        .volume
        .as_ref()
        .and_then(|value| value.h24.or(value.h6).or(value.h1).or(value.m5))
        .map(format_usd_compact)
        .map(|value| format!("recent volume {value}"))
        .unwrap_or_else(|| "volume unavailable".to_string());
    let price_change = pair
        .price_change
        .as_ref()
        .and_then(|value| value.h24.or(value.h6).or(value.h1).or(value.m5))
        .map(format_percent)
        .map(|value| format!("price change {value}"))
        .unwrap_or_else(|| "price change unavailable".to_string());
    let valuation = pair
        .market_cap
        .or(pair.fdv)
        .map(format_usd_compact)
        .map(|value| format!("valuation {value}"))
        .unwrap_or_else(|| "valuation unavailable".to_string());
    let dex_label = pair
        .dex_id
        .clone()
        .unwrap_or_else(|| "unknown dex".to_string());
    let structure = market_structure_label(pair);
    let created_at = format_millis_timestamp(pair.pair_created_at);
    let age = age_label(pair.pair_created_at);

    DexScreenerContext {
        provider: "dexscreener-search".to_string(),
        summary: format!(
            "{symbol}/{quote_symbol} on {dex_label}: {price}, {liquidity}, {recent_volume}, {price_change}, {valuation}, {structure}."
        ),
        source_url: pair.url.clone(),
        observed_at: Some(Utc::now().to_rfc3339()),
        fallback_note: None,
        pair_address: pair.pair_address.clone(),
        dex_id: pair.dex_id.clone(),
        pair_label: Some(format!("{symbol}/{quote_symbol}")),
        base_symbol: pair.base_token.symbol.clone().or(pair.base_token.name.clone()),
        quote_symbol: pair.quote_token.symbol.clone().or(pair.quote_token.name.clone()),
        price_usd: pair.price_usd.clone(),
        liquidity_usd: pair.liquidity.as_ref().and_then(|value| value.usd),
        fdv: pair.fdv,
        market_cap: pair.market_cap,
        volume_usd: metric_window(pair.volume.as_ref()),
        price_change_pct: metric_window(pair.price_change.as_ref()),
        txns: txn_summary(pair.txns.as_ref()),
        pair_created_at: created_at,
        age_label: age,
        market_structure_label: structure,
    }
}

fn pair_score(pair: &DexPair) -> f64 {
    let liquidity = pair
        .liquidity
        .as_ref()
        .and_then(|value| value.usd)
        .unwrap_or_default();
    let volume = pair
        .volume
        .as_ref()
        .and_then(|value| value.h24)
        .unwrap_or_default();
    let market_cap = pair.market_cap.or(pair.fdv).unwrap_or_default();
    let txns = pair
        .txns
        .as_ref()
        .and_then(|value| value.h24.as_ref())
        .map(|value| value.buys.unwrap_or_default() + value.sells.unwrap_or_default())
        .unwrap_or_default() as f64;

    liquidity * 2.0 + volume + market_cap * 0.05 + txns * 100.0
}

pub async fn fetch_pair_context(token_address: &str) -> Result<DexScreenerContext> {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .context("failed to build DexScreener client")?;

    let response = client
        .get(DEXSCREENER_SEARCH_URL)
        .query(&[("q", token_address)])
        .send()
        .await
        .context("DexScreener search request failed")?
        .error_for_status()
        .context("DexScreener search returned non-success status")?;

    let body: DexSearchEnvelope = response
        .json()
        .await
        .context("DexScreener response was not valid JSON")?;

    let pair = body
        .pairs
        .into_iter()
        .filter(|item| item.chain_id.eq_ignore_ascii_case("bsc"))
        .filter(|item| {
            item.base_token.address.eq_ignore_ascii_case(token_address)
                || item.quote_token.address.eq_ignore_ascii_case(token_address)
        })
        .max_by(|left, right| pair_score(left).total_cmp(&pair_score(right)))
        .context("DexScreener returned no BSC pair for this token")?;

    Ok(summarize_pair(&pair))
}

#[cfg(test)]
mod tests {
    use super::{
        summarize_pair, DexLiquidity, DexPair, DexTokenRef, DexTxnCounts, DexTxnsEnvelope,
        DexWindowStats,
    };

    #[test]
    fn summarize_pair_includes_market_context_fields() {
        let pair = DexPair {
            chain_id: "bsc".to_string(),
            dex_id: Some("pancakeswap".to_string()),
            url: Some("https://dexscreener.com/bsc/pair".to_string()),
            pair_created_at: Some(1_713_523_200_000),
            pair_address: Some("0xpair".to_string()),
            base_token: DexTokenRef {
                address: "0xbase".to_string(),
                symbol: Some("TEST".to_string()),
                name: Some("Test".to_string()),
            },
            quote_token: DexTokenRef {
                address: "0xquote".to_string(),
                symbol: Some("WBNB".to_string()),
                name: Some("Wrapped BNB".to_string()),
            },
            price_usd: Some("0.0012".to_string()),
            fdv: Some(120000.0),
            market_cap: Some(110000.0),
            liquidity: Some(DexLiquidity { usd: Some(42000.0) }),
            volume: Some(DexWindowStats {
                h24: Some(98000.0),
                h6: Some(56000.0),
                h1: Some(12000.0),
                m5: Some(900.0),
            }),
            txns: Some(DexTxnsEnvelope {
                h24: Some(DexTxnCounts {
                    buys: Some(120),
                    sells: Some(87),
                }),
                h6: Some(DexTxnCounts {
                    buys: Some(44),
                    sells: Some(31),
                }),
                h1: Some(DexTxnCounts {
                    buys: Some(8),
                    sells: Some(5),
                }),
                m5: Some(DexTxnCounts {
                    buys: Some(1),
                    sells: Some(0),
                }),
            }),
            price_change: Some(DexWindowStats {
                h24: Some(22.4),
                h6: Some(11.2),
                h1: Some(4.4),
                m5: Some(0.9),
            }),
        };

        let context = summarize_pair(&pair);

        assert!(context.summary.contains("TEST/WBNB"));
        assert!(context.summary.contains("pancakeswap"));
        assert!(context.summary.contains("liquidity $42.00K"));
        assert!(context.summary.contains("price change +22.40%"));
        assert_eq!(
            context.source_url.as_deref(),
            Some("https://dexscreener.com/bsc/pair")
        );
        assert_eq!(context.pair_address.as_deref(), Some("0xpair"));
        assert_eq!(context.volume_usd.h24, Some(98000.0));
        assert_eq!(context.txns.h24.buys, Some(120));
        assert_eq!(context.price_change_pct.h6, Some(11.2));
        assert!(context.pair_created_at.is_some());
    }

    #[test]
    fn market_structure_flags_thin_liquidity() {
        let pair = DexPair {
            chain_id: "bsc".to_string(),
            dex_id: Some("fourmeme".to_string()),
            url: None,
            pair_created_at: Some(1_713_523_200_000),
            pair_address: Some("0xpair".to_string()),
            base_token: DexTokenRef {
                address: "0xbase".to_string(),
                symbol: Some("TEST".to_string()),
                name: None,
            },
            quote_token: DexTokenRef {
                address: "0xquote".to_string(),
                symbol: Some("WBNB".to_string()),
                name: None,
            },
            price_usd: Some("0.0002".to_string()),
            fdv: Some(5000.0),
            market_cap: Some(5000.0),
            liquidity: Some(DexLiquidity { usd: Some(1200.0) }),
            volume: Some(DexWindowStats {
                h24: Some(2000.0),
                h6: Some(600.0),
                h1: Some(100.0),
                m5: Some(0.0),
            }),
            txns: Some(DexTxnsEnvelope {
                h24: Some(DexTxnCounts {
                    buys: Some(2),
                    sells: Some(4),
                }),
                h6: None,
                h1: None,
                m5: None,
            }),
            price_change: Some(DexWindowStats {
                h24: Some(-8.0),
                h6: Some(-3.0),
                h1: Some(-1.0),
                m5: Some(0.0),
            }),
        };

        let context = summarize_pair(&pair);

        assert_eq!(
            context.market_structure_label,
            "thin liquidity and fragile exit conditions"
        );
    }
}
