-- Chapter groups get stable ids (#706, owner 2026-11-20 [scenes]: "Group label is becoming id … we can just ditch
-- backwards compatibility"): in every stored chapter version (drafts, published, history) each group's `label` becomes
-- its `id` – the same value, so every jump target, `?label=` link and saved reader state (positional) stays valid. A group
-- without a label gets `g<n>` from its position (1-based), or `group-<n>` when another group of that chapter is already
-- labelled `g<n>`. A group that already has an `id` keeps it and only loses `label`. Nothing else of a document changes.
-- The code no longer knows `Group.label` (RULES E2: the old form leaves the code, no alias).
-- A chapter's `$.groups` is rebuilt in one pass (JSON_TABLE → JSON_ARRAYAGG in group order, as 2026-10-16-endless-strip);
-- JSON_EXTRACT(…, '$') keeps each rebuilt value JSON; JSON_ARRAYAGG is capped by group_concat_max_len, raised for the
-- session. A group that isn't an object stays as it is.
-- Analysis 2026-11-21 on the dev DB (the production seed): 297 chapter versions, 681 groups, every one labelled, every
-- label slug-safe (^[A-Za-z0-9][A-Za-z0-9_-]*$) and unique in its chapter – so the ids are exactly today's labels.
-- Dry run (before):
--   SELECT id, entity_key, status FROM entity_versions WHERE kind = 'chapter' AND JSON_VALID(json)
--     AND EXISTS (SELECT 1 FROM JSON_TABLE(json, '$.groups[*]' COLUMNS (g JSON PATH '$', label JSON PATH '$.label', gid JSON PATH '$.id')) x
--       WHERE JSON_TYPE(x.g) = 'OBJECT' AND (x.label IS NOT NULL OR x.gid IS NULL));
-- Test (after): the same query → no rows. Idempotent: a second run finds nothing (server/tests/Unit/Domain/GroupIdsMigrationTest.php).

SET SESSION group_concat_max_len = 16777216;

UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_SET(`json`, '$.groups', JSON_EXTRACT((
    SELECT JSON_ARRAYAGG(JSON_EXTRACT(IF(JSON_TYPE(g.`grp`) = 'OBJECT',
        JSON_INSERT(JSON_REMOVE(g.`grp`, '$.label'), '$.id',
          CASE
            WHEN JSON_TYPE(JSON_EXTRACT(g.`grp`, '$.id')) = 'STRING' THEN JSON_UNQUOTE(JSON_EXTRACT(g.`grp`, '$.id'))
            WHEN g.`label` IS NOT NULL AND g.`label` <> '' THEN g.`label`
            WHEN JSON_SEARCH(`json`, 'one', CONCAT('g', g.`ord`), NULL, '$.groups[*].label') IS NOT NULL THEN CONCAT('group-', g.`ord`)
            ELSE CONCAT('g', g.`ord`)
          END),
        g.`grp`), '$') ORDER BY g.`ord`)
    FROM JSON_TABLE(`json`, '$.groups[*]' COLUMNS (`ord` FOR ORDINALITY, `grp` JSON PATH '$', `label` VARCHAR(255) PATH '$.label')) g), '$')))
WHERE `kind` = 'chapter' AND JSON_VALID(`json`) AND JSON_TYPE(JSON_EXTRACT(`json`, '$.groups')) = 'ARRAY'
  AND EXISTS (SELECT 1 FROM JSON_TABLE(`json`, '$.groups[*]' COLUMNS (`g` JSON PATH '$', `label` JSON PATH '$.label', `gid` JSON PATH '$.id')) x
    WHERE JSON_TYPE(x.`g`) = 'OBJECT' AND (x.`label` IS NOT NULL OR x.`gid` IS NULL));
