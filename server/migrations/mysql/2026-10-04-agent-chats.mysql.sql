-- The creator's Ask panel chat history (agents/CUTOVER.md 10.28 step 2; Domain\Agent\AgentChats, docs/API.md "Agent
-- chats"): one row per chat, owned by the creator who started it (only they ever see or change it). `turns` = what the
-- panel shows, `messages` = the model conversation (can be several MB). The id is a UUID made by the Ask service.
-- Without this table the panel works but keeps no history (GET/PUT /api/agent-chats answer 503 not_available).
--
-- phpMyAdmin (production): select the database first (d0487143), SQL tab, run this file. Idempotent; needs only the
-- users table. LONGTEXT + JSON_VALID, not the JSON type (MySQL's JSON type would reorder object keys).
CREATE TABLE IF NOT EXISTS `agent_chats` (
  `id` CHAR(36) NOT NULL COMMENT 'UUID from the Ask service',
  `user_id` CHAR(36) NOT NULL COMMENT 'users.id: the owner',
  `title` VARCHAR(200) NOT NULL DEFAULT '',
  `page` VARCHAR(300) NULL DEFAULT NULL COMMENT 'where in the creator the chat started',
  `turns` LONGTEXT NOT NULL COMMENT 'JSON array: what the panel shows' CHECK (JSON_VALID(`turns`)),
  `messages` LONGTEXT NOT NULL COMMENT 'JSON array: the model conversation' CHECK (JSON_VALID(`messages`)),
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`id`),
  KEY `agent_chats_user` (`user_id`, `updated_at`),
  CONSTRAINT `agent_chats_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
