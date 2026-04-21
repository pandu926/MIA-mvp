-- Phase 2: Transaction tracking, wallet clustering, risk scores

-- Buy/sell transactions per token
CREATE TABLE IF NOT EXISTS token_transactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    token_address VARCHAR(42) NOT NULL REFERENCES tokens(contract_address) ON DELETE CASCADE,
    wallet_address VARCHAR(42) NOT NULL,
    tx_type VARCHAR(4) NOT NULL CHECK (tx_type IN ('buy', 'sell')),
    amount_bnb NUMERIC(36, 18) NOT NULL DEFAULT 0,
    tx_hash VARCHAR(66) UNIQUE NOT NULL,
    block_number BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_token_txns_token ON token_transactions(token_address);
CREATE INDEX IF NOT EXISTS idx_token_txns_wallet ON token_transactions(wallet_address);
CREATE INDEX IF NOT EXISTS idx_token_txns_block ON token_transactions(block_number DESC);
CREATE INDEX IF NOT EXISTS idx_token_txns_created ON token_transactions(created_at DESC);

-- Wallet clustering groups (co-moving wallets)
CREATE TABLE IF NOT EXISTS wallet_clusters (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    token_address VARCHAR(42) NOT NULL REFERENCES tokens(contract_address) ON DELETE CASCADE,
    wallet_address VARCHAR(42) NOT NULL,
    cluster_id UUID NOT NULL,
    confidence VARCHAR(20) NOT NULL CHECK (confidence IN ('potential', 'probable')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (token_address, wallet_address)
);

CREATE INDEX IF NOT EXISTS idx_clusters_token ON wallet_clusters(token_address);
CREATE INDEX IF NOT EXISTS idx_clusters_cluster_id ON wallet_clusters(cluster_id);

-- Computed risk scores per token (upserted on each re-analysis)
CREATE TABLE IF NOT EXISTS risk_scores (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    token_address VARCHAR(42) UNIQUE NOT NULL REFERENCES tokens(contract_address) ON DELETE CASCADE,
    composite_score SMALLINT NOT NULL CHECK (composite_score BETWEEN 0 AND 100),
    deployer_history_score SMALLINT CHECK (deployer_history_score BETWEEN 0 AND 100),
    liquidity_lock_score SMALLINT CHECK (liquidity_lock_score BETWEEN 0 AND 100),
    wallet_concentration_score SMALLINT CHECK (wallet_concentration_score BETWEEN 0 AND 100),
    buy_sell_velocity_score SMALLINT CHECK (buy_sell_velocity_score BETWEEN 0 AND 100),
    contract_audit_score SMALLINT CHECK (contract_audit_score BETWEEN 0 AND 100),
    social_authenticity_score SMALLINT CHECK (social_authenticity_score BETWEEN 0 AND 100),
    volume_consistency_score SMALLINT CHECK (volume_consistency_score BETWEEN 0 AND 100),
    computed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_risk_scores_composite ON risk_scores(composite_score);

-- Add aggregated metrics columns to tokens table
ALTER TABLE tokens
    ADD COLUMN IF NOT EXISTS holder_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS buy_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS sell_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS volume_bnb NUMERIC(36, 18) NOT NULL DEFAULT 0;
