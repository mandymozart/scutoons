-- The home scenes as catalog documents (agents/research/CAVE.md; like 2026-10-12-catalog-docs for the other kinds): every
-- row of catalog_home_scenes (`cave`, `tatami_room` from 2026-10-14-homes) gets its first published version
-- `home-scene:<id>`, so the creator's catalog opens, versions and publishes it. Idempotent: rows that have a document keep it.
INSERT INTO `entity_versions` (`entity_key`, `kind`, `toon`, `locale`, `json`, `author`, `created_at`, `status`, `message`)
SELECT CONCAT('home-scene:', t.`id`), 'home-scene', NULL, NULL, t.`data`, NULL, COALESCE(t.`updated_at`, UTC_TIMESTAMP()), 'published', 'catalog entry (2026-10-14-homes_scene_docs)'
FROM `catalog_home_scenes` t
WHERE NOT EXISTS (SELECT 1 FROM `entity_versions` v WHERE v.`entity_key` COLLATE utf8mb4_bin = CONCAT('home-scene:', t.`id`))
ORDER BY t.`position`, t.`id`;
