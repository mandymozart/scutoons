Status 2026-11-05: accepted by the owner ("all recommendations good for now" · "go", the six decisions of §5); building: T1 built (#408, where it chose: §6) and T2 + A1 built (#409, ABI-V2 §9g: the live context reaches every module node; the mood is mapped at the node, §4); T3 built (#410, where it chose: §7) – catpope as a code rig plays on the native deck with the real compiled policy; the headless run of the module waits for #290; T6 built (#412, where it chose: §8). The example (§3) compiles against deck/module as it is (native and wasm32) and passes a strict `testing::Rig` run.

# RIG-MODULE-GROUPS – rigs as module groups: a code rig is a policy over the data tracks

Owner: "of course it is just another layer after the instruments. rigs are module groups." (2026-11-02) · "policy is what we want" (2026-11-03, [rigs]) · 2026-11-05: finish the rig design, then wiring + UI and audio in real scenes. This builds on ABI-V2 §5 (the decided policy design) and corrects it where the code says otherwise (marked **≠ §5**).

## 1. What exists (read from the code, main 5d8d3d4e)
- **Declarative rig** (`deck/space/src/rig/mod.rs`): `Rig { subject, tracks[{id, instrument, preset, gain, pan, mute}], routes[{from, to, scale, offset}], personas / moods: name → Variant { presets: track → preset, values: "<track>:<node>.<port>" → f32 }, rights, sticks }`, `deny_unknown_fields`. `Rig::check`: unique tracks, known instruments, presets that fit the track's instrument, variant paths that are control inputs, route scopes that are tracks (or `being` as a target). `Rig::play(lib, persona, mood)`: for each unmuted track, the preset (track ← persona ← mood), `PresetBook::resolve` (lineage), the variants' values on top, `Instrument::patch`.
- **Flatten** (`flatten.rs`): nodes renamed `<track>_<node>`; tracks summed by chained 4-input `mixer`s (`mixN.gainK` = the track's `gain`) into `out` + `fft`; routes become links, `being:*` → `space.*`. **Findings:** `Track.pan` is never used (there is no stereo strip), and a muted track is left out of the scene, so nothing can unmute it at run time.
- **Bundle / Deck** (`bundle.rs`, `deck.rs`, #368): `Bundle::play` → `Played` (scene, knobs, sticks, streamed paths, live-core lowering). `Deck::step` walks the mood `Ladder` (mood.rs) once per frame. When the mood turns, `replay` re-plays the whole bundle and `Played::diff` returns the values that moved (or `reload` if the shape changed). The native deck (`native/src/rig.rs`, #322) applies them as `Controls::set_base`; the browser does the same through space-web's `DeckState`. `horizon.rs` can prebuild the moods reachable next.
- **catpope** (`deck/native/rigs/catpope.toml`, `CORE`; conformance `tests/rig.rs`, headless 09 / 15 / 23): tracks `voice` (fm + a seeded walk) and `drone` (two osc + a mixer). Presets: `low` = ratio 0.5 + index 1, `bright` = ratio 3 (both derived from `base`). Moods: thinking → `low`; sad → `low` + `drone.gain 0.14`; happy → `bright`; laughing → `bright` + `voice.index 4`; surprised → `bright` + `index 6` + `drone.gain 0.05`. **So mood switching today means re-flattening on the frame thread, diffing, then set_base. No node in the graph knows the mood.**
- **ABI v2 as built** (#339–#345, deck/module): `Module::control(&mut self, ctx, out)` for `domain = "control"` (the derive refuses audio ports there). `Ctx` gives frame / tempo / phase / bar, `seed`, `character()` {id, persona, mood (strings), `mood_index`, energy, touch, strain}, `var(Var)` for `#[reads]` slots, control and event inputs, sensors, assets. `Outputs::set(i, v)` sets a written bit (an unwritten output holds its last value); `emit` sends events. Control outputs carry `path`, `mode` (set | add | mul) and `choices`; `deck_module::math` is libm.
- **Engine** (`engine/src/module.rs`, `evhost.rs`, #342): a control module is a stepper of the control half and runs once per 128-frame block; its outputs apply one block later. `graph.rs bind_targets` binds `<track>:<node>.<port>` → `<track>_<node>.<port>` as a link: set = scale 1, holding the target's own value; add = offset = own; mul = scale = own. **`@…` members and missing nodes stay unbound; they are left for #314.** **The module's context (`character`, `persona`, `mood`, `vars`) is read once, from the node's params, at build.** Module-host's `set_mood_index` / `set_senses` exist but nothing calls them, so today no module ever sees a live mood, energy value or variable.
- **Validation** (#313, `rig/validate.rs`): `Request {kind, id, document, instruments, presets}` → `Rig::check`, then `Bundle::play` for every persona and mood (deck/build `POST /validate`, space-web `rig_problems`). A rig request does not carry modules yet.
- **Catalog / editor:** the factory modules `walk`, `analog` and `fixture` are `module:<id>` documents that can only be derived (#350). The Nodes tab's data is `rig/wiring.rs` (#282): the checked graph, where bound path outputs are links, so they are drawn for free. #283 (the Assignment tab as a form) is being built by another worker. `ModuleStats` exist per instance (module-host `stats.rs`); there is no rollup yet (#307).

## 2. The five questions – proposals
**Q1 – What a code rig can do.** A code rig is a rig document with one extra field: `policy = { module = "catpope_rig", seed = 1, values = { heat = 0.3 } }`. Flatten then adds a node `policy` (`module:catpope_rig`) to the played scene; `policy` becomes a reserved track id. The policy drives children only through control outputs with a path:
- (a) `<track>:<node>.<port>`: any control input of the track's instrument (this binds today);
- (b) `<track>:@preset`, whose `choices` are preset ids of that track's instrument (value = index, applied when it changes);
- (c) `<track>:@gain`: the track's `mixN.gainK` (mute = gain 0, `mode = "mul"`).

**≠ §5:** no `@pan` (flatten has no stereo strip; it waits for #318), no `@mute` (a muted track is not in the graph), no `route.<i>:@scale` (drive the route's target with `add` instead).

Outputs without a path are cables: a route `from = "policy:<out>"` goes to a track input or to `being:glow`. The policy **reads** `Ctx`: tempo / phase / bar; the character (mood, persona, energy, touch, strain – once the engine feeds them, T2); declared variables; declared sensors (#312). It sees a child's output only as data flow: a route `<track>:fft.level → policy:<control input>`, one block late, with no hidden access. It may emit events (for example, sequence an analog track's `notes`, #342). It may **not** create or remove tracks (decided 2026-11-03). Its `#[param]`s are rig knobs: the Parameters tab shows them, `sticks` may name `policy.heat`, and persona / mood tables may set `"policy:heat"`.

**Q2 – Groups in the ABI and the descriptor.** The module's descriptor lists only the policy's own params and its targets (paths with mode and choices). That is its contract: "I need a track `voice` with `voice.index`". The children belong to the rig document, not the module. What the editor and the validator see, the **rig's face**, is computed by the core as `Rig::describe(lib, modules) -> RigFace { tracks: [{ id, instrument, preset, addressable: [{ path, range, curve }] }], policy: { module, params, targets: [{ path, resolved: "voice_voice.index" | preset table | strip gain }] }, moods, personas, knobs }`. This lives once in deck-space; the editor only shows it (A2). The ABI needs one addition (A1, §4); everything else is resolved in deck-space at flatten time.

**Q3 – Determinism and budget.** The policy is an ordinary control-domain node: once per block, a strict budget of 0.75 × quantum, restarted in a show. Seed: the node's `seed` (1 unless `policy.seed` sets it – the module seed rule). Its outputs are a pure function of its state and its per-block inputs. The context (mood, senses, variables) enters through the engine at a block start (T2). A test replays a **context log** (`{frame, mood, energy …}`, in the form of #341's sensor log) and hashes the outputs on every backend. A show stays exactly as reproducible as the mood ladder is today (per deck, per frame). There is only one rate (per block); a policy that only switches should skip unchanged blocks, as the example does. Stats: `policy` gets its own `ModuleStats`; the rollup is by node prefix (`voice_*`, `drone_*`, `policy`, `mix*`) in the stats reader and the Nodes tab (#307). There is no group-level trap.

**Q4 – The editor.** `rig:<subject>` stays the one document. The policy is a **separate `module:<id>` document** with its own drafts, compile and review, derived from a factory template `rig_policy` that the core fills from `RigFace` (one output per chosen target). The Sound workspace (#282) on a rig: **Rust** = the policy's files ("Add policy" when there is none); **Parameters** = the instruments' exposed params plus the policy's; **Nodes** = the played scene, including `policy` and its target links; **Assignment (#283)** = the form for the same rig document (tracks, presets, moods, personas, routes, sticks). Form and code are one document, not two modes: each path is either in the tables or owned by the policy. Rows the policy owns show as "driven by code" (read from `RigFace`) and are read-only in the form. #283 needs nothing for this now: serde keeps the `policy` field and the form ignores it. Marking the owned rows is a follow-up (T6).

**Q5 – Validation by the core, for both forms.** `validate::Request` gains `modules: [descriptor]`, sent by value. `Rig::check` adds:
- The policy module exists, has `domain: control` and is compiled for ABI 2.
- Every target resolves: the track exists; `<node>.<port>` is a control input of its instrument; the `@preset` choices are presets of that track's instrument; `@gain` points at an unmuted track.
- **Each input has one owner.** The engine adds a linked input's base to the link (`source · scale + base`), so a `set_base` under a `set` link would sum. Therefore a `set` or `mul` target may not also be a mood / persona value, a route's `to` or another target, and a preset value on an owned input is dropped with a warning. `add` targets layer over the tables: the base is the table's value (the `being.react` model).
- Then `Bundle::play` runs for every persona × mood as today, now with the policy node in the graph, so `Graph::build` also checks its cables. The declarative form keeps every check it has.

## 3. Example: catpope's mood switching as code (checked against the SDK)
```rust
use deck_module::{Ctx, Module, Outputs};

#[derive(Module)]
#[module(name = "catpope_rig", domain = "control", doc = "catpope: the voice's preset and FM index, the drone's level, by mood")]
#[port(output, control, name = "voice_preset", path = "voice:@preset",
       choices = "cathedral_voice.base|cathedral_voice.low|cathedral_voice.bright")]
#[port(output, control, name = "voice_index", path = "voice:voice.index", mode = "set")]
#[port(output, control, name = "drone_gain", path = "drone:drone.gain", mode = "set")]
pub struct CatpopeRig {
    #[param(range = 0.0..1.0, default = 0.0, doc = "how far the being's energy opens the FM index (0 = the table alone)")]
    pub heat: f32,
    mood: Option<usize>,
}

/// mood → (voice preset choice, FM index, drone gain); row 0 = neutral and every mood the rig does not name
const MOODS: [(&str, usize, f32, f32); 6] = [
    ("", 0, 2.0, 0.1), ("thinking", 1, 1.0, 0.1), ("sad", 1, 1.0, 0.14),
    ("happy", 2, 2.0, 0.1), ("laughing", 2, 4.0, 0.1), ("surprised", 2, 6.0, 0.05),
];

impl Module for CatpopeRig {
    fn control(&mut self, ctx: &Ctx, out: &mut Outputs) {
        let c = ctx.character();
        let row = MOODS.iter().position(|m| m.0 == c.mood).unwrap_or(0);
        let (_, preset, index, drone) = MOODS[row];
        if self.mood != Some(row) {
            self.mood = Some(row); // select on a change only: an output holds its last value
            out.set(Self::VOICE_PRESET, preset as f32);
            out.set(Self::DRONE_GAIN, drone);
        }
        // the policy owns voice.index: the table's value, opened by energy when heat > 0 (libm: the same bits everywhere)
        out.set(Self::VOICE_INDEX, index * deck_module::math::exp2(self.heat * c.energy));
    }
}
```
**Verified** (2026-11-05, a scratch crate against `deck/module` at 5d8d3d4e, deleted afterwards): it compiles natively and for wasm32 (`cargo check --target wasm32-unknown-unknown`); a strict `testing::Rig` run with `set_character(…, mood)` gives (preset, index, drone) = (0, 2, 0.1) for neutral, (1, 1, 0.14) for sad, (2, 4, 0.1) for laughing and (2, 6, 0.05) for surprised, with no allocation and no preset write without a change; its descriptor parses into `deck_catalog::Descriptor` with three path outputs.
**Assumed, not there yet:** the rig's `policy` field and the node in flatten (T1); `voice:@preset` resolved and applied (T1, T3 – unbound today); the live mood reaching `ctx.character()` (T2 – today a deck hands the policy its node's `mood` param once, at build, so it would see one mood forever); `heat × energy` needs `set_senses` fed (T2). It reproduces catpope.toml exactly once `low`'s `index = 1` is dropped as owned by the policy (Q5); the policy writes 1.0 for thinking / sad itself. With A1 the string match becomes `c.mood_index`.

## 4. ABI additions, the rest, and the order
**ABI (the bytes between module and host): one addition, no header change.** A1 (S) adds `#[module(moods = "thinking|sad|happy|laughing|surprised")]`, a new additive descriptor field. The frame thread maps the deck's mood to the position in the module's own list (+1; 0 = a mood not in the list) and hands it to the node as the existing `mood_index`. This removes the open question of which registry `mood_index` counts in (the ladder has 5 moods, catpope also names `sad`). Ticket: part of T2.

**Needed, but not ABI:**
- **T1** deck-space (M) = #314's build part (ABI-V2 §9 A5):
  - `Rig.policy`;
  - flatten adds the node and resolves `@gain` (a link to `mixN.gainK`) and `@preset` (a per-track table of the choices, resolved through `PresetBook`);
  - routes from and to `policy:`;
  - `Rig::describe`;
  - the checks of Q5, with `Request.modules`.
- **T2** engine + Deck (M, #409 – **built**, ABI-V2 §9g): a context command from the frame thread's `Deck` (mood index / string, persona, the being's energy / touch / strain; later scene variables) to every module node. It applies at a block start without allocating, natively and in live-core alike. Includes A1. *Where the note had a hole:* the frame thread sends one context (the mood as a string) and each node maps it to its own list's index (deck-space `context::mood_index`) – not the frame thread per node as §4 says.
- **T3** Deck (S, new ticket or inside T1) applies `@preset`. The Deck reads the policy's preset outputs (streamed as `policy.<out>`, `Stream::Control`) each frame and replays the bundle with those presets as per-track overrides. It passes the diff, minus owned paths, to `set_base`. This is at most one frame late and reuses #368's replay unchanged.
- **T5** (S) the stats rollup by node prefix → #307.
- **T6** (M, after #282 / #283) the editor: the Rust tab on a rig, the `rig_policy` template from `RigFace`, Assignment marking owned rows.
- **T7** (M, later) block-exact preset tables in the control half and `rig_table` as a node. Only with #296's bake, and only if its measurement asks for it.

**Without any ABI change:** the declarative rig stays the default and keeps everything it does today. A policy with plain `<track>:<node>.<port>` targets works as soon as T1 and T2 land (matching the mood string, as the example does). A code rig is opt-in per rig.

**Order:** T1 → T2 (with A1) → T3, then catpope as a code rig plays on the native deck and in a headless scenario (written first as `expected_fail`, A7) → T5 → T6 → T7. T1–T3 ≈ M + M + S.

## 5. For the owner (max 6; my recommendation first)
1. **How `@preset` applies:** on the frame thread through the Deck's replay (T3: ≤ 1 frame late, no engine work) **vs** block-exact tables in the control half (T7). *Recommend the frame thread now; tables with the bake (#296).*
2. **Path ownership:** a `set` / `mul` target is the only writer of its input; preset values there are dropped with a warning; `add` layers over the tables. *Recommend as written (Q5).* The alternative, letting tables and code both write, would sum in the engine (`source · scale + base`).
3. **`rig_table` as a built-in node** (ABI-V2 §5, A5's approved scope) **≠ §5**: *recommend not now.* The declarative rig keeps its working replay; "one shape" holds at the document, check and `RigFace` level. Make it a node only when the bake (#296) needs it.
4. **Reserved members:** *recommend only `@preset` and `@gain` now*; `@pan` with a stereo strip (#318; `Track.pan` is ignored today, a finding worth its own issue); no `@mute` and no route members.
5. **Where the policy's code lives:** a separate `module:<id>` document named by the rig (its own versions, compile, derive from a `rig_policy` template) **vs** sources inline in the rig document. *Recommend separate* – it reuses #281 / #350 as they are, and one policy can serve the personas' rigs.
6. **Addressing moods in code:** a `moods` list in the module's descriptor → `mood_index` (A1) **vs** string matching only. *Recommend A1* (cheap, no header change). Strings keep working, and the example uses them today.

## 6. Built in T1 #408 – where the note left a choice (2026-11-05)
- **Where the descriptor comes from:** a bundle carries the policy's descriptor (`[[module]]`, `Bundle.modules` → `Library.modules`); flatten registers the kind from it (`ModuleDef::of`). The graph-port numbering moved to the catalog (`Descriptor::graph`): the module host's `Layout` binds by it, deck-space registers by it, the engine's `register` calls `ModuleDef::of` – one rule.
- **`@gain`:** flatten writes the policy node's table `"@gain" = { <track> = "mixN.gainK" }`; graph.rs binds `<track>:@gain` through it like any target (mode, holds). `policy::strip(i)` is the mixer chain's rule, shared by flatten's table and `describe`.
- **`@preset`:** mode `set` only, `choices` must be presets of the track's instrument; a persona or mood naming that track's preset is refused (two writers of the choice). `Played.presets` lists the choices, `policy.<out>` is streamed (`Stream::Control`), `Bundle::play_with(persona, mood, chosen)` is the replay #410 calls.
- **Q5 checks:** also refused – two outputs on one target, `@gain` on a muted track, a target without `<track>:`, a policy value outside its range, a track named `policy` (always reserved). Warnings (`Rig::warnings`, `Answer.warnings`): one line per preset and owned input.
- **The node:** `policy` gets `seed`, its parameter values (policy ← persona ← mood `policy:<param>`), and `character` / `persona` / `mood` as strings – the context it is built with until #409 makes it live. Its parameters are rig knobs `policy.<param>`. In the Nodes tab it is a source (depth 0): its targets cross into the audio half and order nothing.
- **`Rig::describe` → `RigFace`:** tracks with every control input of their instrument (`<track>:<node>.<port>`, the flattened input, range, `driven` = the policy output and mode), the policy's params and targets resolved (`input` / `strip` / `preset`), personas, moods. Knobs stay `Played.knobs` (they need a play).
- **Through the API (#414):** `CatalogValidator` sends the policy's descriptor (`modules`, the `module:<id>` document's `descriptor`) and the presets its `@preset` outputs choose from; a module not in the catalog / not compiled yet / under another name is a 422 problem of the API's own. Scene-content serves `modules: [id]` on the rig and the descriptor in the top-level `modules` (`SceneContent.php`, `rig/content.rs`); `catalog_bundle` takes the catalog's module documents (`modules`) and the presets of the policy's choices (`policy::preset_choices`). Choices where the issue left one: the descriptor's `name` must equal the module document's id (the checks look the policy up by name); scene-content serves the descriptor, not the wasm (no deck fetches a module's wasm yet: #410 proves the policy natively with a test build).
- **Not built (follow-ups):** the browser's engine carries no module, so there the policy is the page's; the headless run of the policy waits for a module runtime there (#290, §7).

## 7. Built in T3 #410 – where the note left a choice (2026-11-05)
- **One platform call:** `Deck::frame(level, bpm, running, touch, dt, heard)` = the mood's `step`, then `choose(heard)`; a mood and a choice in one frame are ONE `Step` (the turns' values in order, the choice's last; a reload when either reloads). The adapters only read the values of `Deck::preset_paths()` (`policy.<out>`) from what the engine played: native `rig::Player::frame` from the heard block's control outputs, the browser's `DeckState.step(…, heard)` (live-deck `tick`).
- **The value → a preset:** the index rounded and clamped into `choices`; NaN (not heard) chooses nothing; an output the policy has not written yet reads 0 = its first choice (so a policy lists the track's own preset first). The rig is replayed only when the choice is another preset than the track plays; a choice holds over every later mood replay until the policy chooses again or another rig plays.
- **Owned paths:** `Played.owned` – the flattened inputs of `set` / `mul` targets (`<track>_<node>.<port>`, a `@gain`'s `mixN.gainK`) – are left out of every `Played::diff` (mood and choice alike): a base under such a link would sum.
- **The policy module of the proof:** `deck/conformance/modules/catpope-rig` (§3 with A1: `moods`, the row by `mood_index`); deck-native tests/modules.rs plays catpope as a code rig with it in the engine – for all six moods, one frame after the mood turned, the voice plays the declarative catpope's preset and the inputs read the tables' values. `Player::play(bundle)` plays a bundle (the deck's `--scene rig:` still names core rigs only, #271).
- **Not built:** the browser's engine and the headless host carry no module runtime, so there the policy's outputs are never heard (NaN, nothing chosen): headless 27 step `code_rig_deck` drives the deck with a stand-in output stream (green, native ≡ wasm32 ≡ simd128), step `code_rig_moods` stays `expected_fail` on #290.
- **Checked:** deck-space tests (the stand-in stream, `Deck::frame`), the native test with the real wasm policy, headless 27 (native ≡ wasm32 ≡ simd128), live-deck / engine-host / viewer checks, `check:core-only` (no budget grew), `deck:ci -- --no-release` green.

## 8. Built in T6 #412 – where the note left a choice (2026-11-08)
- **The template** (`rig/template.rs`, space-web `rig_policy_template`, request = the rig + the catalog's documents): `lib.rs` of a `pub struct Policy`, one that compiles and changes nothing – each unmuted track's `@gain` live at 1 (`mul`); each track's `@preset` (the instrument's catalog presets, its own first) and every addressable input with its range are commented-out `#[port]` lines (uncomment = its `Self::NAME`); `moods` = the rig's own table moods. `@preset` stays commented: live, it would clash with a mood table naming that track's preset. deck/build's real_compile compiles it for wasm32 and validates the rig with the descriptor.
- **Ownership:** `RigFace` gained `FaceTrack.preset_driven` / `gain_driven` and `Driven.owned` (`Target::owns`), plus `FaceTrack.presets`; space-web `rig_face` (the bundle → the face). The form locks what `owned` says: the track's preset and its mood cells (`@preset`), the gain (`@gain`), and lists the owned inputs under "Driven by code" (the form has no field for them). An `add` target locks nothing.
- **The rig's draft may name a draft policy:** New policy saves the rig with `policy.module` before the module is compiled; `CatalogDocs::draftReference` lets a draft through when the `module:<id>` has a version, the publish asks again (compiled and published module).
- **Browser status:** `Engine.note` (the core's sentence on what the browser does not play; `rig_play`'s answer) replaces deck.ts's own text; the Assignment tab and the Rust tab's policy bar show it. Nothing of the policy plays in the browser's deck before #290.
- **The Sound page's rig** lives in `lib/editor/rig-session.svelte.ts`, shared by Assignment and the Rust tab; a rig page opens its policy unless `?sel=` names another module.
