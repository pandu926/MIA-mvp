use serde::Serialize;
use serde_json::{json, Value};

use crate::research::{
    dexscreener::DexScreenerContext,
    heurist::HeuristDossier,
    launch_intelligence::{DeployerMemorySummary, WalletStructureSummary},
    linking::LinkedLaunchSummary,
    pattern_engine::PatternEngineSummary,
};

#[derive(Debug, Serialize)]
pub struct PremiumDossierArtifacts {
    pub executive_summary: String,
    pub sections: Value,
    pub citations: Value,
    pub source_status: Value,
    pub raw_payload: Value,
}

fn build_wallet_structure_section(wallet_structure: &WalletStructureSummary) -> Value {
    let mut evidence = wallet_structure.evidence.clone();
    evidence.extend(
        wallet_structure
            .top_flow_wallets
            .iter()
            .map(|wallet| format!("Flow wallet: {wallet}.")),
    );

    json!({
        "id": "wallet-structure",
        "title": "Wallet structure",
        "summary": wallet_structure.summary,
        "stage": "mvp",
        "source_agent": "mia_wallet_structure",
        "provider": "mia_internal_wallet_graph",
        "evidence": evidence,
    })
}

fn build_deployer_memory_section(deployer_memory: &DeployerMemorySummary) -> Value {
    json!({
        "id": "deployer-memory",
        "title": "Deployer memory",
        "summary": deployer_memory.summary,
        "stage": "mvp",
        "source_agent": "mia_deployer_memory",
        "provider": "mia_internal_history",
        "evidence": deployer_memory.evidence,
    })
}

fn build_dex_market_section(dex_context: Option<&DexScreenerContext>) -> Value {
    let mut evidence = Vec::new();
    let (summary, source_url, observed_at, fallback_note) = if let Some(context) = dex_context {
        evidence.push(format!("DexScreener summary: {}", context.summary));
        if let Some(pair_label) = &context.pair_label {
            let dex_label = context.dex_id.as_deref().unwrap_or("unknown dex");
            evidence.push(format!("Resolved pair: {pair_label} on {dex_label}."));
        }
        if let Some(pair_address) = &context.pair_address {
            evidence.push(format!("Pair address: {pair_address}."));
        }
        if let Some(price_usd) = &context.price_usd {
            evidence.push(format!("Spot price: ${price_usd}."));
        }
        if let Some(liquidity_usd) = context.liquidity_usd {
            evidence.push(format!("Liquidity: ${liquidity_usd:.2}."));
        }
        if let Some(market_cap) = context.market_cap {
            evidence.push(format!("Market cap: ${market_cap:.2}."));
        }
        if let Some(fdv) = context.fdv {
            evidence.push(format!("FDV: ${fdv:.2}."));
        }
        if let Some(age_label) = &context.age_label {
            evidence.push(format!("Pair age: {age_label}."));
        }
        if let Some(pair_created_at) = &context.pair_created_at {
            evidence.push(format!("Pair created at: {pair_created_at}."));
        }
        if let Some(h24) = context.volume_usd.h24 {
            evidence.push(format!("24H volume: ${h24:.2}."));
        }
        if let Some(h6) = context.volume_usd.h6 {
            evidence.push(format!("6H volume: ${h6:.2}."));
        }
        if let Some(h1) = context.volume_usd.h1 {
            evidence.push(format!("1H volume: ${h1:.2}."));
        }
        if let Some(m5) = context.volume_usd.m5 {
            evidence.push(format!("5M volume: ${m5:.2}."));
        }
        if let Some(h24) = context.price_change_pct.h24 {
            evidence.push(format!("24H price change: {h24:.2}%."));
        }
        if let Some(h6) = context.price_change_pct.h6 {
            evidence.push(format!("6H price change: {h6:.2}%."));
        }
        if let Some(h1) = context.price_change_pct.h1 {
            evidence.push(format!("1H price change: {h1:.2}%."));
        }
        if let Some(m5) = context.price_change_pct.m5 {
            evidence.push(format!("5M price change: {m5:.2}%."));
        }
        if context.txns.h24.buys.is_some() || context.txns.h24.sells.is_some() {
            evidence.push(format!(
                "24H transactions: {} buys / {} sells.",
                context.txns.h24.buys.unwrap_or_default(),
                context.txns.h24.sells.unwrap_or_default()
            ));
        }
        if context.txns.h6.buys.is_some() || context.txns.h6.sells.is_some() {
            evidence.push(format!(
                "6H transactions: {} buys / {} sells.",
                context.txns.h6.buys.unwrap_or_default(),
                context.txns.h6.sells.unwrap_or_default()
            ));
        }
        if context.txns.h1.buys.is_some() || context.txns.h1.sells.is_some() {
            evidence.push(format!(
                "1H transactions: {} buys / {} sells.",
                context.txns.h1.buys.unwrap_or_default(),
                context.txns.h1.sells.unwrap_or_default()
            ));
        }
        if context.txns.m5.buys.is_some() || context.txns.m5.sells.is_some() {
            evidence.push(format!(
                "5M transactions: {} buys / {} sells.",
                context.txns.m5.buys.unwrap_or_default(),
                context.txns.m5.sells.unwrap_or_default()
            ));
        }
        evidence.push(format!(
            "Market structure read: {}.",
            context.market_structure_label
        ));
        (
            context.summary.clone(),
            context.source_url.clone(),
            context.observed_at.clone(),
            context.fallback_note.clone(),
        )
    } else {
        (
            "Dex market context is temporarily unavailable on this deployment.".to_string(),
            None,
            None,
            Some("DexScreener returned no BSC pair or the provider was unavailable during report generation.".to_string()),
        )
    };

    json!({
        "id": "dex-market-context",
        "title": "Dex market structure",
        "summary": summary,
        "stage": "mvp",
        "source_agent": "dexscreener_market_context",
        "provider": "dexscreener-search",
        "source_url": source_url,
        "observed_at": observed_at,
        "fallback_note": fallback_note,
        "evidence": evidence,
    })
}

