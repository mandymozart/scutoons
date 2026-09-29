-- Base schema (SQLite) of a fresh database, applied by `bin/console migrate` when the qr table is missing.
-- Keep in sync with qr.mysql.sql and migrations/sqlite/*.sqlite.sql.

CREATE TABLE IF NOT EXISTS qr (
    id             TEXT NOT NULL PRIMARY KEY,
    code           TEXT NOT NULL UNIQUE,
    created_at     TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at     TEXT NULL DEFAULT NULL,
    claimed_at     TEXT NOT NULL DEFAULT (datetime('now')),
    redirect_to    TEXT NOT NULL,
    type           TEXT NOT NULL,
    email          TEXT NOT NULL DEFAULT '',
    edition_id     TEXT NULL DEFAULT NULL,
    signature      TEXT NULL DEFAULT NULL,
    edition_number INTEGER NULL DEFAULT NULL,
    claimed_by     TEXT NULL DEFAULT NULL,
    revoked_at     TEXT NULL DEFAULT NULL,
    revoked_reason TEXT NULL DEFAULT NULL
);

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

-- One owner per QR code (migrations/sqlite/2026-09-30-code-owner-guard.sqlite.sql)
CREATE TRIGGER IF NOT EXISTS qr_owner_immutable BEFORE UPDATE OF claimed_by ON qr WHEN OLD.claimed_by IS NOT NULL AND NEW.claimed_by IS NOT NULL AND NEW.claimed_by <> OLD.claimed_by BEGIN SELECT RAISE(ABORT, 'qr code already belongs to another account'); END;
CREATE TRIGGER IF NOT EXISTS inventory_code_owner_insert BEFORE INSERT ON inventory WHEN NEW.code IS NOT NULL AND NEW.user_id IS NOT (SELECT claimed_by FROM qr WHERE code = NEW.code) BEGIN SELECT RAISE(ABORT, 'inventory code does not belong to this account'); END;
CREATE TRIGGER IF NOT EXISTS inventory_code_owner_update BEFORE UPDATE OF user_id, code ON inventory WHEN NEW.code IS NOT NULL AND NEW.user_id IS NOT (SELECT claimed_by FROM qr WHERE code = NEW.code) BEGIN SELECT RAISE(ABORT, 'inventory code does not belong to this account'); END;

-- Revoke + activity statistics (migrations/sqlite/2026-09-30-revoke-events.sqlite.sql)
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
