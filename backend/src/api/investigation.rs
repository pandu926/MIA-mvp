mod llm;
mod sources;
mod types;

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use serde_json::{json, Value};

use crate::{ai::ask_mia, error::AppError, AppState};

use super::{
    deep_research::{
        create_research_run, deep_research_provider_label, has_inflight_research_run,
        load_cached_report, CachedReportRecord,
    },
    investigation_runs::{load_investigation_run_detail, persist_manual_investigation_run},
};

#[allow(unused_imports)]
pub use types::{
    AgentScorecard, AlphaContextSnapshot, AskMiaAnswer, AskMiaRequest, AskMiaResponse,
    AskMiaRunContext, AskMiaRunContextEvent, ContractIntelligence, DeployerSnapshot,
    DeployerTokenSnapshot, HolderSnapshot, InternalEvidence, InvestigationDeepResearchState,
    InvestigationResponse, InvestigationSource, InvestigationTripwires, MarketIntelligence,
    PublicInternalEvidence, RiskSnapshot, TokenSnapshot, TransactionSnapshot,
    WhaleActivitySnapshot,
};

pub(crate) use llm::{
    build_agent_scorecard, build_ask_mia_fallback, build_ask_mia_grounded_layers,
    build_ask_mia_trace, build_investigation_tripwires, build_unscored_analysis,
    clean_sentence, normalize_ask_mia_evidence, parse_json_payload, run_ask_mia_v1,
    synthesize_analysis, validate_ask_mia_question,
};
pub(crate) use sources::{fetch_token_snapshot, load_investigation_bundle};

