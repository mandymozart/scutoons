-- Content store for the creator editor (scutoons agents/MEMORY.md "Creator editor", docs/ACCOUNTS.md "Content API").
-- Every save of a chapter/season/toon config file is a new row (a version). Per (toon, locale, path) at most one row
-- is `published` (the live one); every other row is `draft` (history). MariaDB, idempotent.
CREATE TABLE IF NOT EXISTS `content_versions` (
  `id` BIGINT NOT NULL AUTO_INCREMENT,
  `toon` VARCHAR(64) NOT NULL,
  `locale` VARCHAR(8) NOT NULL,
  `path` VARCHAR(191) NOT NULL COMMENT 'e.g. chapter1.json, chapter2/scene-14.json',
  `json` MEDIUMTEXT NOT NULL,
  `author` CHAR(36) NULL DEFAULT NULL COMMENT 'users.id, NULL = import script',
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `status` VARCHAR(16) NOT NULL DEFAULT 'draft' COMMENT 'draft | published',
  `message` VARCHAR(255) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `content_versions_file` (`toon`, `locale`, `path`, `id`),
  KEY `content_versions_status` (`toon`, `locale`, `path`, `status`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
