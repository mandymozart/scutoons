-- The member save layer goes (#302; owner 2026-11-02: "members who are only readers can not save presets … we move out of
-- the deck. its a creator part. more strict is better."; agents/DECISIONS.md "Readers save nothing"). Dropped: the tables
-- member_presets, member_rigs (#258 / #270), member_modules (#281) and decks.scopes (#270; its only scope was `rigs`).
-- REFUSES to drop anything that holds data: the guard inserts the row count into a table whose CHECK demands 0, so a
-- non-empty table aborts the file with a constraint error before its DROP (report the rows instead, nothing is lost).
-- Idempotent: a missing table / column is created empty first (so the guard has something to count), then dropped.
-- MariaDB 10.11. Look first (all expected 0):
--   SELECT (SELECT COUNT(*) FROM member_presets) AS presets, (SELECT COUNT(*) FROM member_rigs) AS rigs,
--          (SELECT COUNT(*) FROM member_modules) AS modules, (SELECT COUNT(*) FROM decks WHERE scopes <> '') AS scoped_decks;
CREATE TABLE IF NOT EXISTS `member_presets` (`id` VARCHAR(40) NOT NULL) ENGINE=InnoDB;
CREATE TABLE IF NOT EXISTS `member_rigs` (`user_id` CHAR(36) NOT NULL) ENGINE=InnoDB;
CREATE TABLE IF NOT EXISTS `member_modules` (`id` VARCHAR(40) NOT NULL) ENGINE=InnoDB;
ALTER TABLE `decks` ADD COLUMN IF NOT EXISTS `scopes` VARCHAR(100) NOT NULL DEFAULT '' AFTER `mapping_ref`;
CREATE TEMPORARY TABLE `member_layer_guard` (`what` VARCHAR(20) NOT NULL, `n` INT NOT NULL CHECK (`n` = 0));
INSERT INTO `member_layer_guard` SELECT 'member_presets', COUNT(*) FROM `member_presets`;
INSERT INTO `member_layer_guard` SELECT 'member_rigs', COUNT(*) FROM `member_rigs`;
INSERT INTO `member_layer_guard` SELECT 'member_modules', COUNT(*) FROM `member_modules`;
INSERT INTO `member_layer_guard` SELECT 'decks.scopes', COUNT(*) FROM `decks` WHERE `scopes` <> '';
DROP TEMPORARY TABLE `member_layer_guard`;
DROP TABLE IF EXISTS `member_presets`;
DROP TABLE IF EXISTS `member_rigs`;
DROP TABLE IF EXISTS `member_modules`;
ALTER TABLE `decks` DROP COLUMN IF EXISTS `scopes`;
