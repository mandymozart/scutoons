Draft for owner review, 2026-11-03 – the design of module ABI v2 (#323, with #312 sensors, #314 groups, #289 control outputs). Documents only; nothing is built. Read against `main` at b504d5f6 (the engine move, #317). The build tickets of §9 start only on the owner's word.

# ABI-V2 – one module ABI for notes, channels, sensors, assets and groups

Binding, not argued here (DECISIONS 2026-11-02, research/ENGINE-REFACTOR-PLAN.md §2.0, §3.6, ENGINE-ALIGNMENT.md §0): modules are universal Rust/wasm leaves; ABI v2 is the one breaking change and **replaces v1** (no compatibility code: stored modules are recompiled, the host reads v2 only); a module never calls a platform API; module → instrument → rig are layers of one mechanism, and a rig is a module group, written as data or as Rust code ("of course it is just another layer after the instruments. rigs are module groups.").

## 0. The design in one page

**Central call: the module's memory layout is derived from its descriptor, not frozen in a fixed `Io` struct.** One pure function, `Layout::of(&Descriptor)`, shared by the SDK (a `const` evaluation in the derive) and the host, places a small frozen header and a handful of regions (params, control, audio, vars, sensors, assets, events, targets) back to back: one contiguous `inp` block the host writes and one `out` block it reads per call. Every v1 limit (4 stereo ports, 8 controls, 32 variables, 64 events, 1024-frame buffers, 4 prop keys) exists because `Io` is one fixed `repr(C)` struct (`abi.rs` constants); with descriptor-derived regions a port, parameter, event port or sensor is *a descriptor field and a region*, never a change to the ABI. The checks stay: a module exports `dm_layout()` (a hash of its layout) and the host refuses a mismatch.

What follows from it, each in its section:
1. **Ports** – audio ports carry 1…32 planar channels (`any` follows the cable), control ports and **control outputs** are block-rate `f32` with a "written" bit, **event ports in and out** carry a 40-byte `Ev` (note on/off with ids and velocity, or any named deck-events event with positional props), **assets** are read-only slots the host fills at instantiation. All of it is declared in the one `deck_catalog::Descriptor` (#315, built) that the built-in kinds use.
2. **Parameters** – a flat `f32` table with a dirty bitmap, no small cap, units / choices / docs in the descriptor, **factory presets promoted into the descriptor**; the host clamps, the module smooths.
3. **Ctx** – the show's three times, tempo, the character (strings + mood index + energy / touch / strain), **declared variable slots**, a derived seed; reads only; writes only through outputs.
4. **Capabilities** – `needs` (hard) and `wants` (soft) sensor services; the core owns the permission state machine and the sensor channel table; recorded sensor logs make runs reproducible.
5. **Groups** – a code rig is the declarative rig document **plus a control-domain policy module** whose control outputs carry a `path` (a child's parameter, preset, gain, pan, mute); the declarative rig is the same policy written as data (a built-in kind).
6. **Safety** – libm in the SDK, a trapping allocator while strict, memory frozen after init, a per-call time budget, a trap silences one node (outputs zero, targets freeze).
7. **Versioning** – `deck.module` section, `abi: 2`, three modules recompiled, the TS host and `module-abi.ts` deleted with them.

## 0a. ABI v1 as it is (verified in the code, 2026-11-03)

| Fact | Where |
|---|---|
| `Io` is one `repr(C)` struct, ≈ 72 KB: audio `[[[f32; 1024]; 2]; 4]` each way (32 KB each), 8 control in / 8 out, 32 vars with 24-byte keys, 64 `EventRec` of 88 bytes (string keys, 4 props), three 32-byte character strings | `deck/module/src/abi.rs` (consts at l. 11–30, `In`, `Out`) |
| Calls are ≤ 1024 frames; the engine renders fixed **128-frame** quanta (#317) | `abi.rs` `MAX_BLOCK`; plan §3.3 |
| Every field is written by a separate backend `write` (six for the tempo alone, two per audio port); a parameter change is a wasm call (`dm_set_param`) with its own `set_budget(50)` | `deck/module-host/src/lib.rs` `set_tempo`, `set_audio_in`, `set_param` (l. 337–399) |
| `#[port(input, event)]` is refused ("events only go out of a module (v1)"); events are emitted but **nothing consumes them** | `deck/module-macros/src/lib.rs` l. 110; `deck/module/README.md` "Hosts" |
| The host **refuses** control outputs and video ports | `module-host` `Layout::of` l. 125–135 (the ABI itself has `MAX_CONTROL_OUT` and `Out::control`) |
| `dm_control` ("control tick ~60 Hz on the frame thread") exists in the ABI and `Instance::control`, but **nothing calls it**: the engine's module node drives `process` only | grep `\.control(` outside `deck/module*`: no caller; `deck/engine/src/module.rs` l. 142–182 |
| The engine's `ModuleInstance` seam passes audio as stereo `(l, r)` slices | `deck/engine/src/module.rs` l. 22–33 |
| Time budget `0.75 × block`, clamped 2…50 ms, wasmtime epoch interruption (1 ms ticker); memory capped 64 MiB; one import `env.deck_log` | `module-host` `budget` l. 415, `MEMORY_LIMIT`, `wasmtime_backend.rs` |
| Two descriptor types: `deck_module::host::Descriptor` (name, min, max, default; port `kind`) and `deck_catalog::Descriptor` (+ channels, curve, unit, choices, doc, `needs`, `abi`; a v1 JSON parses into it) | `deck/module/src/host.rs`; `deck/catalog/src/lib.rs` l. 120–199 |
| Analog's two limits: no note-event input (notes as four cv lanes + a `sustain` control), a 32-parameter cap raised to 128 (`MAX_PARAMS`); its units / docs / presets ride in **extra** sections `deck.params`, `deck.presets` | `deck/modules/analog/README.md`, `src/lib.rs` l. 24–42 |
| The module event model is not deck-events': props are 4 with 8-byte string keys (deck-events: 6 props keyed by interned `Sym`), times are beats only | `abi.rs` `EventRec`; `deck/events/src/event.rs` l. 15–84 |
| TS host `shared/live-core/src/modules.ts` is imported by its own test only; the test loads `walk_module.wasm` and `fixture_module.wasm` **from the workspace build** | `shared/live-core/test/modules.test.ts` l. 12–15 |

## 1. Ports

### 1.1 The memory model (the part everything else hangs on)
```
inp (host writes, one copy)            out (module writes, host reads, one copy)
  InHdr  – frozen, ~256 B                OutHdr – event counts, silent flags, alloc counter
  params f32[P] + dirty u32[⌈P/32⌉]      control_out f32[Co] + set-bits
  control_in f32[Ci]                     audio_out  per port: ch × 128 f32, planar
  audio_in   per port: ch × 128 f32      events_out per event port: Ev[cap]
  vars f32[V]   sensors {status,f32[n]}
  assets AssetInfo[A]   events_in Ev[cap]
```
- Regions are 16-byte aligned; sizes come from the descriptor only. A stereo synth is ≈ 5 KB each way (v1 ≈ 72 KB); 32 channels × 128 × 4 B is 16 KB per port. `frames` ≤ **128** (the engine quantum); a smaller value is for tests.
- **`any` channels:** the layout reserves the declared maximum (≤ 32); the *actual* count per port is a header byte (`ch_in[p]`, `ch_out[p]`) the engine sets at build from the linked cables (plan §2.6: the widest input; an `any` output follows the widest input). The layout therefore never depends on the graph, so it can be hashed and tested.
- **Exports:** `dm_abi() -> 2`, `dm_io() -> u32`, `dm_layout() -> u32` (hash), `dm_new()`, `dm_process()` **or** `dm_control()` (the one matching the descriptor's `domain`; `Audio` runs in the audio half, `Control` in the control half – plan §2.3), `dm_asset_buf(slot, len) -> u32` (only when the descriptor declares assets), `memory`. Gone: `dm_set_param`, the other of process / control.
- **Order inside a block:** control half (all control-domain modules, in the control graph's level order) → audio half (audio modules in graph order, events delivered). A control output or target written in block *k* applies at block *k+1* on every target (plan §2.6a: a control loop is always one block late), so order never changes the sound.

Rejected: *a fixed bigger `Io`* (32 channels, 512 events …) – it works today but each of sensors, assets, groups, event props would again move `audio`, which is exactly the plan's reason for doing v2 "once"; it also costs 6–10× the copy per block. *Per-field backend writes as today* – ten-plus JS boundary calls per block in the browser backend; one copy each way is what makes the `web` backend cheap (#306).
Cost: SDK L (const layout in the derive, one `Layout` module shared with the host, a layout-hash test), host M (rewrite `Instance` around two copies), engine adapter M. The `Backend` trait is **unchanged** (`write` / `read` / `call` / `set_budget` / `memory_size`), so wasmtime, web and wasmi backends need no work.

### 1.2 How the descriptor declares each port
One shape, `deck_catalog::Descriptor`, for built-ins and modules (plan §2.4). Fields v2 **adds** to what exists (`name, version, domain, doc, feature, scope, needs, abi, ports[], params[]`), all `serde(default)`, so a built-in is unchanged:

| Addition | On | Meaning |
|---|---|---|
| `channels: 1…32 \| "any"` (exists) | audio / cv port | planar channel count; `sidechain: true` marks a second audio input fed by sends (plan §2.6) |
| `names: ["music.note"]`, `props: ["pitch","vel","glide"]` (≤ 6), `capacity: 64` (≤ 512) | `event` port | the deck-events names it emits / accepts and the positional meaning of the prop slots; the host interns them once at load (`Sym`), so the wire carries a 16-bit index, never a string |
| `path`, `mode: set\|add\|mul`, `choices` | control **output** | bind the output to a path (a group's target, §5) instead of a graph cable; `choices` names an enum (value = index) |
| `reads: ["story.act"]` | descriptor | the variable slots the module may read (§3) |
| `assets: [{name, kind: pcm\|bytes\|image, doc, max_bytes}]` | descriptor | read-only data slots (§1.5) |
| `wants: [...]` | descriptor | soft sensor needs (`needs` is the hard list, §4) |
| `presets: [{name, doc, values}]` | descriptor | factory presets (§2) |
| `latency: frames` | descriptor | constant latency for the engine's compensation (plan §2.6a); 0 default |

`deck_module::host::Descriptor` (v1's second type) is **deleted**; `deck_module::host` keeps `validate(&catalog::Descriptor)` and `read_descriptor`. The derive writes the JSON; the custom section is still **`deck.module`** (`SECTION`, `host.rs`). The macro never needs serde (it emits a string); the host feature depends on `deck-catalog`. `video` ports stay declarable and refused by the host with a message until the video leg (#185).

### 1.3 Audio ports
Planar `f32[ch][128]` per port, ports ≤ 8 per direction, ≤ 32 channels per port (plan §2.6's limit). Module code sees `block.audio_in(p) -> Planar` (`ch()`, `chan(i)`, `stereo()`), `audio_out(p) -> PlanarMut`. Inputs with nothing linked are zero-filled by the host; a header bit `in_silent[p]` lets a module early-out; a module may set `out_silent[p]` so the engine's silence flags skip the sum. A `cv` port (pitch Hz left, gate right – `seq`'s and `walk`'s convention, `deck_module::cv`) is a 2-channel audio port of `type: cv`, as the catalog already types it.
Rejected: *interleaved* (the engine pool is planar, channel maps are index copies – plan §2.6). *In-place buffers* (a module writing over its input breaks sidechains and the sum-at-input rule).
Cost: engine adapter copies N channels instead of `l, r` (a `memcpy` per channel; the pool is planar already); the `ModuleInstance` trait changes from `(l, r)` to channel slices. Until **#318** (N-channel pool) lands the adapter runs with 2 channels; the ABI is ready.

### 1.4 Control ports, control outputs (#289), events
- **Control in:** `f32` per block (the control half steps once per 128-frame block, plan §2.3). Every parameter is *also* a control input as in v1 (`ParamDesc.port`, catalog); the `sustain`-style ports of v1 stay. Audio-rate modulation is an audio / cv port, not a control port.
- **Control out (#289, designed once for sensors and rigs as the ticket's comment asks):** `control_out[i]` plus a *written* bit. Unwritten = the consumer keeps the last value (hold). A port without `path` is a graph cable (a normal link target, incl. `being:*` through the rig's routes); a port **with** `path` is a target (§5). The engine work of #289 (host refuses today, `Layout::of`) becomes: route the cable ports as graph outputs of type control, apply the path ports in the control half. Same bytes either way.
- **Events in and out** – the record (40 bytes, little-endian, in the port's region, ≤ `capacity` per block, sorted by `frame`, stable):
```rust
#[repr(C)] pub struct Ev {
    pub frame: u32,      // 0..frames-1: the sample in this block
    pub kind: u8,        // EVENT 0 · NOTE_ON 1 · NOTE_OFF 2 · NOTE_EXPR 3 · RESET 4
    pub port: u8,        // the declared event port
    pub name: u16,       // EVENT: index into the port's `names`
    pub id: u32,         // NOTE_*: the note's id (host-assigned on input, module-assigned on output)
    pub len: f32,        // beats; 0 = a moment / until NOTE_OFF  (outputs only; inputs are expanded by the host)
    pub v: [f32; 6],     // NOTE_ON: pitch Hz, velocity 0..1, glide 0..1, key (MIDI key, fractional) · NOTE_EXPR: which (0 bend semitones, 1 pressure, 2 slide), value
}                        //   EVENT: the port's `props` in order; NaN = absent
```
  This **is deck-events' `Event`** (start, dur, name, ≤ 6 props – `events/src/event.rs`) minus what must never touch the audio thread (children, `data`, hops, `source`). **Notes are MIDI-like**: note on / off with a velocity, any number per block, a per-note expression, an all-notes-off `RESET` (the panic of plan §2.6b); the *id* pairs on with off so polyphony never matches pitches (CLAP's note id, taken as a specification). **Pitch is both, always filled (owner 2026-11-03: "i need both in my final system … depending on the instrument design")**: a note carries `pitch` in Hz (`v[0]`) and `key` as a MIDI key number, fractional (`v[3]`; 60.0 = middle C, 60.5 = a quarter tone up). Which one is the *truth* depends on the source, and the **core's event host fills the other** once, so no module converts: a MIDI source (keys, a drum pad – key 36 is a kick, not a frequency) gives the exact key and the Hz derive through the deck's tuning (A4 reference and tuning table, a deck setting); a Hz source (walk, a continuous glide, a microtonal generator) gives the Hz and the key is `69 + 12·log2(Hz / A4)`, fractional, via libm. A module reads whichever fits its design: a continuous synth or an fm voice reads Hz, a sampler / drum kit / key-mapped instrument reads the key. On output a module sets one or both (NaN = absent) and the event host completes the pair before routing, so a MIDI-only listener and a Hz-only listener both work downstream. **The host expands durations on input**: a deck-events note with `dur` reaches a module as `NOTE_ON` + a later `NOTE_OFF`; a module never counts beats to end a note. On output a module may emit either form; the engine's event host turns `len > 0` / `NOTE_OFF` into deck-events events with `dur` and routes them through the existing graph (the generators' path, `deck/events/src/generator.rs`), mapping `frame` → beats with the block's tempo map.
  Late events (their time already past) are clamped to frame 0 and counted; overflow beyond `capacity` is dropped and counted (`OutHdr.dropped[port]`, `InHdr.late`), both into ModuleStats (§6).
Rejected: *a free-form event struct with string names and keys* (v1's `EventRec`: 88 bytes, string compares on the audio thread). *MIDI bytes only* (no names, no props – our `story.*` / `deck.*` events and the `walk` glide would not fit). *Notes as cv lanes* (v1's analog: four lanes, a ninth note steals; no velocity per event, no per-note expression).
Cost: SDK M (`Ev`, `Block::events_in/out`, `cv::GateCv` rebuilt on events), engine M (an event host that places deck-events events on frames in a block – plan §3.3 already requires "events land on their sample inside a block"), host S.

### 1.5 Asset handles (plan §2.9)
A module declares `assets: [{ name: "kick", kind: "pcm", max_bytes: 8 << 20 }]`. The document's node params bind an *asset id* per slot; the asset manifest resolves it; **deck-media decodes**; the host hands over **read-only** data and the module never fetches or decodes (fixed by plan §2.9).
- **Mechanism, at instantiation only (off the audio thread):** after `dm_new` the host calls `dm_asset_buf(slot, len) -> ptr` (the module allocates once, in its own heap), writes the bytes there, and fills `assets[slot] = { ptr, len, kind, channels, frames, rate, ready }` in the header. PCM is planar `f32` per channel at the asset's rate (resampling is the module's, deterministic); `bytes` is raw (a MIDI file, a sequence – parsed by the module's Rust or by the core before); `image` is RGBA8, small. `ctx.asset(Slot) -> Option<Asset>`; `None` = not ready → the module plays silence, never waits.
- **Immutable per instance.** A different asset (a mood picks another sample) is a different slot (declare both) or a new instance (scene or rig switch, which the stage already crossfades – plan §2.6b). Reason: a swap during playback would allocate or copy megabytes on the audio thread, and `memory.grow` is frozen after init (§6).
Rejected: *an `env.deck_asset_read` import* (a second import every backend implements and the web backend crosses JS for; plan §0 keeps the import list at `deck_log`); *streaming windows* (a state machine for no use case yet); *module-side loading from an id* (breaks "a module never fetches").
Cost: host M (asset binding, `ready`), engine/deck-media S–M (the manifest → bytes call, after P13b); **can ship later than the rest** – the region and the slot table exist from day one, `assets: []` means no region.

## 2. Parameters

- **A flat `f32` table** `params[P]` in the `inp` block, **P ≤ 1024** (a sanity bound, not a design cap: 91 for analog today; v1's `MAX_PARAMS = 128`). Index = declaration order. The host **clamps** to the descriptor range and ignores NaN (keeps the last good value), so the SDK needs no `dm_set_param` call; `dirty[⌈P/32⌉]` marks what changed since the previous call. The derive keeps `self.amount` ergonomic: before each call it copies dirty entries into the fields, and `Module::params_changed(&mut self, dirty: &Dirty)` (optional) lets a module recompute coefficients only for what moved.
- **Descriptor per parameter** (already in the catalog `ParamDesc`): `min, max, default, curve` (lin / log), `unit`, `choices` (an enum: the value is the index, `min 0 … max n-1` implied – analog's `o1_wave: saw|pulse|triangle|sine`), `port` (also a control input), `doc`. The derive reads them from attributes: `#[param(range = 20.0..20000.0, default = 800.0, curve = log, unit = "Hz", doc = "…")]`, `#[param(choices = "saw|pulse|triangle|sine", default = 0)]`. This **kills analog's `PARAMS_JSON` / `deck.params` hack** (a `macro_rules!` that concatenates JSON beside the derive): the table macro feeds attributes to the one derive.
- **Smoothing and automation: the host delivers one value per block and does not smooth; the module smooths.** A filter cutoff wants log-domain glide, a gain linear, a switch none – that is DSP, and a host-side smoother would need a per-sample buffer per parameter (memory and a second source of block-cut dependence, plan §1.2). SDK `Smooth`/`Ramp` helpers (the deck-core `Ramp` semantics) are provided. Sample-accurate modulation is an audio / cv port. *Automation* (LFO, mapping, mood diff) is the host's job: it writes the table, so every producer – a link, a MIDI CC mapping, a preset, a group target – ends in the same entry. Rejected: *a-rate parameter buffers* (cost, and cables already do it); *host smoothing with a declared time* (the block-cut dependence the plan removes).
- **Presets as data → promote into the descriptor.** `presets: [{ "name": "pad", "doc": "…", "values": { "o1_wave": 3 } }]` replaces analog's `deck.presets` section. Applying one is **one host function** (`Preset::apply`: all parameters to default, then the listed values → write the table; exactly what analog's tests do by hand, README "Presets") so rigs and the creator share it; a code rig selects one by writing a `@preset` target (§5). Why in the descriptor and not as separate `preset:<id>` documents: the module ships self-contained and testable (the build service runs it), the creator can *import* a factory preset as a derivable document later without the module depending on the catalog. Rejected: *keeping extra custom sections* (the brief's one-section rule, and a second parser per host).
- Cost: SDK M, descriptor S (the catalog already has the fields), analog S (rows move to attributes), `schema.json` / `kinds.json` regenerate (additive).

## 3. `Ctx` and the globals

Read-only, written once per block by the host into `InHdr`. What a module may read:

| Group | Fields | Notes |
|---|---|---|
| **Time (plan §2.8)** | `frame: f64` (sample frame on the tempo map's axis, as v1), `frames`, `rate`, **`show_ms: f64`** + `show_valid` (the relay-mapped show time at the block start, #321) | three times, no wall clock; a module never reads a platform clock |
| **Tempo** | anchor `(tempo_frame, tempo_beat, bpm, per_bar, running)` as v1; `beat_at`, `frame_of`, `beats()`, `phase()`, plus `bar()` | deck-core's `TempoMap` formulas (`ctx.rs`), unchanged: analog's LFO sync and walk keep working |
| **Seed** | `seed: u64` | see below |
| **Character (#314)** | `id`, `persona`, `mood` (strings), `mood_index: u32` (the position in the scene's `moods` registry), **`energy`, `touch`, `strain`** (0…1, the being's `Senses`: `deck/space/src/space/feel.rs` l. 207–217) | strings stay for readability; the index makes a switch cheap; more senses are new header fields in a future ABI, not a runtime list |
| **Variables** | `ctx.var(Var) -> Option<f32>` | **declared slots**, below |
| **Sensors, assets** | §4, §1.5 | |

- **Variables by declared slot.** v1 hands 32 `(key, value)` pairs with 24-byte keys and `var()` scans them by string every call (`ctx.rs`). v2: the module lists the keys it reads in the descriptor (`reads: ["story.act", "being.glow"]`, derive attribute `#[reads("story.act")]` → an associated const `Self::STORY_ACT`); the host resolves keys → slots at load; the header holds `vars[V]` (V ≤ 256) with **NaN = not set**. Gains: no strings on the audio thread, no cap that matters, and the descriptor now says what a module depends on (the editor, #313's validator and the bake #296 can list it). Rejected: *all variables, always* (hidden dependency, bake cannot fold, the copy grows with the world).
- **Seed rule.** v1 default seed is `1` for every node (`deck/engine/src/module.rs` l. 103–108): two `walk`s in one rig play the identical melody. v2: the engine derives `seed = splitmix(show_seed ⊕ fnv1a(node path))` unless the node's `seed` param is set; the show seed is a scene property (default 0). Identical on every deck for the same document – the determinism rule is kept, correlation is gone. Behaviour change: a rig that relied on equal seeds sets `seed` explicitly (none does today: walk's fixtures name theirs). Anything random in a module draws from this seed (`walk` uses splitmix64 keyed by `(seed, step)` – the model).
- **What a module may write:** audio outputs, event outputs, control outputs (cable or `path`), nothing else – no global, no variable, no file, no network. A module that should move a *world variable* is wired with a control output into the graph/route that owns it (the host applies it); writing variables from modules would create hidden cross-module state and order dependence (rejected, as RIG-EDITING §4 already says "writing back happens through declared outputs").
- Cost: SDK S (accessors), engine S (fill the header; the beat reader and character are already per node), host S.

## 4. Capabilities and sensors (#312)

### 4.1 Declaration
`needs: ["motion"]` (**hard**: without it the module cannot do its job) and `wants: ["position"]` (**soft**: it degrades). Vocabulary closed per ABI version, each a typed channel group read from `Ctx` – never a browser API:

| Name | Channels (f32, block-rate, last sample at the block start) | Source |
|---|---|---|
| `motion` | accel x y z (m/s², gravity included), rotation rate x y z (rad/s), orientation quaternion x y z w | DeviceMotion / native IMU |
| `position` | lat °, lon °, accuracy m, heading °, speed m/s; **coarse (rounded to ≈ 100 m) by default**, `position.precise` is a separate name the audience sees | Geolocation / native GPS |
| `camera.face` | 52 blendshapes 0…1 + head yaw, pitch, roll | face model **in the core** on camera frames (raw frames never leave the adapter) |
| `camera.motion` | motion energy 0…1, centroid x y, mean luma | frame analysis in the core |

`midi_in`, audio in (mic, line) are **not** needs: MIDI arrives as events routed to an event input port by the deck-events graph (the adapter is Web MIDI or a native port, outside the module), and audio input is an `in` node on an audio port (plan §2.6). Sensors are reserved for *continuous channels read directly from `Ctx`*.

### 4.2 Core, adapter, permission
- **The core owns** a sensor channel table in deck-engine: adapters push `(channel, stamp_frame, value)` into a preallocated ring (stamped into sample time, plan §2.8 "Image → sound"); at each block start the engine resolves the latest sample ≤ block start into the module's region. The *resolution rule* is core logic (identical on every target); the adapter only moves bytes.
- **Status per need** in the module's `sensors` region (u32 before the values): `0 unavailable` (platform lacks it) · `1 asking` · `2 denied` · `3 live` · `4 replay` (a recorded log drives it). Values are zero unless 3 / 4.
- **Enabling is on need** (#312 item 3): the core knows the loaded scene's `needs` ∪ `wants`, tells the adapter `Request(service)`, the adapter shows the platform prompt **at the next user gesture** (the deck's Start tap; browsers forbid it earlier) and answers granted / denied; nothing is on by default, and the creator's scene editor shows what a scene will ask for before it can be published (the audience-facing consent line of #312 item 4 – *proposal, owner to confirm*, §10 Q4).
- **Not available:** `needs` missing → the module **is not run** (its node is silent, a typed report `unavailable: motion` goes to the strip / Nodes tab and the conductor's deck report, plan §2.6b). `wants` missing → runs with status ≠ 3 and zeros; the module must handle it (the example in §8c). A headless deck, Safari without Web MIDI, a laptop without an IMU: reported, not worked around; a device that needs a sensor gets a native build with the adapter (DECISIONS 2026-11-02).
- **Privacy:** processed on the device; the camera features are computed in the core and only the feature vector reaches a module; nothing leaves the device unless the user starts a stream. Rejected: *raw frames to modules* (privacy, size, and a GPU in wasm); *a module-callable `fetch`* (breaks universality); *one `sensor(name) -> f32` string lookup* (strings on the audio thread; the closed vocabulary also makes the permission list finite).

### 4.3 Determinism and conformance
A sensor is the one non-deterministic input, so **the log is the input**: a fixture `*.sensorlog` (JSON lines `{frame, channel, value}`, content-hashed like any fixture asset, plan §2.9) is replayed with status 4. The conformance suite (#305) renders a sensor module from a log on every backend and target and compares hashes (native ≡ wasm ≡ wasm + simd128, plan §3.4); a second run with no log asserts the `unavailable` path. The creator can *record* a log from a phone and attach it to a module as its test input. What the browser lacks: DeviceMotion needs the permission call on iOS and a secure context; Geolocation needs a secure context; the camera model needs `getUserMedia` and a gesture; none exists in a worker, so the **adapters live in the page's TS** (≈ one file per service, no logic) and push into the worker's ring (`live-core`), exactly like the relay bytes. Cost: core S–M (table, status, ring, log replay), adapters M (web), native adapters later on need.

## 5. Module groups and code rigs (#314)

### 5.1 What a rig is, in the code that exists
`Rig { subject, tracks[{id, instrument, preset, gain, pan, mute}], routes[{from, to, scale, offset}], personas, moods }`, with `Variant { presets: track → preset, values: "<track>:<node>.<port>" → f32 }` (`deck/space/src/rig/mod.rs` l. 91–187). A rig's *behaviour* is therefore a small set of **verbs on paths**: set a parameter, select a preset, set a strip's gain / pan / mute, change a route's scale / offset. The declarative rig applies a `Variant` by persona then mood; a code rig computes the same verbs.

### 5.2 Design: a policy node over the data tracks
- **Children stay data.** A rig document keeps its tracks, instruments and routes exactly as today (so the validator #313, the bake #296 and the editor treat every rig alike). A **code rig adds one field, `policy: module:<id>`**; the declarative rig has the built-in kind `rig_table` in that place (it interprets `personas` / `moods`). Both are *group policy nodes*: control-domain nodes of the rig's graph. "The declarative rig is the same shape without code" becomes literal – same node slot, same verbs, two implementations.
- **Addressing:** a policy's control outputs carry a `path` in the descriptor (§1.2) in today's grammar `<track>:<node>.<port>`, extended with reserved members: `<track>:@preset` (value = index into `choices`, applied on a change), `<track>:@gain | @pan | @mute` (the track's `bus`, plan §2.6), `route.<i>:@scale | @offset`. Dense ids come for free: the output's position *is* the id; the host resolves each path against the actual children **at load/bind** (names → node index, parameter index, preset index). An unresolved path is a validation error from the core ("rig catpope drives `lead:analog.f1_cutoff` but 'lead' has no such parameter"), not a runtime miss.
- **Driving:** the module writes `control_out[i]` (+ written bit) in its `control` call; the engine applies written path outputs at the next block start (`mode: set` = the engine's `set_base` semantic, `add` / `mul` = a nudge on top of the base – the `[[being.react]]` model, `feel.rs` header). Preset selection is the host's `Preset::apply` (§2), not 60 parameter writes. **Reading** children is explicit data flow: a policy declares control *inputs* linked from child outputs (a meter, an envelope), so there is no hidden bidirectional access and no instantaneous loop (one block late, plan §2.6a).
- **Domain & rate:** `domain: control` ⇒ only `dm_control` runs, once per 128-frame block in the control half (≈ 375/s) – cheaper than an audio module because there are no audio regions to copy.
- **Nesting:** a track's instrument can itself contain a policy (an instrument is a graph of modules; plan §2.7); paths address descendants by the same grammar, so a rig of rigs is a policy over policies.
- **Budget and stats (#307) per group:** the policy is a node with its own `ModuleStats`; children are nodes with theirs. The *rollup* (track, rig) is a sum by node-path prefix done by the engine's stats reader / the editor – no group-level trap (silencing a whole rig for one slow track is too coarse). The creator shows "rig: 4.1 % of the quantum, policy 0.2 %".
- **Baked rigs (#296):** a bake folds the *data* (track values, `rig_table` diffs) and leaves the policy node as a node – the plan's risk 10 settled: nothing in a code rig is folded away.
- **Not allowed:** a policy creating or removing tracks at run time (the graph is built from the document; run-time mutation would break validation, bake and the one-block-late rule). Rejected as the alternative: *the module owns its children* (spawns instruments itself) – every layer stops being data, §10 Q3 (decided).
- Cost: descriptor S, derive S (`path`, `choices`, `mode` attrs), host S (resolution at bind), engine M (apply written path outputs, `Preset::apply`), `deck-space` M (rig doc `policy` field, check path resolution, `rig_table` kind – shared with #313/#322), editor later (#283's Assignment tab shows code or form).

## 6. Determinism, budget, safety

- **Determinism rule for modules (extends plan §3.4):** native ≡ wasm bit for bit. Enforced by (1) the SDK exports `deck_module::math` = the pure-Rust `libm` crate (`sin, cos, exp, exp2, ln, log2, pow, tan, tanh …`) – the one dependency the plan allows; analog's hand-written `dsp::math` (`exp2, log2, sin, tan` from `+ − × ÷ floor`) may stay or move; (2) the **build service writes a `clippy.toml` with `disallowed-methods`** (`f32::sin`, `exp`, `powf`, `ln`, `tan` …, same for f64) – a *warning* shown in the editor's Problems panel, because wasm32 `std` links a deterministic libm but the native `testing::Rig` uses the platform's; (3) the backend config: wasmtime `cranelift_nan_canonicalization(true)`, relaxed-SIMD, threads and shared memory **off** (a module using them fails to compile at load); (4) every random source seeded from `ctx.seed()` (§3); (5) the conformance suite runs every fixture on every backend and compares hashes, with the engine's block patterns 128 / 37 / 1024 (a module's output must not depend on how its caller cuts blocks – analog already tests this).
- **Allocation-free audio path.** The SDK installs a global allocator that **counts** every allocation made inside `dm_process` / `dm_control` into `OutHdr.allocs`, and **traps** when the header's `strict` flag is set. The host sets `strict` in the build service's tests and in the creator's Play (so a creator sees "allocated in process" with the call site, instead of v1's README rule plus a native `no_alloc` test per module), and clears it in a show (a stray one-time allocation must not silence a rig mid-show; the counter reports it). `memory.grow` after init is refused (below), which turns a runaway allocation into a failed allocation, never into an `mmap` on the audio thread.
- **Memory:** 64 MiB hard cap as v1 (`MEMORY_LIMIT`); the host **freezes the limit at the memory size after `dm_new` and the asset load**, so nothing grows while playing. Assets count against it. A descriptor `max_bytes` that cannot fit is refused at load.
- **Time budget:** per call `0.75 × quantum` (2.0 ms at 128 / 48 kHz; the wasmtime epoch ticker is 1 ms, hence v1's clamp ≥ 2 ms), identical for both domains; a hard overrun traps the call. The deterministic unit (fuel / gas) arrives with #306's decision; the stats record wall µs everywhere and gas where the backend has it. Independent of that, the engine counts *soft* overruns (> 50 % of the quantum) so a creator sees a module getting expensive before it silences.
- **What a trap does:** the instance is *silenced*: audio outputs zero, event outputs empty, **control outputs and path targets freeze at their last value** (a rig that traps leaves the sound as it is; zeroing would jump gains and presets), one report (stats `state`, the Nodes tab's red badge, the last message on hover, the deck's HUD). No automatic retry – its timing would be wall-clock dependent; the creator reloads Play, a show reloads the scene. A bad descriptor, a missing export, an import beyond `env.deck_log`, `dm_layout()` mismatch → refused at load with a message (v1's checks, kept: `Loaded::load`).
- **ModuleStats (#307).** A fixed-size struct per instance in `deck-module-host`, updated by the engine around each call (two `clock_us` reads, as `Instance::call` does today): calls, µs sum / max, a log2 histogram of % budget, soft overruns, gas, memory pages and peak, **events in / out / late / dropped, allocs in audio, written targets**, log lines and dropped, traps, state. No allocation; the browser mirrors it as `lc_module_stats()` (#307) with slot offsets generated for TS – the generator stays, `module-abi.ts` (module I/O offsets) goes (§7). The ABI contributes only `OutHdr.allocs` and the dropped / late counters; the clock stays on the host's side.

## 7. Versioning and migration

- **One section, one number.** `deck.module` carries `abi: 2`; `ABI_VERSION = 2`; `dm_abi()` returns 2; `Descriptor::validate` and `Instance::new` already refuse any other number (`host.rs` `validate`, `module-host` `new`) – no change in kind. No minor versions: a header change is v3 and a recompile list. "Room for later" is *additive by construction*: a new capability is a new optional descriptor field plus a new region, and a host that does not know a `needs` / field refuses the module with a message (the capability model of §4). The header itself is frozen.
- **Stored modules** are recompiled by the build service (`module:<id>` documents keep their sources; the compiled `.wasm` on the CDN is named by hash, so a new build is a new path – plan RIG-EDITING §7). A one-time command: `rebuild --all` over the catalog's modules, **the failures are the list** (sources using removed v1 API). No dual-ABI host.
- **The three repository modules** are the migration list (ticket A6): **`walk`** – `cv` stays a 2-channel `cv` port, `notes` becomes a v2 event output with `names = ["music.note"]`, `props = ["pitch","vel","glide"]`, `GateCv::render` takes `&[Ev]`; its fixtures `fixtures/walk.json` regenerate (the *musical* output is unchanged, which is the proof). **`fixture`** – rewritten as the conformance fixture of v2: audio in / out with 2 and 5 channels, an event in and out port, a path control output, `needs`/`wants`, an asset slot, and the hostile modes (panic, runaway loop, alloc in process, memory growth). **`analog`** – `cv1…cv4` → one `notes` event input; `sustain` stays a control input (or a CC event); `volume…` rows become attributes (the `deck.params` / `deck.presets` sections disappear into the descriptor, §2); `MAX_PARAMS` irrelevant; the `Engine` splits its render at event frames (its modulation is block-cut independent already, README "The DSP"), so its tests and the bit-equality native ≡ wasm carry over; `tests/common` moves to the v2 `testing::Rig`.
- **TypeScript.** `shared/live-core/src/module-abi.ts` is *generated from `Offsets::get()`* and `modules.ts` + `test/modules.test.ts` exist for the browser before a Rust backend does (§0a, last row). In v2 offsets depend on the descriptor, so a static generated TS table no longer exists – and the browser backend (#290) is Rust calling four JS functions with no offsets in TS. **Recommendation: delete `modules.ts`, `module-abi.ts`, their test and the `build:modules` script in A6**, not at P9b: the test builds `walk_module.wasm` / `fixture_module.wasm` from the workspace, which are v2 the moment A6 lands, so freezing the TS host would need checked-in v1 binaries to guard a host nothing imports (owner 2026-11-03: no backwards compatibility). `module_abi_ts_is_current` and `the_io_layout_is_what_the_host_computes` (`deck/module/tests/sdk.rs`) are replaced by a layout-hash test and a host/SDK layout equality test over all fixture descriptors.
- **Docs:** `deck/module/README.md` is rewritten in A9 (it says v1 throughout); RIG-EDITING §4's SDK sketch gets a pointer to it.

## 8. Examples (SDK sketches, v2 style)

### 8a. The analog synth with a note-event input
```rust
#![allow(clippy::duplicated_attributes)]
use deck_module::{Block, Ctx, Kind, Module};

#[derive(Module)]
#[module(name = "analog", domain = "audio")]
#[port(input, event, name = "notes", names = "music.note", capacity = 64)]   // was cv1…cv4
#[port(input, control, name = "sustain")]
#[port(output, audio, name = "out", channels = 2)]
pub struct Analog {
    #[param(range = 0.0..1.0, default = 0.7, doc = "master level: the sum of the voices times this")]
    pub volume: f32,
    #[param(choices = "saw|pulse|triangle|sine", default = 0, doc = "oscillator 1 waveform")]
    pub o1_wave: f32,
    #[param(range = 20.0..20000.0, default = 8000.0, curve = log, unit = "Hz", doc = "filter 1 cutoff")]
    pub f1_cutoff: f32,
    /* … the other 88 rows, one attribute each … */
    engine: Engine,
}

impl Module for Analog {
    fn init(&mut self, ctx: &Ctx) { self.engine.init(ctx.rate(), ctx.seed()); }

    fn process(&mut self, block: &mut Block, ctx: &Ctx) {
        let n = block.frames();
        let sustain = block.control_in(0) >= 0.5;
        let patch = self.patch();
        let events = block.events_in(0);             // &[Ev] sorted by frame; the host paired every NOTE_ON with a NOTE_OFF
        let mut out = block.audio_out(0);            // PlanarMut: 2 channels × n frames
        let (l, r) = out.stereo();
        // render in spans, cutting the block at each event's frame
        let mut at = 0;
        for ev in events {
            let f = (ev.frame as usize).min(n);
            self.engine.render_span(&patch, sustain, &ctx.tempo(), ctx.frame() + at as f64, (&mut l[at..f], &mut r[at..f]));
            at = f;
            match ev.kind() {
                Kind::NoteOn  => self.engine.note_on(ev.id, ev.pitch_hz(), ev.velocity(), ev.glide()) /* a key-mapped instrument reads ev.key() */,
                Kind::NoteOff => self.engine.note_off(ev.id),          // by id, never by pitch
                Kind::Reset   => self.engine.all_off(),
                _ => {}
            }
        }
        self.engine.render_span(&patch, sustain, &ctx.tempo(), ctx.frame() + at as f64, (&mut l[at..n], &mut r[at..n]));
    }
}
```
Voice allocation keys on `ev.id` (steal oldest, as today); the four-lane gate-edge detection and the lane/voice coupling in `engine.rs` go away. The velocity reaches the filter / amp modulation as it does now (`gate = velocity`), now per event.

### 8b. A code rig that switches presets by mood (catpope's mood switching)
```rust
use deck_module::{Ctx, Module, Outputs, math};

#[derive(Module)]
#[module(name = "catpope_rig", domain = "control")]
#[reads("story.act")]
#[port(output, control, name = "lead_preset", path = "lead:@preset",         choices = "calm|bright|angry")]
#[port(output, control, name = "bass_preset", path = "bass:@preset",         choices = "calm|bright|angry")]
#[port(output, control, name = "cutoff",      path = "lead:analog.f1_cutoff", mode = "set")]
#[port(output, control, name = "pad_gain",    path = "pad:@gain",            mode = "set")]
pub struct CatpopeRig { mood: Option<u32> }

impl Module for CatpopeRig {
    fn control(&mut self, ctx: &Ctx, out: &mut Outputs) {
        let c = ctx.character();
        let mood = match c.mood { "bright" => 1, "angry" => 2, _ => 0 };
        if self.mood != Some(mood) {                        // select a preset on change, not every block
            self.mood = Some(mood);
            out.set(Self::LEAD_PRESET, mood as f32);        // the host runs Preset::apply on 'lead'
            out.set(Self::BASS_PRESET, mood as f32);
        }
        // energy opens the lead's filter, 400 Hz … 6 kHz, exponentially (libm, not f32::powf)
        out.set(Self::CUTOFF, 400.0 * math::exp2(c.energy * 3.91));
        // the pad ducks in act 3 and later
        let act = ctx.var(Self::STORY_ACT).unwrap_or(0.0);
        out.set(Self::PAD_GAIN, if act >= 3.0 { 0.4 } else { 0.8 });
    }
}
```
The same rig as data is the `moods` / `personas` `Variant` tables with the built-in `rig_table` policy; the code differs only where it computes (the cutoff curve, the act rule). The rig document adds `policy = "module:catpope_rig"`; the check passes only if tracks `lead`, `bass`, `pad` exist and `lead`'s instrument has `analog.f1_cutoff` and the preset names.

### 8c. A sensor-driven module (accelerometer → a control output)
```rust
use deck_module::{Ctx, Module, Outputs, Smooth, Status};

#[derive(Module)]
#[module(name = "tilt", domain = "control", wants = "motion")]      // soft: runs without, reporting 'unavailable'
#[port(output, control, name = "tilt")]                              // 0…1, a cable into any parameter
#[port(output, control, name = "shake")]
pub struct Tilt { lp: Smooth, shake: Smooth }

impl Module for Tilt {
    fn control(&mut self, ctx: &Ctx, out: &mut Outputs) {
        let m = ctx.motion();                                        // Sensor<Motion>: status + the channels
        if m.status() != Status::Live && m.status() != Status::Replay {
            out.set(Self::TILT, 0.5);                                // a defined rest value without the sensor
            out.set(Self::SHAKE, 0.0);
            return;
        }
        let a = m.accel;                                             // [x, y, z] m/s², x to the right
        self.lp.to((a[0] / 9.81).clamp(-1.0, 1.0) * 0.5 + 0.5);
        let g = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]) - 96.2;    // |a|² − g²: bursts above gravity
        self.shake.to((g.abs() / 40.0).min(1.0));
        out.set(Self::TILT, self.lp.step(0.25));                     // one-pole per block
        out.set(Self::SHAKE, self.shake.step(0.5));
    }
}
```
`needs = "motion"` instead of `wants` would make the host not run the node at all on a laptop and report it; the status branch above is then dead code the compiler keeps. The conformance test feeds `tilt.sensorlog` and asserts the output hash on every backend.

## 9. Work breakdown (proposed tickets; the owner orders them – I have created none)

| # | Ticket | C | Depends on | Notes |
|---|---|---|---|---|
| A0 | **Descriptor additions** in `deck-catalog`: event port `names/props/capacity`, `sidechain`, control-out `path/mode/choices`, `reads`, `assets`, `wants`, `presets`, `latency`; regenerate `schema.json` / `kinds.json`, TS types; delete `host::Descriptor` | S | – (#315 built) | additive, `--check` in deck:ci |
| A1 | **SDK + macros:** the `Layout` module (const in the derive, runtime in the host), `InHdr/OutHdr`, `Ev`, `Planar`, `Params` + dirty hook, `Ctx` v2 (character senses, show time, declared vars, sensors, assets), `Outputs` with paths, `Smooth`, `math` (libm), counting / trapping allocator, `testing::Rig` v2, `dm_*` exports, layout hash | L | A0 | the layout function is the first commit so A2 can start |
| A2 | **Host crate + conformance:** `Instance` on two copies, `dm_asset_buf` binding, memory freeze, NaN canonicalization, `Preset::apply`, ModuleStats fields, the v2 `fixture` module and the hostile / alloc / sensor-log / event tests over every backend | L | A0, A1 (layout) | `Backend` trait unchanged |
| A3 | **Engine adapter** (`deck-engine`): `ModuleInstance` v2 (channel slices, events, control outputs, path targets, domain scheduling, derived seeds), the in-block event host both ways (= #289 for the engine), control-half placement, applying written targets one block late | L | A2; the N-channel part waits for **#318** (adapter runs 2 channels first) | collides with #318's pool rewrite – same worker or strictly after |
| A4 | **Sensors:** the core's channel table, status machine, ring, log replay (M); web adapters in live-core (DeviceMotion, Geolocation, camera features) (M) | M + M | A2 / A3 for the core; **#320** for the browser adapters | native adapters only when a device needs them |
| A5 | **Groups:** rig doc `policy`, path grammar + resolution in `deck-space`, `rig_table` kind, checks (shared with #313), stats rollup | L | A3; best after **#322** (native deck plays rigs) so it is heard | the declarative path first, the code path second |
| A6 | **Migrate** `walk`, `fixture`, `analog`; regenerate fixtures; **delete** `modules.ts`, `module-abi.ts`, `modules.test.ts`, `build:modules`; recompile list for stored modules | M | A1, A2 (A3 for hearing) | the three modules' own tests are the proof |
| A7 | **Build service:** SDK template, `clippy.toml` disallowed methods, pinned RUSTFLAGS, strict-allocator test run, `rebuild --all` | S | A1 | runs where #281 runs (owner's bunny step open) |
| A8 | **ModuleStats v2 (#307):** the new counters, the browser mirror, per-path rollup | S–M | A3; browser part after **#320** | |
| A9 | **Docs:** `deck/module/README.md` rewrite, RIG-EDITING §4 pointer, ENGINE-REFACTOR-PLAN §3.6 / P11 status | S | A1 | last |

**Parallelism (two sessions, one worker lane):** A0 → A1 → A2 are serial and need nothing from the engine, so they can run now, in the slot where the plan puts "P11 ABI v2". A6 follows them directly. **A3 must not start before #318 is merged or is handed to the same worker** (both rewrite the engine's module node and the buffer pool); A4's core and A5 can follow A3 in either order; A7 runs beside A2. **Waits for #320 (browser engine):** the web sensor adapters, the browser stats mirror, and *hearing* any module in the creator (#290). **Waits for P4b / #318:** multichannel and `any` end to end, sidechain ports, the multichannel proof in `fixture`. **Chapter 3 guard (plan §7):** nothing here touches the reader; the worker lane yields Nov 9–15.
Estimate from the ledger's classes (M $2.2–2.9, L $4.7–11.2): **≈ $35–55 worker cost** for A0–A9 (3 × L, 1 × M + M, 1 × L for A5, rest S), plus the design review. #323's own label says `complexity:M`; as a build ticket it is L and splits into A0–A9 (the owner decides the board).

## 10. Open questions for the owner

Settled by the owner's principle (2026-11-03: "dont worry about backwards compatibility. we are building this fresh. the refactor is design choice.") and therefore **not** questions: the TS module host, `module-abi.ts` and their test are deleted in A6 (§7); module seeds are derived per node path (§3); v1 shapes (`dm_set_param`, the second descriptor type, `deck.params` / `deck.presets`, cv-lane notes) simply go. Pitch is decided too (Q2).

1. **Descriptor-derived layout (recommended) vs a larger fixed `Io`.** Recommended: derived (§1.1). The fixed `Io` saves the const-layout machinery in the SDK (≈ ⅓ of A1) and the layout-hash test, but every later capability (a new sensor, another region) would again move offsets – a v3 and another recompile of every stored module; and each block copies ≈ 70 KB instead of ≈ 5 KB. *Decide now: it shapes A1.*
2. **Decided by you 2026-11-03: note events carry both Hz and a MIDI key; the instrument reads what suits its design** (§1.4). The core's event host fills the missing one from the deck's tuning, so conversion exists once. Open inside it: whether the tuning (A4, a tuning table for non-12-TET) is a deck setting or a per-rig value – recommended deck setting with a rig override; cost of the alternative is a second place that decides pitch.
3. **Decided by you 2026-11-03: a code rig is a policy module over the data tracks** ("policy is what we want"), not a module that owns its children (§5.2). Run-time creation of tracks stays out; the declarative rig remains the same shape (built-in `rig_table`). Nothing open.
4. **Sensor privacy and prompt policy (#312 item 4 asked for your confirmation):** processed on the device; camera features only (no frames to modules); position coarse unless the scene declares `position.precise`; permission only at a user gesture; the audience sees a scene's `needs` / `wants` before it can start; `needs` ⇒ node not run + reported, `wants` ⇒ runs with zeros. Recommended as written. Alternative: a single flag per module (no hard / soft split) – simpler, but a motion-only instrument then plays silent noise on a laptop instead of saying so.
5. **Presets promoted into the descriptor (recommended)** vs separate `preset:<id>` documents for a module's factory sets. Descriptor: a module is self-contained and the build service can test every preset; the catalog documents stay for *derived* sets. Alternative: keep presets outside the wasm – then a module published without its document set has none, and `Preset::apply` needs a lookup service.
6. **Sequencing against the engine work.** Recommended: A0–A2 and A6 (SDK, host, migration, no engine) in the next free worker slot; A3 only after #318 so one rewrite of the module node serves both. Alternative: A3 first with a stereo adapter – the analog synth plays in the engine sooner (the #332 review waits on this), at the cost of reworking the adapter when #318 lands (≈ S–M extra).
7. **Strictness of the allocator trap in shows (recommended: count, never trap; trap only in tests and Play).** Alternative: always trap – catches a creator's bug on the deck too, but silences a rig mid-show for a one-off allocation.
