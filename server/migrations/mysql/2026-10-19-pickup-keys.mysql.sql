-- A placed pickup is remembered by its placement id alone (agents/research/TEXT-ACROSS-SLICES.md §5 step 3, approved
-- 2026-10-19 instead of stable slice ids): placement ids are unique already (`<item>-<7 random>`, fresh on every paste),
-- and the slice's picture may now be renamed or re-cut (the storyboard's re-slicing) without the reader's pickup
-- coming back. Two stored places carry the old form `…#<slice image>#<placement id>`:
--   progress.state   the save game's variables `[<toon>/]picked:<slice image>#<id>` → `[<toon>/]picked:<id>`
--                    (the save game JSON as text: a key is `"…picked:<path>#<id>"`, a path never holds `"` or `#`)
--   rate_limits.ref  the server's once-per-ref memory of story grants / takes (buckets `grantref:` / `takeref:`):
--                    `vn:<item>:<page>#<slice image>#<id>[@<period>]` → `vn:<item>:<page>#<id>[@<period>]` – the image
--                    is the segment between two `#` that ends in an image file extension
-- The viewer writes the new form with this deploy (slice-items.ts `pickedKey`, the grant's place); readers' browsers
-- rename their own on load (@scutoons/content save-game.ts `pickedKeyOf`, also for a stale tab's write). Idempotent:
-- only rows still holding the old form change – a second run changes 0 rows. Every compared string is a literal.

UPDATE `progress` SET `state` = REGEXP_REPLACE(`state`, 'picked:[^"#]+#', 'picked:')
WHERE `state` REGEXP 'picked:[^"#]+#';

UPDATE `rate_limits` SET `ref` = REGEXP_REPLACE(`ref`, '#[^#]+\\.(webp|png|jpe?g|gif|avif)#', '#')
WHERE (`bucket` LIKE 'grantref:%' OR `bucket` LIKE 'takeref:%') AND `ref` REGEXP '#[^#]+\\.(webp|png|jpe?g|gif|avif)#';
