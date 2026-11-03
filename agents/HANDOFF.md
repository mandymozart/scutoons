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
- **GitHub's deck CI runs by hand only** (`deck.yml` workflow_dispatch): run `npm run deck:ci` locally before merging
  (the pre-push hook does it too).
- **No cloud sessions** (owner 2026-11-03: "no cloud sessions. that was a temporary mesaure. remove it"): workers are
  local sessions, each in its own worktree, two at most (RULES B1).
- **Phones test over the LAN, not by pushing:** `npm run dev:lan` serves https://<LAN-IP>:4100 with the API and the
  local relay proxied. The firewall rule for 4100 is set.
- **Night:** never push; one git operator on `main`; no Docker / MariaDB / phone without a slot from the OSCT
  orchestrator. SCUTOONS ports: 7000–7099, 4100–4199, 9500–9599.
- **Costs:** at every merge, add an entry to `agents/costs.json` and a cost line to the closing comment
  (ORCHESTRATION §10). Local sessions: `python C:\Development\tools\costs\transcript_cost.py <transcript.jsonl>`.
  Transcripts are never deleted.
- **No Claude attribution** in commits or PRs.
- **Over Remote Control no session can be started:** the worker is then a background agent of the orchestrator
  (`Agent`, `model: sonnet` / `haiku`, same brief), one at a time. Sessions can still be messaged.
- **The deck is developed remotely now** (#237): `npm run deck:update -- --no-serve` (Linux binaries in Docker, ~3 min,
  `CARGO_BUILD_JOBS=4` – the PC crashed 2026-11-01 under a full-parallel build), serve
  `deck/target/release/deck-update.exe serve deck/os/out/update` (port 7790, firewall rule set), then
  `ssh root@192.168.1.166 'deck-update now'` (the PC's key is on the X1). Stop the server afterwards. Killing cage on the
  X1 needs the owner's OK each time.
- **Usage report** at the end of every session: `python C:\Development\agents\costs\report_usage.py --issues <N>`
  (orchestrator: `--role orchestrator --per-day`), ORCHESTRATION §10 / §11.9.

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
  `python C:\Developmentgents\costs
eport.py`.

## State (2026-11-01, ~20:00 – the deck session: remote updates, then orchestrating workers)
- `origin/main` = 522dd70e (pushed by the owner earlier); **18 commits on `main` not pushed**, all deck/: 112853e1 …
  2353bd99 (#237, #251, #252, #250, #253, #256, #257, #261, #262, #263, DECISIONS). Each issue has its comment.
- **The X1** (installed, 192.168.1.166, `ssh root@`): updates itself over the LAN (deck-update, timer 2 min); runs
  r2430+ (`deck-update status`), `scene = "idle"` (fireflies). A Eurolite USB-DMX512-PRO MK2 on `/dev/ttyUSB0` drives a
  Cameo Thunder Wash 600 RGBW in **4CH_1** (address 001) – the owner confirmed `pulse-thunderwash` ("went perfect").
  The gamepad (Xbox 360 pad) is plugged in; the RME Fireface is unplugged (needs class-compliant mode).
- Built today (all on the X1): remote updates incl. removal of dropped files (#237), boot to stage 31 → 21.5 s (#251),
  idle default scene (#252), F5 scene switcher + PipeWire stream that really stops / crossfades (#250), volume keys
  (#253), kiosk stop 0.3 s (#256), no segfault + exit 75 on a lost display (#257), DMX at 250000 baud + 4CH_1 profile +
  pulse scene + blackout on scene end (#261), blackout on a stop signal (#263), no serial-getty loop (#262).
- Decisions today (DECISIONS.md): remote updates; F1 = the debug overlay, off by default from the 2027-02-05 release;
  scene switches **crossfade** (parametric), stops black out, lights are hardware (#264).
- Tickets created today, all without board status (owner's inbox): #241–#255 (3D scene system / editor, deck next
  steps), #256, #257, #261–#264. Workers' sessions #250 / #253 / #257 are idle (their usage reported; #250's failed –
  agents.porschuetz.de timed out).
- No servers running (7790 stopped). Docker Desktop was restarted after the crash; scutoons-mariadb-1 healthy.

## Next
1. **#264** parametric crossfade on a scene switch (outputs + audio share one transition time) – complexity M, Sonnet.
   The owner's words and the hardware rules (slew limits, flash cap ~3/s, grand master, panic) are on the issue.
2. **Owner checks on the X1:** F5 + the switch time, the volume keys (Fn Lock), the madness → cathedral audio switch.
3. **#228** gamepad in the native stage (gilrs; the X1 has the pad) · **#249** the 2D layer · **#250**'s browser half
   (/live/deck switcher) · **#252**'s fireflies in /live/deck (space-web has no shader nodes).
4. #241 concept and its slices (#242–#248) when the owner sorts them into Backlog; #255 / #245 are `decide`.
5. #201 the LAN relay scan; #254 external screen (needs a monitor at the X1).

## Waiting for the owner
- **The push** of the 18 deck commits (deploys only the viewer side unchanged – deck/ is not deployed, but the push is
  the owner's call).
- **#245** (where the 3D editor lives) and **#255** (sign-in + group on the relay): `decide`.
- The Fireface UCX II in class-compliant mode; a monitor at the X1 for #254; sorting #241–#264 into the board.
