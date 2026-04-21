use std::sync::Arc;

use reqwest::Url;
use serde_json::Value;

use crate::config::Config;

use super::super::types::{
    ContractIntelligence, EtherscanSourceCodeRow, EtherscanTokenInfoRow, HolderAcquisitionSnapshot,
    HolderChangeSnapshot, HolderChangeWindowSnapshot, HolderDistributionSnapshot, HolderSnapshot,
    HolderSupplyBandSnapshot, HolderSupplySnapshot, MarketIntelligence,
    MoralisTokenHolderStatsResponse, MoralisTokenOwnersResponse,
    MoralisWalletTokenBalancesResponse, TokenSnapshot,
};
use super::helpers::{
    build_news_query, compute_holder_ownership_pct, etherscan_get, format_supply, moralis_get,
    parse_boolish, parse_google_news_rss, parse_optional_f64, parse_optional_i64,
    parse_optional_u64,
};

fn value_to_string(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(text)) => Some(text.clone()),
        Some(Value::Number(number)) => Some(number.to_string()),
        Some(Value::Bool(flag)) => Some(flag.to_string()),
        _ => None,
    }
}

fn value_to_u64(value: Option<&Value>) -> Option<u64> {
    match value {
        Some(Value::Number(number)) => number.as_u64(),
        Some(Value::String(text)) => parse_optional_u64(Some(text.as_str())),
        _ => None,
    }
}

fn value_to_i64(value: Option<&Value>) -> Option<i64> {
    match value {
        Some(Value::Number(number)) => number.as_i64(),
        Some(Value::String(text)) => parse_optional_i64(Some(text.as_str())),
        _ => None,
    }
}

fn value_to_f64(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Number(number)) => number.as_f64(),
        Some(Value::String(text)) => parse_optional_f64(Some(text.as_str())),
        _ => None,
    }
}

