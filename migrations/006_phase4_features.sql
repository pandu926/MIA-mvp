-- Phase 4: Whale tracker + alpha feed + telegram delivery logs

CREATE TABLE IF NOT EXISTS whale_alerts (
    id BIGSERIAL PRIMARY KEY,
    token_address VARCHAR(42) NOT NULL,
    wallet_address VARCHAR(42) NOT NULL,
    tx_hash VARCHAR(66) NOT NULL UNIQUE,
    amount_bnb NUMERIC(24, 8) NOT NULL CHECK (amount_bnb >= 0),
    threshold_bnb NUMERIC(24, 8) NOT NULL CHECK (threshold_bnb >= 0),
    alert_level VARCHAR(16) NOT NULL CHECK (alert_level IN ('watch', 'critical')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_whale_alerts_created_at ON whale_alerts(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_whale_alerts_token_created ON whale_alerts(token_address, created_at DESC);

CREATE TABLE IF NOT EXISTS alpha_rankings (
    id BIGSERIAL PRIMARY KEY,
    window_start TIMESTAMPTZ NOT NULL,
    window_end TIMESTAMPTZ NOT NULL,
    rank SMALLINT NOT NULL CHECK (rank >= 1 AND rank <= 100),
    token_address VARCHAR(42) NOT NULL,
    alpha_score NUMERIC(10, 4) NOT NULL,
    rationale TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_alpha_rankings_window ON alpha_rankings(window_end DESC, rank ASC);
CREATE INDEX IF NOT EXISTS idx_alpha_rankings_token ON alpha_rankings(token_address, window_end DESC);

CREATE TABLE IF NOT EXISTS telegram_delivery_logs (
    id BIGSERIAL PRIMARY KEY,
    channel VARCHAR(32) NOT NULL,
    message_type VARCHAR(32) NOT NULL,
    status VARCHAR(16) NOT NULL CHECK (status IN ('sent', 'skipped', 'failed')),
    payload JSONB NOT NULL,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_telegram_logs_created_at ON telegram_delivery_logs(created_at DESC);
