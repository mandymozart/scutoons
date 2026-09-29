-- Runtime settings the admin app changes without a deploy (same as the .mysql.sql of this date).
CREATE TABLE IF NOT EXISTS settings (
  `key` TEXT NOT NULL PRIMARY KEY,
  `value` TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  updated_by TEXT NULL DEFAULT NULL
);
