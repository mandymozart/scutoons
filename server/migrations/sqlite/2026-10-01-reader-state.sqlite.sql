-- Reader stats and effects (same as the .mysql.sql of this date; agents/ITEMS.md "Substances").
CREATE TABLE IF NOT EXISTS member_stats (
  user_id TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  stat TEXT NOT NULL,
  level REAL NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (user_id, stat)
);

CREATE TABLE IF NOT EXISTS member_effects (
  user_id TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  kind TEXT NOT NULL,
  until TEXT NOT NULL,
  PRIMARY KEY (user_id, kind)
);
