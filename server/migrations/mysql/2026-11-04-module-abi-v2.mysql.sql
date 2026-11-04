-- Module ABI v2 (#343, agents/research/ABI-V2.md §7; owner 2026-11-03: "no backwards compatibility"): a module's compile
-- result is its descriptor (the wasm's `deck.module` JSON) and the content hash of its .wasm. A result of ABI v1 no longer
-- loads anywhere – the deck's host and the browser read ABI 2 only, ModuleSource refuses `abi` ≠ 2 – so it leaves the
-- published module rows: `descriptor` and `wasm` are removed (the sources, `files`, stay), the module reads as not
-- compiled and is no kind in GET /api/kinds until it is compiled again. The recompile is `php bin/console
-- modules:rebuild --all` (dry run first; the failures are the modules whose source uses the removed v1 API).
-- What it touches: rows of catalog_modules whose descriptor's abi is anything but 2 (the member layer is gone,
-- 2026-11-02-drop-member-layer). Draft and history versions in entity_versions stay as they were (history);
-- `modules:rebuild` saves each recompiled module as a new draft.
-- Dry run (the rows it changes): SELECT id, JSON_VALUE(data, '$.descriptor.abi') AS abi FROM catalog_modules
--   WHERE JSON_EXISTS(data, '$.descriptor') AND COALESCE(JSON_VALUE(data, '$.descriptor.abi'), '') <> '2';
-- MariaDB 10.11, idempotent: a second run matches no row (descriptor gone, or abi 2).
UPDATE `catalog_modules`
   SET `data` = JSON_REMOVE(`data`, '$.descriptor')
 WHERE JSON_EXISTS(`data`, '$.descriptor')
   AND COALESCE(JSON_VALUE(`data`, '$.descriptor.abi'), '') <> '2';

UPDATE `catalog_modules`
   SET `data` = JSON_REMOVE(`data`, '$.wasm')
 WHERE JSON_EXISTS(`data`, '$.wasm')
   AND NOT JSON_EXISTS(`data`, '$.descriptor');
