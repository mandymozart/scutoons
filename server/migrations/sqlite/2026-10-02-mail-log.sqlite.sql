-- Sent-mail log of the email module (same as the .mysql.sql of this date; agents/PLATFORM.md "Emails").
CREATE TABLE IF NOT EXISTS mail_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  template TEXT NOT NULL,
  recipient TEXT NOT NULL,
  locale TEXT NOT NULL,
  status TEXT NOT NULL,
  transport TEXT NOT NULL,
  error TEXT NULL DEFAULT NULL,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS mail_log_created ON mail_log (created_at);
CREATE INDEX IF NOT EXISTS mail_log_template ON mail_log (template, created_at);
CREATE INDEX IF NOT EXISTS mail_log_recipient ON mail_log (recipient, created_at);
