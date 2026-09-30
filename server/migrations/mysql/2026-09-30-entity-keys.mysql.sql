-- Entity keys (agents/DATA.md §2): the content store keyed by entity instead of toon/locale/path. New table
-- entity_versions; every row of content_versions is copied with the SAME id (history, "v71" labels and drafts stay),
-- its key computed from the old path. content_versions is left untouched (rollback) and no longer used.
-- Keys: toon:<toon> · toon-settings:<toon>:<l> · season:<toon>:<n>:<l> · chapter:<toon>:<n>:<l> ·
-- scene:<toon>:<n>:<name>:<l> · locations:<toon>:<l> · routes:<toon>:<l> · cast:<toon> · credits:<toon> ·
-- characters · moods · asset-requests · people · roles · events. MariaDB, idempotent (copied rows are skipped).
-- Run AFTER 2026-09-30-content-versions.mysql.sql.
CREATE TABLE IF NOT EXISTS `entity_versions` (
  `id` BIGINT NOT NULL AUTO_INCREMENT,
  `entity_key` VARCHAR(191) NOT NULL COMMENT 'e.g. chapter:catpope:2:de, scene:catpope:2:scene-14:de, characters',
  `kind` VARCHAR(32) NOT NULL COMMENT 'first segment of the key',
  `toon` VARCHAR(64) NULL DEFAULT NULL COMMENT 'NULL = platform-wide (characters, people, events)',
  `locale` VARCHAR(8) NULL DEFAULT NULL COMMENT 'NULL = language-neutral',
  `json` MEDIUMTEXT NOT NULL,
  `author` CHAR(36) NULL DEFAULT NULL COMMENT 'users.id, NULL = import',
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `status` VARCHAR(16) NOT NULL DEFAULT 'draft' COMMENT 'draft | published',
  `message` VARCHAR(255) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `entity_versions_key` (`entity_key`, `id`),
  KEY `entity_versions_status` (`entity_key`, `status`),
  KEY `entity_versions_toon` (`toon`, `status`),
  KEY `entity_versions_kind` (`kind`, `status`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
INSERT INTO `entity_versions` (`id`, `entity_key`, `kind`, `toon`, `locale`, `json`, `author`, `created_at`, `status`, `message`)
SELECT m.id, m.k, SUBSTRING_INDEX(m.k, ':', 1), IF(m.toon = 'scutoons', NULL, m.toon), IF(m.locale = '_', NULL, m.locale),
  m.json, m.author, m.created_at, m.status, m.message
FROM (
  SELECT c.id, c.toon, c.locale, c.json, c.author, c.created_at, c.status, c.message, CASE
    WHEN c.locale = '_' AND c.path = 'index.json' THEN CONCAT('toon:', c.toon)
    WHEN c.locale = '_' AND c.path IN ('cast.json', 'credits.json') THEN CONCAT(REPLACE(c.path, '.json', ''), ':', c.toon)
    WHEN c.locale = '_' AND c.toon = 'scutoons' AND LOCATE('/', c.path) = 0 THEN REPLACE(c.path, '.json', '')
    WHEN c.locale <> '_' AND c.path = 'toon.json' THEN CONCAT('toon-settings:', c.toon, ':', c.locale)
    WHEN c.locale <> '_' AND c.path IN ('locations.json', 'routes.json') THEN CONCAT(REPLACE(c.path, '.json', ''), ':', c.toon, ':', c.locale)
    WHEN c.locale <> '_' AND c.path REGEXP '^chapter[0-9]+/[a-z0-9_-]+[.]json$'
      THEN CONCAT('scene:', c.toon, ':', SUBSTRING(SUBSTRING_INDEX(c.path, '/', 1), 8), ':', REPLACE(SUBSTRING_INDEX(c.path, '/', -1), '.json', ''), ':', c.locale)
    WHEN c.locale <> '_' AND c.path REGEXP '^season[0-9]+[.]json$' THEN CONCAT('season:', c.toon, ':', SUBSTRING(REPLACE(c.path, '.json', ''), 7), ':', c.locale)
    WHEN c.locale <> '_' AND c.path REGEXP '^chapter[0-9]+[.]json$' THEN CONCAT('chapter:', c.toon, ':', SUBSTRING(REPLACE(c.path, '.json', ''), 8), ':', c.locale)
    ELSE CONCAT('legacy:', c.toon, ':', c.locale, ':', REPLACE(REPLACE(c.path, '/', '~'), '.json', ''))
  END AS k
  FROM `content_versions` c
) m
WHERE NOT EXISTS (SELECT 1 FROM `entity_versions` e WHERE e.id = m.id);
