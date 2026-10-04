-- The usage log of the creator's Ask panel (agents/CUTOVER.md 10.28: "Every AI call is logged"; Domain\Agent\AgentUsage,
-- docs/API.md "Agent usage"): one row per AI API call, written by the Ask service (tools/mcp) with the creator's session
-- and the shared AGENT_SERVICE_KEY (POST /api/agent-usage); the admin overview GET /api/admin/agent-usage sums it per
-- creator, service and model per day / month. No limits are enforced from it. `chat_id` has no foreign key on purpose:
-- a chat may be deleted, its cost stays. Without this table the Ask service keeps logging to its own file only.
--
-- phpMyAdmin (production): select the database first (d0487143), SQL tab, run this file. Idempotent; needs only the
-- users table.
CREATE TABLE IF NOT EXISTS `agent_usage` (
  `id` BIGINT NOT NULL AUTO_INCREMENT,
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `user_id` CHAR(36) NOT NULL COMMENT 'users.id: the creator who asked',
  `service` VARCHAR(40) NOT NULL COMMENT 'e.g. anthropic',
  `model` VARCHAR(100) NOT NULL,
  `chat_id` CHAR(36) NULL DEFAULT NULL COMMENT 'agent_chats.id (no FK: the chat may be deleted)',
  `input_tokens` INT NOT NULL DEFAULT 0,
  `output_tokens` INT NOT NULL DEFAULT 0,
  `cache_read_tokens` INT NOT NULL DEFAULT 0,
  `cache_write_tokens` INT NOT NULL DEFAULT 0,
  `cost_usd` DECIMAL(12,6) NOT NULL DEFAULT 0,
  PRIMARY KEY (`id`),
  KEY `agent_usage_created` (`created_at`),
  KEY `agent_usage_user` (`user_id`, `created_at`),
  CONSTRAINT `agent_usage_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
