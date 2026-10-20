Status 2026-10-20: plan, nothing built. Q8's art answered (one tall picture per scene, §8); waiting for the owner's agreement and the other answers (§12, issue #77); then the milestone "Webtoon.com Publishing" (due = the launch, §12 Q2) and its issues. Not on chapter 3's critical path (#64): before 2026-11-15 only two small steps (§10).

# WEBTOON — SCUTOONS chapters as WEBTOON CANVAS episodes

Owner 2026-10-20: "I want to develop an exporter and do some modifications for scutoons to webtoon.com. I do have
vertical alternatives for horizontal scenes, we will need to ask for those and create something in the creator ui to
facilitate for this. If we have any items or text etc on top of our slices we need to bake everything into proper
slices. Please research which resolutions we need … figure out what the fastest way to import them to webtoon.com.
Also how are different languages handled there?"

**In one line:** the reader itself renders a chapter, per language, as one static column (a bare export page in the
viewer, driven by a zero-dependency local Node tool through the installed Chrome). The tool cuts the column into
800-px-wide JPEGs between text boxes, groups them into episodes of about 40k px and writes one upload kit per language.
A person uploads the kit in WEBTOON's web dashboard (there is no API, and bots are forbidden), one series per language,
English first. Horizontal groups get an optional, export-only vertical version: one tall picture of the same scene
(1:4 … 1:7) on which the frames' own text boxes and items are placed. It is asked for in Wanted images and placed in
the chapter editor.

## 1. Resolutions: what WEBTOON takes and what we export
| | WEBTOON CANVAS | Our export |
|---|---|---|
| Image width | ≤ 800 px; the desktop viewer shows every image at 800 CSS px (narrower ones are upscaled) | exactly 800 **[test]** |
| Image height | ≤ 1280 px; longer images are auto-sliced at fixed heights (through bubbles too) | 320–1280, cut by us between boxes (§4) |
| Format | JPG / JPEG / PNG; PNG is served as an 8-bit palette (bands); re-encoded without metadata | baseline sRGB JPEG |
| Per image | ≤ 2 MB | ≤ 1,900,000 B (safe whether "MB" is 10⁶ or 2²⁰) |
| Per upload | ≤ 20 MB and ≤ 100 images in total; the sources don't say "per episode" **[test]** | ≤ 19,000,000 B; about 32 images (≈ 40k px, §5) |
| What readers get | desktop JPEG q90; phone web q70 (800 w) / q70s (500 w); the app was not measured | uploaded at the highest of q95 … q80 that fits |
| Lettering guide | 12–30 px at 800; ≥ 200 px between panels | as on scutoons.com (below); warn < 20 px |
| Series thumbnails | square 1080² < 500 KB, vertical 1080 × 1920 < 700 KB (both required) | the season cover's square / portrait formats |
| Episode thumbnail | 202 × 142 < 500 KB, file name letters and digits | the chapter's `thumbnail` (already 202 × 142) |
| Texts | series title < 50 characters; summary ≤ 500 (2022 advice) **[test]**; episode title, note: undocumented **[test]** | toon title; season text; "Denial (1)" (§5) |

Sources: notice #3320 (2024-11-20, a contest FAQ, the newest official statement of the image limits,
https://www.webtoons.com/en/notice/detail?noticeNo=3320) and the Before-you-publish checklist (2022-05-12,
https://webtoons-static.pstatic.net/creator101/en/pdf/Before-you-publish-Checklist-Updated.pdf); the thumbnail guide
(2026-09-19, https://webtooncanvas.zendesk.com/hc/en-us/articles/32913712749588); the title guidelines (2026-07-08,
https://webtooncanvas.zendesk.com/hc/en-us/articles/35572676527124). q90 / q70 / q70s measured 2026-10-20 in WEBTOON's
viewers. No current Help Center article states the image limits and the dashboard was rebuilt 2026-04-21, so
**[test]** marks what the owner's test draft verifies (§6).

**Why these targets:**
- **Exactly 800 wide, cut by us.** WEBTOON rescales anything else, and its auto-slicer cuts at fixed heights, through
  speech bubbles. That also hurts the AI translation, which re-letters the bubbles it finds (§7).
- **One slice is not one image.** A standard slice is 390 × 625 (`DEFAULT_SLICE`, content.ts:194). At 800 wide that is
  1282.05 px, 2 px over the limit, and boxes cross slice edges (TEXT-ACROSS-SLICES.md §1). So the export renders one
  continuous column and cuts it.
- **Rendered at 400 CSS px × DPR 2**, not 390 × 2.05 and not 1600 then downscaled. 400 × 2 is the only integer mapping:
  2 px borders and the 2.5 px outline (dialog.svelte) stay crisp, and cut lines fall on whole device pixels. Text sizes
  are `cqw` of the slice (dialog.svelte:306), so lettering keeps its proportions. The root font stays 14 px as on a phone
  (shared/tokens/base.css:67-70, `max-width: 480px`), so the one rem inside a slice, the tail's corner inset
  (dialog-tail.ts:28), looks as on a phone. The art is 3x (1170 px) for nearly everything, so Chrome downsamples it
  ×0.684. Rendering at 1600 would cost 4× the pixels, need an in-page resampler and upscale the 1170 art.
- **Lettering reads as on scutoons.com.** A phone shows the 800 image at device width, the same scale as our 390 reader
  column. What is new is JPEG q70 on phones: the build looks at it once on a phone (§4), the test draft again (§6).
- **JPEG, not PNG:** WEBTOON turns PNG into 256 colours; the JPEG it re-encodes stays close to ours.

## 2. What gets baked (the content rules)
**Principle:** the export shows what a new guest sees after scrolling once through the whole chapter: nothing taken
yet, every reveal in its end state, nothing that needs a tap. One rule set, no per-object export flags.

| Thing | In the export | catpope today |
|---|---|---|
| Text boxes | drawn by the reader, per language, with the reader's fallback (`localeChain` / `resolveText`, neutral.ts:172-187) | 128 / 80 / 127 / 80 / ~60 boxes in ch1–5 |
| Boxes revealed by `showTextBox` | drawn in their end state (no box hides itself again) | ch1 s30; ch2 scene-14 (2), scene-17-24 (2) |
| Placed items (none / pickup / events) | drawn at rest as plain art, on vertical versions too (§8); the tap is lost | ch1 2, ch2 8 (6 on the strips), ch3 6, ch4 1, ch5 2 |
| VN lines (`showMessage`), item events, click / sound areas | dropped; the preflight quotes each dropped line (§12 Q4) | ch2 swim pants (a tap event), ch3 scene-25 s0 "No photos, please!" |
| Video | the slice image (its poster) at its best variant | ch1 key visual (3x poster); ch2 key visual (1x only → upscaled 2.05×, its own issue); ch0 is only a music video (§12 Q6) |
| Audio, lyrics notes' animation, FX | dropped (CANVAS has no music, GIF or animation); lyrics boxes are text and stay | ch1 11 slices, ch2 26 |
| Sensitive slices | shown clear (WEBTOON bans blur, box and mosaic censoring); listed for the rating | none |
| `{…}` expressions, choices, jumps inside a chapter | refused (an error): a static episode has one reader and one path | none in catpope; toptoy ch1 is out of scope |
| Horizontal groups | their vertical version (§8); without one, the frames stacked + a warning | 8 groups, 68 frames, 17 boxes, 6 items |
| Translation gaps | a box without the language's text falls back as in the reader; validate's warnings for that language show in the preflight | ch1: 6 one-language boxes |

**ch1's one-language boxes** (v731): `s1c1-4-1`, a German-only heading under `s1c1-5-1` (the same heading in both
languages), is a leftover that English readers of scutoons.com see today; "Snarch" and the sign "Cat Pope" are German
only, RINGEDING- / EDINGEDING! / THRILL THRILL English only. validate warns on all six (none accepted). A scutoons.com
bug with its own issue: the leftover goes, the five get their second language, so a real gap stands out later.

**Which version:** the published one, and a language only where the season lists the chapter as published
(`Season.sequence[].status`), so WEBTOON is never ahead of scutoons.com. `--draft` makes test kits from the newest
draft. The #68 they / them drafts (ch1 v731, ch2 v727, season v730) are published (#68) before the first en kit, or
WEBTOON gets the old pronouns.

## 3. The export view (one renderer)
Everything on a slice is DOM and CSS: collage pieces with blend modes and masks, items with filters, text boxes with
`cqw` sizes, a fit pass, SVG outlines and tails, two self-hosted fonts. Any second renderer (PHP, a canvas re-draw,
`foreignObject`) drifts from the reader, so the reader renders the export. Not on the viewer's chapter page (bound to
the session, VN, inventory, audio, the next bar, tracking and auto-save), not in the creator (login, app shell, and its
canvas is not the reader's `Slice`), not on a page the tool builds (a build per run, the checkout instead of what
readers get): **a shared column component and a thin, bare viewer route**, so the deployed reader draws it, no auth.

