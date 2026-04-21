-- Enable UUID generation
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Core token records
CREATE TABLE IF NOT EXISTS tokens (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    contract_address VARCHAR(42) UNIQUE NOT NULL,
    name VARCHAR(100),
    symbol VARCHAR(20),
    deployer_address VARCHAR(42) NOT NULL,
    deployed_at TIMESTAMPTZ NOT NULL,
    block_number BIGINT NOT NULL,
    tx_hash VARCHAR(66) NOT NULL,
    initial_liquidity_bnb NUMERIC(36, 18),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tokens_deployer ON tokens(deployer_address);
CREATE INDEX IF NOT EXISTS idx_tokens_deployed_at ON tokens(deployed_at DESC);
CREATE INDEX IF NOT EXISTS idx_tokens_block_number ON tokens(block_number DESC);

-- Raw blockchain events log
CREATE TABLE IF NOT EXISTS blockchain_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    block_number BIGINT NOT NULL,
    tx_hash VARCHAR(66) NOT NULL,
    contract_address VARCHAR(42) NOT NULL,
    event_name VARCHAR(100) NOT NULL,
    raw_data JSONB NOT NULL,
    processed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_events_block_number ON blockchain_events(block_number DESC);
CREATE INDEX IF NOT EXISTS idx_events_contract ON blockchain_events(contract_address);
CREATE INDEX IF NOT EXISTS idx_events_tx_hash ON blockchain_events(tx_hash);

-- Indexer state singleton row
CREATE TABLE IF NOT EXISTS indexer_state (
    id INTEGER PRIMARY KEY DEFAULT 1,
    last_processed_block BIGINT NOT NULL DEFAULT 0,
    indexer_status VARCHAR(20) NOT NULL DEFAULT 'idle',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT single_row CHECK (id = 1)
);

INSERT INTO indexer_state (id, last_processed_block, indexer_status)
VALUES (1, 0, 'idle')
ON CONFLICT (id) DO NOTHING;
