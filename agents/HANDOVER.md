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
- Live performance mode restarts 2026-11-16 for the Top Toy workshop (2027-02-17; research/LIVE.md, KANON.md §8);
  parked: the onion camera (checkout
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
2026-10-20 night → 10-21, unattended on `main`: the storyboard studio moved into the creator (#118, owner: "now over night").
- **Built** (research/STUDIO-IN-CREATOR.md §4): the model `@scutoons/content/storyboard` (#119, parity-tested against the
  old JS); `storyboard:<toon>:<n>` + `storyboard_generations` (= the budget ledger) + ch3–5 imported + the work storage
  on the API host (#120); the creator page /<toon>/chapter/<n>/storyboard – shots, inspector, script, problems, rhythm,
  the member's workspace folder (#123); the three.js 3D view (#124); generators through the server (Luma images,
  Runway) with the budget gate in admin → Settings + the Generate panel (#121); the MCP's storyboard tools incl.
  start_generation (#122); the variants board + Compare (#125); the phone camera at /h/<code> (WebRTC), takes, the
  timeline, Output (#126) – the creator's pages moved into the route group `(app)`, URLs unchanged; the local kit part 1
  – the Figma relay inside the MCP (never auto-started from a checkout), the plugin download, generate_local, "Set up
  your machine" (#127).
- **Owner steps:** GitHub secrets LUMA_API_KEY / RUNWAY_API_KEY (+ GEMINI) and an amount in admin → Settings →
  Generation budget (until then paid jobs answer no_budget); measure the API host's request limit + disk (#120).
- **Look at:** the storyboard page on ch3 (the import ran with the 10-20 night push – edits made in the old local studio
  since then are not in it); the 3D view; the phone camera on a real phone.
- **Assumed overnight, owner to confirm** (DECISIONS): the old studio stays until you have used the creator's (#128);
  ✓ approves without a dialog and "the mark before" lasts a session (#125); the import stored ch3–5 as published versions.
- **Open:** Q7 taste rules (#118); #127 part 2 after the art freeze (figma-* scripts → MCP tools, then the switch –
  MCP.md §3a); #128 the removal.
- **Green:** content 352 tests; creator 0 / 0 + 260 tests + build; PHPUnit 474 / 5629 + PHPStan; tools/mcp 35 tests +
  build:mcp; admin 0 / 0; viewer 0 errors (11 old warnings) + 147 tests; figma-bridge test.mjs 29. Not pushed.
