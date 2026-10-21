-- Scripts per season and per chapter (owner 2026-10-20: "i need a script for each seasons, because that is how cat pope
-- was written"; research/KANON.md §6 migration 1, #81): the toon's screenplay `script:<toon>:<l>` is season 1's script
-- `season-script:<toon>:1:<l>` – every version (drafts, history, the published one) keeps its id, status, author, time
-- and note; only the key and the kind change. The pending reviews of a toon script follow (`content_reviews`).
-- Today only catpope has one (de complete, en partial – "Katzenpabst – Das Erwachen" = season 1). The code reads the
-- new kinds only (the old `script` kind leaves EntityKey / @scutoons/content with this change, no alias).
-- Idempotent: only rows still of kind `script` / keys still starting with `script:` change – a second run changes 0 rows.
-- Every compared string is a literal.
-- Check first:
--   SELECT `entity_key`, COUNT(*) FROM `entity_versions` WHERE `kind` = 'script' GROUP BY `entity_key`;
--   SELECT COUNT(*) FROM `entity_versions` WHERE `kind` = 'season-script';   -- 0 before the first run

UPDATE `entity_versions`
SET `entity_key` = CONCAT('season-script:', `toon`, ':1:', `locale`), `kind` = 'season-script'
WHERE `kind` = 'script' AND `toon` IS NOT NULL AND `locale` IS NOT NULL;

UPDATE `content_reviews`
-- `script:<toon>` → `season-script:<toon>:1`, `script:<toon>:<l>` → `season-script:<toon>:1:<l>` (string functions:
-- regex back-references depend on the session's escaping)
SET `entity_key` = CONCAT('season-script:', SUBSTRING_INDEX(SUBSTRING_INDEX(`entity_key`, ':', 2), ':', -1), ':1',
  IF(`entity_key` LIKE 'script:%:%', CONCAT(':', SUBSTRING_INDEX(`entity_key`, ':', -1)), ''))
WHERE `entity_key` LIKE 'script:%';
