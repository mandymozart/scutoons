Status 2026-11-08: design note for the owner's review (#203, slice 3 of #425). W2 built (#430, event cables, silent tracks, world nodes); W1 and W4–W6 not built. Owner 2026-11-08: wiring is configuration, not Rust: question 1 and slice W3 fall away, §2 is to be reread against that (decisions/deck.md [rigs]); the other answers are there too.

# MODULE-WIRING – wiring modules in the editor

**The owner's answers, 2026-11-08 (decisions/deck.md [rigs]); they override the text below where it differs:**
- **Wiring is configuration, not code** (the bench: at most 0.6 % of the audio budget, PC only). The "two file forms" of §2, question 1 and slice W3 fall away. A code rig's `path` outputs stay as they are until a ticket reworks them.
- **Events:** module to module = explicit cables only, no by-name fallback; the world (MIDI, generators, carrier bag, VN state, other scenes, rigs, decks) enters through world nodes, with an image of the origin where there is one; an event input with no cable hears nothing (research/EVENTS-VS-PARAMS.md §5).
- **A note source is a silent track** wired by event cables.
- **When it takes effect:** data on every gesture, Rust on the Compile button.
- **Text view:** TOML if it makes sense long-term for saving configurations, else an alternative view of the nodes; the two must be identifiably related.
- **Reach is wide from the start:** other scenes, characters and rigs, so linked decks can affect each other. Cables in a rig stay relative; the absolute address sits on the world node (changes §3 and W4).
Owner 2026-11-08, binding ([rigs] in decisions/deck.md): the code is the ONE source of a wiring, no override layer; rewiring by hand changes the code and takes effect through the compile; the editor exists only outside the deck ("somit ist das node on the fly verkabel verboten"); addressing must finally reach everything in the Scutooniverse, for now the stage, probably the active scene; what is shown depends on where you are, "aehnlich dem flow editor". The note does not depend on which GUI the editor runs in (#393). Every rule below lives in the core (RULES A1, A2, `check:core-only`).

## 1. What exists (read from the code, main 86a08194)
**Where a cable is written today:**

| Cable | Where it is written | Form |
|---|---|---|
| node → node in an instrument | instrument `link` (`rig/instrument.rs`), changed per preset by `links` / `unlink` | data, `node.port` |
| track → track, track → being | rig `route` (`rig/mod.rs`; `scale`, `offset`; type checked only at `Graph::build`) | data, `<track>:<node>.<port>`, `being:<input>` |
| code rig: policy input / plain output | rig `route`, `policy:<port>` (#408) | data |
| code rig: policy → child | `#[port(output, control, path = "voice:voice.index", mode = "set")]` in the policy's `lib.rs`, resolved by `Rig::targets`, bound by `graph.rs bind_targets` (`bound` cables in the Nodes tab) | **Rust** |
| track → master | implied: `flatten.rs mix` sums every track's `out` into `mixN` → `out` + `fft` | compiled |
| module → module, events | **written nowhere**: `evhost.rs wire` joins every event output to every event input in the whole graph that shares a name (`stepseq.notes` → `analog.notes` via `music.note`), across tracks | implied |

**Findings that shape the design:**
- **`Deck::wiring` / the Nodes tab (#282) shows the flattened graph**: `Played::wiring` over `<track>_<node>` for one persona and mood, read-only, drawn by `shared/rig-designer/src/nodes.ts`. It cannot say which document wrote a cable, so it cannot be edited as it is.
- **Events are invisible to the graph.** `graph.rs PortType` is Video / Audio / Control and `Descriptor::graph` drops event ports (`_ => {}`), so `stepseq.notes` has no port in the Nodes tab, and nothing can keep two sequencer + synth pairs in one rig apart.
- **Why a note source cannot be a track alone:** a track is an audio strip. `Instrument.out` is required and `Instrument::check` refuses a non-audio output; `flatten.rs mix` wires each track's `out` into a mixer. stepseq has only an event output, so `module_instrument` (#425, space-web `rig_module_instrument`; on `feature/425-no-rig-no-sound`, not yet on main) answers "has no audio output – … wire it into a synth". `catpope_analog.toml` works around it by hand: `seq` and `synth` as two nodes of one instrument.
- **A policy's cable is already a table:** `rig/template.rs` (#412) writes one `#[port]` line per target and the hand-written logic only uses `Self::NAME`. Retargeting changes a string literal and no logic.
- `validate.rs Answer.problems` are strings and stop at the first one, so a problem cannot point at a cable yet.

## 2. The code is the source: one model
**Recommendation: one model, two file forms. Every cable has exactly one home, the text of the source that owns it. The graph is a view of those homes, an edit rewrites the one home of that cable, and one verb, Build, makes it take effect.** There is no layer above the sources, and no edit ever writes two homes (an edit that would need both is refused). It must stay two file forms for one reason only: a policy's targets are in its Rust, as decided and built (#314 decisions 2 and 5, #408, #412).
**Which home a graph edit writes:**
- Between nodes of one instrument: the instrument document. It is shared, so the editor says "used by N" first (EDITOR.md §4).
- Between tracks, to the being, from a plain policy output: the rig document's `route`.
- From a policy output with a `path`: that output's `#[port]` line in Rust. Such an output has one target, and `mode` / `choices` stay with it.
- The mixer sum and module internals have no home in the graph: drawn, never editable.

**Write-back without touching hand-written code:** the editor owns a fenced table in the policy's `lib.rs`, which the template emits and a person may edit too:
```rust
#[derive(Module)]
#[module(name = "catpope_rig", domain = "control")]
// wiring: begin – one #[port] per line; the graph edits `path` and `mode` here, the compile decides
#[port(output, control, name = "voice_index", path = "voice:voice.index", mode = "set")]
#[port(output, control, name = "drone_gain", path = "drone:drone.gain")]
// wiring: end
pub struct Policy { /* hand-written from here on */ }
```
A core function rewrites single lines inside the fence: connect = set `path` (and `choices` for `@preset`, from the core's list); disconnect = remove `path` and keep the line, so `Self::NAME` still compiles; a new output = a new line, which holds the target's own value until the code writes it (`Graph::holds`). Outside the fence nothing is ever touched. No fence, or something in it that is not one `#[port]` per line (a hand-written policy such as `catpope-rig`): the cable is read-only with "edit in the Rust tab". No Rust parser is needed.

**What "takes effect through the compile" means, for both:**

| | Data (rig, instrument) | Rust (a policy) |
|---|---|---|
| Edit | the draft document changes | the draft `lib.rs` changes |
| Compile | serde's forms → `Rig::check` → `Bundle::play` for every persona × mood → `Graph::build` (`validate.rs`: `rig_problems` in wasm, `POST /validate` at the API) | cargo → wasm + descriptor (deck/build), then the same validation with the new descriptor |
| Product | a valid version of the document, no stored artifact: decks flatten as they do today | the module document's wasm + descriptor |
| Effect | the editor's preview swaps to it | the same |
| Deck | only after publish and the next load | the same, and a new module only by restart (A9) |

So a compile for data is the validation the core already has, and the only new thing is that the Nodes tab goes through it. A draft that does not build stays a draft: not playable, not publishable, its text not lost. By hand for data means a text view of the document (TOML, the form `rig/mod.rs` already reads; §6 Q5).

## 3. Addressing: one format, limited by a rule
```
address := ["/"] { place "/" } leaf
place   := <kind> ":" <id>       deck:left  scene:nave  character:catpope  item:incense  (later toon:, chapter:, set:, spot: …)
leaf    := today's local path    <track>:<node>.<port> · <track>:@gain · <track>:@preset · being:<input> · policy:<port> · <node>.<port> · @<member>
```
The last segment is always the leaf; a `/` separates levels. Without a leading `/` an address is relative to the document it stands in, so **every path written today is already a valid address.** `character:catpope` resolves to the rig document `rig:character.catpope`; a second copy of a holder on one stage, later: `character:catpope#2` (reserved, unused now).

| What | Relative (in its document) | From a scene | Absolute |
|---|---|---|---|
| A module port in a track | `analog:synth.volume` | `character:catpope/analog:synth.volume` | `/deck:left/scene:nave/character:catpope/analog:synth.volume` |
| A character's parameter | `being:glow` | `character:catpope/being:glow` | `/deck:left/scene:nave/character:catpope/being:glow` |
| A character's state (read only, new) | `@energy` | `character:catpope/@energy` | `/deck:left/scene:nave/character:catpope/@energy` |
| A scene object's sound (#210) | `hiss:noise.gain` | `item:incense/hiss:noise.gain` | `/deck:left/scene:nave/item:incense/hiss:noise.gain` |
| A scene's own node | `fft.level` | `fft.level` | `/deck:left/scene:nave/fft.level` |

- **The limit is a rule, not a shape.** `Reach` is one core function: `resolve(address, context, reach)`. Today `Reach::ActiveScene`: an address must resolve inside the rig being designed (the empty rigging stage) or the deck's active scene; anything else parses and is refused with "out of reach for now". Widening it (a deck, a group, the whole Scutooniverse, with the 2026-11-02 subscription default: nothing travels unless asked) changes this function, not one document.
- **A rig holds only relative addresses,** so it stays portable. Absolute addresses first appear in a scene document, which is not designed yet: the absolute form above is a shape, not yet exercised by anything.
- **Compiled names are not addresses.** `voice_voice.ratio`, `mix1.gain2`, `policy.voice_index`, `space.hue` are slots flatten produces. The run-time, knobs, `Stream` and the relay keep using them; nothing parses an address on the audio or frame path.
- **The relay never parses an address.** Its messages with node paths, `deck.graph` and `deck.values`, use the played scene's compiled slots per sender (the deck place is the sender) and stay. Later, a remote address's `deck:` place becomes a message's `to`; the relay stays a carrier and the resolving stays on the deck.
- **In the editor:** `/edit/rig:character.catpope?sel=analog:synth.volume`; `sel` is an address relative to the subject (EDITOR.md §4; `?sel=voice` stays, a container).

**Migration.** `<track>:<node>.<port>`, `@gain`, `@preset`, `being:`, `policy:`, `node.port`: no change to any document; one parser (`rig/address.rs`) replaces `scoped`, `split_ref` and `graph.rs`'s `strip_suffix(":@gain")` / `contains('@')`. One real change: `Rig.sticks` is authored in compiled names (`voice_voice.ratio`) and becomes `voice:voice.ratio` (SQL migration, E2, and the core TOML). A track id that could collide once flattened (`a_b` + `c` and `a` + `b_c`) stays an error from `Graph::build`.

## 4. Context: what is shown where
- **How the flow editor scopes (creator/src/lib/components/flow/, shared/content/src/flow.ts):** one docked panel (`flow-dock`) for every level: toon, season, chapter, group, slice. The level follows the selection (`deepest` in `flow-panel`); a small switch offers only the levels along the current selection, the author can step up (`picked`), and a new selection follows again. Each level is its own graph (`toonFlow`, `chapterFlow`, `groupFlow`, `sliceFlow`). What crosses the level's edge is drawn for context as a `stub` (the groups a reader comes from, the group of a slice), never expanded; `exit` boxes leave the level. The box of what is open is marked and centred, the zoom carries across levels, a click opens the box's page or `path`. It is read-only and the Flow only reads the place.
- **Carries over:** the level chain, following the selection, the switch back up, stubs at the edge, marked current box, kept zoom, one panel. **Does not:** the Flow computes its graph in TS; a wiring view is computed by the core. Svelte Flow is not needed (nodes.ts places already).
- **The context is an address prefix,** so depth is unlimited by construction. `wire_view(sources, context)` answers what the level holds. Scenes do not nest in the code today (`Scene` is flat) and the scene editor is not designed yet, so the top row is the rule waiting for it: its own ticket after the scene editor's design note.

| Context | Shown as boxes | Stubs at the edge | A new cable may join | Read-only |
|---|---|---|---|---|
| scene (later) | holders, the scene's nodes | what the scene's parent feeds | those, written in the scene document | a holder's inside |
| rig (`character:catpope`) | tracks, policy, being | the mixer sum | track ports, policy, being; writes `route` or the fence | the mixer sum; a policy's owned inputs |
| track (`…/analog`) | the instrument's nodes | the rig's cables into and out of this track | nodes of this instrument; writes `link`, "used by N" first | a preset's `links` (shown, marked) |
| module (`…/analog:synth`) | its ports and, with #419, its internal groups | the cables at its ports | none | everything: internals are Rust |

**The rules:**
1. **Visible:** the children of the context and their ports; a cable crossing the context's edge ends in a stub named by its address relative to the context, one hop only. Going in on a box makes it the context.
2. **Reachable:** a cable is drawn and edited at the lowest level that holds both its ends, which is also the document it is written in. To join two tracks, go up to the rig. Beyond that, `Reach` (§3) caps it.
3. **Read-only,** with the reason on the cable: compiled (the mixer sum); no fence; `owned` by a policy (`set` / `mul`, RIG-MODULE-GROUPS Q5); the definition's `Rights` do not allow a change (lent, core: derive first); module internals.
4. **Moods:** the view shows the rig without a persona or mood, and the mood pinned in the preview (EDITOR.md §1) adds the preset re-routes it causes, marked.

## 5. The editor's interaction
**Core surface (three calls, any GUI; Svelte holds only placement and the gesture, new bridge lines go into `scripts/bridge-files.json`):**
- `wire_view(sources, context)`: nodes, typed ports, cables with `id`, `home`, the `slot` for the live value (as `WireLink.value` today), `readonly` with its reason, stubs, levels.
- `wire_targets(view, from)`: the addresses this port may join, asked when a cable is picked up. The GUI highlights those and knows no rule.
- `wire_edit(sources, op)`, with `op` = connect · disconnect · set scale / offset · add node · remove node: the changed source and `Answer`.

**Type checks (all in the core):** audio to audio (an input sums), control to control (one link; "already has a link"), event to event (many to many, new `PortType::Event`), video stays out of a rig. Each input has one owner (Q5 of RIG-MODULE-GROUPS). A loop needs a `feedback` node. A refused gesture writes nothing and shows the core's sentence at the port.

**A failed compile:** the cable stays dashed "not built" and the source keeps the edit. For data, `Problem { at: address, text }` (replacing the bare strings of `Answer`) marks the cable, and every problem is listed instead of the first. For Rust, cargo's errors go to the Rust tab's problems, with one line in the Nodes tab linking there. Undo is the draft's version history.

**What plays while a compile runs:** the preview keeps the last good build; a compile only replaces it on success (a data rig through `Played::diff`, or a reload if the shape changed; a Rust build by rebuilding the preview's engine). A draft compile never reaches a deck: nothing is sent, the compile service already refuses while a show is live (`deck/build compile.rs live_show`), and there is no patch panel on `/live/deck` and no overlay on the stage. Until #290 the browser preview plays no module at all and `Engine.note` says so; a native editor (#393) would change only that line.

**The Nodes tab grows in place:** first its data moves from `Played::wiring` to `wire_view` (levels, events, homes); then gestures on the same boxes. Live values stay what they are today (the engine's frame by slot).

## 6. Slices and questions
| # | Ticket | Size | Done when |
|---|---|---|---|
| W1 | **Write-back proof:** `wire_edit` connect / disconnect on a rig `route`; drag one cable at track level in the Nodes tab; Build = validate; the preview takes it | S | a dragged route is in the draft document, a wrong type is refused by the core, a reload shows the same cable |
| W2 | Event ports in the graph, event cables in `route` / `link`, `evhost` joins by them; `Instrument.out` optional (a silent track); stepseq → analog drawn and rewired (finishes #425 slice 3) | M | two sequencer + synth pairs in one rig stay apart (headless, native) |
| W3 | The fenced `#[port]` table: template, line edits, `real_compile` | S | retarget a policy output by a cable; hand-written code byte-identical in a test |
| W4 | `rig/address.rs`, `Reach`, `Problem.at`, the `sticks` migration | M | one parser in the core; all problems listed with their cable |
| W5 | `wire_view` by level, stubs, `home`; the Nodes tab's level switch; the old `Played::wiring` removed | M | the rules of §4 tested in the core |
| W6 | Add / remove nodes, instrument `link` editing with "used by N", the TOML view of a document | M | a rig built from modules with no hand-edited file |

**For the owner (my recommendation first):**
1. **Two file forms, one model:** a `path` output's cable lives in the Rust fence, all others in the document. *Recommend as written.* The alternative is that policy outputs lose `path` and every cable is a `route`: one home and no Rust write-back, but it undoes #408's targets (`mode`, `@preset`, `@gain`) and the "code as concrete wiring" you asked for.
2. **Event wiring by name:** inside a rig, module to module becomes explicit cables only, and the world (MIDI, generators) stays by name. *Recommend explicit,* because a by-name join is a wiring written nowhere; `catpope_analog` gains one line. The alternative keeps by-name as the default for an input without a cable.
3. **A note source as a track:** a silent track (no outlet, no strip) wired by event cables. *Recommend silent track,* since one sequencer can then play two synths and no shared instrument is edited. The alternative stays with a sequencer node inside the synth's instrument (`catpope_analog` by hand).
4. **When data builds:** on every gesture (it takes milliseconds), Rust on the Compile button. *Recommend as written.*
5. **By hand for data:** a TOML view, stored as JSON as today, comments not kept. *Recommend TOML:* the core already reads it. The alternative is that the graph and the form are the only editors of data.
6. **The first reach:** the rig on the empty rigging stage, then the deck's active scene, rigs holding only relative addresses. *Recommend as written.*
