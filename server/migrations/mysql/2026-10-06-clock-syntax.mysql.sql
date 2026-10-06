-- Texts: the old clock form `{name:clock}` becomes the expression `{clock(name)}` (agents/EXPRESSIONS.md §1, owner
-- 2026-10-06: ink's inline syntax; the old form is gone from the code – no alias, no load-time conversion). In ink syntax
-- `{time:clock}` would be a conditional ("if time, then the text `clock`"), so every stored version is rewritten (history
-- included: undo / restore must not bring the old form back). Validation calls the old form an error from now on.
-- Analysis 2026-10-06 on the dev DB (= the latest production dump, MariaDB 10.11.14): 0 occurrences in entity_versions
-- (449 versions) and content_versions – not even `:clock` anywhere; the migration is for production's newer test content.
-- Only a plain variable name before `:clock` is rewritten (`[A-Za-z_][A-Za-z0-9_]*`, exactly what the old code read);
-- `{x:clock2}`, `{a b:clock}` stay. Stored JSON keeps `{`, `}` and `:` unescaped, so the pattern matches the JSON text.
-- Escaping: plain string literals (the server's sql_mode has no NO_BACKSLASH_ESCAPES): '\\{' is the regex `\{`, '\\1'
-- the first group (tested on 10.11.14 with sql_mode NO_ENGINE_SUBSTITUTION and the API's strict mode; PHPUnit
-- EntityStoreTest::testTheClockSyntaxMigration).
-- Idempotent (a second run matches no rows).
-- phpMyAdmin: select the database (d0487143), SQL tab. Check first (count = rows to change):
--   SELECT COUNT(*) FROM `entity_versions` WHERE `json` REGEXP '\\{[A-Za-z_][A-Za-z0-9_]*:clock\\}';
--   SELECT COUNT(*) FROM `content_versions` WHERE `json` REGEXP '\\{[A-Za-z_][A-Za-z0-9_]*:clock\\}';
-- then run the file as it is; the same SELECTs answer 0 afterwards.
UPDATE `entity_versions` SET `json` = REGEXP_REPLACE(`json`, '\\{([A-Za-z_][A-Za-z0-9_]*):clock\\}', '{clock(\\1)}') WHERE `json` REGEXP '\\{[A-Za-z_][A-Za-z0-9_]*:clock\\}';
UPDATE `content_versions` SET `json` = REGEXP_REPLACE(`json`, '\\{([A-Za-z_][A-Za-z0-9_]*):clock\\}', '{clock(\\1)}') WHERE `json` REGEXP '\\{[A-Za-z_][A-Za-z0-9_]*:clock\\}';
