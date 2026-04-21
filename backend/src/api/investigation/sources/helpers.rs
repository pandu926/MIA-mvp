use reqwest::Url;
use serde::de::DeserializeOwned;

use crate::config::Config;

use super::super::types::{EtherscanEnvelope, InvestigationSource, TokenSnapshot};

pub(super) fn risk_category(score: i16) -> String {
    if score <= 30 {
        "low".to_string()
    } else if score <= 60 {
        "medium".to_string()
    } else {
        "high".to_string()
    }
}

pub(super) async fn etherscan_get<T: DeserializeOwned>(
    client: &reqwest::Client,
    config: &Config,
    api_key: &str,
    params: Vec<(&str, String)>,
) -> Result<T, anyhow::Error> {
    let mut url = Url::parse(&config.bscscan_api_url)?;
    {
        let mut query_pairs = url.query_pairs_mut();
        query_pairs.append_pair("chainid", &config.bscscan_chain_id.to_string());
        for (key, value) in params {
            query_pairs.append_pair(key, &value);
        }
        query_pairs.append_pair("apikey", api_key);
    }

    let response: EtherscanEnvelope<T> = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    if response.status != "1" {
        return Err(anyhow::anyhow!(
            "{} ({})",
            response.message,
            response.status
        ));
    }

    Ok(response.result)
}

pub(super) async fn moralis_get<T: DeserializeOwned>(
    client: &reqwest::Client,
    config: &Config,
    api_key: &str,
    path: &str,
    params: Vec<(&str, String)>,
) -> Result<T, anyhow::Error> {
    let trimmed_base = config.moralis_api_url.trim_end_matches('/');
    let trimmed_path = path.trim_start_matches('/');
    let mut url = Url::parse(&format!("{trimmed_base}/{trimmed_path}"))?;
    {
        let mut query_pairs = url.query_pairs_mut();
        for (key, value) in params {
            query_pairs.append_pair(key, &value);
        }
    }

    let response = client
        .get(url)
        .header("X-API-Key", api_key)
        .send()
        .await?
        .error_for_status()?
        .json::<T>()
        .await?;

    Ok(response)
}

pub(super) fn parse_boolish(raw: &str) -> Option<bool> {
    match raw.trim() {
        "1" | "true" | "True" => Some(true),
        "0" | "false" | "False" => Some(false),
        _ => None,
    }
}

pub(super) fn format_supply(raw: Option<&str>, decimals: Option<u32>) -> Option<String> {
    let raw = raw?.trim();
    if raw.is_empty() {
        return None;
    }
    let Some(decimals) = decimals else {
        return Some(raw.to_string());
    };
    if decimals == 0 {
        return Some(raw.to_string());
    }
    let digits = raw.trim_start_matches('0');
    if digits.is_empty() {
        return Some("0".to_string());
    }
    let decimals = decimals as usize;
    if digits.len() <= decimals {
        let padded = format!("{:0>width$}", digits, width = decimals);
        return Some(format!("0.{}", padded.trim_end_matches('0')));
    }
    let split = digits.len() - decimals;
    let whole = &digits[..split];
    let fraction = digits[split..].trim_end_matches('0');
    if fraction.is_empty() {
        Some(whole.to_string())
    } else {
        Some(format!("{}.{}", whole, fraction))
    }
}

pub(super) fn compute_holder_ownership_pct(
    total_supply_raw: Option<&str>,
    holder_quantity_raw: &str,
) -> Option<f64> {
    let total_supply = total_supply_raw?.trim().parse::<f64>().ok()?;
    if !total_supply.is_finite() || total_supply <= 0.0 {
        return None;
    }

    let holder_quantity = holder_quantity_raw.trim().parse::<f64>().ok()?;
    if !holder_quantity.is_finite() || holder_quantity < 0.0 {
        return None;
    }

    Some((holder_quantity / total_supply) * 100.0)
}

pub(super) fn parse_optional_f64(raw: Option<&str>) -> Option<f64> {
    raw?.trim().parse::<f64>().ok()
}

pub(super) fn parse_optional_i64(raw: Option<&str>) -> Option<i64> {
    raw?.trim().parse::<i64>().ok()
}

pub(super) fn parse_optional_u64(raw: Option<&str>) -> Option<u64> {
    raw?.trim().parse::<u64>().ok()
}

pub(super) fn build_news_query(token: &TokenSnapshot) -> String {
    match (&token.name, &token.symbol) {
        (Some(name), Some(symbol)) => format!("\"{}\" OR \"{}\" crypto BNB", name, symbol),
        (Some(name), None) => format!("\"{}\" crypto BNB", name),
        (None, Some(symbol)) => format!("\"{}\" crypto BNB", symbol),
        (None, None) => token.contract_address.clone(),
    }
}

pub(super) fn parse_google_news_rss(xml: &str) -> Vec<InvestigationSource> {
    let mut sources = Vec::new();
    let mut remainder = xml;

    while let Some(start) = remainder.find("<item>") {
        let after_start = &remainder[start + 6..];
        let Some(end) = after_start.find("</item>") else {
            break;
        };
        let item = &after_start[..end];
        let title = extract_xml_text(item, "title");
        let link = extract_xml_text(item, "link");
        if let (Some(title), Some(link)) = (title, link) {
            sources.push(InvestigationSource {
                source: source_name_from_url(&link),
                title,
                url: link,
            });
        }
        remainder = &after_start[end + 7..];
        if sources.len() >= 6 {
            break;
        }
    }

    sources
}

fn extract_xml_text(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    let raw = xml[start..end].trim();
    if raw.is_empty() {
        return None;
    }
    let raw = raw
        .trim_start_matches("<![CDATA[")
        .trim_end_matches("]]>")
        .replace("&amp;", "&")
        .replace("&#39;", "'")
        .replace("&quot;", "\"");
    Some(raw)
}

pub(super) fn source_name_from_url(url: &str) -> String {
    Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(|value| value.to_string()))
        .unwrap_or_else(|| "source".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_supply_with_decimals() {
        assert_eq!(
            format_supply(Some("1234500"), Some(4)).as_deref(),
            Some("123.45")
        );
        assert_eq!(
            format_supply(Some("1000"), Some(0)).as_deref(),
            Some("1000")
        );
    }

    #[test]
    fn computes_holder_ownership_pct() {
        let pct = compute_holder_ownership_pct(Some("1000000"), "125000").unwrap();
        assert!((pct - 12.5).abs() < f64::EPSILON);
    }

    #[test]
    fn parses_google_news_items() {
        let xml = r#"<rss><channel><item><title><![CDATA[Test title]]></title><link>https://example.com/post</link></item></channel></rss>"#;
        let items = parse_google_news_rss(xml);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Test title");
    }
}
