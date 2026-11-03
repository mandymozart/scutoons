# HANDOVER — the current state (read after RULES.md; limit 60 lines, aim ≤ 40: replace, never append; no rules, no todos)

Refreshed 2026-11-03 (#380); the longer 2026-10-12 version with the feature notes is archive/HANDOVER-2026-11-03.md. How agents work: RULES.md.
Open work: GitHub issues + the board "SCUTOONS" (RULES C). Decisions: DECISIONS.md (grep a key). Orchestrator state: HANDOFF.md.

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
- Dates: Chapter 3 "Feilchen" public Sun 2026-11-15 (art freeze 11-10, deploy freeze 11-12; plan #64); Live performance mode restarts 2026-11-16; Top Toy workshop 2027-02-17
  (code complete 02-05, freeze 02-12); WEBTOON.com launch 2027-02-28; deck release that counts 2027-02-05. A side task that doesn't help these goes to an issue (workspace CLAUDE.md "Scope creep").
- Owner-requested checkouts (`git worktree list`): drafts and parked branches (newsletter, onion-camera `C:\Development\scutoons-onion-camera`, storyboard, …) and the app's session
  worktrees (`.claude/worktrees/*`) – the owner reviews and merges them; agents don't (RULES B3).

## Last run
The last summary written here is 2026-11-01 (deck day, X1 remote updates, #237–#264) – it moved to archive/HANDOVER-2026-11-03.md and to HANDOFF.md "State". The 2026-11-02/03
engine work (P4b #318, P5 #319, P6 #320, P8 #321, #356, #357) lives in the issues and in the side notes of the orchestrator; the next night summary replaces this section.