fn build_narrative_sections(
    heurist_dossier: Option<&HeuristDossier>,
    dex_context: Option<&DexScreenerContext>,
) -> Vec<Value> {
    let Some(heurist_dossier) = heurist_dossier else {
        return Vec::new();
    };

    heurist_dossier
        .results
        .iter()
        .map(|result| {
            let (provider, source_url, observed_at, fallback_note) =
                if result.section_id == "market_trend" {
                    (
                        Some("heurist_mesh+dexscreener".to_string()),
                        dex_context.as_ref().and_then(|ctx| ctx.source_url.clone()),
                        dex_context.as_ref().and_then(|ctx| ctx.observed_at.clone()),
                        dex_context
                            .as_ref()
                            .and_then(|ctx| ctx.fallback_note.clone()),
                    )
                } else {
                    (Some("heurist_mesh".to_string()), None, None, None)
                };

            json!({
                "id": result.section_id,
                "title": result.title,
                "summary": result.summary,
                "stage": "optional",
                "source_agent": result.agent_id,
                "query": result.query,
                "provider": provider,
                "source_url": source_url,
                "observed_at": observed_at,
                "fallback_note": fallback_note,
            })
        })
        .collect()
}

fn build_executive_summary(
    heurist_dossier: Option<&HeuristDossier>,
    dex_section: &Value,
    wallet_section: &Value,
    deployer_section: Option<&Value>,
    linked_section: Option<&Value>,
) -> String {
    let mut summary_parts = vec![
        "This dossier is evidence-first: MIA aggregates the live market, wallet, deployer, linked-pattern, and optional external layers so you can make the call yourself.".to_string(),
    ];

    if let Some(heurist_dossier) = heurist_dossier {
        let trimmed = heurist_dossier.executive_summary.trim();
        if !trimmed.is_empty() {
            summary_parts.push(trimmed.to_string());
        }
    }

    for section in [
        Some(dex_section),
        Some(wallet_section),
        deployer_section,
        linked_section,
    ] {
        if let Some(section) = section {
            if let Some(summary) = section.get("summary").and_then(Value::as_str) {
                let trimmed = summary.trim();
                if !trimmed.is_empty() {
                    summary_parts.push(trimmed.to_string());
                }
            }
        }
    }

    summary_parts
        .into_iter()
        .take(4)
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn build_premium_dossier_artifacts(
    token_address: &str,
    heurist_dossier: Option<HeuristDossier>,
    dex_context: Option<DexScreenerContext>,
    wallet_structure: WalletStructureSummary,
    deployer_memory: Option<DeployerMemorySummary>,
    linked_launch: Option<LinkedLaunchSummary>,
    pattern_engine: Option<PatternEngineSummary>,
) -> PremiumDossierArtifacts {
    let dex_section = build_dex_market_section(dex_context.as_ref());
    let wallet_section = build_wallet_structure_section(&wallet_structure);
    let deployer_section = deployer_memory.as_ref().map(build_deployer_memory_section);
    let linked_section = linked_launch.as_ref().map(|linked_launch| {
        json!({
            "id": "linked-launch-cluster",
            "title": "Linked launch cluster",
            "summary": linked_launch.summary,
            "stage": "mvp",
            "source_agent": "mia_internal_linking",
            "confidence": linked_launch.confidence,
            "provider": "mia_internal_linking",
            "evidence": linked_launch.evidence,
            "related_tokens": linked_launch.related_tokens,
            "repeated_wallets": linked_launch.repeated_wallets,
        })
    });
    let pattern_section = pattern_engine.as_ref().map(|engine| {
        json!({
            "id": "pattern-match-engine",
            "title": "Historical pattern context",
            "summary": engine.summary,
            "stage": "supporting",
            "source_agent": "mia_pattern_engine",
            "provider": "mia_pattern_engine",
            "confidence": engine
                .horizons
                .iter()
                .map(|item| format!("{}H {:.0}%", item.horizon_hours, item.confidence * 100.0))
                .collect::<Vec<_>>()
                .join(" | "),
            "evidence": engine.evidence,
            "details": {
                "model_version": engine.model_version,
                "horizons": engine.horizons,
            }
        })
    });

    let mut section_items = vec![dex_section.clone(), wallet_section.clone()];
    if let Some(section) = &deployer_section {
        section_items.push(section.clone());
    }
    if let Some(section) = &linked_section {
        section_items.push(section.clone());
    }
    section_items.extend(build_narrative_sections(
        heurist_dossier.as_ref(),
        dex_context.as_ref(),
    ));
    if let Some(section) = &pattern_section {
        section_items.push(section.clone());
    }
    let sections = Value::Array(section_items);

    let mut citations = heurist_dossier
        .as_ref()
        .map(|value| value.citations.clone())
        .unwrap_or_default();
    if let Some(context) = &dex_context {
        citations.push(json!({
            "type": "dexscreener",
            "provider": context.provider,
            "source_url": context.source_url,
            "observed_at": context.observed_at,
        }));
    }
    citations.push(json!({
        "type": "wallet_structure",
        "provider": "mia_internal_wallet_graph",
        "active_wallet_count": wallet_structure.active_wallet_count,
        "holder_count": wallet_structure.holder_count,
        "probable_cluster_wallets": wallet_structure.probable_cluster_wallets,
        "potential_cluster_wallets": wallet_structure.potential_cluster_wallets,
        "repeated_wallet_count": wallet_structure.repeated_wallet_count,
    }));
    if let Some(deployer_memory) = &deployer_memory {
        citations.push(json!({
            "type": "deployer_memory",
            "provider": "mia_internal_history",
            "trust_grade": deployer_memory.trust_grade,
            "trust_label": deployer_memory.trust_label,
            "total_launches": deployer_memory.total_launches,
            "rug_count": deployer_memory.rug_count,
            "graduated_count": deployer_memory.graduated_count,
            "recent_launches": deployer_memory.recent_launches,
        }));
    }
    if let Some(linked_launch) = &linked_launch {
        citations.push(json!({
            "type": "internal_linking",
            "token_address": token_address,
            "confidence": linked_launch.confidence,
            "related_tokens": linked_launch.related_tokens,
            "repeated_wallets": linked_launch.repeated_wallets,
        }));
    }
    if let Some(pattern_engine) = &pattern_engine {
        citations.push(json!({
            "type": "pattern_match_engine",
            "provider": "mia_pattern_engine",
            "model_version": pattern_engine.model_version,
            "horizons": pattern_engine.horizons,
        }));
    }
    let citations = Value::Array(citations);

    let mut raw_payload = heurist_dossier
        .as_ref()
        .map(|value| value.raw_payload.clone())
        .unwrap_or_else(|| json!({}));
    if !raw_payload.is_object() {
        raw_payload = json!({});
    }
    if let Value::Object(map) = &mut raw_payload {
        map.insert(
            "dexscreener_context".to_string(),
            serde_json::to_value(&dex_context).unwrap_or(Value::Null),
        );
        map.insert(
            "wallet_structure".to_string(),
            serde_json::to_value(&wallet_structure).unwrap_or(Value::Null),
        );
        map.insert(
            "deployer_memory".to_string(),
            serde_json::to_value(&deployer_memory).unwrap_or(Value::Null),
        );
        map.insert(
            "linked_launch_cluster".to_string(),
            serde_json::to_value(&linked_launch).unwrap_or(Value::Null),
        );
        map.insert(
            "pattern_engine".to_string(),
            serde_json::to_value(&pattern_engine).unwrap_or(Value::Null),
        );
    }

    let source_status = json!({
        "heurist_mesh": if let Some(heurist_dossier) = heurist_dossier.as_ref() {
            json!({
                "status": "ready",
                "provider": "Heurist Mesh REST API",
                "details": heurist_dossier.source_status,
            })
        } else {
            json!({
                "status": "optional_unavailable",
                "provider": "Heurist Mesh REST API",
                "note": "Narrative enrichment did not run or upstream access was unavailable. Premium launch intelligence is still complete without it."
            })
        },
        "dexscreener": {
            "status": if dex_context.is_some() { "ready" } else { "degraded" },
            "provider": dex_context.as_ref().map(|ctx| ctx.provider.clone()).unwrap_or_else(|| "dexscreener-search".to_string()),
            "source_url": dex_context.as_ref().and_then(|ctx| ctx.source_url.clone()),
            "observed_at": dex_context.as_ref().and_then(|ctx| ctx.observed_at.clone()),
            "fallback_note": dex_context.as_ref().and_then(|ctx| ctx.fallback_note.clone()),
        },
        "mia_wallet_structure": {
            "status": "ready",
            "provider": "mia_internal_wallet_graph",
            "active_wallet_count": wallet_structure.active_wallet_count,
            "probable_cluster_wallets": wallet_structure.probable_cluster_wallets,
            "potential_cluster_wallets": wallet_structure.potential_cluster_wallets,
            "repeated_wallet_count": wallet_structure.repeated_wallet_count,
        },
        "mia_deployer_memory": {
            "status": if deployer_memory.is_some() { "ready" } else { "empty" },
            "provider": "mia_internal_history"
        },
        "mia_internal_linking": {
            "status": if linked_launch.is_some() { "ready" } else { "empty" },
            "provider": "internal_wallet_linking"
        },
        "mia_pattern_engine": {
            "status": if pattern_engine.is_some() { "ready" } else { "empty" },
            "provider": "mia_pattern_engine"
        }
    });

    PremiumDossierArtifacts {
        executive_summary: build_executive_summary(
            heurist_dossier.as_ref(),
            &dex_section,
            &wallet_section,
            deployer_section.as_ref(),
            linked_section.as_ref(),
        ),
        sections,
        citations,
        source_status,
        raw_payload,
    }
}
