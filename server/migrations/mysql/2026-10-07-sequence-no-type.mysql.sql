-- A season's `sequence` holds chapters only (owner 2026-10-07, BACKLOG 10.42): every entry was `{"type":"chapter",
-- "number":n,"status":…}`; the `type` goes. There is no alias: content types, schemas, validation (an entry with a `type`
-- is an error), API, viewer and creator know only `{ number, status }` now, so every stored version is rewritten
-- (history included: undo / restore must not bring the old form back).
-- Analysis 2026-10-07 on the dev DB (= the latest production dump): 49 season versions in entity_versions, 374 sequence
-- entries, all `"type":"chapter"` and always the entry's first key (`{"type":"chapter",`, 374 occurrences); `"type":`
-- occurs nowhere else in a season document (374 in all). Stored JSON is compact; a text can't hold `{"type":"chapter",`
-- unescaped (JSON escapes the quotes). content_versions (the pre-entity table): 0 rows – cleaned the same way
-- (season files are `season<n>.json` there).
-- Idempotent (a second run matches no rows).
-- phpMyAdmin: select the database (d0487143), SQL tab. Check first – must answer 0 (a sequence entry that isn't a
-- chapter; then stop and ask):
--   SELECT COUNT(*) FROM `entity_versions` e, JSON_TABLE(e.`json`, '$.sequence[*]' COLUMNS(`t` VARCHAR(64) PATH '$.type')) s WHERE e.`kind` = 'season' AND s.`t` <> 'chapter';
-- and the rows to change (answers 0 afterwards; the second update covers an entry whose `type` isn't the first key):
--   SELECT COUNT(*) FROM `entity_versions` WHERE `kind` = 'season' AND `json` LIKE '%"type":"chapter"%';
-- then run the file as it is.
UPDATE `entity_versions` SET `json` = REPLACE(`json`, '{"type":"chapter",', '{') WHERE `kind` = 'season' AND `json` LIKE '%{"type":"chapter",%';
UPDATE `entity_versions` SET `json` = REPLACE(`json`, ',"type":"chapter"', '') WHERE `kind` = 'season' AND `json` LIKE '%,"type":"chapter"%';
UPDATE `content_versions` SET `json` = REPLACE(`json`, '{"type":"chapter",', '{') WHERE `path` REGEXP '^season[0-9]+[.]json$' AND `json` LIKE '%{"type":"chapter",%';
UPDATE `content_versions` SET `json` = REPLACE(`json`, ',"type":"chapter"', '') WHERE `path` REGEXP '^season[0-9]+[.]json$' AND `json` LIKE '%,"type":"chapter"%';
