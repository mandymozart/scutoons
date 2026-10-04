-- The Ask panel in more apps + Embassy mail (same as the .mysql.sql of this date; docs/API.md "Agent chats", "Agent
-- usage", "Embassy" → "Mail" / "Inbox").
ALTER TABLE agent_chats ADD COLUMN app TEXT NOT NULL DEFAULT 'creator';
CREATE INDEX IF NOT EXISTS agent_chats_user_app ON agent_chats (user_id, app, updated_at);
ALTER TABLE agent_usage ADD COLUMN app TEXT NOT NULL DEFAULT 'creator';

ALTER TABLE embassy_activities ADD COLUMN subject TEXT NULL DEFAULT NULL;
ALTER TABLE embassy_activities ADD COLUMN mail_from TEXT NULL DEFAULT NULL;
ALTER TABLE embassy_activities ADD COLUMN mail_to TEXT NULL DEFAULT NULL;
ALTER TABLE embassy_activities ADD COLUMN message_id TEXT NULL DEFAULT NULL;
ALTER TABLE embassy_activities ADD COLUMN in_reply_to TEXT NULL DEFAULT NULL;
CREATE INDEX IF NOT EXISTS embassy_activities_message ON embassy_activities (message_id);

CREATE TABLE IF NOT EXISTS embassy_mail_state (
  id TEXT NOT NULL PRIMARY KEY,
  uid_validity INTEGER NULL DEFAULT NULL,
  last_uid INTEGER NOT NULL DEFAULT 0,
  synced_at TEXT NULL DEFAULT NULL,
  error TEXT NULL DEFAULT NULL
);
