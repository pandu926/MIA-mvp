use anyhow::{Context, Result};

use super::types::{
    Config, DeepResearchProvider, DeepResearchUnlockModel, MlRolloutMode, DEFAULT_LLM_MODELS,
};

const DEFAULT_PUBLIC_BNB_RPC_WS_URLS: &[&str] = &[
    "wss://bsc-rpc.publicnode.com",
    "wss://bsc.drpc.org",
    "wss://bnb.api.onfinality.io/public-ws",
];

impl Config {
    pub fn from_env() -> Result<Self> {
        #[cfg(not(test))]
        dotenvy::dotenv().ok();

        let llm_models_raw =
            std::env::var("LLM_MODELS").unwrap_or_else(|_| DEFAULT_LLM_MODELS.to_string());

        let llm_models: Vec<String> = llm_models_raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if llm_models.is_empty() {
            return Err(anyhow::anyhow!(
                "LLM_MODELS must contain at least one model name"
            ));
        }

        let ml_rollout_mode_raw =
            std::env::var("ML_ROLLOUT_MODE").unwrap_or_else(|_| "shadow".to_string());
        let ml_rollout_mode = MlRolloutMode::parse(&ml_rollout_mode_raw).ok_or_else(|| {
            anyhow::anyhow!(
                "ML_ROLLOUT_MODE must be one of: legacy|shadow|ml|hybrid (got: {ml_rollout_mode_raw})"
            )
        })?;

        let deep_research_provider_raw = std::env::var("DEEP_RESEARCH_PROVIDER")
            .unwrap_or_else(|_| "heurist_mesh_x402".to_string());
        let deep_research_provider =
            DeepResearchProvider::parse(&deep_research_provider_raw).ok_or_else(|| {
                anyhow::anyhow!(
                    "DEEP_RESEARCH_PROVIDER must be one of: heurist_mesh_x402|native_x_api (got: {deep_research_provider_raw})"
                )
            })?;

        let deep_research_unlock_model_raw = std::env::var("DEEP_RESEARCH_UNLOCK_MODEL")
            .unwrap_or_else(|_| "unlock_this_report".to_string());
        let deep_research_unlock_model =
            DeepResearchUnlockModel::parse(&deep_research_unlock_model_raw).ok_or_else(|| {
                anyhow::anyhow!(
                    "DEEP_RESEARCH_UNLOCK_MODEL must be one of: unlock_this_report|day_pass (got: {deep_research_unlock_model_raw})"
                )
            })?;

        let deep_research_enabled = parse_bool_env("DEEP_RESEARCH_ENABLED", false)?;
        let ask_mia_function_calling_enabled =
            parse_bool_env("ASK_MIA_FUNCTION_CALLING_ENABLED", false)?;
        let x402_enabled = parse_bool_env("X402_ENABLED", false)?;
        let indexer_deployment_backfill_enabled =
            parse_bool_env("INDEXER_DEPLOYMENT_BACKFILL_ENABLED", false)?;
        let auto_investigation_enabled = parse_bool_env("AUTO_INVESTIGATION_ENABLED", true)?;
        let investigation_fixture_api_enabled =
            parse_bool_env("INVESTIGATION_FIXTURE_API_ENABLED", false)?;

        let x402_facilitator_url = optional_trimmed_env("X402_FACILITATOR_URL");
        let x402_facilitator_api_key = optional_trimmed_env("X402_FACILITATOR_API_KEY");
        let x402_pay_to = optional_trimmed_env("X402_PAY_TO");
        let x402_asset_address = optional_trimmed_env("X402_ASSET_ADDRESS");
        let x402_network =
            std::env::var("X402_NETWORK").unwrap_or_else(|_| "eip155:56".to_string());
        let x402_scheme =
            std::env::var("X402_SCHEME").unwrap_or_else(|_| "exact_permit".to_string());
        let x402_facilitator_id =
            optional_trimmed_env("X402_FACILITATOR_ID").or_else(|| x402_facilitator_url.clone());
        let x402_fee_to = optional_trimmed_env("X402_FEE_TO");
        let x402_caller = optional_trimmed_env("X402_CALLER").or_else(|| x402_fee_to.clone());
        let x402_fee_amount = std::env::var("X402_FEE_AMOUNT").unwrap_or_else(|_| "0".to_string());
        let x402_price_usdc_cents = parse_env_u32("X402_PRICE_USDC_CENTS", 50)?;
        let x402_max_timeout_secs = parse_env_u32("X402_MAX_TIMEOUT_SECS", 60)?;
        let heurist_mesh_api_url = std::env::var("HEURIST_MESH_API_URL")
            .unwrap_or_else(|_| "https://mesh.heurist.xyz".to_string());
        let heurist_mesh_agent_set =
            std::env::var("HEURIST_MESH_AGENT_SET").unwrap_or_else(|_| "deep_research".to_string());
        let heurist_api_key = optional_trimmed_env("HEURIST_API_KEY");
        let heurist_x402_wallet_dir = optional_trimmed_env("HEURIST_X402_WALLET_DIR")
            .or_else(|| optional_trimmed_env("AGENT_WALLET_DIR"));
        let heurist_x402_wallet_id = std::env::var("HEURIST_X402_WALLET_ID")
            .unwrap_or_else(|_| "mia-base-upstream".to_string());
        let heurist_x402_wallet_password = optional_trimmed_env("HEURIST_X402_WALLET_PASSWORD")
            .or_else(|| optional_trimmed_env("AGENT_WALLET_PASSWORD"));
        let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
        let redis_url = std::env::var("REDIS_URL").context("REDIS_URL must be set")?;
        let bnb_rpc_ws_url =
            std::env::var("BNB_RPC_WS_URL").context("BNB_RPC_WS_URL must be set")?;
        let bnb_rpc_ws_urls = parse_rpc_url_list(&bnb_rpc_ws_url);

        if x402_enabled && x402_facilitator_url.is_none() {
            return Err(anyhow::anyhow!(
                "X402_FACILITATOR_URL must be set when X402_ENABLED=true"
            ));
        }
        if x402_enabled && x402_pay_to.is_none() {
            return Err(anyhow::anyhow!(
                "X402_PAY_TO must be set when X402_ENABLED=true"
            ));
        }
        if x402_enabled && x402_asset_address.is_none() {
            return Err(anyhow::anyhow!(
                "X402_ASSET_ADDRESS must be set when X402_ENABLED=true"
            ));
        }

        Ok(Self {
            database_url,
            redis_url,
            bnb_rpc_ws_url,
            bnb_rpc_ws_urls,
            four_meme_contract_address: std::env::var("FOUR_MEME_CONTRACT_ADDRESS")
                .context("FOUR_MEME_CONTRACT_ADDRESS must be set")?,
            app_base_url: std::env::var("APP_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:3313".to_string())
                .trim_end_matches('/')
                .to_string(),
            allowed_origins: build_allowed_origins(),
            log_level: std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            server_port: std::env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .context("SERVER_PORT must be a valid port number")?,
            llm_api_url: std::env::var("LLM_API_URL").context("LLM_API_URL must be set")?,
            llm_api_key: std::env::var("LLM_API_KEY").context("LLM_API_KEY must be set")?,
            llm_models,
            ai_cache_ttl_secs: parse_env_u64("AI_CACHE_TTL_SECS", 300)?,
            ai_buy_threshold: parse_env_u64("AI_BUY_THRESHOLD", 10)?,
            ai_threshold_window_secs: parse_env_u64("AI_THRESHOLD_WINDOW_SECS", 600)?,
            whale_alert_threshold_bnb: parse_env_f64("WHALE_ALERT_THRESHOLD_BNB", 0.5)?,
            alpha_refresh_secs: parse_env_u64("ALPHA_REFRESH_SECS", 3600)?,
            alpha_top_k: parse_env_i64("ALPHA_TOP_K", 10)?,
            indexer_deployment_backfill_enabled,
            auto_investigation_enabled,
            auto_investigation_interval_secs: parse_env_u64(
                "AUTO_INVESTIGATION_INTERVAL_SECS",
                300,
            )?,
            auto_investigation_tx_threshold: parse_env_i64("AUTO_INVESTIGATION_TX_THRESHOLD", 100)?,
            auto_investigation_cooldown_mins: parse_env_i64(
                "AUTO_INVESTIGATION_COOLDOWN_MINS",
                240,
            )?,
            auto_investigation_max_runs_per_scan: parse_env_i64(
                "AUTO_INVESTIGATION_MAX_RUNS_PER_SCAN",
                3,
            )?,
            ai_score_min_tx_count: parse_env_i64("AI_SCORE_MIN_TX_COUNT", 50)?,
            auto_deep_research_tx_threshold: parse_env_i64("AUTO_DEEP_RESEARCH_TX_THRESHOLD", 500)?,
            investigation_fixture_api_enabled,
            telegram_bot_token: std::env::var("TELEGRAM_BOT_TOKEN").ok(),
            telegram_chat_id: std::env::var("TELEGRAM_CHAT_ID").ok(),
            ml_rollout_mode,
            ml_model_version: std::env::var("ML_MODEL_VERSION")
                .unwrap_or_else(|_| "lightgbm-shadow-v0".to_string()),
            ml_min_confidence: parse_env_f64("ML_MIN_CONFIDENCE", 0.55)?,
            moralis_api_key: std::env::var("MORALIS_API_KEY").ok(),
            moralis_api_url: std::env::var("MORALIS_API_URL")
                .unwrap_or_else(|_| "https://deep-index.moralis.io/api/v2.2".to_string()),
            bscscan_api_key: std::env::var("BSCSCAN_API_KEY").ok(),
            bscscan_api_url: std::env::var("BSCSCAN_API_URL")
                .unwrap_or_else(|_| "https://api.etherscan.io/v2/api".to_string()),
            bscscan_chain_id: parse_env_u64("BSCSCAN_CHAIN_ID", 56)?,
            deep_research_enabled,
            ask_mia_function_calling_enabled,
            deep_research_provider,
            deep_research_unlock_model,
            x402_enabled,
            x402_facilitator_url,
            x402_facilitator_api_key,
            x402_pay_to,
            x402_asset_address,
            x402_network,
            x402_scheme,
            x402_facilitator_id,
            x402_fee_to,
            x402_caller,
            x402_fee_amount,
            x402_price_usdc_cents,
            x402_max_timeout_secs,
            heurist_mesh_api_url,
            heurist_mesh_agent_set,
            heurist_api_key,
            heurist_x402_wallet_dir,
            heurist_x402_wallet_id,
            heurist_x402_wallet_password,
        })
    }
}

