-- Containers (agents/research/SCORES.md §2, FEATURES 10.97): an inventory entry can lie in a container entry of the same
-- account (`container_id`; a song book holding scores, the DJ trolley holding records); an overfilled container
-- remembers since when (`overfilled_at` – it breaks after the catalog's `holds.overfill.days`); an ageing item's look
-- counts from `aged_since` (none = `acquired_at`; an upgraded / coloured book keeps the old book's age).
-- Nothing is filled: every existing entry lies loose, nothing is overfilled. MariaDB 10.11, idempotent.
ALTER TABLE `inventory` ADD COLUMN IF NOT EXISTS `container_id` CHAR(36) NULL DEFAULT NULL COMMENT 'the container entry it lies in (same user)' AFTER `edition_number`;
ALTER TABLE `inventory` ADD COLUMN IF NOT EXISTS `overfilled_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC: a container holds more than holds.max since' AFTER `container_id`;
ALTER TABLE `inventory` ADD COLUMN IF NOT EXISTS `aged_since` DATETIME NULL DEFAULT NULL COMMENT 'UTC: an ageing item ages from here (none = acquired_at)' AFTER `overfilled_at`;
ALTER TABLE `inventory` ADD INDEX IF NOT EXISTS `inventory_container` (`container_id`);
