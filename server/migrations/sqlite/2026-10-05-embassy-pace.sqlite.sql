-- Embassy pace + reminders (same as the .mysql.sql of this date; docs/API.md "Embassy" → "Pace").
CREATE TABLE IF NOT EXISTS embassy_pace (
  user_id TEXT NOT NULL PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
  per_week INTEGER NOT NULL,
  reminder TEXT NOT NULL DEFAULT 'weekly',
  onboarded_at TEXT NOT NULL,
  asked_at TEXT NULL DEFAULT NULL,
  reminded_at TEXT NULL DEFAULT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS embassy_checkins (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  user_id TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  at TEXT NOT NULL,
  answer TEXT NOT NULL,
  per_week_before INTEGER NOT NULL,
  per_week_after INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS embassy_checkins_user ON embassy_checkins (user_id, at);

CREATE INDEX IF NOT EXISTS embassy_activities_by_user ON embassy_activities (by_user_id, at);
