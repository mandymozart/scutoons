# SCUTOONS Figma bridge

Full read / write access to the owner's open Figma file **without** the Figma MCP's monthly read cap (Starter: ~20
reads; owner 2026-10-09: "then we just build a SCUTOONS plugin bridge"). A private development plugin – not
published, no cost – talks to a relay on the owner's own computer; tools (the CLI, the storyboard scripts, the SCUTOONS
MCP) talk to the relay. Nothing leaves the computer. Concept: `agents/STORYBOARD.md` §9. Self-healing since #78
(owner 2026-10-20: "i dont like manually restarting figma and the bridge or claude or the machine").

```
scripts / cli.mjs / SCUTOONS MCP ──HTTP──► relay.mjs (localhost:3056) ◄──WebSocket── plugin ui.html ──► code.js (Plugin API, the open file)
                                             ▲ kept running by supervise.mjs (detached)
```

## Setup (once)

1. Figma **desktop** app → any design file → Plugins → Development → **Import plugin from manifest…** →
   `tools/figma-bridge/plugin/manifest.json` (from the checkout you work in).
2. Start the relay under its supervisor, **detached** – it runs in the background without a window and outlives the
   terminal, Claude sessions and their background tasks (it died with one on 2026-10-19):
   - any shell (PowerShell, Git Bash, an agent's Bash tool): `node tools/figma-bridge/supervise.mjs --detach`
   - PowerShell by hand, the same: `Start-Process node -ArgumentList 'tools/figma-bridge/supervise.mjs' -WindowStyle Hidden -WorkingDirectory C:\Development\scutoons`

   A second start says "already running" and changes nothing. A script that finds no relay starts it this way by
   itself (`figma()` in `scripts/storyboard/lib.mjs`).
3. In the file (Cat-Pope): Plugins → Development → **SCUTOONS Bridge** (afterwards **Ctrl+Alt+P** = "Run last plugin").
   The small window shows a green dot when connected, orange while it works, and its code version. It acts as you,
   only while the window is open.

`node tools/figma-bridge/cli.mjs status` → `{ "plugin": true, "stale": false, "busy": null, "queue": 0, "version": …, "file": "Cat-Pope" }`.

**At logon (not installed – the owner's choice):** Windows Task Scheduler → Create Task → trigger "At log on", action
`node` with the arguments `C:\Development\scutoons\tools\figma-bridge\supervise.mjs --detach`, start in
`C:\Development\scutoons` (or a shortcut with that command in `shell:startup`). `--detach` matters: the launcher's window
closes after a second and the supervisor runs on without one – without it, closing that window would end both. Without
the task: step 2 once after a reboot – or let the first script do it.

Supervisor: `supervise.mjs --status` (lock + the relay's /status), `--restart` (a fresh relay, e.g. after a change to
relay.mjs – the supervisor stays), `--stop` (both; after a change to supervise.mjs: `--stop`, then `--detach`). It
restarts a relay that exits (backoff 1 s … 60 s, at most 30 starts an hour); a relay started by hand on the port is
watched, not fought. Its lock (`logs/figma-supervisor.json`) carries a beat every 10 s: a lock left by a crash, a
reboot or a kill (its pid maybe reused by another program) doesn't count, and `--stop` kills only a pid proven to be a
supervisor (the relay names it, or its command line is supervise.mjs).

Only local tools reach the relay: `/cmd` refuses calls with an `Origin` header (browsers send one) and anything that is
not `content-type: application/json`; `/plugin` takes Figma's plugin iframes (Origin `null`) only – no web page in the
owner's browser can drive Figma or take the bridge over. A second plugin window takes over only with its hello.

## What still needs the owner (one click each)

- **The plugin after a change to `plugin/code.js` / `ui.html`**: with "Hot reload plugin" on (Figma desktop: Plugins →
  Development – it was on 2026-10-20: every save reconnected the new code within 0.3 s) nothing; the command it was
  running is lost and `figma()` sends it again. With it off, Figma closes the plugin: **Ctrl+Alt+P** (or Plugins →
  Development → SCUTOONS Bridge). Agents batch such changes, bump `VERSION` in code.js and say so – never during a run.
  A run started before waits for the new code ("the plugin runs old code …") and goes on.
- **After a Figma crash or restart**: open the file, Ctrl+Alt+P – **in the file the run works on**: a run waits for a
  plugin in another file ("the SCUTOONS Bridge runs in the Figma file …, this run needs …") – node ids are per file.
- **Figma frozen**: the heartbeat can't see it (pongs come from Figma's network layer, not the plugin); a waiting run
  pings the plugin itself and prints "Figma answers no ping … looks frozen" once – restart Figma, open the file,
  Ctrl+Alt+P; the run goes on. Busy for 30 min: the run stops (`plugin_stuck`) – run it again, it resumes.
- Two Figma windows running the bridge: the newer one takes over; the older says "another Figma window has the bridge".

## Recovery – what a run does by itself

The failures of 10-10 … 10-19 (#78): plugin closed by code edits (A), heavy calls piling up after timeouts (B, D, E),
the relay dying with a Claude session (F), every error reading "no relay" (G).

| Situation | Relay | Script (`figma()`, lib.mjs) |
| --- | --- | --- |
| no relay | – | starts the supervisor once (detached), waits up to 30 s; if none comes up, the error carries the supervisor's own words |
| no plugin / it went away | answers `no_plugin` / every call in flight or queued `plugin_disconnected` at once | ONE line what to click, polls `/status` every 5 s, goes on when it is back (up to 30 min, `SCUTOONS_FIGMA_WAIT_MIN`) |
| the plugin re-run in another file | `/status` → `file` (the plugin's hello) | waits for it in the run's file (`expectFile`), one line |
| a call takes too long | answers `timeout`, but stays busy until Figma's late answer (or the plugin's idle state: the answer is lost) – the next call waits | never re-sends at once: waits until not busy, retries after 10 / 30 / 60 / 120 s with the timeout doubled (4 retries) |
| Figma stuck on a call | – | pings it; unanswered: one "frozen" line; busy for 30 min: `plugin_stuck`, the run stops (two slices in a row timing out too) |
| many calls at once | ONE queue, one call at the plugin (the plugin has its own queue too); `ping` passes it | – |
| connection dead (Figma closed, crashed, asleep) | heartbeat: a ping every 10 s, `stale` after 30 s without pong | waits, one line |
| plugin closed mid-`region` | – | the plugin removes leftover "scutoons-export (temporary)" frames on start and after every region / export |
| crash mid-run | – | `run.json` beside the output: a slice is done only with its entry AND its three WebPs at the expected size (a 625 file is no 624 one) – run again, it resumes |
| a slice fails for good | – | marked failed (production: its old files removed), the rest goes on, exit code 1 with the list; production writes no chapter.json / uploads.json, clean-slices uploads nothing |
| relay / supervisor killed, reboot | the supervisor restarts the relay; a lock left behind doesn't count | starts the supervisor again |
| low memory | – | preflight and pacing wait while < 4 GB RAM are free |

Long runs (`figma-production.mjs`, `figma-clean-slices.mjs`) start with `figmaPreflight()` (relay, plugin, its code
version = code.js, a ping, free RAM, the file – then passed as `expectFile`), cut with `region` + `partial` (+ `layers`
for storyboard drafts: per layer a shell with the layer's look, holding only its nodes that reach into the slice), and
pause 15 s every 25 exports and after a call slower than twice the median. `figma-production.mjs` writes to new CDN
paths (`--folder`, default `s<n>/c<n>/v624` – the 625 cut keeps its own), copies a cut at another standard (its
lossless masters) to `production-<standard>/` once before overwriting it, and writes `chapter.json` / `uploads.json`
only when every slice is exported at the standard. The standard slice is 390 × 624, cut from the draft document
(owner 2026-10-20: "we go from the draft document. since manual slicing is not required anymore"); the 625 cuts of
chapters 1–5 were resampled to 624 (`scripts/storyboard/convert-624.mjs`, #78). `figma-clean-slices.mjs` and
`figma-reslice.mjs` re-cut only the old 625 cuts (legacy) and refuse a group with 624 slices.

**The log**: `logs/figma-relay.log` – ISO time, command, node, ms, bytes in / out, the caller, connects / close codes,
stale, the supervisor's restarts; appended, rotated to `figma-relay.1.log` at ~5 MB.

## Commands

| CLI | Does |
| --- | --- |
| `status` | the relay's state: plugin, stale, busy (cmd, since, timed out), queue, the plugin's code version, pids |
| `preflight` | what a long run checks first (and waits for) |
| `ping` | file, page, user, code version, busy |
| `pages` | the file's pages (id, name) |
| `read --page "S1C3" [--depth 2]` / `read --node 555:2354` | the node tree: id, name, type, x / y / w / h, text, image hash, locked / hidden – a node is much cheaper than a page (a page loads whole) |
| `export --node 555:2354 --format png --scale 1 --to out.png` | a node as PNG / JPG / SVG (thumbnails for matching, slices for the CDN) |
| `run file-info [--args '{…}']` | a command module from `commands/` (below, "Command modules") |
| `draft storyboards/_out/s1c3/figma/scene-25.spec.json …` | builds / updates a scene's draft frame (from `scripts/storyboard/figma-draft.mjs`): locked `storyboard`, hidden `guides`, `text`, empty `layout` on top, the rail beside it. Re-runs replace shots by id and never touch `layout` – but resize and re-stack the frame and replace the shots' texts: for the lines of a frame in work, `slice-guides` (below). |

`export` and the storyboard scripts' commands (`region`, `sliceFrames`, `exportScales`, `thumbs`, `place`) are
modules in `commands/` (below); the core keeps `ping`, `pages`, `read`, `draft` and `run`. The draft code (`applyDraft` in `plugin/code.js`) is the same one `figma-draft.mjs` inlines for Figma's own
MCP (`use_figma`) – one implementation for both ways.

## Command modules (#93)

New commands are files, not plugin code (owner 2026-10-20: "whenever you write new scripts you need to restart the
bridge, can you conceive a method to load modules on demand while in runtime?"): **`commands/<name>.js`** holds the
body of `async (args, lib) => { … }` and returns the answer (JSON). Write or edit one and call it – no relay restart,
no plugin re-run, no hot reload:

- `node tools/figma-bridge/cli.mjs run <name> [--args '{"k":1}']`, or from a script `figma('<name>', args)` /
  `figma('run', { name, args })` – the relay treats a command with a module file as a module.
- The relay reads the file **from disk at each send** (name `^[a-z][\w-]*$` only, never source from the caller: what
  runs in Figma = what is in the repo) and sends `run { name, hash, src, args }`; the plugin compiles it once per hash
  (`new Function`, else indirect `eval` – the first module probes the sandbox), so an edited file runs at once.
- In scope: `figma` (the Plugin API), `args`, `lib` = the core's helpers (`describe`, `nodeOrPage`, `sweep`,
  `sweepPage`, `temps` / `setTemps`, `copyLook`, `applyDraft`, `errorText`, `TEMP`, `VERSION`) and `lib.commands`
  (the core commands, e.g. `await lib.commands.read({ nodeId })`).
- Plugin-sandbox syntax: no `?.`, no object spread (`??` and array spread work). A syntax error or a throw →
  `plugin_error` with the message; no such file → `bad_request`. The log names the module and its hash
  (`where where@1a2b3c4d → ok …`); `ping` → `modules`, `compiles`, `compiler`.
- A module named like a core command replaces it. `export`, `region`, `sliceFrames`, `exportScales`, `thumbs` and
  `place` moved out of the core this way (2026-10-20) – callers are unchanged (`figma('region', …)`).
- The core (`plugin/code.js`: connection, queue, ping, pages, read, draft, sweep, helpers, `run`) changes rarely; a
  change there still reloads the plugin (above). The smallest example module: `commands/file-info.js`.
- `commands/text-paints.js` (#94): text layers' fill / stroke as SCUTOONS paints – a colour or a linear / radial
  gradient (Figma's gradientTransform → CSS angle / centre + radius); `figma-clean-slices.mjs` uses it for lettering boxes.
- `commands/slice-guides.js` (#107): a storyboard draft frame's slice lines and rail, redrawn – and nothing else. The
  frame by its exact name; only `guides`' children and the "… · rail" group are replaced (the rail stays at its x, a
  shot keeps its old label); the frame and its layers are never resized, re-stacked, shown / hidden or locked, and
  `storyboard`, `text`, `layout` are never touched. A shot not where the cut expects it → nothing is drawn. Run it with
  `figma-chapter.mjs … --name "<frame>" --guides-only` (the lines = lib.mjs `sliceLines`: every 624 from each group's
  top, "end · +r px" per group – where `figma-production.mjs` cuts; its `--dry` checks they agree).

## In the SCUTOONS MCP

Creators (content write) get `figma_status`, `figma_read`, `figma_view` (a node as a PNG the AI can look at) and
`figma_place_draft` (`tools/mcp/src/figma.ts`) – the same relay; `SCUTOONS_FIGMA_PORT` if not 3056. Each call has its
own timeout (below fetch's 300 s) and the errors say what to do (no relay, no plugin, Figma still busy, queue busy).

**Members (#127): the MCP carries the relay.** A member's SCUTOONS MCP (the one-file copy / Claude Desktop extension)
has a TypeScript port of this relay (`tools/mcp/src/figma-relay.ts`, the same protocol on 3056 – its test runs this
relay and the port through the same stand-in-plugin scenarios) and starts it on the first figma_* call when nothing
listens; the plugin is a download built with the MCP (`scutoons.com/figma-plugin`, these `plugin/` files). Members need
no checkout and no `supervise.mjs`. From a checkout the MCP never starts its own relay (only with
`SCUTOONS_FIGMA_RELAY=mcp`): this folder's relay.mjs + supervise.mjs + cli.mjs stay as they are for the production
scripts until #127's second part after the art freeze (the figma-* scripts → MCP tools; agents/MCP.md §3a "Switching").

## Check

`node tools/figma-bridge/test.mjs` – no Figma needed: relays on spare ports (3993–3999), stand-in plugins, the queue,
timeouts, disconnects, heartbeat, local-tools-only, single instance, the log, the lib client's waiting and retrying
(plugin back, right file, Figma stuck), the supervisor (started detached, restart, stop, stale locks), and
`plugin/code.js` against a small fake of the Plugin API (its queue, the temporary-frame sweep, region partial + layers,
a page not loaded, applyDraft, command modules: compiled once per hash, an edit recompiles, errors) – also end to end
through the relay (modules read per call, caller source ignored, unknown module / bad name).
`node --experimental-transform-types --no-warnings tools/figma-bridge/test-mcp.mjs` – the MCP's figma_* tools through
the relay and a stand-in plugin.

## Not yet

- A one-click relaunch button in the file (`relaunchButtons` in the manifest + `figma.root.setRelaunchData`) – Ctrl+Alt+P
  does the same today.
- Figma's REST image export (`GET /v1/images`) for big unattended exports from prepared slice frames: needs a personal
  access token (owner step) and the Starter plan's rate limits checked first.
