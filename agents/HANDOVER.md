# HANDOVER — read this first (short on purpose)

State at the end of the 2026-10-02/03 coordinator session (+ the 2026-10-11 map / phone-editor session, below). Keep this file short: replace, don't append. Details live in
the files it points to – read those only for the part you touch (MEMORY.md is 1800 lines: grep it, never read it whole).

## Where things are
- `main` is the only branch (owner 2026-10-05: `platform` abandoned "for now"); production = `main`. Pushing `main` runs `.github/workflows/deploy.yml`: it deploys only the
  targets whose live `git_sha` is behind (`deploy/changed.mjs`), one FTP upload at a time. `npm run deploy:status` shows
  what is live. Docs: deploy/README.md.
- Production runs the NEW platform on the real hosts since 2026-10-05: scutoons.com/www (viewer), qr., api., creator.,
  admin., shop. (all under `/web/`). Test hosts next./qr-next. and legacy. are deleted; old folders stay as rollback
  (CUTOVER 8.x). 8.5 late-delta check passed – Phase 8 complete. Phase 9 (beta) scratched (owner 2026-10-05); next:
  Phase 10. Embassy is live (10.6, embassy.scutoons.com).
- Content lives in the production DB (entity_versions), edited in the creator. Production is the only source (CUTOVER
  10.30; `shared/content/seed` is gone): local dev loads production's content + catalog from `server/db/prod-seed.sql`
  (gitignored phpMyAdmin export) via `npm run dev:db` / `npm run dev:db -- seed`.
- Production DB: MariaDB 10.11.14, server sql_mode NO_ENGINE_SUBSTITUTION, latin1 server default; every table utf8mb4.
  The API sets a strict session sql_mode (`DB_SQL_MODE`). Codes are case-sensitive (utf8mb4_bin, migration
  2026-10-03-code-case – run on production, verified).
