-- Reader stats and effects (scutoons agents/ITEMS.md "Substances"). MariaDB, idempotent. Run after
-- 2026-10-01-embassy (it only needs the users table: two new tables, nothing else changes).
-- member_stats: a stat's stored level at updated_at (testo, appetite); the current level drifts back to the stat's
--   baseline with its half-life (config/items.json "stats"), computed on read – no cron. No row = the baseline.
-- member_effects: a temporary reader effect (psychedelic) running until `until`; expired rows are ignored and removed
--   on the next write.
CREATE TABLE IF NOT EXISTS `member_stats` (
  `user_id` CHAR(36) NOT NULL,
  `stat` VARCHAR(32) NOT NULL,
  `level` DOUBLE NOT NULL COMMENT '0-100 at updated_at',
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`user_id`, `stat`),
  CONSTRAINT `member_stats_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `member_effects` (
  `user_id` CHAR(36) NOT NULL,
  `kind` VARCHAR(32) NOT NULL,
  `until` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`user_id`, `kind`),
  CONSTRAINT `member_effects_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
