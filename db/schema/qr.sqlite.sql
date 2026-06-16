CREATE TABLE IF NOT EXISTS qr (
    id          TEXT NOT NULL PRIMARY KEY,
    code        TEXT NOT NULL UNIQUE,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at  TEXT NULL DEFAULT NULL,
    claimed_at  TEXT NOT NULL DEFAULT (datetime('now')),
    redirect_to TEXT NOT NULL,
    type        TEXT NOT NULL,
    email       TEXT NOT NULL DEFAULT '',
    edition_id  TEXT NULL DEFAULT NULL,
    signature   TEXT NULL DEFAULT NULL
);
