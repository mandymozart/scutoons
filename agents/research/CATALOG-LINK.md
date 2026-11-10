Status 2026-11-10: concept, proposal, nothing built (#472). Drafts 8–10 were sent to the owner in the chat. The owner answered the four points the same day (§6); §7–§9 follow from the answers.

# CATALOG-LINK – the catalog (creator) and the show (editor): who holds what, and the link

Owner 2026-11-10: *"i am still missing the link to the svelte editor/catalog and assignments to characters and personas moods, locations items. etc. if this is not in the plan, lets make design drafts now so we can move through the development. well organized less guessing later on."*

## 1. What exists, what is missing

- The creator (Svelte, browser) holds who and what exists: characters with personas and looks, locations, items, the story with its events. The native editor holds what plays: rigs, pieces, shows (SHOW.md §3).
- Planned one by one: the character → rig Assignment tab (#283, RIG-EDITING.md §2 tab 4), what lives in which editor and the shared `/edit/<key>` address (#447, NATIVE-EDITOR-GUI.md §5), rig storage with the subject `character.<id>[.<persona>]` (#258), the venue cascade (#330).
- Missing: one picture for **every** kind. A show addresses `character › persona › look`, items and scenes (SHOW.md §8), but nothing says what a location or an item plays, where a handler or a light look of an entity is set, which shows use an entity, or how one editor opens the other.

## 2. One key on both sides

`character:<id>[.<persona>[.<look>]]`, `location:<id>`, `item:<id>`, `scene:<id>`, `rig:<subject>`, `show:<id>`. The same `/edit/<key>?w=<workspace>` opens in the creator or in the native editor. `location:` is new as an address place (SHOW.md §11: `address.rs` has none).

## 3. The assignment: one per entity

One document beside the entity says what it plays:

| Part | What | State |
| --- | --- | --- |
| Rig | tracks, presets per mood | #283, built for characters |
| Body | 3D model, views per mood; a location's scene | exists as assets |
| Handlers | the words it answers (`dance:samba`) and what each plays | new |
| Light | a look of fixtures | new, after #91's look editing |

Read down the ladder: look → persona → character → the kind's default → the standard we ship (the same order as the handler lookup, SHOW.md §8). A cell of a show sets values for one row only; the assignment is the entity everywhere. A member's draft plays on their own decks; published is everyone's. One validator and one resolver in the core (`deck-space`), no logic in TS.

## 4. The surfaces

- **The assignment sheet** (draft 9): one table over characters (personas and looks indented, moods as a line), locations and items; columns Rig · Body · Handlers · Light · Used in; set / inherited / missing; filters "missing only", "used in show X". The same sheet in both editors.
- **Creator** (draft 10 left): a "Plays" tab on the character, location and item page: what it plays, what is missing, the personas, used in shows (rows, cells), the story's events, "Open in editor", "Play on my deck". The lists mark who has no rig, no body, no handler a show asks for.
- **Editor** (draft 10 right): the catalog picker in a cell, an address or an assignment (search, Enter puts the key, Tab goes one level down the ladder; the list comes from the API and is kept for offline); "Open in creator"; the show's problems list links to the assignment ("catpope has no handler for dance:samba").
- **The split**: the editor never edits profile, story or assets; the creator never edits wiring. Review and publish stay in the creator.

## 5. Build order (proposed; each step its own ticket once §6 is answered)

1. The assignment document, its resolver and validator in the core; `location:` as an address place; the API key `assign:<subject>`.
2. "Used in shows": the API indexes a show document's entity keys on save.
3. Editor: the catalog picker and the assignment sheet (takes in #283's tab, for all kinds).
4. Creator: the Plays tab, the missing marks on the lists, the deep links both ways.
5. Handlers and light per entity.

## 6. The owner's answers (2026-11-10) and what follows

Owner: *"1. evaluate long term consequences of inside the rig vs document. since document seems far more general and inside the rig is more detailed and would allow for much tighter integration 2. also here we need to investigate long term consequences. a mood is a value but currently has invididual entries since we need images for each. long term, i think it is rather mapped to a 3d character and will replace those images or make them redundant. 3. exactly on set and a scene as we discussed somewhere earlier can spawn inside the location at points and additional items can be put in to enrich the scene. a location can be between the apartment blocks for example and it has for example large appartment blocks in a circle, a call could be to show them destroyed or start a decay animation, all scenes in that scope would inherit the behavior and a scene in entrance A would also be affected. generally maybe it is time we review real time open world games and how they handle things like this. maybe we can learn a thing or two for our architecture. to prevent inventing and keep ing things in sync various times. 4. they can pick not touch. edit"*

1. **Decided: a location is exactly one set.** Scenes spawn inside it at points; further items are put in to enrich a scene. A call to the location (show the blocks destroyed, start a decay) reaches every scene in its scope, a scene at entrance A included (§8).
2. **Decided: the creator picks, it does not touch.** Picking a rig, a body or a look from a list is the creator's; editing them is the editor's.
3. **To evaluate, done in §7:** the assignment inside the rig or as its own document; what a mood is in the long run.
4. **Asked for: a review of how real-time open-world games handle this** (§9, a first pass from knowledge, not checked against sources).

## 7. Long-term consequences

### 7a. The assignment: inside the rig, or its own document

| | Inside the rig (#283 as it is) | Its own document per entity |
| --- | --- | --- |
| Integration | Tight: tracks, mood presets and routes are checked and baked with the rig, one publish, one version | Loose unless something joins them: two documents can disagree (a mood the rig has no preset for) |
| Entities without sound | A location with only light, an item with only a body needs an empty rig to hang things on | Nothing to invent: a part that is not set is simply not set |
| Sharing | A rig used by several characters (the standard rig per instrument kind) cannot carry each one's assignment | Rigs stay reusable; the entity says which one it plays |
| Growth | Every new part (body, handlers, light, later cloth or behaviour) changes the rig's schema and re-bakes sound for a change that has nothing to do with sound | A new part is a new row |
| "Used in", missing defaults | Asked of rigs, answers only for entities that have one | Asked of entities, answers for all |

**Recommendation: both, each where it is strong.** The entity's document is thin: which rig, body, handlers and look it plays, and its own overrides. The detail stays inside the part (the rig keeps its tracks, mood presets and routes; #283's tab edits those). The tight integration is made at publish, not in the schema: the baker (RIG-EDITING.md §9b) resolves the entity's document and its parts into **one entity image**, validates them against each other (a mood without a preset is an error there) and the deck reads only that. Authoring stays general, the hot path stays tight. This is how the engines do it: an actor or prefab is a thin list of components, each component keeps its own data, and a cooked form is what runs (§9).

### 7b. Moods

Today a mood is an entry per character because each needs its image (`<character>_<view>-<mood>.png`). The owner expects a 3D character to carry the mood and the images to become redundant.

- **If a mood were a level of the address ladder:** every mood multiplies the keys, the bakes and the assignments; two moods cannot be blended; and when the images go, the level stays behind as dead structure.
- **If a mood is a value of the entity (proposed):** `mood` is a parameter like any other, addressed and mapped by the same rules (`character:yellowmouse > mood:angry`, or a vector of feelings, #188). The named moods are **named points** of that value. What a point selects is the part's business: the rig a preset diff, the 3D body a pose or blend shapes, and the 2D view the nearest image. An image is then one renderer's fallback for a named point, not what a mood is; when the 3D character carries it, the images stop being required and nothing else changes.

**Recommendation:** no ladder level. Mood is a value with named points; the entries that exist today become those points, their images stay attached as the 2D fallback. Nothing has to migrate now; the address grammar must only never grow a mood level.

## 8. Locations, scenes, scope (owner 2026-11-10)

- **A location is one set**: its geometry, its fixtures, its own state (intact / destroyed, the hour, the weather).
- **Spawn points** are named places in the set (`entrance-a`, `courtyard`). **A scene spawns at a point**: its cast, its items, its events, placed relative to the point. Further items can be placed into the scene to enrich it. A placement is a reference to the catalog item with its own overrides, never a copy.
- **Scope is the tree** `location › scene › entity`. A word to a location (`location:blocks > destroyed`, `> decay`) is heard by the location and by everything in its scope; a scene at entrance A shows the destroyed blocks and may answer the word itself (its lights flicker, its cast reacts).
- **Durable things are state, not events.** "Destroyed" is a value on the location, so a deck that joins later, or a scene that spawns later, sees it. "Start the decay" is the event that moves the value. This is the same split as cues and state on the relay, and KANON's story variables are the same thing one level up.
- New against what exists: `location:` as an address place, spawn points in the set, scope inheritance for words and state, placements as references with overrides (LOCATION-SCENES.md §"spots" has the shape for pickups).

## 9. What open-world games do (first pass from knowledge, 2026-11-10; not checked against sources, a sourced review is the next step if wanted)

| Pattern | Where | What it is | Ours |
| --- | --- | --- | --- |
| Base object and placed reference | Bethesda's Creation Engine (worldspace › cell › reference) | The catalog holds the object once; the world holds references with a transform and overrides; changes are saved as differences against the base | Catalog item vs placement in a scene; the member's layer as a diff |
| Prefab, variant, nested prefab | Unity | One override chain: variant over base, instance over variant | The ladder look › persona › character › kind default |
| Composition by layers, variant sets | USD (Pixar; film and game pipelines) | Layers of opinions with a fixed strength order; a variant set switches a subtree (intact / destroyed) | Mapping layers precoded < flashed < local; location states; moods as named points |
| Level instances, streaming, data layers | Unreal (World Partition) | A level placed in the world at a transform; cells load by distance; a data layer switches a whole version of the world on and off at run time | A scene spawned at a point; reach and loading ahead; "show the blocks destroyed" |
| Hierarchical tags and tag queries | Unreal Gameplay Tags | Dotted names (`Location.Blocks.EntranceA`) matched by prefix | The address ladder and the filters (SHOW.md §8, §9) |
| Calls down a subtree, groups | Godot (`propagate_call`, groups) | A call on a node reaches its children; a group is addressed as one | A word to a location reaching its scenes; `group:` in filters |
| Entity and components | ECS (Bevy, Unity DOTS), Unreal actor components | An entity is an id and a list of parts; each part owns its data | The thin entity document of §7a |
| World state, phasing | MMO phasing, global variables in quest systems | The world's state is a set of variables; what a player sees is a function of them | KANON's variables; state on a scope (§8) |
| Server-timed live events | Live-service games | The server holds the state and the clock, every client plays the same change at the same time, a late joiner gets the state | The conductor, flash and the show clock |
| Relevancy, interest management | Networked engines | A client is sent only what is near or relevant to it | Reach (SHOW.md §6) |
| Cooked data | All of them | Authoring data is resolved once into the form the runtime reads | The baked rig image, the entity image of §7a |

**What to take from it:**
1. **One override mechanism, used everywhere** (base → variant → instance → local), instead of one per feature. We have three that are the same idea: the ladder, the mapping layers, the member's diff. They should be one resolver in the core.
2. **State lives on a scope and is inherited; events only move it.** That is what keeps late joiners and spawned scenes in sync without resending history.
3. **Placement is a reference with overrides**, never a copy, so the catalog stays the single source.
4. **Variants for whole-world switches**: a location's states are a variant set of its set, not separate locations.
5. **Author general, run cooked**: one resolved image per entity and per location is what a deck reads.

## 10. Still open

1. §7a and §7b are recommendations until the owner says yes.
2. Is the sourced review of §9 wanted as its own research task (engine documentation read and cited, one page per pattern), before the core resolver is built?
3. Can a scene be spawned at several points at once, and can two scenes share a point?
