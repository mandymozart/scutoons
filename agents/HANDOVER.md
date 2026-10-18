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
  (`chapter:<toon>:<n>`, `scene:…`, `season:…`, `toon:<toon>`, texts as locale maps). The per-language legacy copies and
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
2026-10-18 night → morning, unattended on `main`; this run pushed nothing.
- **Collage pieces** (owner 2026-10-17, research/COLLAGE.md §8 steps 1–7, nothing cut): vectors, character pictures and
  images on slices (a slice → the scissors tool; looks, blend, fill, outline, shadow, blur in the piece inspector;
  readers see them on picture slices, under the lettering) and on scores (Catalog → Items → Cloud 9 → More → Collage:
  a piece sticks to a bar + staff; readers get them with the MusicXML). Built with COLLAGE §10's defaults (FEATURES 10.105).
- **Vector library** (research/VECTORS.md steps 3–5): Scrooge in parts, `asset_shapes` + `GET /api/assets?kind=shape`,
  the local import, the MCP tool `import_vectors` – the 436 shapes are on production since 09:57 (run as the owner).
- **Green:** `npm run check` of content / reader / ui / api / creator / viewer (the viewer's 11 old warnings), PHPStan,
  both builds, `check:creator-chapter -- --pieces` 38/38, `check:creator-collage` 34/34. **Red:** `check:tools-css`
  16 old lines (map editor, phone-bar, bar-tabs); the full `check:creator-chapter` can't run (catpope 1 / 2 hold your
  drafts of 2026-10-07/08, catpope 3 lacks its fixtures, the sandbox chapters are scene refs it can't read); FEATURES +
  ISSUES hold 56 open items (limit 40).
- **Before a push:** `main` is 28 commits ahead of `origin/main` and 13 behind (your daytime pushes: the tools as PWAs,
  team push, attributions, phone fixes) – merge `origin/main` first (a trial `git merge-tree`: no conflicts).
