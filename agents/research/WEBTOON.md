Status 2026-10-20: plan, nothing built. Vertical versions redesigned with the owner (§8: a second layout of a horizontal group with its own slice list, every text box, item and event mapped onto both, readers by a condition, the export asks for a missing one); the standard slice is 390 × 624 since #78 (= one WEBTOON image); the launch is in February 2027 after the Top Toy workshop (§10, Q2 answered); waiting for the owner's agreement and the other answers (§12, issue #77); then the milestone "Webtoon.com Publishing" (due = the launch) and its issues. Not on chapter 3's critical path (#64): before 2026-11-15 only two small steps (§10); the vertical versions come after #44's strip editor.

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
English first. Horizontal groups can have a vertical version, and the export needs it: a second layout with its own
slices (one tall picture of the same scene, 1:4 … 1:7) onto which the frames' own text boxes, items and events are
mapped, never copied. The export stops at a group without one and asks the creator to add it; readers get it when its
condition holds. The art is asked for in Wanted images; the creator flips a group between its two layouts.

## 1. Resolutions: what WEBTOON takes and what we export
| | WEBTOON CANVAS | Our export |
|---|---|---|
| Image width | ≤ 800 px; the desktop viewer shows every image at 800 CSS px (narrower ones are upscaled) | exactly 800 **[test]** |
| Image height | ≤ 1280 px; longer images are auto-sliced at fixed heights (through bubbles too) | a standard slice = exactly 1280 (#78); else 320–1280, cut by us between boxes (§4) |
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
- **One standard slice is one image.** Since #78 the standard slice is 390 × 624 (`DEFAULT_SLICE`; it was 390 × 625,
  1282 px at 800, 2 px over the limit): its 3x 1170 × 1872 is exactly 800 × 1280. Still the export renders one continuous
  column and cuts it: boxes cross slice edges (TEXT-ACROSS-SLICES.md §1), and key visuals, group ends and the vertical
  pictures (1:4 … 1:7) have other heights. Where no box crosses a seam, the pieces are the slices.
