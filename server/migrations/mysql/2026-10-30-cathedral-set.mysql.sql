-- The cathedral as a set with its 3D variant (#195; agents/DECISIONS.md 2026-10-30: place → set → assets, home scenes
-- become sets): the catalog home scene `cathedral` with `model` = the set's 3D per quality level (tools/rooms, uploaded
-- as catpope/map/sets/cathedral.<level>.v1.glb – ASSETS.md "Folder convention"; immutable, a rebuild gets v2), so a
-- deck fetches it through GET /api/library/scene-content?sets=cathedral. No still yet (the image slot stays empty, like
-- cave). A row that exists keeps its data and only gains the model when it has none; the published document
-- `home-scene:cathedral` follows the table (CatalogDocs: document and table agree).
-- MariaDB 10.11, idempotent: a second run changes nothing.
INSERT IGNORE INTO `catalog_home_scenes` (`id`, `position`, `data`, `updated_at`, `updated_by`)
SELECT 'cathedral', COALESCE(MAX(`position`), -1) + 1,
  '{"id":"cathedral","name":{"de":"Kathedrale","en":"Cathedral"},"image":{"src":""},"model":{"src":"catpope/map/sets/cathedral.medium.v1.glb","variants":{"low":"catpope/map/sets/cathedral.low.v1.glb","high":"catpope/map/sets/cathedral.high.v1.glb"}}}',
  UTC_TIMESTAMP(), 'migration:2026-10-30-cathedral-set'
FROM `catalog_home_scenes`;

UPDATE `catalog_home_scenes`
SET `data` = JSON_SET(`data`, '$.model', JSON_EXTRACT('{"src":"catpope/map/sets/cathedral.medium.v1.glb","variants":{"low":"catpope/map/sets/cathedral.low.v1.glb","high":"catpope/map/sets/cathedral.high.v1.glb"}}', '$')),
    `updated_at` = UTC_TIMESTAMP(), `updated_by` = 'migration:2026-10-30-cathedral-set'
WHERE `id` = 'cathedral' AND JSON_EXTRACT(`data`, '$.model') IS NULL;

-- one published version per key (EntityStore::setPublished): an older one without the model steps back to draft
UPDATE `entity_versions` SET `status` = 'draft'
WHERE `entity_key` = 'home-scene:cathedral' AND `status` = 'published' AND JSON_EXTRACT(`json`, '$.model') IS NULL;

INSERT INTO `entity_versions` (`entity_key`, `kind`, `toon`, `locale`, `json`, `author`, `created_at`, `status`, `message`)
SELECT 'home-scene:cathedral', 'home-scene', NULL, NULL, t.`data`, NULL, UTC_TIMESTAMP(), 'published', 'the set''s 3D room (2026-10-30-cathedral-set)'
FROM `catalog_home_scenes` t
WHERE t.`id` = 'cathedral'
  AND NOT EXISTS (SELECT 1 FROM `entity_versions` v WHERE v.`entity_key` = 'home-scene:cathedral' AND v.`status` = 'published');
