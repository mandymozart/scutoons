-- Drafts the member confirms in the browser (agents/MCP.md "Draft → link", owner 2026-10-09): the member's own AI (the
-- SCUTOONS MCP server with a personal token) saves an Embassy mail draft or an admin proposal and hands the member its
-- link; the member sends the mail from the organisation page / applies the proposal in the admin app. Nothing goes out
-- or changes without that click. MariaDB 10.11, idempotent – run in phpMyAdmin (database d0487143 selected).
--
-- Check first (both 0 rows before the first run):
--   SELECT TABLE_NAME FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME IN ('embassy_mail_drafts', 'admin_proposals');

CREATE TABLE IF NOT EXISTS `embassy_mail_drafts` (
  `id` CHAR(36) NOT NULL,
  `org_id` CHAR(36) NOT NULL COMMENT 'embassy_orgs.id',
  `person_id` CHAR(36) NULL DEFAULT NULL COMMENT 'embassy_people.id of the recipient (NULL = the organisation''s address)',
  `mail_to` VARCHAR(254) NOT NULL,
  `track` VARCHAR(16) NULL DEFAULT NULL,
  `subject` VARCHAR(300) NOT NULL,
  `text` MEDIUMTEXT NOT NULL,
  `reply_to_activity_id` CHAR(36) NULL DEFAULT NULL COMMENT 'the logged mail it answers',
  `language` VARCHAR(8) NULL DEFAULT NULL COMMENT 'de | en: the opt-out line''s language (NULL = by the person / country)',
  `source` VARCHAR(16) NOT NULL DEFAULT 'agent' COMMENT 'agent (the member''s AI) | person',
  `created_by` CHAR(36) NOT NULL COMMENT 'users.id – only the author sends it',
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`id`),
  KEY `embassy_mail_drafts_org` (`org_id`, `created_at`),
  CONSTRAINT `embassy_mail_drafts_org_fk` FOREIGN KEY (`org_id`) REFERENCES `embassy_orgs` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `admin_proposals` (
  `id` CHAR(36) NOT NULL,
  `title` VARCHAR(200) NOT NULL,
  `changes` MEDIUMTEXT NOT NULL COMMENT 'JSON list of changes (catalog / calendar / asset) – shared/api AdminChange',
  `calendar_base` INT NULL DEFAULT NULL COMMENT 'the calendar version the calendar changes were read from',
  `source` VARCHAR(16) NOT NULL DEFAULT 'agent' COMMENT 'agent (the admin''s AI) | person',
  `status` VARCHAR(16) NOT NULL DEFAULT 'open' COMMENT 'open | applied | dismissed',
  `created_by` CHAR(36) NOT NULL COMMENT 'users.id',
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `decided_by` CHAR(36) NULL DEFAULT NULL COMMENT 'users.id of who applied / dismissed it',
  `decided_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC',
  PRIMARY KEY (`id`),
  KEY `admin_proposals_status` (`status`, `created_at`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
