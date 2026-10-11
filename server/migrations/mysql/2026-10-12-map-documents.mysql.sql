-- The world map as many documents (DRAFT branch, agents/MAP-DOCUMENTS.md; owner 2026-10-12: "alle einen eigenen Doc, den
-- man versionieren kann"): every place becomes `place:<id>`, every route `route:<id>` (the entry without its id – the key
-- holds it), and `map` keeps only its own settings (width, height, layers, accepted_warnings).
--
-- History: EVERY stored version of the old one-document map is split, in order. A place gets a version where it first
-- appears and wherever it changed from the map version before (author, time and note of that map version; the note says
-- which map version it came from); where it left the map: a last version with `"removed":true` (a tombstone – its id stays
-- taken, readers don't get it); where it came back: a version without. The same for routes. A place's PUBLISHED version
-- = its newest version made from a map version up to the published map version – so readers see exactly the map they
-- saw before, and every newer draft stays a draft. The new rows go in in map order (map version, then list order), which
-- is the composed map's order from now on (creation order).
-- Then the lists leave every old `map` version (its settings stay, in place – same ids, same status, same history).
--
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
  -- every version of the one-document map that still has its lists
  `m` AS (
    SELECT `id`, `json`, `author`, `created_at`, `message` FROM `entity_versions`
    WHERE `entity_key` = 'map' AND (JSON_EXISTS(`json`, '$.locations') OR JSON_EXISTS(`json`, '$.routes'))
      AND NOT EXISTS (SELECT 1 FROM `entity_versions` `d` WHERE `d`.`kind` IN ('place', 'route'))
  ),
  `pub` AS (SELECT MAX(`id`) AS `id` FROM `entity_versions` WHERE `entity_key` = 'map' AND `status` = 'published'),
  -- every place / route of every map version (the first one of a duplicate id), without its id
  `raw` AS (
    SELECT 'place' AS `kind`, `m`.`id` AS `map_id`, `t`.`ord`, `t`.`pid`, JSON_COMPACT(JSON_REMOVE(`t`.`entry`, '$.id')) AS `doc`
    FROM `m`, JSON_TABLE(`m`.`json`, '$.locations[*]' COLUMNS (`ord` FOR ORDINALITY, `pid` VARCHAR(64) PATH '$.id', `entry` JSON PATH '$')) `t`
    UNION ALL
    SELECT 'route', `m`.`id`, 1000 + `t`.`ord`, `t`.`pid`, JSON_COMPACT(JSON_REMOVE(`t`.`entry`, '$.id'))
    FROM `m`, JSON_TABLE(`m`.`json`, '$.routes[*]' COLUMNS (`ord` FOR ORDINALITY, `pid` VARCHAR(64) PATH '$.id', `entry` JSON PATH '$')) `t`
  ),
  `entries` AS (
    SELECT * FROM (SELECT `raw`.*, ROW_NUMBER() OVER (PARTITION BY `kind`, `map_id`, `pid` ORDER BY `ord`) AS `n` FROM `raw`
      WHERE `pid` REGEXP '^[a-z0-9][a-z0-9_-]{0,63}$') `x` WHERE `n` = 1
  ),
  -- every id × every map version from its first appearance on: present (its entry) or absent (NULL)
  `grid` AS (
    SELECT `ids`.`kind`, `ids`.`pid`, `m`.`id` AS `map_id`, `m`.`author`, `m`.`created_at`, `m`.`message`, `e`.`ord`, `e`.`doc`
    FROM (SELECT `kind`, `pid`, MIN(`map_id`) AS `since` FROM `entries` GROUP BY `kind`, `pid`) `ids`
    JOIN `m` ON `m`.`id` >= `ids`.`since`
    LEFT JOIN `entries` `e` ON `e`.`kind` = `ids`.`kind` AND `e`.`pid` = `ids`.`pid` AND `e`.`map_id` = `m`.`id`
  ),
  `steps` AS (
    SELECT `grid`.*,
      LAG(`doc`) OVER (PARTITION BY `kind`, `pid` ORDER BY `map_id`) AS `prev`,
      ROW_NUMBER() OVER (PARTITION BY `kind`, `pid` ORDER BY `map_id`) AS `step`
    FROM `grid`
  ),
  -- a version where the entry appears, changes, leaves (the tombstone) or comes back
  `kept` AS (
    SELECT `kind`, `pid`, `map_id`, `author`, `created_at`, `message`, COALESCE(`ord`, 9999) AS `ord`,
      IF(`doc` IS NULL, JSON_COMPACT(JSON_SET(`prev`, '$.removed', TRUE)), `doc`) AS `doc`
    FROM `steps`
    WHERE (`doc` IS NOT NULL AND (`step` = 1 OR `prev` IS NULL OR `doc` <> `prev`)) OR (`doc` IS NULL AND `prev` IS NOT NULL)
  ),
  `marked` AS (
    SELECT `kept`.*, MAX(IF(`map_id` <= (SELECT `id` FROM `pub`), `map_id`, NULL)) OVER (PARTITION BY `kind`, `pid`) AS `live`
    FROM `kept`
  )
SELECT CONCAT(`kind`, ':', `pid`), `kind`, NULL, NULL, `doc`, `author`, `created_at`,
  IF(`map_id` = `live`, 'published', 'draft'),
  LEFT(CONCAT('map v', `map_id`, IF(`message` IS NULL OR `message` = '', '', CONCAT(': ', `message`))), 255)
FROM `marked`
ORDER BY `map_id`, `ord`, `kind`;

UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(JSON_REMOVE(`json`, '$.locations'), '$.routes'))
  WHERE `entity_key` = 'map' AND (JSON_EXISTS(`json`, '$.locations') OR JSON_EXISTS(`json`, '$.routes'));
