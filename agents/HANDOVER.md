# HANDOVER — read this first (short on purpose)

State at the end of the 2026-10-02/03 coordinator session. Keep this file short: replace, don't append. Details live in
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
  2026-10-03-code-case – run on production, verified). Migrations are run by the owner in phpMyAdmin (SQL tab; select
  the database first – `DATABASE()` is `information_schema` otherwise; the DB is `d0487143`).
- The catalog (items, recipes, albums, products, editions, featured, permissions / code types, toons) lives in the
  database since 2026-10-04 (CUTOVER 10.29, agents/CATALOG.md), edited in the admin (`/catalog/<kind>`, history + undo);
  no file seed any more (10.30): local = the prod seed, tests = `server/tests/fixtures/catalog/`. Production: `2026-10-04-catalog.mysql.sql` has run
  (owner check 2026-10-05: all 11 tables, old settings folded). Media tooling for both apps:
  `@scutoons/media` (shared/media).
- Local dev: **MariaDB in Docker only** – no SQLite any more (owner 2026-10-05): `npm run dev:db` (container, migrate,
  prod seed, sandbox, API, test user), PHPUnit with `npm run dev:db test`. A worktree's own database:
  `SCUTOONS_DB_NAME=scutoons_<x>`. Missing media: `npm run dev:db -- media --apply` (from bunny.net). Test user: `npm run dev:test-user` (`--browser <url>` for a sign-in snippet).
- Post-beta work list: CUTOVER.md items 10.10–10.30.

## Working rules learned (owner)
- Commit ONLY on `main`, guarded: `[ "$(git branch --show-current)" = main ] && git commit …`. **Never push** – pushing
  `main` deploys; the owner pushes, or asks you to (owner 2026-10-05). Never add Claude attribution lines.
- Deploy: the owner runs `git push origin main` (or asks an agent to).
- Stored format changes (owner 2026-10-06): first analyse the existing data (dev DB = latest production dump), then a DB
  migration (SQL, idempotent, a SELECT to check first; the owner runs it in phpMyAdmin) and the old form leaves the code –
  no aliases. Only when a migration can't work (backwards compatibility): a code helper WITH creator feedback ("action
  required" error or warning) – never a silent load-time conversion.
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
- Every control at `--control-height`; icon-only buttons are round (`class="sc-round"`, tools.css); one tooltip style
  via `title`; bold very rarely; field groups without frames (legend = label).
- Spacing on a 1rem line: sidebars, outline, fold button all start at 1rem; dividers run edge to edge.
- Right-side drawers/panels drag wider (up to half the window); the sidebar folds to a rail (Feather chevrons button at its top right / drag / Ctrl+\); the script outline folds
  the same way, its rail labelled "Outline". The script is read-only until "Edit script" is confirmed.
- Confirmations are in-page (`confirmDialog` / `askDialog` from @scutoons/ui), never `confirm()`.
- Right-docked page panels (Problems, History, help) sit left of the Ask panel (`--ask-inset`); with both open the
  sidebar folds to its rail. Warnings can be accepted (✓ "Looks fine", versioned in the document – DATA.md §5); errors
  can't. The admin catalog has the same panel (items / recipes / albums; stored in the entry, DATA.md §5).
- Media fields are slots (preview = button, empty dashed slot), not path text fields; multi-resolution media =
  one slot with resolution chips (default 720p → 480p → 1080p).
- Show design drafts (on the real page, mobile + desktop) before building bigger UI.
- Empty states (owner 2026-10-06): an empty list shows nothing – its add button is enough. A text only when ALL hold:
  the page / main area would otherwise be completely blank with nothing to act on, the reason isn't obvious, and it says
  what to do. Never in tabs, inspectors, side panels or lists with an add button; never framed – one muted line in the
  flow (`.sc-empty`, tools.css). DESIGN.md "Empty states".

## Open owner decisions / loose ends
- Production: run `2026-10-06-show-text-box.mysql.sql` with the deploy of `showTextBox` (CUTOVER 10.32 – the SELECT
  first; old code can't read the new event type, new code not the old one).
- Onions: no onion item in the catalog yet (needed before it can respawn). The onion camera (branch `items/onion-camera`,
  6 commits, checkout `C:\Development\scutoons-onion-camera`) waits until **February 2027** (owner 2026-10-05: no priority
  before); it predates the catalog in the database – its items.json changes go through the admin catalog then.
- Respawn: should the server also read a placement's own interval (today only the item's range is enforced)?
- `CloseButton` (sheets/dialogs) round too?
- Upload the text-free season cover art and switch the cover editor's lettering on (CUTOVER / DESIGN "cover editor").
