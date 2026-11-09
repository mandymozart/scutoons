-- A rig's `sticks` are addresses (#432, research/MODULE-WIRING.md §3): `<track>:<node>.<port>` instead of the compiled
-- slot `<track>_<node>.<port>` the documents held so far (`voice_voice.ratio` → `voice:voice.ratio`). The core refuses the
-- old form ("names no track"), so every stored rig moves with the code. The split is the rig's own track list: the
-- longest track id that prefixes the stick followed by `_` (a track id may hold a `_`; a stick that matches no track –
-- or already holds a `:`, or is "" – is left as it is). Nothing else of the document changes.
-- What it touches: catalog_rigs.data (deck-space rig::Rig, `sticks` = [[x, y], [x, y]]); the core rig rows of
-- 2026-11-01-rigs_core land in the old form and are moved here too. catalog_history keeps its past states. member_rigs
-- (2026-11-01-rigs) was dropped by 2026-11-02-drop-member-layer (#302: readers save nothing), so there is no second table.
-- Dry run (before): SELECT id, JSON_EXTRACT(data, '$.sticks') FROM catalog_rigs WHERE JSON_TYPE(JSON_EXTRACT(data, '$.sticks')) = 'ARRAY';
--   → expected on production: one row, character.catpope, [["voice_voice.ratio","voice_voice.index"],["voice_melody.spread","drone_drone.freq"]].
-- Test (after): the same query → [["voice:voice.ratio","voice:voice.index"],["voice:melody.spread","drone:drone.freq"]].
-- Plan's row counts: catalog_rigs 1. MariaDB 10.11 (JSON_TABLE in a dependent subquery, as 2026-10-06-global-save-game),
-- idempotent: a moved stick holds a `:` and is skipped on a second run. COALESCE: JSON_VALUE gives NULL for "" (an empty stick).

UPDATE `catalog_rigs` r
  JOIN (
    SELECT r2.`id`,
      (SELECT t.`id` FROM JSON_TABLE(r2.`data`, '$.track[*]' COLUMNS (`id` VARCHAR(100) PATH '$.id')) t
        WHERE SUBSTRING(JSON_VALUE(r2.`data`, '$.sticks[0][0]'), 1, CHAR_LENGTH(t.`id`) + 1) = CONCAT(t.`id`, '_')
        ORDER BY CHAR_LENGTH(t.`id`) DESC LIMIT 1) AS t00,
      (SELECT t.`id` FROM JSON_TABLE(r2.`data`, '$.track[*]' COLUMNS (`id` VARCHAR(100) PATH '$.id')) t
        WHERE SUBSTRING(JSON_VALUE(r2.`data`, '$.sticks[0][1]'), 1, CHAR_LENGTH(t.`id`) + 1) = CONCAT(t.`id`, '_')
        ORDER BY CHAR_LENGTH(t.`id`) DESC LIMIT 1) AS t01,
      (SELECT t.`id` FROM JSON_TABLE(r2.`data`, '$.track[*]' COLUMNS (`id` VARCHAR(100) PATH '$.id')) t
        WHERE SUBSTRING(JSON_VALUE(r2.`data`, '$.sticks[1][0]'), 1, CHAR_LENGTH(t.`id`) + 1) = CONCAT(t.`id`, '_')
        ORDER BY CHAR_LENGTH(t.`id`) DESC LIMIT 1) AS t10,
      (SELECT t.`id` FROM JSON_TABLE(r2.`data`, '$.track[*]' COLUMNS (`id` VARCHAR(100) PATH '$.id')) t
        WHERE SUBSTRING(JSON_VALUE(r2.`data`, '$.sticks[1][1]'), 1, CHAR_LENGTH(t.`id`) + 1) = CONCAT(t.`id`, '_')
        ORDER BY CHAR_LENGTH(t.`id`) DESC LIMIT 1) AS t11
    FROM `catalog_rigs` r2
    WHERE JSON_TYPE(JSON_EXTRACT(r2.`data`, '$.sticks')) = 'ARRAY'
      AND JSON_LENGTH(r2.`data`, '$.sticks[0]') = 2 AND JSON_LENGTH(r2.`data`, '$.sticks[1]') = 2
  ) s ON s.`id` = r.`id`
SET r.`data` = JSON_SET(r.`data`,
  '$.sticks[0][0]', IF(s.t00 IS NULL OR LOCATE(':', JSON_VALUE(r.`data`, '$.sticks[0][0]')) > 0, COALESCE(JSON_VALUE(r.`data`, '$.sticks[0][0]'), ''),
                      CONCAT(s.t00, ':', SUBSTRING(JSON_VALUE(r.`data`, '$.sticks[0][0]'), CHAR_LENGTH(s.t00) + 2))),
  '$.sticks[0][1]', IF(s.t01 IS NULL OR LOCATE(':', JSON_VALUE(r.`data`, '$.sticks[0][1]')) > 0, COALESCE(JSON_VALUE(r.`data`, '$.sticks[0][1]'), ''),
                      CONCAT(s.t01, ':', SUBSTRING(JSON_VALUE(r.`data`, '$.sticks[0][1]'), CHAR_LENGTH(s.t01) + 2))),
  '$.sticks[1][0]', IF(s.t10 IS NULL OR LOCATE(':', JSON_VALUE(r.`data`, '$.sticks[1][0]')) > 0, COALESCE(JSON_VALUE(r.`data`, '$.sticks[1][0]'), ''),
                      CONCAT(s.t10, ':', SUBSTRING(JSON_VALUE(r.`data`, '$.sticks[1][0]'), CHAR_LENGTH(s.t10) + 2))),
  '$.sticks[1][1]', IF(s.t11 IS NULL OR LOCATE(':', JSON_VALUE(r.`data`, '$.sticks[1][1]')) > 0, COALESCE(JSON_VALUE(r.`data`, '$.sticks[1][1]'), ''),
                      CONCAT(s.t11, ':', SUBSTRING(JSON_VALUE(r.`data`, '$.sticks[1][1]'), CHAR_LENGTH(s.t11) + 2))))
WHERE s.t00 IS NOT NULL OR s.t01 IS NOT NULL OR s.t10 IS NOT NULL OR s.t11 IS NOT NULL;
