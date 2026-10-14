Status 2026-10-14: plan; waits for the owner on Q1–Q4; nothing built (FEATURES 10.97).

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
**1. The score = an item with a `score` field** (no new catalog table – one item per score, unlike albums with several
carriers): category `score` (new `ItemCategory`), `score: { musicxml, pdf }` (CDN paths, admin slots in Catalog →
Items like an album's download). `name` = the song title (locale map). `icon` = the shared cover (a blank songsheet);
`ItemIcon` letters `name` in the reader's language onto it – in the inventory, the song book, the item detail and on
the slice (sized with the rect, container-query units – rem rule).

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
  The alternative is **OpenSheetMusicDisplay** (OSMD, Phonicscore, Vienna – the engine of their PracticeBird app):
  MusicXML-native (Verovio converts MusicXML to MEI internally), pure JS on VexFlow, smaller, BSD licence (Verovio:
  LGPL), a cursor for following along; but endless scroll by default (`pageFormat` gives paper pages, not viewport
  pages), it lays out the whole score at once instead of one page on demand, and its audio player is for GitHub
  sponsors only (Verovio's MIDI is free). Decided by a spike: the owner's real MusicXML files in both, phone + desktop,
  two scales – lyrics, page breaks, load time. pdf.js is out (no reflow).
- Controls (less UI): scale − / + as one square group (`sc-seg`, fit-to-width = the default), page n / total with
  ‹ › when it needs more than one page, swipe / arrow keys / PageUp-Down (Bluetooth page-turn pedals send these –
  the live musician gets this for free), "PDF" download, close. Re-renders on resize / rotation, keeps the page.
- Lives in `@scutoons/reader` (no app state) so live mode reuses it; the viewer wraps it.

**5. Access.** Owning the score item = open + download. CDN files are public by path like album zips – the gate is
the reader UI (same strength as `albumDownload`); good enough for sheet music? (Q4)

**6. Creator / admin.** Admin → Catalog → Items: the `score` slots (MusicXML, PDF), validation (both present, the
MusicXML parses – a Verovio load test in the admin, the server checks extension + size). Creator: nothing new – a
score is placed with "Place an item …".

## Open questions
- **Q1 lyrics per language:** a score's lyrics are text inside media. (a) One MusicXML + PDF for all languages
  (recommended for now – the rule holds, a German song stays German); (b) per language with the source as fallback
  (`score: { [locale]: { musicxml, pdf } }`) – an exception to the media rule.
- **Q2 scores without a song book:** picked up before the reader has the book – (a) loose in the inventory, moving into
  the book once it is found (recommended), or (b) not pickable until then?
- **Q3 one song book per toon or one for all toons** (`toon: null`)? Recommended: one for all, filtered by toon inside.
- **Q4 access:** is a UI gate enough, or must the PDF / MusicXML be served by the API only to owners?

## Steps once answered
1. content: `ItemCategory 'score'`, `ItemDef.score`, the `songbook` skill, validation + schemas, tests; a catalog
   migration only if a column is needed (the item row's JSON may already carry it – check CATALOG.md).
2. admin: the two slots + checks. 3. reader: `ItemIcon` lettering, `ScoreView` (Verovio, lazy). 4. viewer: the song
   book view, the detail actions, the pickup notice, the grid filter. 5. `check:*` browser script for the score view
   (phone + desktop, pagination at two scales). Drafts of the score view on the real page before step 4.
