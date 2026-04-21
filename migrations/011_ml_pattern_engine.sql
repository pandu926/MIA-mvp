CREATE TABLE IF NOT EXISTS ml_pattern_model_registry (
    id BIGSERIAL PRIMARY KEY,
    model_version TEXT NOT NULL UNIQUE,
    model_family VARCHAR(64) NOT NULL CHECK (model_family IN ('lightgbm_similarity_iforest')),
    is_active BOOLEAN NOT NULL DEFAULT false,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

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
);

CREATE INDEX IF NOT EXISTS idx_ml_pattern_model_registry_active
    ON ml_pattern_model_registry(is_active, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_ml_pattern_predictions_token_window
    ON ml_pattern_predictions(token_address, window_end DESC, horizon_hours);
CREATE INDEX IF NOT EXISTS idx_ml_pattern_predictions_model_horizon
    ON ml_pattern_predictions(model_version, horizon_hours, created_at DESC);
