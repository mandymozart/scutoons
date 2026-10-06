-- The VN event `showOnomatopeia` is renamed `showTextBox` (owner 2026-10-06: it reveals ANY text box of the slice with an
-- effect, not only an onomatopoeia; the old name was also misspelt). There is no alias: content, validation, schemas,
-- viewer and creator know only `showTextBox` now, so every stored version is rewritten (history included: undo /
-- restore must not bring the old type back).
-- Stored JSON is compact (`"type":"showOnomatopeia"`); the quoted string `"showOnomatopeia"` occurs only as that type
-- value (checked on the production seed 2026-10-06: 31 occurrences in entity_versions, all `"type":"showOnomatopeia"`,
-- none escaped, none inside a text). content_versions (the pre-entity table) is cleaned the same way.
-- Idempotent (a second run matches no rows).
-- phpMyAdmin: select the database (d0487143), SQL tab. Check first (count = rows to change):
--   SELECT COUNT(*) FROM `entity_versions` WHERE `json` LIKE '%"showOnomatopeia"%';
-- then run the file as it is; the same SELECT answers 0 afterwards.
UPDATE `entity_versions` SET `json` = REPLACE(`json`, '"showOnomatopeia"', '"showTextBox"') WHERE `json` LIKE '%"showOnomatopeia"%';
UPDATE `content_versions` SET `json` = REPLACE(`json`, '"showOnomatopeia"', '"showTextBox"') WHERE `json` LIKE '%"showOnomatopeia"%';
