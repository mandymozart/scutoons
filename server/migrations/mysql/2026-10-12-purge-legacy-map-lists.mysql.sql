-- Purge the old per-toon map lists (owner 2026-10-12: yes to "the old map files before the world map – archive and
-- delete them like the language copies"). `locations:<toon>:<l>` and `routes:<toon>:<l>` were the map before the world
-- map (`map`, 2026-10-01); the library reads them only while no world map is stored – it is stored since then.
-- Analysis 2026-10-12 on the dev DB (= the production dump): 10 keys (catpope de / en, sandbox de / en / ko), one
-- version each, no asset usage rows. Same as 2026-10-11-purge-legacy-copies: COPIED into `entity_versions_legacy`
-- first (created there; created here too if missing), then deleted – only while the world map exists.
-- Runs with the deploy (HANDOVER "Migrations"); idempotent – a second run copies nothing new and deletes nothing.
-- Undo: INSERT INTO entity_versions SELECT * FROM entity_versions_legacy WHERE entity_key REGEXP '^(locations|routes):';

CREATE TABLE IF NOT EXISTS `entity_versions_legacy` LIKE `entity_versions`;

INSERT IGNORE INTO `entity_versions_legacy`
SELECT v.* FROM `entity_versions` v
WHERE v.`entity_key` REGEXP '^(locations|routes):[^:]+:[a-z]{2}$'
  AND EXISTS (SELECT 1 FROM `entity_versions` m WHERE m.`entity_key` = 'map');

DELETE u FROM `asset_usages` u
JOIN `entity_versions_legacy` l ON l.`entity_key` = u.`ref`
WHERE u.`source` = 'content' AND u.`ref` REGEXP '^(locations|routes):';

DELETE v FROM `entity_versions` v
JOIN `entity_versions_legacy` l ON l.`id` = v.`id`
WHERE v.`entity_key` REGEXP '^(locations|routes):';