- **`shared/reader/src/export/webtoon-column.svelte`** (≤ 300 lines): the groups in order as a plain column of the shared
  `Slice`, like the Vertical renderer. **`export/measure.ts`** next to it reads the geometry from the DOM it draws.
- **`viewer/src/routes/(bare)/export/webtoon/+page.svelte`** (~60 lines): resolves media with `toonUrl` (cdn.ts:13) and
  `pickQualityVariant(…, 'high')` (media-quality.ts:48) and exposes `window.__webtoon = { version, load }`. An internal
  tool page: noindex, linked from nowhere, not a printed URL. Like `/account` it shadows a toon id "export".
- **The root layout's bare branch** (about 20 lines in two files): `+layout.ts` fetches the toons on every route and
  throws 503 when that fails (:18-31); for a route id under `/(bare)` it returns `{ toons: [] }`. `+layout.svelte` skips
  `afterNavigate` (:72), both `onMount`s (:77 creator link, :79 PWA, membership, inventory, saved chapters) and the shell
  markup (:160-189: Header, ItemsHost, HomeHost, prompts) for those routes and renders the children only.
- **Input:** the tool fetches the neutral documents with a token (§4) and injects one language's resolved chapter plus
  the placed items' icons from the catalog. The page never calls the API, and no token enters the browser.

