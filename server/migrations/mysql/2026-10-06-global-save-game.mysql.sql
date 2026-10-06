-- ONE save game per member (owner 2026-10-06: "Variables are per save game … make them across all toons"; DATA.md
-- "The save game"): the per-toon game states (`progress` rows with toon_id = <toon>: variables, history, checkpoints,
-- created_at) are merged into the member's save game, the row with the reserved id `scutoons` (which already holds the
-- map). The per-toon rows stay as they are (older viewer builds still read / write them; rollback).
-- Rules: a toon's variables keep their domain – `x` in catpope's state becomes `catpope/x` (owner 2026-10-06: names
-- have a domain, @scutoons/content variables.ts), so two toons never clash; a key in two states → the NEWEST state's
-- value wins (states folded in `updated_at` order, the save game itself included – so values the new viewer wrote later
-- stay); history entries per key the same way;
-- checkpoints by uri: the later `updated_at`; created_at: the earliest; updated_at (column): the newest of all.
-- Analysis 2026-10-06 on the dev DB (the production dump has no progress rows – only the local test user: catpope,
-- sandbox, yellowmouse + scutoons, ≤ 1 KB each, no clashes); the shape is the viewer's (contexts/game.svelte.ts until
-- 2026-10-06): { toonId, variables, progress, history?, created_at, updated_at }; PROGRESS_MAX_BYTES 256 KB per row.
-- MariaDB ≥ 10.6 (JSON_TABLE, recursive CTE; production 10.11.14). Idempotent: a second run folds the same states
-- with the save game (now the newest) last – nothing changes. PHPUnit ProgressSaveGameTest::testTheGlobalSaveGameMigration.
-- phpMyAdmin: select the database (d0487143), SQL tab. Check first – the members with per-toon states:
--   SELECT user_id, COUNT(*) AS toons, SUM(LENGTH(state)) AS bytes FROM progress WHERE toon_id <> 'scutoons' GROUP BY user_id;
-- then run the file as it is. Afterwards every member with per-toon states has a `scutoons` row with `variables`:
--   SELECT COUNT(*) FROM progress p WHERE toon_id <> 'scutoons' AND NOT EXISTS (SELECT 1 FROM progress g WHERE g.user_id = p.user_id AND g.toon_id = 'scutoons' AND JSON_EXISTS(g.state, '$.variables'));  -- 0
SET SESSION group_concat_max_len = 16777216;
INSERT INTO `progress` (`user_id`, `toon_id`, `state`, `updated_at`)
WITH RECURSIVE
  `states` AS (
    SELECT `user_id`, `toon_id`, `state`, `updated_at`,
      -- a toon's variables keep their domain (owner 2026-10-06, @scutoons/content variables.ts): `x` of catpope's state
      -- becomes `catpope/x`; the save game's own keys stay as they are
      IF(`toon_id` = 'scutoons',
        IF(JSON_TYPE(JSON_EXTRACT(`state`, '$.variables')) = 'OBJECT', JSON_EXTRACT(`state`, '$.variables'), '{}'),
        IF(JSON_TYPE(JSON_EXTRACT(`state`, '$.variables')) = 'OBJECT',
          COALESCE((SELECT JSON_OBJECTAGG(CONCAT(o.`toon_id`, '/', j.`k`), JSON_EXTRACT(o.`state`, CONCAT('$.variables."', j.`k`, '"')))
            FROM JSON_TABLE(JSON_KEYS(o.`state`, '$.variables'), '$[*]' COLUMNS (`k` VARCHAR(255) PATH '$')) j), '{}'),
          '{}')) AS `vars`,
      IF(JSON_TYPE(JSON_EXTRACT(`state`, '$.history')) = 'OBJECT', JSON_EXTRACT(`state`, '$.history'), '{}') AS `hist`,
      ROW_NUMBER() OVER (PARTITION BY `user_id` ORDER BY `updated_at`, `toon_id` = 'scutoons', `toon_id`) AS `n`
    FROM `progress` o
    WHERE JSON_VALID(`state`) AND (`toon_id` = 'scutoons' OR JSON_EXISTS(`state`, '$.variables') OR JSON_EXISTS(`state`, '$.progress') OR JSON_EXISTS(`state`, '$.history'))
  ),
  `members` AS (SELECT DISTINCT `user_id` FROM `progress` WHERE `toon_id` <> 'scutoons' AND JSON_VALID(`state`)),
  `fold` AS (
    SELECT s.`user_id`, s.`n`, s.`vars`, s.`hist` FROM `states` s WHERE s.`n` = 1
    UNION ALL
    SELECT s.`user_id`, s.`n`, JSON_MERGE_PATCH(f.`vars`, s.`vars`), JSON_MERGE_PATCH(f.`hist`, s.`hist`)
    FROM `fold` f JOIN `states` s ON s.`user_id` = f.`user_id` AND s.`n` = f.`n` + 1
  ),
  `folded` AS (
    SELECT f.`user_id`, f.`vars`, f.`hist` FROM `fold` f
    WHERE f.`n` = (SELECT MAX(s.`n`) FROM `states` s WHERE s.`user_id` = f.`user_id`)
  ),
  `checkpoints` AS (
    SELECT s.`user_id`, j.`obj`,
      ROW_NUMBER() OVER (PARTITION BY s.`user_id`, j.`uri` ORDER BY j.`upd` DESC, j.`done` DESC, s.`toon_id` = 'scutoons' DESC) AS `pick`
    FROM `states` s, JSON_TABLE(s.`state`, '$.progress[*]' COLUMNS (`uri` VARCHAR(512) PATH '$.uri', `upd` VARCHAR(64) PATH '$.updated_at', `done` VARCHAR(8) PATH '$.isComplete', `obj` JSON PATH '$')) j
    WHERE j.`uri` IS NOT NULL
  ),
  `progressed` AS (
    SELECT `user_id`, CONCAT('[', GROUP_CONCAT(`obj` ORDER BY `obj` SEPARATOR ','), ']') AS `list` FROM `checkpoints` WHERE `pick` = 1 GROUP BY `user_id`
  ),
  `times` AS (
    SELECT `user_id`, MAX(`updated_at`) AS `updated_at`, MIN(JSON_VALUE(`state`, '$.created_at')) AS `created_at` FROM `states` GROUP BY `user_id`
  )
SELECT m.`user_id`, 'scutoons',
  JSON_SET(
    IF(JSON_TYPE(g.`state`) = 'OBJECT', g.`state`, '{}'),
    '$.variables', JSON_EXTRACT(fd.`vars`, '$'),
    '$.history', JSON_EXTRACT(fd.`hist`, '$'),
    '$.progress', JSON_EXTRACT(COALESCE(pr.`list`, '[]'), '$'),
    '$.created_at', COALESCE(t.`created_at`, JSON_VALUE(g.`state`, '$.created_at'))
  ),
  t.`updated_at`
FROM `members` m
JOIN `folded` fd ON fd.`user_id` = m.`user_id`
JOIN `times` t ON t.`user_id` = m.`user_id`
LEFT JOIN `progressed` pr ON pr.`user_id` = m.`user_id`
LEFT JOIN `progress` g ON g.`user_id` = m.`user_id` AND g.`toon_id` = 'scutoons' AND JSON_VALID(g.`state`)
ON DUPLICATE KEY UPDATE `state` = VALUES(`state`), `updated_at` = VALUES(`updated_at`);
