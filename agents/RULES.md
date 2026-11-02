# RULES — how agents work on SCUTOONS (read this whole; hard limit 150 lines)

The owner's standing rules, in one place (owner 2026-10-12, agents/archive/DOCS-REFACTOR.md). Each rule's story and quote:
agents/DECISIONS.md. A change to how agents work changes this file in the same commit as its DECISIONS line.

## Backlog – GitHub issues (every session, except hotfixes; owner 2026-10-19)
- **All open work is a GitHub issue** in `mandymozart/scutoons` (private): bugs, features, changes, owner steps, open
  decisions. The board **SCUTOONS** (GitHub → Projects) has six statuses (owner 2026-10-20): **Backlog** (wanted; the
  board's order top down = the owner's priority) · **Proposals** (new issues + open `decide` questions; an open issue
  without a status counts here) · **In progress** · **Review on staging** (built and committed – the owner checks it on
  production after the push) · **Deployed** (closed by the push) · **Done** (the owner checked it). Never a backlog in
  docs, memory or HANDOVER. ISSUES.md / FEATURES.md are gone; an old number 10.N / M.N: `gh issue list -s all --search '"10.N" in:body'`.
- **Start:** every session works on an issue. The owner names it (`#N`), or the request becomes one before the first
  edit – search first (`gh issue list --search …`), else `gh issue create` (form below). Read it with
  `gh issue view N --comments`, then `npm run backlog -- set N "In progress"`. Unattended runs: `npm run backlog`,
  Backlog top down.
- **Scope creep** (CLAUDE.md one level up): anything outside the issue becomes a new issue (no status = the owner's
  inbox), not part of this session – unless the owner says so.
- **Commits** name the issue: the message ends with `Refs #N` for a step, `Fixes #N` on the commit that finishes it
  (GitHub closes it when the owner pushes = when it is live). `#N` never starts a line (git drops it as a comment).
- **End of a session:** one comment on the issue (`gh issue comment N`): what was built / decided, what is open, which
  checks are green. Finished (`Fixes #N` committed) → `npm run backlog -- set N "Review on staging"`; an open rest
  goes into the issue body's **Open:** list and the status back to Backlog (or what the owner said). The history lives in the issue and in git, never in docs.
