-- "Media quality" per member (same as the .mysql.sql of this date; the runner skips the ADD COLUMN when it exists).
ALTER TABLE users ADD COLUMN media_quality TEXT NOT NULL DEFAULT 'auto';
