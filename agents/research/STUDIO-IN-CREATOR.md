Status 2026-10-20: concept; the owner chose the web creator + three storage places + the local kit for every member (§2.2, §2.4, Q4 / Q5 answered); questions 1–3, 6, 7 open (§5). No app code yet. Issue #118. Once the owner agrees, the stages in §4 become issues.

# STUDIO-IN-CREATOR – the storyboard studio as part of the creator (concept, 2026-10-20)

Owner 2026-10-20: "storyboard studio will become part of creator." · "becomes part of creator now. the storyboard will
need to run online and than it can still be nicely remote controlled through the mcp and our local machine can generate
what ever needs to be done through ai oder local connection to figma. external apis can be connected via the server i
assume." · "typescript and unit tests will help us save a lot of time during future iterations" · "for scripts i get it,
but for maintable code please refrain from such madness" (about the .mjs / .cjs files). The rule: RULES "Code".

## Summary + recommendation

- **Today**, the storyboard is a separate local app with its own API: about 8,000 lines in the studio, 4,200 in the
  scripts and 3,600 in the tools, all plain JavaScript. All of its state is files in the repo (`storyboards/…`) and in
  `storyboards/_out` (1.1 GB, local only). It reads the script by running `docker exec mariadb`, and it keeps the
  generator keys in the repo's `.env` on the owner's PC (§1).
- **The target** is a **Storyboard page in the creator** (Svelte + TS). Its model lives in **@scutoons/content** (TS,
  tested), and the 3D renderer becomes a creator module (TS, three.js, loaded lazily). The data moves online: a
  `storyboard:<toon>:<chapter>` entity (creator-only, like `script:`) and a generations table. **Files live in three
  places** (owner 2026-10-20, §2.2): the member's **local workspace folder** for every attempt (fast, never uploaded),
  a **shared work storage on the owner's own servers** (signed-in members only) for what is marked for review or
  shared, and **the readers' CDN** for what is approved and used.
- **Remote control:** the SCUTOONS MCP reads and saves the storyboard like any other entity. It prepares generations,
  but **the member confirms the cost in the creator**. The **member's machine** handles what needs a local machine:
  Figma, a local GPU and batch renders. It does this through the MCP, which already runs there and already drives
  Figma. **Every member gets the same local kit** (owner 2026-10-20: no collaborator types – install, connect your
  Claude, go): the MCP extension from the account page with the Figma relay inside, the Figma plugin and the workspace
  folder. No checkout is needed. The browser never talks to localhost (§2.4).
- **No desktop app now** (owner 2026-10-20): the creator in Chrome is always current for everyone. An Electron wrapper
  stays an option for later (§2.4 "When a desktop app").
- **External generators run through the server** (PHP). The keys stay on the server, and the **budget gate** with a hard
  cut (STORYBOARD §7 "Budget", decided 2026-10-09) comes before the first paid job. The jobs need no worker and no cron:
  the server starts the provider's task, and the open page asks the server about it, which asks the provider (§2.5).
- **What stays local:** the Figma relay and plugin, the ch3 production pipeline (until after 2026-11-15) and the local
  GPU. Each moves to TS with a thin CLI the next time it is touched. **What is deleted:** the whole studio app, its API,
  the phone HTTPS server and relay, the scene files after the import, and the one-off converters (§3).
- **Order (§4):** the creator work starts now in parallel and never touches the owner's running studio or the ch3
  pipeline. **#62 lands first.** After that the local studio is frozen: bug fixes only, plus #111's Aspect select. Stages
  S0–S2 (model, data, the page with shots and script) are done before 2026-11-15. S3–S5 (3D, Generate + Variants,
  Handheld) follow, so that **ch4's first throw (~Nov 16) is made in the creator**. S6–S7 move the local parts behind the
  MCP and delete the studio.

## 1. Inventory (measured 2026-10-20, `main` d223bbf5)

### 1.1 workflows/storyboard-studio: 19 .svelte + 17 .js + 27 .mjs, 0 .ts, 8,051 lines (tests included)

A Vite + Svelte app without SvelteKit on port 14000 (`npm run dev:studio`). Its API runs as middleware inside the Vite
dev server (vite.config.js).

