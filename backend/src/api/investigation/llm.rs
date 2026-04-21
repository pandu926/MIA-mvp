use std::sync::Arc;

use anyhow::Result;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{
    ai::gateway::{ChatMessage, LlmGateway, LlmRequest, RollingGateway},
    config::Config,
    error::AppError,
};

use super::types::{
    AgentScorecard, AnalysisPayload, AskMiaAnswer, AskMiaPayload, AskMiaRunContext,
    AskMiaTraceStep, ContractIntelligence, InternalEvidence, InvestigationAnalysis,
    InvestigationDeepResearchState, InvestigationTripwires, MarketIntelligence,
};

const ANALYSIS_MAX_TOKENS: u32 = 700;
const ANALYSIS_TEMPERATURE: f64 = 0.2;

fn normalize_confidence_label(value: &str) -> String {
    let trimmed = value.trim();
    let lowered = trimmed.to_lowercase();
    if matches!(lowered.as_str(), "low" | "medium" | "high") {
        return lowered;
    }
    if let Ok(score) = trimmed.parse::<f64>() {
        if score >= 0.75 {
            return "high".to_string();
        }
        if score >= 0.4 {
            return "medium".to_string();
        }
        return "low".to_string();
    }
    if lowered.contains("high") {
        return "high".to_string();
    }
    if lowered.contains("med") {
        return "medium".to_string();
    }
    "low".to_string()
}

fn normalize_confidence_value(value: &Value) -> String {
    match value {
        Value::String(text) => normalize_confidence_label(text),
        Value::Number(number) => normalize_confidence_label(&number.to_string()),
        Value::Bool(flag) => {
            if *flag {
                "high".to_string()
            } else {
                "low".to_string()
            }
        }
        _ => "low".to_string(),
    }
}

fn build_recent_transaction_facts(internal: &InternalEvidence, limit: usize) -> Vec<Value> {
    internal
        .recent_transactions
        .iter()
        .take(limit)
        .map(|tx| {
            serde_json::json!({
                "wallet": tx.wallet_address,
                "type": tx.tx_type,
                "amount_bnb": tx.amount_bnb,
                "block_number": tx.block_number,
                "created_at": tx.created_at,
            })
        })
        .collect()
}

fn build_deployer_recent_launch_facts(internal: &InternalEvidence, limit: usize) -> Vec<Value> {
    internal
        .deployer_recent_tokens
        .iter()
        .take(limit)
        .map(|token| {
            serde_json::json!({
                "symbol": token.symbol,
                "name": token.name,
                "contract_address": token.contract_address,
                "deployed_at": token.deployed_at,
                "buy_count": token.buy_count,
                "sell_count": token.sell_count,
                "volume_bnb": token.volume_bnb,
            })
        })
        .collect()
}

fn build_top_holder_facts(
    contract_intelligence: &ContractIntelligence,
    limit: usize,
) -> Vec<Value> {
    contract_intelligence
        .top_holders
        .iter()
        .take(limit)
        .map(|holder| {
            serde_json::json!({
                "address": holder.address,
                "ownership_pct": holder.ownership_pct,
                "is_owner": holder.is_owner,
                "owner_label": holder.owner_label,
                "entity": holder.entity,
                "is_contract": holder.is_contract,
            })
        })
        .collect()
}

fn build_operator_pattern_launch_facts(internal: &InternalEvidence, limit: usize) -> Vec<Value> {
    internal
        .operator_family
        .related_launches
        .iter()
        .take(limit)
        .map(|launch| {
            serde_json::json!({
                "contract_address": launch.contract_address,
                "symbol": launch.symbol,
                "name": launch.name,
                "deployer_address": launch.deployer_address,
                "deployed_at": launch.deployed_at,
                "is_rug": launch.is_rug,
                "graduated": launch.graduated,
                "overlap_wallets": launch.overlap_wallets,
            })
        })
        .collect()
}

fn build_contract_source_facts(contract_intelligence: &ContractIntelligence) -> Value {
    serde_json::json!({
        "provider": contract_intelligence.provider,
        "available": contract_intelligence.available,
        "source_verified": contract_intelligence.source_verified,
        "contract_name": contract_intelligence.contract_name,
        "compiler_version": contract_intelligence.compiler_version,
        "optimization_used": contract_intelligence.optimization_used,
        "optimization_runs": contract_intelligence.optimization_runs,
        "proxy": contract_intelligence.proxy,
        "implementation": contract_intelligence.implementation,
        "token_type": contract_intelligence.token_type,
        "total_supply": contract_intelligence.total_supply,
        "total_supply_raw": contract_intelligence.total_supply_raw,
        "decimals": contract_intelligence.decimals,
    })
}

fn build_token_facts(
    internal: &InternalEvidence,
    contract_intelligence: &ContractIntelligence,
) -> Value {
    serde_json::json!({
        "contract_address": internal.token.contract_address,
        "name": internal.token.name,
        "symbol": internal.token.symbol,
        "deployer_address": internal.token.deployer_address,
        "deployed_at": internal.token.deployed_at,
        "block_number": internal.token.block_number,
        "tx_hash": internal.token.tx_hash,
        "initial_liquidity_bnb": internal.token.initial_liquidity_bnb,
        "buy_count": internal.token.buy_count,
        "sell_count": internal.token.sell_count,
        "volume_bnb": internal.token.volume_bnb,
        "participant_wallet_count": internal.token.participant_wallet_count,
        "indexed_holder_count": contract_intelligence.indexed_holder_count.or(contract_intelligence.holder_count),
        "graduated": internal.token.graduated,
        "honeypot_detected": internal.token.honeypot_detected,
        "is_rug": internal.token.is_rug,
    })
}

fn build_deployer_facts(internal: &InternalEvidence) -> Option<Value> {
    internal.deployer.as_ref().map(|deployer| {
        serde_json::json!({
            "address": deployer.address,
            "total_tokens_deployed": deployer.total_tokens_deployed,
            "rug_count": deployer.rug_count,
            "graduated_count": deployer.graduated_count,
            "honeypot_detected": deployer.honeypot_detected,
            "trust_grade": deployer.trust_grade,
            "first_seen_at": deployer.first_seen_at,
            "last_seen_at": deployer.last_seen_at,
        })
    })
}

fn build_deployer_memory_facts(internal: &InternalEvidence) -> Option<Value> {
    internal.deployer_memory.as_ref().map(|memory| {
        serde_json::json!({
            "trust_grade": memory.trust_grade,
            "total_launches": memory.total_launches,
            "rug_count": memory.rug_count,
            "graduated_count": memory.graduated_count,
            "honeypot_history": memory.honeypot_history,
            "first_seen_at": memory.first_seen_at,
            "last_seen_at": memory.last_seen_at,
            "recent_launches": memory.recent_launches.iter().map(|launch| serde_json::json!({
                "contract_address": launch.contract_address,
                "symbol": launch.symbol,
                "name": launch.name,
                "is_rug": launch.is_rug,
                "graduated": launch.graduated,
                "deployed_at": launch.deployed_at,
                "buy_count": launch.buy_count,
                "sell_count": launch.sell_count,
                "volume_bnb": launch.volume_bnb,
            })).collect::<Vec<_>>(),
        })
    })
}

