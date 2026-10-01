# MAP — one world map for all of SCUTOONS: data, events, the creator's map builder, location images, a sandbox story

Written 2026-10-01 (branch `platform`) as a concept; **M0 + M1 + M2 built the same day on branch `map`** (owner decisions
and what differs from the concept: [§8 Built](#8-built-m0--m1--m2-2026-10-01-branch-map)). Companion docs: [DATA.md](DATA.md) (entities, API),
[DESIGN.md](DESIGN.md) (reader + creator UI, follow-up bar, VN layout), [ITEMS.md](ITEMS.md) (grant bridge, inventory
notices), [PLAN.md](PLAN.md) (rules: small configuration, API-first, the creator as the only editor).

**Owner request (2026-10-01):** a map builder in the creator; map entries linked to VN events **and** to events in the
scroll (webtoon) reader, e.g. "unlock location"; routes and waypoints; small transparent WebP illustrations per place
(Luma or another generator), added in the builder; "for now the world map we have is the entire universe of SCUTOONS" →
one world map for every toon; an example story in the sandbox with **all** characters that exercises every map
feature; built on the existing Cat Pope map data.

**Timing:** this is post-release work (release 2026-10-04 has priority). Nothing here blocks or changes the release.

Mockups (wireframe level, not committed): `%TEMP%\claude\C--Development-scutoons\2736b5c1-…\scratchpad\map-concept\`
`builder-mockup.png` (creator, 1440 × 900), `reader-mockup.png` (reader, 390 × 844), `terrain.png` (the terrain with
today's 9 catpope locations).

---

## 1. What exists today

| Piece | Where | What it does |
|---|---|---|
| Map overlay | `viewer/src/lib/components/toon/map/map.svelte` (+ `layers.svelte`, `layer.svelte`, `routes.svelte`) | Mounted once in the toon layout (`routes/(toons)/[toonId]/+layout.svelte`: `<Map {locations} {routes} />`). Round toggle top right (`right: 5.5rem`, `map.svg`), key `M` toggles, Escape / click outside closes (`nav.toggleOverlay('map')`). Fullscreen cyan (`#00d1ff`) panel, 4 decorative frame SVGs (`catpope/images/vectors/vector-demon-saw-1.svg`, hard-coded to catpope), slide-in with an overshoot curve. |
| World size + layers | toon settings `map { layers: { terrain }, width, height, center }` | catpope: `terrain = images/map/map-layer-terrain.svg` (337 KB, 506 paths, 2048 × 1046), center (1320, 440). The sandbox points at the same file via `../catpope/…`. **In flight (parallel agent, 2026-10-01):** `map` becomes a language-neutral field stored in `toon:<toon>` (`TOON_NEUTRAL_FIELDS`). |
| Locations | entity `locations:<toon>:<l>` (seed `catpope/{de,en}/locations.json`) | 9 catpope places, per locale (only `label` differs): `{ id, label, toonId, type: "chapter", chapterNumber: "1", image: "", coordinates: { x, y } }`. Rendered as 1 rem circles + a label 25 px above; click → `/<toon>/chapter/<chapterNumber>?label=<location id>` (no group has such a label → it just opens chapter 1). `image` is always empty and unused. |
| Routes | entity `routes:<toon>:<l>` (seed `catpope/{de,en}/routes.json`) | 1 route `season1` "Reinigung und Erleuchtung" (en key is misspelled **`titel`**): `locations: [airport, apartment, medival-street, cathedral, station, monastery, graveyard, garden]`. Drawn as one 8 px black Bézier (tension 0.3). **`garden` does not exist** (silently skipped); `mountain-path` and `library` are on no route. |
| API | `/api/library/toons/<toon>` (`ToonBundle.locations`, `.routes`), `Library.php` `$list('locations')` | Per toon, per locale. No editor: DATA.md lists the map texts as "import (no editor yet)". The creator only knows the kinds (`doc.svelte.ts` → kind `map`, `entityLabel` → "map"). |
| Types | `@scutoons/content` `Map`, `MapLayers { terrain }`, `MapLocation`, `MapRoute`, `MapCoordinates` | |

Today's catpope places on the terrain (world px): north city cluster — airport (1254, 252), apartment (1206, 304),
medival-street (1250, 310), cathedral (1182, 323), station (1202, 332); south-east cluster — mountain-path (1448, 554),
graveyard (1441, 569), library (1431, 586), monastery (1428, 600). The terrain has a big radial city in the middle
(the "ring", around (1290, 440)), a long western land, an island at ≈ (785, 615), a north-east land and a southern coast.

**Gaps**
1. One map per toon (copied into the sandbox); no shared world.
2. No state: every location is always visible, nothing can be unlocked, there is no "you are here", no travel.
3. Routes are a list of location ids only: no free waypoints, no style, no partial reveal, a dangling id goes unnoticed.
4. No pan / zoom / pinch: the content is fixed at zoom 1 around `center`; on a phone (375 px) the 5 northern places
   sit within ≈ 70 world px and their labels overlap (see `reader-mockup.png`).
5. No editor; no validation; `image` unused; frame art and the cyan background are catpope-specific.
6. Location click jumps to a chapter by a bogus `label`.
7. Hidden toggle stays in the DOM (fixed already: `.map.hidden { pointer-events: none }`), but the overlay is mounted
   with all locations + the 337 KB SVG on every toon page, even when never opened.

---

## 2. Data model

### 2.1 One entity: `map` (platform-wide, language-neutral)

Like `characters`: one document for the whole universe, names as `{ de, en }` inside (DATA.md §1: registry texts are
`{ de, en }`). **Key `map`** (kind `map`, shape `[false, false, false, false]` in `entities.ts` / `EntityKey`). If a
second map ever exists (an interior, another planet) the key grows to `map:<id>` – not now (keep the configuration small).
Versioned like every entity (`entity_versions`: drafts, publish, history, restore); the creator writes it through
`PUT /api/entities/map`; seed file `shared/content/seed/scutoons/_/map.json` (next to `characters.json`).

```ts
/** The SCUTOONS world map (entity `map`) */
interface WorldMap {
	/** world units = px of the terrain artwork (2048 × 1046 today); every position is in this space */
	width: number;
	height: number;
	/** drawn bottom → top; path relative to the CDN root */
	layers: MapLayer[];
	locations: MapPlace[];
	routes: MapPath[];
}

interface MapLayer {
	id: string;                       // 'terrain'
	image: string;                    // 'catpope/images/map/map-layer-terrain.svg' (existing file, not moved)
	/** shown only from / up to this zoom (e.g. a street-detail layer for the city) */
	minZoom?: number;
	maxZoom?: number;
}

interface MapPlace {
	id: string;                       // slug, stable forever (events reference it)
	name: { de: string; en: string }; // more languages as keys, fallback to the source language
	x: number; y: number;             // the place's ground point, world units
	image?: MapImage;                 // the small illustration (§5)
	/** the toons whose story uses the place (the builder's "used by"; the reader's per-toon view) */
	toons: string[];
	/** hidden until an event unlocks it (default: visible from the start, like today) */
	hidden?: true;
	/** the chapter the place's card links to (migrated from `chapterNumber`) */
	chapter?: { toon: string; number: number };
}

interface MapImage {
	src: string;                      // 'map/locations/cathedral.webp' (CDN root)
	/** drawn width in screen px at zoom 1 (default 48); the image keeps its ratio */
	size?: number;
	/** the ground point inside the image, % of its box (default 50 / 90 = bottom centre) – sits on (x, y) */
	anchor?: { x: number; y: number };
}

interface MapPath {
	id: string;
	name: { de: string; en: string };
	toons: string[];
	/** ordered, ≥ 2; a waypoint is a place or a free point */
	waypoints: Waypoint[];
	/** how the line is drawn (default 'road') */
	style?: 'road' | 'trail' | 'flight' | 'sea';
	/** hidden until an event reveals it (default visible, like today's season route) */
	hidden?: true;
}

type Waypoint =
	| { location: string }                                          // id = the location id
	| { id: string; x: number; y: number; name?: { de: string; en: string } };  // a free point (a bridge, a pass)
```

Why these fields, and nothing else (PLAN.md "keep the configuration small" – which screen edits it, which code reads it):

| Field | Edited in | Read by |
|---|---|---|
| `width`, `height`, `layers` | builder → Layers | reader map, builder canvas |
| `x`, `y`, `name`, `image` | builder → place | reader map, toast, card |
| `toons` | builder → "Toons" chips | library (per-toon view, visibility of draft toons), reader (labels), validation |
| `hidden` | builder → "versteckt bis freigeschaltet" | reader state (§2.4) |
| `chapter` | builder → place | reader card ("Kapitel 1 lesen →") |
| route `waypoints`, `style`, `hidden` | builder → route | reader lines, travel animation, events |

Not modelled (on purpose): region/continent hierarchy, travel times, opening hours, per-location music, place
descriptions (story context, not product data), per-locale coordinates (the old files had them; they were identical).

**Waypoint ids:** a place waypoint's id is the place id; a free waypoint has its own id, unique **within the route**.
Events address a waypoint as `{ route, to: '<waypoint id>' }`. A route passing the same place twice: `to` means the next
occurrence after the reader's current position on that route.

### 2.2 Per-toon views

No second list per toon. The toon's view is **derived**: places/routes whose `toons` contain the toon are its own
(labelled, full size); places of other toons that are visible to the reader are shown as plain markers without a label
(the one universe: Cat Pope's cathedral is on the map when you read Top Toy). Where the map opens = the reader's
position in this toon (§2.4), else the toon's optional start view, else the centre of the bounding box of its own places.

The toon's start view stays where it is today and shrinks to `map?: { center?: { x, y }, zoom?: number }` in
`toon:<toon>` (the neutral fields, in flight). `width`/`height`/`layers` leave the toon (they belong to the world).
For **older viewer builds** the library keeps filling `toon.map` with `{ layers: { terrain }, width, height, center }`
from the world map + the toon's start view (one release, then drop).

### 2.3 API (library, reader-side)

- New: `GET /api/library/map?locale=&toon=` → the world for this reader: `{ width, height, layers, locations, routes }`
  with `name` resolved to the locale (fallback to the source language, `X-Content-Fallback` as elsewhere) and only the
  places/routes of **listed** toons (published; drafts only with `CONTENT_SHOW_DRAFTS` / `?draft=1`, like
  `Library::DEV_ONLY_TOONS` for the sandbox). An announced toon's places (Top Toy, Mausboss) stay out until the toon is
  published – they would announce chapter content. Same cache rules (ETag, `public, max-age=60`, keyed by version id).
- `ToonBundle.locations` / `.routes` stay, **in today's shape** (`MapLocation { id, label, toonId, type, chapterNumber,
  image, coordinates }`, `MapRoute { id, title, locations }`), computed from `map` for the toon, for older viewer
  builds; the new viewer reads `/library/map` (lazily, when the map opens – §8 performance).
- Creator: `GET/PUT /api/entities/map` (+ versions, publish, restore) – nothing new.
- Compare test (`entities-compare.php`): the bundle's `locations`/`routes` before and after the migration are equal
  (except the dangling `garden`, see 2.6) – this proves the values survived.

### 2.4 Reader state: unlocked, revealed, travelled, "you are here"

Not content – the reader's game state, stored like progress:

- **Where:** a world-wide game state under the reserved id **`scutoons`** – localStorage `scutoons_game_scutoons`, for
  members `GET/PUT /api/progress/scutoons/` (works today: `ProgressService::TOON_PATTERN` accepts any id; reserve the
  id so no toon can be called `scutoons`). Unlocks are world-wide (one universe), positions per toon.
- **Shape** (inside the existing `GameState`, field `map`):

```ts
interface MapState {
	/** place id → when it was unlocked / first seen on the opened map (ms) */
	places: Record<string, { unlocked: number; seen?: number }>;
	/** route id → revealed up to waypoint index (inclusive), travelled up to index */
	routes: Record<string, { revealed: number; travelled?: number; seen?: number }>;
	/** toon → where its hero is ("you are here") */
	positions: Record<string, { location: string } | { route: string; waypoint: string }>;
}
```

- **Merge, not last-write-wins.** The game state sync is last-write-wins (`game.svelte.ts`); for unlocks that would
  lose a discovery made on a second device. The map state merges as sets: union of keys, the smaller `unlocked`, the
  larger `revealed`/`travelled`; positions take the newer write. Done client-side in the sync (`applyRemote` → merge,
  then push); the server stays as it is.
- **Guest → member:** on login the guest's `scutoons` state merges into the account's (same merge) – no import endpoint.
- **Visible on the map** = `!hidden || state.places[id]`; a route line = `!hidden || state.routes[id]` (drawn up to
  `revealed`). New = unlocked but not `seen` yet (cyan halo until the map was opened with the place on screen).

### 2.5 Entity keys and versioning (DATA.md)

| Key | Kind | Status after the migration |
|---|---|---|
| `map` | `map` (new, platform-wide, neutral) | the only source |
| `locations:<toon>:<l>`, `routes:<toon>:<l>` | existing | **deprecated**: left untouched in the store (rollback), no longer read by the library once the `map` entity exists; removed from the seed after one release; `EntityKind` keeps them read-only for the admin overview |
| `toon:<toon>` `.map` | in flight | reduced to the optional start view `{ center?, zoom? }` |

`entityLabel('map')` → "Karte" / "Map"; admin content overview gets a "platform-wide" row for it.

### 2.6 Migration from catpope's `locations.json` / `routes.json` (all values survive)

A one-time script `shared/content/scripts/migrate-map.ts` (pattern of `migrate-characters.ts`: writes the seed, then
`import-content --only=map --update`; `--no-import`):

| Old (per locale) | New (`map`) |
|---|---|
| toon settings `map.width/height` (2048 × 1046) | `width`, `height` |
| `map.layers.terrain` = `images/map/map-layer-terrain.svg` (toon-relative) | `layers: [{ id: 'terrain', image: 'catpope/images/map/map-layer-terrain.svg' }]` – the **same file**, no CDN move (PLATFORM.md CDN rules) |
| `map.center` (1320, 440) | `toon:catpope` `map.center` (the start view) |
| location `id`, `coordinates.x/y` | `id`, `x`, `y` (unchanged) |
| `label` (de file) + `label` (en file) | `name: { de, en }` – verbatim, incl. "Medival Street" (a typo fix is an owner decision, §7) |
| `toonId` | `toons: ['catpope']` |
| `type: 'chapter'` + `chapterNumber: '1'` | `chapter: { toon: 'catpope', number: 1 }` |
| `image: ''` | dropped (no image) |
| route `id`, `title` (de) / `titel` (en, typo) | `id`, `name: { de, en }` (the script reads `title ?? titel`) |
| route `locations[]` | `waypoints: [{ location }]` in the same order, `style: 'road'`, `toons: ['catpope']` |
| `garden` (unknown) | **kept** as `{ location: 'garden' }` → the validation reports it ("unknown place") and the reader skips it exactly as today; the owner places "Garten" in the builder (§7 Q5) |
| sandbox `locations`/`routes` (empty), sandbox `map` (copy of catpope's) | nothing; the sandbox uses the world (§6) |

All catpope places start visible (no `hidden`) → the published reader looks the same until the owner decides
otherwise. Seed SQL: one new `map` statement; the old statements keep their timestamps.

---

## 3. Events: one map vocabulary for VN scripts and scroll slices

### 3.1 The events

Added to the `VNEvent` union (`shared/content/src/visual-novel.ts`) as `MapEvent`. Because a slice event is
`VNSliceEvent = VNEvent & { trigger }`, the same five events work **in VN scripts and on scroll slices** with no
second vocabulary.

| Event | Params | Effect (state) | Reader sees |
|---|---|---|---|
| `unlockLocation` | `{ location, silent? }` | `places[id].unlocked` | toast "Neuer Ort: Kathedrale" / "New place: Cathedral" (place image as the icon), map badge |
| `revealRoute` | `{ route, upTo? }` (waypoint id; default the whole route) | `routes[id].revealed` = index | toast "Neuer Weg: Ringstraße", badge |
| `travel` | `{ route, to?, from?, show?, wait? }` (`to` default = the route's end; `from` default = the current position on that route, else its start) | reveals the route up to `to`, `travelled` = index of `to`, **unlocks every place passed** (arriving somewhere = discovering it), `positions[toon]` = `to` | with `show`: the map opens, the toon's marker moves along the line (≈ 1.2 s per segment, linear; reduced motion: jumps), closes on click; without `show`: toasts for the new things only |
| `setPosition` | `{ location }` or `{ route, waypoint }` | `positions[toon]`, unlocks the place | the "you are here" marker moves (no toast unless the place is new) |
| `focusMap` | `{ location? , route?, zoom?, wait? }` | – | opens the map centred on it (a route: fitted); `wait` (VN): the script continues when the reader closes the map |

"Waypoint reached" is `travel { route, to: '<waypoint>' }` – no sixth event. No `lockLocation` (no story needs to
forget a place; add it when one does).

Examples (scene / chapter JSON, the same in de and en – map events carry no text):

```json
{ "type": "unlockLocation", "params": { "location": "cathedral" } }
{ "type": "travel", "params": { "route": "sandbox-crossing", "to": "bridge", "show": true, "wait": true } }
{ "type": "travel", "params": { "route": "sandbox-ring", "to": "station" },
  "trigger": { "type": "intersect", "once": true } }
```

### 3.2 Scroll slices: "when the slice is seen"

The existing slice events already have it: `slice.events[]` with `trigger: { type: 'intersect' }` and **no `rect`** =
the whole slice (`vn/triggers/intersect.svelte`, threshold 0.5 – the same rule as `sliceSeen` in track-reading.ts).
No new field. Two changes in the runner:

1. **Map-only triggers skip the VN machinery.** `EventProcessor.run()` calls `vn.interrupt()` and, at the end,
   `vn.endOfEvents()` – which sets `eventsDone` and would let the follow-up bar think a VN group finished. A trigger whose
   events are all map events goes to a small `runMapEvents()` (the map context) instead. Inside a VN script the
   processor's `processEvent` hands map events to the same function.
2. **Fast scrolling.** A fling can skip a slice's 50 % moment. `travel { to }` is defined as "at least up to `to`",
   so a later waypoint implies the earlier ones; an `unlockLocation` on a skipped slice is caught because the slice's
   bottom edge passes (`sliceSeen` rule) – use `sliceSeen` for map triggers, not the bare 0.5 threshold.

Rendering in the scroll reader: never opens the map by itself (`show`/`focusMap` on a scroll slice = a validation
warning: it would hijack the scrolling). Toasts + the badge only.

### 3.3 Persistence and idempotency

- Every event is a **set operation on the map state** (§2.4): firing it again changes nothing and shows nothing. That
  covers "↺ Nochmal", scrolling back, a replayed VN group, a second device.
- A toast only when something changed (a new place, a longer reveal). `silent` is for setting up a state without
  noise (e.g. a prologue that starts the hero at home).
- `positions` always update (moving back along a route is allowed – the story may go back).
- No server call per event: the state is saved like progress (debounced push for members). No rate limits, no refs
  (unlike item grants, which are server-checked; ITEMS.md).

### 3.4 What the reader sees

- **Toast:** the existing inventory notice (`items-host.svelte`, `inventory.notices`, 3.5 s, `aria-live="polite"`)
  becomes a generic notice host: kind `item` (today) and kind `map` (icon = the place image, else the map icon). Text
  (paraglide `map.*`): "Neuer Ort: <name>", "Neuer Weg: <name>". Several at once stack; > 3 collapse into "3 neue Orte".
- **Map toggle badge:** while something is new and not seen: a 10 px `primary-cyan` dot with a 2px ink border on the
  map toggle (cyan as a fill – DESIGN.md allows it; no animation: the `pulse` keyframe is reserved for the dialogue's
  "…"). Opening the map clears it once the new things were on screen.
- **On the map:** new places get the cyan halo (`surface-tint-cyan` → `primary-cyan`) until seen; revealed routes are
  drawn dashed (ink), travelled parts solid; reached free waypoints are small ink dots; the toon's position is the
  star's avatar (registry `own_toon`, e.g. Cat Pope's head, 2rem, `radius-sm`, 2px ink border – the VN avatar look)
  standing on the point.
- **Follow-up bar interplay:** toasts sit above the bar (bottom offset = bar height + 1rem) and never cover its
  buttons. If the finished group discovered something, the bar's optional one-line text reads "Neu auf der Karte:
  Ringplatz" / "New on the map: Ring square" as a link that opens the map – opening the map is an overlay, so the
  countdown stops by the existing rule. The two actions stay as they are (max 2). A VN `focusMap`/`travel show` always
  finishes before the bar can show (the bar waits for `eventsDone`).
- **Place card:** a click on a place opens a small card (paper, 2px ink, square) at the top: name, the image, "Du bist
  hier · Cat Pope" when it is the position, and "Kapitel 1 lesen →" when `chapter` is set and readable for this reader
  (the same rules as the chapter list; never for coming-soon/locked). Replaces today's direct jump.

---

## 4. The creator's map builder

### 4.1 Where

Universe section (pinned at the bottom of the sidebar): **Characters · Moods · Wanted images · Karte**. Route `/map`,
selection in the query (DESIGN.md "Creator navigation"): `/map?location=cathedral`, `/map?route=season1`,
`/map?layer=terrain`. Breadcrumb `Creator / Universe / Karte / Kathedrale`. One `ContentDoc` for the entity `map`
(draft / publish / history / restore / 409 on a stale base). Role `creator`.

### 4.2 Layout (see `builder-mockup.png`)

```
┌ top bar: Creator / Universe / Karte / Kathedrale ─────────────────────────────── account ┐
│ sidebar            │ canvas (the world, pan + zoom)                    │ properties      │
│ EBENEN             │ [Auswahl] [+ Ort] [Weg zeichnen] [Hand]   (tools) │ (selected item) │
│  ◉ Terrain         │                                                   │                 │
│ ORTE (n)           │      terrain + routes + places + images           │                 │
│  …                 │                                                   │                 │
│  + Ort             │                                                   │                 │
│ WEGE (n)           │ [Alle Toons] [Cat Pope] [Sandbox] [Leser-Ansicht] │                 │
│  ! garden: unbek.  │                              200 % · Handy-Rahmen │                 │
├────────────────────┴───────────────────────────────────────────────────┴─────────────────┤
│ Info  ↑ Universe   Entwurf · nicht veröffentlicht · 1 Problem   Verlauf  Speichern  [Veröffentlichen] │
└───────────────────────────────────────────────────────────────────────────────────────────┘
```

- **Sidebar** = the children: layers, places (A–Z, with a "hidden" mark), routes; problems inline (red "!" rows).
- **Canvas:** the terrain at world scale; wheel = zoom at the cursor, space-drag / middle-drag / the "Hand" tool =
  pan, pinch on a tablet; zoom 25 %–800 %; "Handy-Rahmen" overlays a 375 × 812 frame at the reader's default zoom so
  the owner sees what fits on a phone; label collisions at that zoom are shown (red outline).
- **Toon chips** filter the canvas (others dimmed) and set the toon for "Leser-Ansicht" (reader view: hidden places
  and routes greyed out as a reader would not see them at the start).
- **ActionBar:** Info, ↑ Universe, state + problem count, History, Save draft, Publish (as everywhere).

### 4.3 Places

- **"+ Ort"**: click on the canvas places a new place, the properties open with the name field focused; the id is
  suggested from the de name (`kathedrale` → editable until the first publish, then fixed: ids are referenced by
  events – a rename after publishing is "duplicate + replace references", offered only when nothing references it).
- **Drag** moves (snap to whole world units; arrow keys nudge 1, Shift+arrow 10); x/y also numeric.
- **Properties:** ID · Name de / en (side by side, like the text boxes) · Position · Bild · Größe · Bodenpunkt ·
  Toons (chips, + from the toon list) · "versteckt bis freigeschaltet" · Kapitel-Link (toon + chapter select) ·
  **Verwendet in** (read-only: every event that references the place, as links to the chapter/scene editor; found by
  the validation index, §4.7) · Delete (refused while referenced).

### 4.4 Routes and waypoints

- **"Weg zeichnen"**: click a place → it becomes a waypoint (snaps within 12 screen px); click on empty terrain → a
  free waypoint (id suggested `wp-1`, editable, optional name). The line follows live (the same Bézier as the reader,
  `routes.svelte` moves to `@scutoons/reader` so both draw identically). Enter / double-click finishes, Esc cancels,
  Backspace removes the last point.
- Selected route: drag a waypoint (a free one moves; a place waypoint moves the place – with a confirmation, it is
  shared), click on the line inserts a free waypoint, the waypoint list in the properties reorders by drag, "Umkehren"
  reverses it. Properties: ID · Name de / en · Stil (road / trail / flight / sea, previewed) · Toons · versteckt.
- Style rendering (reader + builder): road = 6 px ink solid; trail = 4 px ink dashed; flight = 3 px ink dotted arc
  (the curve bows upwards, tension 0.5); sea = 4 px paper dashed with an ink outline. Travelled parts always solid.

### 4.5 Location images

- **Bild:** the existing image picker (CDN browse + path field + preview) filtered to `map/locations/`; when the
  creator's CDN upload exists (PLAN.md Creator phase 2, item 1) an "Hochladen" button next to it (transparent WebP only,
  ≤ 60 KB, ≤ 512 px; refused otherwise with the reason).
- **Größe** (screen px at zoom 1, default 48) and **Bodenpunkt**: a 160 px preview of the image on a checkerboard with
  a draggable crosshair = the ground point (default 50 % / 90 %); the canvas shows the image standing on the place's
  point.
- **Preview at map zooms:** three thumbnails (50 % / 100 % / 200 %) on the terrain colour, and the phone frame on the
  canvas – the owner sees if it reads at a glance. A warning when the image is wider than 2× its size at 200 % (blurry)
  or the file > 60 KB.

### 4.6 Linking from the VN script and slice editors

- The VN event forms (scene editor) and the slice "Ereignisse" section get the five map events in the "add event" menu.
  Their fields are **pickers, never id text fields**: place = a searchable select (name in the UI language, a small
  thumbnail, grouped "this toon / other toons / hidden"); route = select; waypoint = select of the chosen route's
  waypoints (places by name, free ones by name or id); the raw JSON view stays for power use.
- Each picker has "Auf der Karte zeigen" → opens `/map?location=…` in a new tab (the builder keeps its own draft).
- On a slice the trigger defaults to "beim Sehen" (`intersect`, no rect); click/hover with a drawn rect stays possible
  (the existing trigger UI).
- Creating a place from a picker ("+ neuer Ort …") opens the builder with the name prefilled – the map entity has one
  editor (the creator as the only editor of each thing).

### 4.7 Validation (in `@scutoons/content` `validate.ts`, shown in both editors)

| Problem | Level |
|---|---|
| route waypoint → unknown place (today: `garden`) | error |
| event → unknown place / route / waypoint | error |
| duplicate place / route / free-waypoint id (within the route) | error |
| route with < 2 waypoints | error |
| position outside `width × height` | error |
| `hidden` place no event of any toon ever unlocks (also not via `travel`/`setPosition`) | warning ("unreachable") |
| a toon's events use a place/route whose `toons` lacks that toon | warning (button "Toon hinzufügen") |
| place without `toons` | warning |
| `show` / `focusMap` on a scroll slice | warning |
| `travel` on a route that is `hidden` and never revealed before (in reading order) | info (travel reveals it anyway) |
| image missing on the CDN (picker's HEAD check) / > 60 KB / not WebP | warning |
| two labelled places closer than 24 screen px at the toon's start zoom | info (label overlap on phones) |

The "used in" index for 4.3 and the cross-entity checks run in the creator over `/api/entities?toon=` documents (the
chapter + scene drafts of every toon); for the publish of `map` the server additionally refuses removing a place that a
**published** chapter or scene references (409 `in_use { refs }`).

---

## 5. Location illustrations: the image workflow

To be written as `workflows/map/location-images.md` when built (house style like `workflows/items/item-icons.md`).

### 5.1 Output spec

| | |
|---|---|
| Canvas | generate at 1024 × 1024, deliver **256 × 256 px** (one resolution – PLAN.md rule; shown at ≈ 48 px, up to ≈ 100 px at 200 % zoom) |
| Format | WebP with alpha, quality 85, **≤ 40 KB** (hard limit 60 KB) |
| Background | transparent, no ground plane beyond the place's own small base, no cast shadow outside it |
| Framing | the place as a small self-contained diorama, three-quarter view from above (≈ 30°), centred horizontally, ~6 % side padding |
| Anchor | the place's **ground point at 50 % / 90 %** (bottom centre, where the building meets the ground) – the default `anchor`, so most images need no setting |
| Content | one place, no text, no signs with letters, no characters, no logos |
| File | `cdn/root/map/locations/<place id>.webp` (PNG master next to it); a changed image gets a new name (`<id>.v2.webp`, PLATFORM.md CDN rule) |

### 5.2 Prompt template (Luma; any image model that takes style references)

Attach as **style** references: `cdn/root/characters/catpope/catpope_portrait-neutral.webp`,
`cdn/root/characters/yellowmouse/yellowmouse_portrait-neutral.webp`, and – for architecture – one chapter slice
showing the place if there is one (e.g. `catpope/images/webtoon/s1c2/s1c2-slice-15.webp` for the lantern street).

```
Attached are images from the SCUTOONS universe as STYLE references only (rendering, lighting, materials,
colour palette). Do not include the characters.

Create a single small map illustration of a place in exactly that rendering style: <PLACE DESCRIPTION>.
It is a tiny self-contained diorama for a world map: three-quarter view from above (about 30 degrees),
the place sits on a small irregular patch of its own ground, nothing else around it.
Soft realistic 3D render, same light direction and softness as the references, slightly simplified shapes so it
reads at 48 px. Centred horizontally, the point where the place meets the ground at 90 % of the image height,
~6 % padding left and right, 1024 x 1024 px, transparent background, no text, no letters on signs, no logo,
no people or animals, no shadow outside the ground patch.
```

Keep a series consistent: generate the first image of a cluster, approve it, then attach it as a fourth reference for
the others ("same scale, same ground patch, same light").

### 5.3 Post-processing (a new `cdn/tools/make_location_webp.py`, like `make_character_webp.py`)

1. Background removal if the generator returned none (alpha check; `rembg` as fallback).
2. Trim to the alpha bounding box, then pad to a square so the lowest opaque row of the ground patch's centre sits at
   90 % and the content is centred horizontally (the anchor rule; `--anchor x,y` overrides for odd shapes, then the
   builder's anchor must match – the script prints it).
3. Resize to 256 × 256 (Lanczos), WebP q85 with alpha; if > 40 KB → q75, then report.
4. Contact sheet (`--sheet`): every image at 48 px and 96 px on `#00d1ff` (the map), white and black.
5. Owner approves; artist-made images always win (same file name → re-run). Upload with `cdn/sync.py` (new folder
   `map/`), then pick it in the builder.

### 5.4 Place descriptions for today's catpope places (to generate first)

| id | Place description |
|---|---|
| `cathedral` | a white baroque cathedral with two towers and cobalt-blue ornaments on the roofs, a small stone square in front |
| `apartment` | a narrow five-storey old-town apartment house, pale plaster, one lit window, a tiny balcony with plants |
| `airport` | a small modern airport terminal with a control tower and a short runway strip with one parked white jet |
| `medival-street` | a short medieval cobblestone alley with half-timbered houses and two hanging lanterns |
| `station` | a 19th-century railway station hall with an arched glass roof and a clock above the entrance |
| `mountain-path` | a winding stony path up a steep green mountain slope with a wooden signpost without text |
| `monastery` | a remote mountain monastery with a bell tower and terraced gardens on a rocky ledge |
| `library` | an old monastery library building with tall arched windows and stacks of books visible inside |
| `graveyard` | a small walled graveyard with crooked headstones, a cypress tree and a wrought-iron gate |

---

## 6. Example story in the sandbox: "Die Karte" / "The map"

Sandbox chapter **8** (after "Sonntagnachmittag", chapter 7), generated by `make-sandbox.ts` (a `mapWalk()` like
`sunday()`), `isPreview: true`, draft like every sandbox chapter, de (source) + en. Purpose: exercise every map feature
end to end and be the regression bed for a `scripts/check-map.mjs` (headless, as `vn-check.mjs`).

### 6.1 Cast – every registry character at least once

| id | Name (de / en) | Personas / looks used | View |
|---|---|---|---|
| `catpope` | Katzenpapst / Cat Pope | regular (default), human, cat, mountaintiger, biker (default **and** helmet) | avatar (left) |
| `boy` | Junge / Boy | regular | portrait |
| `crows` | Krähen / Crows | regular | portrait |
| `yellowmouse` | Mausboss / Yellow Mouse | regular, helmet → default (the transition image) | portrait |
| `toptoy` | Top Toy | regular | portrait |
| `billthefrog` | Bill der Frosch / Bill the Frog | regular | portrait |

(Generated from the registry like the rest of the sandbox, so a new character gets a line in the finale automatically.)

### 6.2 Places and routes it uses

Existing catpope places (get `sandbox` added to `toons`): apartment, station, cathedral, airport, medival-street,
mountain-path, graveyard, library, monastery. Existing route `season1` is **not** used (it carries the dangling `garden`
– the validation test case stays on catpope's data).

New sandbox places (`toons: ['sandbox']` → never public, the library drops dev-only toons):

| id | Name de / en | x, y | hidden | Image prompt (§5.2 `<PLACE DESCRIPTION>`) |
|---|---|---|---|---|
| `ring-square` | Ringplatz / Ring square | 1290, 440 | yes | a round market square at the centre of a radial city, a fountain in the middle, five streets leaving it like spokes |
| `gang-garage` | Garage der Gang / The gang's garage | 960, 400 | yes | a corrugated-iron biker garage with a big roll-up door half open, three motorbikes in yellow and black, oil barrels |
| `toy-stage` | Top Toys Bühne / Top Toy's stage | 1100, 870 | yes | a small open-air stage made of giant toy building blocks in fuchsia pink, two spotlights, a curtain |
| `frog-pond` | Froschteich / Frog pond | 785, 615 | yes | a lily pond on a tiny island with a wooden jetty, a reading chair and a desk lamp on the jetty |
| `crows-tower` | Krähenturm / Crows' tower | 1210, 170 | yes | a tall thin crooked stone tower on a narrow spit of land, many black crows' nests on its roof, a clock without numbers |

New sandbox routes:

| id | Name de / en | Style | Waypoints | hidden |
|---|---|---|---|---|
| `sandbox-ring` | Ringstraße / Ring road | road | apartment → station → cathedral → ring-square | yes |
| `sandbox-crossing` | Über die Brücke / Across the bridge | road | ring-square → `bridge` (free, 1075, 425, "Brücke" / "Bridge") → gang-garage | yes |
| `sandbox-flight` | Der Flug / The flight | flight | airport → `over-sea` (free, 1000, 560, unnamed) → frog-pond | yes |
| `sandbox-pilgrimage` | Pilgerweg / Pilgrim's way | trail | frog-pond → `pass` (free, 1360, 600, "Pass") → mountain-path → graveyard → library → monastery | yes |
| `sandbox-stage` | Zur Bühne / To the stage | trail | monastery → toy-stage | no (always visible: the "visible from the start" case) |

### 6.3 Script outline (groups in reading order)

VN = a VN group scrolling over slices (the sandbox pattern); Scroll = a plain vertical group with slice events.
Backgrounds reuse catpope art via `../catpope/…` (no new slices).

| # | Group | Type | Who says what (de source, short) | Map events | Tests |
|---|---|---|---|---|---|
| 1 | `morning` | VN | Cat Pope: „Heute will ich die Welt sehen.“ Crows join: „Krah! Eine Nachricht aus der Zukunft: eine Karte!“ | `setPosition apartment` (silent); `unlockLocation apartment` (already visible → no toast); `revealRoute sandbox-ring upTo station`; `focusMap { location: apartment, wait }` | position marker; idempotent unlock; partial reveal; focus + wait |
| 2 | `ring` | Scroll (s1c2 slices 15–19, the street) | dialog boxes only (narration „Die Ringstraße …“) | slice 15 seen: `travel sandbox-ring to station`; slice 17: `travel … to cathedral`; slice 19: `travel … to ring-square` (unlocks the hidden ring-square) | travel without `show` in scroll; reveal beyond `upTo`; unlock by arrival; toasts above the bar; fast-scroll catch-up |
| 3 | `ring-square` | VN | Boy: „Ich hab dich gesehen! Du bist der Papst mit der Karte!“ Top Toy (duo): „Wir spielen heute im Süden – komm zu unserer Bühne!“ Cat Pope · human: „Ich komme mit. Ich bin ja du.“ | `unlockLocation toy-stage` (hidden → toast during VN); `revealRoute sandbox-stage` (visible already → no toast) | toast while the VN box is open; visible-route no-op |
| 4 | `bridge` | VN | Yellow Mouse (helmet): „Steig auf, Papst.“ Cat Pope · biker (helmet): „Brumm brumm!“ Choice: „Über die Brücke“ / „Lieber zurück“ (`jumpTo` label `garage` / `ring-back`) | `revealRoute sandbox-crossing`; `travel { to: bridge, show, wait }` | free waypoint reached with animation; choice branches |
| 5a | `garage` | Scroll (s1c2 slices 9–11) then VN | Yellow Mouse takes the helmet off (helmet → default, transition image): „Willkommen in der Garage.“ Cat Pope · biker (default): „Schicke Maschinen.“ | slice 9 seen: `travel sandbox-crossing to gang-garage`; VN: `unlockLocation gang-garage` (already unlocked by arrival → no toast) | look transition + map; double unlock |
| 5b | `ring-back` | VN | Cat Pope: „Zu wackelig.“ | `travel sandbox-crossing { from: bridge, to: ring-square, show }` then `jumpTo garage` | travelling backwards; position update |
| 6 | `airport` | VN | Cat Pope · cat: „Miau. Fliegen ist ehrlicher als Laufen.“ Cat Pope · mountaintiger: „ROAAAR! Zum Frosch!“ | `unlockLocation airport` (visible → no toast); `revealRoute sandbox-flight`; `travel { to: frog-pond, show, wait }` | flight style; unnamed free waypoint; arrival unlock of an island |
| 7 | `pond` | VN | Bill the Frog: „Ein Frosch mit Brille kennt jeden Weg. Hier, der Pilgerweg.“ Crows: „Und unser Turm – aber nur zum Ansehen.“ | `revealRoute sandbox-pilgrimage`; `unlockLocation crows-tower` (a place on no route – reachable only by an event) | revealing a long route; an unlock without a route |
| 8 | `pilgrimage` | Scroll (5 catpope slices via `../catpope/…`; s1c2 20–22 + two more, any art works – the events are the test) | dialog boxes: „Der Pass.“ „Der Friedhof.“ „Die Bibliothek.“ | one slice each: `travel sandbox-pilgrimage to pass` → `mountain-path` → `graveyard` → `library` → `monastery` | waypoint progression slice by slice; scrolling back = no change; skipped slice caught up |
| 9 | `finale` | VN | at the monastery everyone joins in waves of 4 (the sandbox's goodbye pattern, generated from the registry): each says one line about a place („Mein Teich!“, „Unsere Bühne!“, „Krah – der Turm!“ …); Cat Pope: „Die ganze Welt – und sie gehört uns allen.“ | `travel sandbox-stage to toy-stage` (silent); `focusMap { zoom: 0.25, wait }` (the whole world); `jumpTo /sandbox` | world zoom-out; every character on stage; the end |

Expected end state (the check script asserts it, guest and member): 5 sandbox places + the used catpope places
unlocked, 5 routes revealed, `sandbox-pilgrimage` travelled to its end, `positions.sandbox = toy-stage`; replaying the
chapter adds no toasts; logging in merges the guest state; a second browser profile signed in as the test user sees the
same unlocks after sync.

---

## 7. Phases, open questions, risks

### 7.1 Phases (after the release, in this order)

| Phase | What | Done when |
|---|---|---|
| **M0 data** | `WorldMap` types + schema + validation; `map` entity kind (TS + PHP `EntityKey`); `migrate-map.ts`; `/library/map`; the bundle's `locations`/`routes`/`toon.map` computed from it; seed + SQL | compare test equal (except `garden`); full suite green |
| **M1 reader map** | the map reads `/library/map` lazily when first opened; pan/pinch/zoom; images with anchor; labels by zoom (collision hiding); place card; toon position marker; mount the overlay only when opened | catpope looks as today at start, pans and zooms on a phone |
| **M2 builder** | creator `/map`: canvas, places, routes + waypoints, layers, images (picker), toons, validation, publish | the owner places "Garten" and publishes from the builder |
| **M3 events + state** | `MapEvent` types, `runMapEvents`, slice map triggers (`sliceSeen`), `scutoons` game state + merge sync, notices kind `map`, badge, follow-up bar text | the sandbox story's state assertions pass |
| **M4 editor pickers** | map events in the VN and slice editors with pickers, "used in", cross-entity validation | no id typed by hand |
| **M5 sandbox story** | `mapWalk()` in make-sandbox.ts, `scripts/check-map.mjs` (de 375, en 500) | 0 console errors, end state as in §6.3 |
| **parallel from M0** | location images (owner, workflow §5), `make_location_webp.py`, upload | catpope's 9 places have images |

### 7.2 Open questions for the owner

1. **Unlocks across toons:** a place discovered while reading Cat Pope also shows (as an unlabelled marker) when reading
   Top Toy – the "one universe" idea. OK, or strictly per toon?
2. **Catpope's existing places:** stay visible from the start (as today, proposed for the migration), or become
   `hidden` and get `unlockLocation` events in chapters 1–2 (spoiler-free, but needs those events placed)?
3. **A teaser state** ("?" silhouette for a hidden place that is announced)? Not in the model yet.
4. **Place click:** a card with "Kapitel 1 lesen →" (proposed) instead of the direct jump?
5. **Where is the "Garten"** of the season-1 route? (Near the monastery? It is in the route between graveyard and the
   end.) Or remove it from the route?
6. **Typos in the data:** fix "Medival Street" → "Medieval Street" (en name only; the id `medival-street` stays) and the
   route's `titel`? (The migration keeps them verbatim unless you say so.)
7. **The map's frame art** (the four `vector-demon-saw` saws) and the cyan sea are catpope's: keep them for the world
   map, or a neutral SCUTOONS frame?
8. **A world map outside a toon** (landing page / menu "Weltkarte")? Not proposed now.
9. **Rewards:** should reaching a place ever grant something (an item via the existing grant bridge, an avatar unlock)?
   Out of scope here; the events would compose with `add <item>`.
10. **Image size:** 48 px at zoom 1 as the default – or bigger, the illustrations as the main visual of the map?
11. **Reader map zoom on phones:** the northern cluster needs ≈ 3× zoom to separate its labels. Start the phone view at
    the toon's position zoomed in (proposed), or spread the places on the terrain?

### 7.3 Risks

- **Performance on phones.** The terrain SVG (337 KB, 506 paths) re-rasterises on every zoom step; with images and
  routes the overlay can drop frames on mid-range Android. Mitigations: mount the overlay only when opened (today it is
  always mounted), `will-change: transform` while gesturing and a re-render at rest, one transform for the whole
  content, images `loading="lazy"` + only those inside the viewport (+ margin), labels only above a zoom threshold. If
  still slow: a single raster WebP of the terrain (one resolution, ≈ 2048 px, ≈ 150 KB) instead of the SVG – measure
  first (`scripts/perf/measure.mjs`, PERFORMANCE.md).
- **Spoilers via the map.** `/library/map` sends hidden places (names, positions) to every reader – the unlock state is
  client-side, so the server cannot filter for guests. Acceptable like the client-side chapter gate (PLAN.md "Next"
  5), but: places of unpublished/announced toons are filtered on the server, and names of hidden places could later be
  sent only as ids (the name arrives with the event's chapter). Positions in the follow-up text or toasts must never
  name a place the reader hasn't unlocked.
- **Draft content leaking:** sandbox places and the 5 new routes must never reach production readers – the library
  filter by toon visibility is the guard (tested in `LibraryTest` like `DEV_ONLY_TOONS`).
- **i18n.** Names live in the neutral `map` document (`{ de, en }`): a new language means adding a key in the builder,
  missing → source-language fallback (flagged like other fallbacks). German compounds are long ("Mittelalterliche
  Strasse") – labels need `hyphens: auto` + a max width, or they collide more in de than in en.
- **Sync conflicts.** Without the set-merge (§2.4) last-write-wins loses discoveries across devices; the merge must be
  covered by a unit test in `@scutoons/content` (pure function `mergeMapState`).
- **VN side effects.** Running map events through `EventProcessor` as-is would call `vn.endOfEvents()` from a scroll
  slice and confuse the follow-up bar – hence `runMapEvents` (§3.2).
- **Event spam on replay** is prevented by idempotency; but a chapter with 20 unlocks in a row would still toast 20
  times on the first read – the collapse rule ("3 neue Orte") covers it.
- **One big document.** All places of all toons in one entity: two editors on the map at once get the 409 (stale base)
  – fine at today's team size; split per region only if it ever hurts.
- **Accessibility.** A pan/zoom canvas is hard with a keyboard or screen reader: places are `<button>`s with their
  names, Tab moves between visible places (the map pans to the focused one), and the map has a text alternative
  ("Orte" list: unlocked places A–Z, "Du bist hier"). Reduced motion: no travel animation, no slide-in overshoot.
