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
-- phpMyAdmin: select the database (d0487143), SQL tab. Check first – the places with an unlock in the newest map (a
-- place that is not in the list above keeps its unlock and shows up in the check after the run):
--   SELECT jt.id, jt.unlock FROM `entity_versions` v, JSON_TABLE(v.`json`, '$.locations[*]' COLUMNS (id VARCHAR(64) PATH '$.id', unlock JSON PATH '$.unlock')) jt
--     WHERE v.`id` = (SELECT MAX(`id`) FROM `entity_versions` WHERE `entity_key` = 'map') AND jt.unlock IS NOT NULL;
-- then run the file as it is; the checks at the end answer nothing afterwards.
-- Afterwards: the API's library cache keys its answers by version ids (DATA.md §3) and these rows changed in place, so
-- empty it once – the folder `storage/cache/library` of the API on the server (FTP; with APCu: it empties on a PHP
-- restart) – else readers keep the old chapters / map for up to 7 days.
-- Order: run this BEFORE deploying the code without the old unlock (the published viewer already runs unlockLocation;
-- the new API's map schema no longer knows `unlock`).

-- apartment: slice s1/c1/s1c1-slice-2.webp (chapter 1)
SET @place = 'apartment', @slice = 's1/c1/s1c1-slice-2.webp', @chapter = 'chapter:catpope:1', @scenes = 'scene:catpope:1:%';
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', @place), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', CONCAT('unlock-', @place));
SET @mark = CONCAT('%"type":"unlockLocation","params":{"location":"', @place, '"}%');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @slice IS NOT NULL AND `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes)
    AND JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE @mark;
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) AND `json` LIKE @mark
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = @chapter AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE @mark;
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- apartment-blocks: slice s1/c1/s1c1-slice-72.webp (chapter 1)
SET @place = 'apartment-blocks', @slice = 's1/c1/s1c1-slice-72.webp', @chapter = 'chapter:catpope:1', @scenes = 'scene:catpope:1:%';
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', @place), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', CONCAT('unlock-', @place));
SET @mark = CONCAT('%"type":"unlockLocation","params":{"location":"', @place, '"}%');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @slice IS NOT NULL AND `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes)
    AND JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE @mark;
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) AND `json` LIKE @mark
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = @chapter AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE @mark;
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- waterfall-tower: slice s1/c2/s1c2-slice-3.webp (chapter 2)
SET @place = 'waterfall-tower', @slice = 's1/c2/s1c2-slice-3.webp', @chapter = 'chapter:catpope:2', @scenes = 'scene:catpope:2:%';
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', @place), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', CONCAT('unlock-', @place));
SET @mark = CONCAT('%"type":"unlockLocation","params":{"location":"', @place, '"}%');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @slice IS NOT NULL AND `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes)
    AND JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE @mark;
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) AND `json` LIKE @mark
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = @chapter AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE @mark;
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- infinity-pool: slice s1/c2/s1c2-slice-6.webp (chapter 2)
SET @place = 'infinity-pool', @slice = 's1/c2/s1c2-slice-6.webp', @chapter = 'chapter:catpope:2', @scenes = 'scene:catpope:2:%';
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', @place), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', CONCAT('unlock-', @place));
SET @mark = CONCAT('%"type":"unlockLocation","params":{"location":"', @place, '"}%');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @slice IS NOT NULL AND `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes)
    AND JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE @mark;
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) AND `json` LIKE @mark
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = @chapter AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE @mark;
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- crossroads: slice s1/c2/s1c2-slice-10.webp (chapter 2)
SET @place = 'crossroads', @slice = 's1/c2/s1c2-slice-10.webp', @chapter = 'chapter:catpope:2', @scenes = 'scene:catpope:2:%';
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', @place), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', CONCAT('unlock-', @place));
SET @mark = CONCAT('%"type":"unlockLocation","params":{"location":"', @place, '"}%');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @slice IS NOT NULL AND `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes)
    AND JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE @mark;
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) AND `json` LIKE @mark
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = @chapter AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE @mark;
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- medival-street: slice s1/c2/s1c2-slice-16.webp (chapter 2)
SET @place = 'medival-street', @slice = 's1/c2/s1c2-slice-16.webp', @chapter = 'chapter:catpope:2', @scenes = 'scene:catpope:2:%';
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', @place), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', CONCAT('unlock-', @place));
SET @mark = CONCAT('%"type":"unlockLocation","params":{"location":"', @place, '"}%');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @slice IS NOT NULL AND `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes)
    AND JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE @mark;
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) AND `json` LIKE @mark
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = @chapter AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE @mark;
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- church-square: slice s1/c2/s1c2-slice-20.webp (chapter 2)
SET @place = 'church-square', @slice = 's1/c2/s1c2-slice-20.webp', @chapter = 'chapter:catpope:2', @scenes = 'scene:catpope:2:%';
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', @place), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', CONCAT('unlock-', @place));
SET @mark = CONCAT('%"type":"unlockLocation","params":{"location":"', @place, '"}%');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @slice IS NOT NULL AND `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes)
    AND JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE @mark;
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) AND `json` LIKE @mark
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = @chapter AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE @mark;
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- cathedral: slice s1/c2/s1c2-slice-21.webp (chapter 2)
SET @place = 'cathedral', @slice = 's1/c2/s1c2-slice-21.webp', @chapter = 'chapter:catpope:2', @scenes = 'scene:catpope:2:%';
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', @place), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', CONCAT('unlock-', @place));
SET @mark = CONCAT('%"type":"unlockLocation","params":{"location":"', @place, '"}%');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @slice IS NOT NULL AND `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes)
    AND JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE @mark;
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) AND `json` LIKE @mark
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = @chapter AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE @mark;
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- station: a chapter position only (chapter 2)
SET @place = 'station', @slice = NULL, @chapter = 'chapter:catpope:2', @scenes = 'scene:catpope:2:%';
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', @place), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', CONCAT('unlock-', @place));
SET @mark = CONCAT('%"type":"unlockLocation","params":{"location":"', @place, '"}%');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @slice IS NOT NULL AND `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes)
    AND JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE @mark;
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) AND `json` LIKE @mark
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = @chapter AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE @mark;
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- mountain-village: a chapter position only (chapter 3)
SET @place = 'mountain-village', @slice = NULL, @chapter = 'chapter:catpope:3', @scenes = 'scene:catpope:3:%';
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', @place), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', CONCAT('unlock-', @place));
SET @mark = CONCAT('%"type":"unlockLocation","params":{"location":"', @place, '"}%');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @slice IS NOT NULL AND `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes)
    AND JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE @mark;
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) AND `json` LIKE @mark
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = @chapter AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE @mark;
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- beyond-the-crossroads: a chapter position only (chapter 3)
SET @place = 'beyond-the-crossroads', @slice = NULL, @chapter = 'chapter:catpope:3', @scenes = 'scene:catpope:3:%';
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', @place), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', CONCAT('unlock-', @place));
SET @mark = CONCAT('%"type":"unlockLocation","params":{"location":"', @place, '"}%');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @slice IS NOT NULL AND `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes)
    AND JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE @mark;
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) AND `json` LIKE @mark
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = @chapter AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE @mark;
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- mountain-path: a chapter position only (chapter 3)
SET @place = 'mountain-path', @slice = NULL, @chapter = 'chapter:catpope:3', @scenes = 'scene:catpope:3:%';
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', @place), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', CONCAT('unlock-', @place));
SET @mark = CONCAT('%"type":"unlockLocation","params":{"location":"', @place, '"}%');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @slice IS NOT NULL AND `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes)
    AND JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE @mark;
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) AND `json` LIKE @mark
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = @chapter AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE @mark;
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- forest-clearing: a chapter position only (chapter 3)
SET @place = 'forest-clearing', @slice = NULL, @chapter = 'chapter:catpope:3', @scenes = 'scene:catpope:3:%';
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', @place), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', CONCAT('unlock-', @place));
SET @mark = CONCAT('%"type":"unlockLocation","params":{"location":"', @place, '"}%');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @slice IS NOT NULL AND `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes)
    AND JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE @mark;
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) AND `json` LIKE @mark
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = @chapter AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE @mark;
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- monastery: a chapter position only (chapter 3)
SET @place = 'monastery', @slice = NULL, @chapter = 'chapter:catpope:3', @scenes = 'scene:catpope:3:%';
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', @place), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', CONCAT('unlock-', @place));
SET @mark = CONCAT('%"type":"unlockLocation","params":{"location":"', @place, '"}%');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @slice IS NOT NULL AND `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes)
    AND JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE @mark;
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) AND `json` LIKE @mark
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = @chapter AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE @mark;
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- library: a chapter position only (chapter 3)
SET @place = 'library', @slice = NULL, @chapter = 'chapter:catpope:3', @scenes = 'scene:catpope:3:%';
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', @place), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', CONCAT('unlock-', @place));
SET @mark = CONCAT('%"type":"unlockLocation","params":{"location":"', @place, '"}%');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @slice IS NOT NULL AND `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes)
    AND JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE @mark;
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) AND `json` LIKE @mark
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = @chapter AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE @mark;
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- crypt: a chapter position only (chapter 4)
SET @place = 'crypt', @slice = NULL, @chapter = 'chapter:catpope:4', @scenes = 'scene:catpope:4:%';
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', @place), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', CONCAT('unlock-', @place));
SET @mark = CONCAT('%"type":"unlockLocation","params":{"location":"', @place, '"}%');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @slice IS NOT NULL AND `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes)
    AND JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE @mark;
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) AND `json` LIKE @mark
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = @chapter AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE @mark;
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- graveyard: a chapter position only (chapter 4)
SET @place = 'graveyard', @slice = NULL, @chapter = 'chapter:catpope:4', @scenes = 'scene:catpope:4:%';
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', @place), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', CONCAT('unlock-', @place));
SET @mark = CONCAT('%"type":"unlockLocation","params":{"location":"', @place, '"}%');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @slice IS NOT NULL AND `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes)
    AND JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE @mark;
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) AND `json` LIKE @mark
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = @chapter AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE @mark;
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- well: a chapter position only (chapter 4)
SET @place = 'well', @slice = NULL, @chapter = 'chapter:catpope:4', @scenes = 'scene:catpope:4:%';
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', @place), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', CONCAT('unlock-', @place));
SET @mark = CONCAT('%"type":"unlockLocation","params":{"location":"', @place, '"}%');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @slice IS NOT NULL AND `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes)
    AND JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE @mark;
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) AND `json` LIKE @mark
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = @chapter AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE @mark;
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- garden: a chapter position only (chapter 5)
SET @place = 'garden', @slice = NULL, @chapter = 'chapter:catpope:5', @scenes = 'scene:catpope:5:%';
SET @event = JSON_OBJECT('type', 'unlockLocation', 'params', JSON_OBJECT('location', @place), 'trigger', JSON_OBJECT('type', 'intersect'), 'id', CONCAT('unlock-', @place));
SET @mark = CONCAT('%"type":"unlockLocation","params":{"location":"', @place, '"}%');
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events')), JSON_ARRAY_APPEND(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image'))) - 6), '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @slice IS NOT NULL AND `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes)
    AND JSON_SEARCH(`json`, 'one', @slice, NULL, '$**.image') IS NOT NULL AND `json` NOT LIKE @mark;