| Part | Lines | What it does |
|---|---|---|
| `src/App.svelte`, `TopBar`, `StudioPanel`, `StudioActions`, `steps.svelte.js` | ~550 | the shell: AppShell, step tabs 1–6 (Script · Shots · Layout · Handheld · Generate · Export), per-step panels and widths (localStorage) |
| `ShotList`, `ShotPanel`, `ScriptPanel`, `studio.svelte.js`, `actions.svelte.js` | ~580 | the shots by scene (multi-select), the shot's fields (Shot / Camera / Light / Figures tabs), the scene's script lines |
| `Viewport.svelte` | 550 | the live 3D view: orbit / pan / dolly, figures dragged on the ground, the hand camera written into the shot |
| `Timeline`, `ClipStrip`, `ClipView`, `TrimPair`, `timeline.svelte.js`, `timeline-lanes.js` | ~990 | the picked shots as clips (Shot · Take · Rendered · Generated), playhead, trims |
| `GeneratePanel`, `ManualGenerate`, `VariantsPanel`, `generate.svelte.js`, `frames.js` | ~600 | a generator per shot, cost confirm, Luma by hand, variants / approve (the #62 branch replaces VariantsPanel with the board: +3,000 lines, 4 commits) |
| `HandheldPanel`, `PhonePairing`, `Phone.svelte`, `lib/handheld/*` (7 files) | ~1,150 | the phone as the shot's camera: pairing (QR), pose tracking, takes, render + encode of a take |
| `OutputPanel`, `output.svelte.js` | ~140 | the chapter's handheld videos, downloads |
| `api.mjs` | 306 | 28 routes on the repo's files (below) + `POST /api/run`, which runs 7 scripts as child processes |
| `clips.mjs`, `generated.mjs` | 586 | the clips per shot (reads MP4 / WebM durations itself, ffmpeg thumbs, byte ranges), the generated records (list, upload, mark, approve) |
| `generators/*.mjs` | ~670 | the pluggable registry: comfy-local, runway, luma-manual, a comfyui-remote stub; jobs in memory, rate / euro, spend counter |
| `phone-https.mjs`, `handheld-relay.mjs` | 146 | a self-signed HTTPS server on the LAN (port 14443), the phone ↔ PC relay over Vite's HMR socket |
| `scripts/convert-generated-log.mjs` | 190 | a one-off conversion of the old generated log to v2 |
| `test/`, `generators/test/` | ~1,600 | 13 `node --test` files |

**The local API's routes** and where each one goes: `GET chapters / chapter` (validate + script + variants + handheld)
→ the entity + content's validate. `PUT shot / scene` → a save of the entity. `POST frame` (a rendered PNG) and
`GET/POST variants`, `variants/pick` → the workspace folder / work storage + the entity. `POST prompt` → content's prompt builder.
`POST review` → the generations table (approve). `POST taste` (appends to workflows/scenes/storyboard-taste.md) →
a document the MCP reads (Q7). `GET/PUT takes`, `takes/all` → the entity (pose samples) / the workspace folder (videos). `GET clips / thumbs` → the browser
(`<video>` metadata, a canvas). `GET handheld/urls`, `POST handheld/frame / encode` (ffmpeg) → the browser
(MediaRecorder). `generate*` (6) → the server's generator API. `generated*` (3) → the generations table.
`POST run` → gone (§3).

### 1.2 scripts/storyboard: 24 .mjs + 1 .js + 1 .py, 4,234 lines. Who calls what

| Group | Files (lines) | Called by |
|---|---|---|
| **Model** (pure logic) | `storyboard.mjs` 185 (load, validate, measure), `lib.mjs` 538 (constants, rhythm model, safe-format envelope, entity reading, the Figma bridge client), `schematic.mjs` 93, `shotlist.mjs` 57, `generated-log.mjs` 153 (the record format), `luma.js` 105 + `prompts.mjs` 113 (prompts), `refs.mjs` 65 (references per shot) | everything: `lib.mjs` has 30 importers and `storyboard.mjs` has 12, among them the studio's api.mjs, generators and tests |
| **Views / reports** | `render.mjs` 83 (the storyboard page + rhythm rail), `rhythm-report.mjs` 209, `validate.mjs` 29 | studio `POST /api/run`, by hand |
| **To a chapter** | `compile.mjs` 143 (storyboard → chapter draft), `check-draft.mjs` 15, `approve.mjs` 74 (review + dataset) | studio, by hand |
| **Generation** | `comfy.mjs` 363 (the owner's RTX 2070, Luma agents API), `gemini.mjs` 117, `refpack.mjs` 68 | the studio's comfy-local generator; gemini by hand |
| **Figma / production** (the owner's ch3 pipeline) | `figma-draft` 103, `figma-chapter` 124, `figma-match` 140, `figma-production` 279, `production-upload` 63, `figma-reslice` 130, `figma-clean-slices` 264 | by hand / agents; the MCP's `figma_place_draft` takes figma-draft's specs |
| **One-off** | `convert-624.mjs` 653 (#78), `outline.py` 68 (character outlines for the schematics) | by hand |

`lib.mjs readEntity` reads the script through `docker exec scutoons-mariadb-1 mariadb …`, so it works only on a dev PC
with the prod seed. `mediaRoot` reads character images from the local `media/` dev copy.

### 1.3 tools

- **tools/storyboard-render** (8 .js + 1 .mjs, 1,458 lines, 2 .glb): `core.js` 554 (shot → image, three.js),
  `figures.js` 402, `sets.js` 124, `spec.js` 61 (since af0bb53a also the slice grid: W / S / UNIT 624 /
  SLICE_ASPECTS / slicesOf / aspectSlices / parseAspect), `text-presets.js`, `human-gltf.js`, `tiger-gltf.js`;
  `render-shots.mjs` (headless Chrome → PNG per shot). Used live by the studio's Viewport and headless by
  `POST /api/run frames`.
- **tools/figma-bridge** (10 .js + 5 .mjs, 2,143 lines): `plugin/code.js` + `commands/*` (10 command modules, #93), the
  local relay `relay.mjs` 475 + `supervise.mjs` 204, `cli.mjs`, self-tests. The MCP's `figma_*` tools
  (`tools/mcp/src/figma.ts`, **already TS**) drive it on 127.0.0.1:3056.

### 1.4 Generators and their keys today

| Generator | Where | Key / needs |
|---|---|---|
| Runway (Gen-4 Turbo / 4.5, Aleph 2) | studio `generators/runway.mjs` | `RUNWAY_API_KEY` from the env or the repo's gitignored `.env` (the owner's PC) |
| Luma (agents API) | `comfy.mjs` (image), "Luma – by hand" (copy the prompt, no API) | `LUMA_AGENT(S)_API_KEY`, same `.env` |
| Gemini image | `gemini.mjs`, by hand | `GEMINI_API_KEY`, same `.env` |
| ComfyUI local | `comfy.mjs` via the studio's comfy-local | the owner's GPU and paths (`COMFY_*`) |
| ComfyUI remote (RunPod / Comfy Cloud) | a stub | – |

Safety today: an estimate before each call, a cost confirm, no retry, and a spend counter **per studio session**. There
is no ledger and no budget. Prices are shown in euro at the ECB rate, which the studio fetches.

### 1.5 The data

- **Storyboards:** 14 tracked scene files, `storyboards/catpope/s1c3/scene-25…34.json` (164 KB for ch3), s1c4 35–37
  and s1c5 38. The style folder has 35 reference images and 4 luma-skills briefs. `storyboards/dataset/manifest.jsonl`
  has 4 lines.
- **Local only (`_out`, 1.1 GB):** for s1c3 that is `generated/` 44 MB + `generated.jsonl` (54 records), refpacks
  20 MB, production 54 MB, convert-624 44 MB and Figma matches. Takes live in `storyboards/<toon>/<ch>/takes/<shot>.json`.
- **Rules the studio writes:** `workflows/scenes/storyboard-taste.md` (taste rules appended from reviews).

## 2. The target

### 2.1 Code: where each part goes

| Where | What (from) |
|---|---|
| **@scutoons/content** `storyboard/*.ts` (no Svelte, no three; `node --test` like the other 40 test files) | the types (StoryboardDoc, Scene, Shot, Figure, Camera, Light, Take, GenerationRecord); validate + measure (storyboard.mjs); **the slice grid** (spec.js since af0bb53a: W, S, UNIT, SLICE_ASPECTS, slicesOf, aspectSlices, parseAspect); the rhythm model + safe-format envelope (lib.mjs); schematic SVG (schematic.mjs: a string); compile → chapter draft (compile.mjs, on content's own chapter types); prompts (luma.js, prompts.mjs) + the references' choice (refs.mjs, as asset paths); the record format (generated-log.mjs); shotlist; rhythm report |
| **creator** `src/routes/[toonId]/chapter/[chapterNumber]/storyboard/` + `lib/components/storyboard/*` | the page: the shot list as the page's own list (nav D, #92), the Shot inspector, Script, Timeline, Generate, the variants board + Compare (#62), Handheld, Output – in the creator's AppShell / dock (DOCKING) and steps as tabs |
| **creator** `lib/editor/storyboard-*.ts` (tested) | the UI logic that is pure today: steps, timeline lanes / clips, board layout, zoom, improve form, compare, posters (the #62 branch's modules, with their ~900 test lines, become `.test.ts`) |
| **creator** `lib/storyboard/render/*.ts` (three.js, imported only by the storyboard route) | tools/storyboard-render core / figures / sets / text-presets / gltf loaders; the cast's models from the CDN (`personaModel`, #56) instead of local .glb |
| **server** (PHP) | the `storyboard` entity kind; the generations table + endpoints; the generator registry (Runway, Luma API, Gemini, ComfyUI remote) with server-side keys; the budget gate; the euro rate; the handheld pairing (signalling) |
| **tools/mcp** (TS, local) | storyboard tools (§2.3) and the local runner (§2.4) |

### 2.2 Data online

- **`storyboard:<toon>:<chapter>`**: one versioned entity per chapter with its scenes inline, the same shape the chapter
  took on 2026-10-19. It is creator-only like `script:` (no library endpoint), has drafts and versions like every
  entity (a stale save is refused), and the MCP's `read_entity` / `save_draft` work on it. It holds the shots with their
  fields, the hand camera, figure places, takes (pose samples, a few KB per take) and the shot status (STORYBOARD §7
  "Parallel work").
- **The import:** a migration made from the 14 scene files (RULES "Migrations"). It is idempotent, runs on production
  with the deploy, and happens once, at the cutover (§4 S1). Until then the files stay the only truth, with no dual
  writes. After it, the files leave the repo (git keeps their history).
- **`storyboard_generations`** (table): one row per generation. It holds #62's v2 record (shot, kind, take, parents,
  inputs + roles, generator, model, prompt, seed, ratio, duration, estimate, cost, status, mark, approved, who, date).
  This **is the budget ledger** (STORYBOARD §7: reserved → actual cost). The approve flow writes the shot's `from` and
  the dataset row here (`dataset` = a flag + note, no copy).
- **Files: three places** (owner 2026-10-20: "i like the step plan. 1 2 3"):
  1. **The local workspace folder.** The member picks it once on the Storyboard page (Chrome / Edge folder access,
     the permission remembered), and the MCP is set to the same folder (the extension's setting, default
     `Documents/SCUTOONS`). Every attempt is written there: generated stills and videos, rendered frames, takes,
     thumbnails. Its generation record online names the file (name + hash) and its machine, and nothing is uploaded.
  2. **The shared work storage – on the owner's own servers** (owner 2026-10-20: "i also have some low cost servers
     where i am hosting the apps atm. we could use them for file clutter that is not publish material"). It is never
     the readers' CDN: the API stores the file on the host's disk (the asset layer already has a local storage driver
     next to bunny) and serves it only to signed-in members, so unpublished attempts are never public. A file goes there
     when it is marked for review or shared, so collaborators can see and compare it. It is emptied per chapter after
     the release (an admin action). To measure first: the host's disk quota and its upload limit (PHP's `post_max_size`
     – videos may need chunked uploads). If the disk is too small: more disk on the own servers, never the CDN. A file on someone else's disk shows as a record with "on <member>'s machine – ask to
     share".
  3. **The readers' CDN** (the asset manager) gets only what is approved and placed in the chapter. The rule (owner
     2026-10-20): "the cdn is designated for stuff that needs global delivery fast, collaboration tools do not at the
     meoment" – nothing of the studio's work goes to the CDN before it is reader material.

  Upload time is spent only on what someone deliberately shares or approves. ch3's generated files so far are 44 MB
  for 54 generations.
- **Script text** comes from `script:<toon>` through the API, which replaces `docker exec`. **Character images** come
  from the CDN, which replaces `media/`.

### 2.3 Remote control through the MCP

- It reads and saves with `read_entity` / `save_draft` on `storyboard:…`. Saves are drafts as always (RULES: "drafts +
  a link, the member confirms"). The open creator page notices a newer draft (a version check on focus and every few
  seconds while it is visible) and offers to load it, so an agent's change shows on the owner's screen.
- New tools: `storyboard_validate` (content's validate + rhythm), `list_generations` / `mark_generation`, and
  **`prepare_generation`** (generator, model, prompt, references, estimate), which goes into the shot as a pending
  request. **The member confirms the cost in the creator** (Q3).
- `storyboard-taste.md` becomes a document the MCP serves as a prompt or resource, so it is no longer a repo file the
  studio appends to (Q7).

### 2.4 The local parts: the member's machine, through the MCP

Today the online creator would have to reach localhost to talk to Figma (the relay on 127.0.0.1:3056) or the local GPU.
Browsers increasingly block that (Chrome's local network access prompt), and it would only work on the owner's PC.
**The MCP already runs on the member's machine and already drives Figma.** So the local work is done by the MCP: the
member asks their Claude, and the MCP reads the online storyboard, runs the local step, writes the file into the
workspace folder and adds its generation record:

**The local kit – the same for every member** (owner 2026-10-20: "there needs to be no differentiation between the
types of collaborators. they can just install the stuff and connect their claude and go"). It builds on what exists
(MCP.md: the one-file build and the Claude Desktop extension `scutoons.mcpb`, downloaded from the account page's
"AI connections", no checkout, no npm):
- **The MCP extension** also carries the Figma relay. The MCP starts it on demand, which replaces the separate
  `supervise.mjs --detach`. Its settings: the token and the workspace folder.
- **The Figma plugin** is a download next to it (the manifest + code; Figma desktop → "Import plugin from manifest"),
  versioned with the MCP.
- **A local GPU** (optional): the MCP's `generate_local` talks to a ComfyUI the member runs (`COMFY_URL`). It has no
  owner-specific paths.
- **"Set up your machine"** on the account page: one short page with the three steps and a check (`whoami` +
  `figma_status`). The version check that exists says when the kit is outdated.

The kit's local steps:

- **Figma:** `figma_place_draft` builds its spec from the online storyboard instead of `_out/…/spec.json`. figma-chapter,
  figma-match and figma-production become MCP tools (TS) after ch3.
- **Local GPU (comfy-local):** a `generate_local` tool. It costs no money, so no budget is needed.
- **Batch renders** (all frames of a chapter): every browser renders a shot's frame itself from the
  storyboard (the renderer is deterministic), so frames are not stored at all, and the headless batch is unnecessary. If one is still wanted, the MCP opens the creator's render route in a
  headless browser.
- No always-on local agent and no polling (STORYBOARD §7 "no silent costs"; Q5 answered).

**When a desktop app.** Not now (owner 2026-10-20: the web is current for everyone, Chrome is fine). An Electron wrapper
around the same creator page comes up only if the local steps have to work without Claude (a button instead of asking
your AI). Its costs would then be: signing (Windows) and notarizing (macOS), an updater and release hosting, version
skew against the API, and one more CI build. Because the page is Svelte + TS either way, wrapping it later needs no
rewrite.

### 2.5 External generators through the server

- **The registry moves to PHP:** one interface per provider (estimate, start, status, cancel, result), split by kind
  (image / video, #61). The keys go in the server's config (config.local.php / host env), and the admin's settings show
  "configured", never the key. **Owner step:** put RUNWAY / LUMA / GEMINI keys on the server.
- **Jobs without a worker:** `POST /storyboard/generations` checks the budget, reserves the estimate, starts the
  provider's task and stores the task id. `GET …/{id}` (polled by the open page, and by the MCP) asks the provider once
  per call. On success the server books the actual cost, and the page downloads the result straight into the member's
  workspace folder (through the server as a pass-through when the provider's link refuses the browser; the server
  stores nothing). If no page is
  open, the next open page or the MCP picks it up. A cron URL like the Embassy's is only an option. This fits the host:
  shared PHP, no long-running process, no sockets.
- **The budget gate** (phase 3a, decided 2026-10-09): `generation_budget` in the admin, the ledger above, a hard cut
  with `402 budget_exhausted`, and one notice in the creator. **It comes before the first server-side paid job**, which
  replaces the per-session spend counter.
- The euro display (DECISIONS 2026-10-20 "preise in euro") stays. The server fetches the ECB rate at most once a day.

### 2.6 Handheld camera online

Both the phone and the PC open the creator (HTTPS, a real certificate, so the self-signed one and its "Continue" are
gone). The phone connects through a **WebRTC data channel**. Pairing is a QR code with a one-time code, and the offer and
answer travel through two short API calls (a small table, deleted after pairing). On the same Wi-Fi no TURN server is
needed. The poses run peer to peer, so nothing per frame goes through PHP. The take is recorded in the browser
(MediaRecorder, replacing ffmpeg) and saved into the workspace folder. The phone page needs no sign-in, because the one-time code
grants only this channel (Q6).

## 3. What stays local, what is deleted

| Part | Fate | Why |
|---|---|---|
| tools/figma-bridge (plugin, relay, supervise, cli) | **stays local**, in every member's kit: the relay → TS inside the MCP extension, the plugin a download (after ch3); `supervise.mjs` goes | a Figma plugin can only talk to a socket on the member's machine |
| figma-draft / chapter / match / production, production-upload | **stay as they are until after 2026-11-15**, then become MCP tools in TS | the owner's ch3 pipeline (#107–#109 just landed); art freeze 2026-11-10 |
| comfy.mjs (local GPU) | stays local behind the MCP (`generate_local`) → TS; retired if ComfyUI remote replaces it | the owner's own GPU |
| convert-624.mjs, figma-reslice, figma-clean-slices, convert-generated-log.mjs | **deleted** once their one run is done | one-offs; no re-cut tooling for 625 (DECISIONS 2026-10-20) |
| outline.py | stays a local tool (Python, like the `cdn/` tools) or goes into the asset pipeline | makes character outlines once per image |
| rhythm-report.mjs | content function + a thin CLI | measures existing chapters |
| workflows/storyboard-studio (all of it), `dev:studio`, ports 14000 / 14443 | **deleted** after S5 | replaced by the creator page |
| scripts/storyboard model files, tools/storyboard-render | **moved** (a move, not a copy – PLATFORM "dogmas") | §2.1 |
| `storyboards/<toon>/*.json`, `takes/`, `dataset/` | **deleted** after the import | the entity / table are the truth |
| `storyboards/_out` | the owner's local folder, deletable | never tracked |
| `lib.mjs` readEntity (`docker exec`), `mediaRoot` | **deleted** | the API, the CDN |

## 4. Order – stages (each its own issue once agreed)

| Stage | What | Touches the running studio / ch3? | When | Effort (agent days) |
|---|---|---|---|---|
| **S0** | content `storyboard/*.ts`: types, validate / measure, the slice grid (out of spec.js), rhythm + safe envelope, schematic, prompts, record format – tested. spec.js / lib.mjs **import** it (node's `--experimental-transform-types`, as check-draft.mjs does), so there is one source. | only the imports (same numbers, the studio's tests stay green) | now, after #62 lands | 2–3 |
| **S1** | server: `storyboard` entity kind, `storyboard_generations`, the shared work storage on the app host (quota + upload limit measured first); the import migration (written, dry-run on the dev DB; **applied only at the cutover**) | no | before Nov 15 | 2–3 |
| **S2** | creator page: shot list, Shot inspector, Script, schematic + rhythm rail, validate; read / save the entity (dev DB); the workspace folder (pick, read, write) | no | before Nov 15 | 3–4 |
| **S3** | 3D: the renderer as TS in the creator, the Viewport (orbit, figures, hand camera), frames rendered in the browser, not stored | no | Nov 13–20 | 3–5 |
| **S4** | Generate + Variants board + Compare (port of #62); the budget gate (3a) + the server registry (Runway first, then Luma API / Gemini – #61, #112's fit step) | no | after Nov 15 | 5–8 |
| **S5** | Handheld (WebRTC), Output / Export; **cutover**: apply the import, the creator is the studio | ends it | for ch4's first throw (~Nov 16–20) or right after | 3–4 |
| **S6** | the local kit: MCP storyboard tools, `prepare_generation`, `generate_local`, the workspace setting; the Figma relay inside the extension + the plugin download; Figma tools from the online storyboard (the figma-* scripts → TS); "Set up your machine" on the account page | after ch3 | Nov 16 → Dec | 4–6 |
| **S7** | delete the studio, the moved scripts, `dev:studio`, the scene files | – | after S5 + S6 | 1 |

Against the other work:
- **#62** (variants canvas, phases 1–4 on `claude/studio-layout-afe926`, +3,000 lines JS, not merged): it lands as it
  is (the owner uses it for ch3's gaps, and DECISIONS 2026-10-20 says the ch3 layout waits for #62 + #92). After that
  the local studio is **frozen**: bug fixes, plus #111's Aspect select (a select over content's SLICE_ASPECTS after S0).
  Phase 5 and any new studio features are built only in the creator (Q2). The #62 modules' tests are the spec for S4's
  port.
- **#92** (creator navigation draft D, built on origin/main): the Storyboard page is a chapter page and a tab next to the
  chapter, and the shot list is "the page's own list beside the menu". Nothing waits on it apart from merging it first.
- **#111 / #112:** the grid numbers move into content in S0 (from spec.js, af0bb53a). #112's prompts in slices and the
  fit step (crop to 1170 × 1872k on approve) are written against content's TS prompts after S0, not in luma.js.
- **#61** (generators image vs video, Luma API): it becomes S4's server registry instead of another studio adapter.
- **#64** (ch3, 2026-11-15; art freeze 11-10, deploy freeze 11-12): nothing in S0–S3 changes the studio's behaviour or
  the figma / production scripts. S4–S7 start after 11-15.
- **#56** (cast from 3D models): S3 loads `personaModel` from the CDN, which is #56's open renderer step.

## 5. Questions for the owner (each with a recommendation)

1. **When do you switch to the creator?** (a) **ch4's first throw, ~Nov 16–20** (recommended: S0–S3 fit before it
   without touching ch3), (b) now, with ch4–5's whole-slice rewrite (it would compete with ch3's tools), (c) after ch5
   (2027-01-15).
2. **The local studio until then:** (a) **frozen once #62 lands, only fixes and #111's Aspect select** (recommended:
   nothing is built twice), (b) features continue there (built twice).
3. **Paid generation from the MCP:** (a) **the MCP prepares, and you confirm the cost in the creator** (recommended,
   like "drafts → member confirms"), (b) the MCP may start paid jobs on its own within the budget.
4. ~~Generated files online~~ – **answered 2026-10-20:** three places – the local workspace folder, a shared work
   storage on the owner's own servers, the readers' CDN only for what is approved (§2.2); no desktop app now, the
   web creator in Chrome.
5. ~~Local work~~ – **answered 2026-10-20:** through each member's Claude + the MCP, with one local kit for everyone
   (§2.4).
6. **The phone as a camera:** (a) **directly in the creator (WebRTC, QR pairing, no certificate prompt)** (recommended),
   (b) keep a local relay (then the handheld step only works on your PC).
7. **The taste rules** (`storyboard-taste.md`, appended from reviews): (a) **a document in the DB that the MCP reads and
   the creator shows** (recommended: it works online, with versions), (b) it stays a repo file (agents only).

## 6. Measured how

`git ls-files` per folder, `wc -l` over .js / .mjs / .ts / .svelte; callers by grepping imports across scripts,
workflows/storyboard-studio, tools and tools/mcp/src; keys by grepping `process.env.*`; routes from api.mjs /
generators/index.mjs / generated.mjs; `du` on storyboards/_out; the #62 branch by `git diff --stat main...`.
