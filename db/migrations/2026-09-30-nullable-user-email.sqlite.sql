-- Anonymous accounts (ITEMS.md "Access model 2026-09-30"): users.email and users.confirmed_at become nullable
-- (email IS NULL = anonymous account created by POST /auth/code/). UNIQUE still holds for non-null emails.
-- login_requests.anon_user_id = the anonymous account that asked to attach this email (merged on verify).
-- SQLite cannot drop NOT NULL: the users table is rebuilt with foreign keys off (data kept, child tables untouched).
PRAGMA foreign_keys = OFF;
CREATE TABLE users_rebuild (
  id TEXT NOT NULL PRIMARY KEY,
  email TEXT NULL UNIQUE,
  locale TEXT NOT NULL DEFAULT 'en',
  created_at TEXT NOT NULL,
  confirmed_at TEXT NULL,
  updated_at TEXT NOT NULL
);
INSERT INTO users_rebuild (id, email, locale, created_at, confirmed_at, updated_at) SELECT id, email, locale, created_at, confirmed_at, updated_at FROM users;
DROP TABLE users;
ALTER TABLE users_rebuild RENAME TO users;
PRAGMA foreign_keys = ON;
ALTER TABLE login_requests ADD COLUMN anon_user_id TEXT NULL DEFAULT NULL;
