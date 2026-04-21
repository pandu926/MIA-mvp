-- Phase 3: AI narrative results table
CREATE TABLE IF NOT EXISTS ai_narratives (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    token_address VARCHAR(42) UNIQUE NOT NULL REFERENCES tokens(contract_address) ON DELETE CASCADE,
    narrative_text TEXT NOT NULL,
    risk_interpretation TEXT,
    consensus_status VARCHAR(20) NOT NULL CHECK (consensus_status IN ('agreed', 'diverged', 'single_model')),
    confidence VARCHAR(10) NOT NULL CHECK (confidence IN ('high', 'medium', 'low')),
    -- Structured data snapshot used to generate the narrative (for auditability)
    data_basis JSONB NOT NULL DEFAULT '{}',
    -- Raw model outputs for debugging / re-evaluation
    model_a_response TEXT,
    model_b_response TEXT,
    generated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Staleness marker (generated_at + cache TTL, refreshed on each update)
    expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '5 minutes'
);

CREATE INDEX IF NOT EXISTS idx_narratives_token ON ai_narratives(token_address);
CREATE INDEX IF NOT EXISTS idx_narratives_expires ON ai_narratives(expires_at);
CREATE INDEX IF NOT EXISTS idx_narratives_generated ON ai_narratives(generated_at DESC);
