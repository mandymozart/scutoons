-- The creator's Ask panel chat history (same as the .mysql.sql of this date; docs/API.md "Agent chats").
CREATE TABLE IF NOT EXISTS agent_chats (
  id TEXT NOT NULL PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  title TEXT NOT NULL DEFAULT '',
  page TEXT NULL DEFAULT NULL,
  turns TEXT NOT NULL CHECK (json_valid(turns)),
  messages TEXT NOT NULL CHECK (json_valid(messages)),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS agent_chats_user ON agent_chats (user_id, updated_at);