fn build_wallet_observation_facts(
    internal: &InternalEvidence,
    contract_intelligence: &ContractIntelligence,
) -> Value {
    serde_json::json!({
        "participant_wallet_count": internal.token.participant_wallet_count,
        "indexed_holder_count": contract_intelligence.indexed_holder_count.or(contract_intelligence.holder_count),
        "active_wallet_count": internal.wallet_structure.active_wallet_count,
        "owner_holding_pct": contract_intelligence.owner_holding_pct,
        "owner_in_top_holders": contract_intelligence.owner_in_top_holders,
        "top_holders": build_top_holder_facts(contract_intelligence, 10),
        "holder_supply": contract_intelligence.holder_supply,
        "holder_change": contract_intelligence.holder_change,
        "holder_distribution": contract_intelligence.holder_distribution,
        "holders_by_acquisition": contract_intelligence.holders_by_acquisition,
        "top_flow_wallets": internal.wallet_structure.top_flow_wallets,
        "cluster_signals": {
            "probable_cluster_wallets": internal.wallet_structure.probable_cluster_wallets,
            "potential_cluster_wallets": internal.wallet_structure.potential_cluster_wallets,
            "repeated_wallet_count": internal.wallet_structure.repeated_wallet_count,
        }
    })
}

fn build_operator_pattern_facts(internal: &InternalEvidence) -> Value {
    serde_json::json!({
        "pattern_is_identity_proof": false,
        "probable_cluster_wallets": internal.operator_family.probable_cluster_wallets,
        "potential_cluster_wallets": internal.operator_family.potential_cluster_wallets,
        "repeated_wallet_count": internal.operator_family.repeated_wallet_count,
        "related_launch_count": internal.operator_family.related_launch_count,
        "related_deployer_count": internal.operator_family.related_deployer_count,
        "seller_to_new_builder_count": internal.operator_family.seller_to_new_builder_count,
        "seller_reentry_wallet_count": internal.operator_family.seller_reentry_wallet_count,
        "related_launches": build_operator_pattern_launch_facts(internal, 5),
        "repeated_wallets": internal.operator_family.repeated_wallets,
        "migrated_wallets": internal.operator_family.migrated_wallets,
    })
}

fn buy_share_pct(internal: &InternalEvidence) -> f64 {
    let total_tx = internal.token.buy_count + internal.token.sell_count;
    if total_tx <= 0 {
        return 50.0;
    }

    (internal.token.buy_count as f64 / total_tx as f64) * 100.0
}

fn next_participant_target(participants: i32) -> i32 {
    match participants {
        i32::MIN..=24 => 40,
        25..=59 => 80,
        60..=119 => 140,
        _ => participants + (participants / 4).max(25),
    }
}

fn top_holder_concentration_pct(contract_intelligence: &ContractIntelligence) -> Option<f64> {
    contract_intelligence
        .holder_supply
        .as_ref()
        .and_then(|supply| supply.top10.supply_pct)
}

pub(crate) fn build_investigation_tripwires(
    internal: &InternalEvidence,
    contract_intelligence: &ContractIntelligence,
    analysis: &InvestigationAnalysis,
    deep_research: &InvestigationDeepResearchState,
) -> InvestigationTripwires {
    let total_tx = internal.token.buy_count + internal.token.sell_count;
    let active_wallet_count =
        i32::try_from(internal.wallet_structure.active_wallet_count).unwrap_or(i32::MAX);
    let participant_wallets = internal
        .token
        .participant_wallet_count
        .max(active_wallet_count);
    let next_wallet_target = next_participant_target(participant_wallets);
    let buy_share = buy_share_pct(internal);
    let owner_holding_pct = contract_intelligence.owner_holding_pct;
    let top10_concentration = top_holder_concentration_pct(contract_intelligence);
    let operator_confidence = internal.operator_family.confidence.to_ascii_lowercase();
    let has_operator_pattern = matches!(operator_confidence.as_str(), "medium" | "high");
    let critical_whales = internal.whale_activity_24h.critical_alerts;

    let watching_for = if analysis.score.is_none() {
        format!(
            "MIA is waiting for this launch to clear more than {} total transactions or attach a deep research report before it opens a live AI score. Until then, the key question is whether wallet breadth becomes real instead of thin recycled flow.",
            deep_research.ai_score_gate_tx_count
        )
    } else if deep_research.report_cached {
        "MIA is watching whether live holder, builder, and flow behavior still supports the deep research thesis. If the structure changes materially, the score should move with it.".to_string()
    } else if deep_research.auto_requested {
        "MIA is watching for the queued deep research report to land while checking whether holder breadth and live flow still support the current read.".to_string()
    } else {
        "MIA is watching whether participation keeps broadening without concentration, whale exit pressure, or operator-pattern risk getting worse.".to_string()
    };

    let upgrade_trigger = if analysis.score.is_none() {
        format!(
            "Upgrade from no-score mode only after activity moves above {} total transactions or a deep research report is attached.",
            deep_research.ai_score_gate_tx_count
        )
    } else if let Some(top10_pct) = top10_concentration.filter(|pct| *pct >= 70.0) {
        format!(
            "Upgrade only if top-holder concentration cools from the current {:.1}% band and participant wallets expand toward at least {}.",
            top10_pct, next_wallet_target
        )
    } else if has_operator_pattern {
        format!(
            "Upgrade only if wallet breadth keeps expanding toward {} while operator-pattern pressure stops intensifying across related launches.",
            next_wallet_target
        )
    } else if critical_whales > 0 {
        "Upgrade only if whale-sized flow gets absorbed cleanly and buy pressure stays in control after the spike.".to_string()
    } else {
        format!(
            "Upgrade if participant wallets broaden toward {} and buy-side flow stays ahead of sells without a new concentration spike.",
            next_wallet_target
        )
    };

    let risk_trigger = if let Some(owner_pct) = owner_holding_pct.filter(|pct| *pct >= 8.0) {
        format!(
            "Raise risk immediately if the deployer or early owner wallet starts distributing more supply from its current {:.1}% visible position.",
            owner_pct
        )
    } else if let Some(top10_pct) = top10_concentration.filter(|pct| *pct >= 75.0) {
        format!(
            "Raise risk if top-holder concentration pushes further above {:.1}% or if whales start exiting while breadth stays thin.",
            top10_pct
        )
    } else if has_operator_pattern {
        "Raise risk if repeated-wallet overlap spreads to more launches or seller-to-new-builder links keep increasing.".to_string()
    } else {
        "Raise risk if wallet breadth stalls, buy pressure fades, or concentration starts climbing again.".to_string()
    };

    let deep_research_trigger = if deep_research.report_cached {
        "Deep research is already attached. Rerun it only if source quality degrades or the launch structure changes materially.".to_string()
    } else if deep_research.auto_requested {
        format!(
            "Deep research has already been queued because activity cleared the {} total transaction threshold.",
            deep_research.auto_threshold_tx_count
        )
    } else if i64::from(total_tx) >= deep_research.auto_threshold_tx_count {
        format!(
            "Deep research should be opened now because activity has already cleared the {} transaction auto threshold.",
            deep_research.auto_threshold_tx_count
        )
    } else {
        format!(
            "Deep research auto-starts once total transactions reach {}. It can still be opened manually below that threshold.",
            deep_research.auto_threshold_tx_count
        )
    };

    let invalidation_trigger = if analysis.score.is_none() {
        format!(
            "Invalidate the early read if the token still cannot clear {} total transactions, or if sell pressure takes control before real breadth arrives.",
            deep_research.ai_score_gate_tx_count
        )
    } else if buy_share < 50.0 {
        "Invalidate the current read if sell pressure keeps leading and no new organic wallets arrive.".to_string()
    } else {
        "Invalidate the current read if buy pressure fades, breadth stops expanding, or a new builder/operator warning appears.".to_string()
    };

    InvestigationTripwires {
        headline: "What would make MIA change its mind?".to_string(),
        watching_for: clean_sentence(&watching_for, "MIA is monitoring the next meaningful change."),
        upgrade_trigger: clean_sentence(&upgrade_trigger, "Upgrade requires stronger proof."),
        risk_trigger: clean_sentence(&risk_trigger, "Risk rises if structure worsens."),
        deep_research_trigger: clean_sentence(
            &deep_research_trigger,
            "Deep research is triggered when evidence depth is needed.",
        ),
        invalidation_trigger: clean_sentence(
            &invalidation_trigger,
            "Invalidate the read when the thesis breaks.",
        ),
    }
}

