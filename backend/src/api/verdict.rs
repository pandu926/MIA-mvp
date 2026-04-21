use crate::{error::AppError, indexer::deployer::get_deployer_profile, AppState};
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct VerdictInsight {
    pub label: String,
    pub tone: String,
    pub detail: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct VerdictAction {
    pub label: String,
    pub href: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct VerdictResponse {
    pub token_address: String,
    pub label: String,
    pub tone: String,
    pub score: i16,
    pub confidence_label: String,
    pub headline: String,
    pub summary: String,
    pub evidence: Vec<String>,
    pub concerns: Vec<String>,
    pub narrative_reality: VerdictInsight,
    pub whale_intent: VerdictInsight,
    pub deployer_dna: VerdictInsight,
    pub next_actions: Vec<VerdictAction>,
}

#[derive(Debug)]
struct TokenInput {
    contract_address: String,
    deployer_address: String,
    buy_count: i32,
    sell_count: i32,
    volume_bnb: f64,
    is_rug: bool,
    graduated: bool,
    honeypot_detected: bool,
}

#[derive(Debug)]
struct RiskInput {
    composite_score: i16,
    risk_category: String,
    wallet_concentration_score: Option<i16>,
    buy_sell_velocity_score: Option<i16>,
    volume_consistency_score: Option<i16>,
}

#[derive(Debug)]
struct NarrativeInput {
    consensus_status: String,
    confidence: String,
}

#[derive(Debug)]
struct DeployerInput {
    rug_count: i64,
    graduated_count: i64,
    trust_grade: String,
}

#[derive(Debug)]
struct AlphaInput {
    rank: i16,
    alpha_score: f64,
}

#[derive(Debug)]
struct TransactionInput {
    wallet_address: String,
    amount_bnb: f64,
}

#[derive(Debug)]
struct WhaleInput {
    alert_level: String,
}

fn clamp_i16(value: i16, min: i16, max: i16) -> i16 {
    value.max(min).min(max)
}

fn avg_trade_size(transactions: &[TransactionInput]) -> f64 {
    if transactions.is_empty() {
        return 0.0;
    }
    let total: f64 = transactions.iter().map(|tx| tx.amount_bnb).sum();
    total / transactions.len() as f64
}

fn holder_concentration(transactions: &[TransactionInput]) -> f64 {
    use std::collections::HashMap;

    let mut by_wallet: HashMap<&str, f64> = HashMap::new();
    for tx in transactions {
        *by_wallet.entry(tx.wallet_address.as_str()).or_default() += tx.amount_bnb;
    }
    let mut values: Vec<f64> = by_wallet.values().copied().collect();
    values.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let total: f64 = values.iter().sum();
    if total <= 0.0 {
        return 0.0;
    }
    let top_ten: f64 = values.into_iter().take(10).sum();
    ((top_ten / total) * 100.0).clamp(0.0, 100.0)
}

fn confidence_label(score: i16) -> &'static str {
    if score >= 75 || score <= 25 {
        "HIGH"
    } else if score >= 60 || score <= 40 {
        "MEDIUM"
    } else {
        "LOW"
    }
}

fn tone_for_label(label: &str) -> &'static str {
    match label {
        "HIGH CONVICTION" => "safe",
        "SPECULATIVE ENTRY" => "primary",
        "WATCH" => "warn",
        _ => "danger",
    }
}

fn short_address(value: &str, head: usize, tail: usize) -> String {
    if value.len() <= head + tail + 3 {
        return value.to_string();
    }
    format!("{}...{}", &value[..head], &value[value.len() - tail..])
}

