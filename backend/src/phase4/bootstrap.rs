use anyhow::Result;
use sqlx::PgPool;

/// Ensure Phase 4 schema exists even when DB init scripts were not re-run.
///
/// This makes backend startup self-healing for existing Postgres volumes in
/// production where `/docker-entrypoint-initdb.d` no longer executes.
pub async fn ensure_phase4_schema(db: &PgPool) -> Result<()> {
    let statements = [
        r#"
        CREATE TABLE IF NOT EXISTS whale_alerts (
            id BIGSERIAL PRIMARY KEY,
            token_address VARCHAR(42) NOT NULL,
            wallet_address VARCHAR(42) NOT NULL,
            tx_hash VARCHAR(66) NOT NULL UNIQUE,
            amount_bnb NUMERIC(24, 8) NOT NULL CHECK (amount_bnb >= 0),
            threshold_bnb NUMERIC(24, 8) NOT NULL CHECK (threshold_bnb >= 0),
            alert_level VARCHAR(16) NOT NULL CHECK (alert_level IN ('watch', 'critical')),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_whale_alerts_created_at ON whale_alerts(created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_whale_alerts_token_created ON whale_alerts(token_address, created_at DESC)",
        r#"
        CREATE TABLE IF NOT EXISTS alpha_rankings (
            id BIGSERIAL PRIMARY KEY,
            window_start TIMESTAMPTZ NOT NULL,
            window_end TIMESTAMPTZ NOT NULL,
            rank SMALLINT NOT NULL CHECK (rank >= 1 AND rank <= 100),
            token_address VARCHAR(42) NOT NULL,
            alpha_score NUMERIC(10, 4) NOT NULL,
            rationale TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_alpha_rankings_window ON alpha_rankings(window_end DESC, rank ASC)",
        "CREATE INDEX IF NOT EXISTS idx_alpha_rankings_token ON alpha_rankings(token_address, window_end DESC)",
        r#"
        CREATE TABLE IF NOT EXISTS telegram_delivery_logs (
            id BIGSERIAL PRIMARY KEY,
            channel VARCHAR(32) NOT NULL,
            message_type VARCHAR(32) NOT NULL,
            status VARCHAR(16) NOT NULL CHECK (status IN ('sent', 'skipped', 'failed')),
            payload JSONB NOT NULL,
            error_message TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_telegram_logs_created_at ON telegram_delivery_logs(created_at DESC)",
        r#"
        CREATE TABLE IF NOT EXISTS telegram_runtime_config (
            id SMALLINT PRIMARY KEY DEFAULT 1 CHECK (id = 1),
            enabled BOOLEAN NOT NULL DEFAULT false,
            chat_id TEXT,
            threshold_bnb NUMERIC(24, 8) NOT NULL DEFAULT 0.5 CHECK (threshold_bnb >= 0),
            alpha_digest_enabled BOOLEAN NOT NULL DEFAULT true,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
        r#"
        INSERT INTO telegram_runtime_config (id, enabled, threshold_bnb, alpha_digest_enabled)
        VALUES (1, false, 0.5, true)
        ON CONFLICT (id) DO NOTHING
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS ml_model_registry (
            id BIGSERIAL PRIMARY KEY,
            model_version TEXT NOT NULL UNIQUE,
            rollout_mode VARCHAR(16) NOT NULL CHECK (rollout_mode IN ('legacy', 'shadow', 'ml', 'hybrid')),
            is_active BOOLEAN NOT NULL DEFAULT false,
            metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS ml_alpha_predictions (
            id BIGSERIAL PRIMARY KEY,
            window_end TIMESTAMPTZ NOT NULL,
            token_address VARCHAR(42) NOT NULL,
            score_source VARCHAR(24) NOT NULL,
            model_version TEXT NOT NULL,
            rollout_mode VARCHAR(16) NOT NULL CHECK (rollout_mode IN ('legacy', 'shadow', 'ml', 'hybrid')),
            score NUMERIC(10, 4) NOT NULL,
            confidence NUMERIC(6, 4) NOT NULL CHECK (confidence >= 0 AND confidence <= 1),
            realized_hit_1h BOOLEAN,
            realized_score_1h NUMERIC(10, 4),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            realized_at TIMESTAMPTZ,
            UNIQUE (window_end, token_address, score_source)
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_ml_alpha_predictions_window ON ml_alpha_predictions(window_end DESC)",
        "CREATE INDEX IF NOT EXISTS idx_ml_alpha_predictions_source ON ml_alpha_predictions(score_source, created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_ml_alpha_predictions_realized ON ml_alpha_predictions(realized_hit_1h, created_at DESC)",
        r#"
        CREATE TABLE IF NOT EXISTS ml_pattern_model_registry (
            id BIGSERIAL PRIMARY KEY,
            model_version TEXT NOT NULL UNIQUE,
            model_family VARCHAR(64) NOT NULL CHECK (model_family IN ('lightgbm_similarity_iforest')),
            is_active BOOLEAN NOT NULL DEFAULT false,
            metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS ml_pattern_predictions (
            id BIGSERIAL PRIMARY KEY,
            window_end TIMESTAMPTZ NOT NULL,
            token_address VARCHAR(42) NOT NULL,
            horizon_hours SMALLINT NOT NULL CHECK (horizon_hours IN (1, 6, 24)),
            model_version TEXT NOT NULL,
            match_label VARCHAR(64) NOT NULL,
            outcome_class VARCHAR(64) NOT NULL,
            score NUMERIC(10, 4) NOT NULL,
            confidence NUMERIC(6, 4) NOT NULL CHECK (confidence >= 0 AND confidence <= 1),
            anomaly_score NUMERIC(10, 4),
            expected_path_summary TEXT NOT NULL,
            rationale TEXT NOT NULL,
            analogs JSONB NOT NULL DEFAULT '[]'::jsonb,
            feature_snapshot JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE (window_end, token_address, horizon_hours, model_version)
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_ml_pattern_model_registry_active ON ml_pattern_model_registry(is_active, updated_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_ml_pattern_predictions_token_window ON ml_pattern_predictions(token_address, window_end DESC, horizon_hours)",
        "CREATE INDEX IF NOT EXISTS idx_ml_pattern_predictions_model_horizon ON ml_pattern_predictions(model_version, horizon_hours, created_at DESC)",
        r#"
        CREATE TABLE IF NOT EXISTS payment_attempts (
            id BIGSERIAL PRIMARY KEY,
            token_address VARCHAR(42) NOT NULL,
            resource_path TEXT NOT NULL,
            unlock_model VARCHAR(32) NOT NULL CHECK (unlock_model IN ('unlock_this_report', 'day_pass')),
            provider_path VARCHAR(64) NOT NULL,
            network VARCHAR(32) NOT NULL,
            price_usdc_cents INTEGER NOT NULL CHECK (price_usdc_cents > 0),
            payment_signature_b64 TEXT UNIQUE,
            payment_payload JSONB,
            payment_requirements JSONB NOT NULL,
            verify_response JSONB,
            settle_response JSONB,
            payer_address VARCHAR(128),
            status VARCHAR(24) NOT NULL CHECK (status IN ('required', 'verified', 'settled', 'failed')),
            error_message TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_payment_attempts_token_status ON payment_attempts(token_address, status, created_at DESC)",
        r#"
        CREATE TABLE IF NOT EXISTS entitlements (
            id UUID PRIMARY KEY,
            token_address VARCHAR(42) NOT NULL,
            entitlement_kind VARCHAR(32) NOT NULL CHECK (entitlement_kind IN ('report', 'day_pass')),
            entitlement_secret UUID NOT NULL UNIQUE,
            payer_address VARCHAR(128),
            payment_attempt_id BIGINT REFERENCES payment_attempts(id) ON DELETE SET NULL,
            status VARCHAR(16) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'expired', 'revoked')),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            unlocked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            expires_at TIMESTAMPTZ
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_entitlements_token_status ON entitlements(token_address, status, unlocked_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_entitlements_secret ON entitlements(entitlement_secret)",
        r#"
        CREATE TABLE IF NOT EXISTS deep_research_reports (
            id BIGSERIAL PRIMARY KEY,
            token_address VARCHAR(42) NOT NULL,
            provider_path VARCHAR(64) NOT NULL,
            executive_summary TEXT NOT NULL,
            sections JSONB NOT NULL,
            citations JSONB NOT NULL DEFAULT '[]'::jsonb,
            source_status JSONB NOT NULL DEFAULT '{}'::jsonb,
            raw_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE (token_address, provider_path)
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_deep_research_reports_token_provider ON deep_research_reports(token_address, provider_path)",
        r#"
        CREATE TABLE IF NOT EXISTS deep_research_runs (
            id UUID PRIMARY KEY,
            token_address VARCHAR(42) NOT NULL,
            provider_path VARCHAR(64) NOT NULL,
            status VARCHAR(16) NOT NULL CHECK (status IN ('queued', 'running', 'completed', 'failed')),
            current_phase VARCHAR(32) NOT NULL,
            planner_version VARCHAR(32),
            execution_mode VARCHAR(24),
            budget_usage_cents INTEGER NOT NULL DEFAULT 0,
            paid_calls_count INTEGER NOT NULL DEFAULT 0,
            error_message TEXT,
            report_updated_at TIMESTAMPTZ,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            started_at TIMESTAMPTZ,
            completed_at TIMESTAMPTZ
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_deep_research_runs_token_created ON deep_research_runs(token_address, created_at DESC)",
        "ALTER TABLE deep_research_runs ADD COLUMN IF NOT EXISTS planner_version VARCHAR(32)",
        "ALTER TABLE deep_research_runs ADD COLUMN IF NOT EXISTS execution_mode VARCHAR(24)",
        r#"
        CREATE TABLE IF NOT EXISTS deep_research_run_steps (
            id BIGSERIAL PRIMARY KEY,
            run_id UUID NOT NULL REFERENCES deep_research_runs(id) ON DELETE CASCADE,
            step_key VARCHAR(32) NOT NULL,
            title VARCHAR(128) NOT NULL,
            status VARCHAR(16) NOT NULL CHECK (status IN ('queued', 'running', 'completed', 'failed', 'skipped')),
            agent_name VARCHAR(64),
            tool_name VARCHAR(64),
            summary TEXT,
            evidence JSONB NOT NULL DEFAULT '[]'::jsonb,
            cost_cents INTEGER NOT NULL DEFAULT 0,
            payment_tx VARCHAR(128),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            started_at TIMESTAMPTZ,
            completed_at TIMESTAMPTZ
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_deep_research_run_steps_run_id ON deep_research_run_steps(run_id, id)",
        r#"
        CREATE TABLE IF NOT EXISTS deep_research_tool_calls (
            id BIGSERIAL PRIMARY KEY,
            run_id UUID NOT NULL REFERENCES deep_research_runs(id) ON DELETE CASCADE,
            step_key VARCHAR(32) NOT NULL,
            tool_name VARCHAR(64) NOT NULL,
            provider VARCHAR(64),
            status VARCHAR(16) NOT NULL CHECK (status IN ('queued', 'running', 'completed', 'failed', 'skipped')),
            summary TEXT,
            evidence JSONB NOT NULL DEFAULT '[]'::jsonb,
            latency_ms INTEGER,
            cost_cents INTEGER NOT NULL DEFAULT 0,
            payment_tx VARCHAR(128),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            completed_at TIMESTAMPTZ
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_deep_research_tool_calls_run_id ON deep_research_tool_calls(run_id, id)",
        r#"
        CREATE TABLE IF NOT EXISTS deep_research_payment_ledger (
            id BIGSERIAL PRIMARY KEY,
            run_id UUID NOT NULL REFERENCES deep_research_runs(id) ON DELETE CASCADE,
            tool_call_id BIGINT NOT NULL REFERENCES deep_research_tool_calls(id) ON DELETE CASCADE,
            provider VARCHAR(64) NOT NULL,
            amount_units VARCHAR(64) NOT NULL,
            amount_display VARCHAR(64) NOT NULL,
            asset VARCHAR(128) NOT NULL,
            network VARCHAR(128) NOT NULL,
            tx_hash VARCHAR(128),
            status VARCHAR(16) NOT NULL CHECK (status IN ('pending', 'completed', 'failed')),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_deep_research_payment_ledger_run_id ON deep_research_payment_ledger(run_id, id)",
        "ALTER TABLE deep_research_payment_ledger ALTER COLUMN asset TYPE VARCHAR(128)",
        "ALTER TABLE deep_research_payment_ledger ALTER COLUMN network TYPE VARCHAR(128)",
        r#"
        CREATE TABLE IF NOT EXISTS investigation_runs (
            id UUID PRIMARY KEY,
            token_address VARCHAR(42) NOT NULL,
            trigger_type VARCHAR(16) NOT NULL DEFAULT 'manual' CHECK (trigger_type IN ('manual', 'auto', 'resume')),
            status VARCHAR(16) NOT NULL CHECK (status IN ('queued', 'running', 'completed', 'failed', 'watching', 'escalated', 'archived')),
            current_stage VARCHAR(32) NOT NULL,
            source_surface VARCHAR(32) NOT NULL DEFAULT 'mia',
            current_read VARCHAR(64),
            confidence_label VARCHAR(32),
            investigation_score INTEGER,
            summary TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            started_at TIMESTAMPTZ,
            completed_at TIMESTAMPTZ
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_investigation_runs_token_created ON investigation_runs(token_address, created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_investigation_runs_status_created ON investigation_runs(status, created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_investigation_runs_trigger_created ON investigation_runs(trigger_type, created_at DESC)",
        r#"
        CREATE TABLE IF NOT EXISTS investigation_run_events (
            id BIGSERIAL PRIMARY KEY,
            run_id UUID NOT NULL REFERENCES investigation_runs(id) ON DELETE CASCADE,
            event_key VARCHAR(64) NOT NULL,
            label VARCHAR(128) NOT NULL,
            detail TEXT NOT NULL,
            reason TEXT,
            evidence_delta TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_investigation_run_events_run_created ON investigation_run_events(run_id, created_at ASC, id ASC)",
        "ALTER TABLE investigation_runs ADD COLUMN IF NOT EXISTS status_reason TEXT",
        "ALTER TABLE investigation_runs ADD COLUMN IF NOT EXISTS evidence_delta TEXT",
        r#"
        CREATE TABLE IF NOT EXISTS investigation_watchlist_items (
            id UUID PRIMARY KEY,
            entity_kind VARCHAR(16) NOT NULL CHECK (entity_kind IN ('token', 'builder')),
            entity_key VARCHAR(128) NOT NULL,
            label VARCHAR(128) NOT NULL,
            source_run_id UUID REFERENCES investigation_runs(id) ON DELETE SET NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE (entity_kind, entity_key)
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_investigation_watchlist_kind_updated ON investigation_watchlist_items(entity_kind, updated_at DESC)",
        r#"
        CREATE TABLE IF NOT EXISTS investigation_missions (
            id UUID PRIMARY KEY,
            mission_type VARCHAR(48) NOT NULL CHECK (mission_type IN (
                'watch_hot_launches',
                'watch_builder_cluster',
                'watch_suspicious_recurrence',
                'watch_proof_qualified_launches'
            )),
            status VARCHAR(16) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'paused', 'archived')),
            entity_kind VARCHAR(16) CHECK (entity_kind IN ('token', 'builder')),
            entity_key VARCHAR(128),
            label VARCHAR(160) NOT NULL,
            note TEXT,
            source_watchlist_item_id UUID REFERENCES investigation_watchlist_items(id) ON DELETE SET NULL,
            source_run_id UUID REFERENCES investigation_runs(id) ON DELETE SET NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_investigation_missions_status_updated ON investigation_missions(status, updated_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_tokens_contract_lower ON tokens(LOWER(contract_address))",
        "CREATE INDEX IF NOT EXISTS idx_tokens_deployer_contract_lower ON tokens(LOWER(deployer_address), LOWER(contract_address), deployed_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_token_txns_token_type_wallet_lower ON token_transactions(LOWER(token_address), tx_type, LOWER(wallet_address))",
        "CREATE INDEX IF NOT EXISTS idx_token_txns_wallet_token_lower ON token_transactions(LOWER(wallet_address), LOWER(token_address))",
        "CREATE INDEX IF NOT EXISTS idx_wallet_clusters_token_wallet_lower ON wallet_clusters(LOWER(token_address), LOWER(wallet_address))",
        "CREATE INDEX IF NOT EXISTS idx_wallet_clusters_wallet_token_lower ON wallet_clusters(LOWER(wallet_address), LOWER(token_address))",
        r#"
        CREATE TABLE IF NOT EXISTS investigation_operator_controls (
            id BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (id),
            auto_investigation_paused BOOLEAN NOT NULL DEFAULT FALSE,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
        r#"
        INSERT INTO investigation_operator_controls (id, auto_investigation_paused, updated_at)
        VALUES (TRUE, FALSE, NOW())
        ON CONFLICT (id) DO NOTHING
        "#,
    ];

    for sql in statements {
        sqlx::query(sql).execute(db).await?;
    }

    Ok(())
}
