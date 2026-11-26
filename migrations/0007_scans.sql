CREATE TABLE IF NOT EXISTS scans (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  log_date TEXT NOT NULL,
  name TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  confidence REAL NOT NULL DEFAULT 0,
  items_json TEXT NOT NULL,
  kcal REAL NOT NULL DEFAULT 0,
  image_key TEXT,
  image_mime TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_scans_user_created ON scans (user_id, created_at DESC);
