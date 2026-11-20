-- Rig presets (#677; owner 2026-11-20 [deck-editor], provisional – he tests it): a named set of every knob of one rig
-- ("soft", "dirty") as the catalog kind `rig-presets` (documents `rig-preset:<id>` – drafts, versions, publish like the other
-- catalog kinds). `data` = deck-space rig/rig_preset.rs `RigPreset` { id, rig, name, factory?, origin?, values? }, checked by
-- the core on publish (build service `POST /validate`, kind `rig-presets`). `rig` = data.rig (the rig's subject), a column
-- so scene-content finds a rig's presets. A scene's holder names one in `preset`.
-- What it touches: one new table, empty. Dry run: SHOW TABLES LIKE 'catalog_rig_presets'; → none before, one after.
-- MariaDB 10.11, idempotent.
CREATE TABLE IF NOT EXISTS `catalog_rig_presets` (
  `id` VARCHAR(100) NOT NULL,
  `position` INT NOT NULL DEFAULT 0,
  `data` LONGTEXT NOT NULL COMMENT 'rig preset { id, rig, name, factory?, origin?, values? }' CHECK (JSON_VALID(`data`)),
  `rig` VARCHAR(140) NULL DEFAULT NULL COMMENT 'catalog_rigs.id (= data.rig): the rig it is for',
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_by` VARCHAR(255) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `catalog_rig_presets_position` (`position`),
  KEY `catalog_rig_presets_rig` (`rig`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