fn build_verdict(
    token: &TokenInput,
    risk: Option<&RiskInput>,
    narrative: Option<&NarrativeInput>,
    deployer: Option<&DeployerInput>,
    transactions: &[TransactionInput],
    whales: &[WhaleInput],
    alpha: Option<&AlphaInput>,
) -> VerdictResponse {
    let flow_delta = token.buy_count - token.sell_count;
    let total_flow = (token.buy_count + token.sell_count).max(1) as f64;
    let buy_ratio = token.buy_count.max(0) as f64 / total_flow;
    let avg_trade = avg_trade_size(transactions);
    let concentration = holder_concentration(transactions);
    let critical_whales = whales
        .iter()
        .filter(|row| row.alert_level == "critical")
        .count() as i16;

    let mut score = 50.0;

    if let Some(risk) = risk {
        score += (50.0 - risk.composite_score as f64) * 0.55;
        if let Some(wallet_score) = risk.wallet_concentration_score {
            score -= wallet_score as f64 * 0.08;
        }
        if let Some(flow_score) = risk.buy_sell_velocity_score {
            score -= flow_score as f64 * 0.10;
        }
    }

    if let Some(deployer) = deployer {
        match deployer.trust_grade.as_str() {
            "A" => score += 15.0,
            "B" => score += 8.0,
            "D" => score -= 10.0,
            "F" => score -= 18.0,
            _ => {}
        }
        score -= deployer.rug_count as f64 * 4.0;
        score += deployer.graduated_count as f64 * 2.0;
    }

    if token.honeypot_detected {
        score -= 35.0;
    }
    if token.is_rug {
        score -= 30.0;
    }
    if token.graduated {
        score += 10.0;
    }
    if flow_delta > 0 {
        score += (flow_delta.min(12)) as f64;
    } else if flow_delta < 0 {
        score -= (flow_delta.abs().min(12)) as f64;
    }
    if token.volume_bnb >= 5.0 {
        score += 8.0;
    }
    if token.volume_bnb < 0.5 {
        score -= 5.0;
    }

    if let Some(alpha) = alpha {
        if alpha.alpha_score >= 75.0 {
            score += 12.0;
        } else if alpha.alpha_score >= 55.0 {
            score += 6.0;
        }
    }

    if let Some(narrative) = narrative {
        match narrative.consensus_status.as_str() {
            "agreed" => score += 5.0,
            "diverged" => score -= 4.0,
            _ => {}
        }
        match narrative.confidence.as_str() {
            "high" => score += 5.0,
            "low" => score -= 3.0,
            _ => {}
        }
    }

    if critical_whales > 0 && buy_ratio >= 0.60 {
        score += 6.0;
    }
    if critical_whales > 1 && concentration > 70.0 {
        score -= 5.0;
    }

    let score = clamp_i16(score.round() as i16, 0, 100);

    let label = if token.honeypot_detected
        || token.is_rug
        || risk.map(|r| r.composite_score >= 70).unwrap_or(false)
    {
        "AVOID".to_string()
    } else if score >= 78 {
        "HIGH CONVICTION".to_string()
    } else if score >= 62 {
        "SPECULATIVE ENTRY".to_string()
    } else if score < 42 {
        "AVOID".to_string()
    } else {
        "WATCH".to_string()
    };

    let mut evidence = Vec::new();
    let mut concerns = Vec::new();

    if let Some(risk) = risk {
        evidence.push(format!(
            "Composite risk {}/100 ({})",
            risk.composite_score, risk.risk_category
        ));
    }
    if let Some(alpha) = alpha {
        evidence.push(format!(
            "Alpha rank #{} with score {:.1}",
            alpha.rank, alpha.alpha_score
        ));
    }
    if let Some(deployer) = deployer {
        evidence.push(format!(
            "Deployer grade {} with {} graduates and {} rugs",
            deployer.trust_grade, deployer.graduated_count, deployer.rug_count
        ));
    }
    evidence.push(format!(
        "Flow delta {:+} from {} buys vs {} sells",
        flow_delta, token.buy_count, token.sell_count
    ));
    evidence.push(format!(
        "Average trade size {:.2} BNB across {} transactions",
        avg_trade,
        transactions.len()
    ));

    if token.honeypot_detected {
        concerns.push("Contract is flagged as honeypot.".to_string());
    }
    if risk
        .and_then(|r| r.wallet_concentration_score)
        .unwrap_or_default()
        >= 70
    {
        concerns.push("Wallet concentration is elevated.".to_string());
    }
    if concentration >= 70.0 {
        concerns.push(format!(
            "Top wallets control an estimated {:.1}% of observed flow.",
            concentration
        ));
    }
    if deployer.map(|d| d.rug_count >= 2).unwrap_or(false) {
        concerns.push("Deployer history shows multiple rugs.".to_string());
    }
    if flow_delta < 0 {
        concerns.push("Sell pressure is stronger than buy pressure.".to_string());
    }
    if token.volume_bnb < 0.5 {
        concerns.push("Liquidity is still thin for a confident entry.".to_string());
    }
    if narrative
        .map(|n| n.confidence.as_str() == "low")
        .unwrap_or(false)
    {
        concerns.push("Narrative confidence is still low.".to_string());
    }
    if critical_whales > 1 && buy_ratio < 0.55 {
        concerns
            .push("Whale traffic exists, but it does not confirm clean accumulation.".to_string());
    }

    let narrative_reality = if let Some(narrative) = narrative {
        if buy_ratio >= 0.60
            && narrative.consensus_status == "agreed"
            && narrative.confidence != "low"
        {
            VerdictInsight {
                label: "Narrative Confirmed".to_string(),
                tone: "safe".to_string(),
                detail: "Social narrative and observed buy flow are aligned, which supports momentum continuation.".to_string(),
            }
        } else if buy_ratio < 0.50
            || risk
                .and_then(|r| r.volume_consistency_score)
                .map(|v| v >= 60)
                .unwrap_or(false)
        {
            VerdictInsight {
                label: "Narrative Divergence".to_string(),
                tone: "danger".to_string(),
                detail: "Story quality is weaker than the live flow suggests, so this setup can trap late buyers.".to_string(),
            }
        } else {
            VerdictInsight {
                label: "Narrative Building".to_string(),
                tone: "primary".to_string(),
                detail:
                    "There is some narrative support, but the market structure is not yet decisive."
                        .to_string(),
            }
        }
    } else {
        VerdictInsight {
            label: "Narrative Missing".to_string(),
            tone: "warn".to_string(),
            detail: "No AI narrative has been generated yet, so on-chain evidence should dominate the decision.".to_string(),
        }
    };

    let whale_intent = if whales.is_empty() {
        VerdictInsight {
            label: "No Whale Confirmation".to_string(),
            tone: "warn".to_string(),
            detail: "No token-specific whale alert is active in the current window.".to_string(),
        }
    } else if critical_whales > 0 && buy_ratio >= 0.60 && concentration < 65.0 {
        VerdictInsight {
            label: "Accumulation".to_string(),
            tone: "safe".to_string(),
            detail: format!(
                "{} critical whale event(s) support accumulation instead of thin retail-only flow.",
                critical_whales
            ),
        }
    } else if concentration >= 70.0 || buy_ratio < 0.50 {
        VerdictInsight {
            label: "Exit Liquidity Risk".to_string(),
            tone: "danger".to_string(),
            detail: "Whale activity is present, but the flow structure looks concentrated or distribution-heavy.".to_string(),
        }
    } else {
        VerdictInsight {
            label: "Rotation Watch".to_string(),
            tone: "primary".to_string(),
            detail: "Whales are active, but the setup still looks rotational rather than a clean conviction move.".to_string(),
        }
    };

    let deployer_dna = if let Some(deployer) = deployer {
        if deployer.trust_grade == "A" || deployer.trust_grade == "B" {
            VerdictInsight {
                label: "Repeatable Builder".to_string(),
                tone: "safe".to_string(),
                detail: format!(
                    "This deployer has {} graduates and a {}-grade profile.",
                    deployer.graduated_count, deployer.trust_grade
                ),
            }
        } else if deployer.rug_count >= 2 || deployer.trust_grade == "F" {
            VerdictInsight {
                label: "Serial Risk".to_string(),
                tone: "danger".to_string(),
                detail: format!(
                    "The deployer shows {} rugs and should be treated as hostile until proven otherwise.",
                    deployer.rug_count
                ),
            }
        } else {
            VerdictInsight {
                label: "Unproven Operator".to_string(),
                tone: "primary".to_string(),
                detail: "The deployer is active but not yet reliable enough for blind trust."
                    .to_string(),
            }
        }
    } else {
        VerdictInsight {
            label: "Unknown Deployer".to_string(),
            tone: "warn".to_string(),
            detail: format!(
                "No deployer profile is available for {} yet.",
                short_address(&token.deployer_address, 8, 4)
            ),
        }
    };

    let headline = match label.as_str() {
        "HIGH CONVICTION" => "This token has aligned on-chain, deployer, and momentum signals.",
        "SPECULATIVE ENTRY" => {
            "There is tradable momentum here, but execution discipline still matters."
        }
        "WATCH" => "Interesting setup, but evidence is still incomplete.",
        _ => "Current evidence points to asymmetric downside or manipulated flow.",
    }
    .to_string();

    let summary = format!(
        "{} • {} • {}",
        narrative_reality.label, whale_intent.label, deployer_dna.label
    );

    VerdictResponse {
        token_address: token.contract_address.clone(),
        label: label.clone(),
        tone: tone_for_label(&label).to_string(),
        score,
        confidence_label: confidence_label(score).to_string(),
        headline,
        summary,
        evidence,
        concerns,
        narrative_reality,
        whale_intent,
        deployer_dna,
        next_actions: vec![
            VerdictAction {
                label: "Open Replay Lab".to_string(),
                href: "/backtesting".to_string(),
            },
            VerdictAction {
                label: "Open Watchlist".to_string(),
                href: "/mia/watchlist".to_string(),
            },
            VerdictAction {
                label: "Return To Discover".to_string(),
                href: "/app".to_string(),
            },
            VerdictAction {
                label: "Inspect Whale Network".to_string(),
                href: format!("/whales/network?token={}", token.contract_address),
            },
        ],
    }
}

