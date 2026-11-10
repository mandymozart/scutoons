Status 2026-11-10: research done, proposal for a refactor, nothing built (#472). Ordered by the owner; drafts 1–3 were sent in the chat.

# OPEN-WORLD – what real-time engines do, checked against their documentation, and what it means for our core

Owner 2026-11-10: *"generally maybe it is time we review real time open world games and how they handle things like this. maybe we can learn a thing or two for our architecture. to prevent inventing and keep ing things in sync various times."* · *"yes do the research and visualize your findings to me and consequences for a refactor"*

## 1. How it was checked

30 pages opened and read (Unreal 5.9, Unity 6, Godot stable, OpenUSD, Bevy 0.20, Unity Entities 1.3, the community GAS documentation). **Not verified:** Bethesda's save format and quest aliases (the wikis answer 403), Unreal's API reference pages (render empty), GTA's map swaps (forum level), Fortnite's live events (no primary source). What a late joiner gets in Unreal is inferred from the pages read, not stated on one. The source list with a confidence mark per point is kept outside the repository (the session's notes, `open-world-sources.md`); it replaces CATALOG-LINK.md §9, which was written from memory and was wrong or imprecise in 12 places (§5).

## 2. Findings that hold

| # | Pattern | Who, their word | What it is | For us |
| --- | --- | --- | --- | --- |
| 1 | State and event are two calls | Godot `set_group` / `call_group`; Unreal replicated property / multicast RPC | State reaches whoever is there and whoever comes later; an event reaches only who is present and in range | A word says which it is. The relay already splits snapshot from log (`relay/src/session.rs` `snapshot_for`); the show's words do not say it yet |
| 2 | Onset and while | Unreal GAS cues: `OnActive` / `WhileActive` | One effect, two faces: a one-shot for those present at the start, and a "be in this state" that also runs for a late joiner and on coming back into range | `mood:laughing`: the burst is the onset, the laughing is the while |
| 3 | A passing change is a timed modifier over a base value | GAS: base value / current value, effects with a duration, stacking rules | Nothing is pushed and popped; when the modifier ends the value is the base again; a rule says what a second call does (refresh, extend, cap) | "laugh for 4 bars, then back": no second mechanism, no state stack |
| 4 | One order of strength for all opinions | OpenUSD: LIVERPS (local, inherits, variants, relocates, references, payloads, specializes) | Every value is resolved by one fixed order across all kinds of source; `specializes` is the weakest: a fallback anything beats | One resolver: local › flashed › draft › look › persona › character › kind default (our `specializes`) › shipped |
| 5 | Placement is a reference with its own data | Creation Engine: base object / reference; Unity: prefab instance overrides | The catalog has the thing once; the world holds references with a transform and overrides. Unity's overrides on lists are keyed by index and break on reordering | A placement references the catalog item; overrides are keyed by stable ids, never by position |
| 6 | Shared instances cannot be overridden | USD instanceable prims; Unreal Level Instances (editor-only by default, edits reach all copies) | Sharing and overriding exclude each other | A scene spawned at a point is a placement with overrides, not a shared instance |
| 7 | Points are slots that are claimed | Unreal Smart Objects (free › claimed › occupied; released when the object changes state); Player Start with a tag and a fallback | A shared definition with slots; a user finds one by tag, claims it, releases it | Two scenes share a spawn point without colliding; a scene asks for a named point and falls back to a free one |
| 8 | Values sent once on appearing | Godot synchronizer: per property "spawn" vs "always" / "on change" | What a peer needs when a thing appears is separate from what keeps flowing | A scene spawned later: its cast and first mood once, then live values |
| 9 | Many things follow one switch | Creation Kit "Enable Parent"; Unreal runtime Data Layers (server sets the state, it replicates) | A data layer is two sets of actors switched as a whole; an enable parent is one marker that many placements follow | "Blocks destroyed": start with a switch group inside one set; a whole second set only when the geometry really differs |
| 10 | What is in range is a per-peer filter | Unreal relevancy and streaming sources; Godot visibility filters | Out of range: no events, spawned things are removed; back in range: the state returns, the events do not | Reach behaves the same; this is why durable things must be state (1) |
| 11 | Down and up are two directions | Godot `propagate_call` (down, children first by default); Bevy entity events (bubble child to parent) | Both exist, both are named | A word to a location goes down to its scenes; a scene's report goes up |
| 12 | Authoring data is baked | Unity DOTS baking (editor only, incremental or full) | The runtime never reads authoring data | The entity image (CATALOG-LINK §7a), #296 |

Not a model for us: **MMO phasing** is per player (each sees a world by their own progress and not the others); it fits a per-deck variation at most, not one shared show state.

## 3. Where our code stands (map of 2026-11-10)

- **One address parser** (`deck/space/src/rig/address.rs`, 482 lines, used by `deck/show/src/address.rs`). Place kinds are free strings; only `character` is special-cased; `location`, `scene`, `item` have no type. The `*` wildcard exists.
- **Seven chains that fall back or override, sharing no code:** the preset lineage fold (`rig/preset.rs`), the persona and mood variants (`rig/mod.rs` `play_with`, `rig/policy.rs`), the editor's draft over a bundle (`editor/src/wires.rs`), the flash rule (`show/src/flash.rs`), the vote override (`relay/src/vote.rs`), **the rig lookup persona → character in PHP** (`server/src/Domain/Library/SceneContent.php:142`), **the image fallback in TS** (`shared/content/src/characters.ts:684`, six steps). The last two are rules outside the core.
- **Decided but not built:** the handler lookup look → persona → character → kind default; the mapping layers precoded < flashed < local (INPUT.md §6b); the rig baker (#296). Resolution happens at run time today.
- **State on the relay exists**: `Session` keeps cells, tempo, each deck's mood and more, and `snapshot_for` gives it to a late joiner. A mood is a plain string on a deck; there is no state per entity, location or scene.
- **Sets have spots** (`space/space/glb.rs`: empty nodes named `spot_*`); there is no location, scene-in-location, spawn or placement type in Rust. Placements and respawn live in the API (`ItemPlacements.php`, `ItemRespawn.php`).

## 4. Consequences: the refactor, in the order to do it

The main finding: **three more chains are about to be built** (handlers, mapping layers, the baker's entity image). Building them on one resolver costs little now; refactoring ten chains later costs a lot. So the refactor comes first, and it is small.

1. **One resolver in the core** (`deck-space`). Input: a key and an ordered list of sources, each with a strength. Output: the value and where it came from ("inherited from the character"). Order fixed in one place: local › flashed › a member's draft › look › persona › character › the kind's default › shipped. Overrides keyed by stable ids. First users: the handler lookup and the mapping layers (new, built on it), then the persona / mood variants and the preset fold (moved onto it). The rig lookup in PHP and the image fallback in TS become calls to it or data it produced; until then each is a ticket (AUDIT.md §7).
2. **Words say state or event, and how long.** `set` puts a value on a scope: kept, in the snapshot, inherited. `call` is an event for who is present. A named mood called with a length is a timed modifier over the base value; when it ends the base shows again; a rule says what a second call does. An effect has an onset and a while. On the wire: state per entity and per scope in the relay's `Session`, beside the deck's mood that exists.
3. **The world tree as types.** `location` (one set, its state, its switch groups) › point (a slot: named, tagged, claimed and released) › `scene` (spawned at a point; what is sent once on spawning vs live) › placement (a reference to a catalog entity with overrides). `location`, `scene`, `item` become address places with their own rules. Words go down the tree, reports go up.
4. **The entity document and its baked image** (CATALOG-LINK §7a, #296): the resolver of step 1 run at publish over the entity and its parts.
5. **A location's variants**: switch groups in one set first; a second set only where the geometry differs.

Each step is its own ticket once the owner agrees to the order.

## 5. What was wrong in CATALOG-LINK §9

Bethesda's save stores changes per placed reference, not differences against the base object (search level). Unity's overrides are a tree (nesting and a chosen target), not one chain; the closer model for our ladder is USD's inherits and specializes. USD's order is across kinds of source, layer order only decides within "local". An Unreal Level Instance does not exist at run time by default. A Data Layer holds actors and has states set on the server; it does not swap values. Gameplay Tags match per dotted segment, not by string prefix. Godot's `propagate_call` runs children first by default. MMO phasing is per player. The live-events row has no source.

## 6. Open

1. The order of §4: is step 1 (the resolver) the next build ticket, before #283's remaining work and the handler lookup?
2. What a second call of a running mood does: start over, extend, or be ignored. Proposed: start over.
3. Can one scene be spawned at several points at once (still open from CATALOG-LINK §10)?
4. Bethesda's quest aliases (a scene borrows existing entities for its length and adds behaviour to them) look like the right model for a scene's cast, but could not be read; to verify by hand if wanted.