fn build_analysis_evidence_payload(
    internal: &InternalEvidence,
    contract_intelligence: &ContractIntelligence,
    deep_research_context: Option<&Value>,
) -> Value {
    serde_json::json!({
        "token_facts": build_token_facts(internal, contract_intelligence),
        "deployer_facts": build_deployer_facts(internal),
        "deployer_recent_launches": build_deployer_recent_launch_facts(internal, 5),
        "deployer_memory_facts": build_deployer_memory_facts(internal),
        "recent_transactions": build_recent_transaction_facts(internal, 12),
        "whale_signals": {
            "watch_alerts": internal.whale_activity_24h.watch_alerts,
            "critical_alerts": internal.whale_activity_24h.critical_alerts,
            "latest_levels": internal.whale_activity_24h.latest_levels,
        },
        "wallet_observations": build_wallet_observation_facts(internal, contract_intelligence),
        "operator_pattern_signals": build_operator_pattern_facts(internal),
        "contract_source_facts": build_contract_source_facts(contract_intelligence),
        "premium_deep_research": deep_research_context,
    })
}

pub(crate) async fn synthesize_analysis(
    config: Arc<Config>,
    internal: &InternalEvidence,
    contract_intelligence: &ContractIntelligence,
    market_intelligence: &MarketIntelligence,
    deep_research_context: Option<&Value>,
) -> InvestigationAnalysis {
    let gateway = RollingGateway::new(
        &config.llm_api_url,
        &config.llm_api_key,
        config.llm_models.clone(),
    );

    let system = "You are MIA One, an autonomous launch investigator for Four.Meme on BNB Chain. You receive structured evidence already collected by tools. Use only the provided evidence. Prioritize transaction facts, deployer history, holder concentration, contract/source facts, whale pressure, and any premium_deep_research context. Treat operator_pattern_signals and cluster_signals as tentative behavioral pattern evidence, not identity proof. Ignore any desire to recreate legacy rule scores, cached narratives, stale ranking context, or weak public-news framing unless premium_deep_research directly supports it. Return strict JSON with keys score, label, verdict, conviction, confidence, executive_summary, primary_reason, primary_risk, supporting_points, thesis, risks, next_actions. score must be an integer from 0 to 100. label and verdict must be one of AVOID, WATCH, SPECULATIVE, HIGH CONVICTION. supporting_points/thesis/risks/next_actions must be arrays of short strings.";
    let evidence_json = serde_json::to_string_pretty(&build_analysis_evidence_payload(
        internal,
        contract_intelligence,
        deep_research_context,
    ))
    .unwrap_or_else(|_| "{}".to_string());
    let user = format!(
        "Analyze this token for a human operator. Focus on whether the launch structure is tradable, whether the deployer and holder evidence is trustworthy, whether wallet behavior looks coordinated, and what the next move should be. Use only provable facts and clearly labeled pattern signals from the payload. JSON only.\n\n{}",
        evidence_json
    );

    let request = LlmRequest {
        model: String::new(),
        messages: vec![ChatMessage::system(system), ChatMessage::user(user)],
        temperature: ANALYSIS_TEMPERATURE,
        max_tokens: ANALYSIS_MAX_TOKENS,
        reasoning: crate::ai::gateway::ReasoningConfig { effort: "high" },
    };

    match gateway.generate(&request).await {
        Ok(response) => {
            let text = response.content().to_string();
            match parse_json_payload::<AnalysisPayload>(&text) {
                Ok(payload) => InvestigationAnalysis {
                    provider: response.model,
                    score: Some(payload.score.clamp(0, 100)),
                    label: Some(payload.label),
                    verdict: payload.verdict,
                    conviction: normalize_confidence_value(&payload.conviction),
                    confidence: normalize_confidence_value(&payload.confidence),
                    executive_summary: payload.executive_summary,
                    primary_reason: payload.primary_reason,
                    primary_risk: payload.primary_risk,
                    supporting_points: payload.supporting_points,
                    thesis: payload.thesis,
                    risks: payload.risks,
                    next_actions: payload.next_actions,
                    raw: Some(text),
                },
                Err(err) => build_unscored_analysis(
                    internal,
                    market_intelligence,
                    Some(format!(
                        "LLM returned non-JSON investigation output: {}",
                        err
                    )),
                ),
            }
        }
        Err(err) => build_unscored_analysis(
            internal,
            market_intelligence,
            Some(format!("LLM synthesis failed: {}", err)),
        ),
    }
}

