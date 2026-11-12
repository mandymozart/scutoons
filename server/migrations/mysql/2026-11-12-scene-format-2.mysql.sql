-- Scene documents move to format 2 (#489, agents/research/SCENES.md §9.3): a top-level `location`, `holder[]` (a filler or a
-- placeholder's `fill`, spawn zones), the scene's own graph; the being's look left the document (a character holder brings
-- its own). The core reads format 2 only (RULES A9, E2: the old form leaves the code, no alias), so every stored version of a
-- `scene:<id>` (drafts, published, history – a restore must stay playable) moves here, in two steps:
-- 1. `scene:nave` – the first real scene, created in production from 1a's fixture (server/tests/fixtures/scenes/nave.json
--    before #489: the core rig `character.catpope` flattened by hand, staged as the cathedral): it becomes what the fixture
--    is now – one holder `catpope` (`ref` character:catpope, `rig` rig:character.catpope, the current version), its own
--    graph its `space` node alone (the cathedral's set and camera, as stored), no links; the hand-flattened nodes leave
--    (the resolve step flattens the rig in, to the same scene). Its stage, toons and stamped version stay.
-- 2. any other format-1 scene (none expected): `format` 2 and its `being` removed – its graph stays the scene's own; a
--    character its look named is not made a holder (its rig's nodes are in the graph already).
-- What it touches: entity_versions rows of kind 'scene' (keys `scene:<id>`; the archived chapter groups
-- `scene:<toon>:<chapter>:<name>` live in entity_versions_legacy since 2026-10-19 and are excluded twice).
-- Dry run (before): SELECT id, entity_key, status, JSON_VALUE(json, '$.format') FROM entity_versions WHERE kind = 'scene';
--   → expected on production: none, or the versions of scene:nave with format 1.
-- Test (after): the same query → format 2 everywhere; scene:nave's JSON_EXTRACT(json, '$.holder[0].ref') = "character:catpope".
-- Plan's row counts: entity_versions = the format-1 scene versions (0 – a few). MariaDB 10.11, idempotent: a moved row says
-- format 2 and is skipped on a second run (server/tests/Unit/Domain/SceneFormat2MigrationTest.php).

UPDATE `entity_versions`
SET `json` = JSON_SET(JSON_REMOVE(`json`, '$.being'),
  '$.format', 2,
  '$.holder', JSON_ARRAY(JSON_OBJECT('id', 'catpope', 'ref', 'character:catpope', 'rig', JSON_OBJECT('ref', 'rig:character.catpope'))),
  '$.node', JSON_ARRAY(JSON_QUERY(`json`, REPLACE(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'space', NULL, '$.node[*].kind')), '.kind', ''))),
  '$.link', JSON_ARRAY())
WHERE `kind` = 'scene' AND `entity_key` = 'scene:nave' AND JSON_VALID(`json`) AND JSON_VALUE(`json`, '$.format') = '1'
  AND JSON_SEARCH(`json`, 'one', 'space', NULL, '$.node[*].kind') IS NOT NULL;

UPDATE `entity_versions`
SET `json` = JSON_SET(JSON_REMOVE(`json`, '$.being'), '$.format', 2)
WHERE `kind` = 'scene' AND `entity_key` NOT LIKE 'scene:%:%' AND JSON_VALID(`json`) AND JSON_VALUE(`json`, '$.format') = '1';
