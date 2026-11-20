Status 2026-11-20: narrowed by the owner's answers (§0): no renaming, the containers stay specific to their medium; only what travels between them is unified (assets, sets, events, addresses). Decided: stable chapter group ids (#706), a story place stays relational (§0.3), the relation cleanup filed (#709, #710, #711), story events and choice questions in the address book (#707), shows across toons filed (#708); dropped: the shared sequence core and a group playing a deck scene without a show. Open, not decided: the XR geo-anchor's engine and precision (§0.2). Nothing else built (#703, epic #626). §1 and §2 stay valid; §3 to §8 and scenes-drafts.html are the first, wider proposal, kept as the record and superseded by §0.

# SCENES / CONCEPT — one sequence model for reader, storyboard, XR and deck

## 0. The owner's answer (2026-11-20) and the narrowed proposal

> "everything renaming is not an option. that is really confusing. and also if the scenes keep their type that is actually good because we want to have some dogma for each process. we can pass around assets, and share sets, and events but the container is media specific. so perhaps i was going too far. but some of this research should allow for some sort of unification of concepts"

**Decided:** no renaming. A chapter group, a show row, a deck scene and an XR scene keep their names, their types and their storage. (Naming options A and B in the drafts are off; nothing in §3's vocabulary becomes a stored or visible word.)

**Thought aloud, recorded as the direction:** each container keeps its own rules ("dogma") per process: the reader flips by scroll, the show by the conductor, XR by a found target, and each player keeps its own loading. What is shared is what **travels between** the containers, not the container.

What that leaves to unify, from the map in §2:

| Shared thing | Today | The unification | Size |
|---|---|---|---|
| **Assets** | the catalog and the Library (#698): one rule in the core, drawn by the creator and the deck editor | nothing new: every container picks from the same library; XR (#482) picks from it too when it is built | — (follow the #698 pattern) |
| **Sets** | a deck scene sits at one spot of one set (SCENES.md §8, CATALOG-LINK §8); a chapter group and an XR scene know no set | a chapter group and an XR scene **may name** a set (+ spot), the same field the deck scene has. Then a linked reader's group, a deck scene and an XR scene can stand in the same place with the same cast, without being the same thing | M |
| **Events** | three vocabularies: the reader's story events (VN `events`, `?live=` #518 / #528), a show row's `fire` / `on_event`, a scene's listeners (#517); XR's rules (found / lost / tap → emit) are only on paper | **one event list per toon**: ids declared once, emitted by any container, heard by any. A reader event fires a row or a scene listener (as today), and XR's `emit` / found target emit the same ids | M |
| **Addresses** | `scene:<id>`, `show:<id>`, rows by id; chapter groups have no id (`label` or the array index) | chapter groups get stable ids (was S1) so every container's unit has an address; an `end` or a jump may name an address of another kind (was S9). Chaining across kinds becomes "jump to that address", each container still runs itself | M + S–M |

What is **dropped**: the shared sequence core (S2, S3, S4, S8 as a core), the general words (S0), the lanes as a model, the loading window as one rule. Each player keeps its own `next` and its own preload.

**Dropped too (owner, second answer 2026-11-20):** S5, a chapter group naming a deck scene that a linked reader's decks play without a show. Its only case would be a private session, and testing a show already gives that; a linked reader's story events already reach a scene's listeners (`joinScene` / `leaveScene` …, #517) and fire rows. The 2026-11-11 thought is outdated.

**Kept:** the strip look (S6). The chapter tree, the storyboard groups, the show band and OSCT's panel share a **drawing** in the editor kits, not a model (owner: "makes sense"). Needs a draft first (G1).

### 0.1 Events and the address book, as they are today (read from the code)

Two grammars exist, on purpose:
- **An address says where:** a place `<kind>:<id>` (`scene:nave`, `character:catpope`, `deck:left`) and the ladder below it (`deck/space/src/rig/address.rs`, SHOW.md §8). The **address book** (#477, `server/schema/api/address-book.schema.json`) lists what exists per toon, with the kinds character, persona, look, location, item, chapter, group, variable, season, toon, product, region, scene. **It has no `event` kind.**
- **An event name says what happened:** `<domain>.<name>` with a dot (`story.fx`, `music.note`, `relay.cue`, `deck.mood`), from the name dictionary (`deck/events/src/names.rs`, EVENTS.md). A receiver subscribes to a name or a whole domain (`route.rs`). Story events carry the creator's stable event id (#539); a scene's listener matches type + id (#517); a show row's `fire` names an event id (`ch3.yellowmouse-enters` in the editor's tests).

**Correction (owner asked, 2026-11-20): the event list exists, beside the address book, not in it.** `GET /api/creator/story-events` (#521, `server/src/Domain/Content/FlowIndex.php` `storyEvents`) lists every toon's chapters' story events: per chapter `{ id, type, characters, excerpt, group, groupIndex }`, drafts included, plus the published questions (#550); readable by the content editors and the role `live` (#555). The deck editor's row picker reads it (`deck/editor/src/show/events.rs`, #539: an event without an id is dropped, no row can name it). On a deck, the story adapter turns a story event into `story.<type>` in the name dictionary (`deck/events/src/adapters/story.rs`). What it lacks: events a show or an XR scene declares itself (not from a chapter), and one typeahead with the address book.

**Global events (`all:enter_mainstage`):** there is no `all:` place, and none is needed. Reach comes from where an event is sent, not from its name:
- Pushed down the conductor tree, an event reaches **every** receiver below (SHOW.md §8 "who picks up goes down"); a wildcard (`location:*`) is "all of a kind".
- A place in front **narrows** it: `/deck:foyer/location:* > close_doors`.
- So `enter_mainstage` is an ordinary event in a domain (e.g. `stage.enter_mainstage` or the toon's own story id), sent at the top of the session's tree. "All" is the default reach, not a prefix.

Proposed two tiers for the event list:
| Tier | Who defines it | Where it lives | Examples |
|---|---|---|---|
| **Platform domains** | code | the name dictionary (`names.rs`) | `deck.*`, `relay.*`, `music.*`, `clock.*` |
| **A toon's events** | the creator (chapters today; shows and XR later) | the story-events index (#521), per toon | `ch3.yellowmouse-enters`, `enter_mainstage` |

A toon's event is never global across toons; it is global within the session that plays it.

**Three questions for the owner** (replacing §8):

1. ~~Events~~ **Decided: the story events and the choice questions join the address book as a kind `event`** (owner 2026-11-20: "choice questions seem to be the same concept. Lets do that.") → #707. No `all:` prefix, reach by where it is sent (§0.1). Shows mixing toons (Catpope + Yellow Mouse, soon) need several books at once → #708.
2. ~~Spots on chapter groups~~ **Decided: a story place stays relational, no place field on chapter groups or slices** (owner 2026-11-20, §0.3). An XR scene's geo-anchor stays its own topic (§0.2).

### 0.2 A spot inside an XR scene is a physical place (owner 2026-11-20)

> "important addition. a inside a xr scene a spot is linked to the physical position of the device IRL"

The same word, three resolvers, which is the "container is media specific" rule again:

| Medium | What a spot resolves to | Who resolves it |
|---|---|---|
| **Deck** | a named place in the 3D set (`set` + `spot` on the space node, SCENES.md §8) | the scene document, at load |
| **XR** | **a place in the real world where the device is**: the content stands where the phone is, IRL | the XR viewer's tracking, at run time |
| **Reader** | no position: at most the panel's framing (a spot names *which* place, not where) | — |

What today's XR plan can and cannot do (XR.md, the table "Inside a book" / "Not there"):
- Content is anchored on an **image target** (1 unit = the target's width): a printed page or a poster. A target's physical place (the poster on the wall, the book on the table) is the only real-world position the viewer knows.
- **World tracking, rooms and anchors are not there:** the SLAM binary's licence rules them out; 6DoF needs WebXR `immersive-ar` (Android Chrome only, and it stops image tracking). The handheld camera (#469) already streams a phone's pose that way, but into a deck, not in the XR reader.

So the owner's addition has two readings, and they need different things:
- **(a) A spot is a target's physical place:** "this spot = where poster 3 hangs". Works with the image tracking planned for #482; a spot names a target (or a group of targets in one room).
- **(b) A spot is where the device itself stands:** the phone's own position (room / GPS / SLAM) picks or places the spot. Needs world tracking or geolocation, which the current XR plan rules out (licence, WebXR limits). That makes it a new capability, not a field.

**The owner's answer (2026-11-20, thought aloud):** *"Xr spot is a geolocation on the floor level. So basically a anker in IRL this way we can anker XR scene to the real world."* So reading **(b)**, made concrete: an XR spot is a **geo-anchor**: a geolocation, placed on the floor (the ground plane), that ties an XR scene to a place in the real world.

What it needs, which today's XR plan does not have (XR.md "Not there"):
- **A position:** the device's geolocation. Browser GPS is metres off, too coarse to stand content at an altar by itself; a fine fix needs a visual localisation (VPS) or a known marker near the spot to correct it.
- **The floor:** plane detection (where the ground is), which is world tracking: ruled out for the planned 8th Wall image engine by the SLAM licence; WebXR `immersive-ar` has it but only on Android Chrome, and it stops image tracking.
- So the geo-anchor is a **new XR capability** (research, then a decision on the engine / platform; maybe the native wrapper the owner keeps for "when hardware needs it"), not a field on #482's first build.

**The owner adds (2026-11-20, thought aloud):** *"The world map that is correct is a fictional distorted abstraction of the world so it hold not factual geoolocations. We have a mapping layer between the game map and the REAL WORLD which at this moment only xr scenes will have."*
- The world map holds **no real geolocations**: its `x` / `y` are world units of a fictional, distorted map (MAP.md), never coordinates.
- Between the game map and the real world sits a **mapping layer** (a concept, nothing built): it ties a game place to real anchors. **Only XR scenes use it for now.** An XR scene's geo-anchor is a real point; the mapping layer says which game place it stands for.
- So three things stay apart: a story place (the webtoon, §0.4), a map place (the fictional map), a real anchor (XR, through the mapping layer).

### 0.3 A spot on a chapter group – **decided: relational, no field** (owner 2026-11-20)

> "Does a story place need a field at all, or stay relational: relational. since story board names locations. and webtoon driver events call scenes, and scenes specify locations. so purely relational is totally fine. and we should remove any other unused or previously conceptionalised placeholders. our initial creator has many places of relations that were intended but turned out useless to the overall logic"

**Decided:** a story place is **relational**: the storyboard names the locations (per shot), the webtoon's driver events call deck scenes, and a deck scene names its location (set + spot). Chapter groups and slices get **no** place or spot field. §0.3 and §0.4 below are closed on that and kept as the record of how it was reached. The cleanup the owner asked for in the same answer: §0.6.


> "Set on groups in dont know since a group could play out in a specific spot of a location ... so for example cathedral altar or cathedral confession both"

The owner's lean: a group names a **spot within a location** (`location:cathedral` + `altar` / `confession`), not a whole set. That matches where SCENES.md §8 already points deck scenes ("v1: `set` + `spot`; later `location` + `spot`, the set following from the location"). Two groups of one chapter could then play at two spots of the same cathedral.

**Still open (no ticket until the owner decides):**
- Does a chapter group carry a spot at all, and what does it do there (the reader shows no position: is it only for a linked deck, for XR, for the map)?
- One spot per group, or several ("both")?
- The reference form: `location:cathedral.altar`, or a spot id of its own?
- XR: the geo-anchor's engine and precision (§0.2).

### 0.4 A place on a single slice, and where a place shows today – **closed: relational** (§0.3)

> "But also a single slice can have that same location thingy. At the moment this is relational since only way to tell which group or slice is where is by goto links. And labels. Please investigate"

**Read from the code (main @1d064d30):** neither a chapter group nor a slice has a place field (`shared/content/src/content.ts` `Group`, `Slice`). Where a group or a slice "is" can only be inferred today, from six sources, and only the first one is a real "this happens here":

| Source | Where | What it says | Granularity | Exact or inferred |
|---|---|---|---|---|
| **Storyboard locations** | `storyboard/types.ts`: `StoryboardDoc.locations[] = { ref, place (MAP.md place id) or new, time }`, each `Shot.location` = a `ref` | **this shot happens at this map place** (+ time / weather); prompts, the schematic and Luma use it | per **shot** | exact, but only in the storyboard |
| Storyboard → chapter | `scripts/storyboard/compile.mjs`: one shot → one slice, `slice.shot = shot.id` | the slice keeps its shot's id, so slice → shot → location → place can be followed | per **slice** | exact for compiled slices; lost for hand-made slices and hand edits; the place itself is **not copied** into the chapter |
| **Map events** | MAP.md §3: `unlockLocation`, `setPosition`, `travel`, `setAvatarLocation`, `focusMap`, in a group's VN script or on a slice (`trigger: intersect`) | the reader's map state changes here ("you are here" moves) | group or slice | inferred: unlocking a place is not standing in it |
| Map place → chapter | the world map's place: `chapter?: { toon, number }`, `toons[]` | the place's card links to a chapter ("read chapter 1") | **chapter** | inferred, one chapter per place, no group or slice | **Owner: not needed, against the dogma of ids across the universe (§0.5).**
| `placeUnlocks` | `FlowIndex::placeUnlocks`: place id → the documents whose events unlock it | which chapter unlocks a place (the map builder warns about a hidden place nothing unlocks) | **document** (chapter), not group or slice | inferred |
| Goto links and labels | `FlowIndex` edges `{ source: { toon, chapter, group: label }, target, kind: jump }`; the group's `label` | how groups connect; a label may *name* a place ("cathedral-altar") by convention | group | a naming habit, not data |

The address book lists **locations** (`location:<place>`, places whose `toons` hold the toon) and **groups** (`chapter:<n>.<label>`), but no link between them; it has no spots (sets are "not in it yet").

**What can be read today, without a change:** for compiled chapters, a slice's place by `slice.shot` → the storyboard's shot → `location` → map place. Nothing for hand-made groups or slices; and map events say what the reader learns or where the avatar is put, not where the scene plays.

**Where a slice-level place would fit, next to the others (a sketch for the owner, not a proposal to build):**

| Level | What it would name | Inherits | Who uses it |
|---|---|---|---|
| **Chapter group** | a location, optionally a spot of it (`cathedral`, `cathedral` + `altar`) (§0.3) | — | a linked deck (what to stage), the map ("where this part plays"), the address book (groups by place) |
| **Slice** | its own location / spot when it differs from the group's (a cut to the confessional inside an altar group) | the group's place when it has none | the same, finer; the storyboard's shot already has it |
| **XR scene** | a geo-anchor on the floor (§0.2) | — | the XR viewer only |
| **Deck scene** | a set + spot today, location + spot later (SCENES.md §8) | — | the deck |

The natural source would be the storyboard: its shot already carries the place, and compile could copy it into the slice (and a group's place could follow when all its slices share one), so the place is authored once.

**Open (no ticket until the owner decides):**
- Group, slice, or both (slice overriding the group)?
- Copy the place at compile, or keep following `slice.shot` to the storyboard (one truth, but hand-made slices get none)?
- A place is a map place (MAP.md `location`) and the spot a named point inside it: the same vocabulary as the deck scene's spot (`set` + `spot` → `location` + `spot`)?
- Is a map event (`setPosition`) ever the same thing as "this plays here", or always separate (the owner: today it is "relational")?

**The owner adds (2026-11-20, thought aloud, not decided):** *"Since the webtoon is more of a story rather than a map or actual place. I assume"*

So "place" has two readings, kept apart here:

| Reading | What a place is | Where it lives today | Belongs to |
|---|---|---|---|
| **Story place** | where something happens **in the story**: a part of the tale, told by labels and goto links (and the storyboard's scene heading) | relational: group labels, jump edges, the storyboard's `locations` per shot | the webtoon (chapter groups, slices) |
| **Real place** | a point in the real world, a geo-anchor on the floor (§0.2) | nowhere yet | XR only |

The world map sits between them: its places are story places drawn on a **fictional, distorted** map (owner: no factual geolocations), unlocked by story events. A real place reaches a map place only through the mapping layer, and only XR uses it (§0.2). Read this way, a group's or slice's place (§0.3, §0.4) would be a **story place**, never a real one, and the sketch's "Level" table should not suggest that a webtoon group and an XR scene name the same kind of place: they share at most the story place's id (an XR scene in the cathedral story place, anchored at a real spot), not its meaning. Open: whether a story place needs a field at all, or stays relational (labels, links) as the owner's "I assume" suggests.

3. ~~Stable ids for chapter groups~~ **Decided yes** (owner 2026-11-20: "the more we get to learn and unify the addressing the better") → #706.

Draft tickets left (unfiled): research "XR scenes: a geo-anchor through the game-map ↔ real-world mapping layer" (area:xr, M; the chapter-group half dropped, §0.3) · feature "`end` and jumps take another kind's address" (area:content, S–M, after #706) · feature "Draft: one strip drawing for chapter tree, storyboard groups, show band" (area:creator, S).

### 0.5 The map place → chapter link (owner 2026-11-20)

> "A map place links to one whole chapter, never to a group or a slice. This is something we do not really need since it is kind of against our dogma where we go to ids across the universe."

A world-map place carries `chapter?: { toon, number }`: a toon and a number pair, not an address. What uses it (main @1d064d30):
- **Schema:** `shared/content/schema/map-place.schema.json` (`chapter`, "the chapter the place's card links to").
- **Creator:** `creator/src/lib/components/map/place-props.svelte`: a toon select and a chapter number on the place.
- **Validation:** `shared/content/src/map.ts` (~1635): shape check, a warning when the toon is not in the place's `toons`.
- **Reader:** `viewer/src/lib/components/toon/map/map-overlay.svelte` (~319, ~561): the place card's "Read chapter N" / "Read the prologue" link (this toon's, readable chapters only). The only thing a reader sees.
- **Old viewer builds:** `legacyToonMap` (`map.ts` ~1328) and `Library::legacyMapLists` (`Library.php` ~634) write it back as `chapterNumber` in the old `ToonBundle.locations` shape; `map.ts` ~1394 migrated the old shape in.

**Draft ticket (not filed, waiting for the owner's OK):** chore · area:map · complexity:S · "World map: the place card's chapter link goes, or becomes an address". Owner quote above. What: drop `chapter: { toon, number }` from the map place (schema, creator field, validation, the card's link), or replace it with an address from the address book (`chapter:<n>`, later `chapter:<n>.<group>` with #706, any toon's), so the map speaks ids like everything else. The legacy `chapterNumber` output for old viewer builds goes with it. **Open:** remove the card's "Read chapter" link entirely, or keep it through an address? Do saved places need a migration (drop the field, or turn it into an address)? Docs: MAP.md, this §0.5. Refs #703.

### 0.6 Relation placeholders nothing needs – the cleanup list (for the owner's OK, nothing removed)

The owner (2026-11-20, §0.3): *"we should remove any other unused or previously conceptionalised placeholders. our initial creator has many places of relations that were intended but turned out useless to the overall logic"*. Read from the code (main @1d064d30; a code sweep, spot-checked). Saved-data counts are from the repo's fixtures and the last seed (`git show 04e0f968e^:shared/content/seed/…`), **not production** (no DB read). The deck reads none of these fields.

| # | Field (where defined) | Written by | Still read by | Logic depends on it? | Saved data → migration |
|---|---|---|---|---|---|
| 1 | Slice `_type` / `_label` (content.ts:461–462; chapter / chapter-doc schemas) | nothing | nothing | no | 0 in fixtures and seed → none |
| 2 | Platform event `toon?: string[]` "not shown yet" (content.ts:669; events schema) | no UI (the admin calendar keeps it as an unknown field) | nothing | no | the old seed had it on all 13 events → optional strip |
| 3 | Persona `introduced: { toon, season, chapter }` (characters.ts:208–212) | creator character-persona.svelte | validation only | no | 1 in fixtures → optional strip |
| 4 | Character `voiced_by` → person (characters.ts:347) | creator character-identity.svelte | validation only; credits do not use it | no | 0 → none |
| 5 | Character `origin` / `residence` → map region (characters.ts:351–353) | creator character-identity.svelte | validation; `residenceOf`'s registry fallback (map.ts:1265) is passed only by a test; the reader uses the save game's residences | no | 0 → none |
| 6 | Chapter `featuring` → characters (content.ts:551) | creator featuring field, meta panel, suggestions | **reader: none** (removed from the reader, "data kept", b4f16920b); server adds the ids to the `cast?chapter=` answer (Library.php:395–398), payload only | no | 1 + 4 in fixtures → optional strip; server query + tests change |
| 7 | Chapter `related_items` → items, "the reader does not show it yet" (content.ts:556) | creator related-items field | server adds the items' icons to the chapter's offline files (ChapterFiles.php:191): extra downloads only | no | 0 → none. **Keep** Season `related_items` (the toon page reads it) |
| 8 | Route `toons` (map.ts:247) | creator route-props.svelte | creator only: dims routes under the toon filter; legacy output | display only | 1 → optional strip |
| 9 | Place `chapter: { toon, number }` (§0.5; map.ts:125) | creator place-props.svelte | **reader:** the place card's "Read chapter N" link (map-overlay.svelte:320); **creator:** the Flow's place boxes and its entry rule (a visible place makes its chapter an entry, flow.ts:923; published chapters are entries anyway); legacy output | the card link and the Flow's place boxes | 19 of 20 places in fixtures → **migration** (drop the field) |
| 10 | The legacy map shape for old viewer builds: `MapLocation` / `MapRoute`, `ToonBundle.locations` / `.routes`, `Toon.map` (content.ts:196–212, 739, 862); server `legacyMapLists` / `withLegacyMap` (Library.php:309–321, 634–669+); TS `legacyToonMap(Settings)` (map.ts:1328–1356); a fallback reading the purged `locations:` / `routes:` docs (Library.php:313, dead since migration 2026-10-12) and their EntityKey kinds | the server, computed | **the current reader: nothing**; tests | no (for today's reader) | none stored → none. Open: is the "one release" window for old builds over? |

**Checked and kept** (logic reads them): character `home` / `home_scene` (homes), `own_toon` (library, address book, deck editor), region `representative` (reader), season `trailer.chapter` (reader), jump target `toon`, item `toon` (permissions), item `product`, item `edition_id`, place `toons` (reader, address book), place `set` (deck scenes), season `related_items`.

**Draft tickets (not filed; the owner approves the list first, a removal is not undone lightly):**
- **A** · chore · area:content · S – "Remove relation fields nothing reads": rows 1–7 (schema, types, creator fields, validation; the server's `featuring` in the cast answer and `related_items` in the offline files with their tests). Saved data stays harmless; an optional strip migration for rows 2, 3, 6. Refs #703.
- **B** · chore · area:map · S – "World map: the place → chapter link goes" (row 9, owner: "against our dogma"; and row 8, route `toons`, if the owner agrees it is display only). Removes the reader card's "Read chapter" link and the creator Flow's place boxes; a migration drops the field from saved places. Refs #703.
- **C** · chore · area:platform · S – "Drop the legacy map output for old viewer builds" (row 10), once the owner confirms no old build needs it. Refs #703.

**The owner's answers (2026-11-20), filed:** rows 1–7 → **#709** (voiced_by goes, a voice credit belongs in the credits if needed; chapter `related_items` "really pointless"; rows 2, 3, 6 strip saved data). Row 9 "fine": the place → chapter link goes with the card's "Read chapter" link ("we got another type of progress tracking and visited") and the Flow's place boxes ("we are currently expanding the flow or logical node interface in both osct and in deck editor"); row 8 route `toons` goes too (the new Library scopes by file paths and item addresses, not route toons) → **#710**. Row 10: "older readers are all encouraged to update", legacy compatibility drops → **#711**. Still open: Season `related_items` (kept unless the owner says otherwise; the toon page reads it).

## 1. The idea (owner 2026-11-20, thought aloud, not a decision)

> "XrScenes and Groups/Scenes in the webtoon use a very similar concept, and basically eventually could also all be chained. So the type of scene seems like something we can generalize. So do deck scenes. It really just depends on the deck or reader or XR viewer to interpret them and know when to flip to the next, and how to preserve resources. And I suppose if we think hard enough, between live and reader there isn't much of a conceptual difference at some point. … I noticed groups in the chapter tree look almost like the scenes filmstrip in the storyboard, or the OSCT filmstrip for the scenes."

The owner has said related things before:
- 2026-11-11 `[scenes]`: specialised scenes attached to groups in the reader, or to cells in show rows.
- 2026-11-13 `[scenes]`: the reader acts like a driver. Its events, from its continuous scroll or anything it triggers, drive a deck's scene. The reader itself stays as it is.
- 2026-11-20 `[deck-os]`: being a conductor may in the end be only a flag on a deck.
- 2026-11-21 `[editor-style]`: Edit / Play applies only in the chapter, with the reader as the driver, and in the storyboard.

## 2. What exists today

| Piece | Where | The ordered unit | What a unit holds | Who flips to the next | Preload / free |
|---|---|---|---|---|---|
| **Chapter** (reader) | `shared/content/src/content.ts:500` `Group`, `:528` `Chapter.groups[]`; creator `components/chapter/chapter-tree.svelte`; viewer `routes/(toons)/[toonId]/chapter/[chapterNumber]/+page.svelte` | **group**: no id; the array index or an optional `label` addresses it | `slices[]` (image, video, `audio` per slice, dialog, items, VN `events`), `renderer` (vertical / strip / endless), `continues` (flows into the next on the same page, a **chain**, chains.ts:47), `end` (a jump, optionally `auto`) | **The reader's scroll** inside a chain. Between chains and chapters, `nextStep()` (shared/content/src/progress.ts:185) picks the authored `end`, else the next labelled group, else the next chapter in the season. next-bar.svelte shows it, with a 10 s countdown to the next chapter | `viewer/src/lib/lookahead.ts`: N screens ahead (data-budget tiers 0 / 1.5 / 4 / 10), within the current chain only; nothing prefetches the next chain or chapter. Freeing happens only per chapter (Howler unload) |
| **Storyboard** (creator) | `shared/content/src/storyboard/types.ts:219` `ChapterStoryboard`, entity `storyboard:<toon>:<chapter>`; creator `components/storyboard/*` | **script scene** (`StoryboardDoc`) › **storyboard group** (mirrors `Group`) › **shot** (`sb-<scene>-<n>`, stable id) | shots: camera, figures, takes, dialog | nobody: it is authoring. The timeline (`storyboard-timeline.svelte`, `filmstrip()` in `lib/editor/storyboard-timeline.ts:235`) plays picked shots back to back | — |
| Storyboard → chapter | `scripts/storyboard/compile.mjs` (offline, saved through MCP `save_draft`) | one storyboard group becomes one chapter group, one shot becomes one slice (`slice.shot`) | | | |
| **XR** (#482, docs only) | research/XR.md §5, §8; worktree `wip/482-xr-experiences-7f10f7` has no code | **Experience** (Book · Concert reading · Exhibition · Poster) › surface › entry › target › **scene** | an entity tree anchored on an image target, behaviour rules (found / lost / tap → play / show / emit …) | **A found image target** (random access). Story jumps to and from targets are proposed (§8.2) | §8.3, proposed: the experience document first, the engine on "Start the camera", a scene's files when its target is found |
| **OSCT** (comparison) | `osct/editor/src/lib/ScenesPanel.svelte`, `SceneCards.svelte`, `scene-strip.ts`; `engine/src/utils/prepared-scenes.ts`; `client/src/services/PreloaderService.ts` | Experience › `scenes[]` (order = position) + `groups[]` (a scene names its group; **a group = one stop of the reader's switcher and meant as one loading window**) | surfaces, targets, entity tree, passes | a tracked target (`onSceneSeen` → `switchScene`), the bottom switcher, links `/scene/<id>`, history; scene-to-scene flows planned, not built | **The prepared window**: by default the 2 scenes with targets on each side, nearest first, cancellable. `AssetCache.release(keep)` frees what the next scene does not share |
| **Deck scene** | `deck/space/src/scene_document.rs:48` `SceneDocument` (format 2), `scene_resolve.rs:154` | a scene is **content, not a step**: one AV graph played as a whole, with no "next" | location, holders (cast, spawn zones), listeners (#517), stage, nodes, links, camera | a show's cell (`piece.rs`: the same piece glides, another piece loads and crossfades, `transition_secs`); a listener changes the cast (`join` / `leave`, #516 / #517) | `push.rs` `preload` (the conductor's `scene.sync` fills the cache, "nothing plays"); `published.rs` caches per version. No explicit unload was found: the old world is dropped when the switch rebuilds |
| **Show** | `deck/show/src/document.rs` `Show` › `rows[]` › `cells{outlet → Cell}`; `conductor.rs` | **row** (id) | one cell per outlet: Play (a piece `scene:<id>` + values) · Values · Keep · Mirror · Black · Look · Ask | `Row.fire`: a story event id (Auto after `hold_ms`, or Suggest), at a moment (Now / Bar / Phrase); **GO** = the held suggestion, else the next row; a hand fire is always allowed; a vote's result becomes an event | the flash checks every piece (`ready.rs`). #686 `alone.rs`: `needs()` lists every piece of the outlet in row order and fetches **all** of them before the first row |
| Deck Play (#642 / #686) | develop `deck/ui/src/start.rs` | scenes and shows of the sandbox, then the others | | the deck as its own conductor, GO → next row, THE END | as above |

What is already shared, and where it lives: the reader drives a deck today. A linked reader (`?live=`, #518 / #528) sends its story events, which fire rows (`on_event`) and make a scene's listeners act (#517). The Library (#698) shows the pattern this concept would follow: the rule lives in the core (`deck/catalog/src/library`), the creator calls it through wasm, the deck editor draws it with deck-kit, and the TS twin is deleted.

## 3. The common core

Under the four names (group, storyboard group, XR / OSCT scene, show row) sits one structure. A deck scene is not one of them: it is what a step plays on a stage.

- **Sequence**: an ordered list of **steps**, addressable, owned by one document. A chapter, a show, an experience and a storyboard are sequences.
- **Step**: an id (stable, never reused) plus an optional label, and one **lane** per output that says what plays there:
  - `page`: slices, as the reader draws them;
  - `stage:<outlet>`: a piece, i.e. a deck scene, rig or module with values (today's cell);
  - `target:<id>`: an XR scene;
  - `device` / `look`: light.

  A step also says how it is **entered** (`fire`: an event, a found target, reaching it by scroll, GO, a moment) and how it **ends** (`end`: next, a jump, a hold, auto).
- **Window**: contiguous steps that share one page and one loading unit. The chapter's chain (`continues`) and OSCT's group are both windows. A show's row is a window of one.
- **Cursor**: where a driver stands, i.e. the current step (or window) plus a position inside it.
- **Driver**: anything that moves the cursor. The **player** draws only its own lane.

  | Driver | Moves the cursor by | Kind |
  |---|---|---|
  | reader scroll | position, continuous; reaching a step | linear |
  | story event (reader, VN script, vote) | an event id → the step whose `fire` matches | jump |
  | conductor | GO (next / suggestion), hand fire, auto after hold, moment | linear + jump |
  | XR tracker | found target → its step | random access |
  | timer / `end.auto` | the end of a step | linear |

- **Next**: one rule, `next(sequence, cursor) → candidates`. It returns the authored `end`, else the next step in order, else the sequence's own `end`, which may be another sequence's step. This is `nextStep()`, GO and OSCT's planned flows in one place.
- **Prepare and release**: `window(sequence, cursor, budget) → {prepare, keep, release}`, nearest first and cancellable. This one rule covers the reader's lookahead (screens), OSCT's prepared window (±N steps), `alone.rs` (today: all of them) and `scene.sync` (the conductor's push). The budget is the player's: data tier, GPU memory, the deck's quality governor (A10).
- **Live and reading are two drivers on the same sequence.** A chapter read alone is a sequence with the reader as its only driver and the page as its only lane. Add a `stage` lane to a group and connect a deck, and the same reading drives the deck: that is today's linked reader, without a hand-built show. A show is a sequence whose driver is a conductor and whose lanes are stage outlets. Let a reader drive it, and it is the first use case of SHOW.md §2.
- **Chaining across kinds**: every `end` and jump names a step address: `chapter:<n>.<step>`, `show:<id>.<row>`, `book:<id>.<target>` (XR.md §5.5). `JumpToTarget` already has chapter, season, toon, product and xr. The general form is "a step of any sequence".

## 4. Where it breaks (the real differences)

1. **Linear, continuous vs discrete vs random access.** The reader is a continuous position over slices, with several groups visible in one chain. A show fires discrete rows on a clock (bar, phrase). XR has no order at all: whichever target the camera sees wins, and order only hints at what to preload. So "next" cannot be the model's centre. The cursor plus jumps is the centre, and order is the default and the preload hint.
2. **The lanes share no renderer.** Slices in the DOM, an AV graph in deck-engine and a three.js entity tree have nothing to draw in common. Only the sequence, the cursor, next and the window are shared. This is the A2 split: the players are UI or adapters, the sequence rules are the core's.
3. **Time.** A show has a clock and moments, while the reader and XR have none. A step's `fire.moment` exists only where a clock drives. Under the reader driver, Bar and Phrase mean "now".
4. **Authority and many decks.** Live has a conductor tree, seats, suggestions vs autopilot and "GO is always allowed". Reading has one person. Suggest and hold are needed because a reader can scroll by accident (SHOW §7). That stays a driver's policy, not the sequence's.
5. **The reader runs no Rust today.** `nextStep()` and the lookahead are TypeScript in `shared/content` / `viewer`. Moving next and window into the core means either wasm in the reader (bundle and first paint: the reader's speed is a product rule, LOADING.md) or A6's contract by **shared test vectors** (#474): the core is the one definition, and the reader's TS is checked against its vectors. That is a twin by another name, and A6 allows it only as one of the four mechanisms. **The owner decides (§8 Q3).**
6. **Storage differs, and groups have no ids.** A chapter is one document with id-less groups. Shows are their own documents with row ids. An XR scene sits inside its experience document. Storyboard groups are compiled offline into chapter groups. A step needs a stable id to be addressed or fired, so chapter groups need ids: an SQL migration (E2) that keeps `label` as the jump name.
7. **Words.** `scene:` is the deck scene only (SCENES.md §8.5, proposed; XR.md and `address-book.ts:3` follow it). The owner's word for the general thing is "scene". If the generalised unit were called scene, `scene:` would mean two kinds (A14). This doc therefore says **sequence / step / lane / window**: a group, a row and an XR scene are steps, and a deck scene is what a stage lane plays. **The owner picks the words (§8 Q1).**
8. **XR is a second engine by decision** (2026-11-10 [xr]: three.js accepted, browser only, not on decks). Its player stays TS, but OSCT's `prepared-scenes.ts` is a sequence rule. Copying it as it is would be a TS twin of the window rule. It should come from the core when XR is built (§8 Q5).

## 5. What it would look like in the core (proposal)

- A small sans-IO crate `deck/sequence` (or a module in deck-core). It holds `Sequence`, `Step`, `Lane`, `Window`, `Cursor`, `Driver` input, `next()` and `window()`, takes no documents directly, and is shaped by adapters per kind: chapter → sequence, show → sequence, experience → sequence. It is the same shape as `scene_resolve` (pure, the caller hands in what it needs).
- The show conductor keeps all its features: rows, cells, fire, suggestions, votes, seats, subtrees. Only its ordering, GO-next and preload set come from the crate. Things only grow.
- The deck editor's Show band, the creator's chapter tree, the storyboard's group strip and (later) the XR scene strip draw one **strip**: steps as cards, windows as bands over them (OSCT's look), and the cursor. Its rules, i.e. what a card shows, what a band is, reorder and group / ungroup, come from the core, as the Library does (#698). This is the owner's observation about the filmstrips. Each app draws it its own way: Svelte in the creator, deck-kit in the editor.

## 6. Plan in steps

Each step is small, keeps every existing feature, and waits for the step before. UI steps need a draft first (G1).

| # | Step | Size | Done when |
|---|---|---|---|
| S0 | **Words and the answers** (§8): record them in decisions, the RULES A14 word line, SCENES.md §8.5 | S | the owner answered Q1–Q6 |
| S1 | **Chapter groups get stable ids** (migration, `label` stays the jump name); storyboard groups carry theirs through `compile` | M | every group in the DB has an id; `chapter:<n>.<id>` resolves; the reader's `?label=` unchanged |
| S2 | **The sequence core**: `deck/sequence` with Sequence / Step / Lane / Window / Cursor, `next()`, `window()`; test vectors as JSON; adapters chapter → sequence and show → sequence | M | core tests; vectors cover chains, `end`, `auto`, the season's next chapter, GO, a fired row, a found target |
| S3 | **The show runs on it**: conductor GO-next and `alone.rs`'s fetch use `next()` / `window()`; `alone` prefetches ±N rows and releases the rest instead of all | M | headless scenario: a 12-row show alone keeps ≤ N+1 rows' pieces cached; every show test is still green |
| S4 | **The reader's next and lookahead from the core** (by Q3: wasm or test vectors); the lookahead crosses into the next chain and the next chapter's first screen | M | `nextStep()` has one definition; the reader prefetches the next chapter's first slices at idle on generous / normal |
| S5 | **A stage lane on a chapter group** (the owner's 2026-11-11 "scenes attached to groups in the reader"): a group may name a `scene:<id>` (+ fillers, `Use`) for a stage; a linked reader's cursor plays it on the session's decks with no hand-built show; the reader is unchanged | M | a phone reader on `?live=` scrolls into group 3 → the deck crossfades to its scene; a show row may still override it |
| S6 | **The strip, drafted** (G1): one strip for the chapter tree, storyboard groups, the Show band, the OSCT-like XR strip; plain-JS draft in `agents/research/scenes/` | S | the owner OKs the draft |
| S7 | **The strip's rules in the core, drawn twice** (creator Svelte + deck-kit), as #698 did for the Library | M | the creator's chapter tree and the deck editor's rows band use the core's strip rules; no TS twin |
| S8 | **XR on the sequence** (with #482's build): an experience adapts to a sequence, a found target is a jump, OSCT's prepared window becomes `window()` | M | XR's preload comes from the core; no copy of `prepared-scenes.ts` |
| S9 | **Cross-kind chaining**: `end` / jumps take any step address (`show:`, `book:` …); a chapter's end may open an experience's target or a show row's event | S–M | the reader follows `end → book:<id>.<target>`; an XR scene's "back to the chapter" works |

## 7. Draft follow-up tickets (unfiled, for the owner to approve)

- **S0** · `decide` · "Words for the general sequence: step / lane / window, or scene?" (area:deck, S)
- **S1** · feature · "Chapter groups get stable ids (migration), `chapter:<n>.<id>` in the address book" (area:content, M) · chore · "compile.mjs carries storyboard group ids into chapter groups" (area:creator, S)
- **S2** · feature · "deck/sequence: the sequence core with next() and window(), test vectors" (area:deck, M) · chore · "adapters chapter → sequence and show → sequence" (area:deck, S)
- **S3** · feature · "Show alone prefetches a window of rows, not all" (area:deck, M) · chore · "Conductor GO-next through the sequence core" (area:deck, S)
- **S4** · `decide` · "The reader and the core: wasm vs test vectors for next / lookahead" (area:viewer, S) · feature · "Lookahead across chains and into the next chapter" (area:viewer, M)
- **S5** · feature · "A stage lane on a chapter group: a linked reader plays the group's scene on the session's decks" (area:deck, M) · feature · "Chapter editor: a Scene field on a group" (area:creator, S, after the S6 draft)
- **S6** · feature · "Draft: one strip for chapter tree, storyboard, Show band and XR" (area:creator, S)
- **S7** · feature · "Strip rules in the core, drawn by creator and deck-kit" (area:deck, M)
- **S8** · feature · "XR experience as a sequence; preload window from the core" (area:xr, M; with #482)
- **S9** · feature · "End and jumps take any step address" (area:content, S–M)

## 8. Questions for the owner

1. **Words.** Should the general unit be called **step** (in a **sequence**, grouped into **windows**, with one **lane** per output), keeping `scene:` for the deck scene only? Or should "scene" become the general word and the deck scene get a new one? *Recommended:* step / sequence. The address grammar stays one meaning per word (A14), and nothing stored is renamed.
2. **A chapter group is a step with lanes, and so is a show row.** Is that the picture? (A chapter read alone = a sequence driven by the reader. A show = the same driven by a conductor.) *Recommended:* yes. The show keeps everything it has.
3. **The reader and the core.** Should the reader load the sequence core as wasm, or keep its TS `nextStep()` / lookahead checked against the core's shared test vectors (A6 contract)? *Recommended:* test vectors first (the reader's first paint stays light), and wasm once the reader takes the core for something heavier.
4. **A stage lane on a chapter group (S5).** Should a group be able to name a deck scene that a linked reader's decks play, without a show? This is the 2026-11-11 idea, now with the reader unchanged (2026-11-13). *Recommended:* yes, with a show row still able to override it.
5. **XR's preload from the core.** When XR is built, should OSCT's prepared window be taken from the core rather than copied? *Recommended:* yes. A copy of `prepared-scenes.ts` would be a TS twin.
6. **Windows as loading units everywhere.** Should a chain (chapter), an OSCT group (XR) and a row (show) all be "what is loaded together"? *Recommended:* yes. The budget stays the player's: data tier, GPU, governor.