pub(crate) async fn compute_token_verdict(
    state: &AppState,
    address: &str,
) -> Result<VerdictResponse, AppError> {
    let token_row: Option<(String, String, i32, i32, f64, bool, bool, bool)> = sqlx::query_as(
        r#"
        SELECT
            contract_address,
            deployer_address,
            buy_count,
            sell_count,
            volume_bnb::double precision,
            is_rug,
            graduated,
            honeypot_detected
        FROM tokens
        WHERE contract_address = $1
        "#,
    )
    .bind(address)
    .fetch_optional(&state.db)
    .await?;

    let token_row =
        token_row.ok_or_else(|| AppError::NotFound(format!("Token {} not found", address)))?;

    let token = TokenInput {
        contract_address: token_row.0,
        deployer_address: token_row.1,
        buy_count: token_row.2,
        sell_count: token_row.3,
        volume_bnb: token_row.4,
        is_rug: token_row.5,
        graduated: token_row.6,
        honeypot_detected: token_row.7,
    };

    let risk_row: Option<(i16, Option<i16>, Option<i16>, Option<i16>)> = sqlx::query_as(
        r#"
        SELECT
            composite_score,
            wallet_concentration_score,
            buy_sell_velocity_score,
            volume_consistency_score
        FROM risk_scores
        WHERE token_address = $1
        "#,
    )
    .bind(address)
    .fetch_optional(&state.db)
    .await?;

    let risk = risk_row.map(|row| RiskInput {
        composite_score: row.0,
        risk_category: if row.0 <= 30 {
            "low".to_string()
        } else if row.0 <= 60 {
            "medium".to_string()
        } else {
            "high".to_string()
        },
        wallet_concentration_score: row.1,
        buy_sell_velocity_score: row.2,
        volume_consistency_score: row.3,
    });

    let now = Utc::now();
    let narrative_row: Option<(String, String, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT
            consensus_status,
            confidence,
            expires_at
        FROM ai_narratives
        WHERE token_address = $1
        "#,
    )
    .bind(address)
    .fetch_optional(&state.db)
    .await?;

    let narrative = narrative_row.and_then(|row| {
        if row.2 > now {
            Some(NarrativeInput {
                consensus_status: row.0,
                confidence: row.1,
            })
        } else {
            None
        }
    });

    let deployer_profile = get_deployer_profile(&state.db, &token.deployer_address).await?;
    let deployer = deployer_profile.map(|profile| DeployerInput {
        rug_count: profile.rug_count,
        graduated_count: profile.graduated_count,
        trust_grade: profile.trust_grade.as_str().to_string(),
    });

    let tx_rows: Vec<(String, f64)> = sqlx::query_as(
        r#"
        SELECT wallet_address, amount_bnb::double precision
        FROM token_transactions
        WHERE token_address = $1
        ORDER BY created_at DESC
        LIMIT 100
        "#,
    )
    .bind(address)
    .fetch_all(&state.db)
    .await?;

    let transactions: Vec<TransactionInput> = tx_rows
        .into_iter()
        .map(|row| TransactionInput {
            wallet_address: row.0,
            amount_bnb: row.1,
        })
        .collect();

    let alpha_row: Option<(i16, f64)> = sqlx::query_as(
        r#"
        SELECT rank, alpha_score::double precision
        FROM alpha_rankings
        WHERE token_address = $1
          AND window_end >= $2
        ORDER BY window_end DESC, rank ASC
        LIMIT 1
        "#,
    )
    .bind(address)
    .bind(Utc::now() - Duration::hours(24))
    .fetch_optional(&state.db)
    .await?;

    let alpha = alpha_row.map(|row| AlphaInput {
        rank: row.0,
        alpha_score: row.1,
    });

    let whale_rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT alert_level
        FROM whale_alerts
        WHERE token_address = $1
          AND created_at >= $2
        ORDER BY created_at DESC
        LIMIT 6
        "#,
    )
    .bind(address)
    .bind(Utc::now() - Duration::hours(24))
    .fetch_all(&state.db)
    .await?;

    let whales: Vec<WhaleInput> = whale_rows
        .into_iter()
        .map(|row| WhaleInput { alert_level: row.0 })
        .collect();

    Ok(build_verdict(
        &token,
        risk.as_ref(),
        narrative.as_ref(),
        deployer.as_ref(),
        &transactions,
        &whales,
        alpha.as_ref(),
    ))
}

