-- Auto-update updated_at on row modification
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER tokens_updated_at
    BEFORE UPDATE ON tokens
    FOR EACH ROW
    EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER indexer_state_updated_at
    BEFORE UPDATE ON indexer_state
    FOR EACH ROW
    EXECUTE FUNCTION set_updated_at();