fn fallback_analysis(
    internal: &InternalEvidence,
    market_intelligence: &MarketIntelligence,
    note: Option<String>,
) -> InvestigationAnalysis {
    let mut risks = internal.verdict.concerns.clone();
    risks.extend(market_intelligence.risk_flags.clone());
    risks.truncate(4);

    let mut next_actions = internal
        .verdict
        .next_actions
        .iter()
        .map(|action| action.label.clone())
        .collect::<Vec<_>>();
    if next_actions.is_empty() {
        next_actions = vec![
            "Open Replay Lab".to_string(),
            "Monitor whale flow".to_string(),
            "Review deployer history".to_string(),
        ];
    }

    let mut executive_summary = internal.verdict.headline.clone();
    if let Some(summary) = market_intelligence
        .x_summary
        .clone()
        .or_else(|| market_intelligence.web_summary.clone())
    {
        executive_summary = format!("{} {}", executive_summary, summary);
    }

    InvestigationAnalysis {
        provider: "fallback-heuristic".to_string(),
        score: Some(internal.decision_scorecard.decision_score),
        label: Some(internal.decision_scorecard.verdict.clone()),
        verdict: internal.verdict.label.clone(),
        conviction: internal.verdict.confidence_label.to_lowercase(),
        confidence: internal.verdict.confidence_label.to_lowercase(),
        executive_summary,
        primary_reason: internal.decision_scorecard.primary_reason.clone(),
        primary_risk: internal.decision_scorecard.primary_risk.clone(),
        supporting_points: internal.verdict.evidence.iter().take(4).cloned().collect(),
        thesis: internal.verdict.evidence.iter().take(4).cloned().collect(),
        risks,
        next_actions,
        raw: note,
    }
}

pub(crate) fn build_agent_scorecard(analysis: &InvestigationAnalysis) -> AgentScorecard {
    let confidence_label = normalize_confidence_label(&analysis.confidence);
    AgentScorecard {
        score: analysis.score.unwrap_or(50).clamp(0, 100),
        label: analysis
            .label
            .clone()
            .unwrap_or_else(|| analysis.verdict.clone()),
        confidence_label: confidence_label.clone(),
        headline: format!(
            "{} · {} confidence",
            analysis
                .label
                .clone()
                .unwrap_or_else(|| analysis.verdict.clone()),
            confidence_label
        ),
        summary: analysis.executive_summary.clone(),
        primary_reason: analysis.primary_reason.clone(),
        primary_risk: analysis.primary_risk.clone(),
        supporting_points: analysis.supporting_points.clone(),
    }
}

pub(crate) fn build_unscored_analysis(
    internal: &InternalEvidence,
    market_intelligence: &MarketIntelligence,
    note: Option<String>,
) -> InvestigationAnalysis {
    let fallback = fallback_analysis(internal, market_intelligence, note);
    InvestigationAnalysis {
        provider: "budget-gated-heuristic".to_string(),
        score: None,
        label: None,
        verdict: fallback.verdict,
        conviction: fallback.conviction,
        confidence: fallback.confidence,
        executive_summary: fallback.executive_summary,
        primary_reason: fallback.primary_reason,
        primary_risk: fallback.primary_risk,
        supporting_points: fallback.supporting_points,
        thesis: fallback.thesis,
        risks: fallback.risks,
        next_actions: fallback.next_actions,
        raw: fallback.raw,
    }
}

