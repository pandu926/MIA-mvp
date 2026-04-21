use serde::Serialize;
use serde_json::{json, Value};

use crate::api::investigation::{ContractIntelligence, InternalEvidence, MarketIntelligence};

pub const TOOL_GET_TOKEN_OVERVIEW: &str = "get_token_overview";
pub const TOOL_GET_RISK_SNAPSHOT: &str = "get_risk_snapshot";
pub const TOOL_GET_AGENT_SCORECARD: &str = "get_agent_scorecard";
pub const TOOL_GET_MARKET_STRUCTURE: &str = "get_market_structure";
pub const TOOL_GET_WALLET_STRUCTURE: &str = "get_wallet_structure";
pub const TOOL_GET_OPERATOR_FAMILY: &str = "get_operator_family";
pub const TOOL_GET_DEPLOYER_MEMORY: &str = "get_deployer_memory";
pub const TOOL_GET_WHALE_AND_FLOW_SIGNALS: &str = "get_whale_and_flow_signals";
pub const TOOL_GET_ML_CONTEXT: &str = "get_ml_context";
pub const TOOL_GET_NARRATIVE_CONTEXT: &str = "get_narrative_context";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AskMiaToolName {
    TokenOverview,
    RiskSnapshot,
    AgentScorecard,
    MarketStructure,
    WalletStructure,
    OperatorFamily,
    DeployerMemory,
    WhaleAndFlowSignals,
    MlContext,
    NarrativeContext,
}

impl AskMiaToolName {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TokenOverview => TOOL_GET_TOKEN_OVERVIEW,
            Self::RiskSnapshot => TOOL_GET_RISK_SNAPSHOT,
            Self::AgentScorecard => TOOL_GET_AGENT_SCORECARD,
            Self::MarketStructure => TOOL_GET_MARKET_STRUCTURE,
            Self::WalletStructure => TOOL_GET_WALLET_STRUCTURE,
            Self::OperatorFamily => TOOL_GET_OPERATOR_FAMILY,
            Self::DeployerMemory => TOOL_GET_DEPLOYER_MEMORY,
            Self::WhaleAndFlowSignals => TOOL_GET_WHALE_AND_FLOW_SIGNALS,
            Self::MlContext => TOOL_GET_ML_CONTEXT,
            Self::NarrativeContext => TOOL_GET_NARRATIVE_CONTEXT,
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            TOOL_GET_TOKEN_OVERVIEW => Some(Self::TokenOverview),
            TOOL_GET_RISK_SNAPSHOT => Some(Self::RiskSnapshot),
            TOOL_GET_AGENT_SCORECARD => Some(Self::AgentScorecard),
            TOOL_GET_MARKET_STRUCTURE => Some(Self::MarketStructure),
            TOOL_GET_WALLET_STRUCTURE => Some(Self::WalletStructure),
            TOOL_GET_OPERATOR_FAMILY => Some(Self::OperatorFamily),
            TOOL_GET_DEPLOYER_MEMORY => Some(Self::DeployerMemory),
            TOOL_GET_WHALE_AND_FLOW_SIGNALS => Some(Self::WhaleAndFlowSignals),
            TOOL_GET_ML_CONTEXT => Some(Self::MlContext),
            TOOL_GET_NARRATIVE_CONTEXT => Some(Self::NarrativeContext),
            _ => None,
        }
    }
}

pub struct AskMiaToolContext<'a> {
    pub internal: &'a InternalEvidence,
    pub contract_intelligence: &'a ContractIntelligence,
    pub market_intelligence: &'a MarketIntelligence,
}

pub fn dispatch_tool(tool: AskMiaToolName, context: &AskMiaToolContext<'_>) -> Value {
    match tool {
        AskMiaToolName::TokenOverview => build_token_overview(context),
        AskMiaToolName::RiskSnapshot => build_risk_snapshot(context),
        AskMiaToolName::AgentScorecard => build_agent_scorecard(context),
        AskMiaToolName::MarketStructure => build_market_structure(context),
        AskMiaToolName::WalletStructure => build_wallet_structure(context),
        AskMiaToolName::OperatorFamily => build_operator_family(context),
        AskMiaToolName::DeployerMemory => build_deployer_memory(context),
        AskMiaToolName::WhaleAndFlowSignals => build_whale_and_flow_signals(context),
        AskMiaToolName::MlContext => build_ml_context(context),
        AskMiaToolName::NarrativeContext => build_narrative_context(context),
    }
}

