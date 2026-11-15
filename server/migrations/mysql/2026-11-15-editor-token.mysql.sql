-- The native editor signs in with the decks' handshake (docs/API.md "Decks"): a row is a deck or an editor. An editor's
-- token (`sce_…`) acts as its member with draft rights (Http\EditorTokenScope: reads, draft saves, no publish) and is
-- approved only by a creator or an admin; a deck's (`scd_…`) stays read-only. Every row before this migration is a deck.
-- MariaDB 10.11, idempotent. Check first (0 rows before the first run):
--   SELECT COLUMN_NAME FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'decks' AND COLUMN_NAME = 'kind';
ALTER TABLE `decks` ADD COLUMN IF NOT EXISTS `kind` VARCHAR(16) NOT NULL DEFAULT 'deck' COMMENT 'deck (reads only) or editor (the native editor: reads and draft saves)' AFTER `public_key`;
