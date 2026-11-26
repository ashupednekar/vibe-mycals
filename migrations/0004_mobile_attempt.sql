ALTER TABLE mobile_auth_codes ADD COLUMN attempt_id TEXT;
CREATE INDEX IF NOT EXISTS idx_mobile_codes_attempt ON mobile_auth_codes (attempt_id);
