Status 2026-11-16: concept for the owner's review – where a module can extend the core (camera, animation, devices, picture), attached wherever an entity lives; compared with OSCT's plugins; drafts sent in the chat. Nothing built from it.

# MODULE-EXTENSIONS – modules as the app's plugins

Owner, 2026-11-16: *"draw up the concept of how it is implemented and where i can turn or extend core functionality of camera, animations, devices for input or output with custom modules … the modules should be able to be injected into any part of the application where an entity resides, right? look at osct how we use plugins conceptionally there."*

## 1. The short answer

Yes – with one rule that keeps it from reaching too far: **the core decides where code may attach and what it may touch; a module never reaches into the engine.** An entity kind (character, item, location, scene, camera, show row, outlet / deck) declares **slots**; a module attaches to a slot of an entity; the host gives it what the slot allows to read (its `Ctx`) and lets it write only through its declared outputs, addressed inside that entity. That is what OSCT's plugins already do with `register*` per kind of thing, and what our ABI v2 already does for sound and rigs (ABI-V2.md §1–§5).

One honest limit: **hardware stays native.** A wasm module cannot open a USB port, a camera or a socket (no `fetch`, no raw frames – ABI-V2 §4.2, decided). Device *drivers* are adapters in the deck or an on-demand helper (decisions 2026-11-16 [inputs]); a module gets their data as sources and events, and drives outputs through output ports a device map routes.

## 2. OSCT and SCUTOONS side by side

| | OSCT plugins (staging, docs/plugins.md) | SCUTOONS modules (ABI v2, built) |
|---|---|---|
| language | TypeScript, transpiled in the browser | Rust → wasm, built by `deck/build` (locally in seconds #571, the service on publish) |
| entry | `register(api)` once | a descriptor (`#[derive(Module)]`) + `dm_new` / `dm_process` / `dm_control` |
| what it adds | `registerComponent` (behaviour), `registerNodeType` (object), `registerMaterial` / `registerLayer` (look), `registerTexture`, `registerPass` (post) | today: an **audio** node (instrument, effect), a **control** node (sequencer, LFO, mapping logic), a **rig policy** (#314) |
| reaches the scene | `api.three`, the bus (`api.events`, `api.vars`) | `Ctx` (times, tempo, character, variables, sensors), ports, the deck-events graph; writes only through outputs |
| sandbox | the stage iframe, no session, no foreign origins | wasm: no I/O, memory frozen, a time budget, a trap silences one node |
| rules vs code | `behaviour` rules first, a plugin for what a rule can't | the same split proposed: rows, filters and mappings are data; a module for what data can't say |
| "make it code" | the Inspector turns a node into a `registerNodeType` | proposed: **"make it a module"** – a built-in node / camera move / mapping becomes a module draft with the same params (#571's Module workspace opens it) |

## 3. The slots – where a module can go

Domains the engine runs (existing ✓, proposed ○):

| Domain | Runs | What a module there does | Example |
|---|---|---|---|
| ✓ audio | audio thread, per block | makes or changes sound | an instrument, an echo |
| ✓ control | control half, per block (~375/s) | computes values and events | a sequencer, an LFO, a rig policy |
| ○ motion | control half, writes transforms | moves an entity or a camera | a camera move, a dolly path, an animation, a wobble |
| ○ picture | render thread, per frame | shades pixels (a layer / pass), like OSCT's `registerLayer` / `registerPass` | a glitch over a video, film grain, a tint |
| ○ object | render thread | builds geometry procedurally, like `registerNodeType` | particles, a ribbon, generated plants |
| ○ device codec | control half | turns raw device data into sources, or sources into a device's protocol | a sysex decoder, a lamp's DMX personality, a controller's LED feedback |

Slots per entity kind (what can attach there):

| Entity | Slots | It may write |
|---|---|---|
| character / persona / look | rig (audio + control), motion (animation), picture (look layer) | its rig's paths, its being's transforms and moods |
| item | motion, picture, object | its node |
| location / set | picture (pass), object | its set's nodes |
| scene | motion (cameras, cast), picture (pass), control (scene logic) | its holders, cameras, variables |
| camera (#582) | motion (a move), control (a shot picker) | the camera's position, target, lens, projection |
| show row / cell | control (when / what to fire, a value curve) | the cell's values, events on the bus |
| outlet / deck | device codec (in and out), picture (the outlet's pass) | its sources, its output ports |
| input / output device (#468) | device codec | sources / the device's output port |

The ABI change is small: the descriptor gets a `slot` (the entity kind + domain it is built for) and, for motion and picture, two new regions (a transform table, a pixel-shader stage). Motion is control-domain with transform outputs – no new thread. Picture is the one real addition (a WGSL fragment on the render thread, checked and compiled by the core, like OSCT's GLSL layers); it waits for the owner's word.

## 4. How it feels in the editor

- On any entity's inspector, a **"+ module"** row lists the slots that entity has and the modules that fit (the library browser #580, filtered by slot). Drag a module onto the entity on the stage = the same drop interaction as everything else.
- **"Make it a module"** on a built-in (a camera move, a mapping curve, a node) starts a module draft with the same parameters, in the Module workspace.
- The **Module workspace** (#571, redesign drafted): the file tree and a read-mostly code view with syntax colouring and problems on their lines; "open in VS Code" for real editing (decided: external editor, deck-lsp); the descriptor as a panel (slot, ports, params, capabilities); build and audition; where the module is used.

## 5. What stays out

- Modules creating or removing entities or tracks at run time (graph from the document; ABI-V2 §5.2 rejected it).
- Network, files, raw camera frames, the user's session – never from a module.
- UI written by modules – the editor draws a module's parameters from its descriptor; a module doesn't draw panels.

## 6. For the owner to decide
1. The slot model as above (entity kinds declare slots; modules attach; writes only inside the entity). Recommended: yes.
2. Order of the new domains: **motion first** (cameras #582, animation), then **device codec** (#468), then **picture**, then **object**. Recommended.
3. "Make it a module" from built-ins. Recommended: yes, after motion.
4. Picture modules in WGSL (the core's renderer language) rather than a second shader language. Recommended.
