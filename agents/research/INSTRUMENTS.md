Status 2026-11-01: design + first slices (#191) – the Rust types (`deck/space/src/rig/`) and `Rig::flatten` are built and tested; storage, API, deck loading and UI are proposals waiting on the owner's answers in §8.

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

## 5. Where it lives (proposal)
Following the codebase: authored definitions are **catalog kinds**, member-made ones are **member rows**.
- **Catalog** (agents/CATALOG.md pattern: one row per entry, `id, toon, position, data` JSON, `catalog_history` +
  undo, edited in the creator like items): `catalog_instruments`, `catalog_presets`, `catalog_rigs` (id = the
  subject). The `data` is exactly the serde form of `deck/space/src/rig` – PHP + TS validators mirror it (§7).
- **Member derivatives**: `member_presets (id, user_id, parent, instrument, data, created_at)`, listed in the carrier
  bag; ids namespaced so lineage can cross from the catalog into a member's (`choir.base` → `m:<ulid>`).
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
   (audio_out) + `fft`, the routes as links (a muted track's are off), a bare `space` node for `being:*` routes (the
   caller sets its set / spot), the being's `character` from the subject and `expose` from the instruments. No pan yet
   (the mixer has none).
2. **wasm:** deck/space-web exports `rig_play(json, persona, mood) → scene` (the same code as the native stage).
3. **Page:** picks the character (it already does for the head), takes the rig from scene-content, builds the wire
   graph (`deck.graph`) and live-core's scene from the flattened scene, the sliders / knobs / 3D buttons from the
   instruments' `expose`; a mood change applies the value diff. The incense leaves the voice and becomes the incense
   burner item's rig (#210).
- Constraint: live-core plays lfo · smooth · mul · add · osc · fm only (the page runs seq and noise in JS) – a rig with
  other kinds needs them in live-core first (or the browser plays deck-core's kinds, #182).
- TS types: today only the relay protocol is generated (deck/protocol/schema.json → shared/live). Rigs travel over
  HTTP, not the relay, so this slice has no TS; when scene-content serves them, the forms go into a JSON schema with a
  generator like deck/protocol's (Rust stays the source of truth: the schema must round-trip `rig/tests.rs`' set).

## 8. Open – for the owner
1. **Storage:** catalog tables + member rows (§5, recommended) – or entity documents (`rig:<character>` in
   `entity_versions`, like `characters`)?
2. **Persona:** one rig per character with persona variants (recommended), or a rig per persona? (both fit `subject`)
3. **Persona × mood:** is "persona, then mood" enough, or do personas need their own mood overrides?
4. **Default rights** per hold (§4) – right? May members share / give / sell derivatives (second-hand market)?
5. **Discrete settings** (LFO shape, a sequence's notes, a sample) in presets: now only f32 control inputs.
6. **Mappings:** saved with the preset (as asked), but a performer's own controller differs per deck – also a per-deck
   mapping layer on top?
7. **Mood switches:** instant, cross-faded, or quantized to the bar (#181's clock)?
8. **Core set:** which characters' rigs ship in the image?
