use crate::{
    api::{
        auto_investigation::post_auto_investigation_scan,
        deep_research::{
            create_deep_research_run, get_deep_research_report, get_deep_research_run,
            get_deep_research_run_report, get_deep_research_run_trace, get_deep_research_status,
            preview_deep_research, unlock_deep_research,
        },
        deployer::get_deployer,
        health::health_handler,
        intelligence::get_intelligence_summary,
        investigation::{get_token_investigation, post_token_ask_mia},
        investigation_fixtures::{
            post_failed_run_fixture, post_monitoring_downgrade_fixture,
            post_non_transaction_escalation_fixture, post_stale_running_fixture,
        },
        investigation_missions::{
            create_investigation_mission, list_investigation_missions, update_investigation_mission,
        },
        investigation_ops::{
            archive_stale_investigation_runs, get_investigation_ops_summary,
            recover_stale_running_investigation_runs, retry_failed_investigation_runs,
            update_investigation_ops_control,
        },
        investigation_runs::{
            get_investigation_run, get_investigation_run_detail, list_investigation_runs,
            update_investigation_run_status,
        },
        investigation_watchlist::{
            create_investigation_watchlist_item, delete_investigation_watchlist_item,
            list_investigation_watchlist,
        },
        ml::{activate_model, get_ml_alpha_eval, get_ml_decision, get_ml_health, list_models},
        narratives::get_token_narrative,
        payments::verify_x402_payment,
        phase4::{
            get_alpha_backtest, get_alpha_history, get_deployer_tokens, get_latest_alpha,
            get_telegram_config, list_whale_alerts, telegram_webhook, update_telegram_config,
            whale_network, whale_stream,
        },
        tokens::{get_token, get_token_risk, get_token_transactions, list_tokens},
        verdict::get_token_verdict,
        wallets::get_wallet_intel,
    },
    ws::handler::ws_handler,
    AppState,
};
use axum::{
    http::{
        header::{HeaderName, ACCEPT, AUTHORIZATION, CONTENT_TYPE},
        HeaderValue, Method,
    },
    routing::{delete, get, patch, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

fn build_cors_layer(allowed_origins: &[String]) -> CorsLayer {
    let origin_values: Vec<HeaderValue> = allowed_origins
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect();

    let mut cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            CONTENT_TYPE,
            ACCEPT,
            AUTHORIZATION,
            HeaderName::from_static("payment-signature"),
        ]);

    if !origin_values.is_empty() {
        cors = cors.allow_origin(origin_values);
    }

    cors
}

pub fn create_router(state: AppState) -> Router {
    let cors = build_cors_layer(&state.config.allowed_origins);
    let api_v1 = Router::new()
        .route("/tokens", get(list_tokens))
        .route("/tokens/{address}", get(get_token))
        .route("/tokens/{address}/risk", get(get_token_risk))
        .route(
            "/tokens/{address}/transactions",
            get(get_token_transactions),
        )
        .route("/tokens/{address}/narrative", get(get_token_narrative))
        .route("/tokens/{address}/verdict", get(get_token_verdict))
        .route(
            "/tokens/{address}/investigation",
            get(get_token_investigation),
        )
        .route("/investigations/runs", get(list_investigation_runs))
        .route(
            "/investigations/watchlist",
            get(list_investigation_watchlist).post(create_investigation_watchlist_item),
        )
        .route(
            "/investigations/watchlist/{item_id}",
            delete(delete_investigation_watchlist_item),
        )
        .route(
            "/investigations/missions",
            get(list_investigation_missions).post(create_investigation_mission),
        )
        .route(
            "/investigations/missions/{mission_id}",
            patch(update_investigation_mission),
        )
        .route(
            "/investigations/ops/summary",
            get(get_investigation_ops_summary).patch(update_investigation_ops_control),
        )
        .route(
            "/investigations/ops/archive-stale",
            post(archive_stale_investigation_runs),
        )
        .route(
            "/investigations/ops/retry-failed",
            post(retry_failed_investigation_runs),
        )
        .route(
            "/investigations/ops/recover-stale-running",
            post(recover_stale_running_investigation_runs),
        )
        .route(
            "/investigations/auto-scan",
            post(post_auto_investigation_scan),
        )
        .route(
            "/investigations/test-fixtures/non-tx-escalation",
            post(post_non_transaction_escalation_fixture),
        )
        .route(
            "/investigations/test-fixtures/monitoring-downgrade",
            post(post_monitoring_downgrade_fixture),
        )
        .route(
            "/investigations/test-fixtures/failed-run",
            post(post_failed_run_fixture),
        )
        .route(
            "/investigations/test-fixtures/stale-running",
            post(post_stale_running_fixture),
        )
        .route("/investigations/runs/{run_id}", get(get_investigation_run))
        .route(
            "/investigations/runs/{run_id}/detail",
            get(get_investigation_run_detail),
        )
        .route(
            "/investigations/runs/{run_id}/status",
            patch(update_investigation_run_status),
        )
        .route("/tokens/{address}/ask-mia", post(post_token_ask_mia))
        .route(
            "/tokens/{address}/deep-research/preview",
            post(preview_deep_research),
        )
        .route(
            "/tokens/{address}/deep-research/status",
            get(get_deep_research_status),
        )
        .route(
            "/tokens/{address}/deep-research",
            get(get_deep_research_report),
        )
        .route(
            "/tokens/{address}/deep-research/runs",
            post(create_deep_research_run),
        )
        .route(
            "/tokens/{address}/deep-research/runs/{run_id}",
            get(get_deep_research_run),
        )
        .route(
            "/tokens/{address}/deep-research/runs/{run_id}/trace",
            get(get_deep_research_run_trace),
        )
        .route(
            "/tokens/{address}/deep-research/runs/{run_id}/report",
            get(get_deep_research_run_report),
        )
        .route(
            "/tokens/{address}/deep-research/unlock",
            post(unlock_deep_research),
        )
        .route("/deployer/{address}", get(get_deployer))
        .route("/deployer/{address}/tokens", get(get_deployer_tokens))
        .route("/whales", get(list_whale_alerts))
        .route("/whales/stream", get(whale_stream))
        .route("/whales/network", get(whale_network))
        .route("/whales/feed", get(list_whale_alerts))
        .route("/alpha/latest", get(get_latest_alpha))
        .route("/alpha/history", get(get_alpha_history))
        .route("/alpha/backtest", get(get_alpha_backtest))
        .route("/ml/health", get(get_ml_health))
        .route("/ml/alpha/eval", get(get_ml_alpha_eval))
        .route("/ml/decision", get(get_ml_decision))
        .route("/ml/models", get(list_models).post(activate_model))
        .route("/intelligence/summary", get(get_intelligence_summary))
        .route("/wallets/{address}/intel", get(get_wallet_intel))
        .route("/x402/verify", post(verify_x402_payment))
        .route(
            "/telegram/config",
            get(get_telegram_config).put(update_telegram_config),
        )
        .route("/telegram/webhook", post(telegram_webhook));

    Router::new()
        .route("/health", get(health_handler))
        .route("/ws", get(ws_handler))
        .nest("/api/v1", api_v1)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
