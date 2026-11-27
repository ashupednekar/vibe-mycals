-- Link diary entries back to the scan item they came from, so a logged item
-- can still be corrected later.
ALTER TABLE diary_entries ADD COLUMN scan_id TEXT;
ALTER TABLE diary_entries ADD COLUMN scan_item INTEGER;

CREATE INDEX IF NOT EXISTS idx_entries_scan ON diary_entries (scan_id);
