Status 2026-10-14: plan; Q1, Q2, Q4, Q5 + the renderer (Verovio) decided; the score view is a dev draft (`/account/lab/score`); Q3 + Q6 decided; waits for the owner on Q10, the DJ trolley's rule + capacity (FEATURES 10.97).

# SCORES — sheet music as items, a song book, the score view (plan, 2026-10-14)

Owner 2026-10-14: "i have songs in the script, and i want to be able to create special items, called scores, which i
can place like any other item … when picked up, they go into a song book. the song book itself can be either unlocked
or be a regular item … opening the song book lists the song items as like an inventory does, and the use open is
opening the score in the score view … the scores come as pdf (to download and print) and music xml … a musicxml
renderer … some scale option which can paginate if needed". The score view is also the musician's view of live mode
(research/LIVE.md, parked to Feb 2027) – built now, reused there.

## What exists and fits
- Placing and picking up: a score is a catalog item, so slice placement, pickup, respawn, once-per-ref, gifting and
  QR codes all work unchanged (ITEMS.md "Placed items").
- The album pattern (ITEMS.md "Players, albums") is the closest relative: a file-bearing item + a "you need X" rule.
- Lettering is text rendered by the UI (RULES "Everything is language-independent") – the title on the cover is the
  item's localized `name` drawn over one shared cover image, never baked into the icon.
- Songs in the script (`# <n>. Lied|Song`) are creator-only (`script` never reaches readers) – a score can't point a
  reader at the script; the item carries its own title.

