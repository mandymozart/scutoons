# Handoff – where the SCUTOONS orchestrator left off

A fresh orchestrator session starts here (`C:\Development\ORCHESTRATION.md` §1, "Fresh sessions"). The previous one
updates this file before it ends; it is the only state that carries over. Long sessions are expensive (the deck
orchestrator ≈ $445 list over 2026-10-29 … 11-01, `agents/costs.json`), so sessions are short and this file is complete.

## Starting a session (in this order)
1. Read `CLAUDE.md`, `agents/RULES.md`, `agents/HANDOVER.md`, the auto-memory index, `C:\Development\ORCHESTRATION.md`,
   `agents/ORCHESTRATION.md`, then this file.
2. `git status` (other sessions' uncommitted work is not yours), `git log --oneline -8`, `git log origin/main..main`,
   `npm run backlog`, `gh pr list`.
3. Tell the OSCT orchestrator you took over (send by name: "Orchestration", or ListAgents).
4. Continue with "Next". Before ending: update this file, commit it, add your own cost entries to `agents/costs.json`.

## Rules that bite (details in RULES / DECISIONS / memory)
- **Pushes:** only on the owner's go, bundled. A push to `main` deploys the viewer. The pre-push hook
  (`npm run hooks`) runs `npm run deck:ci -- --no-release` when `deck/` or `shared/live*` changed.
- **GitHub's deck CI runs by hand only** (`deck.yml` workflow_dispatch). Cloud PRs get no GitHub deck CI, so run
  `npm run deck:ci` locally before merging them.
- **New work as cloud sessions** where possible. The owner allows local sessions with Opus 5.5 medium. Cloud sessions
  start only from a message the owner typed in the desktop app, and from GitHub's `main`, so push first.
- **Phones test over the LAN, not by pushing:** `npm run dev:lan` serves https://<LAN-IP>:4100 with the API and the
  local relay proxied. The firewall rule for 4100 is set.
- **Night:** never push; one git operator on `main`; no Docker / MariaDB / phone without a slot from the OSCT
  orchestrator. SCUTOONS ports: 7000–7099, 4100–4199, 9500–9599.
- **Costs:** at every merge, add an entry to `agents/costs.json` and a cost line to the closing comment
  (ORCHESTRATION §10). Local sessions: `python C:\Development\tools\costs\transcript_cost.py <transcript.jsonl>`.
  Transcripts are never deleted.
- **No Claude attribution** in commits or PRs.

## Tools outside the repository
- `C:\Development\tools\costs\transcript_cost.py`: the cost of a local session.
- `.claude/launch.json` (gitignored): `viewer-lan`, `viewer` (8000, local API), `api`, `cdn`. After
  `npm run dev:launch-json`, add `viewer-lan` again (`node scripts/dev/lan.mjs`, port 4100).

## State (2026-11-01, ~01:00)
- `main` pushed at 809d3ad2. This includes:
  - the relay rework (#223)
  - the sticks (#225)
  - the zoom glide (#224 part 1)
  - #208 audio
  - the local CI, the LAN mode
  - the REALTIME-STATE research
- Live at /live/deck: #218 post stack, #219 controls UI, #208 lead.
- **Running tonight:**
  - **#220** quality governor: local session "q220-governor" (`local_02e5ae3e-3349-43ef-8b9d-d16c78a2206d`, Opus medium), committing on `main`, no push. Its first commit, 55df2990 (`quality.rs`, `deck.quality` in the protocol), is in; it was still working at 01:00.
  - **#224** navigation around the character: cloud, branch `cloud/224-navigation` → PR.
  - **#211** MIDI / DMX out, the clock-synced strobe: cloud, branch `cloud/211-outputs` → PR.
- Cloud tasks: #224 = `task_4df3a1b7`, #211 = `task_a6fa1dd7`. They send no note when they end, so check their PRs.
- The previous orchestrator session ("Arch Linux cyberdecks audio/video suite", `ffc3b156-…`) was archived on the
  owner's word (its context was too big). Its transcript stays; never delete transcripts.
- **Board:**
  - Deployed: #204, #209
  - Done: #222
  - In progress: #220, #224, #211
  - Backlog: #208 (X1 / ASIO / browser checks open), #223 (netem run open), #225 (stick assignment list), #213
  - Proposals: #221 several conductors, #216 (local runner partly done)

## Next
1. **Morning review (Opus):**
   - #220: the local commits.
   - #224 and #211: the cloud PRs. For each, `npm run deck:ci` and the viewer check, then merge with `--no-ff` locally.
   - Add the cost entries.
   - Screenshots of `/live/deck` (run `npm run dev:lan` or the browser preview on 41xx).
   - Comment on each issue with a cost line, then set the board.
2. **#213** camera moves (and the conductor's shots, #189): only after #224, because both touch `camera.rs`.
3. **#223:** a real `tc netem` run on Linux (X1 or WSL with a slot). If the tail stays, plan the WebTransport
   datagram side channel.
4. **#225:** the list of which parameters fit which stick axis; hue and glow come off the sticks. The rule for which
   stick a selection goes to.
5. Later queue:
   - #203 patch editor
   - #210 scene objects
   - #212 input sources
   - #205 audience
   - #206 security
   - #207 cloth
   - #215 shadows
   - #191 instruments (PR #217 waits for the owner)

## Waiting for the owner
- **PR #217** (instruments / presets design, #191): its 8 open questions; merge only with the owner.
- **#208:** should the glitch warning also flash with the HUD off?
- **Owner steps:** bunny zone `lan.scutoons.com` + script + NS in KAS + BUNNY_API_KEY + run lan-cert workflow →
  pair the X1. The X1 partition install (#200). Clean up the stray CDN copies (Admin → Assets → Unused).
- **#216:** the self-hosted Linux runner, yes or no; a look at the Deploy minutes.
