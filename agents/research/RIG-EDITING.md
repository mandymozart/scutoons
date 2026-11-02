Status 2026-11-02: the owner's design round (#269) – the rig designer (owner 2026-11-02, §2) replaces the first draft's "three places". Answered: one rig per character with persona variants (7), the Rigs list + a link from the character (6), testing a rig through the DB (5), the patch editor in the browser first, the graph read-only at the start (2), promote only after a member submits (1), the designer in the viewer as a shared package (§9.1), the compiler as a second container in the relay's bunny app (§9.2). Open for the owner: §9.3–9.7. Nothing of §2–8 is built; the #269 draft (catalog pages, the TS validator) is parked on branch `parked/269-rig-editor`.

# RIG-EDITING — the rig designer: where instruments are written, patched, wired and kept

## 1. In one sentence
**One rig designer, where a member writes or patches an instrument (Rust code or nodes), hears it, maps its parameters, and assigns it to a character; the code is compiled into modules the deck engine runs; a member's work stays theirs until they submit it, and only a submitted one can be promoted into the factory everyone gets.**

Owner 2026-11-02: "creator to wire is weird. we need a single place to wire, choose from available instruments and presets, and promote can only happen if a user submitted their patch for consideration" · "i need the node patching environment to create these and alternatively a code form, where one can programme their own patches like we would do in rust. it gets compiled and then can run in engine natively" · "be certain. make it proper."

## 2. The rig designer – four tabs (the owner's)
1. **Parameters** – what the instrument shows: only what its Rust code makes public (`#[param]`, §4) or what its graph exposes. Per parameter: label (de / en), range, curve, default; the mappings (MIDI CC, face, motion, pad, stick – today's `Mapping`). No parameter can be invented here – the code is the source.
2. **Nodes** – the graph: built-in node kinds and compiled modules (§3) as boxes, links as cables – Max/MSP, Unreal Blueprints, TouchDesigner. **Read-only first** (shows the patch, live values on the cables, the being's hoses are the same graph); editing follows (#203: add a node, drag a link, type-checked, the matrix on a phone).
3. **Rust** – a code editor like Shadertoy's: one tab per file, add / rename / remove files, Save, **Compile**. Only a compile that succeeds hands the logic to the other tabs (new parameters appear in 1, the module's ports in 2). Debugging: §5.
4. **Assignment** – which character (persona, mood) plays it, the rig's tracks and presets per mood, routes into the being's look, the phone deck's sticks, and **scene and world variables** (KANON's story variables, the scene's state) feeding parameters. The same is reachable from code: the module API reads globals (tempo, the character's mood / persona / energy, scene and world variables, §4) – "this will grow".

Beside the tabs, always: **Play** (the engine runs what is open, on this device), the preset bar (save, derive, the lineage), **Submit for consideration** (§6).

## 3. One model, two ways to write an instrument
- An **instrument** stays what it is (research/INSTRUMENTS.md §1): a graph of nodes + `out` + `expose` + rights.
- A node is either a **built-in kind** (graph.rs: osc, fm, lfo, walk, events …) or a **module** = compiled code (new). A module declares ports and parameters; once compiled it is a node kind like the built-ins, usable in any graph.
- So "code form" and "node patching" are not two systems: a code patch is a module (often with a one-node graph around it), a node patch wires modules and built-ins. Presets, lineage, rigs, `Rig::play`, the validator stay exactly as they are; modules add one thing to the node kinds: kinds that come from the library instead of graph.rs.

