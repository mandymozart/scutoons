-- Content store for the creator editor (same as the .mysql.sql of this date).
CREATE TABLE IF NOT EXISTS content_versions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  toon TEXT NOT NULL,
  locale TEXT NOT NULL,
  path TEXT NOT NULL,
  json TEXT NOT NULL,
  author TEXT NULL DEFAULT NULL,
  created_at TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'draft',
  message TEXT NULL DEFAULT NULL
);
CREATE INDEX IF NOT EXISTS content_versions_file ON content_versions (toon, locale, path, id);
CREATE INDEX IF NOT EXISTS content_versions_status ON content_versions (toon, locale, path, status);
