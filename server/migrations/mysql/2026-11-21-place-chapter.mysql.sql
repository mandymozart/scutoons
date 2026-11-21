-- The world map's place → chapter link leaves the stored places (#710, owner 2026-11-20 [scenes]: "kind of against our
-- dogma where we go to ids across the universe"): every stored version of a `place:<id>` (drafts, published, history)
-- loses `chapter` ({ toon, number }); the reader's card link, the creator's field and the Flow's place boxes went with
-- the code (RULES E2). Routes keep `toons`: /library/map sends a route only while one of its toons is visible.
-- Dry run (before): SELECT id, entity_key, status FROM entity_versions WHERE kind = 'place' AND JSON_VALID(json) AND JSON_EXISTS(json, '$.chapter');
-- Test (after): the same query → no rows. Idempotent: a second run finds nothing (server/tests/Unit/Domain/PlaceChapterMigrationTest.php).

UPDATE `entity_versions`
SET `json` = JSON_REMOVE(`json`, '$.chapter')
WHERE `kind` = 'place' AND JSON_VALID(`json`) AND JSON_EXISTS(`json`, '$.chapter');