pub fn tool_schema() -> Vec<Value> {
    vec![
        function_tool(
            TOOL_GET_TOKEN_OVERVIEW,
            "Get the token identity, current read anchor, and high-level launch state.",
        ),
        function_tool(
            TOOL_GET_RISK_SNAPSHOT,
            "Get the current risk breakdown, including concentration and behavior scores.",
        ),
        function_tool(
            TOOL_GET_AGENT_SCORECARD,
            "Get the AI-generated investigation score, current read, and dominant reason/risk summary.",
        ),
        function_tool(
            TOOL_GET_MARKET_STRUCTURE,
            "Get live market structure such as buys, sells, volume, and flow balance.",
        ),
        function_tool(
            TOOL_GET_WALLET_STRUCTURE,
            "Get wallet structure, including concentration, clusters, active wallets, and flow leaders.",
        ),
        function_tool(
            TOOL_GET_OPERATOR_FAMILY,
            "Get likely coordinated operator-family pattern signals across repeated wallets, migrations, and related launches.",
        ),
        function_tool(
            TOOL_GET_DEPLOYER_MEMORY,
            "Get historical builder memory including trust grade, launch count, rugs, and graduates.",
        ),
        function_tool(
            TOOL_GET_WHALE_AND_FLOW_SIGNALS,
            "Get whale alert and high-impact flow context for this launch.",
        ),
        function_tool(
            TOOL_GET_ML_CONTEXT,
            "Get MIA's internal ranking and machine-learning context for this token.",
        ),
        function_tool(
            TOOL_GET_NARRATIVE_CONTEXT,
            "Get narrative and public-signal context, including active topic and risk flags.",
        ),
    ]
}

fn function_tool(name: &str, description: &str) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": name,
            "description": description,
            "parameters": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        }
    })
}

fn build_token_overview(context: &AskMiaToolContext<'_>) -> Value {
    json!({
        "contract_address": context.internal.token.contract_address,
        "name": context.internal.token.name,
        "symbol": context.internal.token.symbol,
        "deployer_address": context.internal.token.deployer_address,
        "deployed_at": context.internal.token.deployed_at,
        "holder_count": context.internal.token.holder_count,
        "buy_count": context.internal.token.buy_count,
        "sell_count": context.internal.token.sell_count,
        "volume_bnb": context.internal.token.volume_bnb,
        "graduated": context.internal.token.graduated,
        "honeypot_detected": context.internal.token.honeypot_detected,
        "current_read": {
            "label": context
                .internal
                .agent_scorecard
                .as_ref()
                .map(|scorecard| scorecard.label.as_str())
                .unwrap_or(context.internal.verdict.label.as_str()),
            "confidence_label": context
                .internal
                .agent_scorecard
                .as_ref()
                .map(|scorecard| scorecard.confidence_label.as_str())
                .unwrap_or(context.internal.verdict.confidence_label.as_str()),
            "headline": context
                .internal
                .agent_scorecard
                .as_ref()
                .map(|scorecard| scorecard.headline.as_str())
                .unwrap_or(context.internal.verdict.headline.as_str()),
            "summary": context
                .internal
                .agent_scorecard
                .as_ref()
                .map(|scorecard| scorecard.summary.as_str())
                .unwrap_or(context.internal.verdict.summary.as_str()),
        }
    })
}

fn build_risk_snapshot(context: &AskMiaToolContext<'_>) -> Value {
    match &context.internal.risk {
        Some(risk) => json!({
            "composite_score": risk.composite_score,
            "risk_category": risk.risk_category,
            "deployer_history_score": risk.deployer_history_score,
            "wallet_concentration_score": risk.wallet_concentration_score,
            "buy_sell_velocity_score": risk.buy_sell_velocity_score,
            "social_authenticity_score": risk.social_authenticity_score,
            "volume_consistency_score": risk.volume_consistency_score,
        }),
        None => json!({"available": false, "note": "No live risk snapshot is attached."}),
    }
}

fn build_market_structure(context: &AskMiaToolContext<'_>) -> Value {
    let total_trades = context.internal.token.buy_count + context.internal.token.sell_count;
    let buy_share = if total_trades > 0 {
        (context.internal.token.buy_count as f64 / total_trades as f64) * 100.0
    } else {
        50.0
    };

    json!({
        "buy_count": context.internal.token.buy_count,
        "sell_count": context.internal.token.sell_count,
        "volume_bnb": context.internal.token.volume_bnb,
        "buy_share_pct": buy_share,
        "market_structure_label": if buy_share >= 55.0 {
            "buyers_lead"
        } else if buy_share <= 45.0 {
            "sellers_lead"
        } else {
            "balanced"
        },
        "provider": context.market_intelligence.provider,
        "active_event": context.market_intelligence.active_event,
    })
}

