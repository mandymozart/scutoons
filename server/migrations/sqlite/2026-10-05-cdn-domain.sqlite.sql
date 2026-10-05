-- CUTOVER 10.3 (2026-10-05): cdn.scutoons.com is a custom hostname of the bunny.net pull zone 6731366, so every stored
-- absolute media URL on the pull zone's own hostname is rewritten to the public name:
--   scutoons.b-cdn.net → cdn.scutoons.com   (https://, http://, //, and JSON-escaped https:\/\/ forms alike)
-- Content normally stores CDN-relative paths; this only catches absolute URLs pasted into content / the catalog.
-- Both names serve the same files (the b-cdn hostname stays on the zone), so this is cleanup, not a fix: safe before or
-- after the deploy, but run it only AFTER https://cdn.scutoons.com/ answers from bunny (CUTOVER.md 10.3 step 4).
-- Every version row is rewritten (history included): undo / restore must not bring the old host back.
-- Idempotent (a second run matches no rows). Needs 2026-10-04-catalog first (the catalog tables).
UPDATE "entity_versions" SET "json" = REPLACE("json", 'scutoons.b-cdn.net', 'cdn.scutoons.com') WHERE "json" LIKE '%scutoons.b-cdn.net%';
UPDATE "content_versions" SET "json" = REPLACE("json", 'scutoons.b-cdn.net', 'cdn.scutoons.com') WHERE "json" LIKE '%scutoons.b-cdn.net%';
UPDATE "toons" SET "data" = REPLACE("data", 'scutoons.b-cdn.net', 'cdn.scutoons.com') WHERE "data" LIKE '%scutoons.b-cdn.net%';
UPDATE "settings" SET "value" = REPLACE("value", 'scutoons.b-cdn.net', 'cdn.scutoons.com') WHERE "value" LIKE '%scutoons.b-cdn.net%';
UPDATE "catalog_items" SET "data" = REPLACE("data", 'scutoons.b-cdn.net', 'cdn.scutoons.com') WHERE "data" LIKE '%scutoons.b-cdn.net%';
UPDATE "catalog_recipes" SET "data" = REPLACE("data", 'scutoons.b-cdn.net', 'cdn.scutoons.com') WHERE "data" LIKE '%scutoons.b-cdn.net%';
UPDATE "catalog_albums" SET "data" = REPLACE("data", 'scutoons.b-cdn.net', 'cdn.scutoons.com') WHERE "data" LIKE '%scutoons.b-cdn.net%';
UPDATE "catalog_products" SET "data" = REPLACE("data", 'scutoons.b-cdn.net', 'cdn.scutoons.com') WHERE "data" LIKE '%scutoons.b-cdn.net%';
UPDATE "catalog_editions" SET "data" = REPLACE("data", 'scutoons.b-cdn.net', 'cdn.scutoons.com') WHERE "data" LIKE '%scutoons.b-cdn.net%';
UPDATE "catalog_featured" SET "data" = REPLACE("data", 'scutoons.b-cdn.net', 'cdn.scutoons.com') WHERE "data" LIKE '%scutoons.b-cdn.net%';
UPDATE "catalog_permissions" SET "data" = REPLACE("data", 'scutoons.b-cdn.net', 'cdn.scutoons.com') WHERE "data" LIKE '%scutoons.b-cdn.net%';
UPDATE "catalog_code_types" SET "data" = REPLACE("data", 'scutoons.b-cdn.net', 'cdn.scutoons.com') WHERE "data" LIKE '%scutoons.b-cdn.net%';
UPDATE "catalog_meta" SET "data" = REPLACE("data", 'scutoons.b-cdn.net', 'cdn.scutoons.com') WHERE "data" LIKE '%scutoons.b-cdn.net%';
UPDATE "catalog_history" SET "before_json" = REPLACE("before_json", 'scutoons.b-cdn.net', 'cdn.scutoons.com') WHERE "before_json" LIKE '%scutoons.b-cdn.net%';
UPDATE "catalog_history" SET "after_json" = REPLACE("after_json", 'scutoons.b-cdn.net', 'cdn.scutoons.com') WHERE "after_json" LIKE '%scutoons.b-cdn.net%';
UPDATE "asset_usages" SET "raw" = REPLACE("raw", 'scutoons.b-cdn.net', 'cdn.scutoons.com') WHERE "raw" LIKE '%scutoons.b-cdn.net%';
