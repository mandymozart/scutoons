-- Chapter 0 (the prologue) after the media-quality merge (agents/DATA.md "Media quality"): the player video's
-- `sources` (media queries) -> `variants { low, medium, high }` (480p / 720p / 1080p, src = 720p). For a database that
-- already has chapter:catpope:0:* (production after the 2026-10-04 cutover): content-seed.mysql.sql skips existing keys.
-- Run once after deploying the media-quality build (an older reader build plays `src`, 720p). Idempotent: it inserts
-- the new published version only while the published one still has `sources`, then turns that one into a draft.
-- Same JSON as shared/content/seed/catpope/<l>/chapter0.json. Locally: import-content --toon=catpope --only=chapter --update.
SET NAMES utf8mb4;
INSERT INTO `entity_versions` (`entity_key`, `kind`, `toon`, `locale`, `json`, `author`, `created_at`, `status`, `message`)
SELECT 'chapter:catpope:0:de', 'chapter', 'catpope', 'de', '{"title":"Roar Little Kitty","author":"Mandy Mozart","number":0,"language":"de","release":"2025-10-04","status":"published","description":"Der Prolog zur ersten Staffel.","thumbnail":"images/s1c0-thumbnail.webp","isPreview":true,"groups":[{"label":"roar-little-kitty","renderer":{"type":"vertical"},"slices":[{"image":"images/webtoon/s1c0/s1c0-slice-1.webp","width":390,"height":206,"video":{"src":"videos/roar-little-kitty.720p.mp4","variants":{"low":"videos/roar-little-kitty.480p.mp4","medium":"videos/roar-little-kitty.720p.mp4","high":"videos/roar-little-kitty.1080p.mp4"},"player":true}}]}]}', NULL, UTC_TIMESTAMP(), 'published', 'media quality: video variants' FROM DUAL
WHERE EXISTS (SELECT 1 FROM `entity_versions` WHERE `entity_key` = 'chapter:catpope:0:de' AND `status` = 'published' AND `json` LIKE '%"sources"%');
UPDATE `entity_versions` SET `status` = 'draft'
WHERE `entity_key` = 'chapter:catpope:0:de' AND `status` = 'published' AND `json` LIKE '%"sources"%'
  AND `id` < (SELECT `m` FROM (SELECT MAX(`id`) AS `m` FROM `entity_versions` WHERE `entity_key` = 'chapter:catpope:0:de' AND `json` LIKE '%"variants"%') AS `t`);
INSERT INTO `entity_versions` (`entity_key`, `kind`, `toon`, `locale`, `json`, `author`, `created_at`, `status`, `message`)
SELECT 'chapter:catpope:0:en', 'chapter', 'catpope', 'en', '{"title":"Roar Little Kitty","author":"Mandy Mozart","number":0,"language":"en","release":"2025-10-04","status":"published","description":"The prologue to season one.","thumbnail":"images/s1c0-thumbnail.webp","isPreview":true,"groups":[{"label":"roar-little-kitty","renderer":{"type":"vertical"},"slices":[{"image":"images/webtoon/s1c0/s1c0-slice-1.webp","width":390,"height":206,"video":{"src":"videos/roar-little-kitty.720p.mp4","variants":{"low":"videos/roar-little-kitty.480p.mp4","medium":"videos/roar-little-kitty.720p.mp4","high":"videos/roar-little-kitty.1080p.mp4"},"player":true}}]}]}', NULL, UTC_TIMESTAMP(), 'published', 'media quality: video variants' FROM DUAL
WHERE EXISTS (SELECT 1 FROM `entity_versions` WHERE `entity_key` = 'chapter:catpope:0:en' AND `status` = 'published' AND `json` LIKE '%"sources"%');
UPDATE `entity_versions` SET `status` = 'draft'
WHERE `entity_key` = 'chapter:catpope:0:en' AND `status` = 'published' AND `json` LIKE '%"sources"%'
  AND `id` < (SELECT `m` FROM (SELECT MAX(`id`) AS `m` FROM `entity_versions` WHERE `entity_key` = 'chapter:catpope:0:en' AND `json` LIKE '%"variants"%') AS `t`);
