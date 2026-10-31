# HANDOVER — the current state (read after RULES.md; limit 60 lines: replace, never append; no rules, no todos)

State 2026-10-12. How agents work: agents/RULES.md. Open work: GitHub issues + the board "SCUTOONS" (RULES
"Backlog", since 2026-10-19). Decisions: agents/DECISIONS.md.

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
- The catalog (items, recipes, albums, products, editions, featured, code types, toons) lives in the DB since 2026-10-04,
  edited in the admin (`/catalog/<kind>`, history + undo; agents/CATALOG.md); tests use `server/tests/fixtures/catalog/`.
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
- `deck/` (2026-10-29, #172 / #173): the Rust workspace of research/CYBERDECKS.md – relay, bridge, adapters, protocol from
  `deck/protocol/schema.json` (+ shared/live); `deck/os` = the class-A live ISO (`npm run deck:iso` in Docker, `deck:vm`), deck/pkg, HARDWARE.md.
- Live performance mode restarts 2026-11-16 for the Top Toy workshop (2027-02-17; research/LIVE.md, KANON.md §8);
  parked: the onion camera (checkout `C:\Development\scutoons-onion-camera`, branch `items/onion-camera`).
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
2026-10-30 evening → 31 night, coordinated with the OSCT orchestration; one agent at a time on `main`. Pushed on the
owner's word up to 80e26c22 (API scene-content + the cathedral set live); **29 commits since are not pushed**.
- **The 3-device test (evening, owner):** the conductor on the PC + the X1 on stick s5 + the S22 in Chrome
  (`deck/lab` page over `adb reverse`) in one session `CATH` over the LAN relay – beings in the circle, values, moods.
- **Built, deck:** #188 moods (20, feedback ↔ sound), the set without the incense burner, tethered buttons + feedback
  (hover, gauge, labels, lock, reset, touch), no auto-turning camera · #195 sync from the API into the cache + the
  conductor pushes a scene sync (`scene.sync` / `deck.synced`) · relay keep-alive (ghost decks gone) · deck-persist
  (fast first boot, cut-off recovery, Wi-Fi kept) · `deck-set`, tty2 without a login · #74 the cathedral re-baked in
  Blender (low 35K tris, 0.5 MB; v2 on the CDN + migration 2026-10-31-cathedral-set-v2, not deployed).
- **Built, web:** #197 the browser deck (deck/lab) → #198 `deck-space` crate as wasm (WebGL2 on the S22, no WebGPU
  there) under `/live/deck` in the viewer (unlisted, open to all; Svelte overlay; `@scutoons/live-deck`).
- **ISO:** deck/os/out/s6 (02:57) – all of the above except #195's push (built after). s5 is on the stick.
- **Look at / owner:** push (deploys the v2 migration + `/live/deck` + .htaccess COOP/COEP on `/live/…`); s6 on the
  stick; `/live/deck` on the S22 (fps, touch, the head – it was locked at night); the wasm files are git-ignored, so
  production's `/live/deck` has no 3D / sound until the deploy builds them or they are committed; a hosted relay (wss);
  #181 sync / tap / off per deck; who may turn other decks' buttons; clean up the `scutoons/rooms/` and `.3d` set copies
  under Admin → Assets → Unused.
- **Green:** deck workspace 303 tests + clippy -D warnings (also wasm32); viewer 154 tests / svelte-check 0 errors;
  live-deck 15, live-core 15; server 490 (MariaDB) at #195's API. `check:tools-css` red before this run (creator / ui).
