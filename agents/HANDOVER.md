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
2026-11-22 night (creator overhaul orchestrator, #777): #767 (one Log) and #760 (autosave) are on local `main` (45ec336fd), checks green, nothing pushed. The overhaul merges (#751 … #760) went to a `develop` branch by mistake and were merged into `main`; **scutoons has no develop branch – feature branches merge into local main (RULES B3).**
2026-11-20/21 (orchestrator) · deployed 2026-11-20 20:12: main = 14f7b38 on all seven targets, five migrations applied by deploy.yml (0 errors). Since then on `develop`, **81 commits ahead of `main`, nothing pushed**; deck:ci green after every merge, `npm run check` exit 0.
- Merged after the deploy: #708 (a show's `toons`; migration `2026-11-20-show-toons` runs at the next deploy, the four sandbox demo shows among the rows; the MCP's show tools follow) · #698 (the Library's rules in the core – the redesign session's branch, merged here on the owner's go) · #701 steps 1–2 (the deck editor's Library on them) · #687 step 4 (module and rig side-document history, show versions answer once) · #697 step 2 (deck distances and the DOP track time on the unit field; core Distance dimension) · #534 (jump labels in the core, reading group `id` after #706) · #553 (flash filter on the one evaluator) · #585 (play pill BPM and tempo source) · #582 (camera cut end to end over the relay – fixed: a deck's link filtered as group "decks", not its column).
- Research for the owner to read (no build yet): agents/research/outlets-on-models-and-node-view.md (#712) and agents/research/audio-nodes-foundation.md (#713, with the three-crows rave demo); each ends with its open questions.
- A sweep of the open deck issues #515–600 commented evidence on 24 of them (18 done, 6 partly) – the owner closes.
- Open from the merges: #708 – an older native deck refuses the new show documents until it updates; #585 – is the typed BPM saved with the show?; #582 – a role moved by assign moves the deck's column only after a restart; #687 – Back to here doesn't take a rig's side documents back; #701 – upload into a slot, frame links from documents; deck-native `no_alloc` fails on this PC (deck:ci passes).
- Waiting on the owner: the next deploy; #712 / #713 questions; #688; moods → rig presets; #644; #651; #653 / #631; #633; #682; #674; closing the swept issues; #543 rescope; the unmerged #505 / #515 worktrees.
