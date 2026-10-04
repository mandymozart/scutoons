# HANDOVER — read this first (short on purpose)

State at the end of the 2026-10-02/03 coordinator session. Keep this file short: replace, don't append. Details live in
the files it points to – read those only for the part you touch (MEMORY.md is 1800 lines: grep it, never read it whole).

## Where things are
- Release branch `platform`; production = `main`. Pushing `main` runs `.github/workflows/deploy.yml`: it deploys only the
  targets whose live `git_sha` is behind (`deploy/changed.mjs`), one FTP upload at a time. `npm run deploy:status` shows
  what is live. Docs: deploy/README.md.
- Production runs the NEW platform on the test hosts: api., next. (viewer), qr-next., creator., admin., shop.
  **Not switched yet:** CUTOVER.md Phase 8 (legacy./qr-legacy. hosts, then qr.scutoons.com and scutoons.com → `/web/`).
  Beta (Phase 9) is internal; public release ~a week later.
- Content lives in the production DB (entity_versions), edited in the creator. `shared/content/seed` is only a starter
  set for empty/local DBs (CUTOVER 10.15). Creator-only kinds (script) were not in the production seed – the script was
  added with `shared/content/seed/2026-10-03-script.mysql.sql`.
- Production DB: MariaDB 10.11.14, server sql_mode NO_ENGINE_SUBSTITUTION, latin1 server default; every table utf8mb4.
  The API sets a strict session sql_mode (`DB_SQL_MODE`). Codes are case-sensitive (utf8mb4_bin, migration
  2026-10-03-code-case – run on production, verified). Migrations are run by the owner in phpMyAdmin (SQL tab; select
  the database first – `DATABASE()` is `information_schema` otherwise; the DB is `d0487143`).
- The catalog (items, recipes, albums, products, editions, featured, permissions / code types, toons) lives in the
  database since 2026-10-04 (CUTOVER 10.29, agents/CATALOG.md), edited in the admin (`/catalog/<kind>`, history + undo);
  `server/config/*.json` are only the seed. **Production: run `server/migrations/mysql/2026-10-04-catalog.mysql.sql`**
  (not run yet; safe before or after the deploy, check query in CATALOG.md "Status"). Media tooling for both apps:
  `@scutoons/media` (shared/media).
- Local dev: `npm run dev` (SQLite) or `npm run dev:db` (MariaDB in Docker, same settings as production;
  `npm run dev:db test` = PHPUnit on MariaDB). Test user: `npm run dev:test-user` (`--browser <url>` for a sign-in snippet).
- Post-beta work list: CUTOVER.md items 10.10–10.24.

## Working rules learned (owner)
- Commit ONLY on `platform`, always guarded: `[ "$(git branch --show-current)" = platform ] && git commit …`. The owner
  switches to `main` for deploys; unguarded commits landed on `main` three times. Never add Claude attribution lines.
- Deploy steps for the owner: `git checkout main` · `git merge --ff-only platform` · `git push origin main` ·
  `git checkout platform` (one command per block).
- The owner works in the shared browser pane: test in your OWN tab (`tabs_create`), never in theirs.
- Give the owner steps one at a time; don't re-ask answered questions (e.g. "no upper-case certificates exist – ever").
- Agents work on their own branch in a worktree; the coordinator reviews, merges into `platform`, runs the checks
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

## Open owner decisions / loose ends
- Onions: no onion item in the catalog yet (needed before it can respawn).
- Respawn: should the server also read a placement's own interval (today only the item's range is enforced)?
- `CloseButton` (sheets/dialogs) round too?
- Upload the text-free season cover art and switch the cover editor's lettering on (CUTOVER / DESIGN "cover editor").