pub async fn get_token_verdict(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<VerdictResponse>, AppError> {
    Ok(Json(compute_token_verdict(&state, &address).await?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_token() -> TokenInput {
        TokenInput {
            contract_address: "0xabc".to_string(),
            deployer_address: "0xdeployer".to_string(),
            buy_count: 12,
            sell_count: 2,
            volume_bnb: 8.0,
            is_rug: false,
            graduated: true,
            honeypot_detected: false,
        }
    }

    #[test]
    fn honeypot_forces_avoid() {
        let mut token = base_token();
        token.honeypot_detected = true;
        let verdict = build_verdict(&token, None, None, None, &[], &[], None);
        assert_eq!(verdict.label, "AVOID");
    }

    #[test]
    fn strong_setup_can_reach_high_conviction() {
        let token = base_token();
        let risk = RiskInput {
            composite_score: 18,
            risk_category: "low".to_string(),
            wallet_concentration_score: Some(18),
            buy_sell_velocity_score: Some(10),
            volume_consistency_score: Some(20),
        };
        let narrative = NarrativeInput {
            consensus_status: "agreed".to_string(),
            confidence: "high".to_string(),
        };
        let deployer = DeployerInput {
            rug_count: 0,
            graduated_count: 5,
            trust_grade: "A".to_string(),
        };
        let transactions = vec![
            TransactionInput {
                wallet_address: "0x1".to_string(),
                amount_bnb: 3.0,
            },
            TransactionInput {
                wallet_address: "0x2".to_string(),
                amount_bnb: 2.0,
            },
        ];
        let whales = vec![WhaleInput {
            alert_level: "critical".to_string(),
        }];
        let alpha = AlphaInput {
            rank: 1,
            alpha_score: 88.0,
        };
        let verdict = build_verdict(
            &token,
            Some(&risk),
            Some(&narrative),
            Some(&deployer),
            &transactions,
            &whales,
            Some(&alpha),
        );
        assert!(verdict.score >= 78);
        assert_eq!(verdict.label, "HIGH CONVICTION");
    }
}
