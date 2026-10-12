# FEATURES — open feature requests, changes, owner steps (open items only; done = the item leaves)

Bugs: agents/ISSUES.md (same number line). Template and rules: agents/RULES.md "Docs" – at most 5 lines an item, the
next free number is **10.91**, a done item leaves in the same commit (`10.N done` in the message). Older notes of an
item: `grep "10.N" agents/archive/BACKLOG.md`. `[owner]` = only the owner can do it; `decide` = waits for the owner.

## Creator, admin
- [ ] 10.90 **Catalog in the creator: after the deploy + nine questions** (research/CATALOG-IN-CREATOR.md) – [owner]
  right after the deploy nobody holds `physical`: admin → Users → you → review rule "Physical items", toon `*` – until
  then nobody (admins too) publishes a physical change. `decide` D1–D9 (four eyes, who gets `physical`, cross-toon
  entries, more physical fields, removing, the admin's tabs, products / editions, recipes' toon, physical inputs).
- [ ] 10.83 **Desktop UI fixes – collecting** (owner 2026-10-11: "i like some fixes. we collect them here") – add each
  fix as its own item here. The desktop creator taking the phone concepts is a DRAFT session for the owner (branch
  `claude/zealous-faraday-e6338b`) – new desktop fixes go there or here, not both.
- [ ] 10.14 **Admin claim page rework** – owner 2026-10-02: "the ui in admin for claim needs rework"; no raw data
  (was 10.16b). Details from the owner first; first guess: the two note fields side by side + the decision buttons
  hidden in the bottom bar. Also maybe reword the viewer claim form's "the owner never reads the statement".
- [ ] 10.21 **Asset picker rework** – owner 2026-10-03: "there is redundance and confusion". Drafts first (2–3 layouts):
  the choice box above the grid, the upload as a fold-out, the page's path fields repeating the picker (incl. 10.20).
- [ ] 10.26 **Delete a chapter safely** – owner 2026-10-03: "super dangerous". Proposal: only a never-published,
  last chapter of its season; typed confirmation; removed from the sequence + marked deleted, versions kept (admin
  restores); a published chapter is only hidden. Server enforces, the creator mirrors. `decide` the rules.
- [ ] 10.64 **Media standard: the rest** – importers + admin → Assets → Standard are built (ASSETS.md "The media
  standard"). [owner] after the deploy: run the tool once on production (71 convert, 14 relink), publish its drafts,
  "Unused" cleans up. `decide`: missing 2x / 3x variants as a creator warning? Video blocked (no ffmpeg on the host).
- [ ] 10.68 **Phones: the selection bar's own back arrow duplicates the crumb** – `decide` whether it goes (owner
  2026-10-11 chose the crumb row as the way back up; that part is built).
- [ ] 10.69 **Media quality: three open questions** (from the media-quality merge note) – `decide`: Automatic's
  thresholds (≥ 7 Mbit/s → 1080p, ≥ 5 → 720p); the season trailer's hero loop still picks by viewport – move it to the
  quality setting?; should a member's choice apply when signed out on the same device (it does now)?
- [ ] 10.70 **Slice editor zoom flyout: square or round?** (10.52 / E.4) – `decide`: one square group like the Flow's
  zoom, or round as the E.4 draft showed. Also `CloseButton` round too? (DESIGN "Close cross" leaves it open.)
- [ ] 10.71 **UI-CONCEPTS E.9: the 9 phone findings** – deferred to the owner (archive/UI-CONCEPTS.md E.9). `decide`.
- [ ] 10.72 **Expressions polish: a chip shows its code** (EXPRESSIONS.md "State").

## Map
- [ ] 10.89 **Map / character documents: four questions left** (research/MAP-DOCUMENTS.md) – `decide`: D5 History
  panel per place / character, D6 hide checklist lines equal to the published version, D7 the server refuses a route to
  an unknown place, D8 creation order vs an explicit order list. Built as they are now.
- [ ] 10.25 **Map places ↔ story** – owner 2026-10-03 ("isn't group the better place?"). Unlocking is decided since
  2026-10-11 (only the `unlockLocation` event, MAP §8.5a); open: the place card's link to a group label instead of the
  chapter start, renaming groups to their places (labels are in jump links). `decide`.
- [ ] 10.65 **After the 2026-10-11 map session** – [owner] publish the map draft (dioramas, new places ore-pit /
  ore-mountain / vienna-subway, waterfall-tower; give them unlockLocation events) and move the migrated unlock events
  where the story reaches the place. Build: "Import images" understands `-plaster` `.2x` `.3x`; rename the place form's
  "Chapter link (card)" (owner: reads like an unlock); the resolution slot's layout (owner: preview big, 1x/2x/3x chips
  stacked beside it at a third of its height each, the clear-all × below). Parked: the "Ausschnitt" mode.
