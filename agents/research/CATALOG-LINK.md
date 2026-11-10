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

**Decided (owner 2026-11-10: *"the entity document is a good call from the proposal"*): both, each where it is strong.** The entity's document is thin: which rig, body, handlers and look it plays, and its own overrides. The detail stays inside the part (the rig keeps its tracks, mood presets and routes; #283's tab edits those). The tight integration is made at publish, not in the schema: the baker (RIG-EDITING.md §9b) resolves the entity's document and its parts into **one entity image**, validates them against each other (a mood without a preset is an error there) and the deck reads only that. Authoring stays general, the hot path stays tight. This is how the engines do it: an actor or prefab is a thin list of components, each component keeps its own data, and a cooked form is what runs (§9).

### 7b. Moods

Today a mood is an entry per character because each needs its image (`<character>_<view>-<mood>.png`). The owner expects a 3D character to carry the mood and the images to become redundant.

- **If a mood were a level of the address ladder:** every mood multiplies the keys, the bakes and the assignments; two moods cannot be blended; and when the images go, the level stays behind as dead structure.
- **If a mood is a value of the entity (proposed):** `mood` is a parameter like any other, addressed and mapped by the same rules (`character:yellowmouse > mood:angry`, or a vector of feelings, #188). The named moods are **named points** of that value. What a point selects is the part's business: the rig a preset diff, the 3D body a pose or blend shapes, and the 2D view the nearest image. An image is then one renderer's fallback for a named point, not what a mood is; when the 3D character carries it, the images stop being required and nothing else changes.

**Recommendation:** no ladder level. Mood is a value with named points; the entries that exist today become those points, their images stay attached as the 2D fallback. Nothing has to migrate now; the address grammar must only never grow a mood level.

**Owner 2026-11-10:** *"for moods i think it is still relevant to call a specific mood ... so have all characters across all decks start laughing for example is a very important event"*. So a named mood is **a word that can be called**, by name, to one entity or to all of them: `character:* > mood:laughing` reaches every character on every deck in reach, like any other word of a show (a cell, a cue, the conductor's hand). This binds the model:
- Every named point is callable by its name; a call is an event that moves the value (at once or over a time), and the value it leaves is state, so a deck that joins later has the characters laughing too.
- A call to all is one message with a wildcard address, not one per character; each entity answers through its own parts (the rig's preset, the body's pose or image), and an entity that has no such point falls down the ladder to the kind's default.
- Whether a call is a lasting mood or a passing one (laugh, then back to what it was) is said by the call (`for 4 bars`), not by a second mechanism.

## 8. Locations, scenes, scope (owner 2026-11-10)

- **A location is one set**: its geometry, its fixtures, its own state (intact / destroyed, the hour, the weather).
- **Spawn points** are named places in the set (`entrance-a`, `courtyard`). Two scenes can share a point (owner 2026-11-10). **A scene spawns at a point**: its cast, its items, its events, placed relative to the point. Further items can be placed into the scene to enrich it. A placement is a reference to the catalog item with its own overrides, never a copy.
- **Scope is the tree** `location › scene › entity`. A word to a location (`location:blocks > destroyed`, `> decay`) is heard by the location and by everything in its scope; a scene at entrance A shows the destroyed blocks and may answer the word itself (its lights flicker, its cast reacts).
- **Durable things are state, not events.** "Destroyed" is a value on the location, so a deck that joins later, or a scene that spawns later, sees it. "Start the decay" is the event that moves the value. This is the same split as cues and state on the relay, and KANON's story variables are the same thing one level up.
- New against what exists: `location:` as an address place, spawn points in the set, scope inheritance for words and state, placements as references with overrides (LOCATION-SCENES.md §"spots" has the shape for pickups).

## 9. What open-world games do → research/OPEN-WORLD.md

The first pass that stood here was written from memory and was wrong or imprecise in 12 places (OPEN-WORLD.md §5). The review checked against the engines' documentation, the patterns that hold, where our code stands and the refactor that follows are in **research/OPEN-WORLD.md**. In short: one resolver in the core for every chain that falls back or overrides; a word says whether it is state or an event and how long it lasts (a passing mood is a timed modifier over a base value, with an onset and a while); a spawn point is a slot that is claimed; a placement is a reference with overrides keyed by stable ids; many things follow one switch before a second set is made; authoring data is baked.

## 10. Still open

1. Decided 2026-11-10: the entity document (§7a); a named mood can be called, to all at once (§7b); two scenes can share a point (*"two scenes can share a point of course"*).
2. The sourced review of §9 is ordered (owner: *"yes do the research and visualize your findings to me and consequences for a refactor"*): done, research/OPEN-WORLD.md (its §6 has the open points), drafts sent in the chat.
3. Still open: can one scene be spawned at several points at once?