fn parse_rpc_url_list(primary_url: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let mut push_unique = |value: String| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        if !urls.iter().any(|existing| existing == &trimmed) {
            urls.push(trimmed);
        }
    };

    push_unique(primary_url.to_string());

    if let Some(raw) = optional_trimmed_env("BNB_RPC_WS_URLS") {
        for item in raw.split(',') {
            push_unique(item.to_string());
        }
    }

    if let Some(raw) = optional_trimmed_env("BNB_RPC_PUBLIC_WS_URLS") {
        for item in raw.split(',') {
            push_unique(item.to_string());
        }
    } else {
        for url in DEFAULT_PUBLIC_BNB_RPC_WS_URLS {
            push_unique((*url).to_string());
        }
    }

    urls
}

fn optional_trimmed_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn build_allowed_origins() -> Vec<String> {
    if let Some(raw) = optional_trimmed_env("APP_ALLOWED_ORIGINS") {
        let origins: Vec<String> = raw
            .split(',')
            .map(|origin| origin.trim().trim_end_matches('/').to_string())
            .filter(|origin| !origin.is_empty())
            .collect();

        if !origins.is_empty() {
            return origins;
        }
    }

    let mut origins = vec![
        std::env::var("APP_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:3313".to_string())
            .trim_end_matches('/')
            .to_string(),
        "http://localhost:3313".to_string(),
        "http://127.0.0.1:3313".to_string(),
        "http://localhost:3001".to_string(),
        "http://127.0.0.1:3001".to_string(),
    ];
    origins.sort();
    origins.dedup();
    origins
}

