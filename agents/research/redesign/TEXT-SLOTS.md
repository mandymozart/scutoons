Status: analysis 2026-11-20 for #673 (owner: "analyse the creator concerning text and slot based edit views … which could be panels that we need in parallel … what is meta data we need to read more kanonical rather than additives … the deck is basically linked through this and the events") – feeds draft round 2.

# Text and slot edit views, canonical metadata, the link to the deck

`c/` = `creator/src/lib/components/`, `cat/` = `shared/catalog/src/components/`. Drafts: `workspaces.html` (round 1).

## 1. What exists – 30 edit views, one frame

Every subject is edited in a stack of forms: prose (descriptions, Kanon, scripts, prompts), short meta (names, dates,
flags, enums), slots (images, video, audio, models) and links (pickers, plain `<select>`s, the address typeahead).
`c/page/meta-page` is the shared frame of ~11 of them (Save / Publish / History / Problems).

| Subject | Edit views today (~lines) | Prose | Meta | Slots | Links |
|---|---|---|---|---|---|
| Character | identity 170, persona 115, inspector 227 (profile · model · wanted · prompts · hidden kanon), persona-model 81, prompts 104, kanon-panel 102, moods 136, cast 397 | description (LM) ×2, Kanon | name, colour, testo, tags, tendencies, look labels | look .glb, A-pose, faces, moods | home place / region / scene, voiced_by – **plain selects** |
| Toon / season / chapter | toon-editor 202, season-editor 325, meta-panel 161, credits ×6 (~1000), cover-inspector 234 | descriptions | title, authors, release, status, map centre, tile colour | avatar, tile, trailer, cover art, keyvisual | trailer chapter, related items, supporters |
| Item (+ catalog kinds) | cat/item-form 232 (+ an Advanced JSON field), 10 other forms; item-inspector 206 (placement) | one description | ~25 flags and fields | picture, variants, model, score, vinyl | toon, edition, product, album, container, sound |
| Slice / text box | slice-inspector 231 (5 tabs), dialog-inspector 293, slice-media 415, slice-audio 363, audio-panel 158, group-panel 110 | text on the canvas | font, size, tail, volume, trigger … | image / video variants, samples | – |
| VN events | vn-events 488 + 11 forms | message, choice, ask texts | type, trigger, fx, wait, condition | – | address typeahead (the good one) |
| Place / region / route | place-props 230 (**no prose – Kanon is a link away**), place-image 230, region 119, route 138, layer 54 | region: description, greeting, claim | name, position, cells, colours | painted / plaster images, sign | chapter, representative |
| Script / Kanon | script-editor 555 + script-meta 81 (one editor, four routes) | the text | title page | – | – |
| Storyboard | shot-inspector 114, figures 70, script-panel 58 (**read-only**), generate-form 208 | action, note, prompt | camera, lens, light … | image, still | cast, mood, pose, place |
| Collage / sound | piece-inspector 238, place-fields 103; sound workspace tabs | code | rect, mood, parameters | picture | look, assignment |

## 2. Panels needed in parallel

| While you work in … | you need beside it | today |
|---|---|---|
| a chapter's script, VN events, text boxes | the cast's sheets and Kanon, the place's Kanon | other routes |
| the map, placing a place | its Kanon, its image, the events that unlock it, its chapter | Kanon leaves the map; unlocking events invisible |
| a slice, placing an item | the item's sheet and where else it is placed | a picker only |
| a storyboard shot, a generation | the script scene, the characters' looks and Kanon, the place | script read-only, no Kanon |
| Generations / Wanted | the subject's Kanon, slots, prompts | inspector tabs |
| the cast | the full character sheet | a picker |
| VN events, a deck scene | who can receive the word (address book), the scene's listeners | typeahead only |

**Six panel types cover it** (each a universal tab, any workspace shows it for the selection):
1. **Sheet** – the subject read first: short meta on top, prose below, links as chips, every value edits in place.
2. **Kanon / Text** – the subject's Kanon, the script scene, briefs: one editor (today four routes + a hidden tab).
3. **Slots** – every media slot of the subject, with **Wanted** (what is missing) and "from Library / Generations".
4. **Links** – where used (placements, cast, map, scenes), its address, its references.
5. **Events** – the events that act on the subject and what reacts to them (scene listeners, deck receivers).
6. **Inspector** – only the selected *instance*: a text box, an item in a slice, a shot's camera, a collage piece.

Sheet = what a thing *is*; Inspector = how *this use* of it is placed. Today both are mixed in one form.

## 3. Canonical, not additive

