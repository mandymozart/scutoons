-- Places are unlocked by the story's unlockLocation event only (owner 2026-10-11: "Freischalten von Locations sollten wir
-- an diesen Events machen … dann kann auch ein anderer eine Location freischalten"; agents/MAP.md §8.5a). The old link
-- of a place to a slice / a chapter position (`map` locations[].unlock – read from the reader's progress) goes: every
-- place's unlock becomes an unlockLocation event on its slice (a scroll trigger: the toast, no map), and a place that
-- only had a chapter position gets it on the chapter's first slice ("tun wir einfach auf den ersten Slice … später
-- anpassen"). Then `unlock` leaves the map (`unlock: null` – "not unlockable yet" – too).
--   the event: {"type":"unlockLocation","params":{"location":"<place>"},"trigger":{"type":"intersect"},"id":"unlock-<place>"}
-- The slice is found by its image in the chapter's language-neutral documents (`chapter:catpope:<n>`,
-- `scene:catpope:<n>:*`) – every version that has it, history included; when no version of that chapter has the slice
-- (re-cut since) in its current versions (the newest and the newest published), the chapter's first slice
-- (`$.groups[0].slices[0]` of the chapter document) instead – an old version may then have both, that does no harm.
-- Analysis 2026-10-11 on the dev DB (the production seed; older than production for chapters 2–5): 20 places, all
-- hidden; 8 with a slice (chapters 1, 2), 11 with a chapter position only (station: 2; chapter 3: mountain-village,
-- beyond-the-crossroads, mountain-path, forest-clearing, monastery, library; chapter 4: crypt, graveyard, well;
-- chapter 5: garden), airport `unlock: null`.
-- Idempotent: an event goes in only where the place has none yet; a second run changes nothing. JSON stays compact.
-- Every compared string is a literal, never a user variable: a literal takes the column's collation (production's table
-- is utf8mb4_unicode_ci, phpMyAdmin's connection utf8mb4_general_ci – a variable gave "#1267 Illegal mix of collations").
-- phpMyAdmin: select the database (d0487143), SQL tab. Check first (map versions with an unlock / versions with an event):
--   SELECT SUM(`entity_key` = 'map' AND `json` LIKE '%"unlock":%'), SUM(`json` LIKE '%"type":"unlockLocation"%') FROM `entity_versions`;
-- then run the file as it is; the checks at the end answer nothing afterwards.
-- Afterwards: the API's library cache keys its answers by version ids (DATA.md §3) and these rows changed in place, so
-- empty it once – the folder `storage/cache/library` of the API on the server (FTP; with APCu: it empties on a PHP
-- restart) – else readers keep the old chapters / map for up to 7 days.
-- Order: run this BEFORE deploying the code without the old unlock (the published viewer already runs unlockLocation;
-- the new API's map schema no longer knows `unlock`).

-- apartment: slice s1/c1/s1c1-slice-2.webp (chapter 1)
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', 'apartment'), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', 'unlock-apartment');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c1/s1c1-slice-2.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c1/s1c1-slice-2.webp', NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c1/s1c1-slice-2.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c1/s1c1-slice-2.webp', NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c1/s1c1-slice-2.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c1/s1c1-slice-2.webp', NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:1' OR `entity_key` LIKE 'scene:catpope:1:%')
    AND JSON_SEARCH(`json`, 'one', 's1/c1/s1c1-slice-2.webp', NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"apartment"}%';
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:1' OR `entity_key` LIKE 'scene:catpope:1:%') AND `json` LIKE '%"type":"unlockLocation","params":{"location":"apartment"}%'
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:1' OR `entity_key` LIKE 'scene:catpope:1:%') GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = 'chapter:catpope:1' OR `entity_key` LIKE 'scene:catpope:1:%') GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = 'chapter:catpope:1' AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"apartment"}%';
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'apartment', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'apartment', NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', 'apartment', NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'apartment', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'apartment', NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- apartment-blocks: slice s1/c1/s1c1-slice-72.webp (chapter 1)
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', 'apartment-blocks'), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', 'unlock-apartment-blocks');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c1/s1c1-slice-72.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c1/s1c1-slice-72.webp', NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c1/s1c1-slice-72.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c1/s1c1-slice-72.webp', NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c1/s1c1-slice-72.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c1/s1c1-slice-72.webp', NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:1' OR `entity_key` LIKE 'scene:catpope:1:%')
    AND JSON_SEARCH(`json`, 'one', 's1/c1/s1c1-slice-72.webp', NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"apartment-blocks"}%';
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:1' OR `entity_key` LIKE 'scene:catpope:1:%') AND `json` LIKE '%"type":"unlockLocation","params":{"location":"apartment-blocks"}%'
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:1' OR `entity_key` LIKE 'scene:catpope:1:%') GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = 'chapter:catpope:1' OR `entity_key` LIKE 'scene:catpope:1:%') GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = 'chapter:catpope:1' AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"apartment-blocks"}%';
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'apartment-blocks', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'apartment-blocks', NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', 'apartment-blocks', NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'apartment-blocks', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'apartment-blocks', NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- waterfall-tower: slice s1/c2/s1c2-slice-3.webp (chapter 2)
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', 'waterfall-tower'), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', 'unlock-waterfall-tower');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-3.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-3.webp', NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-3.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-3.webp', NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-3.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-3.webp', NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%')
    AND JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-3.webp', NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"waterfall-tower"}%';
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%') AND `json` LIKE '%"type":"unlockLocation","params":{"location":"waterfall-tower"}%'
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%') GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%') GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = 'chapter:catpope:2' AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"waterfall-tower"}%';
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'waterfall-tower', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'waterfall-tower', NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', 'waterfall-tower', NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'waterfall-tower', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'waterfall-tower', NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- infinity-pool: slice s1/c2/s1c2-slice-6.webp (chapter 2)
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', 'infinity-pool'), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', 'unlock-infinity-pool');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-6.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-6.webp', NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-6.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-6.webp', NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-6.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-6.webp', NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%')
    AND JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-6.webp', NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"infinity-pool"}%';
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%') AND `json` LIKE '%"type":"unlockLocation","params":{"location":"infinity-pool"}%'
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%') GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%') GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = 'chapter:catpope:2' AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"infinity-pool"}%';
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'infinity-pool', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'infinity-pool', NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', 'infinity-pool', NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'infinity-pool', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'infinity-pool', NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- crossroads: slice s1/c2/s1c2-slice-10.webp (chapter 2)
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', 'crossroads'), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', 'unlock-crossroads');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-10.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-10.webp', NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-10.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-10.webp', NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-10.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-10.webp', NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%')
    AND JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-10.webp', NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"crossroads"}%';
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%') AND `json` LIKE '%"type":"unlockLocation","params":{"location":"crossroads"}%'
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%') GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%') GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = 'chapter:catpope:2' AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"crossroads"}%';
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'crossroads', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'crossroads', NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', 'crossroads', NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'crossroads', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'crossroads', NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- medival-street: slice s1/c2/s1c2-slice-16.webp (chapter 2)
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', 'medival-street'), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', 'unlock-medival-street');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-16.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-16.webp', NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-16.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-16.webp', NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-16.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-16.webp', NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%')
    AND JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-16.webp', NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"medival-street"}%';
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%') AND `json` LIKE '%"type":"unlockLocation","params":{"location":"medival-street"}%'
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%') GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%') GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = 'chapter:catpope:2' AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"medival-street"}%';
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'medival-street', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'medival-street', NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', 'medival-street', NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'medival-street', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'medival-street', NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- church-square: slice s1/c2/s1c2-slice-20.webp (chapter 2)
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', 'church-square'), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', 'unlock-church-square');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-20.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-20.webp', NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-20.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-20.webp', NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-20.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-20.webp', NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%')
    AND JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-20.webp', NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"church-square"}%';
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%') AND `json` LIKE '%"type":"unlockLocation","params":{"location":"church-square"}%'
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%') GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%') GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = 'chapter:catpope:2' AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"church-square"}%';
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'church-square', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'church-square', NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', 'church-square', NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'church-square', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'church-square', NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- cathedral: slice s1/c2/s1c2-slice-21.webp (chapter 2)
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', 'cathedral'), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', 'unlock-cathedral');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-21.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-21.webp', NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-21.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-21.webp', NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-21.webp', NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-21.webp', NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%')
    AND JSON_SEARCH(`json`, 'one', 's1/c2/s1c2-slice-21.webp', NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"cathedral"}%';
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%') AND `json` LIKE '%"type":"unlockLocation","params":{"location":"cathedral"}%'
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%') GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%') GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = 'chapter:catpope:2' AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"cathedral"}%';
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'cathedral', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'cathedral', NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', 'cathedral', NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'cathedral', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'cathedral', NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- station: a chapter position only (chapter 2)
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', 'station'), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', 'unlock-station');
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%') AND `json` LIKE '%"type":"unlockLocation","params":{"location":"station"}%'
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%') GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = 'chapter:catpope:2' OR `entity_key` LIKE 'scene:catpope:2:%') GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = 'chapter:catpope:2' AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"station"}%';
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'station', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'station', NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', 'station', NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'station', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'station', NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- mountain-village: a chapter position only (chapter 3)
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', 'mountain-village'), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', 'unlock-mountain-village');
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:3' OR `entity_key` LIKE 'scene:catpope:3:%') AND `json` LIKE '%"type":"unlockLocation","params":{"location":"mountain-village"}%'
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:3' OR `entity_key` LIKE 'scene:catpope:3:%') GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = 'chapter:catpope:3' OR `entity_key` LIKE 'scene:catpope:3:%') GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = 'chapter:catpope:3' AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"mountain-village"}%';
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'mountain-village', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'mountain-village', NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', 'mountain-village', NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'mountain-village', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'mountain-village', NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- beyond-the-crossroads: a chapter position only (chapter 3)
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', 'beyond-the-crossroads'), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', 'unlock-beyond-the-crossroads');
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:3' OR `entity_key` LIKE 'scene:catpope:3:%') AND `json` LIKE '%"type":"unlockLocation","params":{"location":"beyond-the-crossroads"}%'
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:3' OR `entity_key` LIKE 'scene:catpope:3:%') GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = 'chapter:catpope:3' OR `entity_key` LIKE 'scene:catpope:3:%') GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = 'chapter:catpope:3' AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"beyond-the-crossroads"}%';
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'beyond-the-crossroads', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'beyond-the-crossroads', NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', 'beyond-the-crossroads', NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'beyond-the-crossroads', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'beyond-the-crossroads', NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- mountain-path: a chapter position only (chapter 3)
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', 'mountain-path'), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', 'unlock-mountain-path');
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:3' OR `entity_key` LIKE 'scene:catpope:3:%') AND `json` LIKE '%"type":"unlockLocation","params":{"location":"mountain-path"}%'
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:3' OR `entity_key` LIKE 'scene:catpope:3:%') GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = 'chapter:catpope:3' OR `entity_key` LIKE 'scene:catpope:3:%') GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = 'chapter:catpope:3' AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"mountain-path"}%';
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'mountain-path', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'mountain-path', NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', 'mountain-path', NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'mountain-path', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'mountain-path', NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- forest-clearing: a chapter position only (chapter 3)
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', 'forest-clearing'), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', 'unlock-forest-clearing');
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:3' OR `entity_key` LIKE 'scene:catpope:3:%') AND `json` LIKE '%"type":"unlockLocation","params":{"location":"forest-clearing"}%'
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:3' OR `entity_key` LIKE 'scene:catpope:3:%') GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = 'chapter:catpope:3' OR `entity_key` LIKE 'scene:catpope:3:%') GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = 'chapter:catpope:3' AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"forest-clearing"}%';
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'forest-clearing', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'forest-clearing', NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', 'forest-clearing', NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'forest-clearing', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'forest-clearing', NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- monastery: a chapter position only (chapter 3)
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', 'monastery'), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', 'unlock-monastery');
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:3' OR `entity_key` LIKE 'scene:catpope:3:%') AND `json` LIKE '%"type":"unlockLocation","params":{"location":"monastery"}%'
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:3' OR `entity_key` LIKE 'scene:catpope:3:%') GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = 'chapter:catpope:3' OR `entity_key` LIKE 'scene:catpope:3:%') GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = 'chapter:catpope:3' AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"monastery"}%';
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'monastery', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'monastery', NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', 'monastery', NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'monastery', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'monastery', NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- library: a chapter position only (chapter 3)
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', 'library'), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', 'unlock-library');
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:3' OR `entity_key` LIKE 'scene:catpope:3:%') AND `json` LIKE '%"type":"unlockLocation","params":{"location":"library"}%'
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:3' OR `entity_key` LIKE 'scene:catpope:3:%') GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = 'chapter:catpope:3' OR `entity_key` LIKE 'scene:catpope:3:%') GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = 'chapter:catpope:3' AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"library"}%';
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'library', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'library', NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', 'library', NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'library', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'library', NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- crypt: a chapter position only (chapter 4)
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', 'crypt'), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', 'unlock-crypt');
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:4' OR `entity_key` LIKE 'scene:catpope:4:%') AND `json` LIKE '%"type":"unlockLocation","params":{"location":"crypt"}%'
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:4' OR `entity_key` LIKE 'scene:catpope:4:%') GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = 'chapter:catpope:4' OR `entity_key` LIKE 'scene:catpope:4:%') GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = 'chapter:catpope:4' AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"crypt"}%';
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'crypt', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'crypt', NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', 'crypt', NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'crypt', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'crypt', NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- graveyard: a chapter position only (chapter 4)
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', 'graveyard'), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', 'unlock-graveyard');
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:4' OR `entity_key` LIKE 'scene:catpope:4:%') AND `json` LIKE '%"type":"unlockLocation","params":{"location":"graveyard"}%'
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:4' OR `entity_key` LIKE 'scene:catpope:4:%') GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = 'chapter:catpope:4' OR `entity_key` LIKE 'scene:catpope:4:%') GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = 'chapter:catpope:4' AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"graveyard"}%';
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'graveyard', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'graveyard', NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', 'graveyard', NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'graveyard', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'graveyard', NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- well: a chapter position only (chapter 4)
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', 'well'), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', 'unlock-well');
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:4' OR `entity_key` LIKE 'scene:catpope:4:%') AND `json` LIKE '%"type":"unlockLocation","params":{"location":"well"}%'
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:4' OR `entity_key` LIKE 'scene:catpope:4:%') GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = 'chapter:catpope:4' OR `entity_key` LIKE 'scene:catpope:4:%') GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = 'chapter:catpope:4' AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"well"}%';
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'well', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'well', NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', 'well', NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'well', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'well', NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- garden: a chapter position only (chapter 5)
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', 'garden'), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', 'unlock-garden');
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:5' OR `entity_key` LIKE 'scene:catpope:5:%') AND `json` LIKE '%"type":"unlockLocation","params":{"location":"garden"}%'
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = 'chapter:catpope:5' OR `entity_key` LIKE 'scene:catpope:5:%') GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = 'chapter:catpope:5' OR `entity_key` LIKE 'scene:catpope:5:%') GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = 'chapter:catpope:5' AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE '%"type":"unlockLocation","params":{"location":"garden"}%';
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'garden', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'garden', NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', 'garden', NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'garden', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'garden', NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- airport: "not unlockable yet" (unlock: null) – no event, the field goes
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'airport', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'airport', NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', 'airport', NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'airport', NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', 'airport', NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- Check after: the same SELECT – no map version with an unlock (0), the events in the chapters' versions (> 0).
