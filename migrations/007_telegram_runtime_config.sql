-- Runtime-editable Telegram config used by frontend alerts settings

CREATE TABLE IF NOT EXISTS telegram_runtime_config (
    id SMALLINT PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    enabled BOOLEAN NOT NULL DEFAULT false,
    chat_id TEXT,
    threshold_bnb NUMERIC(24, 8) NOT NULL DEFAULT 0.5 CHECK (threshold_bnb >= 0),
    alpha_digest_enabled BOOLEAN NOT NULL DEFAULT true,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO telegram_runtime_config (id, enabled, threshold_bnb, alpha_digest_enabled)
VALUES (1, false, 0.5, true)
ON CONFLICT (id) DO NOTHING;