fn parse_env_u64(key: &str, default: u64) -> Result<u64> {
    std::env::var(key)
        .unwrap_or_else(|_| default.to_string())
        .parse()
        .with_context(|| format!("{key} must be a valid integer"))
}

fn parse_env_u32(key: &str, default: u32) -> Result<u32> {
    std::env::var(key)
        .unwrap_or_else(|_| default.to_string())
        .parse()
        .with_context(|| format!("{key} must be a valid integer"))
}

fn parse_env_i64(key: &str, default: i64) -> Result<i64> {
    std::env::var(key)
        .unwrap_or_else(|_| default.to_string())
        .parse()
        .with_context(|| format!("{key} must be a valid integer"))
}

fn parse_env_f64(key: &str, default: f64) -> Result<f64> {
    std::env::var(key)
        .unwrap_or_else(|_| default.to_string())
        .parse()
        .with_context(|| format!("{key} must be a valid float"))
}

fn parse_bool_env(key: &str, default: bool) -> Result<bool> {
    match std::env::var(key) {
        Ok(raw) => match raw.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            other => Err(anyhow::anyhow!(
                "{key} must be a boolean-like value (got: {other})"
            )),
        },
        Err(_) => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn set_required_vars() {
        env::set_var("DATABASE_URL", "postgres://mia:pass@localhost/mia_db");
        env::set_var("REDIS_URL", "redis://localhost:6379");
        env::set_var("BNB_RPC_WS_URL", "wss://example.com/ws");
        env::remove_var("BNB_RPC_WS_URLS");
        env::remove_var("BNB_RPC_PUBLIC_WS_URLS");
        env::set_var("FOUR_MEME_CONTRACT_ADDRESS", "0xABCDEF");
        env::remove_var("APP_BASE_URL");
        env::remove_var("APP_ALLOWED_ORIGINS");
        env::set_var(
            "LLM_API_URL",
            "http://127.0.0.1:8317",
        );
        env::set_var("LLM_API_KEY", "sk-test-key-123");
        env::remove_var("LOG_LEVEL");
        env::remove_var("SERVER_PORT");
        env::remove_var("LLM_MODELS");
        env::remove_var("AI_CACHE_TTL_SECS");
        env::remove_var("AI_BUY_THRESHOLD");
        env::remove_var("AI_THRESHOLD_WINDOW_SECS");
        env::remove_var("WHALE_ALERT_THRESHOLD_BNB");
        env::remove_var("ALPHA_REFRESH_SECS");
        env::remove_var("ALPHA_TOP_K");
        env::remove_var("INDEXER_DEPLOYMENT_BACKFILL_ENABLED");
        env::remove_var("AUTO_INVESTIGATION_ENABLED");
        env::remove_var("AUTO_INVESTIGATION_INTERVAL_SECS");
        env::remove_var("AUTO_INVESTIGATION_TX_THRESHOLD");
        env::remove_var("AUTO_INVESTIGATION_COOLDOWN_MINS");
        env::remove_var("AUTO_INVESTIGATION_MAX_RUNS_PER_SCAN");
        env::remove_var("AI_SCORE_MIN_TX_COUNT");
        env::remove_var("AUTO_DEEP_RESEARCH_TX_THRESHOLD");
        env::remove_var("INVESTIGATION_FIXTURE_API_ENABLED");
        env::remove_var("TELEGRAM_BOT_TOKEN");
        env::remove_var("TELEGRAM_CHAT_ID");
        env::remove_var("ML_ROLLOUT_MODE");
        env::remove_var("ML_MODEL_VERSION");
        env::remove_var("ML_MIN_CONFIDENCE");
        env::remove_var("MORALIS_API_KEY");
        env::remove_var("MORALIS_API_URL");
        env::remove_var("BSCSCAN_API_KEY");
        env::remove_var("BSCSCAN_API_URL");
        env::remove_var("BSCSCAN_CHAIN_ID");
        env::remove_var("DEEP_RESEARCH_ENABLED");
        env::remove_var("ASK_MIA_FUNCTION_CALLING_ENABLED");
        env::remove_var("DEEP_RESEARCH_PROVIDER");
        env::remove_var("DEEP_RESEARCH_UNLOCK_MODEL");
        env::remove_var("X402_ENABLED");
        env::remove_var("X402_FACILITATOR_URL");
        env::remove_var("X402_PAY_TO");
        env::remove_var("X402_ASSET_ADDRESS");
        env::remove_var("X402_NETWORK");
        env::remove_var("X402_PRICE_USDC_CENTS");
        env::remove_var("X402_MAX_TIMEOUT_SECS");
        env::remove_var("HEURIST_MESH_API_URL");
        env::remove_var("HEURIST_MESH_AGENT_SET");
        env::remove_var("HEURIST_API_KEY");
    }

    fn clear_vars() {
        for var in &[
            "DATABASE_URL",
            "REDIS_URL",
            "BNB_RPC_WS_URL",
            "BNB_RPC_WS_URLS",
            "BNB_RPC_PUBLIC_WS_URLS",
            "FOUR_MEME_CONTRACT_ADDRESS",
            "APP_BASE_URL",
            "APP_ALLOWED_ORIGINS",
            "LOG_LEVEL",
            "SERVER_PORT",
            "LLM_API_URL",
            "LLM_API_KEY",
            "LLM_MODELS",
            "AI_CACHE_TTL_SECS",
            "AI_BUY_THRESHOLD",
            "AI_THRESHOLD_WINDOW_SECS",
            "WHALE_ALERT_THRESHOLD_BNB",
            "ALPHA_REFRESH_SECS",
            "ALPHA_TOP_K",
            "INDEXER_DEPLOYMENT_BACKFILL_ENABLED",
            "AUTO_INVESTIGATION_ENABLED",
            "AUTO_INVESTIGATION_INTERVAL_SECS",
            "AUTO_INVESTIGATION_TX_THRESHOLD",
            "AUTO_INVESTIGATION_COOLDOWN_MINS",
            "AUTO_INVESTIGATION_MAX_RUNS_PER_SCAN",
            "AI_SCORE_MIN_TX_COUNT",
            "AUTO_DEEP_RESEARCH_TX_THRESHOLD",
            "TELEGRAM_BOT_TOKEN",
            "TELEGRAM_CHAT_ID",
            "ML_ROLLOUT_MODE",
            "ML_MODEL_VERSION",
            "ML_MIN_CONFIDENCE",
            "MORALIS_API_KEY",
            "MORALIS_API_URL",
            "BSCSCAN_API_KEY",
            "BSCSCAN_API_URL",
            "BSCSCAN_CHAIN_ID",
            "DEEP_RESEARCH_ENABLED",
            "ASK_MIA_FUNCTION_CALLING_ENABLED",
            "DEEP_RESEARCH_PROVIDER",
            "DEEP_RESEARCH_UNLOCK_MODEL",
            "X402_ENABLED",
            "X402_FACILITATOR_URL",
            "X402_PAY_TO",
            "X402_ASSET_ADDRESS",
            "X402_NETWORK",
            "X402_PRICE_USDC_CENTS",
            "X402_MAX_TIMEOUT_SECS",
            "HEURIST_MESH_API_URL",
            "HEURIST_MESH_AGENT_SET",
            "HEURIST_API_KEY",
        ] {
            env::remove_var(var);
        }
    }

    #[test]
    fn loads_config_from_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_vars();
        set_required_vars();

        let config = Config::from_env().expect("should load config");
        assert_eq!(config.database_url, "postgres://mia:pass@localhost/mia_db");
        assert_eq!(config.redis_url, "redis://localhost:6379");
        assert_eq!(
            config.bnb_rpc_ws_urls,
            vec![
                "wss://example.com/ws".to_string(),
                "wss://bsc-rpc.publicnode.com".to_string(),
                "wss://bsc.drpc.org".to_string(),
                "wss://bnb.api.onfinality.io/public-ws".to_string(),
            ]
        );
        assert_eq!(config.four_meme_contract_address, "0xABCDEF");
        assert_eq!(config.app_base_url, "http://localhost:3313");
        assert!(config
            .allowed_origins
            .contains(&"http://localhost:3313".to_string()));
        assert_eq!(
            config.llm_api_url,
            "http://127.0.0.1:8317"
        );
        assert_eq!(config.llm_api_key, "sk-test-key-123");
        assert!(config.auto_investigation_enabled);
        assert!(!config.indexer_deployment_backfill_enabled);
        assert_eq!(config.auto_investigation_interval_secs, 300);
        assert_eq!(config.auto_investigation_tx_threshold, 100);
        assert_eq!(config.auto_investigation_cooldown_mins, 240);
        assert_eq!(config.auto_investigation_max_runs_per_scan, 3);
        assert_eq!(config.ai_score_min_tx_count, 50);
        assert_eq!(config.auto_deep_research_tx_threshold, 500);
        assert!(config.moralis_api_key.is_none());
        assert_eq!(
            config.moralis_api_url,
            "https://deep-index.moralis.io/api/v2.2"
        );
        assert!(!config.investigation_fixture_api_enabled);
        clear_vars();
    }

    #[test]
    fn errors_when_database_url_missing() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_vars();
        let result = Config::from_env();
        assert!(result.is_err(), "should fail without DATABASE_URL");
        assert!(result.unwrap_err().to_string().contains("DATABASE_URL"));
    }

    #[test]
    fn errors_when_llm_api_url_missing() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_vars();
        set_required_vars();
        env::remove_var("LLM_API_URL");
        let result = Config::from_env();
        assert!(result.is_err(), "should fail without LLM_API_URL");
        assert!(result.unwrap_err().to_string().contains("LLM_API_URL"));
        clear_vars();
    }

    #[test]
    fn errors_when_llm_api_key_missing() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_vars();
        set_required_vars();
        env::remove_var("LLM_API_KEY");
        let result = Config::from_env();
        assert!(result.is_err(), "should fail without LLM_API_KEY");
        assert!(result.unwrap_err().to_string().contains("LLM_API_KEY"));
        clear_vars();
    }

    #[test]
    fn llm_models_default_to_gpt_pool() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_vars();
        set_required_vars();
        let config = Config::from_env().unwrap();
        assert!(!config.llm_models.is_empty());
        assert_eq!(config.llm_models[0], "gpt-5.4");
        assert!(config.llm_models.len() >= 3);
        clear_vars();
    }

    #[test]
    fn llm_models_can_be_overridden_via_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_vars();
        set_required_vars();
        env::set_var("LLM_MODELS", "gpt-5.2, gpt-5.4-mini");
        let config = Config::from_env().unwrap();
        assert_eq!(
            config.llm_models,
            vec!["gpt-5.2", "gpt-5.4-mini"]
        );
        clear_vars();
    }

    #[test]
    fn errors_when_llm_models_is_empty_string() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_vars();
        set_required_vars();
        env::set_var("LLM_MODELS", "  ,  ,  ");
        let result = Config::from_env();
        assert!(result.is_err(), "should fail with empty model list");
        clear_vars();
    }

    #[test]
    fn phase4_defaults_are_loaded() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_vars();
        set_required_vars();
        let config = Config::from_env().unwrap();
        assert!((config.whale_alert_threshold_bnb - 0.5).abs() < f64::EPSILON);
        assert_eq!(config.alpha_refresh_secs, 3600);
        assert_eq!(config.alpha_top_k, 10);
        assert_eq!(config.ml_rollout_mode, MlRolloutMode::Shadow);
        assert_eq!(config.ml_model_version, "lightgbm-shadow-v0");
        assert!((config.ml_min_confidence - 0.55).abs() < f64::EPSILON);
        clear_vars();
    }

    #[test]
    fn investigation_fixture_api_can_be_enabled_via_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_vars();
        set_required_vars();
        env::set_var("INVESTIGATION_FIXTURE_API_ENABLED", "true");
        let config = Config::from_env().unwrap();
        assert!(config.investigation_fixture_api_enabled);
        clear_vars();
    }

    #[test]
    fn deployment_backfill_can_be_enabled_via_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_vars();
        set_required_vars();
        env::set_var("INDEXER_DEPLOYMENT_BACKFILL_ENABLED", "true");
        let config = Config::from_env().unwrap();
        assert!(config.indexer_deployment_backfill_enabled);
        clear_vars();
    }

    #[test]
    fn premium_config_requires_facilitator_url_when_x402_is_enabled() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_vars();
        set_required_vars();
        env::set_var("X402_ENABLED", "true");
        let result = Config::from_env();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("X402_FACILITATOR_URL"));
        clear_vars();
    }

    #[test]
    fn rpc_ws_urls_include_primary_and_fallbacks_without_duplicates() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_vars();
        set_required_vars();
        env::set_var(
            "BNB_RPC_WS_URLS",
            "wss://example.com/ws,wss://fallback-1/ws,wss://fallback-2/ws",
        );
        env::set_var(
            "BNB_RPC_PUBLIC_WS_URLS",
            "wss://public-1/ws,wss://public-2/ws,wss://fallback-2/ws",
        );

        let config = Config::from_env().unwrap();
        assert_eq!(
            config.bnb_rpc_ws_urls,
            vec![
                "wss://example.com/ws".to_string(),
                "wss://fallback-1/ws".to_string(),
                "wss://fallback-2/ws".to_string(),
                "wss://public-1/ws".to_string(),
                "wss://public-2/ws".to_string(),
            ]
        );
        clear_vars();
    }

    #[test]
    fn rpc_ws_urls_append_default_public_fallbacks_when_not_overridden() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_vars();
        set_required_vars();

        let config = Config::from_env().unwrap();
        assert!(config
            .bnb_rpc_ws_urls
            .contains(&"wss://bsc-rpc.publicnode.com".to_string()));
        assert!(config
            .bnb_rpc_ws_urls
            .contains(&"wss://bsc.drpc.org".to_string()));
        assert!(config
            .bnb_rpc_ws_urls
            .contains(&"wss://bnb.api.onfinality.io/public-ws".to_string()));
        clear_vars();
    }
}
