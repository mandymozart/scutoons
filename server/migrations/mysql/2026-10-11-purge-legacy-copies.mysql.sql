-- Purge the old per-language copies of the language-neutral documents (owner 2026-10-11: "die Altlasten – können wir
-- die mal purgen?"). Since 2026-10-06 a chapter, scene, season and the toon entry are ONE language-neutral document
-- (`chapter:catpope:2`, `scene:catpope:2:scene-14`, `season:catpope:1`, `toon:catpope`); the old copies per language
-- (`chapter:catpope:2:de`, `scene:catpope:2:scene-14:en`, `season:catpope:1:de`, `toon-settings:catpope:de`) are read
-- only by the creator's fallback for a neutral document that does not exist yet – which no longer happens.
-- Analysis 2026-10-11 on the dev DB (= the latest production dump): 32 keys, 147 versions, 0.9 MB; EVERY one has its
-- neutral document; no review points at them; 938 rows of the derived asset usage index do (removed with them, so an
-- image is not "used" only by an old copy).
-- Safe and reversible: the versions are first COPIED into `entity_versions_legacy` (same columns), then deleted. To
-- undo: INSERT INTO entity_versions SELECT * FROM entity_versions_legacy; (then admin → Assets → Rescan).
-- Idempotent: a second run copies nothing new (INSERT IGNORE on the same ids) and deletes nothing.
-- phpMyAdmin: select the database (d0487143), SQL tab. Check first – every row must say 1 in `has_neutral`:
--   SELECT entity_key, COUNT(*) AS versions,
--     (SELECT COUNT(*) > 0 FROM entity_versions n WHERE n.entity_key = CASE WHEN v.entity_key LIKE 'toon-settings:%'
--        THEN CONCAT('toon:', SUBSTRING_INDEX(SUBSTRING_INDEX(v.entity_key, ':', 2), ':', -1))
--        ELSE SUBSTRING(v.entity_key, 1, LENGTH(v.entity_key) - 3) END) AS has_neutral
--   FROM entity_versions v
--   WHERE v.entity_key REGEXP '^((chapter|season):[^:]+:[0-9]+:[a-z]{2}|scene:[^:]+:[0-9]+:[^:]+:[a-z]{2}|toon-settings:.+)$'
--   GROUP BY entity_key ORDER BY has_neutral, entity_key;
-- (a row with 0: stop and tell the agent – that document has no neutral version yet). Then run the file as it is;
-- afterwards the same SELECT answers no rows, and admin → Assets → Rescan refreshes the usage index.

CREATE TABLE IF NOT EXISTS `entity_versions_legacy` LIKE `entity_versions`;

INSERT IGNORE INTO `entity_versions_legacy`
SELECT v.* FROM `entity_versions` v
WHERE v.`entity_key` REGEXP '^((chapter|season):[^:]+:[0-9]+:[a-z]{2}|scene:[^:]+:[0-9]+:[^:]+:[a-z]{2}|toon-settings:.+)$'
  AND EXISTS (SELECT 1 FROM `entity_versions` n WHERE n.`entity_key` = CASE WHEN v.`entity_key` LIKE 'toon-settings:%'
        THEN CONCAT('toon:', SUBSTRING_INDEX(SUBSTRING_INDEX(v.`entity_key`, ':', 2), ':', -1))
        ELSE SUBSTRING(v.`entity_key`, 1, LENGTH(v.`entity_key`) - 3) END);

DELETE u FROM `asset_usages` u
JOIN `entity_versions_legacy` l ON l.`entity_key` = u.`ref`
WHERE u.`source` = 'content';

DELETE v FROM `entity_versions` v
JOIN `entity_versions_legacy` l ON l.`id` = v.`id`;
