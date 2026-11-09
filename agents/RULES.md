# RULES — the law book (read whole; hard limit 150 lines)

Numbered rules, ONE or two lines each; `(date [key])` is the owner's decision: `grep -n '\[key\]' agents/decisions/law.md` gives the owner's
words. A rule changes in the same commit as its DECISIONS line; `[rules-only]` = no recorded decision yet. Map of all docs: agents/README.md.
Old section names in code comments (until 2026-11-03): "Backlog" = C · "Git, sessions, checkout" = B + D · "Dev setup and checks" = F · "Code" = A + J ·
"Product rules" = I · "UI rules" = H · "Working with the owner" = G · "Migrations and stored data" = E · "Docs" = K.

## A · The one core (deck, live, engine) — the dogma agents keep working around
- **A1** Deck logic is Rust in the ONE universal core (deck-core + its crates, native + wasm). A platform is only the compile target
  + thin adapters that move bytes / events between the core and a platform service. A decision (what plays, when, how a message
  is read) in TS or in a platform crate is a defect. (2026-11-02 [one-core]: "deck-core is everything … really need to stick to one universal core.")
- **A2** The browser is UI: no business logic. Editor Svelte / TS may hold authoring logic that exists before compile (state, drafts,
  selection, hints); rendering, sound, playback and every runtime rule are the core's; validity = core or server, an editor check is a hint.
  (2026-11-02 [browser-is-ui]: "browser, UI, no business logic.")
- **A3** Modules, rigs, scenes: Rust and universal, one implementation (play, flatten, checks) on every deck; no JS / TS modules, no second
  validator or interpreter per platform. The TS module host is deleted, not ported. (2026-11-02 [rigs-in-core], 2026-11-03 [ts-module-host])
- **A4** One engine, one clock, one governor: audio graph, tempo clock, quality governor, scene playback, wire codecs exist once in the
  core; native ≡ wasm is the contract (golden-render conformance). Scenes are API content, never TOML / TS constants. (2026-11-02 [one-engine] [tempo-core] [scenes-from-api])
- **A5** Devices (MIDI, camera, position, motion, video decode) are adapters feeding the core; a module never calls a browser API; video
  plays on the core's own playback. What a browser lacks (Web MIDI in Safari …) is ignored — that device gets a native build, no TS fallback. (2026-11-02 [devices-via-core] [own-video])
