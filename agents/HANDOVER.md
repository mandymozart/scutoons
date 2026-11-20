# HANDOVER — the current state (read after RULES.md; limit 60 lines, aim ≤ 40: replace, never append; no rules, no todos)

Refreshed 2026-11-03 (#380), last run 2026-11-20 (evening); the longer 2026-10-12 version with the feature notes is archive/HANDOVER-2026-11-03.md. How agents work: RULES.md.
Open work: GitHub issues + the board "SCUTOONS" (RULES C). Decisions: agents/decisions/<topic>.md (index DECISIONS.md; grep a key). Orchestrator state: HANDOFF.md.

## Where things are
- `main` is the only long-lived branch and is production; pushing it runs `.github/workflows/deploy.yml` (deploys only targets whose live `git_sha` is behind,
  then the pending DB migrations over `POST /api/deploy/migrate`; `npm run deploy:status`, deploy/README.md). Workers use branches in worktrees (RULES B2, D1).
- Production hosts since 2026-10-05: scutoons.com/www (viewer), qr., api., creator., admin., shop., embassy. (under `/web/`); cdn.scutoons.com → bunny.net.
  MariaDB 10.11, tables utf8mb4, strict session sql_mode, codes case-sensitive; `migration_status` shows what is pending.
- Content lives in the production DB (`entity_versions`, language-neutral documents, texts as locale maps), edited in the creator; the catalog (items, recipes, albums,
  products, editions, code types, toons, instruments / presets / rigs) lives in the DB, edited in the admin / creator (CATALOG.md, DATA.md). Tests use `server/tests/fixtures/catalog/`.
- Local dev: MariaDB in Docker with the production seed (`npm run dev:db`), test user `agent@scutoons.test` (RULES F).
- AI = the member's own Claude through the SCUTOONS MCP (MCP.md); developers' agents run migrations over it. Embassy (EMBASSY.md) and the newsletter (NEWSLETTER.md) are live.
- `deck/` (Rust workspace; research/CYBERDECKS.md): relay, bridge, adapters, deck-core / engine, `deck/os` (live ISO, `npm run deck:iso`, `deck:vm`). The engine plan is
  research/ENGINE-REFACTOR-PLAN.md (phases P…; P8 show clock built, the bridge audit #357 done, the guards #363 in progress, then the fix tickets #364–#378, then P9 on).
- The hosted relay `wss://live.scutoons.com`: DECIDED 2026-11-03 to run on bunny Magic Containers (relay + compile service, #349, [bunny]); whether it is up is not
  recorded here – check the DNS before relying on it. Until then every Live test runs against the local relay: `cargo run --release -p relay -- 127.0.0.1:7070` in
  deck/, the deck at `/live/deck?relay=ws://localhost:7070` (or `VITE_LIVE_URL` in viewer/.env.development).
- Dates: Chapter 3 "Feilchen" public Sun 2026-11-15 (art freeze 11-10; the deploy freeze of 11-12 lifted by the owner on 11-11; plan #64); Live performance mode restarts 2026-11-16; Top Toy workshop 2027-02-17
  (code complete 02-05, freeze 02-12); WEBTOON.com launch 2027-02-28; deck release that counts 2027-02-05. A side task that doesn't help these goes to an issue (workspace CLAUDE.md "Scope creep"). Owner 2026-11-20: the web tools overhaul (epic #626) comes **before** this February work. Short-term goal: **posters printed in December 2026, rolled out in January 2027** (XR type Poster, #674). Overhaul state (night 2026-11-20/21, develop, not pushed): tokens #655, kit #656 (floating panels + universal tabs), shell #664 (deck-style bar, item = workspace, Home, menu + Settings), desktop-only #659 merged; plan and order in research/redesign/PLAN.md §8; next: the Library draft (#653), then the area tickets.
- Owner-requested checkouts (`git worktree list`): drafts and parked branches (newsletter, onion-camera `C:\Development\scutoons-onion-camera`, storyboard, …) and the app's session
  worktrees (`.claude/worktrees/*`) – the owner reviews and merges them; agents don't (RULES B3).

## Last run
2026-11-20 shift (orchestrator, day into night) · branch `develop`, **292 commits ahead of `main`, nothing pushed**; deck:ci green after every merge, `npm run check` exit 0 end to end, the touched server suites green.
- Merged (on top of the 2026-11-19/20 list): #650 (wav / mp3 / ogg) #676 #642 #677 #678 #327 (step 3) #679 #686 #685 #684 #675 #699 #697 #705 #683 #662 #687 (steps 1–3).
- **History (#687):** deck-core `history.rs`; server `/api/entities/<key>/revert|revert-to` and `/api/scenes|shows/<id>/revert|revert-to`; the creator, the deck editor, admin's Calendar and the MCP (`revert_change` needs a local core, `revert_to` anywhere). No migration. Admin's deploy target now sets up the wasm.
- **Production after the next deploy:** pending migrations run by deploy.yml: `2026-11-19-device-presets`, `2026-11-19-analog-sections`, `2026-11-20-asset-edits`, `2026-11-20-rig-presets`. The deploy needs the owner's go. New MCP tools (shows, history) reach a session after an MCP restart.
- The **sandbox toon** is in production (draft): four demo shows, Sandgirl, Sandy, dunes scenes, a chapter (agents/SANDBOX.md); Play lists the published sandbox shows on every deck (#686).
- Open from the merges: #687 – modules and a rig's side documents in the History, `/api/shows/<id>/versions` wraps its answer twice; #697 – unit fields for the DOP point time, m, tempo; #662 – the editor's handheld and playback smoothing stay outside the core; deck-native `no_alloc` fails on this PC (deck:ci passes).
- Waiting on the owner: the deploy; #688 (one conductor + standby vs co-conductors); moods → rig presets; who marks rig presets factory; #644's final call; #651's questions; #653 / #631 concepts; #633 cue channels; #682 the 8th Wall boundary; #674 posters (queue or XR session); the board's "In progress" triage; the unmerged #505 / #515 worktrees.