fn build_agent_scorecard(context: &AskMiaToolContext<'_>) -> Value {
    if let Some(scorecard) = &context.internal.agent_scorecard {
        return json!({
            "score": scorecard.score,
            "label": scorecard.label,
            "confidence_label": scorecard.confidence_label,
            "headline": scorecard.headline,
            "summary": scorecard.summary,
            "primary_reason": scorecard.primary_reason,
            "primary_risk": scorecard.primary_risk,
            "supporting_points": scorecard.supporting_points,
        });
    }

    json!({
        "score": context.internal.decision_scorecard.decision_score,
        "label": context.internal.decision_scorecard.verdict,
        "confidence_label": context.internal.decision_scorecard.confidence_label,
        "headline": context.internal.verdict.headline,
        "summary": context.internal.verdict.summary,
        "primary_reason": context.internal.decision_scorecard.primary_reason,
        "primary_risk": context.internal.decision_scorecard.primary_risk,
        "supporting_points": context.internal.verdict.evidence,
        "fallback": true,
    })
}

fn build_wallet_structure(context: &AskMiaToolContext<'_>) -> Value {
    json!({
        "summary": context.internal.wallet_structure.summary,
        "participant_wallet_count": context.internal.wallet_structure.participant_wallet_count,
        "active_wallet_count": context.internal.wallet_structure.active_wallet_count,
        "probable_cluster_wallets": context.internal.wallet_structure.probable_cluster_wallets,
        "potential_cluster_wallets": context.internal.wallet_structure.potential_cluster_wallets,
        "repeated_wallet_count": context.internal.wallet_structure.repeated_wallet_count,
        "top_flow_wallets": context.internal.wallet_structure.top_flow_wallets,
        "indexed_holder_count": context.contract_intelligence.indexed_holder_count.or(context.contract_intelligence.holder_count),
        "owner_holding_pct": context.contract_intelligence.owner_holding_pct,
        "owner_in_top_holders": context.contract_intelligence.owner_in_top_holders,
        "wallet_concentration_score": context.internal.risk.as_ref().and_then(|risk| risk.wallet_concentration_score),
        "evidence": context.internal.wallet_structure.evidence,
        "provider_notes": context.contract_intelligence.notes,
    })
}

fn build_operator_family(context: &AskMiaToolContext<'_>) -> Value {
    json!({
        "confidence": context.internal.operator_family.confidence,
        "summary": context.internal.operator_family.summary,
        "signal_score": context.internal.operator_family.signal_score,
        "safety_score": context.internal.operator_family.safety_score,
        "related_launch_count": context.internal.operator_family.related_launch_count,
        "related_deployer_count": context.internal.operator_family.related_deployer_count,
        "repeated_wallet_count": context.internal.operator_family.repeated_wallet_count,
        "seller_to_new_builder_count": context.internal.operator_family.seller_to_new_builder_count,
        "seller_reentry_wallet_count": context.internal.operator_family.seller_reentry_wallet_count,
        "repeated_wallets": context.internal.operator_family.repeated_wallets,
        "migrated_wallets": context.internal.operator_family.migrated_wallets,
        "related_launches": context.internal.operator_family.related_launches,
        "evidence": context.internal.operator_family.evidence,
    })
}

fn build_deployer_memory(context: &AskMiaToolContext<'_>) -> Value {
    match &context.internal.deployer_memory {
        Some(memory) => {
            json!({
                "summary": memory.summary,
                "trust_grade": memory.trust_grade,
                "trust_label": memory.trust_label,
                "total_launches": memory.total_launches,
                "rug_count": memory.rug_count,
                "graduated_count": memory.graduated_count,
                "honeypot_history": memory.honeypot_history,
                "first_seen_at": memory.first_seen_at,
                "last_seen_at": memory.last_seen_at,
                "recent_launches": memory.recent_launches,
                "evidence": memory.evidence,
            })
        }
        None => json!({"available": false, "note": "No deployer memory is attached."}),
    }
}

