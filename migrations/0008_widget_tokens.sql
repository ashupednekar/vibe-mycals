CREATE TABLE IF NOT EXISTS widget_tokens (
  token TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_widget_tokens_user ON widget_tokens (user_id);
