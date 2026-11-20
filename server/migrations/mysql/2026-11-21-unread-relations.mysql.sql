-- Relation fields nothing reads leave the stored content (#709, owner 2026-11-20 [scenes]): every stored version
-- (drafts, published, history) loses
--   events            an event's `toon` (the toons it belonged to, never shown)            `events`
--   character:<id>    a persona's `introduced` ({ toon, season?, chapter? }, editor only)    `$.personas[*].introduced`
--   chapter:<toon>:…  the chapter's `featuring` (neutral and per-locale documents)          `$.featuring`
-- The code no longer knows these fields (RULES E2: the old form leaves the code). A list is rebuilt from its entries
-- in order, each entry without the field; nothing else of a document changes.
-- Dry run (before):
--   SELECT id, entity_key, status FROM entity_versions WHERE kind = 'events' AND JSON_VALID(json) AND json LIKE '%"toon"%';
--   SELECT id, entity_key, status FROM entity_versions WHERE kind = 'character' AND JSON_VALID(json) AND json LIKE '%"introduced"%';
--   SELECT id, entity_key, status FROM entity_versions WHERE kind = 'chapter' AND JSON_VALID(json) AND JSON_EXISTS(json, '$.featuring');
-- Test (after): the same queries → no rows. Idempotent: a second run finds nothing (server/tests/Unit/Domain/UnreadRelationsMigrationTest.php).

UPDATE `entity_versions`
SET `json` = (
  SELECT CONCAT('[', GROUP_CONCAT(JSON_COMPACT(JSON_REMOVE(t.`e`, '$.toon')) ORDER BY t.`ord` SEPARATOR ','), ']')
  FROM JSON_TABLE(`json`, '$[*]' COLUMNS (`ord` FOR ORDINALITY, `e` JSON PATH '$')) t
)
WHERE `kind` = 'events' AND JSON_VALID(`json`) AND JSON_TYPE(`json`) = 'ARRAY'
  AND EXISTS (SELECT 1 FROM JSON_TABLE(`json`, '$[*]' COLUMNS (`toon` JSON PATH '$.toon')) x WHERE x.`toon` IS NOT NULL);

UPDATE `entity_versions`
SET `json` = JSON_SET(`json`, '$.personas', JSON_EXTRACT((
  SELECT CONCAT('[', GROUP_CONCAT(JSON_COMPACT(JSON_REMOVE(t.`p`, '$.introduced')) ORDER BY t.`ord` SEPARATOR ','), ']')
  FROM JSON_TABLE(`json`, '$.personas[*]' COLUMNS (`ord` FOR ORDINALITY, `p` JSON PATH '$')) t
), '$'))
WHERE `kind` = 'character' AND JSON_VALID(`json`) AND JSON_TYPE(JSON_EXTRACT(`json`, '$.personas')) = 'ARRAY'
  AND EXISTS (SELECT 1 FROM JSON_TABLE(`json`, '$.personas[*]' COLUMNS (`introduced` JSON PATH '$.introduced')) x WHERE x.`introduced` IS NOT NULL);

UPDATE `entity_versions`
SET `json` = JSON_REMOVE(`json`, '$.featuring')
WHERE `kind` = 'chapter' AND JSON_VALID(`json`) AND JSON_EXISTS(`json`, '$.featuring');
