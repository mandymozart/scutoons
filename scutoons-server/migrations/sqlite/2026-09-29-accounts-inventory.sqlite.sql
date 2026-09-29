-- Local development (SQLite): the same tables as 2026-09-29-accounts-inventory.mysql.sql.
-- ALTER TABLE ... ADD COLUMN is skipped by bin/console migrate when the column exists.

ALTER TABLE qr ADD COLUMN edition_number INTEGER NULL DEFAULT NULL;
ALTER TABLE qr ADD COLUMN claimed_by TEXT NULL DEFAULT NULL;

CREATE TABLE IF NOT EXISTS users (
  id TEXT NOT NULL PRIMARY KEY,
  email TEXT NOT NULL UNIQUE,
  locale TEXT NOT NULL DEFAULT 'en',
  created_at TEXT NOT NULL,
  confirmed_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS login_requests (
  id TEXT NOT NULL PRIMARY KEY,
  email TEXT NOT NULL,
  token_hash TEXT NOT NULL UNIQUE,
  code_hash TEXT NOT NULL,
  locale TEXT NOT NULL DEFAULT 'en',
  claim_code TEXT NULL DEFAULT NULL,
  claim_item_id TEXT NULL DEFAULT NULL,
  claim_variant TEXT NULL DEFAULT NULL,
  ip_hash TEXT NOT NULL,
  attempts INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  used_at TEXT NULL DEFAULT NULL
);
CREATE INDEX IF NOT EXISTS login_requests_email ON login_requests (email, created_at);
CREATE INDEX IF NOT EXISTS login_requests_ip ON login_requests (ip_hash, created_at);

CREATE TABLE IF NOT EXISTS sessions (
  id TEXT NOT NULL PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  token_hash TEXT NOT NULL UNIQUE,
  created_at TEXT NOT NULL,
  last_used_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS sessions_user ON sessions (user_id);

CREATE TABLE IF NOT EXISTS code_items (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  code TEXT NOT NULL,
  item_id TEXT NOT NULL,
  variant TEXT NULL DEFAULT NULL,
  qty INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS code_items_code ON code_items (code);

CREATE TABLE IF NOT EXISTS inventory (
  id TEXT NOT NULL PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  item_id TEXT NOT NULL,
  variant TEXT NULL DEFAULT NULL,
  qty INTEGER NOT NULL DEFAULT 1,
  source TEXT NOT NULL,
  code TEXT NULL DEFAULT NULL,
  edition_number INTEGER NULL DEFAULT NULL,
  acquired_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS inventory_user ON inventory (user_id, item_id);
CREATE INDEX IF NOT EXISTS inventory_code ON inventory (code);

CREATE TABLE IF NOT EXISTS avatar_slots (
  user_id TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  slot TEXT NOT NULL,
  entry_id TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (user_id, slot)
);

CREATE TABLE IF NOT EXISTS progress (
  user_id TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  toon_id TEXT NOT NULL,
  state TEXT NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (user_id, toon_id)
);

CREATE TABLE IF NOT EXISTS rate_limits (
  id TEXT NOT NULL PRIMARY KEY,
  bucket TEXT NOT NULL,
  ref TEXT NULL DEFAULT NULL,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS rate_limits_bucket ON rate_limits (bucket, created_at);
CREATE INDEX IF NOT EXISTS rate_limits_ref ON rate_limits (bucket, ref);
