Status 2026-11-01: design + first slices (#191) – the Rust types (`deck/space/src/rig/`), `Rig::flatten`, the browser engine's lowering, the core rigs and the phone page's rig mode (`?rig=1`) are built and tested; storage is decided (§5, 2026-11-01); the tables, the API, the native deck loading rigs and the creator UI are next, the rest of §8 open.

# INSTRUMENTS — characters' instruments, presets, derivatives

Owner 2026-10-31 (#191): "every character can house several instruments, like a track in Ableton holds an instrument"
· presets save synths and their settings to characters, moods and personas and are altered as **derivatives** (parent →
child) · over time we collect instruments, settings and routings in variations across the characters' traits · scene
objects bring their own sound (#210) · **everything is audiovisual** (#212) · controls are mappable (#209, #212) ·
lent / owned / core with rights from the definition (#191) · **Rust first**: the Svelte /live page is only the bridge.

## 1. Instrument
A **patch of the scene graph** (scene.rs's `[[node]]` / `[[link]]`, node ids local to it) + one audio output `out` (the
track's) + `expose` (the control inputs it shows: path `node.port`, label `{ de, en }`, optional narrower `range` –
the default range is params.rs's) + `kind` (drum · bass · lead · pad · voice · sampler · texture · fx) + `rights`.
- Never holds scene-only kinds (screen, audio_out, tempo, space, outlet): the rig / scene adds them once.
- `Instrument::check` builds it as a graph (graph.rs's rules: types, one link per input, no cycles).
- Audiovisual: its control outputs (an LFO, an envelope, the FFT later) are routed by the rig to the being's look (§3).
- Generators and inputs are instruments too (owner 2026-11-01): an arpeggiator / sequence generator is an instrument
  whose output drives another track (instruments rigged together); face tracking and phone motion are input
  instruments (#212). Sequences are mostly not in presets: they come from a scene, a character, the relay or MIDI
  (#234); patterns and audio clips are imported with the original kept (#235).

## 2. Preset and derivatives
`{ id, instrument, parent?, name, tags, values, unlink, link, mapping, rights }` – it holds **only what it changes**
against its parent (a root: against the instrument as patched). That *is* the lineage's "what changed".
- **values**: control input `node.port` → f32. A value on a linked input is refused (the link would win – feel.rs's
  nudges have the same rule); `unlink` frees it first.
- **routing changes**: `unlink` (inputs whose link goes) and `link` (added links; one into a linked input replaces it).
- **mappings** (#209 / #212): `{ source, target, range? }`, source `midi:<ch>:cc:<n>` · `face:<blendshape>` ·
  `motion:<axis>` · `pad:<control>` · `stick:<left|right>.<x|y>`. A child's mapping with the same (source, target)
  replaces its parent's.
- `PresetBook::lineage` = root first; refused: an unknown parent, a loop, a parent of another instrument.
  `resolve` folds the chain root → leaf; `derive(parent, id, held, edits)` writes a child with only the edits that
  differ from the parent's folded values, and needs the parent's `derive` right for how it is held.
- **tags** carry the traits we collect variations across (`cathedral`, `sad`, `vespers`, a genre …) – search, not logic.

## 3. Rig (a character's tracks)
`{ subject, track[], route[], personas{}, moods{}, rights }`.
- `subject` = KANON's subject form: `character.<id>` (`character.<id>.<persona>` for a persona's own rig) or
  `item.<id>` for a scene object (#210: the incense burner = an item with a one-track rig).
- **track** `{ id, instrument, preset?, gain, pan, mute }` – the Ableton track.
- **route** `{ from, to, scale, offset }` with `<track>:<node>.<port>`; `to` may be `being:<input>` (the space node's
  hue · glow · light · pulse) – sound and image from one routing. Every track reaches the master through its strip; an
  audio route is a send into another track's audio input (an fx track), a control route a modulation.
- **variants**: `personas.<p>` then `moods.<m>` (the `moods` registry's ids – the 20 portrait moods), each
  `{ presets: { track → preset }, values: { "<track>:<node>.<port>" → f32 } }`. `Rig::play(lib, persona, mood)`:
  per unmuted track the preset = mood's ‖ persona's ‖ the track's, folded through its lineage, then the persona's and
  the mood's single values on top. A mood the rig doesn't name plays the defaults. The feelings (#188) already switch
  the head's mood; the same switch calls `play` → the changed values go to the running graph (no rebuild unless a
  preset's routing differs).
- `Rig::check` refuses duplicate tracks, unknown instruments / presets, a preset of another instrument, variants and
  routes naming unknown tracks.

## 4. Ownership and rights
How a deck **holds** a definition is `Hold` – lent (a conductor's grant for one scene, via the relay, #182) · owned (the
carrier bag, #192) · core (the deck image). What it may do is the definition's `Rights` per hold: `play` · `tweak` (live,
unsaved) · `derive` (save a child) · `copy` (a copy without lineage). Default: lent = play + tweak, core = + derive,
owned = all. Saving into a carrier bag = the definition's right + the deck's grant (owner 2026-10-30), not a token scope.

## 5. Where it lives (owner 2026-11-01: two layers; instruments are items)
Two layers (owner 2026-11-01: "factory presets and rigs and … modification by individual users"):
- **Factory** = catalog kinds (agents/CATALOG.md: one row per entry, `id, toon, position, data` JSON, `catalog_history`
  + undo, edited in the creator like items): `catalog_instruments`, `catalog_presets`, `catalog_rigs` (id = the
  subject). `data` is the serde form of `deck/space/src/rig` – PHP + TS validators mirror it.
- **Members' modifications** = member rows on top of the factory: `member_presets (id, user_id, parent, instrument,
  data, created_at)` – a derivative, its `parent` a factory or an own preset – and `member_rigs (user_id, subject,
  data)` – a member's variant of a factory rig (other presets per track, more tracks with instruments they hold).
  Member ids are namespaced (`m:<ulid>`) so a lineage crosses from the factory into a member's.
- **Instruments are items too** (owner 2026-11-01: "a image like any other item so instruments can be found or gained
  in the carrier bag"): a `category: "instrument"` item with `instrument: "<catalog_instruments id>"` – the pattern of
  `category: "furniture"` + its `furniture` block (ITEMS.md). The item brings what every item has (picture, 3D model,
  name, grantable by story / scan, editions, a physical card or figure – the Tonie idea); the instrument row brings
  the graph. Several items may grant one instrument (variants, editions). Holding the item in the carrier bag =
  **owned**; a factory rig may use any instrument, a member's rig only the ones they hold (owned / core / lent).
- **Core**: the rigs + their instruments / presets a deck image ships, as TOML in `deck/stage/rigs/` – exported from the
  catalog when the ISO is built (the same documents, so `Rig::play` doesn't care where they came from).
- **Samples** (the sampler kind): assets per ASSETS.md, `<toon>/sounds/<instrument>/<name>.v<n>.<ext>`, referenced
  from node params; the asset registry indexes catalog rows as uses already.

## 6. How decks fetch it (#195)
- `GET /api/library/scene-content?…&characters=plf&items=incense_burner` gains per character `rig` (its rig, persona
  variant included) and per item its rig, plus top-level `instruments` and `presets` maps holding every referenced one
  **with its ancestors** (a lineage must resolve offline). Each `{ id, version }` so the sync skips what's cached.
- Owned derivatives: `&presets=<ids>` with the paired deck token (#180) – read scope, the member's own rows only.
- Lent: the conductor's `scene.sync` names them in its reserved `beings` field (renamed `presets`/`rigs` when built);
  the deck fetches them like the rest and drops them when the scene ends.
- The deck caches them on DECKDATA next to sets and portraits (`<cache>/rigs/…`), plays offline from the cache.

## 7. The phone page's test patch → the selected character's rig
Today `shared/live-deck/src/scene.ts` hard-codes the trimmed cathedral (PARAMS, ENGINE_TOML for live-core, the
incense in Web Audio). Replaced in steps:
1. **Rust (built):** `Rig::flatten(lib, persona, mood)` – the played tracks' patches in one scene: node ids
   `<track>_<node>`, four-input mixers chained (`mix1` → `mix2.in1` …, a track's `gain` = its mixer gain) → `out`
   (audio_out) + `fft`, the routes as links (a muted track's are off), always a bare `space` node (the caller sets its
   set / spot) breathing on `fft.level → space.pulse` unless a route drives `being:pulse`, the being's `character` from the subject and `expose` from the instruments. No pan yet
   (the mixer has none, #232); one stereo output (#233).
2. **wasm (built):** deck-space `rig::Bundle` = `{ rig, instrument[], preset[] }` (scene-content's answer, or a core
   set's TOML); `play(persona, mood)` → `{ scene (TOML), nodes, links, character, knobs, tracks }` – a knob = an exposed
   parameter with params.rs's range (or the instrument's), curve, label and current value (scene.ts's `Spec`).
   deck/space-web exports it as `rig_play(bundle, persona, mood)`, shared/live-deck's `SpaceStage.playRig`.
3. **Engine + page (built, opt-in `?rig=1`):** deck-space `rig/engine.rs` lowers the flattened scene for live-core
   (the batch kinds stay; a strip gain becomes a `mul` before the gain, the knob aliased to it; mixer / audio_out /
   fft / space go; noise, seq … are the page's, with their strip gain). Core rigs ship in `deck/stage/rigs/`
   (`rig::bundle::CORE`): catpope = the phone patch as data (the voice + the drone, presets base / bright / low, mood
   variants), the incense burner = the incense out of the voice (#210). space-web `core_rig` → `rig_play`;
   shared/live-deck `rig.ts` (sliders, wire graph, streamed paths, the mood diff – values only or a reload) and
   `Deck.useRig` / `moodRig`; the viewer's /live/deck plays the character's core rig with `?rig=1` (the panel: its
   gains in the bar, the rest in the fold, no pads). shared/live-deck's tests run on `test/fixtures/rigs.json`, written
   by deck-space (`UPDATE_FIXTURES=1 cargo test -p deck-space rig_fixture`; a stale file fails the Rust test).
- Open: the phone check on the S22, then the default (no `?rig`); sequences, #234 (the voice holds its pitch, the
  melody is the fixed patch's); routes into page nodes (an LFO on the incense) are dropped; rigs from scene-content.
- Constraint: live-core plays lfo · smooth · mul · add · osc · fm only – other kinds are the page's or need live-core
  first (#182).
- TS types: only the relay protocol is generated (deck/protocol/schema.json → shared/live). Rigs travel over HTTP, so
  shared/live-deck's `RigPlayed` is hand-written next to the wasm API it types (space.ts) and tested on the fixture; when scene-content serves them, the forms go into a JSON schema with a
  generator like deck/protocol's (Rust stays the source of truth: the schema must round-trip `rig/tests.rs`' set).

## 8. Open – for the owner
1. ~~Storage~~ – decided 2026-11-01 (§5): factory catalog tables + member rows; instruments are items.
2. **Persona:** one rig per character with persona variants (recommended), or a rig per persona? (both fit `subject`)
3. **Persona × mood:** is "persona, then mood" enough, or do personas need their own mood overrides?
4. **Default rights** per hold (§4) – right? May members share / give / sell derivatives (second-hand market)?
5. **Discrete settings** (LFO shape, a sequence's notes, a sample) in presets: now only f32 control inputs.
6. **Mappings:** saved with the preset (as asked), but a performer's own controller differs per deck – also a per-deck
   mapping layer on top?
7. **Mood switches:** instant, cross-faded, or quantized to the bar (#181's clock)?
8. **Core set:** which characters' rigs ship in the image?
