-- Skeletons on model and animation versions (#817; owner 2026-11-22: "storing shareable animations into our asset
-- library … we can reuse them across characters", 2026-11-22: "there will be more skeleton families, so compatibility has
-- to be checked"): what the deck's core read from a .glb – its joints by name, the joints its first clip turns, its clips
-- by name, the family it recognised (deck-catalog skeleton.rs `Skeleton`). The API asks the core before it stores one
-- (`/validate` kind `skeleton`) and keeps what it was sent; a character's idle is checked against it (kind `animation-fit`).
-- NULL = not read yet (the creator reads and stores it when it opens the file).
-- What it touches: one table gains a nullable column, no row changes. MariaDB 10.11, idempotent. Check first (0 rows before):
--   SELECT TABLE_NAME, COLUMN_NAME FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE()
--     AND TABLE_NAME = 'asset_versions' AND COLUMN_NAME = 'skeleton';
ALTER TABLE `asset_versions` ADD COLUMN IF NOT EXISTS `skeleton` LONGTEXT NULL DEFAULT NULL COMMENT 'skeleton { family?, joints, animated?, clips? } (deck-catalog skeleton.rs)' CHECK (`skeleton` IS NULL OR JSON_VALID(`skeleton`)) AFTER `spots`;
