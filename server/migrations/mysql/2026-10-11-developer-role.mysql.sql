-- The developer role and migrations through the API (BACKLOG 10.61, owner 2026-10-11: "nicht Admin, darüber hinaus: eine
-- Developer-Rolle, die mein Account hat, andere erst freigeschaltet"). After this file, pending migrations run through
-- the API (/api/dev/*): the deploy triggers them (POST /api/deploy/migrate with MIGRATE_KEY), a developer's agent plans
-- and applies them over MCP – no more pasting into phpMyAdmin. This file itself is the last one run by hand.
-- MariaDB 10.11, idempotent – run in phpMyAdmin (database d0487143 selected first).
--
--   1. the role `developer` (id 6) – above admin; only a developer grants it, the last one can't be revoked
--   2. the owner's account gets it (granted_by 'migration')
--   3. dev_migration_runs: every plan (dry run) and apply, who / what triggered it, when, the log
--   4. schema_migrations completed: files run by hand in phpMyAdmin were never recorded – without this, the first run
--      through the API would repeat them all
--
-- Check first – (a) the owner's account (1 row); (b) the migrations already recorded (anything missing below is added as
-- run: every file up to this one has been run on production by hand – owner 2026-10-05 / 2026-10-11):
--   SELECT id, email FROM users WHERE email = 'tilman@porschuetz.de';
--   SELECT name, applied_at FROM schema_migrations ORDER BY name;

INSERT IGNORE INTO `roles` (`id`, `name`, `description`) VALUES
  (6, 'developer', 'Runs the database migrations (above admin; only a developer grants it)');

INSERT IGNORE INTO `user_roles` (`user_id`, `role_id`, `granted_at`, `granted_by`)
  SELECT `id`, 6, UTC_TIMESTAMP(), 'migration' FROM `users` WHERE `email` = 'tilman@porschuetz.de';

CREATE TABLE IF NOT EXISTS `dev_migration_runs` (
  `id` CHAR(36) NOT NULL,
  `status` VARCHAR(16) NOT NULL DEFAULT 'planned' COMMENT 'planned | applied | failed | stale (a newer plan, or the files changed)',
  `plan` MEDIUMTEXT NOT NULL COMMENT 'JSON: the pending files (name, sha256), their statements, the dry run''s row counts',
  `created_by` CHAR(36) NULL DEFAULT NULL COMMENT 'users.id of the developer (NULL = the deploy)',
  `created_via` VARCHAR(120) NOT NULL DEFAULT 'session' COMMENT 'deploy | session | ai:<the AI connection''s name>',
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `applied_by` CHAR(36) NULL DEFAULT NULL COMMENT 'users.id (NULL = the deploy)',
  `applied_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC',
  `log` MEDIUMTEXT NULL DEFAULT NULL COMMENT 'what the apply did, line by line',
  `error` TEXT NULL DEFAULT NULL,
  `backup` VARCHAR(255) NULL DEFAULT NULL COMMENT 'the backup of the changed rows, below storage/ (JSON lines)',
  PRIMARY KEY (`id`),
  KEY `dev_migration_runs_created` (`created_at`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `schema_migrations` (`name` VARCHAR(191) NOT NULL, `applied_at` DATETIME NOT NULL, PRIMARY KEY (`name`)) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

INSERT IGNORE INTO `schema_migrations` (`name`, `applied_at`) VALUES
  ('2026-09-29-accounts-inventory.mysql.sql', UTC_TIMESTAMP()),
  ('2026-09-30-code-owner-guard.mysql.sql', UTC_TIMESTAMP()),
  ('2026-09-30-content-versions.mysql.sql', UTC_TIMESTAMP()),
  ('2026-09-30-entity-keys.mysql.sql', UTC_TIMESTAMP()),
  ('2026-09-30-member-characters.mysql.sql', UTC_TIMESTAMP()),
  ('2026-09-30-revoke-events.mysql.sql', UTC_TIMESTAMP()),
  ('2026-09-30-roles-signup.mysql.sql', UTC_TIMESTAMP()),
  ('2026-09-30-sensitive-content.mysql.sql', UTC_TIMESTAMP()),
  ('2026-09-30-settings.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-01-embassy.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-01-media-quality.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-01-reader-state.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-02-assets.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-02-embassy-stores.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-02-mail-log.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-02-ownership-signup.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-02-ownership.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-03-code-case.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-04-agent-chats.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-04-agent-usage.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-04-ask-embassy-mail.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-04-catalog.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-04-reviews.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-05-cdn-domain.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-05-embassy-pace.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-05-no-naked.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-06-agent-web-search.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-06-clock-syntax.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-06-display-name.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-06-global-save-game.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-06-repeat.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-06-show-text-box.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-06-target-value.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-07-account-deletion-frees-codes.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-07-give-item.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-07-newsletter.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-07-sequence-no-type.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-07-show-hints.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-09-api-tokens.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-09-drop-ask.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-09-mcp-drafts.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-09-tokens-expire.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-11-unlock-location-events.mysql.sql', UTC_TIMESTAMP()),
  ('2026-10-11-developer-role.mysql.sql', UTC_TIMESTAMP());
