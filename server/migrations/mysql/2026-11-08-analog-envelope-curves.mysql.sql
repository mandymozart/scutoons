-- The analog factory module's envelope times are logarithmic (#429): 12 lines of `params.rs` gain `curve = log`, so that the
-- catalog's `module:analog` keeps the repository's files byte for byte (server tests/Unit/Domain/FactoryModulesTest). Only the
-- knob's travel changes, not the sound. The sources only: the descriptor and the wasm in the entry stay as they are until an
-- admin presses Compile on module:analog (POST /api/modules/compile { factory: "analog" }); until then the deck plays the build it has.
-- What it touches: catalog_modules: the row `analog` when it is a factory module. entity_versions: the factory's published
-- version of `module:analog` (the same text; no new version: a factory module is the repository's and nobody edits it).
-- Dry run (before): SELECT id, LOCATE('curve = log, unit = \\"s\\", doc = \\"filter envelope', data) FROM catalog_modules WHERE id = 'analog';
--   → expected: one row, 0.
-- Test (after): the same query → one row, not 0.
-- Plan's row counts: 1 · 1. MariaDB 10.11, idempotent: LOCATE guards a second run (a new text still holds the searched one).
-- Escaping: plain string literals (no NO_BACKSLASH_ESCAPES): `\\` is one backslash of the JSON text.

UPDATE `catalog_modules`
   SET `data` = REPLACE(REPLACE(`data`,
         ', unit = \\"s\\", doc = \\"filter envelope', ', curve = log, unit = \\"s\\", doc = \\"filter envelope'),
         ', unit = \\"s\\", doc = \\"amp envelope', ', curve = log, unit = \\"s\\", doc = \\"amp envelope')
 WHERE `id` = 'analog' AND JSON_CONTAINS(`data`, 'true', '$.factory')
   AND LOCATE('curve = log, unit = \\"s\\", doc = \\"filter envelope', `data`) = 0;

UPDATE `entity_versions`
   SET `json` = REPLACE(REPLACE(`json`,
         ', unit = \\"s\\", doc = \\"filter envelope', ', curve = log, unit = \\"s\\", doc = \\"filter envelope'),
         ', unit = \\"s\\", doc = \\"amp envelope', ', curve = log, unit = \\"s\\", doc = \\"amp envelope')
 WHERE `entity_key` = 'module:analog' AND `status` = 'published'
   AND `message` = 'factory module (migration 2026-11-05-factory-modules)'
   AND LOCATE('curve = log, unit = \\"s\\", doc = \\"filter envelope', `json`) = 0;
