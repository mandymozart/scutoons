# HANDOVER — the current state (read after RULES.md; limit 60 lines, aim ≤ 40: replace, never append; no rules, no todos)

Refreshed 2026-11-03 (#380), last run 2026-11-20; the longer 2026-10-12 version with the feature notes is archive/HANDOVER-2026-11-03.md. How agents work: RULES.md.
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
  (code complete 02-05, freeze 02-12); WEBTOON.com launch 2027-02-28; deck release that counts 2027-02-05. A side task that doesn't help these goes to an issue (workspace CLAUDE.md "Scope creep"). Owner 2026-11-20: the web tools overhaul (epic #626) comes **before** this February work. Short-term goal: **posters printed in December 2026, rolled out in January 2027** (XR type Poster, #674).
- Owner-requested checkouts (`git worktree list`): drafts and parked branches (newsletter, onion-camera `C:\Development\scutoons-onion-camera`, storyboard, …) and the app's session
  worktrees (`.claude/worktrees/*`) – the owner reviews and merges them; agents don't (RULES B3).

## Last run
2026-11-19/20 night shift (orchestrator) · branch `develop`, **215 commits ahead of `main`, nothing pushed**; develop's deck:ci green after every merge (the last runs: #663 329 s, #651 running at writing).
- Merged: #329 #591 #617 #605 (steps 1–5) #627 #628 #629 #630 #632 #633 #634 #637 #638 #639 #640 #641 #643 #646 #647 #648 #649 #650 (flac / mp4) #651 #652 #663 #327 (steps 1, 2, 4).
- **Production after the next deploy:** the owner applies three migrations via the MCP (plan → apply): `2026-11-19-device-presets` (#633), `2026-11-19-analog-sections` (#649, rows 1 · 1), `2026-11-20-asset-edits` (#651, three nullable columns). #650's commit says `Fixes #650` – reopen it until wav / mp3 / ogg land.
- The **sandbox toon** exists in production (draft) with the four demo shows, Sandgirl, dunes scenes and a chapter (agents/SANDBOX.md); the MCP has show tools (`list_shows`, `read_show`, `save_show_draft`) – a session must start after commit 298279fb to see them.
- Running at writing: #676 (Edit / Play, provisional per #644) and #650's wav / mp3 / ogg (crates approved). Queue: #642 (deck start) → #677 (rig presets) → #327 step 3 (VA-API, crate approved) → #678 (time maps in the asset pipeline) → #679 (units in the UI kits) → if time #675, #662, a ticket for the live_quic / ring flakes.
- Waiting on the owner: #644's final call after testing; #653 (shared Library, a concept); #631 (panels and tabs, OSCT #318's rules, no cross-project sharing); the RME cue channels 7/8 vs 3/4 (#633); #651's questions (edits checked by the core on save? pitch on speed?); the 8th Wall boundary (#682).
