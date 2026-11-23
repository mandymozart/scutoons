# HANDOVER — the current state (read after RULES.md; limit 60 lines, aim ≤ 40: replace, never append; no rules, no todos)

Refreshed 2026-11-03 (#380), last run 2026-11-21 (night); the longer 2026-10-12 version with the feature notes is archive/HANDOVER-2026-11-03.md. How agents work: RULES.md.
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
  (code complete 02-05, freeze 02-12); WEBTOON.com launch 2027-02-28; deck release that counts 2027-02-05. A side task that doesn't help these goes to an issue (workspace CLAUDE.md "Scope creep"). Owner 2026-11-20: the web tools overhaul (epic #626) comes **before** this February work. Short-term goal: **posters printed in December 2026, rolled out in January 2027** (XR type Poster, #674). Overhaul state (2026-11-22, local main, not pushed): one workspace host, docking gone (#751); a tab takes the whole stage, simple split (#765); every slot picks / drops through the Library (#752); Items to open per kind from the core (#770); one Log replacing Problems (#767); autosave, Publish the only state (#760). Drafts for the owner: research/redesign/{stage,log,characters,spores}.html, AI-HOOKS.md (the hooks / spore-canvas concept for the Screen Institute conference ≈ 2026-11-30). Next: #773 build (the character workspace A + the 3D place), #756, #780, #779.
- Owner-requested checkouts (`git worktree list`): drafts and parked branches (newsletter, onion-camera `C:\Development\scutoons-onion-camera`, storyboard, …) and the app's session
  worktrees (`.claude/worktrees/*`) – the owner reviews and merges them; agents don't (RULES B3).

## Last run
2026-11-23/24 night (creator orchestrator, epic #626 – the redesign lap, plan agents/research/redesign/LAP.md, the owner's 64 points + drawn set in research/redesign/lap/). Built one worker after another and merged into local main; the deck orchestrator ("Epic: Scenes") pushed main several times, so production has #820–#828, #831, #832 (+ the dashboard veil fix); #829 goes with the next push.
- Built: #820 panels float, never push/snap · #821 the bar (icons, menu far right, scrolling item tabs, flat workspace row; closes #809 #811) · #831 the deck editor's colours (`--tool-*`, RULES H11) · #832 one UI kit from deck-kit (RULES H12, /guide#ui-kit) · #822 a view takes the stage (four states, four layouts, drop zones to the edges, full screen) · #824 the dashboard is the app switcher (+ #793's rest) · #825 one launcher (Alt Space / Ctrl K) · #826 crafts (migrations 2026-11-24-crafts/-seed-crafts, applied by the deploy and checked) · #827 saved layouts in the DB (migration 2026-11-24-workspace-layouts) · #828 resume like a hibernated computer · #823 fling to close, motion tokens, swipe-up user menu · #829 workspaces generalized (character views, Generate canvas stub, no Wanted pictures).
- Waiting on the owner: #830 (the deck's side – design pictures in research/redesign/lap/crafts-v2.html, then the deck orchestrator builds it) · #837 (team invite signs up without a code?) · #838 (resume across devices) · the active tab's look (deck's pressed fill vs the thin outline of point 46) and standalone icon buttons round (H4) vs the deck's corner · touch checks of #823 on a real device · existing members are asked their crafts once.
- Filed: #835 (pad / two-finger full screen) · #836 (MCP crafts) · #839 (Library wanted filter) · #840 (areas still on their own pages).
