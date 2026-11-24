-- Thumbnails (#842; owner 2026-11-24 "a standardized way of storing those and updating those if things change … like in the XR").
-- `thumbnails`: one still per key at the deck's STILL shape (264 × 168 WebP, centre-cut) – an address (`item:camera`,
-- `scene:…`), an asset (`asset:<id>`, the file is the asset's `thumb` rendition `<slug>.thumb.webp`) or a member's workspace
-- snapshot (`workspace:<id>`, `owner` = that member; shared keys have owner ''). `stamp` is what the still was taken from
-- (an asset's version number, an entity's updated_at …); `stale` = 1 once the thing changed and the still waits to be made again.
-- What it touches: one new table (empty). Dry run: SHOW TABLES LIKE 'thumbnails'; → thumbnails.
-- MariaDB 10.11, idempotent.
CREATE TABLE IF NOT EXISTS `thumbnails` (
  `owner` CHAR(36) NOT NULL DEFAULT '' COMMENT 'the member of a workspace snapshot; empty for shared stills',
  `thumb_key` VARCHAR(200) NOT NULL COMMENT 'asset:<id> | workspace:<id> | an address',
  `path` VARCHAR(500) NOT NULL COMMENT 'the storage key of the WebP',
  `stamp` VARCHAR(100) NOT NULL DEFAULT '' COMMENT 'the source it was taken from',
  `stale` TINYINT(1) NOT NULL DEFAULT 0,
  `origin` VARCHAR(16) NOT NULL DEFAULT 'client' COMMENT 'client (a browser or the deck) | server (an asset job)',
  `size` INT UNSIGNED NOT NULL DEFAULT 0,
  `taken_at` DATETIME NOT NULL COMMENT 'UTC',
  `taken_by` CHAR(36) NULL DEFAULT NULL,
  PRIMARY KEY (`owner`, `thumb_key`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
