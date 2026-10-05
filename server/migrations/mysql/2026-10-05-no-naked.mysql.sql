-- The naked A-pose look is removed (owner 2026-10-05, agents/ITEMS.md "Reader avatar bodies & character unlocks"): the
-- only standard A-pose look is `swim`. The API already ignores `naked` (CharacterUnlocks::RETIRED_LOOKS), this cleans the
-- member rows up:
--   avatar_body             a body worn in the `naked` look → the same character's `swim` look
--   member_character_looks  `naked` unlocks are deleted (a member who had only that look keeps the character: a
--                           member_characters row is added first – it unlocks the character with its default look)
--   settings                an AVATAR_DEFAULT_UNLOCKS override stored there loses "naked" (normally none: the default
--                           lives in config/settings.php, now {"catpope":["swim"]})
-- MariaDB, idempotent (a second run matches no rows). Run after 2026-09-30-member-characters and 2026-09-30-settings.
-- phpMyAdmin: select the database (d0487143), SQL tab, run the file as it is.
UPDATE `avatar_body` SET `look_id` = 'swim' WHERE `look_id` = 'naked';

INSERT INTO `member_characters` (`user_id`, `character_id`, `source`, `ref`, `unlocked_at`)
SELECT l.`user_id`, l.`character_id`, l.`source`, l.`ref`, l.`unlocked_at` FROM `member_character_looks` l
WHERE l.`look_id` = 'naked'
  AND NOT EXISTS (SELECT 1 FROM `member_characters` c WHERE c.`user_id` = l.`user_id` AND c.`character_id` = l.`character_id`);

DELETE FROM `member_character_looks` WHERE `look_id` = 'naked';

-- (a JSON object, or a JSON string holding one: the escaped forms first; backslashes doubled in SQL literals)
UPDATE `settings`
SET `value` = REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(`value`,
    ', \\"naked\\"', ''), ',\\"naked\\"', ''), '\\"naked\\", ', ''), '\\"naked\\",', ''), '\\"naked\\"', ''),
    ', "naked"', ''), ',"naked"', ''), '"naked", ', ''), '"naked",', ''), '"naked"', '')
WHERE UPPER(`key`) = 'AVATAR_DEFAULT_UNLOCKS' AND `value` LIKE '%naked%';
