-- The creature airlift's carriers (agents/research/CAVE.md D8): crazy creatures and flying robot taxis, one drawn at
-- random per flight; data = @scutoons/content AirliftCarrier { id, name, image }. Edited in the creator's catalog
-- (documents `airlift-carrier:<id>`), no rows yet – the owner adds them. MariaDB 10.11, idempotent.
CREATE TABLE IF NOT EXISTS `catalog_airlift_carriers` (
  `id` VARCHAR(100) NOT NULL,
  `position` INT NOT NULL DEFAULT 0,
  `data` LONGTEXT NOT NULL COMMENT '{ id, name, image }' CHECK (JSON_VALID(`data`)),
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_by` VARCHAR(255) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `catalog_airlift_carriers_position` (`position`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
