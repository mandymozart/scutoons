-- The default character rig sings only when triggered (#819, decisions/deck.md [rigs] 2026-11-23): its route list gains
-- `trigger:sing → voice:@open` (deck/native/rigs/default.toml), so the walk's gates stay shut until the player holds the
-- sing button / key or a story's text line of the character opens them (deck-space space/sing.rs). A rig that takes the
-- voice over (`takes = "voice"`) gets the route on its own track by the core (rig/extend.rs), nothing stored for it.
-- deck-space's the_servers_scene_content_reads_as_bundles test keeps the row equal to the deck's core rig. Only the rig
-- row changes: the instrument `default_voice` and the preset `default_voice.base` keep their data (the walk's `open`
-- input has its default in the core's catalog).
-- What it touches: catalog_rigs: the row `character.default` when its routes read no `trigger:sing` yet (an admin's
-- edit of the row otherwise stays). catalog_history keeps its past states.
-- Dry run (before): SELECT id, JSON_EXTRACT(data, '$.route') FROM catalog_rigs WHERE id = 'character.default';
--   -> expected: one row, [{"from":"voice:ear.level","to":"being:mouth","scale":4.0,"offset":0.0}].
-- Test (after): the same query -> [{"from":"voice:ear.level","to":"being:mouth","scale":4.0,"offset":0.0},
--   {"from":"trigger:sing","to":"voice:@open","scale":1.0,"offset":0.0}].
-- Plan's row counts: catalog_rigs 1. MariaDB 10.11, idempotent: a second run finds `trigger:sing` and changes nothing.
-- Needs 2026-11-22-rig-default (the row).

UPDATE `catalog_rigs`
   SET `data` = JSON_ARRAY_APPEND(`data`, '$.route', JSON_OBJECT('from', 'trigger:sing', 'to', 'voice:@open', 'scale', 1.0, 'offset', 0.0)),
       `updated_at` = UTC_TIMESTAMP(),
       `updated_by` = 'migration:2026-11-23-rig-default-trigger'
 WHERE `id` = 'character.default'
   AND JSON_TYPE(JSON_EXTRACT(`data`, '$.route')) = 'ARRAY'
   AND JSON_SEARCH(`data`, 'one', 'trigger:sing', NULL, '$.route[*].from') IS NULL;