- **Owner decision:** label `decide`, assigned to the owner (agents use the owner's gh login: `--add-assignee @me`),
  the question with the options and your recommendation in the issue; the answer → a DECISIONS line (+ the topic doc),
  the label off. **Owner step** (secrets, publishing, art, uploads): label `owner`, assigned too – the owner's
  "Assigned to me" is their to-do list.
- **Priorities are the owner's:** a new issue gets no status or Proposals; agents move an issue into Backlog, reorder
  Backlog, or set Deployed / Done only when the owner says so, and move only their own issue to In progress, Review on
  staging and back. The push closes a `Fixes #N` issue → Deployed (board workflow "Item closed", #63); Done is the
  owner's.
- **Hotfix** = production is broken and the fix is small: no issue needed, the commit message starts with `hotfix:`;
  whatever it leaves open becomes an issue.
- **Issue form:** title = the thing (no number). Labels: one type (`bug` · `feature` · `chore`), one `area:*`, +
  `owner` / `decide`. Body ≤ ~15 lines: owner YYYY-MM-DD: "quote" · what / why in 1–3 lines · **Open:** ·
  **Owner step:** · **Decide:** · **Docs:** agents/X.md §y – details go into the topic doc or research/. A milestone
  only for a dated target (a chapter release, `v1.0.0`); a release plan's work items are issues in its milestone.

## Git, sessions, checkout
- **One checkout** `C:\Development\scutoons`, **one branch `main`** (= production), **one dev DB `scutoons`**. No
  `git worktree`, no per-agent DBs, no `isolation: worktree` – only when the owner asks for one by name ("in its own
  worktree"); then its own DB `SCUTOONS_DB_NAME=scutoons_<x>`, and the owner merges it.
- **Branch names** (any branch besides `main`: cloud sessions, worktrees, stacked PRs; owner 2026-11-01): `<kind>/<issue>-<short-title>`
  – kind `feature` · `fix` · `chore` · `wip`, e.g. `feature/236-events-threads`. Never a generated name; a session that starts others names their branches so.
- Commit guarded and only your own paths: `[ "$(git branch --show-current)" = main ] && git commit -m … -- <paths>`.
  Other sessions work in the same checkout: never `git add -A`, never commit, reset, stash or reformat their files.
  Bare `git stash` / `stash pop` never.
- **Never push** – a push to `main` deploys. The owner pushes, or asks you to. Never a Claude attribution line.
- **Push when a step is done** (owner 2026-11-01: the repo is public, so Actions minutes cost nothing – the old "push bundled" of issue 73 is void): the git operator may push after each commit; a push to `main` still deploys, so only on the owner's word.
- Agents run one after another (or at most two lanes on separate files); only one session runs git operations at a time.
- Unattended (night / day) runs: commit on `main` in small steps, never push, the dev DB may be changed freely, stop at
  owner decisions instead of guessing; the result goes into HANDOVER "Last run" (replacing the previous one) and, per
  issue, into its comment.

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
  toolbars, tab rows, every language switcher) and controls with text are square; fold / unfold buttons are quiet
  (borderless, muted – `sc-quiet`) (DESIGN "Round vs square").
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
- Default slice 390 × 624 (its 3x 1170 × 1872 = WEBTOON's 800 × 1280), 1x WebP + `.2x` / `.3x`; every slice carries width / height.
  New art from a lossless master 2340 wide (6x); WEBTOON's 800 × 1280 is an export, never a stored level (ASSETS.md).
- Content is served by the API from the DB (one request per chapter); JSON only as import / export.
- New items and recipes are data (the admin catalog); fundamentals (what items can do) change with the code (ITEMS.md).
- **Every character makes music:** SCUTOONS came out of a neurodiversity in which hearing music automatically suggests
  stories and pictures (the owner: about 1 in 5,000 people have it); the collaborators began giving songs the pictures
  and stories their brains associate with them – that is how SCUTOONS came to life. All characters share this trait,
  so **every character at some point sings or makes music – or rather lives out stories** (DECISIONS 2026-10-31).
- **AI models: we use them without the big AI companies' license restrictions** (owner 2026-10-31: "Meta is stealing
  from everyone, so are all big AI companies … the entire system is broken"): a model's non-commercial clause (e.g. TRIBE
  v2's CC BY-NC) is not a reason not to use it in our tool chain. Our focus stays on **small creators** – an ecosystem that
  benefits them through hire. Agents note a model's license where they use it (so the owner sees it), but don't block on
  it; the owner carries that decision.

## Code
- **TypeScript + unit tests** (owner 2026-10-20: "typescript and unit tests will help us save a lot of time during future
  iterations"): new code is TypeScript (Svelte + TS in the apps, TS in the shared packages) and its logic has unit tests.
  Scripts and tools too: logic in tested TS modules, a CLI only as a thin entry; no new plain-JS / .mjs layers (the
  storyboard's move into the creator: #118).
- **Deck logic is Rust in the one universal core** (owner 2026-11-02: "deck-core is everything … only lightweight adapters for other browser services and the crate compiler target"): deck-core and its crates, built for native and wasm. A platform file is only an adapter (it moves bytes / events between the core and a platform service) or the build target – a decision (what plays, when, how a message is read) in TypeScript or in a platform crate is a defect. The browser is UI – no business logic (owner: "browser, UI, no business logic"): the editor's Svelte / TS may hold authoring logic before it is compiled to the core or its modules (state, drafts, hints), but rendering, sound, playback and every runtime rule are the core's, and validity is decided by the core or the server – an editor check is a hint; module extensions are Rust, universal across decks. What a browser lacks (Web MIDI in Safari …) is ignored for now – a device that needs it gets a native build of the core, no TS fallback. The TypeScript rule above is for the apps and tools; on the deck side TS is UI and adapters (research/ENGINE-ALIGNMENT.md §0).
- Shared packages: apps → shared, never the reverse; `tokens ← ui ← reader`; `content` without Svelte; no app state,
  routing or paraglide in shared; moving code into shared is a move, not a copy (PLATFORM.md "dogmas").
- One component = one job; ≤ 300 lines (> 400 needs a reason in its header); logic in `lib/editor/*.ts`; no local
  copies of tools.css vocabulary; imports via `$lib/…`; suffixes `-editor -panel -inspector -field -picker -slot -canvas`.
- Code comments may cite docs, issues and old item numbers ("#44", "BACKLOG 10.61", "DESIGN.md …"): a doc keeps its
  name wherever it lives under agents/ (grep `agents/` for the name or the number).

## Docs – keep them tight (owner 2026-10-12: "nicht zu sehr ausrasten lassen")
| File | Holds | Limit |
|---|---|---|
| RULES.md | how agents work | 150 lines |
| HANDOVER.md | the current state + "Last run" (replace, never append; no rules, no todos) | 60 lines |
| DECISIONS.md | one line per owner decision: date · decision · "quote" · → doc | newest on top |
| GOTCHAS.md | tool traps | 80 lines |
| topic docs (DESIGN, DOCKING, DATA, ITEMS, MAP, PLATFORM, ASSETS, CATALOG, EXPRESSIONS, MCP, EMBASSY, NEWSLETTER) | only how it IS now | trim when over ~500 lines |
| research/ | concepts, research, parked plans; line 1 = the status | – |
| archive/ | frozen history – never edited, never read on start | – |

- Open work and its history are GitHub issues ("Backlog" above); no todo lists, Done sections or "built" logs in docs.
- A decision: one line in DECISIONS (+ RULES if it changes how agents work, + the topic doc's "how it is").
- Stale text you notice: fix it where it stands in the same commit (or an issue) – never append a correction.
- No new top-level file in agents/ without the owner; new concepts go to research/.
