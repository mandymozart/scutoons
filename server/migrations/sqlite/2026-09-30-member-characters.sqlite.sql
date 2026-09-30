-- Reader avatar bodies & character unlocks (same as the .mysql.sql of this date; default unlocks = setting
-- AVATAR_DEFAULT_UNLOCKS, not rows).
CREATE TABLE IF NOT EXISTS member_characters (
  user_id TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  character_id TEXT NOT NULL,
  source TEXT NOT NULL DEFAULT 'admin',
  ref TEXT NULL DEFAULT NULL,
  unlocked_at TEXT NOT NULL,
  PRIMARY KEY (user_id, character_id)
);

CREATE TABLE IF NOT EXISTS member_character_looks (
  user_id TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  character_id TEXT NOT NULL,
  look_id TEXT NOT NULL,
  source TEXT NOT NULL DEFAULT 'admin',
  ref TEXT NULL DEFAULT NULL,
  unlocked_at TEXT NOT NULL,
  PRIMARY KEY (user_id, character_id, look_id)
);

CREATE TABLE IF NOT EXISTS avatar_body (
  user_id TEXT NOT NULL PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
  character_id TEXT NOT NULL,
  look_id TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
