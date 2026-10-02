-- Ownership claims, claim attachments and the transfer history (same as the .mysql.sql of this date; agents/ITEMS.md
-- "Ownership, claims, gifting").
CREATE TABLE IF NOT EXISTS ownership_claims (
  id TEXT NOT NULL PRIMARY KEY,
  code TEXT NOT NULL,
  claimant_id TEXT NOT NULL,
  owner_id TEXT NULL DEFAULT NULL,
  statement TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'open',
  admin_id TEXT NULL DEFAULT NULL,
  admin_note TEXT NULL DEFAULT NULL,
  created_at TEXT NOT NULL,
  decided_at TEXT NULL DEFAULT NULL
);
CREATE INDEX IF NOT EXISTS ownership_claims_code ON ownership_claims (code, status);
CREATE INDEX IF NOT EXISTS ownership_claims_claimant ON ownership_claims (claimant_id, created_at);
CREATE INDEX IF NOT EXISTS ownership_claims_status ON ownership_claims (status, created_at);

CREATE TABLE IF NOT EXISTS ownership_claim_files (
  id TEXT NOT NULL PRIMARY KEY,
  claim_id TEXT NOT NULL,
  filename TEXT NOT NULL,
  name TEXT NOT NULL,
  content_type TEXT NOT NULL,
  size INTEGER NOT NULL,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS ownership_claim_files_claim ON ownership_claim_files (claim_id);

CREATE TABLE IF NOT EXISTS item_transfers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  kind TEXT NOT NULL,
  code TEXT NULL DEFAULT NULL,
  item_id TEXT NULL DEFAULT NULL,
  variant TEXT NULL DEFAULT NULL,
  qty INTEGER NOT NULL DEFAULT 1,
  from_user_id TEXT NULL DEFAULT NULL,
  to_user_id TEXT NOT NULL,
  by_user_id TEXT NULL DEFAULT NULL,
  claim_id TEXT NULL DEFAULT NULL,
  note TEXT NULL DEFAULT NULL,
  created_at TEXT NOT NULL,
  seen_at TEXT NULL DEFAULT NULL
);
CREATE INDEX IF NOT EXISTS item_transfers_code ON item_transfers (code, created_at);
CREATE INDEX IF NOT EXISTS item_transfers_to ON item_transfers (to_user_id, created_at);
CREATE INDEX IF NOT EXISTS item_transfers_from ON item_transfers (from_user_id, kind, created_at);
