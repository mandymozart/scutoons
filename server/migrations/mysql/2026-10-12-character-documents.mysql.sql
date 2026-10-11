-- The character registry as many documents (DRAFT branch, agents/MAP-DOCUMENTS.md; owner 2026-10-12, D10: "yes" – the
-- same as the map): every character becomes `character:<id>` (the entry without its id – the key holds it), and
-- `characters` keeps only the registry's own fields (today: its accepted warnings). Composed again for every reader
-- (Domain\Content\CollectionDocs::CHARACTERS, @scutoons/content character-docs.ts): /library/characters, casts, the
-- creator, the MCP – the answers stay the same.
--
-- History (owner D4: "only newest"): per character at most two versions – the one in the PUBLISHED registry version
-- (published) and, when the newest registry version is a newer draft, its entry there (a draft) where it differs or is
-- new; a character the published registry has and the newest draft dropped gets a draft with `"removed":true`. Author,
-- time and note of that registry version (the note says which: `characters v<id>`). Older versions are not split. The
-- new rows go in in registry order (the published one, then the draft's new characters) – the composed order from now on.
-- Then the list leaves every `characters` version (in place – same ids, same status). The runner backs the changed rows
-- up before the UPDATE (apply_migrations), so the old composed versions are not lost.
-- Idempotent: the split runs only while a `characters` version still has its list AND no character document exists;
-- the strip only touches versions that still have it. A second run changes nothing.
-- Runs with the deploy (POST /api/deploy/migrate) – plan_migrations shows the row counts first. Only with the code that
-- reads the documents (this branch). Afterwards nothing to empty (the caches are keyed by version ids); the asset usage
-- index lists the registry's images under `characters` until the next `assets:scan` (admin: Assets → Rescan).
-- Check first (registry versions with a list / character documents / accepted warnings on a character – must be 0):
--   SELECT SUM(JSON_EXISTS(`json`, '$.characters')) FROM `entity_versions` WHERE `entity_key` = 'characters';
--   SELECT COUNT(*) FROM `entity_versions` WHERE `kind` = 'character';
--   SELECT COUNT(*) FROM `entity_versions` WHERE `entity_key` = 'characters' AND `json` LIKE '%"accepted_warnings"%@%';

INSERT INTO `entity_versions` (`entity_key`, `kind`, `toon`, `locale`, `json`, `author`, `created_at`, `status`, `message`)
WITH
  `m` AS (
    SELECT `id`, `json`, `author`, `created_at`, `message`, `status` FROM `entity_versions`
    WHERE `entity_key` = 'characters' AND JSON_EXISTS(`json`, '$.characters')
      AND NOT EXISTS (SELECT 1 FROM `entity_versions` `d` WHERE `d`.`kind` = 'character')
  ),
  `ends` AS (SELECT (SELECT MAX(`id`) FROM `m` WHERE `status` = 'published') AS `pub`, (SELECT MAX(`id`) FROM `m`) AS `newest`),
  `raw` AS (
    SELECT `m`.`id` AS `map_id`, `t`.`ord`, `t`.`pid`, JSON_COMPACT(JSON_REMOVE(`t`.`entry`, '$.id')) AS `doc`
    FROM `m` JOIN `ends` ON `m`.`id` IN (`ends`.`pub`, `ends`.`newest`),
      JSON_TABLE(`m`.`json`, '$.characters[*]' COLUMNS (`ord` FOR ORDINALITY, `pid` VARCHAR(64) PATH '$.id', `entry` JSON PATH '$')) `t`
  ),
  `entries` AS (
    SELECT * FROM (SELECT `raw`.*, ROW_NUMBER() OVER (PARTITION BY `map_id`, `pid` ORDER BY `ord`) AS `n` FROM `raw`
      WHERE `pid` REGEXP '^[a-z0-9][a-z0-9_-]{0,63}$') `x` WHERE `n` = 1
  ),
  `p` AS (SELECT `e`.* FROM `entries` `e` JOIN `ends` ON `e`.`map_id` = `ends`.`pub`),
  `d` AS (SELECT `e`.* FROM `entries` `e` JOIN `ends` ON `e`.`map_id` = `ends`.`newest` AND `ends`.`newest` <> COALESCE(`ends`.`pub`, 0)),
  `rows` AS (
    SELECT `pid`, `map_id`, `ord`, `doc`, 'published' AS `status` FROM `p`
    UNION ALL
    SELECT `d`.`pid`, `d`.`map_id`, `d`.`ord`, `d`.`doc`, 'draft' FROM `d`
      LEFT JOIN `p` ON `p`.`pid` = `d`.`pid`
      WHERE `p`.`pid` IS NULL OR `p`.`doc` <> `d`.`doc`
    UNION ALL
    SELECT `p`.`pid`, `ends`.`newest`, 9999, JSON_COMPACT(JSON_SET(`p`.`doc`, '$.removed', TRUE)), 'draft' FROM `p`
      JOIN `ends` ON `ends`.`newest` <> `ends`.`pub`
      LEFT JOIN `d` ON `d`.`pid` = `p`.`pid`
      WHERE `d`.`pid` IS NULL
  )
SELECT CONCAT('character:', `rows`.`pid`), 'character', NULL, NULL, `rows`.`doc`, `m`.`author`, `m`.`created_at`, `rows`.`status`,
  LEFT(CONCAT('characters v', `m`.`id`, IF(`m`.`message` IS NULL OR `m`.`message` = '', '', CONCAT(': ', `m`.`message`))), 255)
FROM `rows` JOIN `m` ON `m`.`id` = `rows`.`map_id`
ORDER BY `rows`.`map_id`, `rows`.`ord`;

UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, '$.characters'))
  WHERE `entity_key` = 'characters' AND JSON_EXISTS(`json`, '$.characters');
