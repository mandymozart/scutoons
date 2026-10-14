-- The A-pose belongs to a LOOK (owner 2026-10-12: "nackt does not exist anymore. a poses come per look"; agents/ITEMS.md
-- "Reader avatar bodies"): `character.apose.looks[]` (the character-level A-pose with its `swim` and `naked` looks)
-- becomes `apose: { path, slots }` on a persona look. Worn items are no longer drawn on the body (owner: "additional
-- items dont get overlayed and we just put them in the slots"), so the slot instances lose their image anchors
-- (x, y, size – owner 2026-10-13: "drop x/y/size entirely"); id, slot type and label stay.
--
--   entity_versions   every `character:<id>` version that still has `$.apose`: its standard A-pose look (not `naked`,
--                     not `sensitive`; the default one, else the first) goes to the regular persona's default look as
--                     `apose` – path (when set) + slots without x/y/size; then `$.apose` goes (the naked look with it).
--                     Production 2026-10-13: catpope, yellowmouse, toptoy (each swim + naked) → regular/default.
--   avatar_body       + `persona_id`; a body worn in an A-pose look (`swim` / `naked`) → the regular persona's default
--                     look of that character (the published document; `default` when it has none)
--   member_character_looks  + `persona_id` (in the key); `swim` / `naked` unlocks go (a member who had only that row
--                     keeps the character: a member_characters row is added first – it unlocks the default body)
--   settings          an AVATAR_DEFAULT_UNLOCKS override stored there: { "<character>": [looks] } → ["<character>", …]
--                     (the characters whose default body every member has; config/settings.php: ["catpope"])
--
-- MariaDB 10.11, idempotent: the document update only matches versions that still have `$.apose`, the member rows only
-- `swim` / `naked` looks, the setting only an object; the ALTERs use IF NOT EXISTS (the key is rebuilt the same way).
-- Runs with the deploy (POST /api/deploy/migrate) – plan_migrations shows the row counts first. Only with the code that
-- reads `look.apose` (same commit). Check first:
--   SELECT entity_key, id, status FROM entity_versions WHERE kind = 'character' AND JSON_EXISTS(json, '$.apose');
--   SELECT look_id, COUNT(*) FROM avatar_body GROUP BY look_id;
--   SELECT look_id, COUNT(*) FROM member_character_looks GROUP BY look_id;

UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(
  IF(
    (SELECT p.`ord` FROM JSON_TABLE(`json`, '$.personas[*]' COLUMNS (`ord` FOR ORDINALITY, `pid` VARCHAR(64) PATH '$.id')) p
      WHERE p.`pid` = 'regular' LIMIT 1) IS NULL
    OR (SELECT a.`look` FROM JSON_TABLE(`json`, '$.apose.looks[*]' COLUMNS (`ord` FOR ORDINALITY, `aid` VARCHAR(64) PATH '$.id',
          `sens` VARCHAR(8) PATH '$.sensitive', `look` JSON PATH '$')) a
        WHERE COALESCE(a.`aid`, '') <> 'naked' AND COALESCE(a.`sens`, '') <> 'true' LIMIT 1) IS NULL,
    `json`,
    JSON_SET(`json`,
      CONCAT('$.personas[',
        (SELECT p.`ord` - 1 FROM JSON_TABLE(`json`, '$.personas[*]' COLUMNS (`ord` FOR ORDINALITY, `pid` VARCHAR(64) PATH '$.id')) p
          WHERE p.`pid` = 'regular' LIMIT 1),
        '].looks[',
        COALESCE((SELECT l.`lord` - 1 FROM JSON_TABLE(`json`, '$.personas[*]' COLUMNS (`pid` VARCHAR(64) PATH '$.id',
            NESTED PATH '$.looks[*]' COLUMNS (`lord` FOR ORDINALITY, `dflt` VARCHAR(8) PATH '$.default'))) l
          WHERE l.`pid` = 'regular' AND l.`lord` IS NOT NULL ORDER BY (COALESCE(l.`dflt`, '') = 'true') DESC, l.`lord` LIMIT 1), 0),
        '].apose'),
      JSON_EXTRACT(REGEXP_REPLACE(
        (SELECT JSON_COMPACT(JSON_REMOVE(a.`look`, '$.id', '$.label', '$.default', '$.sensitive'))
          FROM JSON_TABLE(`json`, '$.apose.looks[*]' COLUMNS (`ord` FOR ORDINALITY, `aid` VARCHAR(64) PATH '$.id',
            `sens` VARCHAR(8) PATH '$.sensitive', `dflt` VARCHAR(8) PATH '$.default', `look` JSON PATH '$')) a
          WHERE COALESCE(a.`aid`, '') <> 'naked' AND COALESCE(a.`sens`, '') <> 'true'
          ORDER BY (COALESCE(a.`dflt`, '') = 'true') DESC, a.`ord` LIMIT 1),
        ',"(x|y|size)":-?[0-9.eE+-]+', ''), '$')
    )
  ),
  '$.apose'))