- **Migrations: the owner no longer runs any by hand** (owner 2026-10-12: "Ich muss keine Migrations mehr ausführen –
  wichtig für alle Agents"; BACKLOG 10.61): a migration file in `server/migrations/mysql/` runs on production by itself
  with the deploy (after the API upload, `POST /api/deploy/migrate` with MIGRATE_KEY). An agent with the owner's
  developer role checks and runs them over the SCUTOONS MCP: `migration_status`, `plan_migrations` (a dry run with row
  counts), `apply_migrations` (that plan; changed rows backed up). So: never ask the owner to open phpMyAdmin – write the
  file idempotent (a second run changes nothing), dry-run it on the dev DB, push = it runs; check with migration_status
  (agents/MCP.md §8, server/docs/DATABASE.md "Migrations").
- The catalog (items, recipes, albums, products, editions, featured, permissions / code types, toons) lives in the
  database since 2026-10-04 (CUTOVER 10.29, agents/CATALOG.md), edited in the admin (`/catalog/<kind>`, history + undo);
  no file seed any more (10.30): local = the prod seed, tests = `server/tests/fixtures/catalog/`. Production: `2026-10-04-catalog.mysql.sql` has run
  (owner check 2026-10-05: all 11 tables, old settings folded). Media tooling for both apps:
  `@scutoons/media` (shared/media).
- Local dev: **MariaDB in Docker only** – no SQLite any more (owner 2026-10-05): `npm run dev:db` (container, migrate,
  prod seed, sandbox, API, test user), PHPUnit with `npm run dev:db test`. A worktree's own database:
  `SCUTOONS_DB_NAME=scutoons_<x>`. Missing media: `npm run dev:db -- media --apply` (from bunny.net). Test user: `npm run dev:test-user` (`--browser <url>` for a sign-in snippet).
- Map + phone editors (2026-10-11): places are unlocked ONLY by the `unlockLocation` event (MAP.md §8.5a/b – toast;
  in a VN script / click / item tap the map flies to the place); place images in 1x / 2x / 3x with a plaster model, a
  `size` per place on a hex grid (`snapPlace`), shared `MapTerrain`; the phone editors have one bottom bar (`PhoneBar`:
  Save · History · Publish · More + a selection bar, sheet tabs on the sheet – UI-CONCEPTS E.10). Next steps: BACKLOG
  10.67 (the map's scale ÷8, deep zoom, tiles), 10.61 (migrations behind a developer role), 10.62–10.66; small UI
  fixes the owner found on the phone (the resolution slot's layout, the chapter-link wording, the size slider's label)
  in 10.65.
- Open work (post-beta items 10.x + known bugs): agents/BACKLOG.md – one list (owner 2026-10-06; "CUTOVER 10.x" in older notes = the same number there).

## Working rules learned (owner)
- Commit ONLY on `main`, guarded: `[ "$(git branch --show-current)" = main ] && git commit …`. **Never push** – pushing
  `main` deploys; the owner pushes, or asks you to (owner 2026-10-05). Never add Claude attribution lines.
- Deploy: the owner runs `git push origin main` (or asks an agent to).
- Stored format changes (owner 2026-10-06): first analyse the existing data (dev DB = latest production dump), then a DB
  migration (SQL, idempotent, a SELECT to check first; it runs with the deploy – see "Migrations" above) and the old form leaves the code –
  no aliases. Only when a migration can't work (backwards compatibility): a code helper WITH creator feedback ("action
  required" error or warning) – never a silent load-time conversion. Exception: data in readers' browsers (localStorage) – no
  SQL reaches it, no author is involved: a one-time helper on load (e.g. the old per-toon game states → the save game).
- One checkout, one dev DB `scutoons`, no worktrees / extra DBs / SQLite (owner 2026-10-06); agents one after another or
  in at most two lanes with separate files, each committing only its own named paths.
- The owner works in the shared browser pane: test in your OWN tab (`tabs_create`), never in theirs.
- Give the owner steps one at a time; don't re-ask answered questions (e.g. "no upper-case certificates exist – ever").
- Agents work on their own branch in a worktree; the coordinator reviews, merges into `main` (no push), runs the checks
  (`npm run check`, `check:tools-css`, server phpunit + phpstan).

## UI rules (owner) – check before building UI
- Less UI is better UI. No duplicate controls: the creator header's text-language switch is the ONLY language switch
  (script, cover editor, credits follow it).
- Sizes in rem, never px (owner 2026-10-04: "a dogma across the app") – px only inside the tokens (border widths) and
  the existing breakpoints.
- Every control at `--control-height`; one tooltip style via `title`; bold very rarely; field groups without frames (legend = label).
- Round vs square (owner 2026-10-07): a **standalone icon-only button is round** (`class="sc-round"`, tools.css);
  **grouped controls are square** – segmented groups (`sc-seg`), toolbars, tab rows and the language switchers of every
  app (creator, admin, embassy, viewer, shop); a control with text is square too (DESIGN.md "Round vs square").
- Spacing on a 1rem line: sidebars, outline, fold button all start at 1rem; dividers run edge to edge.
- ONE layout model for every area (agents/DOCKING.md, approved 2026-10-06): regions tile (sidebar | page | right dock –
  AppShell in creator, admin, embassy), one `Divider` for every drag edge (up to half the window), one fold to a
  `Rail` as wide as its round button + 0.5rem either side (sidebar: chevrons at its top right / drag / Ctrl+\; script outline: rail "Outline"; right dock: the panel's
  `DockFold`, rail with its icon + title, not remembered); every rail's button at the height of the region headers'
  buttons (the dock panels'); sizes remembered per panel kind. The main folds to a rail only BY HAND (the leftmost panel's "Fold
  the page", Ctrl+Shift+\ – never by itself; owner 2026-10-07). Dock panels and the slice inspector MAXIMIZE = fold
  everything else to its rail (button / Ctrl+Space / Esc; railed regions stay mounted; the ActionBar stays – DOCKING.md).
  The script is read-only until "Edit script" is confirmed.
- Confirmations are in-page (`confirmDialog` / `askDialog` from @scutoons/ui), never `confirm()` – the primary action last, one
  primary only: the save-draft note's button says "Save without note" while empty, "Save" once typed (owner 2026-10-06).
- Page panels (Problems, History, Flow, Reader, Info, the admin's list detail) open SIDE BY SIDE in the shell's right
  dock (`useDock`, one per kind, fixed places detail · History · Problems · Flow · Reader · Info); out of
  room → narrower together, then the least recently used folds to its rail (BACKLOG 10.57, DOCKING.md "Several
  panels"); the sidebar folds by itself only for the page's `foldWhen` or a maximized region. Pickers stay modal
  drawers (`Sheet`). Warnings can be accepted (✓ "Looks fine", versioned in the document – DATA.md §5); errors
  can't. The admin catalog has the same panel (items / recipes / albums; stored in the entry, DATA.md §5).
- Media fields are slots (preview = button, empty dashed slot), not path text fields; multi-resolution media =
  one slot with resolution chips (default 720p → 480p → 1080p).
- Show design drafts (on the real page, mobile + desktop) before building bigger UI.
- Empty states (owner 2026-10-06): an empty list shows nothing – its add button is enough. A text only when ALL hold:
  the page / main area would otherwise be completely blank with nothing to act on, the reason isn't obvious, and it says
  what to do. Never in tabs, inspectors, side panels or lists with an add button; never framed – one muted line in the
  flow (`.sc-empty`, tools.css). DESIGN.md "Empty states".

## Open owner decisions / loose ends
- 2026-10-11: `2026-10-11-unlock-location-events` ran on production (verified by MCP: no `unlock` on the map, events in
  chapters 1–5) – pasted, so NOT recorded in `schema_migrations` – `2026-10-11-developer-role` records it (BACKLOG
  10.61, built: run that file once in phpMyAdmin + set the secret MIGRATE_KEY – from then on the deploy runs migrations,
  developers' agents over MCP); empty the API's `storage/cache/library` once. Publish the map draft v257 (dioramas, sizes, grid). Lesson:
  phpMyAdmin's connection is utf8mb4_unicode_ci, the tables utf8mb4_general_ci – migrations compare columns with
  literals, never user variables (#1267); give the owner a comment-free ASCII `.paste.sql`.
- AI = the member's own Claude through the SCUTOONS MCP (agents/MCP.md, owner 2026-10-09; the Ask panel and service are
  gone). Production: run `2026-10-09-mcp-drafts.mysql.sql` + `2026-10-09-api-tokens.mysql.sql` (the SELECTs first) with
  the deploy; after it delete the bunny Magic Containers app, the `ask.` DNS, the ghcr image `scutoons-ask`, GitHub vars
  `VITE_ASK_URL` / `BUNNY_ASK_APP_ID` / `BUNNY_ASK_CONTAINER`, secret `AGENT_SERVICE_KEY`; revoke the Anthropic API key;
  try the Claude Desktop extension once (scutoons.com/account → AI connections). mcp-drafts + api-tokens ran on production
  2026-10-09, deployed a5f3d18; drop-ask (the Ask tables are gone) too.
- Newsletter (BACKLOG 10.58, agents/NEWSLETTER.md, branch `newsletter`): production – run
  `2026-10-07-newsletter.mysql.sql` (the SELECTs first), secret `NEWSLETTER_CRON_KEY`, KAS cron every 5 minutes; then
  set the `qr-claim` contacts' languages, import the event lists, write + test + start (the steps: NEWSLETTER.md "Going live").
- Copy / paste (BACKLOG 10.39, built 2026-10-07; PHPUnit incl. `ItemLimitsTest` + steps 6–8 checked headless after
  Docker came back; still owed: `check:tools-dock` + the admin's item-limit fields with the admin running). Production: nothing to run
  (item limits = catalog JSON fields).
- Production: run `2026-10-07-account-deletion-frees-codes.mysql.sql` (the two SELECTs first; the trigger line with
  delimiter `$$`) – any time, no code depends on it: a deleted account (`DELETE FROM users`) frees its codes, a unique
  item is back up for grabs (owner 2026-10-07; ITEMS.md "Item limits").
- Production: run `2026-10-06-show-text-box.mysql.sql` with the deploy of `showTextBox` (CUTOVER 10.32 – the SELECT
  first; old code can't read the new event type, new code not the old one).
- Production: run `2026-10-06-clock-syntax.mysql.sql` with the deploy of expressions step 2 (CUTOVER 10.33 – the SELECT
  first; `{x:clock}` → `{clock(x)}`, 0 rows in the dump).
- Production: run `2026-10-06-repeat.mysql.sql` + `2026-10-06-display-name.mysql.sql` with the deploy of expressions
  step 3 (CUTOVER 10.34 – the SELECTs first; trigger / item `once` → `repeat`, 81 versions in the dump). Event ids need
  no migration: the creator assigns them, the next save of a chapter / scene stores them (a bar says so).
- Production: run `2026-10-06-target-value.mysql.sql` (CUTOVER 10.35 – the SELECT first; numeric jump target values →
  strings, 11 versions of scene-14 in the dump). Decide: the published scene-14 still has an unknown `end` event
  (content:check reports it) – publish the scene's newest version or delete the event.
- Production: run `2026-10-06-global-save-game.mysql.sql` with the deploy of viewer + API (CUTOVER 10.36 – the SELECTs
  first: members with per-toon states; each toon's variables go in as `<toon>/x` – no clashes). One save game for
  every toon, variables with a domain: `x` this toon's, `global.x`, `<toon>.x` (DATA.md "The save game"); guests' old
  keys are taken in by the viewer on load (the stored-format rule's exception: no SQL reaches a browser, no author
  involved).
- Production: run `2026-10-07-sequence-no-type.mysql.sql` with the deploy of BACKLOG 10.42 (the SELECTs first: a
  sequence entry that isn't a chapter → must be 0; then the rows to change, 49 versions in the dump): a season's
  `sequence` holds chapters only, `"type":"chapter"` goes (old code reads entries without it, new code rejects it).
- Onions: no onion item in the catalog yet (needed before it can respawn). The onion camera (branch `items/onion-camera`,
  6 commits, checkout `C:\Development\scutoons-onion-camera`) waits until **February 2027** (owner 2026-10-05: no priority
  before); it predates the catalog in the database – its items.json changes go through the admin catalog then.
- Respawn: should the server also read a placement's own interval (today only the item's range is enforced)?
- `CloseButton` (sheets/dialogs) round too?
- Upload the text-free season cover art and switch the cover editor's lettering on (CUTOVER / DESIGN "cover editor").
