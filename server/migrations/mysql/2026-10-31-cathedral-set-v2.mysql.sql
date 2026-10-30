-- The cathedral set's 3D, re-baked onto one atlas in Blender (#74; tools/rooms/blender/bake_atlas.py): low / medium /
-- high 35K / 65K / 105K triangles, 0.52 / 0.98 / 1.45 MB, uploaded as catpope/map/sets/cathedral.<level>.v2.glb
-- (immutable; v1 stays until nothing uses it). The catalog home scene `cathedral` moves from v1 to v2 – only while it
-- still points at v1, so a later hand edit is kept; the published document `home-scene:cathedral` follows the table.
-- MariaDB 10.11, idempotent: a second run changes nothing.
UPDATE `catalog_home_scenes`
SET `data` = JSON_SET(`data`, '$.model', JSON_EXTRACT('{"src":"catpope/map/sets/cathedral.medium.v2.glb","variants":{"low":"catpope/map/sets/cathedral.low.v2.glb","high":"catpope/map/sets/cathedral.high.v2.glb"}}', '$')),
    `updated_at` = UTC_TIMESTAMP(), `updated_by` = 'migration:2026-10-31-cathedral-set-v2'
WHERE `id` = 'cathedral'
  AND JSON_UNQUOTE(JSON_EXTRACT(`data`, '$.model.src')) = 'catpope/map/sets/cathedral.medium.v1.glb';

-- one published version per key (EntityStore::setPublished): the v1 one steps back to draft
UPDATE `entity_versions` SET `status` = 'draft'
WHERE `entity_key` = 'home-scene:cathedral' AND `status` = 'published'
  AND JSON_UNQUOTE(JSON_EXTRACT(`json`, '$.model.src')) = 'catpope/map/sets/cathedral.medium.v1.glb';

INSERT INTO `entity_versions` (`entity_key`, `kind`, `toon`, `locale`, `json`, `author`, `created_at`, `status`, `message`)
SELECT 'home-scene:cathedral', 'home-scene', NULL, NULL, t.`data`, NULL, UTC_TIMESTAMP(), 'published', 'the set''s 3D re-baked, v2 (2026-10-31-cathedral-set-v2)'
FROM `catalog_home_scenes` t
WHERE t.`id` = 'cathedral'
  AND NOT EXISTS (SELECT 1 FROM `entity_versions` v WHERE v.`entity_key` = 'home-scene:cathedral' AND v.`status` = 'published');
