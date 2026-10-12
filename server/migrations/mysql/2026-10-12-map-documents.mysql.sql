-- The world map as many documents (DRAFT branch, agents/MAP-DOCUMENTS.md; owner 2026-10-12: "alle einen eigenen Doc, den
-- man versionieren kann"): every place becomes `place:<id>`, every route `route:<id>` (the entry without its id – the key
-- holds it), and `map` keeps only its own settings (width, height, layers, accepted_warnings).
--
-- History (owner 2026-10-12, D4: "only newest"): per place / route at most two versions – the one in the PUBLISHED map
-- version (published) and, when the newest map version is a newer draft, its entry there (a draft) where it differs or
-- is new; a place the published map has and the newest draft dropped gets a draft with `"removed":true` (publishing it
-- takes the place off readers' maps). Author, time and note of that map version (the note says which: `map v<id>`).
-- Readers see exactly the map they saw before; the newest draft stays the draft. Older map versions are not split.
-- The new rows go in in map order (published map, then the draft's new entries), the composed map's order from now on.
-- Then the lists leave every `map` version (its settings stay, in place – same ids, same status). The runner backs the
-- changed rows up before the UPDATE (apply_migrations), so the old composed versions are not lost.
-- Idempotent: the split runs only while a `map` version still has `locations` / `routes` AND no place / route document
-- exists; the strip only touches versions that still have the lists. A second run changes nothing.
-- Runs with the deploy (POST /api/deploy/migrate, owner 2026-10-12) – plan_migrations shows the row counts first. Only
-- with the code that reads the documents (this branch): the old code reads `map` alone and would see an empty map.
-- Afterwards: nothing to empty – the library and flow caches are keyed by the version ids, and the place / route keys
-- are new. The asset usage index (agents/ASSETS.md) still lists the map's images under `map` until the next
-- `assets:scan` (admin: Assets → Rescan) – or the next save of each place.
-- Check first (map versions with lists / place + route documents / map versions with accepted warnings on a place):
--   SELECT SUM(JSON_EXISTS(`json`, '$.locations') OR JSON_EXISTS(`json`, '$.routes')) FROM `entity_versions` WHERE `entity_key` = 'map';
--   SELECT COUNT(*) FROM `entity_versions` WHERE `kind` IN ('place', 'route');
--   SELECT COUNT(*) FROM `entity_versions` WHERE `entity_key` = 'map' AND `json` LIKE '%"accepted_warnings"%@%';
-- (the last must be 0: accepted warnings of a place would have to move into its document – none in the dump 2026-10-06)

INSERT INTO `entity_versions` (`entity_key`, `kind`, `toon`, `locale`, `json`, `author`, `created_at`, `status`, `message`)
WITH
  -- every version of the one-document map that still has its lists (none once split)
  `m` AS (
    SELECT `id`, `json`, `author`, `created_at`, `message`, `status` FROM `entity_versions`
    WHERE `entity_key` = 'map' AND (JSON_EXISTS(`json`, '$.locations') OR JSON_EXISTS(`json`, '$.routes'))
      AND NOT EXISTS (SELECT 1 FROM `entity_versions` `d` WHERE `d`.`kind` IN ('place', 'route'))
  ),
  `ends` AS (SELECT (SELECT MAX(`id`) FROM `m` WHERE `status` = 'published') AS `pub`, (SELECT MAX(`id`) FROM `m`) AS `newest`),
  -- the places / routes of the published and the newest map version (the first one of a duplicate id), without their id
  `raw` AS (
    SELECT 'place' AS `kind`, `m`.`id` AS `map_id`, `t`.`ord`, `t`.`pid`, JSON_COMPACT(JSON_REMOVE(`t`.`entry`, '$.id')) AS `doc`
    FROM `m` JOIN `ends` ON `m`.`id` IN (`ends`.`pub`, `ends`.`newest`),
      JSON_TABLE(`m`.`json`, '$.locations[*]' COLUMNS (`ord` FOR ORDINALITY, `pid` VARCHAR(64) PATH '$.id', `entry` JSON PATH '$')) `t`
    UNION ALL
    SELECT 'route', `m`.`id`, 1000 + `t`.`ord`, `t`.`pid`, JSON_COMPACT(JSON_REMOVE(`t`.`entry`, '$.id'))
    FROM `m` JOIN `ends` ON `m`.`id` IN (`ends`.`pub`, `ends`.`newest`),
      JSON_TABLE(`m`.`json`, '$.routes[*]' COLUMNS (`ord` FOR ORDINALITY, `pid` VARCHAR(64) PATH '$.id', `entry` JSON PATH '$')) `t`
  ),
  `entries` AS (
    SELECT * FROM (SELECT `raw`.*, ROW_NUMBER() OVER (PARTITION BY `kind`, `map_id`, `pid` ORDER BY `ord`) AS `n` FROM `raw`
      WHERE `pid` REGEXP '^[a-z0-9][a-z0-9_-]{0,63}$') `x` WHERE `n` = 1
  ),
  `p` AS (SELECT `e`.* FROM `entries` `e` JOIN `ends` ON `e`.`map_id` = `ends`.`pub`),
  `d` AS (SELECT `e`.* FROM `entries` `e` JOIN `ends` ON `e`.`map_id` = `ends`.`newest` AND `ends`.`newest` <> COALESCE(`ends`.`pub`, 0)),
  `rows` AS (
    -- what readers see: published
    SELECT `kind`, `pid`, `map_id`, `ord`, `doc`, 'published' AS `status` FROM `p`
    UNION ALL
    -- the newest draft where it is new or differs
    SELECT `d`.`kind`, `d`.`pid`, `d`.`map_id`, `d`.`ord`, `d`.`doc`, 'draft' FROM `d`
      LEFT JOIN `p` ON `p`.`kind` = `d`.`kind` AND `p`.`pid` = `d`.`pid`
      WHERE `p`.`pid` IS NULL OR `p`.`doc` <> `d`.`doc`
    UNION ALL
    -- published, but the newest draft dropped it: a removal waiting to be published
    SELECT `p`.`kind`, `p`.`pid`, `ends`.`newest`, 9999, JSON_COMPACT(JSON_SET(`p`.`doc`, '$.removed', TRUE)), 'draft' FROM `p`
      JOIN `ends` ON `ends`.`newest` <> `ends`.`pub`
      LEFT JOIN `d` ON `d`.`kind` = `p`.`kind` AND `d`.`pid` = `p`.`pid`
      WHERE `d`.`pid` IS NULL
  )
SELECT CONCAT(`rows`.`kind`, ':', `rows`.`pid`), `rows`.`kind`, NULL, NULL, `rows`.`doc`, `m`.`author`, `m`.`created_at`, `rows`.`status`,
  LEFT(CONCAT('map v', `m`.`id`, IF(`m`.`message` IS NULL OR `m`.`message` = '', '', CONCAT(': ', `m`.`message`))), 255)
FROM `rows` JOIN `m` ON `m`.`id` = `rows`.`map_id`
ORDER BY `rows`.`map_id`, `rows`.`ord`, `rows`.`kind`;

UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(JSON_REMOVE(`json`, '$.locations'), '$.routes'))
  WHERE `entity_key` = 'map' AND (JSON_EXISTS(`json`, '$.locations') OR JSON_EXISTS(`json`, '$.routes'));
