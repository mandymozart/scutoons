-- AI connections = personal access tokens (agents/MCP.md §3, owner 2026-10-04 / 2026-10-09): a member creates one on
-- their account page and gives it to their own AI (Claude Desktop / Claude Code with the SCUTOONS MCP server). Only
-- the SHA-256 of the token is stored (like sessions); `sct_…` tokens are taken as `Authorization: Bearer` with the
-- account's current roles; readers can't have one. MariaDB 10.11, idempotent – run in phpMyAdmin (database d0487143).
--
-- Check first (0 rows before the first run):
--   SELECT TABLE_NAME FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'api_tokens';

CREATE TABLE IF NOT EXISTS `api_tokens` (
  `id` CHAR(36) NOT NULL,
  `user_id` CHAR(36) NOT NULL COMMENT 'users.id',
  `name` VARCHAR(100) NOT NULL COMMENT 'what the member called it, e.g. "Claude on my laptop" – saves are signed with it',
  `token_hash` CHAR(64) NOT NULL COMMENT 'SHA-256 of the token (the token itself is shown once, never stored)',
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `last_used_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC, updated at most once a minute',
  `expires_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC; NULL = until revoked',
  `revoked_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC; set = the token no longer works',
  PRIMARY KEY (`id`),
  UNIQUE KEY `api_tokens_hash` (`token_hash`),
  KEY `api_tokens_user` (`user_id`, `revoked_at`),
  CONSTRAINT `api_tokens_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
