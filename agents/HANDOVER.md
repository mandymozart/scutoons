# HANDOVER — the current state (read after RULES.md; limit 60 lines: replace, never append; no rules, no todos)

State 2026-10-12. How agents work: agents/RULES.md. Open work: agents/ISSUES.md (bugs) + agents/FEATURES.md.
Decisions: agents/DECISIONS.md.

## Where things are
- `main` is the only branch and is production. Pushing `main` runs `.github/workflows/deploy.yml`: it deploys only the
  targets whose live `git_sha` is behind (`deploy/changed.mjs`), one FTP upload at a time, then runs the pending DB
  migrations (`POST /api/deploy/migrate`). `npm run deploy:status` shows what is live (deploy/README.md).
- Production runs the platform on the real hosts since 2026-10-05: scutoons.com/www (viewer), qr., api., creator.,
  admin., shop., embassy. (all under `/web/`); cdn.scutoons.com → bunny.net. The cutover is finished
  (archive/CUTOVER.md); the old beta phase was dropped, open work is the 10.x line.
- Content lives in the production DB (`entity_versions`), edited in the creator; language-neutral documents
  (`chapter:<toon>:<n>` with its groups inline – the `scene:…` documents were inlined 2026-10-19 –, `season:…`, `toon:<toon>`, texts as locale maps). The per-language legacy copies and
  the per-toon map lists were purged 2026-10-11/12 (archived in `entity_versions_legacy`).
- The catalog (items, recipes, albums, products, editions, featured, code types, toons) lives in the DB since
  2026-10-04, edited in the admin (`/catalog/<kind>`, history + undo; agents/CATALOG.md); tests use
  `server/tests/fixtures/catalog/`.
- Production DB: MariaDB 10.11, tables utf8mb4, strict session sql_mode (`DB_SQL_MODE`); codes case-sensitive
  (utf8mb4_bin). All migrations up to `2026-10-12-purge-legacy-map-lists` are applied (`migration_status`: 0 pending);
  `2026-10-13-apose-per-look` (branch of the A-pose-per-look worktree) runs with its deploy.
- Local dev: MariaDB in Docker with the production seed (`npm run dev:db`), test user `agent@scutoons.test`.
- Horizontal groups (2026-10-16, FEATURES 10.100): Swiper left the chapter page – a horizontal group is the pinned strip
  (`strip.svelte`) or, with `params.endless`, the endless band (`strip-endless.svelte`); `2026-10-16-endless-strip`
  migrates the stored params with that deploy. Swiper is gone entirely (2026-10-17): the home slider runs on the band's
  utility, the score view on CSS scroll-snap and loads lazily (research/TEXT-ACROSS-SLICES.md §9).
- The world map: places unlock only by the `unlockLocation` event; place images 1x/2x/3x on a hex grid, `size` per
  place, shared `MapTerrain`; the phone editors share one bottom bar (`PhoneBar`) – agents/MAP.md §8.
- AI = the member's own Claude through the SCUTOONS MCP (agents/MCP.md); the Ask panel / service is gone (2026-10-09,
  clean-up done). Developers' agents run migrations over the MCP (developer role).
- Embassy live (embassy.scutoons.com, agents/EMBASSY.md); newsletter live (agents/NEWSLETTER.md).
- Parked until February 2027: live performance mode (research/LIVE.md), the onion camera (checkout
  `C:\Development\scutoons-onion-camera`, branch `items/onion-camera`).
- Other checkouts (`git worktree list`): owner-requested drafts and parked branches (newsletter, onion-camera,
  storyboard, the 2026-10-12 catalog → creator and map-places drafts) and the app's session worktrees
  (`.claude/worktrees/claude-*`) – the owner reviews and merges them; agents don't.
- Reader avatar (owner 2026-10-12): the A-pose belongs to a look (`look.apose`), the body is { character, persona,
  look }; no naked body; worn items sit as icons in the avatar page's slots, nothing is drawn on the body (ITEMS.md
  "Reader avatar bodies", "Wardrobe"). Real dress-up is a separate research session.
- Painted map + fog of war (2026-10-12, MAP.md §8.9): painted terrain (Luma) + a plaster layer of the same shapes,
  tiles on the CDN (`catpope/map/terrain/<layer>/v<n>/`); `MapLayer.fog: { radius }` shows the painted layer only around
  unlocked places.
- Storyboard 3D cast: artists will provide a 3D model per character; the renderer loading it is FEATURES 10.95.
  The sketch → 3D experiment is removed; its learnings: research/CHARACTERS3D.md.

## Last run
2026-10-18 evening, unattended on `main`: custom collage presets (research/COLLAGE.md §12, steps 1–6, one agent each).
- **Built** ("Next: 'Save as preset' – my own colourations as named presets … applied to any piece"): the platform-wide
  document `colorations`; the server bakes custom presets into the chapter and score answers (readers unchanged); the
  creator draws / edits / checks them on slices and the score stage, and the chapter / season / toon / score Publish
  lists offer "Collage presets"; the preset grid previews each look on the piece ("in dem Wort auf dem Selector zeigen,
  was das dann wird") + Save as preset … · Update preset · ••• Rename / Delete; the MCP validates and judges presets.
  §12.8's defaults (FEATURES 10.105); changed from the spec: from the 6th custom preset (measured) a "Custom (n)" fold.
- **Final review** (2 lenses) → fixed: an unsaved presets copy is offered at load (chapter editor and score stage, next
  to the page's Recover bar); a score's catalog page lists "Collage presets" in Publish; a name never ends in half an
  emoji (the server refused the document); Rename selects the old name. The minors left: COLLAGE §12.9 "Open".
- **Where:** steps 1–4 and the phone item / piece tool fix are on production since your push this evening. Not pushed,
  rebased on origin/main (2026-10-19): the autosave fix, the MCP part (live after the MCP's next restart), the docs, the
  final-review fixes – 10 commits.
- **Green:** content 319 tests; creator 0 / 0, 105 tests; @scutoons/ui 0 / 0; viewer 0 errors (11 old warnings), 143
  tests; per step: PHPUnit of the touched classes (74 tests) + PHPStan, `check:creator-collage` 51/51,
  `check:creator-chapter -- --pieces` (catpope 4 slice 1: phone 10/10, recover 5/5; its 2 old misses: anchor top-left,
  scale handle), `check:mcp` 48/48 – `colorations` cleaned up. By hand on a slice: the grid's previews, Save as preset,
  Rename, Delete (the tombstone), both Recover bars at load.
- **Red:** `check:tools-css` 16 old lines (map editor, phone-bar, bar-tabs), none in a touched file; 58 open FEATURES +
  ISSUES (limit 40) – no new item added.