pub(crate) async fn fetch_contract_intelligence(
    config: Arc<Config>,
    token: &TokenSnapshot,
) -> ContractIntelligence {
    let address = token.contract_address.as_str();
    let deployer_address = token.deployer_address.as_str();
    let total_tx = i64::from(token.buy_count) + i64::from(token.sell_count);
    let mut intel = ContractIntelligence {
        provider: "external-intelligence".to_string(),
        ..ContractIntelligence::default()
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build();

    let Ok(client) = client else {
        intel
            .notes
            .push("Failed to build external metadata HTTP client.".to_string());
        return intel;
    };

    if let Some(api_key) = config
        .bscscan_api_key
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        intel.provider = "etherscan-v2-chain56".to_string();
        let token_info_fut = etherscan_get::<Vec<EtherscanTokenInfoRow>>(
            &client,
            &config,
            api_key,
            vec![
                ("module", "token".to_string()),
                ("action", "tokeninfo".to_string()),
                ("contractaddress", address.to_string()),
            ],
        );
        let source_code_fut = etherscan_get::<Vec<EtherscanSourceCodeRow>>(
            &client,
            &config,
            api_key,
            vec![
                ("module", "contract".to_string()),
                ("action", "getsourcecode".to_string()),
                ("address", address.to_string()),
            ],
        );
        let (token_info, source_code) = tokio::join!(token_info_fut, source_code_fut);

        match token_info {
            Ok(rows) => {
                if let Some(row) = rows.into_iter().next() {
                    intel.available = true;
                    intel.contract_name = row.token_name;
                    intel.token_type = row.token_type;
                    intel.total_supply_raw = row.total_supply.clone();
                    intel.decimals = row.divisor.and_then(|value| value.parse::<u32>().ok());
                    intel.total_supply = format_supply(row.total_supply.as_deref(), intel.decimals);
                    intel.description = row.description.filter(|value| !value.trim().is_empty());
                    intel.website = row.website.filter(|value| !value.trim().is_empty());
                    intel.twitter = row.twitter.filter(|value| !value.trim().is_empty());
                    intel.telegram = row.telegram.filter(|value| !value.trim().is_empty());
                    intel.discord = row.discord.filter(|value| !value.trim().is_empty());
                    if row.symbol.is_none() {
                        intel.notes.push(
                            "Token info endpoint returned metadata without symbol field."
                                .to_string(),
                        );
                    }
                }
            }
            Err(_) => intel
                .notes
                .push("Explorer token metadata is currently unavailable for this contract.".to_string()),
        }

        match source_code {
            Ok(rows) => {
                if let Some(row) = rows.into_iter().next() {
                    intel.source_verified = row
                        .source_code
                        .as_ref()
                        .map(|value| !value.trim().is_empty())
                        .unwrap_or(false);
                    intel.contract_name = intel.contract_name.or(row.contract_name);
                    intel.compiler_version = row.compiler_version;
                    intel.optimization_used =
                        row.optimization_used.as_deref().and_then(parse_boolish);
                    intel.optimization_runs = row.runs.and_then(|value| value.parse::<i64>().ok());
                    intel.proxy = row.proxy.as_deref().and_then(parse_boolish);
                    intel.implementation =
                        row.implementation.filter(|value| !value.trim().is_empty());
                }
            }
            Err(_) => intel
                .notes
                .push("Explorer source metadata is currently unavailable for this contract.".to_string()),
        }

        intel.notes.push(
            "Holder concentration and top-holder data are sourced from Moralis when activity clears the AI-score gate."
                .to_string(),
        );
    } else {
        intel.notes.push(
            "BSCSCAN_API_KEY is not configured, so explorer metadata and verified-source lookups are skipped."
                .to_string(),
        );
    }

    if let Some(api_key) = config
        .moralis_api_key
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        if total_tx > config.ai_score_min_tx_count {
            let creator_balance_path = format!("wallets/{deployer_address}/tokens");
            let top_holders_path = format!("erc20/{address}/owners");
            let holder_stats_path = format!("erc20/{address}/holders");
            let creator_balance_fut = moralis_get::<MoralisWalletTokenBalancesResponse>(
                &client,
                &config,
                api_key,
                &creator_balance_path,
                vec![
                    ("chain", "bsc".to_string()),
                    ("exclude_native", "true".to_string()),
                    ("token_addresses", address.to_string()),
                ],
            );
            let top_holders_fut = moralis_get::<MoralisTokenOwnersResponse>(
                &client,
                &config,
                api_key,
                &top_holders_path,
                vec![
                    ("chain", "bsc".to_string()),
                    ("limit", "10".to_string()),
                    ("order", "DESC".to_string()),
                ],
            );
            let holder_stats_fut = moralis_get::<MoralisTokenHolderStatsResponse>(
                &client,
                &config,
                api_key,
                &holder_stats_path,
                vec![("chain", "bsc".to_string())],
            );

            let (creator_balance, top_holders, holder_stats) =
                tokio::join!(creator_balance_fut, top_holders_fut, holder_stats_fut);

            match creator_balance {
                Ok(response) => {
                    if let Some(balance) = response
                        .result
                        .into_iter()
                        .find(|row| row.token_address.eq_ignore_ascii_case(address))
                    {
                        intel.available = true;
                        intel.provider = if intel.provider == "etherscan-v2-chain56" {
                            "etherscan-v2-chain56+moralis-wallet-v2.2".to_string()
                        } else {
                            "moralis-wallet-v2.2".to_string()
                        };
                        intel.owner_holding_pct =
                            balance.percentage_relative_to_total_supply.or_else(|| {
                                compute_holder_ownership_pct(
                                    balance.total_supply.as_deref(),
                                    &balance.balance,
                                )
                            });
                        if intel.total_supply_raw.is_none() {
                            intel.total_supply_raw = balance.total_supply.clone();
                        }
                        if intel.total_supply.is_none() {
                            intel.total_supply = balance.total_supply_formatted;
                        }
                        intel.notes.push(
                            "Creator holding percentage enriched from Moralis wallet token balances."
                                .to_string(),
                        );
                    } else {
                        intel.notes.push(
                            "Moralis did not return the deployer wallet balance for this token."
                                .to_string(),
                        );
                    }
                }
                Err(_) => intel
                    .notes
                    .push("Creator holding data is currently unavailable from Moralis.".to_string()),
            }

            match top_holders {
                Ok(response) => {
                    if !response.result.is_empty() {
                        intel.available = true;
                        if intel.provider == "etherscan-v2-chain56" {
                            intel.provider = "etherscan-v2-chain56+moralis-wallet-v2.2".to_string();
                        } else {
                            intel.provider = "moralis-wallet-v2.2".to_string();
                        }
                        if intel.total_supply_raw.is_none() {
                            intel.total_supply_raw = response.total_supply.clone();
                        }
                        let decimals = intel.decimals;
                        let total_supply_raw =
                            intel.total_supply_raw.clone().or(response.total_supply);
                        let mut owner_in_top_holders = false;
                        let top_rows = response
                            .result
                            .into_iter()
                            .map(|row| {
                                let is_owner =
                                    row.owner_address.eq_ignore_ascii_case(deployer_address);
                                if is_owner {
                                    owner_in_top_holders = true;
                                }
                                let balance_raw = row.balance.clone();
                                let quantity = row
                                    .balance_formatted
                                    .clone()
                                    .or_else(|| format_supply(Some(&balance_raw), decimals))
                                    .unwrap_or_else(|| balance_raw.clone());

                                HolderSnapshot {
                                    address: row.owner_address,
                                    quantity,
                                    quantity_raw: balance_raw.clone(),
                                    ownership_pct: row.percentage_relative_to_total_supply.or_else(
                                        || {
                                            compute_holder_ownership_pct(
                                                total_supply_raw.as_deref(),
                                                &balance_raw,
                                            )
                                        },
                                    ),
                                    is_owner,
                                    address_type: None,
                                    owner_label: row.owner_address_label,
                                    entity: row.entity,
                                    is_contract: row.is_contract,
                                }
                            })
                            .collect::<Vec<_>>();
                        intel.owner_in_top_holders = owner_in_top_holders;
                        intel.top_holders = top_rows;
                        intel.notes.push(
                            "Top holders enriched from Moralis token owners endpoint (top 10)."
                                .to_string(),
                        );
                    } else {
                        intel
                            .notes
                            .push("Moralis did not return top holders for this token.".to_string());
                    }
                }
                Err(_) => intel
                    .notes
                    .push("Top-holder data is currently unavailable from Moralis.".to_string()),
            }

            match holder_stats {
                Ok(stats) => {
                    intel.available = true;
                    if intel.holder_count.is_none() {
                        intel.holder_count = value_to_u64(stats.total_holders.as_ref());
                    }
                    if intel.indexed_holder_count.is_none() {
                        intel.indexed_holder_count = intel.holder_count;
                    }
                    intel.holder_supply = Some(HolderSupplySnapshot {
                        top10: HolderSupplyBandSnapshot {
                            supply: value_to_string(stats.holder_supply.top10.supply.as_ref()),
                            supply_pct: value_to_f64(
                                stats.holder_supply.top10.supply_percent.as_ref(),
                            ),
                        },
                        top25: HolderSupplyBandSnapshot {
                            supply: value_to_string(stats.holder_supply.top25.supply.as_ref()),
                            supply_pct: value_to_f64(
                                stats.holder_supply.top25.supply_percent.as_ref(),
                            ),
                        },
                        top50: HolderSupplyBandSnapshot {
                            supply: value_to_string(stats.holder_supply.top50.supply.as_ref()),
                            supply_pct: value_to_f64(
                                stats.holder_supply.top50.supply_percent.as_ref(),
                            ),
                        },
                        top100: HolderSupplyBandSnapshot {
                            supply: value_to_string(stats.holder_supply.top100.supply.as_ref()),
                            supply_pct: value_to_f64(
                                stats.holder_supply.top100.supply_percent.as_ref(),
                            ),
                        },
                    });
                    let one_hour = stats
                        .holder_change
                        .one_hour
                        .as_ref()
                        .or(stats.holder_change.five_min.as_ref())
                        .or(stats.holder_change.ten_min.as_ref())
                        .or(stats.holder_change.six_hours.as_ref());
                    let twenty_four_hours = stats
                        .holder_change
                        .twenty_four_hours
                        .as_ref()
                        .or(stats.holder_change.three_days.as_ref());
                    let seven_days = stats
                        .holder_change
                        .seven_days
                        .as_ref()
                        .or(stats.holder_change.thirty_days.as_ref());
                    intel.holder_change = Some(HolderChangeSnapshot {
                        one_hour: HolderChangeWindowSnapshot {
                            change: value_to_i64(
                                one_hour.and_then(|window| window.change.as_ref()),
                            ),
                            change_pct: value_to_f64(
                                one_hour.and_then(|window| window.change_percent.as_ref()),
                            ),
                        },
                        twenty_four_hours: HolderChangeWindowSnapshot {
                            change: value_to_i64(
                                twenty_four_hours.and_then(|window| window.change.as_ref()),
                            ),
                            change_pct: value_to_f64(
                                twenty_four_hours.and_then(|window| window.change_percent.as_ref()),
                            ),
                        },
                        seven_days: HolderChangeWindowSnapshot {
                            change: value_to_i64(
                                seven_days.and_then(|window| window.change.as_ref()),
                            ),
                            change_pct: value_to_f64(
                                seven_days.and_then(|window| window.change_percent.as_ref()),
                            ),
                        },
                    });
                    intel.holder_distribution = Some(HolderDistributionSnapshot {
                        whales: value_to_u64(stats.holder_distribution.whales.as_ref()),
                        sharks: value_to_u64(stats.holder_distribution.sharks.as_ref()),
                        dolphins: value_to_u64(stats.holder_distribution.dolphins.as_ref()),
                        fish: value_to_u64(stats.holder_distribution.fish.as_ref()),
                        octopus: value_to_u64(stats.holder_distribution.octopus.as_ref()),
                        crabs: value_to_u64(stats.holder_distribution.crabs.as_ref()),
                        shrimps: value_to_u64(stats.holder_distribution.shrimps.as_ref()),
                    });
                    intel.holders_by_acquisition = Some(HolderAcquisitionSnapshot {
                        swap: value_to_u64(stats.holders_by_acquisition.swap.as_ref()),
                        transfer: value_to_u64(stats.holders_by_acquisition.transfer.as_ref()),
                        airdrop: value_to_u64(stats.holders_by_acquisition.airdrop.as_ref()),
                    });
                    intel.notes.push(
                        "Holder metrics enriched from Moralis token holder stats.".to_string(),
                    );
                }
                Err(_) => intel
                    .notes
                    .push("Holder metrics are currently unavailable from Moralis.".to_string()),
            }
        } else {
            intel.notes.push(format!(
                "Moralis creator-balance lookup skipped because token activity is {} tx, below the >{} threshold.",
                total_tx, config.ai_score_min_tx_count
            ));
        }
    } else {
        intel.notes.push(
            "MORALIS_API_KEY is not configured, so deployer holding percentage is not enriched from Moralis."
                .to_string(),
        );
    }

    intel
}

