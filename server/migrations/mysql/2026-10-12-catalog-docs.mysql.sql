-- The catalog entries become versioned documents (agents/CATALOG-IN-CREATOR.md; owner 2026-10-12: "alle einen eigenen
-- Doc, den man versionieren kann"): every item, recipe, album, product and edition gets its first version in
-- entity_versions – key `item:<id>` / `recipe:<id>` / `album:<id>` / `product:<id>` / `edition:<id>`, status published,
-- json = the table's `data` as it is (the catalog table stays what readers get; publishing a newer version writes it).
-- No column changes. content_review_rules.rule gets a third value `physical` (VARCHAR(16), no change needed).
-- Idempotent: only entries without any version get one – a second run inserts nothing.
-- Check first (rows that would be inserted per kind; after the run: none):
--   SELECT 'item' k, COUNT(*) n FROM catalog_items t WHERE NOT EXISTS (SELECT 1 FROM entity_versions v WHERE v.entity_key COLLATE utf8mb4_bin = CONCAT('item:', t.id))
--   UNION ALL SELECT 'recipe', COUNT(*) FROM catalog_recipes t WHERE NOT EXISTS (SELECT 1 FROM entity_versions v WHERE v.entity_key COLLATE utf8mb4_bin = CONCAT('recipe:', t.id))
--   UNION ALL SELECT 'album', COUNT(*) FROM catalog_albums t WHERE NOT EXISTS (SELECT 1 FROM entity_versions v WHERE v.entity_key COLLATE utf8mb4_bin = CONCAT('album:', t.id))
--   UNION ALL SELECT 'product', COUNT(*) FROM catalog_products t WHERE NOT EXISTS (SELECT 1 FROM entity_versions v WHERE v.entity_key COLLATE utf8mb4_bin = CONCAT('product:', t.id))
--   UNION ALL SELECT 'edition', COUNT(*) FROM catalog_editions t WHERE NOT EXISTS (SELECT 1 FROM entity_versions v WHERE v.entity_key COLLATE utf8mb4_bin = CONCAT('edition:', t.id));

INSERT INTO `entity_versions` (`entity_key`, `kind`, `toon`, `locale`, `json`, `author`, `created_at`, `status`, `message`)
SELECT CONCAT('item:', t.`id`), 'item', NULL, NULL, t.`data`, NULL, COALESCE(t.`updated_at`, UTC_TIMESTAMP()), 'published', 'catalog entry (2026-10-12-catalog-docs)'
FROM `catalog_items` t
WHERE NOT EXISTS (SELECT 1 FROM `entity_versions` v WHERE v.`entity_key` COLLATE utf8mb4_bin = CONCAT('item:', t.`id`))
ORDER BY t.`position`, t.`id`;

INSERT INTO `entity_versions` (`entity_key`, `kind`, `toon`, `locale`, `json`, `author`, `created_at`, `status`, `message`)
SELECT CONCAT('recipe:', t.`id`), 'recipe', NULL, NULL, t.`data`, NULL, COALESCE(t.`updated_at`, UTC_TIMESTAMP()), 'published', 'catalog entry (2026-10-12-catalog-docs)'
FROM `catalog_recipes` t
WHERE NOT EXISTS (SELECT 1 FROM `entity_versions` v WHERE v.`entity_key` COLLATE utf8mb4_bin = CONCAT('recipe:', t.`id`))
ORDER BY t.`position`, t.`id`;

INSERT INTO `entity_versions` (`entity_key`, `kind`, `toon`, `locale`, `json`, `author`, `created_at`, `status`, `message`)
SELECT CONCAT('album:', t.`id`), 'album', NULL, NULL, t.`data`, NULL, COALESCE(t.`updated_at`, UTC_TIMESTAMP()), 'published', 'catalog entry (2026-10-12-catalog-docs)'
FROM `catalog_albums` t
WHERE NOT EXISTS (SELECT 1 FROM `entity_versions` v WHERE v.`entity_key` COLLATE utf8mb4_bin = CONCAT('album:', t.`id`))
ORDER BY t.`position`, t.`id`;

INSERT INTO `entity_versions` (`entity_key`, `kind`, `toon`, `locale`, `json`, `author`, `created_at`, `status`, `message`)
SELECT CONCAT('product:', t.`id`), 'product', NULL, NULL, t.`data`, NULL, COALESCE(t.`updated_at`, UTC_TIMESTAMP()), 'published', 'catalog entry (2026-10-12-catalog-docs)'
FROM `catalog_products` t
WHERE NOT EXISTS (SELECT 1 FROM `entity_versions` v WHERE v.`entity_key` COLLATE utf8mb4_bin = CONCAT('product:', t.`id`))
ORDER BY t.`position`, t.`id`;

INSERT INTO `entity_versions` (`entity_key`, `kind`, `toon`, `locale`, `json`, `author`, `created_at`, `status`, `message`)
SELECT CONCAT('edition:', t.`id`), 'edition', NULL, NULL, t.`data`, NULL, COALESCE(t.`updated_at`, UTC_TIMESTAMP()), 'published', 'catalog entry (2026-10-12-catalog-docs)'
FROM `catalog_editions` t
WHERE NOT EXISTS (SELECT 1 FROM `entity_versions` v WHERE v.`entity_key` COLLATE utf8mb4_bin = CONCAT('edition:', t.`id`))
ORDER BY t.`position`, t.`id`;
