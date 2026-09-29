-- Revoke single codes + activity statistics (same as the .mysql.sql of this date).
ALTER TABLE qr ADD COLUMN revoked_at TEXT NULL DEFAULT NULL;
ALTER TABLE qr ADD COLUMN revoked_reason TEXT NULL DEFAULT NULL;

CREATE TABLE IF NOT EXISTS code_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  code TEXT NOT NULL,
  event TEXT NOT NULL,
  device_hash TEXT NULL DEFAULT NULL,
  user_id TEXT NULL DEFAULT NULL,
  ip_hash TEXT NULL DEFAULT NULL,
  toon TEXT NULL DEFAULT NULL,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS code_events_code ON code_events (code, created_at);
CREATE INDEX IF NOT EXISTS code_events_event ON code_events (event, created_at);
CREATE INDEX IF NOT EXISTS code_events_dedupe ON code_events (code, event, device_hash, created_at);
