Status 2026-10-14: plan; Q1, Q4 + the renderer (Verovio) decided, waits for the owner on Q2, Q3, Q5; nothing built (FEATURES 10.97).

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

**2. The song book = a lens, not a container.** Items don't nest today, and a real container (scores moved "into" an
item) would change the inventory model, the server's grants, gifting and guest import. Instead: the song book is an
item with the skill `songbook`; it lists every score the reader owns. While the reader has a song book, scores leave
the main inventory grid and appear only inside it. "Unlocked" vs "a regular item" is then one model: unlocked =
`starter: true` (every reader has it), found = `grantable: ["story"]` placed on a slice, bought = a product. Pick a
score up → a notice "In dein Liederbuch gelegt" / "Added to your song book".

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
- Controls (less UI): scale − / + as one square group (`sc-seg`, fit-to-width = the default), page n / total with
  ‹ › when it needs more than one page, swipe / arrow keys / PageUp-Down (Bluetooth page-turn pedals send these –
  the live musician gets this for free), "PDF" download, close. Re-renders on resize / rotation, keeps the page.
- Lives in `@scutoons/reader` (no app state) so live mode reuses it; the viewer wraps it.

**5. Access – protected** (owner 2026-10-14: "we will need a protection for this soon"). Built from the start:
a file once on a public CDN path stays out there. The score files live outside the public paths; the API answers
`GET /api/items/<item>/score/<pdf|musicxml>` only to a reader who owns the item (account or a guest's device
inventory? – guests can't prove ownership to the server, so: members only, Q5) with a short-lived signed URL
(bunny.net token authentication on a separate pull zone – [owner] enable it, the key goes to the server's env).
Album downloads can move to the same path later.

**6. Creator / admin.** Admin → Catalog → Items: the `score` slots (MusicXML, PDF), validation (both present, the
MusicXML parses – a Verovio load test in the admin, the server checks extension + size). Creator: nothing new – a
score is placed with "Place an item …".

## Open questions
- **Q2 scores without a song book:** picked up before the reader has the book – (a) loose in the inventory, moving into
  the book once it is found (recommended), or (b) not pickable until then?
- **Q3 one song book per toon or one for all toons** (`toon: null`)? Recommended: one for all, filtered by toon inside.
- **Q5 guests:** a guest's inventory lives only on the device, the server can't check it – score files only for
  members (a guest sees the cover + "Sign in to open"), or open the MusicXML in the view for guests and protect the PDF?

## Steps once answered
1. content: `ItemCategory 'score'`, `ItemDef.score`, the `songbook` skill, validation + schemas, tests; a catalog
   migration only if a column is needed (the item row's JSON may already carry it – check CATALOG.md).
2. admin: the two slots + checks. 3. reader: `ItemIcon` lettering, `ScoreView` (Verovio, lazy). 4. viewer: the song
   book view, the detail actions, the pickup notice, the grid filter. 5. `check:*` browser script for the score view
   (phone + desktop, pagination at two scales). Drafts of the score view on the real page before step 4.
