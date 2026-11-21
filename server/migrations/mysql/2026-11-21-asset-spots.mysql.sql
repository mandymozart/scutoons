-- Spots on models (#724; owner 2026-11-21 [rigs]: "Points on a model are spots (stored on the asset version)"): a model
-- version's spots – what the deck's core read from the file (the names a spot can attach to, the file's own spots from
-- its empties `spot_*` / `port_*`) and the hand edits over them. The document is the core's (deck-catalog spot.rs
-- `Spots`); the API asks the core before it stores one (`/validate` kind `spots`) and keeps what it was sent.
-- NULL = no spots stored (the deck reads the file's own).
-- What it touches: one table gains a nullable column, no row changes. MariaDB 10.11, idempotent. Check first (0 rows before):
--   SELECT TABLE_NAME, COLUMN_NAME FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE()
--     AND TABLE_NAME = 'asset_versions' AND COLUMN_NAME = 'spots';
ALTER TABLE `asset_versions` ADD COLUMN IF NOT EXISTS `spots` LONGTEXT NULL DEFAULT NULL COMMENT 'spots { file?: { nodes, bones, spots }, hand?: [..] } (deck-catalog spot.rs)' CHECK (`spots` IS NULL OR JSON_VALID(`spots`)) AFTER `note`;
