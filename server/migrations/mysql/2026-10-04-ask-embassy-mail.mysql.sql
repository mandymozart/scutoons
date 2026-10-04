-- The Ask panel in more apps + Embassy mail (agents/EMBASSY.md "Communication strategy" / "Automation"; docs/API.md
-- "Agent chats", "Agent usage", "Embassy" → "Mail" / "Inbox"):
-- 1. `agent_chats.app` / `agent_usage.app`: which app's Ask panel a chat / an AI call belongs to (creator | embassy |
--    admin); existing rows are the creator's.
-- 2. `embassy_activities`: subject, from / to address, the mail's Message-ID (ours on mail_out, theirs on mail_in – the
--    key for threading and for not logging a reply twice) and In-Reply-To of mails sent from the app / read back by IMAP.
-- 3. `embassy_mail_state`: per mailbox folder where the inbox sync stopped (UIDVALIDITY + last UID), when it last ran
--    and its last error.
-- Until it ran: chats and usage are all the creator's (other apps answer 503 not_available), mails are sent and logged
-- without subject / addresses, the inbox sync answers 503 not_available.
--
-- phpMyAdmin (production): select the database first (d0487143), SQL tab, run this file. Additive, MariaDB, idempotent
-- (IF NOT EXISTS); needs 2026-10-01-embassy, 2026-10-04-agent-chats and 2026-10-04-agent-usage.
ALTER TABLE `agent_chats` ADD COLUMN IF NOT EXISTS `app` VARCHAR(16) NOT NULL DEFAULT 'creator' COMMENT 'creator | embassy | admin: whose Ask panel';
CREATE INDEX IF NOT EXISTS `agent_chats_user_app` ON `agent_chats` (`user_id`, `app`, `updated_at`);
ALTER TABLE `agent_usage` ADD COLUMN IF NOT EXISTS `app` VARCHAR(16) NOT NULL DEFAULT 'creator' COMMENT 'creator | embassy | admin: whose Ask panel';

ALTER TABLE `embassy_activities` ADD COLUMN IF NOT EXISTS `subject` VARCHAR(300) NULL DEFAULT NULL COMMENT 'mails: the subject';
ALTER TABLE `embassy_activities` ADD COLUMN IF NOT EXISTS `mail_from` VARCHAR(254) NULL DEFAULT NULL COMMENT 'mails: sender address';
ALTER TABLE `embassy_activities` ADD COLUMN IF NOT EXISTS `mail_to` VARCHAR(254) NULL DEFAULT NULL COMMENT 'mails: recipient address';
ALTER TABLE `embassy_activities` ADD COLUMN IF NOT EXISTS `message_id` VARCHAR(255) NULL DEFAULT NULL COMMENT 'mails: Message-ID without <> (ours on mail_out, theirs on mail_in)';
ALTER TABLE `embassy_activities` ADD COLUMN IF NOT EXISTS `in_reply_to` VARCHAR(255) NULL DEFAULT NULL COMMENT 'mails: In-Reply-To without <>';
CREATE INDEX IF NOT EXISTS `embassy_activities_message` ON `embassy_activities` (`message_id`);

CREATE TABLE IF NOT EXISTS `embassy_mail_state` (
  `id` VARCHAR(64) NOT NULL COMMENT 'the mailbox folder, e.g. INBOX',
  `uid_validity` BIGINT NULL DEFAULT NULL COMMENT 'IMAP UIDVALIDITY of the folder (a change resets last_uid)',
  `last_uid` BIGINT NOT NULL DEFAULT 0 COMMENT 'the highest UID read',
  `synced_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC: the last sync attempt',
  `error` VARCHAR(500) NULL DEFAULT NULL COMMENT 'the last attempt''s error (NULL = it worked)',
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
