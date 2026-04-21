use crate::{
    api::verdict::compute_token_verdict,
    error::AppError,
    indexer::deployer::get_deployer_profile,
    research::{
        decision_scorecard::build_decision_scorecard,
        launch_intelligence::{
            build_deployer_memory_summary, build_operator_family_summary,
            build_wallet_structure_summary,
        },
    },
    AppState,
};

use super::super::types::{
    ContractIntelligence, DeployerSnapshot, InternalEvidence, MarketIntelligence, SourceStatus,
};
use super::{
    external::{fetch_contract_intelligence, fetch_market_intelligence},
    internal::{
        fetch_alpha_context, fetch_deployer_recent_tokens, fetch_narrative_cache,
        fetch_recent_transactions, fetch_risk_snapshot, fetch_token_snapshot, fetch_whale_activity,
    },
};

pub(crate) async fn load_investigation_bundle(
    state: &AppState,
    address: &str,
) -> Result<
    (
        InternalEvidence,
        ContractIntelligence,
        MarketIntelligence,
        SourceStatus,
    ),
    AppError,
> {
    let token = fetch_token_snapshot(state, address).await?;

    let risk_fut = fetch_risk_snapshot(state, address);
    let narrative_fut = fetch_narrative_cache(state, address);
    let transactions_fut = fetch_recent_transactions(state, address, 25);
    let whales_fut = fetch_whale_activity(state, address);
    let alpha_fut = fetch_alpha_context(state, address);
    let verdict_fut = compute_token_verdict(state, address);

    let (risk, narrative_cache, recent_transactions, whale_activity_24h, alpha_context, verdict) = tokio::join!(
        risk_fut,
        narrative_fut,
        transactions_fut,
        whales_fut,
        alpha_fut,
        verdict_fut,
    );

    let risk = risk?;
    let narrative_cache = narrative_cache?;
    let recent_transactions = recent_transactions?;
    let whale_activity_24h = whale_activity_24h?;
    let alpha_context = alpha_context?;
    let verdict = verdict?;

    let deployer_profile = get_deployer_profile(&state.db, &token.deployer_address).await?;
    let deployer = deployer_profile.as_ref().map(|profile| DeployerSnapshot {
        address: profile.address.clone(),
        total_tokens_deployed: profile.total_tokens_deployed,
        rug_count: profile.rug_count,
        graduated_count: profile.graduated_count,
        honeypot_detected: profile.honeypot_detected,
        trust_grade: profile.trust_grade.as_str().to_string(),
        trust_label: profile.trust_grade.label().to_string(),
        first_seen_at: profile.first_seen_at,
        last_seen_at: profile.last_seen_at,
    });

    let deployer_recent_tokens =
        fetch_deployer_recent_tokens(state, &token.deployer_address, 8).await?;

    let (
        contract_intelligence,
        market_intelligence,
        wallet_structure,
        deployer_memory,
        operator_family,
    ) = tokio::join!(
        fetch_contract_intelligence(state.config.clone(), &token),
        fetch_market_intelligence(state.config.clone(), &token),
        build_wallet_structure_summary(&state.db, &token),
        build_deployer_memory_summary(&state.db, &token),
        build_operator_family_summary(&state.db, &token),
    );
    let wallet_structure = wallet_structure?;
    let deployer_memory = deployer_memory?;
    let operator_family = operator_family?;
    let decision_scorecard = build_decision_scorecard(
        &token,
        risk.as_ref(),
        &market_intelligence,
        &wallet_structure,
        deployer_memory.as_ref(),
        &operator_family,
        alpha_context.as_ref(),
    );

    let mut source_notes = Vec::new();
    source_notes.extend(contract_intelligence.notes.iter().cloned());
    source_notes.extend(market_intelligence.notes.iter().cloned());

    let internal = InternalEvidence {
        token,
        risk,
        agent_scorecard: None,
        verdict,
        narrative_cache,
        deployer,
        deployer_recent_tokens,
        recent_transactions,
        whale_activity_24h,
        alpha_context,
        wallet_structure,
        deployer_memory,
        operator_family,
        decision_scorecard,
    };

    let source_status = SourceStatus {
        bscscan_configured: state
            .config
            .bscscan_api_key
            .as_ref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false),
        market_provider: market_intelligence.provider.clone(),
        notes: source_notes,
    };

    Ok((
        internal,
        contract_intelligence,
        market_intelligence,
        source_status,
    ))
}
