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
- `.claude/launch.json` (gitignored): `viewer-lan`, `viewer` (8000, local API), `api`, `cdn`, and `viewer-prodapi`
  (4101: the viewer over plain http against the production API – the in-app browser refuses the LAN mode's self-signed
  certificate, and `viewer` needs the local API + MariaDB; its script lives in a session scratchpad, so after
  `npm run dev:launch-json` add it again: `node <script>` that spawns `npm run dev -w viewer -- --port 4101` with
  `VITE_API_URL=https://api.scutoons.com/api`, `VITE_CDN_URL=https://cdn.scutoons.com`). Add `viewer-lan` again too.
- The workspace rules moved to `C:\Developmentgents\ORCHESTRATION.md` (2026-11-01, + §11 "Usage": tickets carry
  `complexity:S/M/L`, each with a model tier and a ceiling in turns / list USD, a shift budget, the orchestrator's own cap
  per session, every title `#N`). **Read §11's numbers there, they changed twice on 2026-11-01** (at the last word: the
  orchestrator on Sonnet 5.5 medium, 80 turns / $20 per session, $80 a shift; workers S Haiku, M Sonnet, L Sonnet high).
  The model per kind of task: `C:\Developmentgents\models.json` (`python C:\Developmentgents\models.py set <kind>
  <model>`) – read it before every brief. Guides in
  `C:\Developmentgents\guides\`; cost booking `python C:\Developmentgents\costsdd_entry.py <ledger>
  <transcript.jsonl> --issues N --role worker` (`--role orchestrator --per-day` for yourself); the report
  `python C:\Developmentgents\costseport.py`.

## State (2026-11-01, ~10:45)
- `main` pushed to 62cf64a2 on 2026-11-01 (the owner's word): the viewer deploy (`deploy-app.yml`, the wasm built there)
  is green and `scutoons.com/version.json` reports 62cf64a. The commits below went out with it.
- The list below is what this push carried (kept for the record):
  - #220 quality governor: 55df2990 · 8db51ef3 · d0b1165c · 3f10be0f (the local worker, done, its comment on the issue).
  - #224 navigation: PR #226 merged as 31c6e414 (clean).
  - #211 outputs (MIDI / DMX / strobe): PR #227 merged as 5a1e583e + be72c665 (the merge's two fixes: the stage link's
    `Out::Quality` arm's `continue`, clippy 1.99 `manual_slice_fill` in deck-outputs).
  - cbd8373b HANDOFF + costs; this session's HANDOFF / costs commit.
- Green on the whole: `npm run deck:ci` (release build, 424+ tests, clippy, wasm32), `npm run check -w viewer` (156 tests),
  the wasm rebuilt into `viewer/static/live/` (git-ignored – production builds it on deploy).
- Seen in the in-app browser (viewer-prodapi, WebGL2): the quality bar + panel row, the governor stepping down under
  a throttled tab; tap → zoom toward the knob, tap empty → the being framed. **At the portrait only the core's six
  knobs showed** – the folded knobs may sit inside the head mesh (#224 comment: the S22 check).
- The three PRs' branches (`wip/brave-maxwell-rn7qch`, `wip/keen-hypatia-6pd4ph`) stay on origin until the push;
  PR #217 (#191 design) still waits for the owner. The cloud sessions #224 / #211 are idle (ListAgents).
- Costs: the three workers are in `agents/costs.json` (#220 ≈ $12.02 local; #224 / #211 cloud: no estimate in their
  reports – read from claude.ai → Usage); this orchestrator session's entry is per day (`add_entry.py --per-day`).
- No servers running (viewer-prodapi stopped; ports 4100–4199, 7000–7099, 8000, 9000 free). OSCT's orchestrator today:
  "#243 Orchestrator: Toggle switch merge" (`local_6ae84e2b-…`), idle, no local workers; cloud credit 50/50.
- **Board:** Review on staging: #220, #224, #211 (+ the older ones). In progress: nothing of this shift.
  Backlog (owner's order): #208, #223, #225, #213. Proposals: #221, #216.

## Next
1. **Check on the S22 and the Enttec** (the push is done): `/live/deck` on the phone (the portrait's knobs, the glides,
   the quality row), the strobe on the Enttec (PR #227's steps). GitHub deck CI by hand if wanted (`deck.yml`).
   Then close the three issues' open rests: #224 (the fold at the portrait if hidden on the S22 too), #211 (mapping UI,
   Art-Net, conductor routing), #220 (X1 checks).
2. **#213** camera moves (+ the conductor's shots #189): now unblocked (#224 is in); reuse the eased `goal` / `want`.
   complexity:L (a new area in `camera.rs` + the overlay) → a cloud session the owner starts; tier and ceiling from §11 /
   `models.json` at that time.
3. **#223:** a real `tc netem` run on Linux (X1 or WSL with a slot). If the tail stays, the WebTransport datagram side
   channel.
4. **#225:** which parameters fit which stick axis; hue and glow off the sticks; the rule for the selection's stick.
5. Later queue: #203 patch editor · #210 scene objects · #212 input sources · #205 audience · #206 security · #207 cloth ·
   #215 shadows · #191 instruments (PR #217 waits for the owner).
6. Every new brief: complexity + tier + ceiling (§11, `models.json`), `#N` in the session title, `add_entry.py` at the merge.

## Waiting for the owner
- **The push** of the 9 commits (above), then the S22 / Enttec checks.
- **PR #217** (instruments / presets design, #191): its 8 open questions; merge only with the owner.
- **#208:** should the glitch warning also flash with the HUD off?
- **A shift issue** for the orchestrator (§11.3 wants `#N` in every session title and ledger entry) – or keep listing the
  merged issues per session, as this entry does. This session ran on Fable 5.1 (started before §11); the next one is
  Sonnet 5.5 medium per §11.
- **Owner steps:** bunny zone `lan.scutoons.com` + script + NS in KAS + BUNNY_API_KEY + run lan-cert workflow →
  pair the X1. The X1 partition install (#200). Clean up the stray CDN copies (Admin → Assets → Unused).
- **#216:** the self-hosted Linux runner, yes or no; a look at the Deploy minutes.
- The cloud workers' real cost for #224 / #211 (claude.ai → Settings → Usage) into `costs.json` (`billed`).
