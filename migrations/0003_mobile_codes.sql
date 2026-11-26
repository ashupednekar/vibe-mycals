CREATE TABLE IF NOT EXISTS mobile_auth_codes (
  code TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  expires_at INTEGER NOT NULL,
  used INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_mobile_codes_expiry ON mobile_auth_codes (expires_at);
