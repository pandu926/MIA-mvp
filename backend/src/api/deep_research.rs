mod planner;
mod runs;
mod service;
mod tool_registry;
mod types;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use uuid::Uuid;

use crate::{error::AppError, AppState};

pub(crate) use self::runs::{create_research_run, has_inflight_research_run};
pub(crate) use self::service::{load_cached_report, CachedReportRecord};
pub(crate) use self::types::deep_research_provider_label;
pub use types::build_deep_research_preview;
use types::DeepResearchStatusResponse;

use self::runs::{
    get_research_run as load_research_run, get_research_run_trace as load_research_run_trace,
};
use self::service::{
    build_payment_required_response, build_report_response, generate_or_load_report,
    load_active_entitlement, parse_entitlement_from_headers, verify_payment_and_issue_entitlement,
};
use self::types::{
    resource_url, runs_resource_url, unlock_resource_url, DeepResearchPreviewResponse,
    DeepResearchRunResponse, DeepResearchRunTraceResponse,
};

pub async fn preview_deep_research(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<DeepResearchPreviewResponse>, AppError> {
    Ok(Json(build_deep_research_preview(&address, &state.config)))
}

pub async fn get_deep_research_status(
    State(state): State<AppState>,
    Path(address): Path<String>,
    headers: HeaderMap,
) -> Result<Json<DeepResearchStatusResponse>, AppError> {
    let provider = deep_research_provider_label(state.config.deep_research_provider);
    let report_cached = load_cached_report(&state.db, &address, provider)
        .await?
        .is_some();
    let entitlement = match parse_entitlement_from_headers(&headers) {
        Some(secret) => load_active_entitlement(&state.db, &address, secret).await?,
        None => None,
    };
    let premium_state = if !state.config.deep_research_enabled {
        "disabled"
    } else if entitlement.is_some() && report_cached {
        "report_ready"
    } else if entitlement.is_some() {
        "unlocked"
    } else if !state.config.x402_enabled {
        "preview_only"
    } else {
        "locked"
    };

    Ok(Json(DeepResearchStatusResponse {
        token_address: address,
        premium_state: premium_state.to_string(),
        provider_path: provider.to_string(),
        unlock_model: state.config.deep_research_unlock_model.as_str().to_string(),
        x402_enabled: state.config.x402_enabled,
        native_x_api_reserved: true,
        report_cached,
        has_active_entitlement: entitlement.is_some(),
        entitlement_expires_at: entitlement.as_ref().and_then(|record| record.expires_at),
        notes: vec![
            "Premium depth stays separate from the free workflow.".to_string(),
            "The premium core is launch intelligence, not contract review.".to_string(),
            "Narrative enrichment is optional and can be missing without breaking the report."
                .to_string(),
        ],
    }))
}

pub async fn get_deep_research_report(
    State(state): State<AppState>,
    Path(address): Path<String>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    if !state.config.deep_research_enabled {
        return Err(AppError::FeatureDisabled(
            "Deep Research is disabled on this deployment.".to_string(),
        ));
    }

    let entitlement = match parse_entitlement_from_headers(&headers) {
        Some(secret) => load_active_entitlement(&state.db, &address, secret).await?,
        None => None,
    };

    let Some(entitlement) = entitlement else {
        return build_payment_required_response(
            &state.config,
            &resource_url(&state.config, &address),
        );
    };

    let cached = generate_or_load_report(&state, &address).await?;
    build_report_response(&address, &state.config, cached, Some(&entitlement), None)
}

pub async fn create_deep_research_run(
    State(state): State<AppState>,
    Path(address): Path<String>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    if !state.config.deep_research_enabled {
        return Err(AppError::FeatureDisabled(
            "Deep Research is disabled on this deployment.".to_string(),
        ));
    }

    let entitlement = match parse_entitlement_from_headers(&headers) {
        Some(secret) => load_active_entitlement(&state.db, &address, secret).await?,
        None => None,
    };

    let Some(_entitlement) = entitlement else {
        return build_payment_required_response(
            &state.config,
            &runs_resource_url(&state.config, &address),
        );
    };

    Ok(Json(create_research_run(&state, &address).await?).into_response())
}

pub async fn get_deep_research_run(
    State(state): State<AppState>,
    Path((address, run_id)): Path<(String, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<DeepResearchRunResponse>, AppError> {
    let entitlement = match parse_entitlement_from_headers(&headers) {
        Some(secret) => load_active_entitlement(&state.db, &address, secret).await?,
        None => None,
    };

    if entitlement.is_none() {
        return Err(AppError::PaymentRequired(
            "Active Deep Research entitlement is required to access a run.".to_string(),
        ));
    }

    load_research_run(&state.db, &address, run_id)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound("Deep Research run not found.".to_string()))
}

pub async fn get_deep_research_run_trace(
    State(state): State<AppState>,
    Path((address, run_id)): Path<(String, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<DeepResearchRunTraceResponse>, AppError> {
    let entitlement = match parse_entitlement_from_headers(&headers) {
        Some(secret) => load_active_entitlement(&state.db, &address, secret).await?,
        None => None,
    };

    if entitlement.is_none() {
        return Err(AppError::PaymentRequired(
            "Active Deep Research entitlement is required to access a run trace.".to_string(),
        ));
    }

    load_research_run_trace(&state.db, &address, run_id)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound("Deep Research run trace not found.".to_string()))
}

pub async fn get_deep_research_run_report(
    State(state): State<AppState>,
    Path((address, run_id)): Path<(String, Uuid)>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let entitlement = match parse_entitlement_from_headers(&headers) {
        Some(secret) => load_active_entitlement(&state.db, &address, secret).await?,
        None => None,
    };

    let Some(entitlement) = entitlement else {
        return Err(AppError::PaymentRequired(
            "Active Deep Research entitlement is required to access a run report.".to_string(),
        ));
    };

    let run = load_research_run(&state.db, &address, run_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Deep Research run not found.".to_string()))?;
    if !run.report_ready {
        return Err(AppError::NotReady(
            "Deep Research run is not ready yet.".to_string(),
        ));
    }

    let cached = generate_or_load_report(&state, &address).await?;
    build_report_response(&address, &state.config, cached, Some(&entitlement), None)
}

pub async fn unlock_deep_research(
    State(state): State<AppState>,
    Path(address): Path<String>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    if !state.config.deep_research_enabled {
        return Err(AppError::FeatureDisabled(
            "Deep Research is disabled on this deployment.".to_string(),
        ));
    }
    if !state.config.x402_enabled {
        return Err(AppError::FeatureDisabled(
            "x402 unlocks are disabled on this deployment.".to_string(),
        ));
    }

    if !headers.contains_key(service::PAYMENT_SIGNATURE_HEADER) {
        return build_payment_required_response(
            &state.config,
            &unlock_resource_url(&state.config, &address),
        );
    }

    let (entitlement, settlement) =
        verify_payment_and_issue_entitlement(&state, &address, &headers).await?;
    let cached = generate_or_load_report(&state, &address).await?;
    build_report_response(
        &address,
        &state.config,
        cached,
        Some(&entitlement),
        Some(&settlement),
    )
}

#[cfg(test)]
mod tests {
    use super::build_deep_research_preview;
    use crate::config::{Config, DeepResearchProvider, DeepResearchUnlockModel, MlRolloutMode};

    fn fixture_config() -> Config {
        Config {
            database_url: "postgres://mia:pass@localhost/mia_db".to_string(),
            redis_url: "redis://localhost:6379".to_string(),
            bnb_rpc_ws_url: "wss://example.com/ws".to_string(),
            bnb_rpc_ws_urls: vec!["wss://example.com/ws".to_string()],
            four_meme_contract_address: "0xabc".to_string(),
            app_base_url: "https://mia.example.com".to_string(),
            allowed_origins: vec!["https://mia.example.com".to_string()],
            log_level: "info".to_string(),
            server_port: 8080,
            llm_api_url: "https://llm.example".to_string(),
            llm_api_key: "sk-test".to_string(),
            llm_models: vec!["gpt-5.4".to_string()],
            ai_cache_ttl_secs: 300,
            ai_buy_threshold: 10,
            ai_threshold_window_secs: 600,
            whale_alert_threshold_bnb: 0.5,
            alpha_refresh_secs: 3600,
            alpha_top_k: 10,
            indexer_deployment_backfill_enabled: false,
            auto_investigation_enabled: true,
            auto_investigation_interval_secs: 300,
            auto_investigation_tx_threshold: 100,
            auto_investigation_cooldown_mins: 240,
            auto_investigation_max_runs_per_scan: 3,
            ai_score_min_tx_count: 50,
            auto_deep_research_tx_threshold: 500,
            investigation_fixture_api_enabled: false,
            telegram_bot_token: None,
            telegram_chat_id: None,
            ml_rollout_mode: MlRolloutMode::Shadow,
            ml_model_version: "shadow".to_string(),
            ml_min_confidence: 0.55,
            moralis_api_key: None,
            moralis_api_url: "https://deep-index.moralis.io/api/v2.2".to_string(),
            bscscan_api_key: None,
            bscscan_api_url: "https://api.etherscan.io/v2/api".to_string(),
            bscscan_chain_id: 56,
            deep_research_enabled: true,
            ask_mia_function_calling_enabled: false,
            deep_research_provider: DeepResearchProvider::HeuristMeshX402,
            deep_research_unlock_model: DeepResearchUnlockModel::UnlockThisReport,
            x402_enabled: true,
            x402_facilitator_url: Some("https://facilitator.bankofai.io".to_string()),
            x402_facilitator_api_key: Some("facilitator-key".to_string()),
            x402_pay_to: Some("0xabc123abc123abc123abc123abc123abc123abcd".to_string()),
            x402_asset_address: Some("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".to_string()),
            x402_network: "eip155:56".to_string(),
            x402_scheme: "exact_permit".to_string(),
            x402_facilitator_id: Some("https://facilitator.bankofai.io".to_string()),
            x402_fee_to: Some("0xfee0000000000000000000000000000000000001".to_string()),
            x402_caller: Some("0xfee0000000000000000000000000000000000001".to_string()),
            x402_fee_amount: "0".to_string(),
            x402_price_usdc_cents: 50,
            x402_max_timeout_secs: 60,
            heurist_mesh_api_url: "https://mesh.heurist.xyz".to_string(),
            heurist_mesh_agent_set: "deep_research".to_string(),
            heurist_api_key: Some("heurist-key".to_string()),
            heurist_x402_wallet_dir: Some("/tmp/mia-agent-wallet".to_string()),
            heurist_x402_wallet_id: "mia-base-upstream".to_string(),
            heurist_x402_wallet_password: Some("wallet-pass".to_string()),
        }
    }

    #[test]
    fn preview_contract_freezes_launch_intelligence_scope() {
        let response = build_deep_research_preview("0x123", &fixture_config());

        assert_eq!(
            response.provider_path,
            "MIA launch intelligence + optional narrative enrichment"
        );
        assert_eq!(response.payment_network, "eip155:56");
        assert_eq!(response.unlock_model, "unlock_this_report");
        assert_eq!(response.unlock_cta, "Unlock this report");
        assert!(response
            .notes
            .iter()
            .any(|item| item.contains("launch intelligence")));
    }

    #[test]
    fn resource_path_targets_token_specific_report() {
        assert_eq!(
            super::types::deep_research_resource_path("0x123"),
            "/api/v1/tokens/0x123/deep-research"
        );
    }

    #[test]
    fn runs_path_targets_token_specific_report() {
        assert_eq!(
            super::types::deep_research_runs_path("0x123"),
            "/api/v1/tokens/0x123/deep-research/runs"
        );
    }

    #[test]
    fn runs_resource_url_uses_configured_base_url() {
        let config = fixture_config();
        assert_eq!(
            super::types::runs_resource_url(&config, "0x123"),
            "https://mia.example.com/api/v1/tokens/0x123/deep-research/runs"
        );
    }

    #[test]
    fn resource_url_uses_configured_base_url() {
        let config = fixture_config();
        assert_eq!(
            super::types::resource_url(&config, "0x123"),
            "https://mia.example.com/api/v1/tokens/0x123/deep-research"
        );
        assert_eq!(
            super::types::unlock_resource_url(&config, "0x123"),
            "https://mia.example.com/api/v1/tokens/0x123/deep-research/unlock"
        );
    }
}
