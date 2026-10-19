-- One document per chapter (owner 2026-10-19, Q3: "ja, ein Dokument"; agents/research/TEXT-ACROSS-SLICES.md §5 step 3):
-- a chapter group that points at a scene document (`{"ref":{"src":"chapter<N>/<name>.json"}}`) becomes that scene's
-- group, inlined – the content the reader swapped in until now (Library::withScenes replaced the whole group with the
-- scene document: label, renderer, slices, events, continues). `ref` and the `scene` document kind leave the code with
-- this deploy; the scene documents stay in the database as history: COPIED into `entity_versions_legacy` (same columns,
-- as 2026-10-11-purge-legacy-copies), then deleted from entity_versions with their rows of the derived asset usage
-- index – afterwards admin → Assets → Rescan (the chapters now name those files themselves). Undo of the move:
-- INSERT INTO entity_versions SELECT * FROM entity_versions_legacy WHERE kind = 'scene'; (the inlining stays).
-- Which scene version a chapter version gets – what that version showed:
--   the chapter's current published version → the scene's newest PUBLISHED version (what readers see today);
--   the chapter's newest version (a draft)   → the scene's newest version (what the creator shows today);
--   any older version                         → the scene as it was then: its newest version with an id ≤ the
--                                               chapter version's id (a published chapter version: the newest
--                                               published one) – a restore brings back the same picture;
--   no such version                           → the scene's oldest version; no scene document at all → the group
--                                               stays (the validator then names the ref – none in production).
-- The scene entity from `src` exactly as EntityKey::sceneOf: `chapter<N>/<name>.json` → `scene:<toon>:<N>:<name>`.
-- Production 2026-10-19 (MCP): only catpope chapter 2 (groups 2–4 → scene-14, scene-17, scene-18; chapter v705 and the
-- scenes' newest versions are all published); the dev DB's sandbox toon (make-sandbox) uses refs in chapters 1–9. No
-- text-box, event, placement id or group label repeats once inlined (checked on the newest versions).
-- Order: inline first (it reads the scene rows), then copy, then delete.
-- One pass per chapter version (JSON_TABLE → JSON_ARRAYAGG in group order, as 2026-10-16-endless-strip);
-- JSON_EXTRACT(…, '$') keeps the picked value JSON; writes are JSON_COMPACT. No `SET SESSION` (an `other` statement
-- would make the file "mixed" and plan_migrations could not count it): JSON_ARRAYAGG stays within MariaDB's default
-- group_concat_max_len (1 MB) – chapters are ≤ 70 kB (ch2 v705: 64.7 kB); a chapter over 900 kB is left alone, so a
-- cut-off aggregate can never be written. Idempotent: only versions with a group holding `ref.src` change, INSERT
-- IGNORE copies no id twice, the deletes find no scene row – a second run changes 0 rows. Runs with the deploy
-- (POST /api/deploy/migrate), together with the code that no longer reads `ref`; plan_migrations shows the row count
-- first.

UPDATE `entity_versions` c SET c.`json` = JSON_COMPACT(JSON_SET(c.`json`, '$.groups', JSON_EXTRACT((
    SELECT JSON_ARRAYAGG(JSON_EXTRACT(IF(g.`src` IS NULL, g.`grp`, COALESCE(
        (SELECT s.`json` FROM `entity_versions` s
            WHERE s.`kind` = 'scene'
              AND s.`entity_key` = CONCAT('scene:', c.`toon`, ':', SUBSTRING(SUBSTRING_INDEX(g.`src`, '/', 1), 8), ':', SUBSTRING_INDEX(SUBSTRING_INDEX(g.`src`, '/', -1), '.json', 1))
              AND (s.`status` = 'published' OR c.`status` <> 'published')
              AND (s.`id` <= c.`id`
                    OR c.`id` = (SELECT MAX(p.`id`) FROM `entity_versions` p WHERE p.`entity_key` = c.`entity_key` AND p.`status` = 'published')
                    OR c.`id` = (SELECT MAX(n.`id`) FROM `entity_versions` n WHERE n.`entity_key` = c.`entity_key`))
            ORDER BY s.`id` DESC LIMIT 1),
        (SELECT s.`json` FROM `entity_versions` s
            WHERE s.`kind` = 'scene'
              AND s.`entity_key` = CONCAT('scene:', c.`toon`, ':', SUBSTRING(SUBSTRING_INDEX(g.`src`, '/', 1), 8), ':', SUBSTRING_INDEX(SUBSTRING_INDEX(g.`src`, '/', -1), '.json', 1))
            ORDER BY s.`id` ASC LIMIT 1),
        g.`grp`)), '$') ORDER BY g.`ord`)
    FROM JSON_TABLE(c.`json`, '$.groups[*]' COLUMNS (`ord` FOR ORDINALITY, `grp` JSON PATH '$', `src` VARCHAR(191) PATH '$.ref.src')) g), '$')))
WHERE c.`kind` = 'chapter' AND LENGTH(c.`json`) < 900000 AND EXISTS (SELECT 1 FROM JSON_TABLE(c.`json`, '$.groups[*]' COLUMNS (`src` VARCHAR(191) PATH '$.ref.src')) r
    WHERE r.`src` IS NOT NULL);

INSERT IGNORE INTO `entity_versions_legacy`
SELECT v.* FROM `entity_versions` v WHERE v.`kind` = 'scene';

DELETE u FROM `asset_usages` u
JOIN `entity_versions_legacy` l ON l.`entity_key` = u.`ref`
WHERE u.`source` = 'content' AND l.`kind` = 'scene';

DELETE v FROM `entity_versions` v
JOIN `entity_versions_legacy` l ON l.`id` = v.`id`
WHERE v.`kind` = 'scene';