pub(crate) async fn fetch_market_intelligence(
    _config: Arc<Config>,
    token: &TokenSnapshot,
) -> MarketIntelligence {
    let mut fallback = fetch_google_news_intelligence(token).await;
    fallback.notes.push(
        "Realtime X narrative is disabled in this MVP, so market intelligence uses Google News RSS only."
            .to_string(),
    );
    fallback
}

async fn fetch_google_news_intelligence(token: &TokenSnapshot) -> MarketIntelligence {
    let query = build_news_query(token);
    let mut intel = MarketIntelligence {
        provider: "google-news-rss".to_string(),
        ..MarketIntelligence::default()
    };

    let Ok(client) = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    else {
        intel
            .notes
            .push("Failed to build Google News HTTP client.".to_string());
        return intel;
    };

    let mut url = match Url::parse("https://news.google.com/rss/search") {
        Ok(value) => value,
        Err(err) => {
            intel
                .notes
                .push(format!("Failed to build Google News URL: {}", err));
            return intel;
        }
    };
    {
        let mut query_pairs = url.query_pairs_mut();
        query_pairs.append_pair("q", &query);
        query_pairs.append_pair("hl", "en-US");
        query_pairs.append_pair("gl", "US");
        query_pairs.append_pair("ceid", "US:en");
    }

    match client.get(url).send().await {
        Ok(response) => match response.error_for_status() {
            Ok(ok) => match ok.text().await {
                Ok(xml) => {
                    let sources = parse_google_news_rss(&xml);
                    if !sources.is_empty() {
                        intel.available = true;
                        intel.web_summary = Some(format!(
                            "{} recent news hits mention {}.",
                            sources.len(),
                            token
                                .name
                                .clone()
                                .or(token.symbol.clone())
                                .unwrap_or_else(|| token.contract_address.clone())
                        ));
                        intel.sources = sources;
                        intel.active_event =
                            intel.sources.first().map(|source| source.title.clone());
                        intel.narrative_alignment = Some(
                            "News-only fallback. Realtime X discourse is unavailable in this environment.".to_string(),
                        );
                    } else {
                        intel.notes.push(
                            "Google News RSS returned no clear items for the token query."
                                .to_string(),
                        );
                    }
                }
                Err(err) => intel
                    .notes
                    .push(format!("Failed to read Google News RSS payload: {}", err)),
            },
            Err(err) => intel
                .notes
                .push(format!("Google News RSS request failed: {}", err)),
        },
        Err(err) => intel
            .notes
            .push(format!("Google News RSS transport failed: {}", err)),
    }

    intel
}
