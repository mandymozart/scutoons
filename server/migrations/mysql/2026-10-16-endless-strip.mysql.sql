-- Swiper goes: every horizontal group is our own strip – pinned, or the endless band (owner 2026-10-15: "Ditch Swiper.
-- Give our own strip an endless drag mode (loop + momentum, no 3D effects) for the four ch3–5 groups"; FEATURES 10.100,
-- agents/research/TEXT-ACROSS-SLICES.md §8). `RendererParams` keeps only `endless` (the schemas refuse any other key);
-- autoPlay, loop, flip, keyboard, cube, freeMode and fullWidth leave every chapter / scene version, drafts and history too:
--   horizontal + fullWidth true             → no params (the pinned strip; fullWidth wins over a loop next to it –
--                                             ch2 g0 v633 / v646 and scene-14 v635 carry both)
--   horizontal + loop true (or endless)     → {"endless":true} (the four ch3–5 Swipers are `{loop, freeMode}`)
--   horizontal, anything else               → no params (the pinned strip)
--   vertical (any other type) with params   → no params
-- Where: a chapter's `$.groups[i].renderer.params`, a scene document's `$.renderer.params` – JSON paths only, so the
-- `loop` of a slice video or an audio sample stays. Booleans are read as JSON_UNQUOTE(JSON_EXTRACT(…)) = 'true':
-- MariaDB 10.11's JSON_VALUE returns 1 for a JSON true. Writes are JSON_COMPACT (JSON_SET / JSON_REMOVE re-serialise
-- with spaces; LIKE checks elsewhere rely on compact JSON).
-- A chapter's `$.groups` is rebuilt in one pass (JSON_TABLE → JSON_ARRAYAGG in group order), however many groups carry
-- old keys; only a group whose params hold a key other than `endless` changes. JSON_EXTRACT(…, '$') keeps each rebuilt
-- value JSON (IF() and a subquery return text – JSON_ARRAYAGG / JSON_SET would store it as a string). JSON_ARRAYAGG is
-- capped by group_concat_max_len (a chapter's groups ≈ 48 kB on the dev DB): raised for the session, as 2026-10-06 does.
-- Idempotent: only params with a key other than `endless` are touched – a second run changes 0 rows. Every compared
-- string is a literal. Analysis 2026-10-16 on the dev DB (the production seed; older than production for chapters 3–5):
-- chapter:catpope:2 g0 – 7 versions {fullWidth}, 9 {loop, autoPlay, fullWidth}; scene-14 – 23 versions {freeMode, loop}
-- (→ endless), 9 {fullWidth}; scene-17 / scene-18 – one {fullWidth} each; no vertical group with params.
-- Runs with the deploy (POST /api/deploy/migrate) – plan_migrations shows the row counts first. Only with the code that
-- reads `endless` (same deploy). `entity_versions_legacy` keeps its old-form rows (an undo of 2026-10-11-purge-legacy-copies
-- would bring the old keys back).

SET SESSION group_concat_max_len = 16777216;

UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_SET(`json`, '$.groups', JSON_EXTRACT((
    SELECT JSON_ARRAYAGG(JSON_EXTRACT(IF(JSON_LENGTH(JSON_REMOVE(g.`params`, '$.endless')) > 0,
        IF(g.`type` = 'horizontal' AND COALESCE(JSON_UNQUOTE(JSON_EXTRACT(g.`params`, '$.fullWidth')), '') <> 'true'
            AND (JSON_UNQUOTE(JSON_EXTRACT(g.`params`, '$.loop')) = 'true' OR JSON_UNQUOTE(JSON_EXTRACT(g.`params`, '$.endless')) = 'true'),
          JSON_SET(g.`grp`, '$.renderer.params', JSON_OBJECT('endless', TRUE)),
          JSON_REMOVE(g.`grp`, '$.renderer.params')),
        g.`grp`), '$') ORDER BY g.`ord`)
    FROM JSON_TABLE(`json`, '$.groups[*]' COLUMNS (`ord` FOR ORDINALITY, `grp` JSON PATH '$', `type` VARCHAR(32) PATH '$.renderer.type', `params` JSON PATH '$.renderer.params')) g), '$')))
WHERE `kind` = 'chapter' AND EXISTS (SELECT 1 FROM JSON_TABLE(`json`, '$.groups[*]' COLUMNS (`params` JSON PATH '$.renderer.params')) g
    WHERE JSON_LENGTH(JSON_REMOVE(g.`params`, '$.endless')) > 0);

-- a scene document is one group: its renderer sits at the top
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_UNQUOTE(JSON_EXTRACT(`json`, '$.renderer.type')) = 'horizontal'
    AND COALESCE(JSON_UNQUOTE(JSON_EXTRACT(`json`, '$.renderer.params.fullWidth')), '') <> 'true'
    AND (JSON_UNQUOTE(JSON_EXTRACT(`json`, '$.renderer.params.loop')) = 'true' OR JSON_UNQUOTE(JSON_EXTRACT(`json`, '$.renderer.params.endless')) = 'true'),
  JSON_SET(`json`, '$.renderer.params', JSON_OBJECT('endless', TRUE)),
  JSON_REMOVE(`json`, '$.renderer.params')))
WHERE `kind` = 'scene' AND JSON_LENGTH(JSON_REMOVE(JSON_EXTRACT(`json`, '$.renderer.params'), '$.endless')) > 0;
