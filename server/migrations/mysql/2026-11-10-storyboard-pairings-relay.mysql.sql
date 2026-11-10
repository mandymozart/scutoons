-- The handheld camera moves to the live road (#469, owner 2026-11-10: "Change the handheld to the new dogma as well";
-- agents/research/HANDHELD-CAMERA.md): the phone and the PC meet in a relay session, no more WebRTC. The pairing's
-- offer / answer columns go; a pairing names its relay `session` (the PC's conductor token and the phone's handheld token
-- are for it) and when the phone claimed the code (`claimed_at`: a code is good for one claim). A pairing lives 10
-- minutes, so the open ones are dropped (a QR shown during the deploy: show a new one). MariaDB 10.11, idempotent.
-- Look first:
--   SELECT COUNT(*) FROM storyboard_pairings;
DELETE FROM `storyboard_pairings`;
ALTER TABLE `storyboard_pairings` DROP COLUMN IF EXISTS `offer`;
ALTER TABLE `storyboard_pairings` DROP COLUMN IF EXISTS `answer`;
ALTER TABLE `storyboard_pairings` ADD COLUMN IF NOT EXISTS `session` VARCHAR(16) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL DEFAULT '' COMMENT 'the relay session the phone and the PC join (H + 11 characters)' AFTER `shot`;
ALTER TABLE `storyboard_pairings` ADD COLUMN IF NOT EXISTS `claimed_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC: the phone claimed the code (once)' AFTER `created_by`;
