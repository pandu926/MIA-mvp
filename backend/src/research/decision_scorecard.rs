use serde::Serialize;

use crate::{
    api::investigation::{AlphaContextSnapshot, MarketIntelligence, RiskSnapshot, TokenSnapshot},
    research::launch_intelligence::{
        DeployerMemorySummary, OperatorFamilySummary, WalletStructureSummary,
    },
};

#[derive(Debug, Clone, Serialize)]
pub struct DecisionSubscore {
    pub id: String,
    pub label: String,
    pub score: i16,
    pub weight_pct: i16,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecisionScorecard {
    pub decision_score: i16,
    pub verdict: String,
    pub confidence_label: String,
    pub primary_reason: String,
    pub primary_risk: String,
    pub subscores: Vec<DecisionSubscore>,
}

fn clamp_score(value: f64) -> i16 {
    value.round().clamp(0.0, 100.0) as i16
}

fn trust_grade_score(grade: &str) -> f64 {
    match grade.trim().to_ascii_uppercase().as_str() {
        "A" => 88.0,
        "B" => 74.0,
        "C" => 58.0,
        "D" => 38.0,
        "F" => 20.0,
        _ => 48.0,
    }
}

fn weighted_average(subscores: &[DecisionSubscore]) -> i16 {
    if subscores.is_empty() {
        return 50;
    }

    let total_weight: f64 = subscores
        .iter()
        .map(|subscore| subscore.weight_pct as f64)
        .sum();
    if total_weight <= 0.0 {
        return 50;
    }

    let weighted = subscores
        .iter()
        .map(|subscore| subscore.score as f64 * subscore.weight_pct as f64)
        .sum::<f64>()
        / total_weight;

    clamp_score(weighted)
}

fn verdict_for_score(score: i16) -> String {
    if score >= 78 {
        "HIGH CONVICTION".to_string()
    } else if score >= 62 {
        "SPECULATIVE ENTRY".to_string()
    } else if score >= 42 {
        "WATCH".to_string()
    } else {
        "AVOID".to_string()
    }
}

fn confidence_for_score(score: i16, highest: i16, lowest: i16) -> String {
    let spread = highest.saturating_sub(lowest);
    if score >= 75 || score <= 25 || spread >= 40 {
        "HIGH".to_string()
    } else if score >= 62 || score < 42 || spread >= 24 {
        "MEDIUM".to_string()
    } else {
        "LOW".to_string()
    }
}

pub fn build_decision_scorecard(
    token: &TokenSnapshot,
    risk: Option<&RiskSnapshot>,
    market_intelligence: &MarketIntelligence,
    wallet_structure: &WalletStructureSummary,
    deployer_memory: Option<&DeployerMemorySummary>,
    operator_family: &OperatorFamilySummary,
    alpha_context: Option<&AlphaContextSnapshot>,
) -> DecisionScorecard {
    let total_trades = (token.buy_count + token.sell_count).max(1) as f64;
    let buy_share = (token.buy_count.max(0) as f64 / total_trades) * 100.0;

    let market_score = {
        let mut score = 50.0 + ((buy_share - 50.0) * 0.7);
        if token.volume_bnb >= 5.0 {
            score += 12.0;
        } else if token.volume_bnb < 0.5 {
            score -= 10.0;
        }
        if let Some(excitement_score) = market_intelligence.excitement_score {
            score += (excitement_score as f64 - 50.0) * 0.18;
        }
        if let Some(risk_snapshot) = risk {
            if let Some(flow_risk) = risk_snapshot.buy_sell_velocity_score {
                score -= (flow_risk as f64 - 35.0).max(0.0) * 0.22;
            }
        }
        clamp_score(score)
    };

    let wallet_score = {
        let mut score = 72.0;
        if let Some(risk_snapshot) = risk {
            if let Some(wallet_risk) = risk_snapshot.wallet_concentration_score {
                score -= wallet_risk as f64 * 0.55;
            }
        }
        if wallet_structure.active_wallet_count >= 30 {
            score += 8.0;
        } else if wallet_structure.active_wallet_count <= 10 && token.volume_bnb >= 5.0 {
            score -= 10.0;
        }
        if wallet_structure.holder_count >= 200 {
            score += 6.0;
        } else if wallet_structure.holder_count < 50 {
            score -= 8.0;
        }
        score -= wallet_structure.probable_cluster_wallets as f64 * 4.0;
        score -= wallet_structure.potential_cluster_wallets as f64 * 2.0;
        clamp_score(score)
    };

    let builder_score = {
        let mut score = deployer_memory
            .map(|memory| trust_grade_score(&memory.trust_grade))
            .unwrap_or(48.0);
        if let Some(memory) = deployer_memory {
            score -= memory.rug_count as f64 * 6.0;
            score += memory.graduated_count as f64 * 2.5;
            if memory.honeypot_history {
                score -= 12.0;
            }
        }
        clamp_score(score)
    };

    let operator_score = operator_family.safety_score;

    let ml_score = {
        let mut score = alpha_context.map(|alpha| alpha.alpha_score).unwrap_or(50.0);
        if let Some(alpha) = alpha_context {
            if alpha.rank <= 3 {
                score += 8.0;
            } else if alpha.rank <= 10 {
                score += 4.0;
            }
        }
        clamp_score(score)
    };

    let subscores = vec![
        DecisionSubscore {
            id: "market_structure".to_string(),
            label: "Market Structure".to_string(),
            score: market_score,
            weight_pct: 24,
            summary: if buy_share >= 55.0 {
                format!(
                    "Buy-side pressure still leads at {:.0}% with {:.2} BNB in tracked flow.",
                    buy_share, token.volume_bnb
                )
            } else if buy_share <= 45.0 {
                format!(
                    "Sell-side pressure is heavier at {:.0}% of tracked trades, so the move is fragile.",
                    100.0 - buy_share
                )
            } else {
                format!(
                    "Flow is balanced around {:.0}% buy share, so market structure is still unresolved.",
                    buy_share
                )
            },
        },
        DecisionSubscore {
            id: "wallet_structure".to_string(),
            label: "Wallet Structure".to_string(),
            score: wallet_score,
            weight_pct: 22,
            summary: format!(
                "{} Active wallets: {}. Probable clusters: {}.",
                wallet_structure.summary,
                wallet_structure.active_wallet_count,
                wallet_structure.probable_cluster_wallets
            ),
        },
        DecisionSubscore {
            id: "builder_history".to_string(),
            label: "Builder History".to_string(),
            score: builder_score,
            weight_pct: 20,
            summary: deployer_memory
                .map(|memory| memory.summary.clone())
                .unwrap_or_else(|| {
                    "Builder history is still thin, so MIA cannot give this launch a strong trust lift yet."
                        .to_string()
                }),
        },
        DecisionSubscore {
            id: "operator_family".to_string(),
            label: "Operator Pattern".to_string(),
            score: operator_score,
            weight_pct: 22,
            summary: operator_family.summary.clone(),
        },
        DecisionSubscore {
            id: "ml_alignment".to_string(),
            label: "ML Alignment".to_string(),
            score: ml_score,
            weight_pct: 12,
            summary: alpha_context
                .map(|alpha| {
                    format!(
                        "MIA ranks the token at #{}, with alpha score {:.1}.",
                        alpha.rank, alpha.alpha_score
                    )
                })
                .unwrap_or_else(|| {
                    "ML context is attached, but the token does not yet carry a strong live rank."
                        .to_string()
                }),
        },
    ];

    let decision_score = weighted_average(&subscores);
    let highest = subscores
        .iter()
        .map(|subscore| subscore.score)
        .max()
        .unwrap_or(50);
    let lowest = subscores
        .iter()
        .map(|subscore| subscore.score)
        .min()
        .unwrap_or(50);
    let strongest_support = subscores
        .iter()
        .max_by_key(|subscore| subscore.score)
        .map(|subscore| format!("{}: {}", subscore.label, subscore.summary))
        .unwrap_or_else(|| "No strong supporting layer was recovered.".to_string());
    let strongest_risk = subscores
        .iter()
        .min_by_key(|subscore| subscore.score)
        .map(|subscore| format!("{}: {}", subscore.label, subscore.summary))
        .unwrap_or_else(|| "No dominant risk layer was recovered.".to_string());

    DecisionScorecard {
        decision_score,
        verdict: verdict_for_score(decision_score),
        confidence_label: confidence_for_score(decision_score, highest, lowest),
        primary_reason: strongest_support,
        primary_risk: strongest_risk,
        subscores,
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::build_decision_scorecard;
    use crate::{
        api::investigation::{
            AlphaContextSnapshot, MarketIntelligence, RiskSnapshot, TokenSnapshot,
        },
        research::launch_intelligence::{
            DeployerMemorySummary, OperatorFamilySummary, WalletStructureSummary,
        },
    };

    fn token_snapshot() -> TokenSnapshot {
        TokenSnapshot {
            contract_address: "0xabc".to_string(),
            name: Some("Token".to_string()),
            symbol: Some("TOK".to_string()),
            deployer_address: "0xdeployer".to_string(),
            deployed_at: Utc::now(),
            block_number: 1,
            tx_hash: "0xtx".to_string(),
            initial_liquidity_bnb: Some(2.5),
            participant_wallet_count: 240,
            holder_count: 240,
            buy_count: 120,
            sell_count: 70,
            volume_bnb: 12.0,
            is_rug: false,
            graduated: false,
            honeypot_detected: false,
        }
    }

    fn risk_snapshot() -> RiskSnapshot {
        RiskSnapshot {
            composite_score: 28,
            risk_category: "low".to_string(),
            deployer_history_score: Some(22),
            liquidity_lock_score: Some(18),
            wallet_concentration_score: Some(26),
            buy_sell_velocity_score: Some(34),
            contract_audit_score: Some(12),
            social_authenticity_score: Some(24),
            volume_consistency_score: Some(30),
            computed_at: Utc::now(),
        }
    }

    fn market_intelligence() -> MarketIntelligence {
        MarketIntelligence {
            provider: "test".to_string(),
            available: true,
            x_summary: None,
            web_summary: None,
            active_event: Some("viral topic".to_string()),
            narrative_alignment: Some("aligned".to_string()),
            excitement_score: Some(72),
            risk_flags: Vec::new(),
            sources: Vec::new(),
            raw_summary: None,
            notes: Vec::new(),
        }
    }

    fn wallet_structure() -> WalletStructureSummary {
        WalletStructureSummary {
            summary: "Wallet participation looks broad.".to_string(),
            evidence: Vec::new(),
            active_wallet_count: 44,
            participant_wallet_count: 240,
            holder_count: 240,
            probable_cluster_wallets: 1,
            potential_cluster_wallets: 1,
            repeated_wallet_count: 1,
            top_flow_wallets: Vec::new(),
        }
    }

    fn deployer_memory() -> DeployerMemorySummary {
        DeployerMemorySummary {
            summary: "Builder has a B-grade history with no rugs.".to_string(),
            evidence: Vec::new(),
            trust_grade: "B".to_string(),
            trust_label: "Good".to_string(),
            total_launches: 4,
            rug_count: 0,
            graduated_count: 2,
            honeypot_history: false,
            first_seen_at: None,
            last_seen_at: None,
            recent_launches: Vec::new(),
        }
    }

    fn operator_family() -> OperatorFamilySummary {
        OperatorFamilySummary {
            confidence: "low".to_string(),
            summary: "No strong cross-launch operator-family pattern has been recovered yet."
                .to_string(),
            evidence: Vec::new(),
            safety_score: 82,
            signal_score: 18,
            related_launch_count: 0,
            related_deployer_count: 0,
            repeated_wallet_count: 0,
            seller_to_new_builder_count: 0,
            seller_reentry_wallet_count: 0,
            probable_cluster_wallets: 0,
            potential_cluster_wallets: 0,
            repeated_wallets: Vec::new(),
            migrated_wallets: Vec::new(),
            related_launches: Vec::new(),
        }
    }

    #[test]
    fn healthy_profile_scores_above_watch() {
        let scorecard = build_decision_scorecard(
            &token_snapshot(),
            Some(&risk_snapshot()),
            &market_intelligence(),
            &wallet_structure(),
            Some(&deployer_memory()),
            &operator_family(),
            Some(&AlphaContextSnapshot {
                rank: 3,
                alpha_score: 82.0,
                rationale: "Strong".to_string(),
                window_end: Utc::now(),
            }),
        );

        assert!(
            scorecard.decision_score >= 62,
            "score={}",
            scorecard.decision_score
        );
        assert_ne!(scorecard.verdict, "AVOID");
    }

    #[test]
    fn suspicious_operator_pattern_pulls_score_down() {
        let mut suspicious_operator = operator_family();
        suspicious_operator.safety_score = 24;
        suspicious_operator.signal_score = 76;
        suspicious_operator.confidence = "high".to_string();
        suspicious_operator.summary =
            "A likely coordinated operator family is visible across repeated wallets and related launches."
                .to_string();

        let scorecard = build_decision_scorecard(
            &token_snapshot(),
            Some(&risk_snapshot()),
            &market_intelligence(),
            &wallet_structure(),
            Some(&deployer_memory()),
            &suspicious_operator,
            Some(&AlphaContextSnapshot {
                rank: 4,
                alpha_score: 79.0,
                rationale: "Still strong".to_string(),
                window_end: Utc::now(),
            }),
        );

        assert!(scorecard
            .primary_risk
            .to_ascii_lowercase()
            .contains("operator pattern"));
        assert!(
            scorecard.decision_score < 70,
            "score={}",
            scorecard.decision_score
        );
    }
}