WHERE `kind` = 'character' AND JSON_EXISTS(`json`, '$.apose');

ALTER TABLE `avatar_body` ADD COLUMN IF NOT EXISTS `persona_id` VARCHAR(64) NOT NULL DEFAULT 'regular' AFTER `character_id`;

UPDATE `avatar_body` SET `persona_id` = 'regular', `look_id` = COALESCE((
    SELECT l.`lid` FROM `entity_versions` v, JSON_TABLE(v.`json`, '$.personas[*]' COLUMNS (`pid` VARCHAR(64) PATH '$.id',
      NESTED PATH '$.looks[*]' COLUMNS (`lord` FOR ORDINALITY, `lid` VARCHAR(64) PATH '$.id', `dflt` VARCHAR(8) PATH '$.default'))) l
    WHERE v.`entity_key` = CONCAT('character:', `avatar_body`.`character_id`) AND v.`status` = 'published'
      AND l.`pid` = 'regular' AND l.`lid` IS NOT NULL
    ORDER BY v.`id` DESC, (COALESCE(l.`dflt`, '') = 'true') DESC, l.`lord` LIMIT 1), 'default')
WHERE `look_id` IN ('swim', 'naked');

ALTER TABLE `member_character_looks` ADD COLUMN IF NOT EXISTS `persona_id` VARCHAR(64) NOT NULL DEFAULT 'regular' AFTER `character_id`,
  DROP PRIMARY KEY, ADD PRIMARY KEY (`user_id`, `character_id`, `persona_id`, `look_id`);

INSERT INTO `member_characters` (`user_id`, `character_id`, `source`, `ref`, `unlocked_at`)
SELECT l.`user_id`, l.`character_id`, MIN(l.`source`), MIN(l.`ref`), MIN(l.`unlocked_at`) FROM `member_character_looks` l
WHERE l.`look_id` IN ('swim', 'naked')
  AND NOT EXISTS (SELECT 1 FROM `member_characters` c WHERE c.`user_id` = l.`user_id` AND c.`character_id` = l.`character_id`)
GROUP BY l.`user_id`, l.`character_id`;

DELETE FROM `member_character_looks` WHERE `look_id` IN ('swim', 'naked');

-- (a JSON object, or a JSON string holding one – an override stored as text)
UPDATE `settings` SET `value` = JSON_COMPACT(JSON_KEYS(`value`))
WHERE UPPER(`key`) = 'AVATAR_DEFAULT_UNLOCKS' AND JSON_VALID(`value`) AND JSON_TYPE(`value`) = 'OBJECT';

UPDATE `settings` SET `value` = JSON_QUOTE(JSON_COMPACT(JSON_KEYS(JSON_UNQUOTE(`value`))))
WHERE UPPER(`key`) = 'AVATAR_DEFAULT_UNLOCKS' AND JSON_VALID(`value`) AND JSON_TYPE(`value`) = 'STRING'
  AND JSON_VALID(JSON_UNQUOTE(`value`)) AND JSON_TYPE(JSON_UNQUOTE(`value`)) = 'OBJECT';