What export mode switches, mostly for free, because the shared `Slice` without the app's callbacks is static:
- every slice and collage piece `eager`, the `high` variant; collage fill masks (CSS masks, no load event) preloaded;
- items without `onitemtap` are a plain `<img>` at rest (slice-item.svelte:129), so no tap hint;
- no `dialogHidden`, so every box is in its end state; no `expressions`; no click / sound areas; no blur;
- a video slice is passed without `video`, so its image (the poster) is drawn at the slice's size;
- no header, VN, next bar, fx layer, reader effects, audio or tracking (not mounted); animations off
  (`prefers-reduced-motion` emulated + `animation: none`; the check asserts `document.getAnimations()` is empty);
- a horizontal group: its vertical version through `verticalSlice` (§8), else its frames stacked. Box rects are % of
  each frame and sizes are `cqw`, so stacked frames compose exactly as in the strip.

**Ready and report:** `load()` resolves when the hyphenation probe passes (§4), every font face in use is loaded with
the real text (`document.fonts.load`; the faces are split by unicode-range), every image is complete with a natural
width, three animation frames have passed (the fit pass and tails run on rAF / ResizeObserver) and two geometry readings
match. It does not decode every image: ch1's 95 slices at 1170 × 1875 would hold about 0.8 GB, so decoding happens per
piece (§4). It returns the column height, each slice's top / bottom / image / natural width, and each overlay's rect:
the union of every `.slice-layers > *` and its descendants (tails, notes, outlines and overflowing `.line`s reach
outside the box), padded by max(6 px, 0.25 em). A free tail is clipped at its slice (dialog.svelte:252-255) but
`getBoundingClientRect` ignores the clip, so its rects are clamped to their slice. Plus overflow (a fixed box still
overflowing at fit 0.8, the test of dialog.svelte) and lettering under 10 CSS px. The fonts each box really used come
from the tool over CDP (`CSS.getPlatformFontsForNode`, check-fonts.mjs:216).

## 4. The exporter: render, cut, encode (`tools/webtoon-export`)
**Raw CDP, no dependencies:** Node 22's WebSocket drives the installed Chrome or Edge, like the checks
(creator-reader-check.mjs:68-71, chrome-profile.mjs). TypeScript runs with `--experimental-transform-types`
(package.json:28), so it imports `@scutoons/content` directly. playwright-core is not used: it launches Chromium with
`--disable-component-update` by default, which makes it ignore the hyphenation data (step 4), and the MCP bundle could
not carry it later.

1. **Fetch** with an `sct_` token (`SCUTOONS_TOKEN`; a Bearer token is a session, CurrentSession.php:21): `GET
   /api/entities/chapter:<toon>:<n>` (published, or `?draft=1`; creator role, EntitiesController.php:71-84), the season,
   the toon entry and the catalog. `X-Content-Version` (:82) is recorded, so a kit names exactly what it shows. A scene
   (`ref`) group is an error for now: `resolveChapter` leaves refs to the library (neutral.ts:295-297), catpope has none.
   `--input <file>` reads a neutral chapter from disk instead (fixtures).
2. **Resolve** each language with `resolveChapter` (neutral.ts:298), the function `NeutralDoc.php` mirrors with golden
   fixtures, so texts are exactly what readers get. The preflight's data checks run on the neutral chapter (§9).
3. **Launch** `--headless=new --hide-scrollbars --force-color-profile=srgb` with a temp profile, viewport 400 × 640
   CSS at DPR 2, `mobile: false` (no text autosizing). Default page: `https://scutoons.com/export/webtoon`; `--viewer`
   points at a dev server. A page whose `__webtoon.version` differs from the tool's is refused.
4. **Hyphenation (German):** text boxes use `hyphens: auto` with `lang` (dialog.svelte:309). Chrome downloads its
   dictionaries into the profile; an empty temp profile hyphenates nothing, not even after 6 s. The tool copies the
   launched browser's own `hyphen-data` (`%LOCALAPPDATA%\Google\Chrome\User Data\hyphen-data`, Edge's under
   `Microsoft\Edge`) into the temp profile; de (umlaut words, `de` and `de-DE`), en and fr then hyphenate from the first
   layout. Never `--disable-component-update`: the copied data is ignored then. The page probes each language (a long
   word in a narrow `lang` box must wrap with a hyphen). No hyphenation = a hard error, never a silent bake.
5. **Measure, then cut once** (CSS px, deterministic): every language the chapter is published in is loaded and
   measured (no capture). Box positions don't depend on the language, so one cut set is computed on the union of all
   languages' padded overlays, and the de and en kits get the same cuts and image numbers. A cut line is an integer y
   outside every overlay; pieces are 160–640 CSS (320–1280 px). Dynamic programming picks the fewest pieces, then the
   cheapest lines (a slice seam costs 0, a line through art costs more the closer it is to a box). A cut through art is
   invisible on WEBTOON, which shows images edge to edge; a cut through a box never happens: an overlay cluster taller
   than a piece is an error naming the boxes. No pixel gutter detection.
