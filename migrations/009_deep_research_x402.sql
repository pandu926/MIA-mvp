-- Phase 3: Deep Research entitlements, payment audit trail, and cached reports

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
);

CREATE INDEX IF NOT EXISTS idx_payment_attempts_token_status
    ON payment_attempts(token_address, status, created_at DESC);

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
);

CREATE INDEX IF NOT EXISTS idx_entitlements_token_status
    ON entitlements(token_address, status, unlocked_at DESC);

CREATE INDEX IF NOT EXISTS idx_entitlements_secret
    ON entitlements(entitlement_secret);

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
);

CREATE INDEX IF NOT EXISTS idx_deep_research_reports_token_provider
    ON deep_research_reports(token_address, provider_path);
