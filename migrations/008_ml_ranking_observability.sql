-- Phase 5: ML ranking rollout and observability

CREATE TABLE IF NOT EXISTS ml_model_registry (
    id BIGSERIAL PRIMARY KEY,
    model_version TEXT NOT NULL UNIQUE,
    rollout_mode VARCHAR(16) NOT NULL CHECK (rollout_mode IN ('legacy', 'shadow', 'ml', 'hybrid')),
    is_active BOOLEAN NOT NULL DEFAULT false,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

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
);

CREATE INDEX IF NOT EXISTS idx_ml_alpha_predictions_window ON ml_alpha_predictions(window_end DESC);
CREATE INDEX IF NOT EXISTS idx_ml_alpha_predictions_source ON ml_alpha_predictions(score_source, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_ml_alpha_predictions_realized ON ml_alpha_predictions(realized_hit_1h, created_at DESC);