pub(crate) fn validate_ask_mia_question(question: &str) -> Result<(), AppError> {
    if question.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Question is required for Ask MIA.".to_string(),
        ));
    }
    if question.chars().count() > 400 {
        return Err(AppError::BadRequest(
            "Question is too long. Keep it under 400 characters.".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn build_ask_mia_grounded_layers(
    internal: &InternalEvidence,
    market_intelligence: &MarketIntelligence,
    run_context: Option<&AskMiaRunContext>,
) -> Vec<String> {
    let mut layers = vec![
        "agent_score".to_string(),
        "token_facts".to_string(),
        "transactions".to_string(),
        "wallets".to_string(),
        "operator_family".to_string(),
        "deployer".to_string(),
    ];
    let _ = market_intelligence;
    if internal.whale_activity_24h.watch_alerts > 0
        || internal.whale_activity_24h.critical_alerts > 0
    {
        layers.push("whales".to_string());
    }
    if run_context.is_some() {
        layers.push("run_journal".to_string());
    }
    layers
}

pub(crate) fn build_ask_mia_trace(tool_trace: &[String]) -> Vec<AskMiaTraceStep> {
    tool_trace
        .iter()
        .map(|tool| {
            let (title, detail) = match tool.as_str() {
                "get_token_overview" => (
                    "Token overview",
                    "Resolve the token identity, launch facts, and baseline activity before answering.",
                ),
                "get_risk_snapshot" => (
                    "Risk snapshot",
                    "Pull the composite risk read and concentration-sensitive warnings.",
                ),
                "get_agent_scorecard" => (
                    "Agent scorecard",
                    "Read the AI-generated score, current read, and dominant reason/risk summary.",
                ),
                "get_market_structure" => (
                    "Market structure",
                    "Check whether buy and sell pressure still support the current move.",
                ),
                "get_wallet_structure" => (
                    "Wallet structure",
                    "Inspect holder spread, owner visibility, and concentration in the wallet map.",
                ),
                "get_operator_family" => (
                    "Operator pattern",
                    "Look for repeated-wallet and migration patterns that suggest a coordinated operator family.",
                ),
                "get_deployer_memory" => (
                    "Builder memory",
                    "Review the deployer's launch history, trust grade, and repeat behavior.",
                ),
                "get_whale_and_flow_signals" => (
                    "Whale and flow signals",
                    "Read recent large flow, unusual size, and transaction pressure.",
                ),
                "get_ml_context" => (
                    "ML context",
                    "Attach MIA's internal ranking and replay proof context.",
                ),
                "get_narrative_context" => (
                    "Narrative context",
                    "Check whether public narrative supports or lags the on-chain activity.",
                ),
                _ => (
                    "Internal tool",
                    "Use a grounded internal read before producing the final answer.",
                ),
            };

            AskMiaTraceStep {
                tool: tool.clone(),
                title: title.to_string(),
                detail: detail.to_string(),
            }
        })
        .collect()
}

pub(crate) fn build_ask_mia_evidence_summary(
    question: &str,
    internal: &InternalEvidence,
    contract_intelligence: &ContractIntelligence,
    market_intelligence: &MarketIntelligence,
    run_context: Option<&AskMiaRunContext>,
) -> Value {
    let recent_transactions = build_recent_transaction_facts(internal, 8);
    let deployer_recent = build_deployer_recent_launch_facts(internal, 5);

    serde_json::json!({
        "question": question,
        "token": build_token_facts(internal, contract_intelligence),
        "agent_scorecard": internal.agent_scorecard.as_ref().map(|scorecard| serde_json::json!({
            "score": scorecard.score,
            "label": scorecard.label,
            "confidence_label": scorecard.confidence_label,
            "headline": scorecard.headline,
            "summary": scorecard.summary,
            "primary_reason": scorecard.primary_reason,
            "primary_risk": scorecard.primary_risk,
            "supporting_points": scorecard.supporting_points,
        })),
        "deployer": build_deployer_facts(internal),
        "deployer_recent_tokens": deployer_recent,
        "contract_source_facts": build_contract_source_facts(contract_intelligence),
        "wallets": build_wallet_observation_facts(internal, contract_intelligence),
        "operator_family": build_operator_pattern_facts(internal),
        "whales": {
            "watch_alerts": internal.whale_activity_24h.watch_alerts,
            "critical_alerts": internal.whale_activity_24h.critical_alerts,
            "latest_levels": internal.whale_activity_24h.latest_levels,
        },
        "recent_transactions": recent_transactions,
        "deployer_memory": build_deployer_memory_facts(internal),
        "optional_market_context": if market_intelligence.available {
            Some(serde_json::json!({
                "provider": market_intelligence.provider,
                "sources": market_intelligence.sources.iter().take(4).map(|source| serde_json::json!({
                    "title": source.title,
                    "url": source.url,
                    "source": source.source,
                })).collect::<Vec<_>>(),
                "active_event": market_intelligence.active_event,
            }))
        } else {
            None
        },
        "run_context": run_context.map(|context| serde_json::json!({
            "run_id": context.run_id,
            "status": context.status,
            "current_stage": context.current_stage,
            "continuity_note": context.continuity_note,
            "latest_reason": context.latest_reason,
            "latest_evidence_delta": context.latest_evidence_delta,
            "recent_events": context.recent_events.iter().map(|event| serde_json::json!({
                "label": event.label,
                "detail": event.detail,
                "at": event.at,
            })).collect::<Vec<_>>(),
        })),
    })
}

pub(crate) fn build_ask_mia_fallback(
    question: &str,
    internal: &InternalEvidence,
    _market_intelligence: &MarketIntelligence,
) -> AskMiaAnswer {
    let question_lower = question.to_ascii_lowercase();
    let builder_grade = internal
        .deployer
        .as_ref()
        .map(|item| item.trust_grade.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let total_trades = internal.token.buy_count + internal.token.sell_count;
    let buy_share = if total_trades > 0 {
        (internal.token.buy_count as f64 / total_trades as f64) * 100.0
    } else {
        50.0
    };
    let owner_share: Option<f64> = None;
    let mut evidence = Vec::new();
    if let Some(scorecard) = &internal.agent_scorecard {
        evidence.push(format!(
            "MIA's current AI score is {} out of 100 with {} confidence and current read {}.",
            scorecard.score, scorecard.confidence_label, scorecard.label
        ));
    } else {
        evidence.push(
            "MIA has not opened an AI score yet because this token is still below the live activity budget gate."
                .to_string(),
        );
    }
    if let Some(risk) = &internal.risk {
        evidence.push(format!(
            "Internal fallback risk snapshot is {} out of 100 in the {} bucket.",
            risk.composite_score, risk.risk_category
        ));
        if let Some(wallet_score) = risk.wallet_concentration_score {
            evidence.push(format!(
                "Internal concentration heuristic is {} out of 100.",
                wallet_score
            ));
        }
    }
    evidence.push(format!(
        "Tracked flow shows {} buys versus {} sells, with buy-side share at {:.0}%.",
        internal.token.buy_count, internal.token.sell_count, buy_share
    ));
    if let Some(deployer) = &internal.deployer {
        evidence.push(format!(
            "The deployer has a {} grade with {} launches, {} rugs, and {} graduates.",
            deployer.trust_grade,
            deployer.total_tokens_deployed,
            deployer.rug_count,
            deployer.graduated_count
        ));
    }
    if internal.operator_family.confidence != "low" {
        evidence.push(format!(
            "Operator-family pattern signal is {} with {} related launch(es), {} related deployer wallet(s), and {} repeated wallet(s).",
            internal.operator_family.confidence,
            internal.operator_family.related_launch_count,
            internal.operator_family.related_deployer_count,
            internal.operator_family.repeated_wallet_count
        ));
    }
    if let Some(scorecard) = &internal.agent_scorecard {
        evidence.push(format!(
            "The AI investigation score is {} out of 100 with current read {}.",
            scorecard.score, scorecard.label
        ));
    }

    let (short_answer, why, next_move) = if question_lower.contains("organic")
        || question_lower.contains("manufactured")
    {
        (
            if buy_share >= 55.0
                && internal.deployer.as_ref().map(|d| d.rug_count).unwrap_or(0) == 0
            {
                "It looks more organic than manufactured right now.".to_string()
            } else {
                "It looks more manufactured or fragile than fully organic right now.".to_string()
            },
            "MIA compares live trade flow, builder memory, holder concentration, and repeated-wallet patterns. When flow is narrow or builder history is mixed, the setup reads as less organic.".to_string(),
            "Keep this in watch mode until participation broadens and the narrative holds up.".to_string(),
        )
    } else if question_lower.contains("deployer") || question_lower.contains("previous") {
        (
            format!(
                "The builder profile is currently graded {}.",
                builder_grade.to_uppercase()
            ),
            "MIA uses deployer memory to compare this token against prior launches from the same wallet. The main signal here is whether the builder repeatedly launches tokens that graduate cleanly or fail quickly.".to_string(),
            "Treat builder history as a trust modifier, not a standalone buy signal."
                .to_string(),
        )
    } else if question_lower.contains("sybil")
        || question_lower.contains("operator")
        || question_lower.contains("coordinated")
        || question_lower.contains("recycled")
    {
        (
            if internal.operator_family.confidence == "high" {
                "MIA sees a strong coordinated-operator pattern warning here.".to_string()
            } else if internal.operator_family.confidence == "medium" {
                "MIA sees a medium coordinated-operator pattern warning here.".to_string()
            } else {
                "MIA does not yet see a strong coordinated-operator pattern here.".to_string()
            },
            clean_sentence(
                &internal.operator_family.summary,
                "MIA checks repeated wallets, seller migration, and related launches to decide whether the pattern looks recycled.",
            ),
            "Use operator-pattern warnings as a capital-protection signal. If this rises while wallet breadth stays weak, stay cautious.".to_string(),
        )
    } else if question_lower.contains("score")
        || question_lower.contains("pulled")
        || question_lower.contains("decision")
    {
        if let Some(scorecard) = &internal.agent_scorecard {
            (
                format!(
                    "The current AI investigation score is {} out of 100, which maps to {}.",
                    scorecard.score, scorecard.label
                ),
                clean_sentence(
                    &scorecard.primary_risk,
                    "MIA's AI score is produced from transaction facts, holder evidence, deployer history, contract-source facts, and any deep-research layer that exists.",
                ),
                "Check the weakest evidence layer first. That is usually the fastest way to understand why the setup is being held back.".to_string(),
            )
        } else {
            (
                "There is no AI score yet for this token.".to_string(),
                "MIA keeps analysis available, but the AI scoring pass stays locked until activity clears the budget gate or a Deep Research report exists.".to_string(),
                "Focus on the evidence layers first, or unlock Deep Research if you want a paid score below the live transaction gate.".to_string(),
            )
        }
    } else if question_lower.contains("next hour") || question_lower.contains("watch") {
        (
            "Watch the next hour for breadth, not just speed.".to_string(),
            "The fastest way this setup improves is if more wallets join without risk concentration spiking. The fastest way it weakens is if flow fades while the same few wallets still dominate.".to_string(),
            "Monitor buy-share, wallet breadth, and whether the builder's pattern keeps repeating.".to_string(),
        )
    } else if question_lower.contains("stay out") || question_lower.contains("risky") {
        (
                "The strongest reason to stay out is concentration risk around an early-stage launch.".to_string(),
                "MIA's free lane is built to protect capital first. If builder memory is mixed or holder concentration is elevated, the token can still move, but the setup is less forgiving.".to_string(),
                "Wait for cleaner participation or keep size small and rules-based.".to_string(),
            )
    } else {
        (
            clean_sentence(
                internal
                    .agent_scorecard
                    .as_ref()
                    .map(|scorecard| scorecard.headline.as_str())
                    .unwrap_or(internal.verdict.headline.as_str()),
                "MIA sees a live setup, but it still needs context.",
            ),
            clean_sentence(
                internal
                    .agent_scorecard
                    .as_ref()
                    .map(|scorecard| scorecard.summary.as_str())
                    .unwrap_or(internal.verdict.summary.as_str()),
                "The token should be judged from flow, builder history, wallets, and model support together.",
            ),
            internal
                .verdict
                .next_actions
                .first()
                .map(|action| {
                    clean_sentence(&action.label, "Stay disciplined and watch the next move.")
                })
                .unwrap_or_else(|| "Stay disciplined and watch the next move.".to_string()),
        )
    };

    if let Some(owner_pct) = owner_share {
        evidence.push(format!(
            "The owner appears inside the live holder map with {:.2}% of supply.",
            owner_pct
        ));
    }

    AskMiaAnswer {
        short_answer,
        why,
        evidence: normalize_ask_mia_evidence(evidence, &internal.verdict.evidence),
        next_move,
    }
}

pub(crate) fn normalize_ask_mia_evidence(primary: Vec<String>, fallback: &[String]) -> Vec<String> {
    let mut merged = primary
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    if merged.len() < 2 {
        merged.extend(
            fallback
                .iter()
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty()),
        );
    }
    merged.truncate(5);
    merged
}

pub(crate) fn clean_sentence(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    let selected = if trimmed.is_empty() {
        fallback.trim()
    } else {
        trimmed
    };
    if selected.ends_with('.') || selected.ends_with('!') || selected.ends_with('?') {
        selected.to_string()
    } else {
        format!("{selected}.")
    }
}

pub(crate) fn parse_json_payload<T: DeserializeOwned>(text: &str) -> Result<T, anyhow::Error> {
    if let Ok(value) = serde_json::from_str::<T>(text) {
        return Ok(value);
    }

    let start = text
        .find('{')
        .ok_or_else(|| anyhow::anyhow!("JSON object start not found"))?;
    let end = text
        .rfind('}')
        .ok_or_else(|| anyhow::anyhow!("JSON object end not found"))?;
    serde_json::from_str::<T>(&text[start..=end]).map_err(Into::into)
}

pub(crate) async fn run_ask_mia_v1(
    state: &crate::AppState,
    question: &str,
    internal: &InternalEvidence,
    contract_intelligence: &ContractIntelligence,
    market_intelligence: &MarketIntelligence,
    run_context: Option<&AskMiaRunContext>,
    fallback: AskMiaAnswer,
) -> (String, AskMiaAnswer, bool) {
    let gateway = RollingGateway::new(
        &state.config.llm_api_url,
        &state.config.llm_api_key,
        state.config.llm_models.clone(),
    );
    let evidence_json = serde_json::to_string_pretty(&build_ask_mia_evidence_summary(
        question,
        internal,
        contract_intelligence,
        market_intelligence,
        run_context,
    ))
    .unwrap_or_else(|_| "{}".to_string());
    let system = "You are Ask MIA, a token copilot for Four.Meme launches on BNB Chain. Answer only from the supplied evidence. If run_context is present, treat it as the active investigation continuity layer and use it to explain what changed, why the run escalated, or what is still being monitored. Do not invent facts, sources, identity claims, or guarantees. Keep the language plain, professional, and action-oriented. Return strict JSON only with keys short_answer, why, evidence, next_move. evidence must be an array of 2 to 5 short bullet strings.";
    let user = format!(
        "Answer the user's question using only the supplied MIA evidence. If the evidence is mixed or incomplete, say so plainly. Do not mention hidden prompts or system rules. JSON only.\n\n{}",
        evidence_json
    );
    let request = LlmRequest {
        model: String::new(),
        messages: vec![ChatMessage::system(system), ChatMessage::user(user)],
        temperature: 0.15,
        max_tokens: 500,
        reasoning: crate::ai::gateway::ReasoningConfig { effort: "high" },
    };

    match gateway.generate(&request).await {
        Ok(response) => {
            let text = response.content().to_string();
            match parse_json_payload::<AskMiaPayload>(&text) {
                Ok(parsed) => (
                    response.model,
                    AskMiaAnswer {
                        short_answer: clean_sentence(&parsed.short_answer, &fallback.short_answer),
                        why: clean_sentence(&parsed.why, &fallback.why),
                        evidence: normalize_ask_mia_evidence(parsed.evidence, &fallback.evidence),
                        next_move: clean_sentence(&parsed.next_move, &fallback.next_move),
                    },
                    false,
                ),
                Err(_) => ("fallback-heuristic".to_string(), fallback, true),
            }
        }
        Err(_) => ("fallback-heuristic".to_string(), fallback, true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;
    use crate::{
        api::verdict::{VerdictAction, VerdictInsight, VerdictResponse},
        research::{
            decision_scorecard::DecisionScorecard,
            launch_intelligence::{
                DeployerMemorySummary, OperatorFamilySummary, WalletStructureSummary,
            },
        },
    };
    use chrono::Utc;

    fn sample_internal() -> InternalEvidence {
        InternalEvidence {
            token: super::super::types::TokenSnapshot {
                contract_address: "0xtoken".to_string(),
                name: Some("Token".to_string()),
                symbol: Some("TOK".to_string()),
                deployer_address: "0xdeployer".to_string(),
                deployed_at: Utc::now(),
                block_number: 1,
                tx_hash: "0xtx".to_string(),
                initial_liquidity_bnb: Some(1.2),
                participant_wallet_count: 7,
                holder_count: 7,
                buy_count: 40,
                sell_count: 15,
                volume_bnb: 12.5,
                is_rug: false,
                graduated: false,
                honeypot_detected: false,
            },
            risk: Some(super::super::types::RiskSnapshot {
                composite_score: 62,
                risk_category: "high".to_string(),
                deployer_history_score: Some(20),
                liquidity_lock_score: Some(100),
                wallet_concentration_score: Some(80),
                buy_sell_velocity_score: Some(35),
                contract_audit_score: Some(0),
                social_authenticity_score: Some(50),
                volume_consistency_score: Some(45),
                computed_at: Utc::now(),
            }),
            agent_scorecard: Some(super::super::types::AgentScorecard {
                score: 61,
                label: "WATCH".to_string(),
                confidence_label: "medium".to_string(),
                headline: "WATCH".to_string(),
                summary: "summary".to_string(),
                primary_reason: "reason".to_string(),
                primary_risk: "risk".to_string(),
                supporting_points: vec!["point".to_string()],
            }),
            verdict: VerdictResponse {
                token_address: "0xtoken".to_string(),
                label: "WATCH".to_string(),
                tone: "warn".to_string(),
                score: 50,
                confidence_label: "MEDIUM".to_string(),
                headline: "Legacy headline".to_string(),
                summary: "Legacy summary".to_string(),
                evidence: vec!["legacy evidence".to_string()],
                concerns: vec!["legacy concern".to_string()],
                narrative_reality: VerdictInsight {
                    label: "Narrative".to_string(),
                    tone: "warn".to_string(),
                    detail: "detail".to_string(),
                },
                whale_intent: VerdictInsight {
                    label: "Whales".to_string(),
                    tone: "warn".to_string(),
                    detail: "detail".to_string(),
                },
                deployer_dna: VerdictInsight {
                    label: "DNA".to_string(),
                    tone: "warn".to_string(),
                    detail: "detail".to_string(),
                },
                next_actions: vec![VerdictAction {
                    label: "Do something".to_string(),
                    href: "/mia".to_string(),
                }],
            },
            narrative_cache: Some(super::super::types::NarrativeCacheSnapshot {
                narrative_text: "old narrative".to_string(),
                risk_interpretation: Some("old risk".to_string()),
                consensus_status: "agreed".to_string(),
                confidence: "medium".to_string(),
                generated_at: Utc::now(),
                expires_at: Utc::now(),
            }),
            deployer: Some(super::super::types::DeployerSnapshot {
                address: "0xdeployer".to_string(),
                total_tokens_deployed: 4,
                rug_count: 1,
                graduated_count: 1,
                honeypot_detected: false,
                trust_grade: "B".to_string(),
                trust_label: "Neutral".to_string(),
                first_seen_at: Some(Utc::now()),
                last_seen_at: Some(Utc::now()),
            }),
            deployer_recent_tokens: vec![super::super::types::DeployerTokenSnapshot {
                contract_address: "0xprev".to_string(),
                name: Some("Prev".to_string()),
                symbol: Some("PREV".to_string()),
                deployed_at: Utc::now(),
                buy_count: 5,
                sell_count: 3,
                volume_bnb: 2.0,
                composite_score: Some(70),
                risk_category: Some("high".to_string()),
            }],
            recent_transactions: vec![super::super::types::TransactionSnapshot {
                wallet_address: "0xwallet".to_string(),
                tx_hash: "0xhash".to_string(),
                tx_type: "buy".to_string(),
                amount_bnb: 1.1,
                block_number: 2,
                created_at: Utc::now(),
            }],
            whale_activity_24h: super::super::types::WhaleActivitySnapshot {
                watch_alerts: 1,
                critical_alerts: 0,
                latest_levels: vec!["watch".to_string()],
            },
            alpha_context: Some(super::super::types::AlphaContextSnapshot {
                rank: 1,
                alpha_score: 88.0,
                rationale: "legacy alpha".to_string(),
                window_end: Utc::now(),
            }),
            wallet_structure: WalletStructureSummary {
                summary: "wallet summary".to_string(),
                evidence: vec!["wallet evidence".to_string()],
                active_wallet_count: 5,
                participant_wallet_count: 7,
                holder_count: 7,
                probable_cluster_wallets: 2,
                potential_cluster_wallets: 1,
                repeated_wallet_count: 1,
                top_flow_wallets: vec!["0xwallet | net flow 1.1 BNB | 1 tx".to_string()],
            },
            deployer_memory: Some(DeployerMemorySummary {
                summary: "memory summary".to_string(),
                evidence: vec!["memory evidence".to_string()],
                trust_grade: "B".to_string(),
                trust_label: "Neutral".to_string(),
                total_launches: 4,
                rug_count: 1,
                graduated_count: 1,
                honeypot_history: false,
                first_seen_at: Some(Utc::now()),
                last_seen_at: Some(Utc::now()),
                recent_launches: vec![],
            }),
            operator_family: OperatorFamilySummary {
                confidence: "medium".to_string(),
                summary: "operator summary".to_string(),
                evidence: vec!["operator evidence".to_string()],
                safety_score: 55,
                signal_score: 45,
                related_launch_count: 2,
                related_deployer_count: 1,
                repeated_wallet_count: 1,
                seller_to_new_builder_count: 1,
                seller_reentry_wallet_count: 1,
                probable_cluster_wallets: 2,
                potential_cluster_wallets: 1,
                repeated_wallets: vec!["0xwallet".to_string()],
                migrated_wallets: vec!["0xwallet".to_string()],
                related_launches: vec![],
            },
            decision_scorecard: DecisionScorecard {
                decision_score: 50,
                verdict: "WATCH".to_string(),
                confidence_label: "MEDIUM".to_string(),
                primary_reason: "legacy reason".to_string(),
                primary_risk: "legacy risk".to_string(),
                subscores: vec![],
            },
        }
    }

    fn sample_contract_intelligence() -> ContractIntelligence {
        ContractIntelligence {
            provider: "moralis".to_string(),
            available: true,
            source_verified: true,
            contract_name: Some("Token".to_string()),
            compiler_version: Some("v0.8".to_string()),
            optimization_used: Some(true),
            optimization_runs: Some(200),
            proxy: Some(false),
            implementation: None,
            token_type: Some("ERC20".to_string()),
            total_supply: Some("1000000".to_string()),
            total_supply_raw: Some("1000000000000000000000000".to_string()),
            decimals: Some(18),
            indexed_holder_count: Some(123),
            holder_count: Some(123),
            description: Some("desc".to_string()),
            website: Some("https://x.test".to_string()),
            twitter: Some("https://twitter.com/x".to_string()),
            telegram: None,
            discord: None,
            owner_holding_pct: Some(12.5),
            owner_in_top_holders: true,
            holder_supply: None,
            holder_change: None,
            holder_distribution: None,
            holders_by_acquisition: None,
            top_holders: vec![super::super::types::HolderSnapshot {
                address: "0xholder".to_string(),
                quantity: "100".to_string(),
                quantity_raw: "100".to_string(),
                ownership_pct: Some(10.0),
                is_owner: false,
                address_type: None,
                owner_label: Some("label".to_string()),
                entity: Some("entity".to_string()),
                is_contract: Some(false),
            }],
            notes: vec!["note".to_string()],
        }
    }

    fn sample_deep_research_state() -> InvestigationDeepResearchState {
        InvestigationDeepResearchState {
            report_cached: false,
            report_generated_at: None,
            auto_threshold_met: false,
            auto_threshold_tx_count: 500,
            auto_requested: false,
            ai_score_enabled: true,
            ai_score_gate_tx_count: 50,
            score_enriched: false,
        }
    }

    #[test]
    fn extracts_json_payload_from_wrapped_text() {
        let payload: AnalysisPayload = parse_json_payload(
            "Here you go\n{\"score\":67,\"label\":\"WATCH\",\"verdict\":\"WATCH\",\"conviction\":\"medium\",\"confidence\":\"medium\",\"executive_summary\":\"x\",\"primary_reason\":\"y\",\"primary_risk\":\"z\",\"supporting_points\":[\"p\"],\"thesis\":[\"a\"],\"risks\":[\"b\"],\"next_actions\":[\"c\"]}",
        )
        .unwrap();
        assert_eq!(payload.verdict, "WATCH");
        assert_eq!(payload.score, 67);
    }

    #[test]
    fn normalizes_numeric_confidence_payloads() {
        let payload: AnalysisPayload = parse_json_payload(
            "{\"score\":72,\"label\":\"WATCH\",\"verdict\":\"WATCH\",\"conviction\":85,\"confidence\":0.82,\"executive_summary\":\"x\",\"primary_reason\":\"y\",\"primary_risk\":\"z\",\"supporting_points\":[\"p\"],\"thesis\":[\"a\"],\"risks\":[\"b\"],\"next_actions\":[\"c\"]}",
        )
        .unwrap();

        assert_eq!(normalize_confidence_value(&payload.conviction), "high");
        assert_eq!(normalize_confidence_value(&payload.confidence), "high");
    }

    #[test]
    fn rejects_empty_ask_mia_question() {
        let result = validate_ask_mia_question("   ");
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn normalizes_ask_mia_json_payload() {
        let payload: AskMiaPayload = parse_json_payload(
            "Answer\n{\"short_answer\":\"Risk is elevated\",\"why\":\"Flow is narrow\",\"evidence\":[\"One\",\"Two\"],\"next_move\":\"Watch it\"}",
        )
        .unwrap();
        assert_eq!(payload.short_answer, "Risk is elevated");
        assert_eq!(payload.evidence.len(), 2);
    }

    #[test]
    fn analysis_payload_drops_legacy_noise_layers() {
        let payload = build_analysis_evidence_payload(
            &sample_internal(),
            &sample_contract_intelligence(),
            None,
        );

        assert!(payload.get("token_facts").is_some());
        assert!(payload.get("wallet_observations").is_some());
        assert!(payload.get("operator_pattern_signals").is_some());
        assert!(payload.get("risk").is_none());
        assert!(payload.get("market_intelligence").is_none());
        assert!(payload.get("alpha_context").is_none());
        assert!(payload.get("narrative_cache").is_none());
        assert_eq!(
            payload["token_facts"]["participant_wallet_count"].as_i64(),
            Some(7)
        );
        assert_eq!(
            payload["token_facts"]["indexed_holder_count"].as_u64(),
            Some(123)
        );
    }

    #[test]
    fn ask_mia_payload_uses_truth_first_field_names() {
        let payload = build_ask_mia_evidence_summary(
            "what changed?",
            &sample_internal(),
            &sample_contract_intelligence(),
            &MarketIntelligence::default(),
            None,
        );

        assert_eq!(
            payload["token"]["participant_wallet_count"].as_i64(),
            Some(7)
        );
        assert_eq!(payload.get("risk"), None);
        assert_eq!(payload.get("ml"), None);
        assert_eq!(payload.get("narrative_cache"), None);
        assert_eq!(
            payload["wallets"]["cluster_signals"]["probable_cluster_wallets"].as_i64(),
            Some(2)
        );
    }

    #[test]
    fn tripwires_explain_upgrade_and_risk_conditions() {
        let tripwires = build_investigation_tripwires(
            &sample_internal(),
            &sample_contract_intelligence(),
            &InvestigationAnalysis {
                provider: "gpt-5.4".to_string(),
                score: Some(61),
                label: Some("WATCH".to_string()),
                verdict: "WATCH".to_string(),
                conviction: "medium".to_string(),
                confidence: "medium".to_string(),
                executive_summary: "summary".to_string(),
                primary_reason: "reason".to_string(),
                primary_risk: "risk".to_string(),
                supporting_points: vec![],
                thesis: vec![],
                risks: vec![],
                next_actions: vec![],
                raw: None,
            },
            &sample_deep_research_state(),
        );

        assert_eq!(tripwires.headline, "What would make MIA change its mind?");
        assert!(tripwires.watching_for.contains("participation") || tripwires.watching_for.contains("wallet breadth"));
        assert!(tripwires.upgrade_trigger.contains("Upgrade"));
        assert!(tripwires.risk_trigger.contains("Raise risk"));
        assert!(tripwires.deep_research_trigger.contains("Deep research"));
        assert!(tripwires.invalidation_trigger.contains("Invalidate"));
    }

    #[test]
    fn tripwires_explain_score_gate_when_ai_score_is_locked() {
        let mut deep_research = sample_deep_research_state();
        deep_research.ai_score_enabled = false;
        let analysis = InvestigationAnalysis {
            provider: "budget-gated-heuristic".to_string(),
            score: None,
            label: None,
            verdict: "WATCH".to_string(),
            conviction: "medium".to_string(),
            confidence: "medium".to_string(),
            executive_summary: "summary".to_string(),
            primary_reason: "reason".to_string(),
            primary_risk: "risk".to_string(),
            supporting_points: vec![],
            thesis: vec![],
            risks: vec![],
            next_actions: vec![],
            raw: None,
        };

        let tripwires = build_investigation_tripwires(
            &sample_internal(),
            &sample_contract_intelligence(),
            &analysis,
            &deep_research,
        );

        assert!(tripwires.watching_for.contains("50"));
        assert!(tripwires.upgrade_trigger.contains("50"));
        assert!(tripwires.invalidation_trigger.contains("50"));
    }
}
