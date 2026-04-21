-- Add deployer intelligence columns to tokens table
ALTER TABLE tokens
    ADD COLUMN IF NOT EXISTS is_rug BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS graduated BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS honeypot_detected BOOLEAN NOT NULL DEFAULT false;

CREATE INDEX IF NOT EXISTS idx_tokens_is_rug ON tokens(is_rug);
CREATE INDEX IF NOT EXISTS idx_tokens_graduated ON tokens(graduated);