6. **Capture** each piece per asked language: scroll to its top, read `scrollY` (clamped at the document's end) and
   assert the piece lies inside the viewport (a clip outside it returns plain white, no error), two frames, decode the
   piece's images, then `Page.captureScreenshot { format: 'jpeg', quality, clip }`. The clip is in page coordinates
   (creator-chapter-check.mjs:327-328 adds the scroll). The viewport always holds a whole piece, so nothing is stitched.
7. **Quality:** per episode the highest of 95 · 92 · 90 · 88 · 85 · 82 · 80 at which every image is ≤ 1.9 MB and the
   episode ≤ 19 MB, re-captured at the next step (the page is static); still over at 80 → one episode more (§5). Only
   the thumbnails are encoded in the page (WebP → JPEG, `OffscreenCanvas.convertToBlob`, the precedent:
   map-tiles.mjs:61-68). The same input and Chrome give the same bytes; the kit records the Chrome version.

Measured 2026-10-20 on Chrome 155 (raw CDP, probe scripts): steps 4 and 6 as written; a page-relative 400 × 640 clip
returns an exact 800 × 1280 JPEG.

The CLI: `npm run webtoon -- catpope 1-2 --lang en,de [--draft] [--viewer URL] [--out DIR]`. The output goes outside
the checkout by default.

## 5. Episodes
A chapter does not fit one episode (ch1 ≈ 122k px at 800 wide, ch2 ≈ 112k + its vertical versions, ch3 ≈ 122k + 2).
CANVAS episodes run about 3–41k px; Originals 75–91k.

- **Size:** about 40k px per episode (≈ 32 images, 10–16 MB), so n = round(H / 40k), plus one when a budget forces it.
  Season 1 becomes about 15 episodes (ch1 3, ch2 3–4, ch3 3–4, ch4 2–3, ch5 2).
- **Breaks** fall on slice seams that no box crosses in any of the chapter's languages, so de and en get the same
  parts. Within ±30 % of the even split the tool prefers a group start, then the seam above a screenplay heading ("8 INT.
  KITCHEN"; the headings stay, owner via #68), then the nearest free seam. ch1 that way: s0–30 · s31–62 · s63–94 (s31
  opens "8 INT. KITCHEN"), 40.2k · 41.0k · 41.0k px.
- **Stable:** the breaks are stored in the kit's `plan.json` as slice image names, written after all languages are
  measured; the next run reuses them, so a published episode never shifts when a box moves. Editing `plan.json` moves a
  break. Nothing goes into the DB.
- **Corrections:** scutoons.com keeps fixing published text (#68). `plan.json` keeps each image's sha256; the upload
  sheet marks the episodes whose images changed since the last kit, and only those are re-uploaded through Edit **[test]**.
- **Titles:** the chapter title plus the part: "Denial (1)" / "Leugnen (1)"; an unsplit chapter keeps its title.
- **Thumbnails:** every part of a chapter uses the chapter's `thumbnail` (content.ts:517, already 202 × 142), saved as
  JPEG. Series thumbnails: the season cover's square and portrait images (catpope's has `lettering: true`, so the art
  itself is text-free), JPEG stepped down from q90 until under 500 / 700 KB (the square file is 548 KB today).
- **Creator's notes** (plain text; links are not clickable there): the first part of a chapter gets its description and
  the season's credits, the last part "End of chapter 1 – next: Anger" and one line naming scutoons.com as the
  interactive version with music, a map and things to collect; an optional content-warning line (ch4, §6). The tool's
  own en / de template; the owner edits it once. Only existing addresses (`scutoons.com`, `scutoons.com/catpope`).
- **End card** (§12 Q7): one image per language that the owner makes, a local kit input (`--end-card`), never SCUTOONS
  media; it goes last in a chapter's last episode. No URL on story slices, no QR code: WEBTOON's community policy bans
  series made mainly to drive traffic to external sites.

## 6. The upload kit and the fastest import
**There is no faster legitimate path than a person with a perfect kit.** CANVAS has no API, no bulk import and no app
upload; publishing is the desktop website only, one episode form at a time (multi-file drag and drop, Save Draft,
Preview PC / Mobile, Schedule for later). The Clip Studio button imports CSP projects only.

**Why no bot:** the WEBTOON ACCOUNT Terms §8 (effective 2026-10-12, https://account.webtoon.com/en/terms/terms-of-use)
forbid automated means such as bots and scripts without written authorization; the Service Terms §5(d) and
§6(b)(iii)–(v) bar circumvention and other automated means; the CANVAS Terms §6(d)(ii) forbid letting anyone else use
the account. Claude driving a browser is automated means too. At about an hour per language per season, a bot would
save little and risk the account that holds the series.

**The kit** (ASCII names, zero-padded, so name order is reading order):
```
<out>/catpope/plan.json            language-free: breaks, episode numbers, source versions, Chrome version, sha256s
<out>/catpope/en/index.html        the upload sheet: series fields, then one card per episode
                 series/square.jpg  series/vertical.jpg
                 thumbs/e001.jpg …  (outside the episode folders, so "select all" in a folder takes only pages)
                 e001/e001p001.jpg … e001p032.jpg
<out>/catpope/de/ …
```
The upload sheet is a static page (rem sizes, no dependencies). Each episode card: title and note with copy buttons,
the scheduled date in order, the folder, image count, MB and quality, the preflight warnings, "changed since the last
kit", and the episode as a phone-width strip with ticks at the cuts.

**Once per language (about 15 min, desktop Chrome):** sign in with WEBTOON ACCOUNT, fill in the content-rating
questionnaire (required before a series can even be created; five categories incl. substances, plus key themes), then
Publish: title, 1–2 categories, summary, both thumbnails, Create Series; keywords if offered (≤ 15, locked 30 days).
Expected Teen 13+: mild violence in ch3, "WTF", the "Sin Tonic" ad (alcohol), ch5's `lsd` pickup (a substance), ch4's
depression (WEBTOON advises a content warning at the episode's start). The preflight lists the inputs per chapter (§9).

**Per episode (2–3 min, an estimate):** Add Episode · paste the title · drag in the folder's files · the thumbnail ·
paste the note · Schedule for later with the sheet's date · Preview Mobile for a chapter's first part. Season 1 is
about an hour per language. Scheduled episodes must be in chronological order; the clock is the device's time zone.
For the launch, request a Content Review (WEBTOON: 2–3 business days), so upload about a week early.

**Test draft (owner, ~45 min, ch1 kit, in the real en series, Save Draft, never published) – [test]:**
1. Where a series' source language is set (the form, or the site / footer language) – before Create Series. Right after
   Create Series: is a series without episodes listed or searchable (search indexes new series in 20–30 min)?
2. File order when the files are dragged in reverse order: does the preview start at `p001`?
3. One exactly 800 × 1280 piece passes with no "optimized" pop-up (the checklist says "less than"); the MB counter
   counts per episode.
4. Limits: paste a 70- and a 1,500-character episode title / note and a 600-character summary; is the episode
   thumbnail required?
5. The scheduler's time steps, and the old rule that drafts publish only in the order they were saved.
6. Replace the draft's images through Edit: does the order hold (the correction path, §5)?
7. Preview Mobile, on a phone if it opens there: lettering at phone quality.
The findings go into the issue; the budgets are tool parameters.

**Ask creatorsupport@webtoon.com now, don't wait for it** (the owner sends, Claude drafts): parallel en and de series of
one comic – fine? Which terms govern a German-source series (the CANVAS Terms still name only the en / es services)? Do
AI-translated views count toward the en series' ad thresholds, and does AI translation cover episodes published before
opting in? Is the chapter-end card acceptable? Would they authorize a browser-assisted upload in writing?

**AI-assisted art:** the CANVAS Terms, community policy and ad revenue terms have no AI clause and no disclosure field
(full-text check 2026-10-20); WEBTOON's contests ban generative AI; the EU AI Act Art. 50(4) asks at most a light
disclosure for evidently artistic or fictional work. ch3's gaps may be generated shots (#61), so §12 Q9.

## 7. Languages
**How WEBTOON does it:** webtoons.com runs one CANVAS platform in seven languages (en, es, fr, de, id, th, zh-hant;
notice 3621, 2026-03-26); one account manages series in all of them. Every series has exactly one source language,
and there is no way to add your own translation to it. Creators publish their own translations as separate series
("… (Español)", "[GER]" on /de/canvas); WEBTOON's translation terms allow translating a series independently.

The only multi-language series is WEBTOON's **AI Translation Program** (Help Center 45569501851924, 2026-08-27): a beta
for English-source series with ≥ 10 episodes and > 2,000 page views in 365 days. It replaces the lettering inside our
images with no preview and may run outside the bubbles; with it on, an episode publishes only after every translation
is done (hours), so episodes are scheduled at least a day ahead. It covers newly uploaded episodes; older ones are
undocumented (asked, §6), and re-saving an episode re-triggers its translation. The translations are jointly owned
with WEBTOON and may not be used outside it; a language switched off is locked 30 days, leaving the program 60.

| Language | Where | Ad revenue | Our texts | Plan |
|---|---|---|---|---|
| en | CANVAS /en/ | yes (+ Super Like) | complete | series 1, at launch |
| de | CANVAS /de/ | not yet ("promised") | complete (source) | series 2, about two weeks later |
| es, fr, id, th, zh-hant | AI translation inside the en series | unknown (asked, §6)¹ | none | from episode 10, decided then |
| ko | Naver 도전만화 (690 px wide, Naver ID) | – | none | not now |
| ja | LINE Manga Indies (geo-blocked outside Japan) | – | none | not now |

¹ The en / es rule is about a series' source language: an es-source series earns, the es view of an en series is not
documented.

- **Money follows the source language:** ad revenue sharing covers en and es sources only (Dashboard FAQ
  48452348349332, 2026-04-22); thresholds per series: > 40k monthly page views and > 1k subscribers; the creator 18+,
  a W-8BEN, PayPal from US$25 (notice 3567, 2026-01-05; one help article still says US$100). So en is the primary
  series; de earns nothing yet but is the source and reaches German readers.
- **Germany:** CANVAS is unavailable to children under 16 residing in Germany (WEBTOON's reader help center, article
  4402125154580, 2026-08-23; found once, not re-checked). Younger German readers stay on scutoons.com.
- **One kit per language:** pixels differ per language (lettering), so each language is its own render, kit, title,
  summary, rating and audience. The cuts and breaks are shared (§4, §5).
- **AI translation:** for es, fr, id, th and zh-hant only, never de (we run our own). Our tightly fitted boxes may
  overflow; we accept that rather than re-letter. Never copy those translations back to scutoons.com.

## 8. Vertical versions of horizontal groups
| Chapter | Group | Layout | Frames (1x) | Boxes | Items |
|---|---|---|---|---|---|
| 2 | scene-12 · 13 · 14 · 15 | pinned strip | 7 · 12 · 12 · 13 × 390 × 693 (2x at most) | 0 · 1 · 2 · 2 | 2 · 3 · 1 · 0 |
| 3 | scene-28-duel · scene-32-hall | endless | 6 × 390 × 844 each (3x) | 1 · 3 | – |
| 4 | scene-36-cat | endless | 6 × 390 × 844 | 2 | – |
| 5 | scene-38-garden | endless | 6 × 390 × 844 | 6 (a list read in order) | – |

**The art** (owner 2026-10-20: "one vertical image of the same scene, it is also longer between 1:4 to 1:7"): one tall
picture per horizontal group, the whole scene recomposed top to bottom, without text and without the pickups. At the
390 basis that is 390 × 1560–2730 (1x); delivered 1170 wide (3x: 4680–8190 px tall, within WebP's 16,383). At 800 wide
it is 3200–5600 px, which the cut (§4) splits into 3–5 WEBTOON images like any other part of the column.

**The model:** an optional `Group.vertical`, next to `continues` / `end` (content.ts:469-490): one picture, and
placements that put the frames' own boxes and items on it. Nothing is copied.
```ts
vertical?: VerticalVersion;  // Group: export only – readers never draw it
// VerticalVersion (@additionalProperties false): image, width, height, imageVariants?, place?: VerticalPlace[]
// VerticalPlace: { of: string; x: number; y: number }  // a frame box (Dialog.id) or placed item (SliceItem.id)
                                                       // of this group; its anchor point in % of the picture
```
- **One picture, not a list,** because that is what the art is (keep the configuration small). If a scene ever needs
  two, the field becomes a list with a migration over a handful of groups.
- **`verticalSlice(group)`** in `@scutoons/content` (pure, tested; the export column and the creator canvas use it)
  turns a resolved group into the one slice to draw: the picture plus every frame box and item, each keeping its text,
  look, width, rotation and anchor; its height is rescaled to keep its px (the picture is taller than a frame); a free
  tail's tip moves with its box. Width % and `cqw` sizes carry over unchanged, because frames and picture share the 390
  basis. Its position is its placement, else the **automatic spot**: frame i of n at y % → y = (i + y / 100) / n of the
  picture, x as in its frame – the strip read left to right becomes the picture read top to bottom. So nothing is
  dropped and a box added to a frame later shows up by itself; the author only moves what sits wrong.
- **Why placements, not copies:** each text exists once in every language, so a translation or a #68-style fix
  reaches both versions with no linking field and no change to translation checks, resolution (TS `{ ...group }`,
  neutral.ts:265; PHP `clone`, NeutralDoc.php:170-177) or `TextCheck.php`. No item is placed twice, so placement counts
  (ItemPlacements.php), pickup refs and event ids stay as they are, and "bake everything" holds for ch2's six strip
  items. Not a second group (the reader would show it; reach, chains, the next bar and `Checkpoint.group` indexes would
  need a hide flag), not `renderer.params` (closed, `@additionalProperties false`, content.ts:439; validate.ts:931).
  Frame pieces are art: a vertical picture includes them (catpope has none).
- **Where the field flows:** the types (content.ts; `GroupDoc` inherits it, neutral.ts:115); the generated schemas
  and the server copy (`npm run schema -w @scutoons/content`; a test that `vertical` stays optional); `checkVertical`
  next to `checkRendererParams` (validate.ts:902: horizontal groups only, the picture with its width / height like a
  slice image, each `of` names one box or item of the group's frames and is placed once, a stale `of` warns); `ChapterFiles` walks with
  `['vertical']` skipped (the call at ChapterFiles.php:70; offline downloads don't fetch export-only images); the
  library's chapter answer drops it (Library.php:344 ff.); the MCP rules text (the MCP validates with the bundled
  validator anyway). AssetUsages walks the whole document, so the pictures count as used. No migration: a new optional
  field reshapes no stored data.
- **Readers keep the strip and the band.** A reader option ("reduced motion → vertical") would change what progress
  positions mean and where the frames' unlock events and reveals fire. It can come later without a migration.

**Asking for them:** Wanted images' stored media requests already do it: `view: 'media'`, `toon`, `path`, `size`,
`note` (characters.ts:392-417), listed under Toon media with copy as text / CSV (wanted-images.svelte:56-57), closed by
themselves once a file exists at `path` (characters-editor.svelte:113-127). One request per horizontal group (the 8,
written as one draft through the MCP): path = its picture's 1x name in the chapter's folder
(`s1c2-scene-12-vertical.webp`; `.2x` / `.3x` beside it), size "1170 wide, 1:4 … 1:7, one image", note = the brief
("the whole scene recomposed top to bottom, without text and without the pickups – they are placed on top; leave room
between panels, ≥ 200 px at 800"). A later horizontal group: its empty "Vertical version" slot and the export preflight
name it.

**Creator UI** (a design draft on the real page first, phone + desktop):
```
ChapterTree                      GroupPanel (horizontal groups only)
⋮ scene-12  7                    Name [scene-12]  Layout [Horizontal ▾]  Continues ☑  [+ End]
  1 … 7  (the frames)            Vertical version  [ picture ] (×)   1x · 2x · 3x
  Vertical                                         └ the dashed slot while empty
```
1. **The slot:** a horizontal group's panel gets "Vertical version", the same `MediaVariantsSlot kind="image"` a
   slice's image uses (slice-media.svelte:219; one slot with 1x / 2x / 3x chips, DESIGN "Media with quality levels =
   one slot"). Empty = the dashed slot; picking writes `vertical.image` / `imageVariants` and reads the size (like
   `fillSliceSize`); the round × clears it (undo brings it back, the file stays an unused asset). No import step, no
   list, no "+" button.
2. **The tree:** with a picture set, one row "Vertical" after the group's frames; selecting it shows the picture in the
   canvas. `slicePath` (chapter-editor.svelte:508) gains this `['vertical']` target.
3. **The canvas** (slice-canvas.svelte, today `doc.resolvedSlice(slicePath)` at :94): for that row it draws
   `verticalSlice(doc.resolvedGroup(groupPath, locale))` (doc.svelte.ts:438) – the picture with the frames' boxes and
   items at their places or automatic spots. The only tool is moving: a drag writes the placement
   (`lib/editor/vertical.ts`, pure, tested). Text and style are edited on the frame.
4. Group copy / paste re-ids boxes and items (slice-clip.ts:104-107) and remaps `of` with them.
5. Logic in `lib/editor/*.ts`: slice-canvas (955 lines) doesn't grow; group-panel (111) takes the slot.
6. No counter, no hint line (DESIGN: the explanations are the fields' tooltips), no Problems entry, no Reader-panel
   mode (DOCKING: two messages only): the canvas is the preview, Wanted images and the preflight are the lists.

## 9. Preflight
Data checks run on the neutral chapter before rendering (in the tool; they move to `@scutoons/content` only if the
creator ever shows them); render checks come from the page's report. Errors stop that chapter; warnings never stop the
export. Everything prints in the CLI and appears on the upload sheet.

| Check | Level |
|---|---|
| Choices, jumps inside the chapter, ends other than the next chapter (`chapterFlow` / `chapterExits`, flow.ts:750, 867) | error |
| `{…}` expressions in a text; a scene (`ref`) group | error |
| validate's errors (`validateNeutral`) | error |
| Missing media, a glyph in a fallback font, no hyphenation, no possible cut, a page of another version | error |
| The chapter not published in that language (without `--draft`) | error |
| validate's unaccepted warnings for that language (missing translations, validate.ts:1538-1558) | warning |
| A horizontal group without a vertical version (frames stacked) | warning |
| A slice without 3x art (upscaled: 1x by 2.05×, 2x by 1.026×) | warning |
| Dropped VN lines (quoted), sensitive slices exported clear | warning |
| A fixed box overflowing at fit 0.8; lettering under 20 px at 800 | warning |
| Title or summary over the limit | warning |
| For the rating questionnaire: substance items (`isSubstance`, substances.ts:107), sensitive slices | info |
| Items baked at rest; audio dropped; boxes / items at automatic spots in a vertical version | info |

## 10. Timing and the chapter releases
Chapter 3 releases Sun 2026-11-15 (art freeze Nov 10, deploy freeze Nov 12), ch4 Tue 2026-12-15 and ch5 Fri
2027-01-15 (DECISIONS 2026-10-20); by #64's pattern each has an art freeze about 5 and a deploy freeze about 3 days
before. The owner's art time is the bottleneck in all three months.

- **Before Nov 15: no build work.** Two small steps fit without touching ch3: the WEBTOON ACCOUNT and the support email
  (lead time; Claude drafts it), and ch1's one-language boxes (an agent draft, published with #68's drafts).
- **After #64 closes:** the build (export view, exporter, kit; vertical model, creator UI). The owner orders it against
  #62 (studio) and ch4. The export view changes the viewer's root layout, so it deploys outside a chapter's deploy
  freeze.
- **The launch** (§12 Q2) starts with ch1's three episodes (WEBTOON's advice: launch with 3; the subscribe pop-up
  follows the third), then one episode every Sunday, never ahead of scutoons.com; ch2's first part follows a week later
  and needs the four strips' vertical versions by then. A Sunday 2026-12-13 launch puts the upload, the Content Review
  and the ch2 vertical versions into ch4's last two weeks. A Sunday 2027-01-24 launch comes after ch5: the whole season
  is on scutoons.com, all eight vertical versions can be done in one go, the upload falls just after ch5's release,
  and the weekly queue (15 episodes, to about mid-April) can't run dry. Episode 10, the earliest AI-translation date,
  lands about 7 weeks after the launch. The de series follows about two weeks after en.

## 11. Left out, and why
- **A WEBTOON panel in the creator, WEBTOON metadata in the DB** (cuts, titles, notes, upload state): one-time entries;
  `plan.json`, the upload sheet and WEBTOON's dashboard hold them. Less UI is better UI.
- **An MCP tool `export_webtoon` and a creator "Export for WEBTOON" entry:** later, when members other than the owner
  export. The core has no dependencies, so rolldown can bundle it (tools/mcp/scripts/build.mjs); it would need the
  MCP's first process spawn, a Chrome path on macOS / Linux and a job for runs longer than a tool call.
- **Rendering in PHP, in a creator canvas or via `foreignObject`:** second renderers that drift. **Rendering at 1600,
  pixel gutter detection, 16-px alignment, encoding in the page:** no visible gain; Chrome's capture returns the JPEG.
- **Copied or linked texts on vertical versions (`textOf`), their own captions or pieces, pickups painted into the
  art:** placements cover boxes and items with one text each; a caption or pieces can come later as optional fields.
- **Derived "vertical version" requests (a flow index v7) and a Problems entry:** 8 stored requests, the group's empty
  slot and the preflight say it; derive them if horizontal groups become common.
- **Several pictures per vertical version, a slice list or import for it:** the art is one picture per scene (owner).
- **Scene (`ref`) groups in the export:** catpope has none; one fetch per ref when a chapter needs it.
- **`--break` / `--episode` flags, zip files, q70 copies in every kit:** `plan.json` is the override, a run re-renders
  everything, the uploader multi-selects files, and one look on a phone during the build is enough.
- **A per-episode crop as the episode thumbnail:** the chapter's thumbnail is fine, and WEBTOON lets it change later.
- **Baking VN messages as a message box:** a new look to render for one line in the season (§12 Q4).
- **Our own es translation, ko / ja, the Clip Studio route, upload automation:** cost or risk without a matching gain now.
- **Re-lettering for the AI program's overflow:** the risk is accepted.

## 12. Owner questions (with recommendations)
1. **Languages:** English at launch, German as its own series about two weeks later, no own Spanish, ko / ja not now?
   A separate de series splits the audience (the thresholds are per series), but WEBTOON's AI could only make German
   from our English translation of a German original, de earns nothing either way, and under-16s in Germany can't read
   CANVAS. Rec.: yes; en earns money and reaches the most readers.
2. **Launch date** (the milestone's due date; then one episode every Sunday, never ahead of scutoons.com): (a) Sun
   2026-12-13, if by about Dec 6 the kit has passed the test draft, the ch2 vertical versions are in and the export view
   is deployed before ch4's freeze; or (b) Sun 2027-01-24, after ch5, with the whole season ready (§10). Rec.: (b);
   chapter releases on scutoons.com come first, and a full queue never forces a hiatus.
3. **Version:** export only what is published on scutoons.com, test kits from drafts? Rec.: yes; #68 publishes the
   they / them drafts first.
4. **VN lines:** drop them in the export (the preflight quotes them)? catpope's one story line is ch3's "No photos,
   please!" on a slice whose furniture still moves (DECISIONS 2026-10-20). Rec.: drop; whether that line becomes a
   normal text box (then on scutoons.com too) is asked with ch3's kit.
5. **Episodes:** about 40k px (≈ 32 images, ch1 = 3 parts), titled "Denial (1)", every part with the chapter's
   thumbnail? Rec.: yes; confirm the cuts on the first export.
6. **Chapter 0** (only the music video): skip it, and put the video in the creator profile's links and the first note?
   Rec.: skip; a one-image episode pointing elsewhere is the external-traffic pattern WEBTOON bans.
7. **End card:** one image per language at each chapter's last episode ("made for scutoons.com – with music, a map and
   things to collect"), made by you, kept as a local kit input? Rec.: yes, mentioned in the Content Review request. If
   a funder (SKE / austro mechana, Westbahnstudios, Künstlerhof Frohnau) requires a logo on every publication, it goes
   there – please check.
8. **Vertical versions:** the art is answered – "one vertical image of the same scene, it is also longer between 1:4 to
   1:7" (owner 2026-10-20, §8). Still open: export only, so scutoons.com readers keep strips and bands? Rec.: export
   only.
9. **AI-assisted art** (ch3's gap shots, #61; anything else?): if any, one disclosure line in the creator profile and
   no WEBTOON contests? Rec.: yes; WEBTOON asks nothing, the line costs nothing.
10. **Series thumbnails:** the season cover's art without lettering (WEBTOON advises no text; one set for both series),
    or with the cover's lettering per language? Rec.: without.
