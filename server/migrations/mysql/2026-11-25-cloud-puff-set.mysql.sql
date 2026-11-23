-- module:cloud's `puff` slot holds a set of models, each puff one of them (#851; owner 2026-11-23: "we can add x many
-- clouds into the puff slot"): a `model` slot's value is a list of .glb paths (one path still reads as a set of one,
-- deck-space node_assets). The cloud's default set is the owner's two Tripo blob clouds, scutoons/3d/cloud-three-blobs.glb
-- and scutoons/3d/cloud-four-blobs.glb (uploaded to the library); the bubble scutoons/3d/cloud-puff.glb leaves the set
-- (it stays in the library). Dated 2026-11-25 so it sorts after 2026-11-24-cloud-light-sandbox and -update-cloud-puff-slot.
-- What it touches:
-- - catalog_instruments: the row `cloud_storm` (the cloud-storm rig's instrument, rig:item.cloud, placed by
--   scene:cloud-light) – its node `cloud`'s `assets.puff` becomes the set, only while it is still the single path the
--   migration 2026-11-24-cloud-light-sandbox wrote (a creator's own value stays). The row is then
--   server/tests/fixtures/rigs/cloud-storm.json's instrument (deck-native tests/cloud_storm.rs keeps it equal to the TOML).
-- - catalog_modules: the factory row `cloud` – lib.rs's two doc texts say "a set" (the module computes the same bit for
--   bit; FactoryModulesTest keeps lib.rs the repository's), only while the old asset doc is there.
-- - entity_versions: the factory's published version of `module:cloud` takes the entry's data, as
--   2026-11-24-update-cloud-puff-slot did.
-- AFTER THE DEPLOY an admin may press Compile on module:cloud once so its descriptor shows the new slot doc; until then
-- the deck draws the set all the same (the pick is the space's, not the module's).
-- Dry run (before): SELECT JSON_EXTRACT(data, '$.node[0].assets.puff') FROM catalog_instruments WHERE id = 'cloud_storm';
--   -> "scutoons/3d/cloud-puff.glb". SELECT INSTR(JSON_UNQUOTE(JSON_EXTRACT(data, '$.files."lib.rs"')), 'a set of low-poly
--   models') FROM catalog_modules WHERE id = 'cloud'; -> 0.
-- Test (after): the first -> ["scutoons/3d/cloud-three-blobs.glb", "scutoons/3d/cloud-four-blobs.glb"]; the second > 0.
-- Plan's row counts: 1 . 1 . 1. MariaDB 10.11, idempotent: every statement is guarded by the old value.
-- Escaping: plain string literals (no NO_BACKSLASH_ESCAPES): \n a newline of the source text, \' a quote.

UPDATE `catalog_instruments`
   SET `data` = JSON_SET(`data`, '$.node[0].assets.puff', JSON_ARRAY('scutoons/3d/cloud-three-blobs.glb', 'scutoons/3d/cloud-four-blobs.glb')),
       `updated_at` = UTC_TIMESTAMP(), `updated_by` = 'migration:2026-11-25-cloud-puff-set'
 WHERE `id` = 'cloud_storm'
   AND JSON_UNQUOTE(JSON_EXTRACT(`data`, '$.node[0].id')) = 'cloud'
   AND JSON_UNQUOTE(JSON_EXTRACT(`data`, '$.node[0].kind')) = 'module:cloud'
   AND JSON_TYPE(JSON_EXTRACT(`data`, '$.node[0].assets.puff')) = 'STRING'
   AND JSON_UNQUOTE(JSON_EXTRACT(`data`, '$.node[0].assets.puff')) = 'scutoons/3d/cloud-puff.glb';

UPDATE `catalog_modules`
   SET `data` = JSON_SET(`data`, '$.files."lib.rs"', REPLACE(REPLACE(JSON_UNQUOTE(JSON_EXTRACT(`data`, '$.files."lib.rs"')),
         '(each the model in the slot `puff`, a .glb from\n//! the library; the core\'s lumpy placeholder while there is none)',
         '(each one of the models in the slot `puff`, .glb files\n//! from the library; the core\'s lumpy placeholder while there is none)'),
         'doc = "the puff\'s shape: a low-poly model (.glb, at most 4 MiB, fitted to a metre); none: the placeholder"',
         'doc = "the puffs\' shapes: a set of low-poly models (.glb, each at most 4 MiB, fitted to a metre), each puff one of them; none: the placeholder"')),
       `updated_at` = UTC_TIMESTAMP(), `updated_by` = 'migration:2026-11-25-cloud-puff-set'
 WHERE `id` = 'cloud' AND JSON_CONTAINS(`data`, 'true', '$.factory')
   AND INSTR(JSON_UNQUOTE(JSON_EXTRACT(`data`, '$.files."lib.rs"')), 'doc = "the puff\'s shape: a low-poly model (.glb, at most 4 MiB') > 0;

UPDATE `entity_versions` e
  JOIN `catalog_modules` m ON m.`id` = 'cloud' AND JSON_CONTAINS(m.`data`, 'true', '$.factory')
   SET e.`json` = m.`data`
 WHERE e.`entity_key` = 'module:cloud' AND e.`status` = 'published'
   AND e.`message` = 'factory module (migration 2026-11-22-factory-cloud)'
   AND INSTR(JSON_UNQUOTE(JSON_EXTRACT(m.`data`, '$.files."lib.rs"')), 'a set of low-poly models') > 0
   AND INSTR(JSON_UNQUOTE(JSON_EXTRACT(e.`json`, '$.files."lib.rs"')), 'a set of low-poly models') = 0;
