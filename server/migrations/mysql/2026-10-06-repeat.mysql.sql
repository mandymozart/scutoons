-- Events: the boolean `once` (a slice trigger's `trigger.once`, a placed item's `once` for its tap events) becomes the
-- repeat rule `repeat` (agents/EXPRESSIONS.md §5, step 3: every time = no `repeat` – the new default –, once per page
-- view = {"mode":"view"}, once for good = {"mode":"once"}, every n … = {"mode":"interval","every":{…}}). The old field is
-- gone from the code (no alias, validation calls it an error), so every stored version is rewritten (history included:
-- undo / restore must not bring it back). Until now a missing `once` meant true (once per page view) – those events get
-- {"mode":"view"} too, so nothing changes for readers.
-- Analysis 2026-10-06 on the dev DB (= the latest production dump, 457 versions in entity_versions, 0 in content_versions):
-- 221 slice events with a trigger: 166 `"once":true`, 0 `"once":false`, 55 without; 7 placed items (4 tap events, 3
-- pickups), none with `once`. The quoted `"once":` occurs nowhere else (a text can't hold it unescaped: JSON escapes the
-- quotes). Audio samples have a `trigger` too (always with `mode`, never a top-level `type`) – they are not touched.
-- How (MariaDB's PCRE2; stored JSON is compact):
--   1. a VN slice event's trigger without once / repeat (an object with a top-level `"type":"…"` and `"trigger":{…}` –
--      samples have no top-level type) gets `,"repeat":{"mode":"view"}` at its end. The pattern walks the JSON exactly:
--      a DEFINE block for strings (escapes included), objects and arrays (recursion), so only top-level keys count.
--   2. a placed item with `"action":"events"` and no top-level once / repeat gets `,"repeat":{"mode":"view"}`.
--   3. every `"once":true` left (triggers, placed items) → `"repeat":{"mode":"view"}`.
--   4. every `"once":false` → `"repeat":{"mode":"every"}` (every time – the default, but written out: a trigger without
--      once / repeat is what step 1 converts, so a second run must not see one).
-- Checked on a copy of the dev DB: the result equals a JSON-walking reference transform for all 457 versions (81 changed,
-- valid JSON, no duplicate keys, samples untouched; < 1 s). PHPUnit EntityStoreTest::testTheRepeatMigration (edge
-- cases, the API's strict sql_mode). Escaping: plain string literals (no NO_BACKSLASH_ESCAPES), '\{' is the regex `\{`.
-- Idempotent (a second run matches no rows).
-- phpMyAdmin: select the database (d0487143), SQL tab. Check first – rows to change (all three answer 0 afterwards):
--   SELECT COUNT(*) FROM `entity_versions` WHERE `json` LIKE '%"once":%';
--   SELECT COUNT(*) FROM `entity_versions` WHERE `json` LIKE '%"trigger":%' AND `json` REGEXP '(?(DEFINE)(?<s>"(?:[^"\\\\]|\\\\.)*+")(?<o>\\{(?:[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a))*+\\})(?<a>\\[(?:[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a))*+\\])(?<t>[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a)))\\{(?=(?&t)*?"type":")((?&t)*?"trigger":\\{)((?:[^{}"]++|"(?!(?:once|repeat)")(?:[^"\\\\]|\\\\.)*+"|\\{[^{}]*+\\})++)\\}';
--   SELECT COUNT(*) FROM `entity_versions` WHERE `json` LIKE '%"action":"events"%' AND `json` REGEXP '(?(DEFINE)(?<s>"(?:[^"\\\\]|\\\\.)*+")(?<o>\\{(?:[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a))*+\\})(?<a>\\[(?:[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a))*+\\])(?<t>[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a)))\\{(?=(?&t)*?"action":"events")(?!(?&t)*?"(?:once|repeat)":)((?&t)*+)\\}';
-- (content_versions: the same), then run the file as it is.
UPDATE `entity_versions` SET `json` = REGEXP_REPLACE(`json`, '(?(DEFINE)(?<s>"(?:[^"\\\\]|\\\\.)*+")(?<o>\\{(?:[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a))*+\\})(?<a>\\[(?:[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a))*+\\])(?<t>[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a)))\\{(?=(?&t)*?"type":")((?&t)*?"trigger":\\{)((?:[^{}"]++|"(?!(?:once|repeat)")(?:[^"\\\\]|\\\\.)*+"|\\{[^{}]*+\\})++)\\}', '{\\5\\6,"repeat":{"mode":"view"}}') WHERE `json` LIKE '%"trigger":%' AND `json` REGEXP '(?(DEFINE)(?<s>"(?:[^"\\\\]|\\\\.)*+")(?<o>\\{(?:[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a))*+\\})(?<a>\\[(?:[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a))*+\\])(?<t>[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a)))\\{(?=(?&t)*?"type":")((?&t)*?"trigger":\\{)((?:[^{}"]++|"(?!(?:once|repeat)")(?:[^"\\\\]|\\\\.)*+"|\\{[^{}]*+\\})++)\\}';
UPDATE `entity_versions` SET `json` = REGEXP_REPLACE(`json`, '(?(DEFINE)(?<s>"(?:[^"\\\\]|\\\\.)*+")(?<o>\\{(?:[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a))*+\\})(?<a>\\[(?:[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a))*+\\])(?<t>[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a)))\\{(?=(?&t)*?"action":"events")(?!(?&t)*?"(?:once|repeat)":)((?&t)*+)\\}', '{\\5,"repeat":{"mode":"view"}}') WHERE `json` LIKE '%"action":"events"%' AND `json` REGEXP '(?(DEFINE)(?<s>"(?:[^"\\\\]|\\\\.)*+")(?<o>\\{(?:[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a))*+\\})(?<a>\\[(?:[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a))*+\\])(?<t>[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a)))\\{(?=(?&t)*?"action":"events")(?!(?&t)*?"(?:once|repeat)":)((?&t)*+)\\}';
UPDATE `entity_versions` SET `json` = REPLACE(`json`, '"once":true', '"repeat":{"mode":"view"}') WHERE `json` LIKE '%"once":true%';
UPDATE `entity_versions` SET `json` = REPLACE(`json`, '"once":false', '"repeat":{"mode":"every"}') WHERE `json` LIKE '%"once":false%';
UPDATE `content_versions` SET `json` = REGEXP_REPLACE(`json`, '(?(DEFINE)(?<s>"(?:[^"\\\\]|\\\\.)*+")(?<o>\\{(?:[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a))*+\\})(?<a>\\[(?:[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a))*+\\])(?<t>[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a)))\\{(?=(?&t)*?"type":")((?&t)*?"trigger":\\{)((?:[^{}"]++|"(?!(?:once|repeat)")(?:[^"\\\\]|\\\\.)*+"|\\{[^{}]*+\\})++)\\}', '{\\5\\6,"repeat":{"mode":"view"}}') WHERE `json` LIKE '%"trigger":%' AND `json` REGEXP '(?(DEFINE)(?<s>"(?:[^"\\\\]|\\\\.)*+")(?<o>\\{(?:[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a))*+\\})(?<a>\\[(?:[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a))*+\\])(?<t>[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a)))\\{(?=(?&t)*?"type":")((?&t)*?"trigger":\\{)((?:[^{}"]++|"(?!(?:once|repeat)")(?:[^"\\\\]|\\\\.)*+"|\\{[^{}]*+\\})++)\\}';
UPDATE `content_versions` SET `json` = REGEXP_REPLACE(`json`, '(?(DEFINE)(?<s>"(?:[^"\\\\]|\\\\.)*+")(?<o>\\{(?:[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a))*+\\})(?<a>\\[(?:[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a))*+\\])(?<t>[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a)))\\{(?=(?&t)*?"action":"events")(?!(?&t)*?"(?:once|repeat)":)((?&t)*+)\\}', '{\\5,"repeat":{"mode":"view"}}') WHERE `json` LIKE '%"action":"events"%' AND `json` REGEXP '(?(DEFINE)(?<s>"(?:[^"\\\\]|\\\\.)*+")(?<o>\\{(?:[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a))*+\\})(?<a>\\[(?:[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a))*+\\])(?<t>[^{}\\[\\]"]++|(?&s)|(?&o)|(?&a)))\\{(?=(?&t)*?"action":"events")(?!(?&t)*?"(?:once|repeat)":)((?&t)*+)\\}';
UPDATE `content_versions` SET `json` = REPLACE(`json`, '"once":true', '"repeat":{"mode":"view"}') WHERE `json` LIKE '%"once":true%';
UPDATE `content_versions` SET `json` = REPLACE(`json`, '"once":false', '"repeat":{"mode":"every"}') WHERE `json` LIKE '%"once":false%';