-- the current versions (each document's newest and its newest published) – an old version alone doesn't count
SET @found = (SELECT COUNT(*) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) AND `json` LIKE @mark
  AND (`id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)
    OR `id` IN (SELECT MAX(`id`) FROM `entity_versions` WHERE `locale` IS NULL AND `status` = 'published' AND (`entity_key` = @chapter OR `entity_key` LIKE @scenes) GROUP BY `entity_key`)));
UPDATE `entity_versions` SET `json` = JSON_COMPACT(IF(JSON_EXISTS(`json`, CONCAT('$.groups[0].slices[0]', '.events')), JSON_ARRAY_APPEND(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_EXTRACT(@event, '$')), JSON_INSERT(`json`, CONCAT('$.groups[0].slices[0]', '.events'), JSON_ARRAY(JSON_EXTRACT(@event, '$')))))
  WHERE @found = 0 AND `entity_key` = @chapter AND `locale` IS NULL AND JSON_EXISTS(`json`, '$.groups[0].slices[0]') AND `json` NOT LIKE @mark;
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- airport: "not unlockable yet" (unlock: null) – no event, the field goes
SET @place = 'airport';
UPDATE `entity_versions` SET `json` = JSON_COMPACT(JSON_REMOVE(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock')))
  WHERE `entity_key` = 'map' AND JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id') IS NOT NULL
    AND JSON_EXISTS(`json`, CONCAT(SUBSTRING(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id')), 1, CHAR_LENGTH(JSON_UNQUOTE(JSON_SEARCH(`json`, 'one', @place, NULL, '$.locations[*].id'))) - 3), '.unlock'));

-- Check after (both answer nothing when all went in):
--   the map's places that still have an unlock (any version):
--   SELECT v.`id`, jt.id FROM `entity_versions` v, JSON_TABLE(v.`json`, '$.locations[*]' COLUMNS (id VARCHAR(64) PATH '$.id', has INT EXISTS PATH '$.unlock')) jt WHERE v.`entity_key` = 'map' AND jt.has = 1;
--   a place of the newest map (not the airport) without an unlockLocation event in any chapter / scene document:
--   SELECT jt.id FROM `entity_versions` m, JSON_TABLE(m.`json`, '$.locations[*]' COLUMNS (id VARCHAR(64) PATH '$.id')) jt
--     WHERE m.`id` = (SELECT MAX(`id`) FROM `entity_versions` WHERE `entity_key` = 'map') AND jt.id <> 'airport'
--       AND NOT EXISTS (SELECT 1 FROM `entity_versions` c WHERE c.`locale` IS NULL AND (c.`entity_key` LIKE 'chapter:catpope:%' OR c.`entity_key` LIKE 'scene:catpope:%')
--         AND c.`json` LIKE CONCAT('%"type":"unlockLocation","params":{"location":"', jt.id, '"}%'));
