-- The usage log of the creator's Ask panel (same as the .mysql.sql of this date; docs/API.md "Agent usage").
CREATE TABLE IF NOT EXISTS agent_usage (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  created_at TEXT NOT NULL,
  user_id TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  service TEXT NOT NULL,
  model TEXT NOT NULL,
  chat_id TEXT NULL DEFAULT NULL,
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  cache_read_tokens INTEGER NOT NULL DEFAULT 0,
  cache_write_tokens INTEGER NOT NULL DEFAULT 0,
  cost_usd NUMERIC NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS agent_usage_created ON agent_usage (created_at);
CREATE INDEX IF NOT EXISTS agent_usage_user ON agent_usage (user_id, created_at);
