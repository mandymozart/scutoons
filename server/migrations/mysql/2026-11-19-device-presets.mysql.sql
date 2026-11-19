-- Device presets (#633; owner 2026-11-18 [workspaces], the orchestrator's calls 2026-11-19 [delegated]): a machine's audio
-- settings – host, device, buffer, the outlet groups on the device's channels – and the hardware they are for, as the catalog
-- kind `device-presets` (documents `device-preset:<id>` – drafts, versions, publish like the other catalog kinds).
-- `data` = deck-space device_preset.rs `DevicePreset` { id, name, origin?, hardware { names?, driver?, channels?, usb? },
-- host?, device?, buffer?, outlets? }, checked by the core on publish (build service `POST /validate`, kind
-- `device-presets`). Only members' own presets: the factory ones ship with the deck (deck/space/presets/device-presets.toml)
-- and a catalog write refuses `factory`.
-- What it touches: one new table, empty. Dry run: SHOW TABLES LIKE 'catalog_device_presets'; → none before, one after.
-- MariaDB 10.11, idempotent.
CREATE TABLE IF NOT EXISTS `catalog_device_presets` (
  `id` VARCHAR(100) NOT NULL,
  `position` INT NOT NULL DEFAULT 0,
  `data` LONGTEXT NOT NULL COMMENT 'device preset { id, name, origin?, hardware, host?, device?, buffer?, outlets? }' CHECK (JSON_VALID(`data`)),
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_by` VARCHAR(255) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `catalog_device_presets_position` (`position`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
