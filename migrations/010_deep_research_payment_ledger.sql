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
);

CREATE INDEX IF NOT EXISTS idx_deep_research_payment_ledger_run_id
ON deep_research_payment_ledger(run_id, id);

ALTER TABLE deep_research_payment_ledger
    ALTER COLUMN asset TYPE VARCHAR(128);

ALTER TABLE deep_research_payment_ledger
    ALTER COLUMN network TYPE VARCHAR(128);