- **Rendered at 400 CSS px × DPR 2**, not 390 × 2.05 and not 1600 then downscaled. 400 × 2 is the only integer mapping:
  2 px borders and the 2.5 px outline (dialog.svelte) stay crisp, and cut lines fall on whole device pixels. Text sizes
  are `cqw` of the slice (dialog.svelte:306), so lettering keeps its proportions. The root font stays 14 px as on a phone
  (shared/tokens/base.css:67-70, `max-width: 480px`), so the one rem inside a slice, the tail's corner inset
  (dialog-tail.ts:28), looks as on a phone. The art is 3x (1170 px) for nearly everything, so Chrome downsamples it
  ×0.684. The 800 × 1280 images are an export from our masters or the 3x (owner 2026-10-20: "we have the webtoon
  800x1280 as an export from our masters or 3x"): the 3x by default, `--masters <dir>` swaps in a slice's 2340-wide
  master where one exists (ASSETS.md "Slice resolutions"). Rendering at 1600 would cost 4× the pixels, need an in-page resampler and upscale the 1170 art.
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
| Horizontal groups | always their vertical version (§8); a group without one stops the chapter's export and is named with a link to add it (§9) – never the frames stacked | 8 groups, 68 frames, 17 boxes, 6 items |
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
- a horizontal group: its vertical version through `verticalSlices` (§8), whatever its `if`; a group without one never
  reaches the page (the preflight stops the chapter, §9).

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
   than a piece is an error naming the boxes. No pixel gutter detection. A standard slice is exactly one 640-CSS piece
   (#78), so wherever no box crosses a seam the pieces are the slices.
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
A chapter does not fit one episode (at 800 wide: ch1 ≈ 122k px, ch2 ≈ 112k + 4 vertical pictures of 3.2–5.6k ≈ 125–134k,
ch3 ≈ 122k + 2 ≈ 128–133k, ch4 ≈ 83k + 1, ch5 ≈ 54k + 1).
CANVAS episodes run about 3–41k px; Originals 75–91k.

- **Size:** about 40k px per episode, at most 45k (≈ 32 images, 10–16 MB), so n = ceil(H / 45k), plus one when a
  budget forces it. Season 1 becomes about 13 episodes (ch1 3, ch2 3, ch3 3, ch4 2, ch5 2).
- **Breaks** fall on slice seams that no box crosses in any of the chapter's languages, so de and en get the same
  parts. Within ±30 % of the even split the tool prefers a group start, then the seam above a screenplay heading ("8 INT.
  KITCHEN"; the headings stay, owner via #68), then the nearest free seam. ch1 that way: s0–30 · s31–62 · s63–94 (s31
  opens "8 INT. KITCHEN"), 40.2k · 41.0k · 41.0k px.
- **Stable:** the breaks are stored in the kit's `plan.json` as slice image names, written after all languages are
  measured; the next run reuses them, so a published episode never shifts when a box moves. Editing `plan.json` moves a
  break. A break never falls inside a horizontal group; a stored break whose slice is gone (a converted or replaced
  picture) is planned again with a warning. Nothing goes into the DB.
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

## 8. Vertical versions: a second layout of a horizontal group
Owner 2026-10-20, after (1) at the top of this doc: (2) "one vertical image of the same scene, it is also longer
between 1:4 to 1:7" · (3) "creator, it can become a slice list as well. we only have to map the items and events
accordingly ... it is basically like a flip vertical to horizontal for a group. we can use this as a feature also.
depending on some conditions the group can show the vertical or the horizontal version" · (4) "for the export of
horizontal only scenes, we will need to ask the creator to add the vertical." · (5) "any text box, item, event def gets
onto both."

| Chapter | Group | Layout | Frames (1x) | Boxes | Items | Events, sound (frame) |
|---|---|---|---|---|---|---|
| 2 | scene-12 | pinned strip | 7 × 390 × 693 | 0 | 2 pickups | unlock (0), music (0) |
| 2 | scene-13 | pinned strip | 12 × 390 × 693 | 1 | 2 pickups, 1 tap item | unlock (0) |
| 2 | scene-14 | pinned strip | 12 × 390 × 693 | 2 | 1 pickup | unlock (0), 2 reveals (4, 7) |
| 2 | scene-15 | pinned strip | 13 × 390 × 693 | 2 | – | unlock (0) |
| 3 | scene-28-duel · scene-32-hall | endless | 6 × 390 × 844 each | 1 · 3 | – | – |
| 4 | scene-36-cat | endless | 6 × 390 × 844 | 2 | – | – |
| 5 | scene-38-garden | endless | 6 × 390 × 844 | 6 (a list read in order) | – | – |

Frames = a horizontal group's own slices (ch2's 2x at most, the others 3x). Unlock = `unlockLocation` (map "open"),
reveal = `showTextBox`, both whole-frame intersect events. Every box, item and event of chapters 1–4 has an id; none of
the 8 groups has a free tail, a sample, a piece, a sensitive frame or a `layer` value (checked 2026-10-20). Code lines
in this section: main after #44 steps 1–3 (098e5c98).

**The art** (owner (2)): one tall picture per group, the whole scene recomposed top to bottom, without text and without
the placed items (they go on top); 390 × 1560–2730 at 1x, delivered 1170 wide (4680–8190 px, within WebP's 16,383); at 800 wide 3200–5600 px
= 3–5 WEBTOON images after the cut (§4). In the creator it is a slice list (owner (3)): one slice (the picture whole,
the default) or its tiles once #44 step 4 cuts tall pictures (TEXT-ACROSS-SLICES.md §5); at 390 × 1872 (1:4.8) or 390 × 2496
(1:6.4) the picture is exactly 3 or 4 standard slices (#78), i.e. 3 or 4 WEBTOON images.

**The model:** `Group.vertical`, next to `continues` / `end` (content.ts:475-495). It holds pictures and positions only;
every text box, item and event stays defined once, on its frame, and is mapped onto the vertical layout.
```ts
vertical?: VerticalVersion; // Group, horizontal / endless only: the second layout; the WEBTOON export always draws it
// VerticalVersion (@additionalProperties false): { slices: VerticalSlice[]; if?: VNEventCondition }
//   if (comes with the reader step): readers get this layout when it holds; none = export only
// VerticalSlice (closed): image, width, height, imageVariants?, shot?, place?: VerticalPlace[]
// VerticalPlace (closed): { of: string; x: number; y: number }
//   of = a text box id, a placement id or any event id of an area run of this group's frames, placed at most once;
//   x / y = its anchor point in % of this vertical slice
```
- **The automatic spot, else a placement.** Frame i of n owns the share i/n … (i+1)/n of the whole vertical version's
  height (its slices stacked): the strip read left to right becomes the picture read top to bottom. An unplaced element
  sits in its share: an anchor at y % of frame i lands at (i + y / 100) / n of the height; x, width, `cqw` size,
  rotation and anchor as on the frame (both share the 390 basis); its height kept in px, a free tail's tip at its
  distance. The author moves only what sits wrong: the move writes a placement into the vertical slice that holds it,
  so reordering or splitting the vertical slices carries it along. Inserting or removing a frame changes n: every share
  and unplaced spot moves, placed ones stay (the creator's hairlines show it).
- **"Gets onto both" = one definition.** Added to a frame, an element shows up on the vertical layout at its automatic
  spot. Added on the vertical layout, it goes into the frame whose share is under it, at the matching frame spot, and a
  placement pins it where it was dropped. Text, look, size, settings and events are edited once for both; deleting
  deletes both, with the placement. So each text keeps one locale map, and `event:<id>` history, `resolved(id)`,
  once-events, reveals, pickups, placement counts (ItemPlacements.php:38-41 walks the frames) and reach stay single.
  Copies would double grants and fire once-events twice.
- **Derived, never stored:** a vertical slice is sensitive when a frame whose share it covers is; its layer stack is its
  frames' stacks in frame order, a later frame in front – the order of #44's group-wide Layers list.

**Mapping per kind:** `verticalSlices(group)` in `@scutoons/content` (pure, tested) returns one mapped `Slice` per
vertical slice plus the frames' shares; the reader, the export column and the creator canvas draw what it returns.

| Defined on the frame | On the vertical layout | `of` | Keyed by (the same on both) |
|---|---|---|---|
| Text box | its spot; hidden until its reveal | box id | reveal `<group>\|<id>`, across the group (visual-novel.svelte.ts:38, 306) |
| Placed item (pickup / events / none) | its spot | placement id | `picked:<placement id>` (slice-items.ts:31-32); taps `event:<id>` |
| Area run (click / hover, intersect with a rect) | its spot, size kept; every event of the run with the same rect and stack place | any event id of the run | `event:<id>`; the run `on/<first id>` (intersect.svelte:36) |
| Whole-frame event (no rect) | the frame's share as its rect (on the vertical slice holding its middle) | – | the same |
| Sound in view, music | on the frame's share, with the frame's index | – | – |
| Sound area (a click / hover sample – audio, not an event) | not mapped yet (no id; none in the 8 groups): a validator warning | – | – |
| Group script, label, continues, end | nothing to place; a deferred script starts at the first share | – | `event:<id>` |
| Collage pieces, video | part of the picture | – | – |

- **Why shares.** Whole-slice triggers watch their slice at 0.5 (intersect.svelte:53-64, music.ts:73, samples.ts:66):
  a 390 × 2730 slice never shows half of itself on an 844-px phone, so ch2's 4 unlocks and 2 reveals would never fire;
  and events with the same trigger run as one (triggers.svelte:19-27), so scene-14's frames 0, 4 and 7 would fire
  together – while a run given two rects splits (triggers.svelte:15-17). `of` matches any id of its run.

**The reader** (owner (3): "we can use this as a feature also"):
- **Which layout, decided once.** `group.svelte:46-56` picks the renderer: a horizontal group whose `vertical.if` holds
  gets a new `vertical-version.svelte` (≤ 300 lines) over `verticalSlices(group)`; every other group renders as today.
  `if` is the events' "Only if" (`VNEventCondition`, visual-novel.ts:177; `testCondition`, vn-script.ts:19, with the
  reader's resolver, runner.ts:44) over the names events and texts read today – a device name only if the owner wants
  one (§12 Q8). A page's groups are decided together, once per page view, before they mount (inside the `{#key}`,
  +page.svelte:351): at once when none has an `if`, else after `readerKnown()` (reader-name.svelte.ts:101) and a
  member's save-game sync (world.svelte.ts:175-188; `sync()` then hands out its promise), at most 3 s (`arriveAt`
  waits up to 10 s anyway, +page.svelte:92). So no group changes height after the arrival aligned the page (:111-128);
  a choice on the page or a turned phone counts from the next page view; a flip between page views fires and grants
  nothing twice. The VN script is the renderer's sibling (group.svelte:42-44): the layout never restarts it.
- **Frame units.** One share per frame lies over the column (no pointer events) and carries the frame's page index
  (`data-slice-index` = `sliceOffset` + i) and its sound (`sliceAudio` with the frame's index and the frames, so music's
  walk back, music.ts:19, holds). The vertical slices carry neither: the viewer's slice.svelte gets one prop that leaves
  out `data-slice-index` (:266) and `sliceAudio` (:288-293). So reading progress (track-reading.ts:88-99), checkpoints
  (`position`, `{group, slice}`: content.ts:459-471, +page.svelte:229-249), resume and `?label=` arrivals, chain offsets
  (chains.ts:71-82), the follow-up bar (next-bar.svelte:129) and a deferred script start
  (interactions/visual-novel.svelte:113) keep their meaning, and a checkpoint from one layout resumes at the same frame
  in the other. `sliceInView` (visual-novel.svelte.ts:245-258, screen effects) also finds a vertical slice.
- **Payload.** The library's chapter answer (Library.php:376) keeps `vertical` when it has an `if`, else drops it.
  Offline downloads walk that answer (ChapterFiles.php:54-56, :134-155): a layout readers can get is saved with the
  frames (the condition may change offline); an export-only one costs them nothing.

**The WEBTOON export** always draws the vertical version, whatever its `if`: the column (§3) passes `verticalSlices`
to the shared `Slice` without callbacks – boxes in their end state, items at rest, no areas. A horizontal group without
one is an **error** that stops its chapter (§9), never frames stacked: "Chapter 2 · scene-14 (12 frames) has no
vertical version – add it in the creator: …/catpope/chapter/2?group=scene-14&version=vertical".

**Asking for them** (owner (1), (4)):
- **The artist:** Wanted images' stored media requests (`view: 'media'`, `toon`, `path`, `size`, `note`;
  characters.ts:391-417), closed by themselves once a file is at `path` (characters-editor.svelte:113-127). `path` is
  from the CDN root (characters.ts:399; `imageExists` → `asset`, creator characters.ts:235, cdn.ts:5-6), so it starts
  with the toon: `catpope/s1/c2/s1c2-scene-12-v1.webp`, the name the empty vertical list gives its first slice (below),
  so "Add slice" picks a delivered file up. One request per group the owner names as missing, written as one MCP draft;
  size "1170 wide, 1:4 … 1:7, one image"; note "the whole scene recomposed top to bottom, without text and without the
  placed items (they go on top: scene-13's swim pants is a tap item, not a pickup); room between panels and for the
  boxes, ≥ 300 px at 1170 (≈ 200 at 800)".
- **The creator:** the export's error links to the group's empty vertical list.

**Creator** (on #44's strip editor; a design draft on the real page first, phone + desktop):
```
┌ catpope › Ch2 › scene-12                                                 Save  Publish ┐
├ sidebar ──────────────────────────┬ canvas ──────────────────────┬ inspector ───────────┤
│ ⋮ scene-12                     7  │ ┌───────────────────────┐ 1  │ [Group│Layers│⚡│Slice]│
│ [Horizontal │▓Vertical version▓]  │ │ tall picture  ┌─────┐ │    │ Layout [Horizontal ▾]│
│ ▣1  ┌╌+╌┐  Import slices …        │ │ ⚡ unlock     │ box │ │    │ Readers see it if    │
│ ⋮ scene-13                    12  │ │┄┄┄┄┄┄┄┄┄┄┄┄┄┄└─────┘┄│ 2  │ [ Only if …        ] │
phone, no vertical version yet: [Horizontal │Vertical version] → only ┌╌+╌┐ and Import slices …
```
1. **The flip:** one square `sc-seg` above the open group's film strip (horizontal and endless groups only): the group's
   Layout value (`Horizontal` / `Endless`) │ `Vertical version`, so no half reads as another Layout value. View state,
   not data (`view.version` and the open vertical slice in ChapterView, chapter-state.svelte.ts:50;
   `&version=vertical` in `hrefFor`, chapter-editor.svelte:504). The film strip, the canvas and the Slice tab switch
   with it; the open frame maps to the vertical slice over its share and back; the tooltip says readers get it by a
   condition, the WEBTOON export always. Empty, the vertical film strip shows the list's own empty slot ("Add slice" or
   drop images, slice-list.svelte:242-264) and "Import slices …": no new pattern, no Problems entry (it would flag every
   horizontal group of every toon).
2. **List and import** take a `listPath` (`[...group, 'vertical', 'slices']`) and a `stem` (`s1c2-scene-12-v` → `-v1`,
   `-v2`; today `s1c2-slice-<n>`, slice-list.svelte:134, slice-import.svelte:206-219), and create a missing parent
   instead of falling back to the chapter's root (`getAt(file, groupPath) ?? file`, slice-list.svelte:128,
   slice-import.svelte:239). The first slice creates `vertical`; removing the last one (after its confirmation, :144)
   removes it. **Layout → Vertical** while a vertical version exists asks first (`askDialog`: "removes its vertical
   version – n slices, m placements"), then drops it in the layout's undo step (group-panel.svelte:44-50).
3. **The canvas:** a new `vertical-canvas.svelte` (≤ 300 lines) stacks the vertical slices like strip-canvas, with
   hairlines and frame numbers at the shares. Each is a `SliceCanvas` with one new prop, `source`: the mapped slice to
   draw (else `doc.resolvedSlice`, slice-canvas.svelte:106) and, per overlay, where its move writes. Selecting an
   overlay selects its definition on its frame (`view.sliceIndex` + the usual `{kind, index}`, slice-canvas.svelte:4-7),
   so the inspectors, Layers and the clipboard work as on the frame – on both layouts. A drag or nudge writes only the
   placement; a resize, a turn or a text edit changes the definition. Adding a text, item or area: `sliceTarget()`
   (chapter-editor.svelte:731) of the frame under the click, the inverse spot, and the placement. A drop over a vertical
   seam moves the placement to that slice (not #44's `moveAcrossSeam`, strip-anchor.ts:43, which moves definitions);
   "Back to the automatic spot" removes it. No piece tool, no Audio tab; a vertical slice's Slice tab is its picture.
4. **The condition** (with the reader step): under Layout in the Group tab, while a vertical version exists, "Readers
   see it if" = event-condition.svelte on `[...group, 'vertical']`; empty = export only. The Reader panel shows what a
   reader gets; an export-only layout is seen on the canvas (DOCKING's two messages stay).
5. **Logic in `lib/editor/vertical.ts`** (pure, tested); the existing components take a prop or a few lines each. The
   spot and its inverse; place, move across a seam, unplace; deleting a definition removes its placement; slices pasted
   into the vertical list keep only the picture fields. Group paste remaps `of`: `prepareGroup` (clip.ts:329-345) gives
   the events new ids itself (`newEventId`, event-ids.ts:11) instead of dropping them (:100, :337), keeps the old → new
   maps of boxes (`renameBoxes`, :288), placements (`freshSlice`, :105) and events, and rebases the vertical pictures
   like the frames'. A problem on a vertical slice opens it (`gotoProblem`, chapter-editor.svelte:650).

**Validation** (`checkVertical`, next to `checkRendererParams`, validate.ts:959):
- **Errors:** `vertical` on a vertical group; no slices; a vertical slice without image / width / height (the picture
  part of `checkSlice`, validate.ts:671-676); an `of` placed twice; a broken `if` (`checkCondition`, validate.ts:424).
- **Warnings:** an `of` naming nothing placeable in the group (old data, the MCP; the reader ignores it); a sound
  area on a frame of a group with a vertical version (not on the vertical layout yet).
- **No new id rule:** an element without an id keeps its automatic spot; box, event and placement ids are unique per
  chapter already (validate.ts:1061-1124).

**Where the fields flow:** the types (content.ts; `GroupDoc` inherits, neutral.ts:115); the schemas and the server's
copy (`npm run schema -w @scutoons/content` → server/schema/content, checked on save by ContentSchemas.php); the
validator; resolving passes `vertical` through, it holds no text (neutral.ts:248, NeutralDoc.php:169-177); TextCheck.php
one line for `vertical.if.expr` (like :61 for events); Library.php:376 (the `if` rule); ChapterFiles, ItemPlacements
and AssetUsages unchanged (the last walks the whole document: the pictures count as used); flow, reach and dead things
stay on the frames (flow.ts:1255 already reads any `if.expr` in a group); the MCP's workflow rules
(tools/mcp/src/resources.ts:82) get one paragraph: place, never copy.

**Migrations: none.** A new optional field reshapes no stored data, and every box, item and event already has an id.

**Left out:** an element on one layout only (owner (5)); per-layout text, look, size or settings; a tail tip of its own
on the vertical layout (none of the 8 groups has a free tail); a vertical slice's own captions, pieces, audio or
sensitive flag; sound areas on the vertical layout; a horizontal version of a vertical group; a live flip, a reader
setting, a chapter / toon default, `device.reducedMotion`; a Problems entry.

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
| A horizontal group without a vertical version – named with its frames and the creator link `…/chapter/<n>?group=<label>&version=vertical` | error |
| validate's unaccepted warnings for that language (missing translations, validate.ts:1538-1558) | warning |
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
- **After #64 closes:** the build (export view, exporter, kit) and the vertical versions (model, creator, export branch –
  after #44's strip editor, whose files the creator part changes); their reader side is outside the milestone. The owner
  orders them against #62 (studio) and ch4. The export view changes the viewer's root layout, so it deploys outside a chapter's deploy
  freeze.
- **The launch** (owner 2026-10-20: "launch date is february. after the scutoons top toy event."): a Sunday after the
  Top Toy workshop (Wed 2027-02-17 – Fri 02-19, no code work from the 17th; milestone "Top Toy workshop"), so Sun
  2027-02-21 or 02-28 – planned: **Sun 2027-02-28** (the upload and the Content Review, 2–3 business days, right after
  the workshop, Feb 20–23). It starts with ch1's three episodes (WEBTOON's advice: launch with 3; the subscribe pop-up
  follows the third), then one episode every Sunday, never ahead of scutoons.com: the whole season is on scutoons.com
  by then (ch5 on 2027-01-15), all eight vertical versions are done in one go, and the weekly queue (13 episodes, to
  late May) can't run dry. The build (export view, exporter, kit, vertical versions) falls between #64 and the
  workshop, beside the reader's foundation work (Nov 15 – January); the test draft by early February. Episode 10, the
  earliest AI-translation date, lands about 7 weeks after the launch. The de series follows about two weeks after en.

## 11. Left out, and why
- **A WEBTOON panel in the creator, WEBTOON metadata in the DB** (cuts, titles, notes, upload state): one-time entries;
  `plan.json`, the upload sheet and WEBTOON's dashboard hold them. Less UI is better UI.
- **An MCP tool `export_webtoon` and a creator "Export for WEBTOON" entry:** later, when members other than the owner
  export. The core has no dependencies, so rolldown can bundle it (tools/mcp/scripts/build.mjs); it would need the
  MCP's first process spawn, a Chrome path on macOS / Linux and a job for runs longer than a tool call.
- **Rendering in PHP, in a creator canvas or via `foreignObject`:** second renderers that drift. **Rendering at 1600,
  pixel gutter detection, 16-px alignment, encoding in the page:** no visible gain; Chrome's capture returns the JPEG.
- **Copied or linked texts on vertical versions (`textOf`), an element on one layout only, per-layout size or look, a
  vertical slice's own captions, pieces, audio or sensitive flag, pickups painted into the art:** one definition gets
  onto both layouts (owner: "any text box, item, event def gets onto both"); a caption or pieces can come later.
- **Derived "vertical version" requests (a flow index v7), a Problems entry, a dashed "missing" mark:** the stored
  requests, the empty vertical list and the export's error with its link say it; derive them if horizontal groups
  become common.
- **A live flip on rotation, a reader setting "scroll horizontal scenes vertically", a chapter / toon default,
  `device.reducedMotion`:** the group's `if` decides once per page view (§8); no own reduced-motion setting (DECISIONS
  2026-10-07), and the band keeps drag + glide under reduced motion (TEXT-ACROSS-SLICES.md §8).
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
2. **Launch date** – answered (owner 2026-10-20: "launch date is february. after the scutoons top toy event."): a
   Sunday after the workshop (Feb 17–19, 2027); planned Sun 2027-02-28, the milestone's due date (§10). Sun 02-21 only
   if the upload and the Content Review are done before the workshop – say so if you want it.
3. **Version:** export only what is published on scutoons.com, test kits from drafts? Rec.: yes; #68 publishes the
   they / them drafts first.
4. **VN lines:** drop them in the export (the preflight quotes them)? catpope's one story line is ch3's "No photos,
   please!" on a slice whose furniture still moves (DECISIONS 2026-10-20). Rec.: drop; whether that line becomes a
   normal text box (then on scutoons.com too) is asked with ch3's kit.
5. **Episodes:** about 40k px, at most 45k (≈ 32 images, ch1 = 3 parts), titled "Denial (1)", every part with the chapter's
   thumbnail? Rec.: yes; confirm the cuts on the first export.
6. **Chapter 0** (only the music video): skip it, and put the video in the creator profile's links and the first note?
   Rec.: skip; a one-image episode pointing elsewhere is the external-traffic pattern WEBTOON bans.
7. **End card:** one image per language at each chapter's last episode ("made for scutoons.com – with music, a map and
   things to collect"), made by you, kept as a local kit input? Rec.: yes, mentioned in the Content Review request. If
   a funder (SKE / austro mechana, Westbahnstudios, Künstlerhof Frohnau) requires a logo on every publication, it goes
   there – please check.
8. **Vertical versions for readers** (answered 2026-10-20: one tall picture per scene, a slice list of its own, every
   text box, item and event on both layouts, readers by a condition, the export asks for a missing one – §8). Still
   open: (a) no condition = export only, so readers keep strips and bands until an author adds one? (b) the conditions:
   only the names events and texts read today, or also `device.portrait` (the window taller than wide when the page
   opens; opt-in per group)? A layout picked by orientation is no quality level of one picture, so RULES' "media quality
   … never by viewport" stays untouched – fine? No reduced-motion name (DECISIONS 2026-10-07). (c) decided once per page
   view, before the page's groups appear (on a page with a condition a member waits up to 3 s for the save game), so a
   choice or a turned phone counts from the next page (a live flip risks the 2026-10-10 kind of scroll jump)? And which
   of the 8 vertical pictures exist already (then the requests are uploads, not new art)? Rec.: (a) yes, (b) also
   `device.portrait`, opt-in, and fine, (c) yes.
9. **AI-assisted art** (ch3's gap shots, #61; anything else?): if any, one disclosure line in the creator profile and
   no WEBTOON contests? Rec.: yes; WEBTOON asks nothing, the line costs nothing.
10. **Series thumbnails:** the season cover's art without lettering (WEBTOON advises no text; one set for both series),
    or with the cover's lettering per language? Rec.: without.
