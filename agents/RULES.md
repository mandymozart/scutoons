# RULES — how agents work on SCUTOONS (read this whole; hard limit 150 lines)

The owner's standing rules, in one place (owner 2026-10-12, agents/archive/DOCS-REFACTOR.md). Each rule's story and quote:
agents/DECISIONS.md. A change to how agents work changes this file in the same commit as its DECISIONS line.

## Git, sessions, checkout
- **One checkout** `C:\Development\scutoons`, **one branch `main`** (= production), **one dev DB `scutoons`**. No
  `git worktree`, no per-agent DBs, no `isolation: worktree` – only when the owner asks for one by name ("in its own
  worktree"); then its own DB `SCUTOONS_DB_NAME=scutoons_<x>`, and the owner merges it.
- Commit guarded and only your own paths: `[ "$(git branch --show-current)" = main ] && git commit -m … -- <paths>`.
  Other sessions work in the same checkout: never `git add -A`, never commit, reset, stash or reformat their files.
  Bare `git stash` / `stash pop` never.
- **Never push** – a push to `main` deploys. The owner pushes, or asks you to. Never a Claude attribution line.
- Agents run one after another (or at most two lanes on separate files); only one session runs git operations at a time.
- Unattended (night / day) runs: commit on `main` in small steps, never push, the dev DB may be changed freely, stop at
  owner decisions instead of guessing; the result goes into HANDOVER "Last run" (replacing the previous one).

## Migrations and stored data
- **The owner never runs migrations by hand** (2026-10-12). A file in `server/migrations/mysql/` runs on production
  with the deploy (`POST /api/deploy/migrate`); check / dry-run / apply over the SCUTOONS MCP with the developer role:
  `migration_status`, `plan_migrations` (row counts), `apply_migrations`. Never ask for phpMyAdmin.
  Details: server/docs/DATABASE.md "Migrations", agents/MCP.md §8.
- A migration is **idempotent** (a second run changes nothing), compares columns with literals (no user variables –
  collation #1267), and is dry-run on the dev DB before the commit.
- **Stored format changes:** 1. analyse the existing data (dev DB = the latest production dump); 2. a SQL migration,
  and the old form leaves the code – no aliases; 3. only if a migration can't work (backwards compatibility): a code
  helper WITH creator feedback ("action required" error or a warning) – never a silent load-time conversion.
  Exception: data in readers' browsers (localStorage) – a one-time helper on load.
- Production is the only source of content and catalog (DB, edited in creator / admin). No file seeds; local data =
  `server/db/prod-seed.sql` via `npm run dev:db`. Local `media/` is a dev copy – don't judge assets by it.

## Dev setup and checks
- **MariaDB in Docker only** (no SQLite): `npm run dev:db` (container, migrate, prod seed, sandbox, API), PHPUnit with
  `npm run dev:db test`. Don't reset or reseed the owner's DB without asking. Missing media: `npm run dev:db -- media --apply`.
- **Fixed ports** (`shared/dev-ports.json`, `npm run check:ports`); start servers only with `npm run dev`
  (idempotent; `npm run dev:status`). Never kill servers you didn't start; throw-away ports only for your own test runs.
- **Debug as the test user** `agent@scutoons.test` (`npm run dev:test-user`, `--browser <url>` for a sign-in snippet),
  never as the owner, never in the owner's browser tab – open your own (`tabs_create`). Never paste tokens anywhere.
- **Done = green:** `npm run check` (0 errors), `check:tools-css`, server PHPUnit + PHPStan, the builds of the apps you
  touched, plus the named `check:*` browser script of the area. Breakage another task caused: report it, don't skip it.
- **Restarting the SCUTOONS MCP is allowed** (owner 2026-10-14: "i allow you to do it … give all agents that rule"):
  after a change in tools/mcp, stop only YOUR session's server (the `scutoons-mcp.mjs` node process whose parent is
  your session's claude.exe, + its child) – it restarts by itself; never another session's. How: agents/GOTCHAS.md.
- Tool traps (Windows, heredocs, paraglide, vite preview …): agents/GOTCHAS.md.

## Working with the owner
- Steps one at a time; never re-ask an answered question; ask before scope creep (CLAUDE.md one level up).
- Show design drafts (on the real page, phone + desktop) before building bigger UI; small owner-described fixes may be built.
- Saving through the MCP never reaches readers: drafts + a link, the member publishes / sends / applies in the browser.
- No in-app AI features: AI = the member's own Claude through the SCUTOONS MCP (agents/MCP.md).

## UI rules (canonical text: DESIGN.md, DOCKING.md – read the part you build)
- **Less UI is better UI.** No duplicate controls: the creator header's text-language switch is the ONLY language switch.
- **Sizes in rem, never px** – also sizes set from JS (divide by the root font size). px only inside the tokens
  (border widths) and the existing breakpoints.
- Every control at `--control-height`; one tooltip style via `title`; bold very rarely; field groups without frames
  (fieldset has no border, legend = the label – tools.css).
- **Round vs square:** a standalone icon-only button is round (`sc-round`); grouped controls (segmented `sc-seg`,
  toolbars, tab rows, every language switcher) and controls with text are square (DESIGN "Round vs square").
- Spacing on a 1rem line: sidebars, outline, fold button start at 1rem; dividers run edge to edge.
- **One layout model** (DOCKING.md): regions tile (sidebar | page | right dock, AppShell), one `Divider` per drag
  edge, one fold to a `Rail`, page panels side by side in the right dock (`useDock`), maximize folds the rest; the
  main folds only by hand. Pickers stay modal drawers (`Sheet`).
- Confirmations in-page (`confirmDialog` / `askDialog`), never `confirm()`; the primary action last, one primary only.
- Media fields are slots (preview = button, empty = dashed slot); multi-resolution media = one slot with chips.
- **Empty states:** an empty list shows nothing (its add button is enough); a muted one-liner (`.sc-empty`) only when
  the whole area would be blank, the reason isn't obvious, and it says what to do (DESIGN "Empty states").
- Warnings can be accepted (✓ "Looks fine", stored in the document – DATA.md §5); errors can't.

## Product rules
- **Printed / public URLs never change** (QR `?c=`, `/verify.html`, `/<toon>`, `/<toon>/chapter/<n>`, `?code=`, locale
  prefixes …): new URLs may be added; a change only via server-side code mappings with tests and a rollback. The full
  table: PLATFORM.md "Printed URLs". Codes are case-sensitive; certificates print them upper case.
- **Everything is language-independent:** media are shared by all languages, text lives only in text fields (locale
  maps); lettering is text, rendered by the UI (DATA.md "Language-independent media").
- Every chapter belongs to a season (the season's `sequence` is the single source of visibility).
- **Keep the configuration small:** model only what a screen edits or code reads.
- Media quality is a reader setting (low / medium / high, or Automatic by bandwidth) – one file per level, never by
  viewport (no `srcset` / `<source media>`). A changed file gets a new name (immutable CDN paths).
- Default slice 390 × 625, 1x WebP; every slice carries width / height.
- Content is served by the API from the DB (one request per chapter); JSON only as import / export.
- New items and recipes are data (the admin catalog); fundamentals (what items can do) change with the code (ITEMS.md).

## Code
- Shared packages: apps → shared, never the reverse; `tokens ← ui ← reader`; `content` without Svelte; no app state,
  routing or paraglide in shared; moving code into shared is a move, not a copy (PLATFORM.md "dogmas").
- One component = one job; ≤ 300 lines (> 400 needs a reason in its header); logic in `lib/editor/*.ts`; no local
  copies of tools.css vocabulary; imports via `$lib/…`; suffixes `-editor -panel -inspector -field -picker -slot -canvas`.
- Code comments may cite docs and item numbers ("BACKLOG 10.61", "DESIGN.md …"): a doc keeps its name wherever it lives
  under agents/ (grep `agents/` for the name or the number).

## Docs – keep them tight (owner 2026-10-12: "nicht zu sehr ausrasten lassen")
| File | Holds | Limit |
|---|---|---|
| RULES.md | how agents work | 150 lines |
| HANDOVER.md | the current state + "Last run" (replace, never append; no rules, no todos) | 60 lines |
| DECISIONS.md | one line per owner decision: date · decision · "quote" · → doc | newest on top |
| ISSUES.md / FEATURES.md | open bugs / open features, changes, owner steps – nothing done | 40 open items together |
| GOTCHAS.md | tool traps | 80 lines |
| topic docs (DESIGN, DOCKING, DATA, ITEMS, MAP, PLATFORM, ASSETS, CATALOG, EXPRESSIONS, MCP, EMBASSY, NEWSLETTER) | only how it IS now | trim when over ~500 lines |
| research/ | concepts, research, parked plans; line 1 = the status | – |
| archive/ | frozen history – never edited, never read on start | – |

- **Item template** (ISSUES / FEATURES), at most 5 lines; more detail goes into the topic doc or research/:
  `- [ ] 10.N **Title** – owner YYYY-MM-DD: "quote". What / why in 1–3 lines. → DOC.md §x` (+ `[owner]` if only the
  owner can do it). Numbers: the next free 10.N, one line for both files; numbers are never reused.
- **Done = the item leaves the list** in the same commit; the commit message says `10.N done`. No Done section, no
  "built" logs in docs – history is git.
- A decision: one line in DECISIONS (+ RULES if it changes how agents work, + the topic doc's "how it is").
- Stale text you notice: fix it where it stands in the same commit (or a one-line issue) – never append a correction.
- No new top-level file in agents/ without the owner; new concepts go to research/.
