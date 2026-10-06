-- Jump targets: a numeric `value` becomes a string (`{"type":"chapter","value":2}` → `"value":"2"`). JumpToTarget.value is
-- a string (shared/content content.ts, the JSON schema the server checks saves against); the old chapter 2 stroll's
-- choices stored the number, so `content:check` reports those versions as not fitting their type.
-- Analysis 2026-10-06 on the dev DB (= the latest production dump, MariaDB 10.11.14): 11 versions, all
-- `scene:catpope:2:scene-14` (+ its de / en files; drafts 10–93, published 127 / 128 / 201), 3 targets each – every
-- numeric target value there sits right after its `"type"` (a JSON walk counted the same 33); content_versions empty.
-- Only jump target types are matched (chapter, season, toon, product, xr), only a number right after `"type":"…",`.
-- Escaping: plain string literals (no NO_BACKSLASH_ESCAPES): '\\1' is the first group. Idempotent (a second run matches
-- no rows). PHPUnit EntityStoreTest::testTheTargetValueMigration.
-- phpMyAdmin: select the database (d0487143), SQL tab. Check first (count = rows to change):
--   SELECT COUNT(*) FROM `entity_versions` WHERE `json` REGEXP '"type":"(chapter|season|toon|product|xr)","value":-?[0-9]';
--   SELECT COUNT(*) FROM `content_versions` WHERE `json` REGEXP '"type":"(chapter|season|toon|product|xr)","value":-?[0-9]';
-- then run the file as it is; the same SELECTs answer 0 afterwards.
UPDATE `entity_versions` SET `json` = REGEXP_REPLACE(`json`, '("type":"(chapter|season|toon|product|xr)","value":)(-?[0-9]+(\\.[0-9]+)?)', '\\1"\\3"') WHERE `json` REGEXP '"type":"(chapter|season|toon|product|xr)","value":-?[0-9]';
UPDATE `content_versions` SET `json` = REGEXP_REPLACE(`json`, '("type":"(chapter|season|toon|product|xr)","value":)(-?[0-9]+(\\.[0-9]+)?)', '\\1"\\3"') WHERE `json` REGEXP '"type":"(chapter|season|toon|product|xr)","value":-?[0-9]';