**Things that are one description but are edited as piles of fields:**
- **Character:** ~13 identity fields + persona + look + Kanon across tabs; Kanon is a hidden fifth tab, never beside the
  profile. → one sheet: identity, personas, looks as the head, the Kanon sections as the body, links as chips.
- **Place:** no prose at all; the description is `kanon:place.<id>` behind a link. → sheet: name, position, images, Kanon,
  unlocking events.
- **Region:** prose as fields (description, greeting, claim) but no Kanon role (`KanonRole` = toon · character · persona ·
  look · place, `shared/content/src/kanon.ts:29`). → give regions (and items) a Kanon role, or decide they don't need one.
- **Item:** one description, ~25 flags, an Advanced JSON field. → sheet: what it is, what it does, where it is.
- **Toon / season:** title, description, tile, map centre, cast, credits, cover, Kanon on five pages. → one sheet each.

**Facts stored or edited twice (to merge):**

| Fact | Where |
|---|---|
| a character's description | identity + persona `description` **and** the Kanon's "Who / Backstory" |
| the toon's name | toon-editor `title` and the catalog kind `toons` `name` |
| authors | toon, season, chapter, script title page, credits people – **five places** |
| a chapter's title | chapter meta and the script title page |
| characters | `characters/` and the cast (which can create a character); cast `role` vs persona `label` |
| place vs location | the map says `place`, the address book and the deck say `location:` |
| items | catalog definition, collage stage, slice placement |
| person links | `voiced_by` a plain select, credits the PersonPicker; place / region / scene links plain selects, not the address typeahead |

**The pattern to copy:** the cover's lettering *refers* to the toon / season title instead of copying it. Authors,
titles and names should all be referenced from one place (credits, the toon) the same way.

## 4. The link: Library → item → events → deck

**What exists:**
- **Address book** (`shared/api/src/address-book.ts`, server `AddressBook.php`, `GET /api/addresses/<toon>`, Rust twin
  `deck/protocol/src/address_book.rs`, #477 open): kinds character, persona, look, location, item, chapter, group,
  variable, season, toon, product, region, scene. **Only character, persona, look, item and scene are addresses** (a
  receiver exists, decisions/deck.md [address]); the rest are references.
- **VN events** (18 types, `deck/protocol/src/story_events.rs`): showMessage, showChoices, joinScene, leaveScene, fx,
  unlockLocation, migrate, jumpTo, setVariable, giveItem, takeItem, askText … Triggers on a slice: **intersect** (scrolled
  into view), **hover**, **click** – mono or poly. There is no "chapter read" trigger; *unlock* is an action, not a trigger.
- **Scene documents** (`shared/api/src/scene.ts`): `listener[]` reacts to story events (`on: joinScene`, character →
  join / leave), holders filled by addresses, location, camera / shots, `toons`.
- **The deck:** the reader sends story events to the conductor while driving; far inputs use the address path
  (`/scene:nave/character:catpope/…`); a tab in Play receives them (decisions/deck.md [workspaces]).
- **"Events" means three things in the code:** VN story events, deck event names, and the calendar's events
  (`library.ts fetchEvents`). And `library.ts` is the *reader's* library, not the asset Library.

**What the Links / Events panels and the Flows workspace must show** (round 1 drew this; round 2 uses the real names):
1. the entity, its `kind:id`, whether it is an address, draft / published;
2. where defined and placed (catalog, slices, cast, map, Kanon);
3. the events that name it, by trigger (intersect / click / hover) and chapter › group › slice – e.g. *giveItem
   item:record-player · s1c2 slice 14 · click*;
4. the scenes that react (listeners, holders it can fill);
5. its deck reach: the far-input path and the live receivers;
6. the gaps (Problems): references without an address, unpublished scenes missing from a book, the place / location split.

## 5. Consequences for round 2 and the plan

- Every workspace gets the six panel types; the **Sheet** replaces the stacked forms, the **Inspector** keeps only the
  instance.
- Events in the drafts use the real trigger and action names (round 1 invented "on found / on use / on enter";
  "found / tap" stay for XR targets, which are OSCT's).
- **Decide (owner):** (1) one word for place / location; (2) ✓ Kanon roles for regions and items (owner 2026-11-20); (3) authors and titles
  referenced from one place; (4) "Events" naming – owner 2026-11-20: calendar events are public relations, real-world dates (concerts,
  performances, happenings, exhibitions – an exhibition date can point to its XR Exhibition experience; admin Calendar, reader home); ✓ "Events" in the creator = story events, the PR dates
  stay "Calendar" (owner 2026-11-20); (5) ✓ cast as a season facet, not a second character editor (owner 2026-11-20).
- Data work this implies: sheets need no new storage; the merges (authors, titles, descriptions vs Kanon, place /
  location) are migrations – ticket them per area.
