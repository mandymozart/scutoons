-- Show documents: the one `toon` becomes the list `toons` (#708: a show uses several toons, the first its home). Every
-- stored version of a `show:<id>` (drafts, published, history) is rewritten: `toons` = [the toon] ([] for an empty one),
-- `toon` removed; a document that has `toons` already keeps it. The core still reads a `toon` (a deck's kept file).
-- Dry run (before): SELECT id, entity_key, status FROM entity_versions WHERE kind = 'show' AND JSON_EXISTS(json, '$.toon');
-- Test (after): the same query → no rows. Idempotent: a second run finds nothing (server/tests/Unit/Domain/ShowToonsMigrationTest.php).

UPDATE `entity_versions`
SET `json` = JSON_REMOVE(JSON_INSERT(`json`, '$.toons', IF(JSON_TYPE(JSON_EXTRACT(`json`, '$.toon')) = 'STRING' AND JSON_VALUE(`json`, '$.toon') <> '', JSON_ARRAY(JSON_VALUE(`json`, '$.toon')), JSON_ARRAY())), '$.toon')
WHERE `kind` = 'show' AND JSON_VALID(`json`) AND JSON_EXISTS(`json`, '$.toon');
