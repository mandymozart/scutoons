-- "Hide sensitive content" per member (same as the .mysql.sql of this date).
ALTER TABLE users ADD COLUMN hide_sensitive INTEGER NOT NULL DEFAULT 1;
