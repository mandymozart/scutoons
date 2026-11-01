-- What a paired deck may write (#270, agents/research/INSTRUMENTS.md §4–6; docs/API.md "Decks"): a deck token is
-- read-only (DeckTokenScope) unless the member granted it a write scope at approval – the way the approval grants its
-- lifetime. `rigs` = the member's own presets and rig variants (/api/rigs/*: "save as preset" from the deck's knobs).
-- Empty = reads only (every deck approved before this migration). MariaDB 10.11, idempotent. Check first (0 rows before
-- the first run):
--   SELECT COLUMN_NAME FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'decks' AND COLUMN_NAME = 'scopes';
ALTER TABLE `decks` ADD COLUMN IF NOT EXISTS `scopes` VARCHAR(100) NOT NULL DEFAULT '' COMMENT 'write scopes granted at approval, comma-separated (rigs); empty = reads only' AFTER `mapping_ref`;