pub async fn get_token_investigation(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<InvestigationResponse>, AppError> {
    let (mut internal, contract_intelligence, market_intelligence, source_status) =
        load_investigation_bundle(&state, &address).await?;

    let cached_deep_research: Option<CachedReportRecord> =
        load_cached_deep_research(&state, &address).await?;
    let total_tx = i64::from(internal.token.buy_count) + i64::from(internal.token.sell_count);
    let auto_requested =
        maybe_queue_auto_deep_research(&state, &address, total_tx, cached_deep_research.is_some())
            .await?;
    let ai_score_enabled =
        total_tx > state.config.ai_score_min_tx_count || cached_deep_research.is_some();
    let deep_research_context = cached_deep_research
        .as_ref()
        .map(build_deep_research_prompt_context);
    let deep_research = InvestigationDeepResearchState {
        report_cached: cached_deep_research.is_some(),
        report_generated_at: cached_deep_research.as_ref().map(|report| report.updated_at),
        auto_threshold_met: total_tx >= state.config.auto_deep_research_tx_threshold,
        auto_threshold_tx_count: state.config.auto_deep_research_tx_threshold,
        auto_requested,
        ai_score_enabled,
        ai_score_gate_tx_count: state.config.ai_score_min_tx_count,
        score_enriched: cached_deep_research.is_some(),
    };
    let analysis = if ai_score_enabled {
        let analysis = synthesize_analysis(
            state.config.clone(),
            &internal,
            &contract_intelligence,
            &market_intelligence,
            deep_research_context.as_ref(),
        )
        .await;
        if analysis.score.is_some() {
            internal.agent_scorecard = Some(build_agent_scorecard(&analysis));
        }
        analysis
    } else {
        build_unscored_analysis(
            &internal,
            &market_intelligence,
            Some(format!(
                "AI score skipped because total transaction count {} is at or below budget gate {}.",
                total_tx, state.config.ai_score_min_tx_count
            )),
        )
    };
    let tripwires =
        build_investigation_tripwires(&internal, &contract_intelligence, &analysis, &deep_research);

    let mut response = InvestigationResponse {
        token_address: address,
        generated_at: Utc::now(),
        active_run: None,
        deep_research,
        tripwires,
        internal: PublicInternalEvidence::from(internal),
        contract_intelligence,
        market_intelligence,
        analysis,
        source_status,
    };

    let active_run = persist_manual_investigation_run(&state.db, &response).await?;
    response.active_run = Some(active_run);

    Ok(Json(response))
}

pub async fn post_token_ask_mia(
    State(state): State<AppState>,
    Path(address): Path<String>,
    Json(payload): Json<AskMiaRequest>,
) -> Result<Json<AskMiaResponse>, AppError> {
    let question = payload.question.trim().to_string();
    validate_ask_mia_question(&question)?;

    let (mut internal, contract_intelligence, market_intelligence, _) =
        load_investigation_bundle(&state, &address).await?;
    let cached_deep_research: Option<CachedReportRecord> =
        load_cached_deep_research(&state, &address).await?;
    let total_tx = i64::from(internal.token.buy_count) + i64::from(internal.token.sell_count);
    let ai_score_enabled =
        total_tx > state.config.ai_score_min_tx_count || cached_deep_research.is_some();
    let deep_research_context = cached_deep_research
        .as_ref()
        .map(build_deep_research_prompt_context);
    if ai_score_enabled {
        let analysis = synthesize_analysis(
            state.config.clone(),
            &internal,
            &contract_intelligence,
            &market_intelligence,
            deep_research_context.as_ref(),
        )
        .await;
        if analysis.score.is_some() {
            internal.agent_scorecard = Some(build_agent_scorecard(&analysis));
        }
    } else {
        let _ = build_unscored_analysis(
            &internal,
            &market_intelligence,
            Some(format!(
                "AI score skipped because total transaction count {} is at or below budget gate {}.",
                total_tx, state.config.ai_score_min_tx_count
            )),
        );
    }

    let run_context = if let Some(run_id) = payload.run_id {
        let detail = load_investigation_run_detail(&state.db, run_id)
            .await?
            .ok_or_else(|| AppError::BadRequest("Attached run was not found.".to_string()))?;
        if !detail.run.token_address.eq_ignore_ascii_case(&address) {
            return Err(AppError::BadRequest(
                "Attached run does not belong to this token.".to_string(),
            ));
        }
        Some(build_ask_mia_run_context(&detail))
    } else {
        None
    };

    let grounded_layers =
        build_ask_mia_grounded_layers(&internal, &market_intelligence, run_context.as_ref());
    let fallback = build_ask_mia_fallback(&question, &internal, &market_intelligence);

    let (mode, provider, tool_trace, answer, fallback_used) =
        if state.config.ask_mia_function_calling_enabled {
            match ask_mia::run_function_calling(
                &state.config,
                &question,
                &internal,
                &contract_intelligence,
                &market_intelligence,
                run_context.as_ref(),
            )
            .await
            {
                Ok(result) => (
                    "function_calling".to_string(),
                    result.provider,
                    result.tool_trace,
                    result.answer,
                    false,
                ),
                Err(err) => {
                    tracing::warn!("Ask MIA v2 failed, falling back to v1: {}", err);
                    let (provider, answer, fallback_used) = run_ask_mia_v1(
                        &state,
                        &question,
                        &internal,
                        &contract_intelligence,
                        &market_intelligence,
                        run_context.as_ref(),
                        fallback,
                    )
                    .await;
                    (
                        "grounded".to_string(),
                        provider,
                        Vec::new(),
                        answer,
                        fallback_used,
                    )
                }
            }
        } else {
            let (provider, answer, fallback_used) = run_ask_mia_v1(
                &state,
                &question,
                &internal,
                &contract_intelligence,
                &market_intelligence,
                run_context.as_ref(),
                fallback,
            )
            .await;
            (
                "grounded".to_string(),
                provider,
                Vec::new(),
                answer,
                fallback_used,
            )
        };

    Ok(Json(AskMiaResponse {
        token_address: address,
        question,
        generated_at: Utc::now(),
        mode,
        provider,
        grounded_layers,
        analysis_trace: build_ask_mia_trace(&tool_trace),
        tool_trace,
        run_context,
        answer,
        fallback_used,
    }))
}

async fn load_cached_deep_research(
    state: &AppState,
    token_address: &str,
) -> Result<Option<CachedReportRecord>, AppError> {
    if !state.config.deep_research_enabled {
        return Ok(None);
    }

    load_cached_report(
        &state.db,
        token_address,
        deep_research_provider_label(state.config.deep_research_provider),
    )
    .await
}

async fn maybe_queue_auto_deep_research(
    state: &AppState,
    token_address: &str,
    total_tx: i64,
    already_cached: bool,
) -> Result<bool, AppError> {
    if !state.config.deep_research_enabled
        || already_cached
        || total_tx < state.config.auto_deep_research_tx_threshold
    {
        return Ok(false);
    }

    if has_inflight_research_run(&state.db, token_address).await? {
        return Ok(false);
    }

    let state_clone = state.clone();
    let token_address = token_address.to_string();
    tokio::spawn(async move {
        if let Err(err) = create_research_run(&state_clone, &token_address).await {
            tracing::warn!(
                token_address = %token_address,
                error = %err,
                "auto deep research background run failed"
            );
        }
    });

    Ok(true)
}

fn build_deep_research_prompt_context(cached: &CachedReportRecord) -> Value {
    let sections = if let Some(items) = cached.sections.as_array() {
        items
            .iter()
            .take(6)
            .map(|item| {
                json!({
                    "id": item.get("id").and_then(Value::as_str),
                    "title": item.get("title").and_then(Value::as_str),
                    "summary": item.get("summary").and_then(Value::as_str),
                    "stage": item.get("stage").and_then(Value::as_str),
                    "confidence": item.get("confidence").and_then(Value::as_str),
                    "source_agent": item.get("source_agent").and_then(Value::as_str),
                    "fallback_note": item.get("fallback_note").and_then(Value::as_str),
                })
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    json!({
        "executive_summary": cached.executive_summary,
        "sections": sections,
        "citations": cached.citations,
        "source_status": cached.source_status,
        "updated_at": cached.updated_at,
    })
}

fn build_ask_mia_run_context(
    detail: &super::investigation_runs::InvestigationRunDetailResponse,
) -> AskMiaRunContext {
    let recent_events = detail
        .timeline
        .iter()
        .rev()
        .take(3)
        .map(|event| AskMiaRunContextEvent {
            label: event.label.clone(),
            detail: event.detail.clone(),
            at: event.at,
        })
        .collect::<Vec<_>>();

    AskMiaRunContext {
        run_id: detail.run.run_id,
        status: detail.run.status.clone(),
        current_stage: detail.run.current_stage.clone(),
        continuity_note: detail.continuity_note.clone(),
        latest_reason: detail.run.status_reason.clone(),
        latest_evidence_delta: detail.run.evidence_delta.clone(),
        recent_events,
    }
}
