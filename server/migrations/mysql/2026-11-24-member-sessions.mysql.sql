-- Resume across devices (#838; owner 2026-11-24 "Resume should resume across devices down the hierarchy").
-- `member_sessions`: one row per member and device – the creator's resume state (open items, their workspaces, panels,
-- views, scroll) as JSON; the server checks only that it is an object and its size, the creator reads the inside.
-- `device_id` is a random id the creator keeps in the browser, `device_name` a short label (browser · OS).
-- What it touches: one new table (empty). Dry run: SHOW TABLES LIKE 'member_sessions'; → member_sessions.
-- MariaDB 10.11, idempotent.
CREATE TABLE IF NOT EXISTS `member_sessions` (
  `user_id` CHAR(36) NOT NULL,
  `device_id` VARCHAR(64) NOT NULL COMMENT 'random, kept in the browser',
  `device_name` VARCHAR(60) NOT NULL DEFAULT '' COMMENT 'browser · OS',
  `state` LONGTEXT NOT NULL COMMENT 'the creator resume state' CHECK (JSON_VALID(`state`)),
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`user_id`, `device_id`),
  KEY `member_sessions_newest` (`user_id`, `updated_at`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
