# Merge note — branch `media-quality` (2026-10-01)

Branched from `platform` a83b1cc. **Merge after the 2026-10-04 release** (owner). Nothing pushed.
What it is: DESIGN.md "Media quality", "Video placement – full viewport", "Creator slice tab"; PLAN.md "RULE: one file per
quality level"; DATA.md "Media quality"; MEMORY.md 2026-10-01 "Media quality setting + full-viewport video placement".

## Merge

- `git merge media-quality` into `platform` (or `main` after the release). Expected conflicts are small: the account page
  gets one import + one `<section>` (`viewer/src/routes/(account)/account/+page.svelte`, `sso` also edits that page –
  keep both); `viewer/messages/*.json` adds a new top-level `media_quality` group after `player`; `creator/messages/*.json`
  renames `creator.image` → `creator.background`, drops `video_source_*` / `video_sources_hint`, adds `video_*` keys;
  `server/tests/api-smoke.php` adds a "Media quality" block after "Marketing opt-in"; `server/src/Domain/Accounts/Users.php`
  + `AccountService.php` (`sso` touches other server files). Docs append sections (PLAN, DESIGN, DATA, MEMORY, PLATFORM
  cutover list, server API/CHANGELOG/DATABASE).
- `wardrobe` (avatar/items): no overlap expected.

## After merging — local (the owner's dev stack)

1. `cd server && php bin/console migrate` → `2026-10-01-media-quality` (`users.media_quality`, default `auto`).
2. Backup, then the new chapter-0 version in the local store:
   `cp server/db/local.sqlite server/db/local.sqlite.bak-2026-10-0X-media-quality`
   `php bin/console import-content ../shared/content/seed --toon=catpope --only=chapter --update`
   (expect "2 updated" = `chapter:catpope:0:{de,en}`; everything else unchanged – checked on a copy of the store
   2026-10-01. If the owner changed chapter 0 in the creator since, re-check with `--dry-run` first.)
3. `npm run dev` (restarts nothing that runs; the viewer/creator pick the new code up by HMR or a restart).

## After merging — production (after the release)

1. phpMyAdmin: `server/migrations/mysql/2026-10-01-media-quality.mysql.sql` (idempotent).
2. Deploy api + viewer + creator ("Deploy all"). An older viewer build still running meanwhile plays `video.src` (720p)
   once step 3 ran – no error, just no setting.
3. phpMyAdmin: `shared/content/seed/2026-10-01-media-quality-chapter0.mysql.sql` (chapter 0's new published version per
   locale; idempotent; `content-seed.mysql.sql` would skip the existing keys). The library cache keys on version ids, so
   the new version is live at once.
4. CDN: nothing new to upload (the three `roar-little-kitty.{480p,720p,1080p}.mp4` are the release's files). **Check
   CORS on `cdn.scutoons.com`**: "Automatic" measures with a `fetch` of 512 KB of the 480p file; without
   `Access-Control-Allow-Origin` it fails and such readers get low (Chrome/Edge/Android decide slow connections without it,
   but a 4G-class connection is always measured). DevTools on `/catpope/chapter/0` (setting Automatic): a `fetch` with
   `Range: bytes=0-524287` → 206.
5. Smoke: account page (guest + member) → the selector; chapter 0 per level loads 480p / 720p / 1080p.

## Follow-up in this branch (owner, 2026-10-01)

Four fits (`contain` "Ganz sichtbar", `width` "An Breite", `height` "An Höhe", `cover` "Füllen (beschnitten)"), frames in
the column (`SliceVideo.frame`, presets 5:8 / 6:13 / 9:16 / 1:1 / custom 4:1…1:4), the creator's basic picker
(`creator/src/lib/components/placement-picker.svelte`: "Ganz · Breite · Höhe · Füllen", "Oben · Mitte · Unten" when it
matters, the ratio select) and the short size line. The owner's trial in the preview (chapter 0 published as full viewport,
fit height) lives only in the worktree's DB copy – nothing to carry over unless the owner wants it in the seed. No stored content uses `viewport` / `frame`, so
nothing to migrate. New creator messages `video_frame_*`, `video_fit*`, `video_position`, `slice_size_*`.

## Verified on this branch (own servers on throw-away ports 18000 / 18001 / 19000 / 20000, a copy of the owner's store)

`npm run check` (0 errors), `npm run build` (all apps), `server composer test`, `qr composer test`, vn-check,
check-media-quality 46/46, check-prolog 46/46 and the other browser checks – numbers in MEMORY.md and the final report.

## Open for the owner

- Automatic's thresholds (≥ 7 Mbit/s → 1080p, ≥ 5 → 720p, else 480p; ≈ 1.5 × the chapter-0 bitrates) and "a 4G-class
  connection is always measured" (Chrome's `downlink` estimate was ≈ 1.5 Mbit/s even on localhost).
- The season trailer's hero loop (`Season.trailer.sources`, 1280w / 1920w by `min-width`) still picks by viewport –
  move it to the quality setting too?
- Should a member's choice also apply when signed out on the same device? (It does now: the choice is mirrored to the
  device.)
