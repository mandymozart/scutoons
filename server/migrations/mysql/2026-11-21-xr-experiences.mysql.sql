-- XR experiences (#482, research/XR.md §4.4): one published document per experience as the catalog kind `experiences`
-- (documents `experience:<id>` – drafts, versions, publish like the other catalog kinds). `data` = { id, type, title, toons, assets,
-- source?, configuration, plugins }: `configuration` is OSCT's game configuration as OSCT wrote it, `plugins[].code` its
-- compiled ES modules as text. The reader's `/xr/<experience>` reads it at GET /api/library/experiences/<id>.
-- What it touches: one new table, empty. Dry run: SHOW TABLES LIKE 'catalog_experiences'; → none before, one after.
-- MariaDB 10.11, idempotent.
CREATE TABLE IF NOT EXISTS `catalog_experiences` (
  `id` VARCHAR(100) NOT NULL,
  `position` INT NOT NULL DEFAULT 0,
  `data` LONGTEXT NOT NULL COMMENT 'XR experience { id, type, title, toons, source?, assets, configuration, plugins }' CHECK (JSON_VALID(`data`)),
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_by` VARCHAR(255) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `catalog_experiences_position` (`position`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
