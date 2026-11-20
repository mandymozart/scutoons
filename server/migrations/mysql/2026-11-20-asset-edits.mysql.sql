-- Asset edits (#651; owner 2026-11-19 [media]): an asset's edit list – trim, crop, fade in / out, speed, loudness – in
-- the source's time, applied while playing (no derivative is baked), and each rendition's time map against the source:
-- the source's time at the file's 0 and the file's frame rate, measured by the deck's core (deck-media timemap). The
-- numbers are the core's (deck-core asset_edit cleans them on read); the server keeps what it was sent. NULL = no edits /
-- not measured (the deck takes offset 0 and says so).
-- What it touches: two tables gain nullable columns, no row changes. MariaDB 10.11, idempotent. Check first (0 rows before):
--   SELECT TABLE_NAME, COLUMN_NAME FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE()
--     AND ((TABLE_NAME = 'assets' AND COLUMN_NAME = 'edits') OR (TABLE_NAME = 'asset_renditions' AND COLUMN_NAME IN ('time_offset', 'frame_rate')));
ALTER TABLE `assets` ADD COLUMN IF NOT EXISTS `edits` LONGTEXT NULL DEFAULT NULL COMMENT 'edit list { start?, end?, crop?, fade_in?, fade_out?, speed?, gain_db? } (deck-space asset_edits.rs)' CHECK (`edits` IS NULL OR JSON_VALID(`edits`)) AFTER `tags`;
ALTER TABLE `asset_renditions` ADD COLUMN IF NOT EXISTS `time_offset` DOUBLE NULL DEFAULT NULL COMMENT 's of the source at this file''s 0 (measured)' AFTER `bitrate`;
ALTER TABLE `asset_renditions` ADD COLUMN IF NOT EXISTS `frame_rate` DOUBLE NULL DEFAULT NULL COMMENT 'this file''s frames per second (measured)' AFTER `time_offset`;