fn build_whale_and_flow_signals(context: &AskMiaToolContext<'_>) -> Value {
    let recent_transactions = context
        .internal
        .recent_transactions
        .iter()
        .take(6)
        .map(|tx| {
            json!({
                "wallet_address": tx.wallet_address,
                "tx_type": tx.tx_type,
                "amount_bnb": tx.amount_bnb,
                "created_at": tx.created_at,
            })
        })
        .collect::<Vec<_>>();

    json!({
        "watch_alerts_24h": context.internal.whale_activity_24h.watch_alerts,
        "critical_alerts_24h": context.internal.whale_activity_24h.critical_alerts,
        "latest_levels": context.internal.whale_activity_24h.latest_levels,
        "recent_transactions": recent_transactions,
    })
}

fn build_ml_context(context: &AskMiaToolContext<'_>) -> Value {
    match &context.internal.alpha_context {
        Some(alpha) => json!({
            "rank": alpha.rank,
            "alpha_score": alpha.alpha_score,
            "rationale": alpha.rationale,
            "window_end": alpha.window_end,
        }),
        None => json!({"available": false, "note": "No live alpha context is attached."}),
    }
}

fn build_narrative_context(context: &AskMiaToolContext<'_>) -> Value {
    json!({
        "provider": context.market_intelligence.provider,
        "available": context.market_intelligence.available,
        "active_event": context.market_intelligence.active_event,
        "narrative_alignment": context.market_intelligence.narrative_alignment,
        "excitement_score": context.market_intelligence.excitement_score,
        "risk_flags": context.market_intelligence.risk_flags,
        "x_summary": context.market_intelligence.x_summary,
        "web_summary": context.market_intelligence.web_summary,
        "sources": context.market_intelligence.sources,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    use crate::api::{
        investigation::{
            AgentScorecard, AlphaContextSnapshot, DeployerSnapshot, DeployerTokenSnapshot,
            HolderSnapshot, InvestigationSource, RiskSnapshot, TokenSnapshot, TransactionSnapshot,
            WhaleActivitySnapshot,
        },
        verdict::{VerdictAction, VerdictInsight, VerdictResponse},
    };
    use crate::research::{
        decision_scorecard::{DecisionScorecard, DecisionSubscore},
        launch_intelligence::{
            DeployerMemorySummary, OperatorFamilySummary, WalletStructureSummary,
        },
    };

    fn sample_context() -> AskMiaToolContext<'static> {
        let internal = Box::leak(Box::new(InternalEvidence {
            token: TokenSnapshot {
                contract_address: "0xabc".to_string(),
                name: Some("Token".to_string()),
                symbol: Some("TOK".to_string()),
                deployer_address: "0xdeployer".to_string(),
                deployed_at: Utc::now(),
                block_number: 1,
                tx_hash: "0xtx".to_string(),
                initial_liquidity_bnb: Some(1.2),
                participant_wallet_count: 123,
                holder_count: 123,
                buy_count: 80,
                sell_count: 20,
                volume_bnb: 42.0,
                is_rug: false,
                graduated: false,
                honeypot_detected: false,
            },
            risk: Some(RiskSnapshot {
                composite_score: 61,
                risk_category: "high".to_string(),
                deployer_history_score: Some(50),
                liquidity_lock_score: Some(40),
                wallet_concentration_score: Some(78),
                buy_sell_velocity_score: Some(66),
                contract_audit_score: Some(10),
                social_authenticity_score: Some(44),
                volume_consistency_score: Some(58),
                computed_at: Utc::now(),
            }),
            agent_scorecard: Some(AgentScorecard {
                score: 64,
                label: "WATCH".to_string(),
                confidence_label: "medium".to_string(),
                headline: "WATCH · medium confidence".to_string(),
                summary: "Agent synthesis sees active flow, but concentration still matters."
                    .to_string(),
                primary_reason:
                    "Buy-side still leads and the launch is active enough to stay on the board."
                        .to_string(),
                primary_risk:
                    "Repeated wallets and concentration still reduce trust in the structure."
                        .to_string(),
                supporting_points: vec![
                    "Buy pressure is still positive.".to_string(),
                    "Builder history is mixed.".to_string(),
                ],
            }),
            verdict: VerdictResponse {
                token_address: "0xabc".to_string(),
                label: "WATCH".to_string(),
                tone: "warn".to_string(),
                score: 58,
                confidence_label: "MEDIUM".to_string(),
                headline: "Watch this launch closely.".to_string(),
                summary: "There is activity, but concentration still matters.".to_string(),
                evidence: vec!["Buy pressure is positive.".to_string()],
                concerns: vec!["Concentration remains elevated.".to_string()],
                narrative_reality: VerdictInsight {
                    label: "Mixed".to_string(),
                    tone: "warn".to_string(),
                    detail: "Narrative is present but not clean.".to_string(),
                },
                whale_intent: VerdictInsight {
                    label: "Watch".to_string(),
                    tone: "warn".to_string(),
                    detail: "Whale activity is not decisive yet.".to_string(),
                },
                deployer_dna: VerdictInsight {
                    label: "Neutral".to_string(),
                    tone: "neutral".to_string(),
                    detail: "Builder memory is mixed.".to_string(),
                },
                next_actions: vec![VerdictAction {
                    label: "Wait for confirmation".to_string(),
                    href: "/mia".to_string(),
                }],
            },
            narrative_cache: None,
            deployer: Some(DeployerSnapshot {
                address: "0xdeployer".to_string(),
                total_tokens_deployed: 4,
                rug_count: 1,
                graduated_count: 1,
                honeypot_detected: false,
                trust_grade: "neutral".to_string(),
                trust_label: "Neutral".to_string(),
                first_seen_at: Some(Utc::now()),
                last_seen_at: Some(Utc::now()),
            }),
            deployer_recent_tokens: vec![DeployerTokenSnapshot {
                contract_address: "0xprior".to_string(),
                name: Some("Prior".to_string()),
                symbol: Some("PR".to_string()),
                deployed_at: Utc::now(),
                buy_count: 12,
                sell_count: 5,
                volume_bnb: 9.5,
                composite_score: Some(70),
                risk_category: Some("high".to_string()),
            }],
            recent_transactions: vec![TransactionSnapshot {
                wallet_address: "0xwallet".to_string(),
                tx_hash: "0xrecent".to_string(),
                tx_type: "buy".to_string(),
                amount_bnb: 3.0,
                block_number: 10,
                created_at: Utc::now(),
            }],
            whale_activity_24h: WhaleActivitySnapshot {
                watch_alerts: 2,
                critical_alerts: 1,
                latest_levels: vec!["watch".to_string(), "critical".to_string()],
            },
            alpha_context: Some(AlphaContextSnapshot {
                rank: 12,
                alpha_score: 0.72,
                rationale: "Strong relative participation.".to_string(),
                window_end: Utc::now(),
            }),
            wallet_structure: WalletStructureSummary {
                summary:
                    "Wallet structure shows moderate concentration with some repeat participation."
                        .to_string(),
                evidence: vec!["Two wallets dominate early flow.".to_string()],
                active_wallet_count: 18,
                participant_wallet_count: 123,
                holder_count: 123,
                probable_cluster_wallets: 2,
                potential_cluster_wallets: 3,
                repeated_wallet_count: 2,
                top_flow_wallets: vec!["0xwallet".to_string()],
            },
            deployer_memory: Some(DeployerMemorySummary {
                summary: "Builder has mixed history with one prior rug.".to_string(),
                evidence: vec!["Four launches tracked.".to_string()],
                trust_grade: "C".to_string(),
                trust_label: "Mixed".to_string(),
                total_launches: 4,
                rug_count: 1,
                graduated_count: 1,
                honeypot_history: false,
                first_seen_at: Some(Utc::now()),
                last_seen_at: Some(Utc::now()),
                recent_launches: vec![crate::research::launch_intelligence::DeployerLaunchRef {
                    contract_address: "0xprior".to_string(),
                    symbol: Some("PR".to_string()),
                    name: Some("Prior".to_string()),
                    is_rug: true,
                    graduated: false,
                    deployed_at: Utc::now(),
                    buy_count: 12,
                    sell_count: 5,
                    volume_bnb: 9.5,
                }],
            }),
            operator_family: OperatorFamilySummary {
                confidence: "medium".to_string(),
                summary: "MIA sees a medium-confidence likely coordinated operator-family pattern."
                    .to_string(),
                evidence: vec!["Two repeated wallets connect multiple launches.".to_string()],
                safety_score: 41,
                signal_score: 59,
                related_launch_count: 2,
                related_deployer_count: 1,
                repeated_wallet_count: 2,
                seller_to_new_builder_count: 0,
                seller_reentry_wallet_count: 1,
                probable_cluster_wallets: 2,
                potential_cluster_wallets: 3,
                repeated_wallets: vec!["0xwallet".to_string()],
                migrated_wallets: vec!["0xmigrated".to_string()],
                related_launches: Vec::new(),
            },
            decision_scorecard: DecisionScorecard {
                decision_score: 49,
                verdict: "WATCH".to_string(),
                confidence_label: "MEDIUM".to_string(),
                primary_reason: "Market Structure: Buy-side still leads.".to_string(),
                primary_risk: "Operator Pattern: repeated wallets connect multiple launches."
                    .to_string(),
                subscores: vec![
                    DecisionSubscore {
                        id: "market_structure".to_string(),
                        label: "Market Structure".to_string(),
                        score: 67,
                        weight_pct: 24,
                        summary: "Buy-side still leads.".to_string(),
                    },
                    DecisionSubscore {
                        id: "operator_family".to_string(),
                        label: "Operator Pattern".to_string(),
                        score: 41,
                        weight_pct: 22,
                        summary: "Repeated wallets connect multiple launches.".to_string(),
                    },
                ],
            },
        }));

        let contract_intelligence = Box::leak(Box::new(ContractIntelligence {
            provider: "holder-provider".to_string(),
            available: true,
            source_verified: false,
            contract_name: None,
            compiler_version: None,
            optimization_used: None,
            optimization_runs: None,
            proxy: None,
            implementation: None,
            token_type: None,
            total_supply: None,
            total_supply_raw: None,
            decimals: None,
            indexed_holder_count: Some(456),
            holder_count: Some(456),
            description: None,
            website: None,
            twitter: None,
            telegram: None,
            discord: None,
            owner_holding_pct: Some(17.5),
            owner_in_top_holders: true,
            holder_supply: None,
            holder_change: None,
            holder_distribution: None,
            holders_by_acquisition: None,
            top_holders: vec![HolderSnapshot {
                address: "0xowner".to_string(),
                quantity: "1000".to_string(),
                quantity_raw: "1000".to_string(),
                ownership_pct: Some(17.5),
                is_owner: true,
                address_type: Some("wallet".to_string()),
                owner_label: Some("creator".to_string()),
                entity: None,
                is_contract: Some(false),
            }],
            notes: vec!["owner visible in holder set".to_string()],
        }));

        let market_intelligence = Box::leak(Box::new(MarketIntelligence {
            provider: "mia-market".to_string(),
            available: true,
            x_summary: Some("Hype exists but is mixed.".to_string()),
            web_summary: Some("No strong project footprint.".to_string()),
            active_event: Some("launch momentum".to_string()),
            narrative_alignment: Some("mixed".to_string()),
            excitement_score: Some(62),
            risk_flags: vec!["concentration".to_string()],
            sources: vec![InvestigationSource {
                title: "Source".to_string(),
                url: "https://example.com".to_string(),
                source: "example".to_string(),
            }],
            raw_summary: None,
            notes: vec!["fallback avoided".to_string()],
        }));

        AskMiaToolContext {
            internal,
            contract_intelligence,
            market_intelligence,
        }
    }

    #[test]
    fn tool_schema_contains_all_expected_tools() {
        let names = tool_schema()
            .into_iter()
            .filter_map(|entry| {
                entry
                    .get("function")
                    .and_then(|value| value.get("name"))
                    .and_then(|value| value.as_str())
                    .map(ToString::to_string)
            })
            .collect::<Vec<_>>();

        assert_eq!(names.len(), 10);
        assert!(names.contains(&TOOL_GET_TOKEN_OVERVIEW.to_string()));
        assert!(names.contains(&TOOL_GET_NARRATIVE_CONTEXT.to_string()));
        assert!(names.contains(&TOOL_GET_AGENT_SCORECARD.to_string()));
        assert!(names.contains(&TOOL_GET_OPERATOR_FAMILY.to_string()));
    }

    #[test]
    fn wallet_structure_tool_exposes_owner_and_concentration_fields() {
        let context = sample_context();
        let payload = dispatch_tool(AskMiaToolName::WalletStructure, &context);

        assert_eq!(payload["owner_holding_pct"], 17.5);
        assert_eq!(payload["owner_in_top_holders"], true);
        assert_eq!(payload["indexed_holder_count"], 456);
        assert_eq!(payload["wallet_concentration_score"], 78);
    }

    #[test]
    fn deployer_memory_tool_exposes_recent_launches() {
        let context = sample_context();
        let payload = dispatch_tool(AskMiaToolName::DeployerMemory, &context);

        assert_eq!(payload["trust_grade"], "C");
        assert_eq!(payload["rug_count"], 1);
        assert_eq!(payload["recent_launches"][0]["contract_address"], "0xprior");
    }
}