## 4. The module: what code sees, how it runs
- **A crate per module** against a small SDK crate (`deck-module`, in deck/): a trait the code implements – `process(&mut self, block, ctx)` for audio (block-wise, no allocation), `control(&mut self, ctx)` for control rate, optional `video` later as WGSL data (wasm has no GPU, #185). `#[param(range = 0.0..8.0, default = 2.0)]` on fields = the parameters tab; `#[port]` = inputs / outputs with their types (audio / control / video). The SDK generates the descriptor the compiler extracts (the parameters, ports, the ABI version).
- **ctx = the globals, read-only**: the tempo (bpm, beat, phase – the deck clock #181), the character (id, persona, mood, the feelings #188), scene and world variables, the device's controls. Writing back to the world happens through declared outputs (a control output routed to `being:glow`, an event emitted like deck-events' generators) – never by mutating globals, so a module can't break a show.
- **Where it runs (two tiers, Rust first):**
  - **wasm (every draft, every member's module):** compiled to `wasm32-unknown-unknown`, run by wasmtime on a deck (cached compile, instantiate 72 µs, 1.0–2.4× native – the 2026-10-29 measurements, #185) and natively by the browser's engine (live-core in its AudioWorklet). Sandboxed: a module cannot touch files, network or other modules; a trap silences that node, not the show. Hot-reloadable.
  - **native (promoted factory modules):** compiled into the deck image as a cargo feature, the same code (#185's scene-driven builds: a scene names its modules, `deck-scene-features` picks them) – the speed the owner asked for, and no dylibs (no ABI, no sandbox – DECISIONS 2026-10-30). Shipped with `deck-update` bundles (#237). The browser always runs wasm.
- **The compile service**: rustc can't run in a browser or on the FTP hosts. A **build service** – a small Rust HTTP service with a pinned toolchain + the SDK + its dependencies prebuilt into its image (a few seconds per compile) – takes the files, answers the `.wasm` + the descriptor or the compiler's errors as JSON (file, line, column, message).
  - **Where (owner 2026-11-02: "on the same server we run the relay … i will run the entire apps in one magic container on the long run"):** a **second container in the relay's bunny Magic Containers app** (`scutoons-live`), its own image (`scutoons-build`, ghcr.io, its own workflow like `live-image.yml`). bunny runs an app's containers in one pod: they share the network (localhost, one port each), the resources (8 CPU / 32 GiB per pod on a standard account, 10 GB ephemeral disk) and the lifecycle (they start, stop and scale together). That is the first step towards the one-pod platform.
  - **What sharing a pod costs, and the rules for it:** (1) **CPU** – there are no per-container limits, so a compile could starve the relay ("performance and stability is everything", #223): one compile at a time, `CARGO_BUILD_JOBS=2` (the relay needs < 1 % of a core), a queue, and **no compiles while the relay reports a live session** (`127.0.0.1:7700/status` → `live > 0`: the request waits or is answered "after the show"). (2) **Lifecycle** – updating either image restarts the pod, the relay too: the deploy freeze of `live-image.yml` (no update while `live > 0`) covers both workflows; and the relay can no longer be stopped between shows while members compile – the pod stays up (cost: one always-on pod instead of a stoppable one). (3) **Start time** – the toolchain image is ~1.5–2 GB against the relay's 3 MB, so a cold pod start takes longer (minutes on a new node, cached after).
  - **Hostile code**: compiling must never run member code on the server. Member crates get no `build.rs`, no proc-macros of their own, no dependencies beyond the SDK's vetted list (the service writes `Cargo.toml` itself, `--offline`); rustc itself gets a time and memory limit (a hostile source can at most waste a compile slot). A module's `#[test]`s run as wasm in wasmtime inside the build container, with fuel and memory limits – never natively. The build container holds no secrets (the relay's `LIVE_TOKEN_SECRET` is the relay container's env only); it is reached through the API, which checks who may compile (§9.3) and stores the result.
  - **The native tier** (promoted modules in the deck image) is not built here: it is part of the deck image build (deck/os, Docker on the PC / CI), as today.

## 5. Debugging
- **Compile**: rustc's errors at their file and line in the editor (squiggles, a problems list – the same Problems panel the creator has); warnings shown, not blocking.
- **Run**: every port has a scope / meter in the node view (values on the cables); `log!()` in the SDK prints to a console under the editor (rate-limited, per module); a trap (panic, out-of-bounds, a time-budget overrun) silences the node, marks it red and shows the message; a CPU meter per module (µs per block against the budget).
- **Determinism**: a module gets its seed from ctx (like deck-events' generators), so two decks play the same – a test page can render N blocks offline and compare (unit tests for modules: a `#[test]` file in the module runs on the build service).

## 6. Who owns what – save, test, submit, promote
- **Save = the member's, everywhere** (owner, decision 5): a member's modules, instruments, presets and rigs are stored globally, bound to the user (member rows; today `member_presets` / `member_rigs`, #258 / #270, + `member_modules`). Their decks read them from the DB next to the factory and play them – **that is how a rig assignment is tested**, on any of their decks.
- **Submit for consideration**: the member sends a module / instrument / preset / rig (with its sources) to review – a frozen copy, a note. Nothing else can be promoted.
- **Promote**: a reviewer (creator role, the review rules) takes a submitted one into the factory: a factory draft with the sources, then Publish – from then on everyone has it; a promoted module may be marked for the native tier (built into the next deck image). The member's own copy stays theirs.
- **Confirmed = wired to a character** (decision 5): a factory rig assigned to a character (published) plays for everyone. **This mapping is the important part.** The designer and the creator's character page show every character **without a default rig**; a deck never fails for lack of one – **the fallback is always a standard rig we provide** (§9.4).

## 7. Where it is stored (the owner's question)
| What | Where | Why |
|---|---|---|
| a module's **source files** (`{ "lib.rs": "…", "osc.rs": "…" }`, + `Cargo.toml` fields the SDK allows) | **in the DB as a versioned document** – factory `module:<id>` in `entity_versions` (drafts, versions, history, restore, review, publish – like items); a member's in `member_modules` (`m:<ulid>`, versions kept) | everything else is a document already; history and review come free; Shadertoy keeps sources in its DB too. Size: text, small; a limit (e.g. 256 KB per module, 32 files) |
| the **compiled `.wasm`** | **the CDN as an asset**, named by its content hash (`modules/<id>/<hash>.wasm`) – the immutable-paths rule | decks and browsers fetch and cache it like any asset; the same source + toolchain = the same hash |
| the **descriptor** (parameters, ports, ABI version, toolchain, the wasm's hash) | in the module document, written by a successful compile | the parameters tab and the validator read it without running anything |
| **native builds** of promoted modules | the deck image / `deck-update` bundles (deck/os), as today's features | built with the image, signed |
| instruments, presets, rigs, assignments | as now: catalog kinds + member rows (#258) | unchanged |

Not a git repo per module (heavy, a second history beside the documents), not files on the FTP hosts (not versioned, not reviewable).

## 8. Where the designer lives
Members write and submit, so the designer must be reachable by **members, not only creators** – the creator app is for the team. **Decided (owner 2026-11-02): one page in the viewer next to the browser deck** (`/live/designer`, the engine already runs there – live-core + space-web), as its own shared package (`@scutoons/rig-designer`) so the creator can mount the same component for reviewing and promoting. The creator keeps: the review queue (submitted work), promote, the character → rig mapping overview with the missing defaults, the catalog list (#269's pages trimmed to that). The deck stage gets the designer's Play + presets later as an overlay; editing code on a deck's screen is not a goal.

## 9. For the owner – open
1. ~~Where the designer lives~~ → **owner 2026-11-02: the viewer next to the browser deck, a shared package the creator mounts for review** (§8).
2. ~~The build service: where it runs~~ → **owner 2026-11-02: on the relay's server** – a second container in the bunny app `scutoons-live`, towards one pod for the whole platform later (§4: CPU, lifecycle and start-time rules).
3. **Who may compile:** every signed-in member (limits per user and day) or only members who own an instrument item / were invited (recommended first: creators + invited members, then open)?
4. **The standard fallback rig:** one platform rig every character falls back to, or one per instrument kind / per toon (recommended: one platform rig, a toon may set its own default)?
5. **Native tier:** every promoted module goes native with the next deck image (recommended), or only on demand?
6. **The SDK's first API** (§4): audio + control rate, params, ports, tempo, character, scene / world variables read-only, events out – enough for v1, or more (MIDI out, samples, the being's look directly)?
7. **Licence of submitted code:** a member's submitted module becomes the platform's to ship (owner's terms, credited) – the same rule as other submissions?

## 10. Order once §9 is answered (each an issue)
1. The SDK crate + the descriptor + one built-in rewritten as a module (proves the ABI); wasmtime in deck-stage, the wasm path in live-core.
2. The build service (sandbox, limits, errors as JSON) + the `module:<id>` document and `member_modules`.
3. The designer page: Rust tab + Compile + Play + the console; then Parameters; then the read-only Nodes view.
4. Assignment tab + the mapping overview (missing defaults, the fallback rig) + scene / world variables in ctx.
5. Submit → review → promote; the native tier in the deck image.
6. Node editing (#203), the stage overlay.
