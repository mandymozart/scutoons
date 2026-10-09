-- AI connections expire after six months (owner 2026-10-09): renewing on the account page moves a token six months on;
-- used within its last 14 days, its member gets one reminder mail per period (`reminded_at`). Tokens made before this
-- get their six months from when they were made. MariaDB 10.11, idempotent – phpMyAdmin (database d0487143 selected).
--
-- Check first: the tokens without an end (they get one):
--   SELECT id, name, created_at FROM api_tokens WHERE expires_at IS NULL;

ALTER TABLE `api_tokens` ADD COLUMN IF NOT EXISTS `reminded_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC: the expiry reminder of this period went out' AFTER `expires_at`;
UPDATE `api_tokens` SET `expires_at` = DATE_ADD(`created_at`, INTERVAL 182 DAY) WHERE `expires_at` IS NULL;
