Status 2026-11-10: concept, proposal, nothing built (#472). Drafts 8–10 were sent to the owner in the chat. Four points are the owner's call (§6).

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

## 6. Open, the owner's call

1. One assignment document per entity (proposed), or does it stay inside the rig as #283 has it for characters?
2. Moods: states of a character that select presets and views (proposed), or their own level of the address ladder?
3. A location's body: exactly one scene (proposed), or several?
4. May the creator set simple assignments itself (pick a rig or a look from a list), or is it read-only there with "Open in editor" (proposed: pick from lists, yes; wiring, no)?
