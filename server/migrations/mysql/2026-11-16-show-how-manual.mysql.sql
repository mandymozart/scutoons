-- Show documents: a row's `fire.how` "hand" is now "manual" (#597; the core reads "manual" only, RULES E2: no alias). Every
-- stored version of a `show:<id>` (drafts, published, history) is rewritten; the editor saves through EntityStore's
-- json_encode (no spaces), the spaced form covers a hand-made import.
-- Dry run (before): SELECT id, entity_key, status FROM entity_versions WHERE kind = 'show' AND (json LIKE '%"how":"hand"%' OR json LIKE '%"how": "hand"%');
-- Test (after): the same query → no rows. Idempotent: a second run finds nothing (server/tests/Unit/Domain/ShowHowManualMigrationTest.php).

UPDATE `entity_versions`
SET `json` = REPLACE(REPLACE(`json`, '"how":"hand"', '"how":"manual"'), '"how": "hand"', '"how": "manual"')
WHERE `kind` = 'show' AND (`json` LIKE '%"how":"hand"%' OR `json` LIKE '%"how": "hand"%');
