-- Review before publish (same as the .mysql.sql of this date; agents/CUTOVER.md 10.28 "Review levels").
CREATE TABLE IF NOT EXISTS content_review_rules (
  toon TEXT NOT NULL,
  user_id TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  rule TEXT NOT NULL,
  created_at TEXT NOT NULL,
  created_by TEXT NULL DEFAULT NULL,
  PRIMARY KEY (toon, user_id, rule)
);
CREATE INDEX IF NOT EXISTS content_review_rules_user ON content_review_rules (user_id);

CREATE TABLE IF NOT EXISTS content_reviews (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_key TEXT NOT NULL,
  toon TEXT NOT NULL,
  version_id INTEGER NOT NULL,
  author TEXT NULL DEFAULT NULL,
  note TEXT NULL DEFAULT NULL,
  state TEXT NOT NULL DEFAULT 'pending',
  created_at TEXT NOT NULL,
  decided_by TEXT NULL DEFAULT NULL,
  decided_at TEXT NULL DEFAULT NULL,
  decision_note TEXT NULL DEFAULT NULL
);
CREATE INDEX IF NOT EXISTS content_reviews_key ON content_reviews (entity_key, state);
CREATE INDEX IF NOT EXISTS content_reviews_toon ON content_reviews (toon, state);
CREATE INDEX IF NOT EXISTS content_reviews_author ON content_reviews (author, id);