- **A6** No second copy: scenes, rules, defaults, ranges, status texts and monitoring numbers live once, in the core or one data file. A TS twin
  of Rust logic is a defect, even "trimmed", "for now" or "kept in TS because". Every control and readout has ONE owner of its state and number: the core (#362).
  (2026-11-03 [no-duplicate]: "no duplicate code base in the end.")
- **A7** Enforced, not trusted (2026-11-03 [core-only]: "enforce it even for dumb agents"; #363): `check:core-only` (every file that touches the engine; a new bridge
  file needs an `adapter:` / `ui:` line in scripts/bridge-files.json), `check:deck-deps` (no platform library in a core crate; main.rs only shrinks), the headless
  deck host (deck/headless: a move ticket is done when its `expected_fail` scenario is green). Baselines, budgets, allow rules and escapes only shrink – growth
  needs `Owner-approved-baseline:` in a commit message. Every deck brief carries the acceptance line; every merge runs the diff check; the audit reruns (AUDIT.md).
- **A8** Rust first for everything native (relay, bridge, audio, video, 3D, decks) — no Node / Electron — and as much as possible in the engine,
  also for the browser. (2026-10-29 [rust-first], 2026-10-31 [rust-engine])
- **A9** No backwards compatibility for the audio engine (nothing released; no A/B flags, no dual hosts). A deck takes a new module only by restart. (2026-11-02 [no-backcompat-engine], 2026-11-03 [module-restart])
- **A10** Performance and stability first: adapt to link and device (latest value wins, never a backlog), size by benches; every input and mapping is audiovisual. (2026-10-31 [perf-first] [audiovisual])
- **A11** Words: the native app is the **deck** (the stage is a part of it; the per-venue layer is the **stage environment**); "main" / "original" /
  "leader–follower", never master / slave; a bus that sums children is a folder bus. (2026-11-02 [names])
- **A12** Readers save nothing; the sound designer lives in the creator (creators / admins), the deck page holds only performance + hardware settings. (2026-11-02 [readers-save-nothing] [designer-in-creator])
- **A13** `npm run deck:ci` runs locally before every merge / push of deck work (GitHub's deck CI is by hand); phones test over the LAN (`npm run dev:lan`). (2026-10-31 [deck-ci-local])
- **A14** Naming addresses: the part before `:` says where it comes from (`sound:`, `clock:`, `being:`, a track); places (`deck:`, `scene:`, `character:`, `item:`) and sources never share a word; the part after says what it is, a whole lowercase word, the same everywhere; the address is the identity, what a person reads is a label; every reserved word is listed once in the one parser (rig/address.rs) with a reason. (2026-11-09 [names])

## B · Sessions, orchestration, models (workspace rules: C:\Development\agents\ORCHESTRATION.md)
- **B1** At most TWO local sessions at a time across all projects; workers one after another; no cloud sessions. (2026-11-02 [no-cloud]: "no cloud sessions anymore please we have credits")
- **B2** A worker: one issue, its own worktree, branch `<kind>/<issue>-<short-title>` (kind feature · fix · chore · wip; never a generated name), a brief by
  the template, no merge, no push, no attribution line. (2026-11-01 [branch-names], 2026-11-02 [no-cloud])
- **B3** Only the orchestrator merges a worker branch, into local `main`, after the checks and the diff check (A7), and only on the owner's word ("merge"; for a
  night shift the owner's hand-over is that word: "just merge to main, but do not push"). Owner-requested drafts are never merged by agents. (2026-10-06 [night-runs], 2026-10-12 [worktree-requests]; "merge" wording [rules-only])
- **B4** Never push — a push to `main` deploys; the owner pushes, or says "push" (then one push per finished step is fine, Actions minutes don't matter). (2026-10-06 [night-runs], 2026-11-01 [public-repo])
- **B5** Model per task: the cheapest that passes (`C:\Development\agents\models.json`); Opus for engine fundamentals and design; the model of a started session is set with Complexity M and L = Opus 5.5 high from the start (owner 2026-11-04 [models]); workers go through the Agent tool at night (ORCHESTRATION "Starting a worker").
  `set_session_model` (ORCHESTRATION "Starting a worker"). (2026-11-01 workspace §11 [models]; Opus rule [rules-only])
- **B6** Record every owner statement the same day: one line with date, key, the owner's words in its `agents/decisions/<topic>.md` (index: DECISIONS.md); relay it to the issue and the next brief. Grep the area's decisions file before writing an issue or a brief. (2026-10-12 [docs])
- **B7** One git operator per checkout; commit only your own paths (`git commit -m … -- <paths>`), never `add -A`, never reset / stash / reformat another session's files, never a bare `git stash`. (2026-10-06 [night-runs])
- **B8** Unattended runs: small commits, dev DB free to change, stop at owner decisions instead of guessing; the result replaces HANDOVER "Last run" and goes into each issue. (2026-10-06 [night-runs])
- **B9** Every session ends with the usage report (`python C:\Development\agents\costs\report_usage.py --issues N`) and a comment on its issue. (workspace CLAUDE.md, 2026-11-01 [usage])

## C · Backlog — GitHub issues (every session except hotfixes; 2026-10-19 [issues-only])
- **C1** All open work is an issue in `mandymozart/scutoons`; bugs, features, owner steps, open decisions. Never a backlog in docs, memory or HANDOVER. Old numbers 10.N: `gh issue list -s all --search '"10.N" in:body'`.
- **C2** Board **SCUTOONS**, six statuses (2026-10-20 [board]): Backlog (order top down = the owner's priority) · Proposals (new issues, open `decide`) · In progress · Review on staging (built + committed) · Deployed (closed by the push) · Done (the owner).
  Move your issue on start and finish — `npm run backlog -- set N "In progress"` — and put the step in every brief (2026-10-21 [board-move]: "important move issues alog the board once finihsed").
- **C3** Start: the owner names `#N`, or the request becomes an issue before the first edit (search first, then `gh issue create`); read `gh issue view N --comments`. Unattended: `npm run backlog`, top down.
- **C4** Anything outside the issue becomes a new issue (no status = the owner's inbox), unless the owner says so (CLAUDE.md one level up: ask before scope creep).
- **C5** Commits end `Refs #N`, the finishing one `Fixes #N` (never `#N` at a line start). End of session: one issue comment (built, open, checks), Finished → "Review on staging"; an open rest → the body's **Open:** list, status back to Backlog.
- **C6** Priorities are the owner's: agents move only their own issue (In progress, Review on staging, back); Backlog / reorder / Deployed / Done only on the owner's word. `decide` = assigned to the owner with options + your recommendation; `owner` = the owner's step (secrets, publishing, art).
- **C7** Issue form: title = the thing; one type label (`bug` `feature` `chore`), one `area:*`, one `complexity:S/M/L`, + `owner` / `decide`; body ≤ ~15 lines (owner quote · what / why · **Open:** · **Docs:**); a milestone only for a dated target. **Hotfix** (production broken, small): `hotfix:` commit, no issue.

## D · Git
- **D1** `main` = production. Workers use their own worktree (B2); the shared checkout `C:\Development\scutoons` is not switched by a worker. Own `npm install` per worktree (GOTCHAS). Owner-requested worktree drafts get their own DB `SCUTOONS_DB_NAME=scutoons_<x>`. (2026-11-02 [no-cloud], 2026-10-06 [worktree-requests])
- **D2** The repo is public; Actions minutes are free (2026-11-01 [public-repo]); per-app deploy jobs stay, docs-only pushes start no deploy (2026-10-20 [process]).

## E · Data, migrations
- **E1** The owner never runs migrations by hand: a file in `server/migrations/mysql/` runs with the deploy; check / dry-run / apply over the MCP (`migration_status`, `plan_migrations`, `apply_migrations`). Never ask for phpMyAdmin. (2026-10-12 [migrations])
- **E2** Idempotent, columns compared with literals (collation #1267), dry-run on the dev DB first. Format change = analyse the data → SQL migration (old form leaves the code, no aliases) → only if impossible a code helper WITH creator feedback; never a silent load-time conversion (localStorage excepted). (2026-10-06 [migrations])
- **E3** Production is the only source of content and catalog (DB, edited in creator / admin); no file seeds; local = `server/db/prod-seed.sql` via `npm run dev:db`; local `media/` is a dev copy. (2026-10-05 [mariadb], 2026-10-04 [local-media])
- **E4** Content is served by the API from the DB (one request per chapter); JSON only as import / export. New items / recipes are data (packs); fundamentals change with the code. (2026-09-30 [api-content], 2026-10-02 [packs])

## F · Dev setup and checks
- **F1** MariaDB in Docker only, no SQLite (`npm run dev:db`, PHPUnit `npm run dev:db test`); one dev DB `scutoons`; don't reset the owner's DB. (2026-10-05 [mariadb])
- **F2** Fixed ports (`shared/dev-ports.json`, `check:ports`); servers only via `npm run dev`; never kill a server you didn't start. (2026-09-30 [dev-ports])
- **F3** Debug as `agent@scutoons.test` (`npm run dev:test-user`), never as the owner, never in the owner's browser tab; never paste tokens. (2026-09-30 [test-user])
- **F4** Done = green: `npm run check` (0 errors), `check:tools-css`, PHPUnit + PHPStan, builds of touched apps, the area's `check:*`, `check:core-only` + `deck:ci` for deck work. Breakage you didn't cause: report, don't skip. (2026-09-30 [done-green])
- **F5** You may restart your OWN session's SCUTOONS MCP after a tool change (how: GOTCHAS). (2026-10-14 [mcp-restart]: "i allow you to do it … give all agents that rule")
- **F6** Tool traps (Windows, heredocs, paraglide, vite preview …): GOTCHAS.md.

## G · Working with the owner
- **G1** Steps one at a time; never re-ask an answered question. Bigger UI: drafts on the real page (phone + desktop) first, small owner-described fixes may be built. (2026-10-06 [drafts-first])
- **G2** AI = the member's own Claude through the SCUTOONS MCP, no in-app AI; the AI saves drafts + a link, the member publishes / sends / applies in the browser (`save_draft` is always allowed). (2026-10-09 [mcp-byo-ai] [mcp-drafts], 2026-10-21 [mcp-draft-ok])
- **G3** Models: used without the big AI companies' license restrictions; note the license where you use one, don't block on it. Focus: small creators. (2026-10-31 [ai-license]: "Meta is stealing from everyone …")

## H · UI (canonical text: DESIGN.md, DOCKING.md — read the part you build)
- **H1** Less UI is better UI; no duplicate controls (the creator header's text-language switch is the only one). [rules-only]
- **H2** Sizes in rem, never px — also sizes set from JS (÷ root font size); px only in tokens (border widths) and breakpoints. (2026-10-04 [rem]: "a dogma across the app")
- **H3** One `--control-height`; one tooltip style via `title`; bold very rarely; fieldsets without frames. (2026-10-02 [control-height] [bold])
- **H4** Standalone icon-only button = round (`sc-round`); grouped controls, tabs and anything with text = square; fold buttons quiet (`sc-quiet`). (2026-10-07 [round-square], 2026-10-20 [fold])
- **H5** Spacing on a 1rem line (sidebars, outline, fold button start at 1rem); dividers run edge to edge. One layout model (DOCKING.md): regions tile, one `Divider` per drag edge, one fold to a `Rail`, panels in the right dock (`useDock`); pickers are modal drawers (`Sheet`). [rules-only]
- **H6** Confirmations in-page (`confirmDialog` / `askDialog`), never `confirm()`; primary action last, one only. Media fields are slots (preview = button, empty = dashed slot; multi-resolution = one slot with chips). (2026-10-03 [confirm])
- **H7** Empty list shows nothing; a muted `.sc-empty` line only when the area would be blank and it says what to do. Warnings can be accepted (✓, stored in the doc); errors can't. (2026-10-06 [empty-states])

## I · Product
- **I1** Printed / public URLs never change (QR `?c=`, `/verify.html`, `/<toon>`, `/<toon>/chapter/<n>`, `?code=`, locale prefixes); new may be added; a change only by server-side mapping + tests + rollback (PLATFORM "Printed URLs"). Codes are case-sensitive. (2026-09-30 [urls], 2026-10-03 [codes])
- **I2** Everything is language-independent: media shared by all languages, text only in text fields (locale maps), lettering is text. Every chapter belongs to a season (its `sequence` is the single source of visibility). Keep the configuration small. (2026-10-01 [language-neutral], 2026-09-30 [config-small])
- **I3** Media quality is a reader setting (low / medium / high / Automatic), one file per level, never by viewport (no `srcset`); a changed file gets a new name (immutable CDN paths). Slice 390 × 624 (its 3x 1170 × 1872 = WEBTOON's 800 × 1280),
  1x WebP + `.2x` / `.3x`, every slice carries width / height; new art from a lossless 2340-wide master; WEBTOON's 800 × 1280 is an export, never a stored level (ASSETS.md). (2026-10-01 [media-quality], 2026-10-20 [slice-624])
- **I4** "Carrier bag" (German Tragetasche), never "inventory". Story texts are the **Kanon**, never "bible". (2026-10-30 [carrier-bag], 2026-10-20 [kanon])
- **I5** Every character at some point sings, makes music — or lives out stories (SCUTOONS came out of a neurodiversity where hearing music suggests stories and pictures). (2026-10-31 [music])

## J · Code (apps, shared packages, tools)
- **J1** New code is TypeScript with unit tests for its logic (apps, shared, scripts); a CLI only a thin entry; no new plain-JS / .mjs layers. On the deck side TS is UI + adapters only (A1). (2026-10-20 [ts-tests]: "typescript and unit tests will help us save a lot of time")
- **J2** Shared: apps → shared, never the reverse; `tokens ← ui ← reader`; `content` without Svelte; moving code into shared is a move, not a copy (PLATFORM "dogmas").
- **J3** One component = one job, ≤ 300 lines (> 400 needs a reason in its header); logic in `lib/editor/*.ts`; no local copies of tools.css vocabulary; imports `$lib/…`; suffixes `-editor -panel -inspector -field -picker -slot -canvas`. [rules-only]
- **J4** Comments may cite docs / issues / old item numbers; a doc keeps its name wherever it lives under agents/.

## K · Docs (2026-10-12 [docs]: "nicht zu sehr ausrasten lassen")
- **K1** Caps (owner 2026-10-12 [docs]): RULES 150 · HANDOVER 60, aim ≤ 40 (replace, never append) · GOTCHAS 80; new 2026-11-03 (#380, to confirm): AUDIT 80 · ORCHESTRATION 100 · topic docs "how it IS now", trim over ~500 lines · research/ line 1 = status · archive/ frozen. Map: README.md.
- **K2** A decision = one DECISIONS line (+ a rule here if it changes how agents work, + the topic doc). Stale text you notice: fix it where it stands (or an issue), never append a correction. No new top-level file in agents/ without the owner.