## Proposal
**1. The score = an item with a `score` field, in ONE language** (owner 2026-10-14: "a score is always in one
language. you want to sing the german version, you get the german edition"): no new catalog table, one item per
edition – the German and the English edition of a song are two items. Category `score` (new `ItemCategory`),
`score: { language, musicxml, pdf }` (admin slots in Catalog → Items like an album's download). `icon` = the shared
cover (a blank songsheet); `ItemIcon` letters the title **in the score's own language** (`name[score.language]`) onto
it – the edition's real title, whatever the reader's language – in the inventory, the song book, the item detail and
on the slice (sized with the rect, container-query units – rem rule). The media rule holds: the files are one
language because the edition is.

**2. The song book = a container item** (owner 2026-10-14: "has to be moved manually, dragging it into it or clicking
selecting the score first and then clicking the book. this will be the rule for any item that can store other items").
A new item fundamental (ITEMS.md, with the code): a container item says what it takes (`holds: { categories:
["score"] }`); an inventory entry can lie in a container entry (`in: <entry id>` – a column on the inventory, a
migration; guests: in their localStorage inventory, carried by the import at sign-in). A picked-up score lands loose in
the inventory like any item; the reader moves it: drag it onto the book in the grid, or select it and tap the book
("In das Liederbuch legen" offered on the book). Taking out works the same way and puts it back into the regular
inventory (owner 2026-10-14: "it can, goes back into regular inventory"). The book's grid shows only what lies in it.
Giving a container away takes its contents along (owner 2026-10-14: "the scores go along with the book"): one
transaction moves the book and every entry in it; an entry that can't be given (not giftable, code-bound) or that
would exceed the recipient's max stays with the giver, loose; the gift form says so before sending ("3 scores go
with the book, 1 stays with you").
A song book is never a starter – it is found (`grantable: ["story"]`), gifted by a campaign or bought (Q8).

**2b. What goes into which container – one editable rule** (owner 2026-10-14: "can we decide what items can go into
what containable item? with a simple rule we can edit? so creating new containers is easier"). A container is any item
with a `holds` rule, edited in admin → Catalog → Items ("Holds" in the item form):
`holds: { categories?: ItemCategory[], items?: string[], max?: number }` – an item fits when its category is listed
OR its id is listed; `max` = how many entries fit (none = no limit). Song book = `{ categories: ["score"], max: n }`;
a pencil case later = `{ items: ["pencil", "eraser"] }`. Planned next to the song book (owner 2026-10-14: "that is
for sure"): the **DJ trolley** (`dj_trolley`) holds the records (the media items with an album – a category rule
`media`, or the record items listed; capacity open). Making a new container = filling in "Holds" on an item, no
code. Validation: unknown categories / item ids warn, a container may not hold containers (no nesting). The reader's
item detail shows where an item fits ("Passt in: Liederbuch"). If grouping by category is ever too coarse, item tags
come later – not now (keep the configuration small).

**2a. Many song books, one you can make** (owner 2026-10-14: "there can be more song books, it's a property of a song
book item. you can also create specific ones with a recipe. you need a drill and 4 zip ties, black or white or other
colors. you might need to set up those items for me"). Every song book is its own container item; what it takes is
its property (`holds`). Items to set up (admin catalog, as a proposal the admin applies – once the code knows `holds`):
| id | what | notes |
|---|---|---|
| `songbook` | container, `holds: score` | not a starter – obtained like the parts (placed / campaign gift) |
| `drill` | tool, virtual | kept by the recipe (a tool, not an ingredient) |
| `zip_tie` | stackable | placed items or campaign gifts (see below) |
| `plastic_sheet` | stackable, colour variants: black, white, red, blue, light blue, orange, green, brown, mother-of-pearl (owner 2026-10-14) | colours the book (owner 2026-10-14: "doesn't have a color unless you add a piece of plastic in a specific color") |
| `songbook_diy` | container, `holds: score`; plain (no variant) or a colour variant | output of the recipes |
| recipe `songbook_diy` | drill (kept) + 4 × zip_tie → songbook_diy (plain) | |
| recipe `songbook_diy_<colour>` | songbook_diy (plain) + plastic_sheet (colour X) → songbook_diy (colour X) | one per colour; a book with scores keeps them (owner 2026-10-14: "why would they leave the book?") |
**Colour and size** (owner 2026-10-14: "song books have color variants and maybe even size variants. so specific books
can hold only up to [n] scores"): a container's capacity is part of its property – `holds: { categories: ["score"],
max?: n }` (none = no limit); a full book refuses the move ("Das Liederbuch ist voll." / "The song book is full.").
Colours are variants. Sizes as their own items (`songbook_small`, `songbook_large` …, each with its own `max`)
rather than size variants: one variant axis per item keeps colour × size from multiplying the variants and recipes,
and a size can then have its own icon, price and recipe (Q10).
Where readers get the parts (owner 2026-10-14): for now placed items in the story (`grantable: ["story"]`) or gifts
through campaigns (research/GIFT-INVITES.md, built in its own session).
Two recipe fundamentals the code needs (ITEMS.md, with the code): an input that is **kept** (`RecipePart.keep`) and
a **variant** on a part (`RecipePart.variant` – explicit per colour, no hidden inheritance). Icons: placeholders until
the owner's art (like the players').

**3. The song book view** – the inventory grid component reused (one job: a grid of owned items), filtered to
`category: score`, opened from the book's item detail ("Öffnen" / "Open"). A score's detail: "Öffnen" → the score
view, "PDF herunterladen" (the print file).

**4. The score view** – a full-screen reader view (own component; item-detail and inventory are already > 300 lines):
- Renderer **Verovio** (`verovio` npm, bundled + lazy-loaded only here – "alles bei uns"): MusicXML in, SVG per page
  out; `scale` and `pageWidth / pageHeight` = the viewport → real pagination with good line / page breaks
  (`breaks: smart`), `getPageCount()`, `renderToSVG(n)`; later `renderToMIDI()` for playback in live mode.
  Decided (owner 2026-10-14: "go verovio … looks more elaborate") over OpenSheetMusicDisplay (Phonicscore, Vienna;
  MusicXML-native, lighter, BSD, but endless scroll first and a sponsors-only audio player); no spike. Verovio is
  LGPL: its toolkit stays a separate, replaceable file in the bundle. pdf.js is out (no reflow).
- Controls (less UI): scale − / + as one square group (a phone starts at 30 %, wider screens at 40 %, the last choice is
  remembered on the device), page n / total with ‹ › when it needs more than one page; pages are slides of the app's
  Swiper (owner 2026-10-14: "use the swiper we got already"), rendered when near; arrow keys / PageUp-Down / Space
  (Bluetooth page-turn pedals send these – the live musician gets this for free), "PDF" download, close. Re-renders on
  resize / rotation and stays at the bar the page started with; a system taller than the page scrolls.
- Draft (2026-10-14): `shared/reader/src/score/`, the dev page `/account/lab/score` with two MuseScore exports of the
  owner's songs (local `media/scutoons/scores/`, a dev copy). Learned: "Cloud 9" (SATB) reads well – a phone 4 pages,
  desktop 4; "Like A Baby" as the full score (voice, choir, organ, piano) is 30 pages on a desktop and its lyrics
  collide – the reader edition should be the choir / voice parts only, exported from MuseScore as its own file.
- Lives in `@scutoons/reader` (no app state) so live mode reuses it; the viewer wraps it.

**5. Access – protected** (owner 2026-10-14: "we will need a protection for this soon"; "guests can view the score in
the app, but only members can download the PDF"; albums the same). The files live on their own bunny pull zone with
token authentication (no public path); the API hands out short-lived signed URLs: the MusicXML to anyone who opens the
score in the view (a guest's ownership lives on the device – the server can't check it), the PDF only to a member who
owns the item; album downloads move to the same zone and rule. Signing = one HMAC (SHA-256 of key + path + expiry,
base64url) in PHP, ~30 lines + tests; [owner] create the pull zone with token auth in the bunny panel and put its key
into the server's env.

**6. Creator / admin.** Admin → Catalog → Items: the `score` slots (MusicXML, PDF), validation (both present, the
MusicXML parses – a Verovio load test in the admin, the server checks extension + size). Creator: nothing new – a
score is placed with "Place an item …".

## Open questions
- **Q10 sizes:** sizes as their own items with their own capacity (recommended), or size variants of one item (then
  each variant carries its `max`)? And the capacities: the basic book, the DIY book, the sizes.

## Steps once answered
1. content: `ItemCategory 'score'`, `ItemDef.score`, `ItemDef.holds`, validation + schemas, tests; migrations: the
   catalog columns if needed (CATALOG.md) and the inventory's `in`.
2. server: the move API (put in / take out, ownership + `holds` checked), the signed-URL endpoint, the guest import.
3. admin: the score slots + checks. 4. reader: `ItemIcon` lettering; `ScoreView` (drafted). 5. viewer: the container
   grid, drag / select-then-tap moves, the score detail (Open, PDF for members). 6. `check:*` browser script (phone +
   desktop, two zooms, a swipe). Drafts of the container moves on the real page before step 5.
