<!-- status: concept for the owner's review, 2026-11-24 – the extended Library (things, files, assignments) and the recap of what it makes redundant. The model's core is decided (marked); the list of what goes is partly decided, partly the agent's consequence to confirm (marked). Answers F1 (Catalog and Library as one concept). Facts about today read from shared/media/src/library, creator/src/lib/components/library and the pickers. -->
# The extended Library

## The owner's words (2026-11-24)
- *"Cast picker is a bit unclear still. We do have a conceptual gap between what's in the Library and what's in the specific view … (tree). Let's try to clear it up now."*
- *"So I am right, the cast now resides in the Library."* · *"Yes, kind of makes no sense to list scenes in their own scenes-only view."*
- *"Well, but a cast is a character assigned to a toon. So characters get listed alongside their assets like models. Unreal has stuff like Character, CharacterPersona, CharacterMood etc. How should we handle this and keep our go-to places to find stuff about narratives and content quickly and reliably?"*
- On personas, looks and moods inside the character: *"Yes, stay inside character. Makes total sense."* On the cast found in the Library and edited in the toon: *"Yes. But edited in the toon means we can still just open it and come to the character workspace to edit it."*
- *"We need to recap and kick out redundant concepts and redraw many screens now, due to the … extended Library."*

## Today
- The Library lists **files** only: image · video · audio · model · animation · file. It narrows by scope (what the open item uses · toon · all · unused) and by places – among them "By cast", one per season's cast, showing the files of its characters.
- **Things** (characters, scenes, items, places) are a second list beside it ("items to open"), and again in trees: the Scenes tree lists every scene, the Catalog tree its kinds and entries, the Toon tree the toons.
- **Picking** has five sheets of its own: characters, items, assets, people, roles.
- **The cast** is edited from each character's Cast tab, one character at a time. A season has no view of its whole cast.

## The model
**Decided**
1. **The Library lists things and files.** A thing is a document with a name, a type and an address – a character, a scene, a toon, a place, an item, a show, a module. A file is a picture, a model, a sound, a video.
2. **A thing is stepped into; its parts are not entries of their own.** A character is one entry. Inside it: its personas, then looks, then moods – and beside them the files it uses (the model, the portraits, the voice). Each level keeps its address.
3. **An assignment is never an entry.** A cast is characters assigned to a toon's season. It shows as a place in the Library (the cast's characters and their files) and as "used in" on the character. It is edited in the thing that holds it: the toon's season.
4. **Opening a thing goes to its workspace.** A character opened from a cast comes up in the Characters workspace, where it is edited.
5. **The Scenes workspace has no list of scenes of its own.**

**Follows from it – to confirm**
6. **Three go-to places, always the same:** Search (by name, across everything) · the Library (browse by type, narrow by place) · the tree of what is open (what this thing is made of).
7. **A picker is the Library opened from a slot:** what fits the slot, what this thing already has first, everything else after.
8. **"What uses this, what does this use"** on every entry.

## What goes – the recap
| What | Why | State |
|---|---|---|
| The Scenes workspace's Tree of all scenes | the Library filtered to scenes | decided |
| A list of characters beside the Library | the Library lists them | decided |
| "Import images" in the Map, the Catalog and Characters | MCP for bulk; the Library places into slots | decided |
| The Library's "To do" group: Blank – wanted · Missing versions · Missing credits | a gap may be on purpose | decided |
| The Bones and Clips lists in the creator's Rig panel | the deck's asset view has them | decided |
| The Show's "fires on" sheet and its own pill in the bar | the row's ⚡ slot; views | decided |
| The second list of things beside the files ("items to open") | one Library | decided |
| The Catalog tree's and the Toon tree's top level (every entry, every toon) | lists of what exists are the Library's | decided |
| The five picker sheets (character, item, asset, person, role) | the Library opened from a slot | decided |
| Editing the cast from each character's Cast tab | the cast is edited in the toon's season; the tab shows "cast in" and links there | decided |
| "By cast" showing files only | the place lists the characters and their files | decided |
| Trees that travel between workspaces (TREES.md, point 4) | referencing is done from the Library | decided |

## What stays
- **A tree for what a thing is made of:** a toon's seasons, chapters and cast; a chapter's slices; a show's rows; a rig's tracks; the map's layers, places and routes.
- **Flow** for the story's structure, **Kanon** for the written canon of a subject – reached from the entry.
- **The Inspector** for the selected thing's values.

## Screens to redraw
| Screen | What changes |
|---|---|
| C12 Library (and F1) | things and files in one list; types as the first filter; places (this item · this season's cast · this toon · everything); stepping into a character; "used in"; video tiles with a preview picture; no "To do" group |
| A new screen: a slot's pick | the Library opened from a "who" field: the cast first, everyone else after |
| C1 Toon | the tree of one toon: seasons, chapters, the Cast branch; the cast edited here |
| C4 Characters | the Cast tab as "cast in" with links to the seasons |
| C5 Scenes | no Tree; the preview across the back |
| C9 Catalog | an entry open; entries found in the Library |
| A4 Search | finds things and files |
| Every screen that shows the Library panel | the same panel |
| The deck's Library (D2, D3, show.html, the Spores pages) | the same Library |

## Answered by the owner (2026-11-24)
1. **The types of thing the Library lists:** Toon · Character · Scene · Show · Place · Item (with recipes, products, editions, albums, home scenes) · Module · Instrument and rig · Generator – and, for now, Kanon texts and people (for credits) too.
2. **Built in lap 2.**
3. **A video's tile is one frame.** Playing on hover is not needed at this point.
4. **The deck's Library is the same extended Library, with the same panel.**
6. **The five consequences drawn with a question card stand as drawn:** one list ("items to open" goes); the Catalog's and the Toon's trees without their top level; one picker, the Library opened from a slot; the Cast tab as "cast in"; "By cast" lists the characters and their files.
5. **The type filter is one quiet select** ("All types"), its list in two groups – things and files. The places stay a row of chips.

## Open
Nothing in this concept. What dropping a Toon, a Show, a Generator, a Kanon text or a person does, and where each opens, is asked in PLAN.md (P11).

**Roles (#882, read from answer 1 – the owner may overrule):** roles are no type of the Library – its list names people for credits, not roles – so the credits' role chooser (`credits/role-picker.svelte`) stays a chooser of its own: one control, not a second picker.
