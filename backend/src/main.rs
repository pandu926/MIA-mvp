mod ai;
mod api;
mod config;
mod db;
mod error;
mod indexer;
mod phase4;
mod research;
#[allow(unused_imports)]
mod risk;
mod ws;

use anyhow::Result;
use config::Config;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub redis: redis::aio::ConnectionManager,
    pub config: Arc<Config>,
    /// Broadcast hub for connected WebSocket clients.
    pub ws_hub: ws::WsBroadcastHub,
    /// Channel to submit tokens for AI narrative analysis.
    pub ai_queue_tx: mpsc::Sender<ai::queue::AiJob>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Arc::new(Config::from_env()?);

    // Setup tracing
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.log_level)),
        )
        .json()
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    tracing::info!("MIA backend starting up");

    // Database pool
    let db_pool = db::create_pool(&config.database_url).await?;
    phase4::bootstrap::ensure_phase4_schema(&db_pool).await?;

    // Redis connection manager
    let redis_client = redis::Client::open(config.redis_url.as_str())?;
    let redis_manager = redis::aio::ConnectionManager::new(redis_client).await?;
    tracing::info!("Redis connection established");

    // WebSocket broadcast hub
    let ws_hub = ws::WsBroadcastHub::new();

    // AI job queue (bounded at 100 jobs)
    let (ai_queue_tx, ai_queue_rx) = ai::queue::create_queue(100);

    // Shared application state
    let state = AppState {
        db: db_pool.clone(),
        redis: redis_manager.clone(),
        config: Arc::clone(&config),
        ws_hub: ws_hub.clone(),
        ai_queue_tx: ai_queue_tx.clone(),
    };

    // Cancellation token for graceful shutdown
    let cancel = CancellationToken::new();

    // Spawn AI worker as background task
    let ai_worker = ai::worker::AiWorker::new(
        ai_queue_rx,
        redis_manager,
        db_pool.clone(),
        Arc::clone(&config),
        ws_hub,
    );
    tokio::spawn(async move {
        ai_worker.run().await;
    });

    let auto_investigation_settings =
        research::auto_investigation::AutoInvestigationSettings::from_config(&config);
    tokio::spawn(
        research::auto_investigation::run_auto_investigation_scheduler(
            state.db.clone(),
            auto_investigation_settings,
        ),
    );

    // Spawn hourly alpha scheduler (Phase 4).
    let alpha_scheduler = Arc::new(phase4::alpha::AlphaScheduler::new(
        state.db.clone(),
        config.alpha_refresh_secs,
        config.alpha_top_k,
        phase4::telegram::TelegramConfig {
            bot_token: config.telegram_bot_token.clone(),
            chat_id: config.telegram_chat_id.clone(),
        },
        config.ml_rollout_mode,
        config.ml_model_version.clone(),
        config.ml_min_confidence,
    ));
    tokio::spawn({
        let scheduler = Arc::clone(&alpha_scheduler);
        async move {
            scheduler.run().await;
        }
    });

    // Spawn block listener as background task
    let listener = indexer::BlockListener::new(
        Arc::clone(&config),
        db_pool,
        ai_queue_tx,
        state.ws_hub.clone(),
    );
    let listener_cancel = cancel.clone();
    let listener_handle = tokio::spawn(async move {
        if let Err(e) = listener.start(listener_cancel).await {
            tracing::error!("BlockListener fatal error: {}", e);
        }
    });

    // Build and start Axum server
    let router = api::create_router(state);
    let addr = format!("0.0.0.0:{}", config.server_port);
    let listener_tcp = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!(address = %addr, "Server listening");

    axum::serve(listener_tcp, router)
        .with_graceful_shutdown(shutdown_signal(cancel))
        .await?;

    listener_handle.await?;
    tracing::info!("MIA backend shut down cleanly");

    Ok(())
}

async fn shutdown_signal(cancel: CancellationToken) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }

    tracing::info!("Shutdown signal received");
    cancel.cancel();
}