- [ ] 10.66 **Place dioramas: world-fixed size, clickable, outlines, routes behind** – owner 2026-10-11 ("die Dioramen
  müssen wesentlich kleiner sein"). Built: world-unit size, hex grid (`snapPlace`), the ÷8 scale (10.67). Check what is
  left: (2) the image selects the place, (3) a dashed outline for a place without an image, (4) routes behind dioramas.
- [ ] 10.63 **Map rendering: culling, a spatial index, tiles** (research 2026-10-11) – when needed: viewport culling with
  rbush / quadtree, the terrain as WebP for the usual zooms, a tile pyramid for deep zoom. Measure first on a mid-range
  Android (×5 / ×10 / ×25 places).
- [ ] 10.73 **Map M3–M5** (MAP.md §8.6): the events `revealRoute`, `travel`, `setPosition`, `focusMap`,
  `runMapEvents`; the event pickers, "Used in", the server's 409 `in_use`; the sandbox story.

## Reader
- [ ] 10.88 **Count guests in the reading statistics** – owner 2026-10-12: "Gäste mitzählen. Das ja, mit Seele." –
  `decide` first what "mit Seele" means (dictation unclear). Guests' save games live only in localStorage: an anonymous
  write under `scutoons_device` → a new table (migration) → `ReadingReport` adds "Gäste"; the privacy page must say it.
- [ ] 10.12 **Credits / download pages unlinked** – `/catpope/credits` and `/catpope/download` work, nothing links them.
  `decide`: link them (toon page / season slot / menu) or drop the routes.
- [ ] M.3 **VN: the end actions inside the message box** (part 2; part 1 built 2026-10-07) – drafts on the real page
  first (archive/UI-CONCEPTS.md E.5). [owner] a look on the phone at part 1 (characters stand on the box / bar).
- [ ] 10.56 **Reader FX: an item's own pickup sound** (catalog `ItemDef.sound`, a migration) – `decide` the sound
  design first (research/FX.md open decisions). Phases 1–4 built.
- [ ] 10.74 **PWA: preloading / save to device** (DATA.md §6 concept) – after the data refactor, which is done.
- [ ] 10.75 **Performance P4–P7** (research/PERFORMANCE.md) – check what is left: P5 production PHP tuning (OPcache,
  compiled DI container, authoritative classmap), P6 cache headers / Brotli on the CDN, P7 parallel catalog loads.

## Items, content
- [ ] 10.38 **[owner] Taking items on production** – set "a story may take it" on the items in admin (e.g. cucumber,
  lsd), and the toptoy ch1 blotter-deal content via the creator (the dev draft 819 isn't on production).
- [ ] 10.76 **Items: open owner decisions** (ITEMS.md "Where to buy" / "Record player repair") – how the tape / CD
  player are obtained (`grantable: []`); the repair coupon's place once chapter 5 exists (move from `s1c2-slice-22`);
  the Awakening track list + durations; music button vs the vinyl's related entry; a viewer display for chapter
  related items. `decide`.
- [ ] 10.77 **Respawn: should the server also read a placement's own interval?** (today only the item's range is
  enforced). `decide`. An onion item in the catalog is needed before onions can respawn (onion camera parked to Feb 2027).
- [ ] 10.78 **[owner] Text-free season cover art** – upload it and switch the cover editor's lettering on (DESIGN
  "cover editor"); lettering-free re-exports of the media in DATA.md "Audit" too.
- [ ] 10.79 **Account restart** – concept waits for the owner's answers Q1–Q9 (research/ACCOUNT-RESTART.md); the
  map-only reset is built.
- [ ] 10.59 **Storyboard: script → shots → Figma → chapter draft** (research/STORYBOARD.md §9; timeline ch3 2026-11-15,
  ch4 12-15, ch5 2027-01-15). Branch `storyboard` in its own checkout (owner's request). Open: the owner's answers per
  scene (STORYBOARD §10), the first Figma bridge run, the remaining generated shots.

## Platform, release
- [ ] 10.8 **Public release** – one version everywhere (`1.0.0`, today root 0.0.0, frontends 0.1.0, api/qr 1.0.0),
  tag `v1.0.0` (owner 2026-10-03).
- [ ] 10.4 **Asset folder restructure** – `assets:layout` re-run + owner review, batch relink, the model path change,
  then prune (ASSETS.md "Migration plan"); backups of the bunny storage later.
- [ ] 10.62 **A derived reference index for the content documents** (research 2026-10-11) – `entity_refs` filled on
  save / publish by one extractor (TS + PHP) + a rebuild command → "used by", guards on delete / rename, migrations
  that find targets by SQL; stable ids for referenced things (slices have none). When the triggers show up.
- [ ] 10.19 **`console setup`** – a setup step for a fresh environment (owner: priority last).
