Status: built 2026-10-18 – the collage editor (the night's §8 steps 1–7, nothing cut) and custom presets ("Save as preset", §12 steps 1–6); on production since the owner's push that evening, except the presets' autosave fix and the MCP part (§12.6), on `main`, not pushed – owner questions §10 and §12.8 open, built with their defaults (FEATURES 10.105); deferred §9. Today's state: DESIGN.md Part 2 "Collage pieces", DATA.md §2, CATALOG.md "Score collages"; this doc keeps the design and how it was measured.

# COLLAGE – pieces (vectors, characters, images) on slices and scores

Owner 2026-10-17: "an editor for the scores, so that, even though the scores are dynamically building depending on page size, I can still add these vector graphics on them and blend them … our avatars and portraits and full … placing vector graphics onto the slices … all of the available blending modes … drop shadow … outlines, color fill gradients, and blurring them … with a start and end point." Standing decisions: colour is never stored on a shape – a colouration is a named preset applied to any shape (VECTORS §3); shapes are CDN assets with tags (several per shape); the random lab collage is not the goal; vectors never distorted.

**One piece shape, one renderer, one picker, one inspector, two hosts** (the slice editor that exists, one new score stage page). Less UI is better UI.

## 1. Checked in the code (2026-10-18)
- `claude/editor-inputs-e870d0` is merged into main (ee723dc1, 4740c13c, 80b01804): ScrubNumber, PropRow, PropSelect, ColorField, LinkedPair, Segmented square / fallback, icons-extra.ts.
- Vector library: d53a041b (`asset_shapes`, `GET /api/assets?kind=shape`, `@scutoons/api` AssetShape) + d72e88d0 (436 shapes in `media/catpope/vectors/`); on production since the morning of 2026-10-18 (MCP `import_vectors` as the owner – research/VECTORS.md step 5).
- CORS: `access-control-allow-origin: *` for WebP and SVG on the local CDN (:8001) and for WebP on cdn.scutoons.com (curl). CSS masks of character pictures are safe.
- Local score items: 6 published (cloud9_en …), full score only (`?part=choir` → no_part); `GET /api/items/cloud9_en/score/musicxml` signs `scores/cloud9_en/4971f460e393.musicxml` (200).
- Verovio 6.3.0 (`svgAdditionalAttribute: ['measure@n','staff@n']`, `adjustPageHeight:false`): outer `<svg>` = the page in px; `svg.definition-scale viewBox="0 0 pageWidth×10 pageHeight×10"`; `g.page-margin transform="translate(mx,my)"`; `g.measure[data-n] > g.staff[data-n]` whose first 5 `<path d="M x1 y L x2 y">` are the staff lines, 180 units apart.
- `.slice-layers` is `position:absolute; z-index:1` (slice-layers.svelte:80) – a stacking context; nothing inside blends with the picture. fx `inFrame` animates every child of `.slice` separately (cues.ts:228).
- `ProtectedFiles::publicItem` rebuilds `score` as an allowlist – "the public catalog never names their paths". `ScoreField`'s `set` / `setChoir` spread keep extra score fields.
- Any `character` key is walked by `ChapterFiles::characterRefs` (saves the whole persona offline) and `Library::referencedCharacters` (adds to the cast).
- Fall-throughs: chapter-editor.svelte:1277 (unknown tool → addSampleArea), slice-edit.ts removeSelected (→ events), slice-inspector rows (→ item). `slice-clip.ts LISTS: Record<kind>` and `makeClip(sel.kind)` make the clipboard type-check only once it knows `piece`.
- score-looks.css styles `.leaf > svg` (54, 175, 229, 261): the music SVG must stay a direct child of `.leaf`.
- `check:tools-css` already reports lines on main (score-view.svelte:358 Feather "x", map tabs, bar-tabs): green = no new line in a touched file. After the build: 16 lines, none in a collage file (the score view's "x" became `CloseButton` in step 5).

## 2. Data model (`shared/content/src/collage.ts`, exported from index.ts)

```ts
import type { Anchor } from './anchor.ts';
import type { CharacterImageVariant } from './characters.ts';

/** the 16 CSS blend modes – identical in Chromium, Firefox, WebKit on an HTML box (never on SVG children) */
export const BLEND_MODES = ['normal', 'multiply', 'screen', 'overlay', 'darken', 'lighten', 'color-dodge', 'color-burn',
	'hard-light', 'soft-light', 'difference', 'exclusion', 'hue', 'saturation', 'color', 'luminosity'] as const;
export type BlendMode = (typeof BLEND_MODES)[number];

/** % of the piece's own box (0 0 top-left … 100 100 bottom-right): moves, scales, turns with the piece */
export interface PiecePoint { x: number; y: number }
export interface GradientStop { at: number /* % along the line, ascending */; color: string /* #rgb | #rrggbb */; opacity?: number /* 0…1 */ }
/** one linear gradient across the whole silhouette */
export interface PieceGradient { angle?: number /* CSS deg, 0 = to the top; default 180 */; stops: GradientStop[] /* ≥ 2 */ }
// lengths: % of the PIECE's width, drawn in cqw of the piece box
export interface PieceOutline { color: string; width: number }
export interface PieceShadow { color: string; x: number; y: number; blur: number; opacity?: number }
/** even, or with from + to: sharp at `from`, `amount` from `to` on (progressive) */
export interface PieceBlur { amount: number; from?: PiecePoint; to?: PiecePoint }

/** a colouration – a preset's values, or a piece's own over its preset (a key replaces it whole; null = off) */
export interface PieceStyle {
	blend?: BlendMode;
	opacity?: number; // 0…1
	fill?: string | PieceGradient | null; // the file's silhouette (its alpha) in a colour / gradient
	outline?: PieceOutline | null;
	shadow?: PieceShadow | null;
	blur?: PieceBlur | null;
}
/** a named colouration: one of the ten COLLAGE_LOOKS in code, or a custom preset (CollagePreset, §12.2) – one id namespace */
export interface CollageLook extends PieceStyle { id: string }

/** how a character picture was picked (the editor re-resolves `src`); the renderer draws `src` only.
 *  Not `character`: that key makes ChapterFiles save the whole persona offline and the Library add it to the cast */
export interface PieceFigure { id: string; persona?: string; look?: string; mood?: string; view: CharacterImageVariant }

/** a score piece's place: a bar of the file – never a page, never a Verovio id */
export interface ScoreAt {
	bar: string;     // as the file writes it (MusicXML measure@number = MEI @n): "12", "0", "6a"
	nth?: number;    // the n-th bar with that number; default 1
	staff?: number;  // from the top (staff@n); default 1; hidden in a condensed system → the bar's top staff
	page?: boolean;  // on the page that holds the bar (a background); rect in % of that page
}

export interface CollagePiece {
	id: string;        // `piece-<7 random>` (newPlacementId)
	src: string;       // the file, in the host's path form (§2.1)
	ratio: number;     // the file's width ÷ height, copied at pick time – laid out before load, never distorted
	figure?: PieceFigure;
	/** x / y = the `anchor` point, width; height always from `ratio`.
	 *  slice: % of the picture. score (music): x = % of the bar's time (0 = its first beat, 100 = its end), y = staff
	 *  spaces below the staff's top line, width = staff spaces. score (`at.page`): % of the page */
	rect: { x: number; y: number; width: number };
	anchor?: Anchor;   // default 'tl'; score pieces are written 'c'
	rotation?: number; // degrees around the centre
	preset?: string;   // a CollageLook id (a code look's or a custom preset's); none = as drawn – readers get custom presets baked
	style?: PieceStyle;// own values over the preset
	at?: ScoreAt;      // scores only (required there)
	behind?: boolean;  // scores only: under the notes; default over
}

/** ScoreFiles.collage (full score) / ScorePart.collage (score.parts.<name>) */
export interface CollageSheet {
	file: string;            // the vault path it was placed on; another file on the part = "check the bars"
	pieces: CollagePiece[];  // back → front within each side
}
```
Where: `Slice.pieces?: CollagePiece[]` (content.ts, next to `items`; array order = paint order back → front); `ScoreFiles.collage?` / `ScorePart.collage?: CollageSheet | null` (containers.ts). All new and optional: no stored key exists (prod seed: 0) → **no migration** (precedents `items`, `layer`, `anchor`, `Dialog.fit`). `SliceDoc` inherits through its `Omit`; `resolveSliceWith` / `NeutralDoc::slice` pass pieces through (no text). Not stored: kind (derived: `figure` → Character, `.svg` → Shape, else Image), mask (every effect works from the file's alpha), `layer` (array order).

Pure helpers in collage.ts (tested): `COLLAGE_LOOKS`, `collageLook(id, presets?)` (the code look first, then a custom preset), `pieceStyle(piece, presets?)` = `{ ...look, ...piece.style }` minus nulls, `pieceHeight(width, ratio, boxRatio)` = `width / (ratio × boxRatio)`, `heavyWeight(style)` (outline 1, shadow 1, blur 1, progressive blur 2), `pieceName(piece)`, `COLLAGE_LIMITS`, `collageProblems(pieces, host: 'slice' | 'score', add, presets?)`; for the custom presets `customPreset`, `bakePresets`, `colorationProblems` (§12.2, §12.5).

**Presets by reference.** A piece names a preset (`preset: 'negative'`) and may replace single keys in `style` (a whole shadow, or `null` = off). Renderer and inspector both read `pieceStyle()`, so a preset change reaches every piece that has not replaced that key. Ten code presets from the lab's INKS / GRADIENTS / BLENDS: `ink` fill #111111 · `red` fill #ff1900 · `cyan` fill #00d1ff · `negative` fill #ffffff + difference · `burn` #ff1900→#b300ff + multiply · `acid` #00d1ff→#37ff00 + exclusion · `sticker` outline #ffffff 3 + shadow #111111 2/2/0 @0.8 · `glow` shadow #00d1ff 0/0/6 · `ghost` opacity 0.5 + blur 3 · `fade` blur 6 from 50/0 to 50/100. Custom presets ("Save as preset", §12) share that id namespace: one platform-wide document `colorations`, drawn the same way in the creator; readers get them baked into the piece's `style` by the server (§12.3), so their renderer only ever resolves the ten.

### 2.1 Paths and usages
Toon documents store `toonAssetRef(toon, cdnPath)` (catpope: `vectors/wreath.svg`; others: `../catpope/vectors/wreath.svg`, `../characters/catpope/…webp`). Catalog rows store CDN-root paths (`catpope/vectors/wreath.svg`; a `../` path there is never indexed). `src` is a media-extension string: AssetUsages, the delete check, relink / rename and ChapterFiles pick it up with no server change. `CollageSheet.file` (.musicxml) is not a media path and never leaves the server (§4.1).

## 3. Slices

**Layer stack** (layers.ts): `SliceLayerKind += 'piece'`, `LayeredSlice.pieces`. `entries()` lists pieces first with layer `-Infinity` (the sort's tie-break keeps array order), so `sliceLayers()` puts them behind every text box, item and area; `refEntries` gets a `'piece'` case (exhaustive switch). `setSliceLayers` skips pieces and numbers the rest 0…n-1; `hasSliceLayers` ignores pieces – art never switches the reader's hit testing. The reader's `SliceLayers` builds its own `{dialog, items, events, audio}` and never sees pieces.

**The picture group** – new `shared/reader/src/collage/slice-picture.svelte`, used by `Slice` and by the creator canvas around their own `<img>` (a snippet, so `.slice img` / `.creator-canvas img` still apply):
```svelte
{#if drawn}<div class="slice-media" {@attach near}>{@render children()}{#if shown}<div class="slice-pieces">{#each pieces as piece (piece.id)}…CollagePieceView…{/each}</div>{/if}</div>{:else}{@render children()}{/if}
/* .slice-media { position:relative; isolation:isolate; overflow:hidden }  – the ONE blend group: picture + art
   .slice-pieces { position:absolute; inset:0; pointer-events:none }  – never z-index/transform/opacity/filter/will-change/isolation/container-type */
```
- `drawn` = pieces given, a `src` resolver given, an image slice (not blurred, not video); the creator passes `always` (no remount under the canvas). Slices without pieces render today's DOM unchanged (first-screen gate `querySelector('video, img')`, fx `imgOf`, the band's `--plane`).
- Isolation makes the backdrop the slice picture alone – the same in vertical, strip, endless band, old iOS, the creator canvas and the Reader dock. The group paints at the z-0 level, so the earlier slice's `.slice-layers` (z 1) still paints over it: text across slices unchanged. fx impacts animate `.slice-media` as one child: no flicker. Pieces are clipped to the picture (owner question 2). Same name and place as the `.slice-media` wrapper TEXT-ACROSS-SLICES §5 plans.
- `near`: IntersectionObserver (rootMargin 100 %), once near stays mounted (CSS masks are not lazy); the creator passes `eager`.
- `Slice` gets `pieceSrc?: (src) => string | null` (none = no art, like items without `itemInfo`); viewer `toon/slice.svelte` passes `(s) => toonUrl(toonId, s)`, the canvas `(s) => toonUrl(doc.toon, s)`.

**Editing** (the existing slice editor):
- Tool `piece` (icon `shapes` – a triangle, a square, a circle, icons-extra.ts; Feather `scissors` stays Cut) in the tool strip: text · item · piece · sound · event. chapter-state: `TOOL_TAB.piece = 'layers'`, `KINDS.layers += 'piece'`, `select()` / `setTab()` treat it like item; `pickingPiece`, `pieceSpot`, `pickPiece(spot)`.
- Click → the picker → the piece centred at the spot (anchor 'c', width 30 %); a drag → anchor 'tl' at the rect, its width kept. `chapter-editor` gets an explicit `Record<Tool, …>` create map (no ternary fall-through).
- Canvas: `ItemKind += 'piece'`; items from a pure `lib/editor/canvas-pieces.ts` (path `[...slice,'pieces',i,'rect']`, label `▲ name`, rotation, anchor factors, height `pieceHeight(width, ratio, sliceRatio)`, writes `drawnHeights` for AnchorField's reanchor); `transformable`; `handlesOf(piece) = []` (move, rotate, uniform scale – never stretched; `scaledRect` writes no height); centre snap, grid, nudge, undo per drag come free. Blur ramp: two round dots on the selected piece (`canvas-point-handles.svelte`, rotation-aware `pointInBox` in canvas-geometry.ts) – first cut line. A piece's box takes the pointer only under the piece tool or while selected (`pieceTakesPointer`): art is often as large as the slice, so another tool draws over it (the text tool drags a text box, the item tool places); picking another tool lets a selected piece go.
- Layers list (moved into `slice/layer-list.svelte`, so slice-inspector stays under 300): piece rows at the bottom (thumb = the file on `.sc-checker`, glyph `shapes`, the name `pieceLabel`), a top rule marks the boundary; `moveLayerRows()` (collage-edit.ts): piece ↔ piece reorders `pieces[]` and answers where each piece went (`followPieces` keeps the selected piece selected – the stack keeps its indices; undo / redo follow it by id, `piecesMoved`), stack ↔ stack = `setSliceLayers`, a drag across is refused. `+ piece` in `.adds`; selected piece → `PieceInspector host="slice"`.
- `removeSelected`: explicit `piece` and `event` branches. Phone bar label `piece`.
- Clipboard: `LISTS.piece = ['pieces']`, `ClipKind += 'piece'`, `clipWhat`; paste = new id, `offsetRect`, `src` rebased across toons; `rebaseSliceMedia` / `foreignFiles` walk `pieces[].src`; Shift+V `applyPieceLook` copies `preset` + `style` (cut line).
- As built: the canvas draws the pieces through `SlicePicture always eager` around its own `<img>`; `slice-edit.ts` `SLICE_LISTS` (one kind → list map for remove, copy, paste) + `addPiece`; `clip.ts` `offsetRect` (moved) / `preparePiece` / `applyPieceLook`; one name for a piece everywhere – `pieceLabel` (collage-words.ts): a shape's library title, a character's name · mood, else the file; the canvas gets it as its `pieceName` prop (the chapter editor has the registry), the lists and the inspector's head use it too. A Layers drag across the boundary changes nothing and a toast says why ("Pieces always lie under the text boxes and items"). `check:creator-chapter -- --pieces` runs only the piece case (+ save, reload, restore) on a chapter / group / slice of `SCUTOONS_CHAPTER` / `_GROUP` / `_SLICE` – catpope 1 and 2 carry unpublished drafts in the dev DB, catpope 3 group 2 has inline slices.

**Validation** (`checkSlice` → `collageProblems(s.pieces, 'slice', …)`): errors – not an array / object, missing or duplicate id, `src` without a media extension, `ratio ≤ 0`, rect not numbers or width ≤ 0, a `height`, bad anchor / rotation, blend not in BLEND_MODES, opacity outside 0…1, colours not #rgb/#rrggbb, gradient < 2 stops or `at` outside 0…100, only one of from / to, `figure.view` unknown, `at` / `behind` on a slice. Warnings – unknown preset; > 12 pieces; heavy weight > 4 ("slow on phones"); outline > 8, blur > 20, shadow blur > 25 (% of the piece); pieces on a video slice ("shown on picture slices only"). Schemas regenerate (chapter, chapter-doc, group, group-doc, catalog – both copies); the server guard then enforces the enums and the colour format (`PieceColor`'s pattern) with no PHP change.

## 4. Scores

### 4.1 Storage and delivery
`score.collage` / `score.parts.<name>.collage` in the `item:<id>` document: the creator already edits it with drafts, history, undo and Publish (MetaPage); each collage is bound to its file by construction; `publicItem` strips it (no vault path, no bulk in every reader's `/api/items`). Readers get it with the signed URL they already request: `ProtectedFiles::score(…, 'musicxml', part)` answers `{ url, expires, collage?: { pieces } }` (never `file`); `@scutoons/api` `scoreFile` widens its type; viewer `score-actions.svelte` passes `pieces` + `pieceSrc={asset}` to ScoreView. Server guard: `CatalogValidator::container` checks each collage structurally (`file` under `scores/`, pieces a list, unique string ids, `src` media extension, `ratio > 0`, rect numbers, `at.bar` non-empty string, `style.blend` in `BLEND_MODES`, the colours – fill, gradient stops, outline, shadow – #rgb / #rrggbb) + a CatalogTest parity test of BLEND_MODES (like FX_SOUNDS). TS `validateContainers` runs `collageProblems(…, 'score')` (at required; > 60 pieces / heavy > 20 warn), wants `collage.file` under `scores/` like the server, and warns when `collage.file` ≠ the part's musicxml ("made on another file – check the bars"). The stage edits the part readers open (`readerPart`: choir if present, else the full score – locally the full score).
As built: `ProtectedFiles::score` adds `collage: { pieces }` to the MusicXML answer when the opened score / part has pieces (`collagePieces`; an empty list = none); `CatalogValidator::collage` runs on `score.collage` and every `parts.<name>.collage` at a catalog write (the published item); the viewer passes `pieceSrc={asset}` (catalog paths from the CDN root).

### 4.2 Anchoring (survives page size, zoom and reflow)
- Music (default): reference point = bar `at.bar` (`nth`), staff `at.staff`: x = `rect.x` % of the bar's **time** (0 = its first beat, 100 = its barline; bar widths vary 2–3× between layouts – a piece at 80 % of a bar stays at 80 %), y = the staff's top line + `rect.y` staff spaces, width = `rect.width` staff spaces (scales with zoom like the music). The piece's anchor (written 'c') sits there. Time, not the staff lines' width (step 5 review fix, 2026-10-18): a line's first bar holds the clef, key and time signature inside its lines (its first note 5–17 sp in on the local files, 0.4–6 mid-line) and Verovio's justification stretches only the space between the notes – % of the staff lines put a piece 6 sp off between phone and desktop (Cloud 9; 10 on Like a Baby) whenever its bar starts a line in one layout and not the other. The bar's time on a page (`beats`): each moment a note / rest starts, in any staff, stands in the middle of its leftmost notehead (+ 0.6 sp from the glyph's edge), the barline at 100; linear between two moments, at the bar's mean pace before the first and after the barline. A piece on a note stays on it in every layout: measured over all six local scores, 9 layouts (phone / zoom ± / stage frames / tablet / desktop), every note and rest, a piece half a staff space into its notehead: p95 ≤ 0.33 sp, p99 ≤ 0.6 sp (was p95 5–9 sp). Known limit: a note pushed off its moment's column (a second voice moved aside, a second's displaced head) keeps a fixed offset that the time stretches with the bar (up to ~5.5 sp – e.g. a whole-note bar that fills a phone line, roarlittlekitty bar 16). Known limit: the beats are timed by `dur.ppq`, but Verovio places events by their written value, so a MuseScore export where the two differ (nohesaid bar 71: eighth rests of a dotted eighth's length) drops some moments, with drift ≤ 1 sp (p99 0.16 sp over that file).
- Page (`at.page`): `rect` in % of the page that holds the bar (backgrounds).
- A bar that moves to another page takes its pieces along (geometry is read from every rendered page). A bar missing from the file: readers skip the piece, the stage lists it. A replaced file: the stage's notice "made on another file – check the bars" + **Checked** (writes `file`). Renumbered bars move pieces musically without notice (known limit).
- `shared/reader/src/score/score-bars.ts` (pure): `barIndex(mei)` (measure id → `{ n, nth, onsets, rests }` from `tk.getMEI()`, 6–46 ms with getMEI on the local files; ids stay through `redoLayout`) – `onsets`: note / rest id → its start in parts of the bar, from the `dur.ppq` Verovio's MusicXML import writes (÷ the staff's `staffDef@ppq`; a chord's notes start with it, grace notes take no time, the bar = its longest layer, an event without a length stops its layer's count); `rests`: bar rests (mRest, mRpt …, a column at 0.5 only when nothing else is drawn); `beatsOf`, `barX(bar, t)` / `barTime(bar, x)`; `findBar(bars, at)`. `shared/reader/src/score/score-anchors.ts` (pure, node-tested against a fixture SVG from a real 6.3.0 render and by test/score-reflow.test.ts: one written file laid out by the real Verovio at phone / zoom + / desktop – bars that start a line on the phone only, every note within 0.06 sp, the old staff-line % 6.4 sp off): `pageGeometry(svg, bars)` → `{ width, height, sp, bars: { id, n, nth, left, right, staves: { n, top, bottom }[], beats: { t, x }[] }[], frame }` in viewBox units (margin translate included; sp = the median line distance; the glyphs: `g.note > g.notehead > use`, `g.rest > use`); `placeScorePiece(piece, geo)` → `{ left, top, width }` in % of the page or null; `anchorScorePiece(point, geo, page?)` → `{ at, x, y }` (the bar whose system band holds the point, else the nearest; the nearest staff; x = the bar's time at the point, rounded to 0.01 %, y to 0.001 sp; staff 1 / nth 1 not written). No DOM measuring, no px, no rem. The leaf is not exactly the SVG's box: Verovio writes the element's px height rounded up and centres the viewBox in it (`xMidYMid meet`, up to half a pixel) – `frame` is the viewBox in % of the element's width and `pageFrame(geo)` the layer's CSS (width, aspect-ratio, margins – no transform), so % of the page are % of the music (measured: ≤ 0.4 px in Chromium / Firefox / WebKit at 390 and 1280).

### 4.3 Rendering in ScoreView (behaves as today without pieces)
- Move, not copy: `score/score-layout.ts` takes `ZOOMS`, `DEFAULT_ZOOM`, `PHONE_ZOOM`, `margin()`, `scoreOptions(w, h, zoom, look, remPx)` (merges `measure@n` / `staff@n` into the look's `svgAdditionalAttribute`) out of score-view.svelte. `ScoreToolkit += getMEI`.
- New props: `pieces?`, `pieceSrc?`, `embedded?` (no portal, no window keys, no stored zoom, absolute + `isolation:isolate` in its box), `zoom` (`$bindable`), `page` (`$bindable`), `follow?: ScoreAt` (the layout anchor and "turn to it" instead of the first bar), `onlayout?(bars)`, `onclose?` optional (no close button / Escape without it), `decor?: Snippet<[number, PageGeometry | null]>` (the lab's one-argument snippets still fit). The header's Feather "x" becomes `CloseButton` (the file's tools-css line).
- Per page: `{@html svg}` → `<ScorePieces>` (`.score-pieces { position:absolute; pointer-events:none; overflow:hidden }` on `pageFrame`, no z / transform / isolation – overflow clips a piece at the page's edge so the leaf never scrolls for it; each piece carries its own z: behind 0, over 2) → age → decor. With pieces: `.sheet[data-pieces] .leaf > :global(svg) { position:relative; z-index:1 }` and `.score-age` z 2. The paper is `.sheet`'s white inside `.score-view`'s stacking context; blends reach it across `.pages` / `.leaf` (tested). Only rendered pages (current ±1) draw pieces. The PDF stays clean.
- As built: the bars are read (one `getMEI` per load) only with `pieces` (even empty – the stage), `follow` or `onlayout`, so a plain score does no extra work; `onlayout(bars)` after every layout; `decor(n, geometry)` gets null without bars, and with pieces the music is z 1 – a decor over it brings its own z (the stage's overlay: 3). `rendered` holds `{ svg, geometry }` per page. The player's rAF loop was renamed `keepUp` (the prop is `follow`).

### 4.4 The score stage (creator)
Catalog → Items → a score → secondary **Collage** (catalog-editor actions, `category === 'score'`) → `/catalog/items/[id]/collage` (`/catalog/items` itself still goes to `[kind]`). `score-stage.svelte` = `MetaPage toon="" fill help="catalog" docs={[neutralDoc(catalogKey('items', id))]}` (Save draft, History, Publish, autosave, leave guard, Ctrl+S / Z), crumbs via `usePageNav`; the file signed with `authApi.scoreFile(id, 'musicxml', part)`; registry + moods docs for character rows.
- Main: a square Segmented `smartphone` / `monitor` → `score-frame.svelte`: the Reader dock's screens (390 × 844 / 1440 × 810 CSS px – simulated screens, `SCREENS` moved to `lib/editor/screens.ts`) scaled to the column with `transform: scale(k)` – the paper is inside the group, so blends equal the reader. Inside: `<ScoreView embedded>` with its own header, zoom ± and page footer. Its sheet (= Verovio's page) is the screen minus that header and footer, sized by rem tokens – and the creator's root is not the reader's (16.8px from 768px; the reader 14px on a phone, 24px on a desktop) – so the frame sets the reader's tokens for that screen in its px (`readerSizes(SCREENS[screen])`: spacing, type, `--control-height` with the phone's 44px touch floor; `SCREENS` carries `root` + `touch`): header / sheet / footer 67 / 710 / 67 on the phone, 90 / 630 / 90 on the desktop – the reader's, so its page breaks are too (`check:creator-collage` compares them with the viewer's own score view). An embedded view's Play label goes by its own width (`@container`), not the creator's window. Zoom per frame: 30 / 40 (`width / root < 40`, as the reader's start zoom).
- `decor` → `score-overlay.svelte` per page: boxes with the same `pieceBox()` CSS as the pieces, `pointer-events:auto; touch-action:none` only on boxes (empty paper still swipes). Move = re-anchor on every pointer move (`anchorScorePiece`, one checkpoint per drag; pointer % from the overlay's `getBoundingClientRect`, so the frame's scale cancels out); the layer clips to the page (`overflow: clip`: a box over the page's edge made the leaf scroll – its scroll bar narrowed the leaf and the pieces' frame with it, every piece off its note, unlike readers'; review fix 2026-10-18); a round handle on the box's corner (`pieceHandle`, held on the page at its edges, the drag keeping its offset from the corner) scales `rect.width`; Delete / Backspace removes (outside fields), Esc deselects – on the live region only (`use:region` `score-stage`, the focus on the stage or nowhere: with a sidebar link or a dock focused they do nothing). Rotation in the inspector.
- Inspector region (Divider + aside, `panelWidth('sc-panel-width:score-stage', 22)`, below the stage under 1280 px and on phones): `collage-list.svelte` (over pieces front first, a rule, behind pieces; drag within a side; ⚠ on a missing bar; a click selects and `follow` turns to its page) + `+ piece` (lands at the centre of the current page, width 12 sp) + `PieceInspector host="score"`. The first piece writes `collage = { file: <part musicxml>, pieces: [] }`; the last removed takes `collage` away.
- `validate` = collageProblems + file check + missing bars (from `onlayout`).
- As built: the edits in `lib/editor/score-collage.ts` (`stagePart`, `ensureSheet`, `newScorePiece` / `addScorePiece`, `centrePlace`, `movePiece`, `cornerWidth` – the corner's distance from the centre is the half diagonal, `removeScorePiece`, `setPiecePage` – Bar / Page keeps the piece where it is on its page, `collageRows` / `moveCollageRows`, `missingBars`, `fileChanged`, `stageProblems`); the pure half of the score collage is `@scutoons/reader/score-anchors` (no Svelte, no Verovio – node tests); the score view's labels in the creator's language (`collage-words.ts`); the frame carries the reader's sizes for its screen (its header / footer, so its page breaks – review fix 2026-10-18: a phone's 2.75rem at the creator's root was 46px, not 44, the desktop's control height the creator's: 697 / 680px sheets instead of 710 / 630, Like a Baby 39 pages instead of 35); Verovio's page is rounded down to the sheet (`scoreOptions`: a page a fraction taller drew its SVG 1px over the sheet and scrolled – readers' phones too). An item only in the catalog table (no document yet – the local score items, an admin write or a migration) starts its document from the table's data (`loadCatalogDoc`, `ContentDoc.startFrom`: unchanged until edited, the first save its first version) – in the catalog editor too. `check:creator-collage` (CDP, 35 checks, the saved versions deleted afterwards): Collage under More, each screen's score view as high as the reader's own (the viewer's lab: header, sheet, footer, the Play label, the page fitting its sheet), a disc dragged onto bar 10 stays on its staff lines through Phone → Desktop → zoom 45 / 50, a character portrait on bar 12 staff 2 at zoom 30 / 35, Negative's pixels, the corner scale, a piece over the page's bottom edge (the leaf does not scroll, the layers as wide as the music, the handle on the page and scaling from there, dragged back onto its note), Behind, Page, a missing bar, save / reload, the keys on the live region only (a sidebar link focused: Backspace / Delete / Esc leave the piece), Delete + Ctrl+Z, a drag held still half-way = one undo step, 375 px.

## 5. The picker – `collage/collage-picker-sheet.svelte`
Creator `PickerSheet` (Sheet right, widthKey `picker`), Tabs **Shapes · Characters**, a secondary **Image …** in the filter row hands over to `AssetPickerSheet` (kind image; the WebP rendition of a PNG + WebP twin preferred).
- Shapes (`shape-grid.svelte`, list cached in `lib/editor/shapes.svelte.ts`): `assetsApi.list({ kind: 'shape', folder: 'catpope', page_size: 500 })`, every page (`allPages`: the server gives 500 at most, the next pages until `pagination.total`), once per session – when the picker first opens (`active`; a failed load tries again on the next opening), or when Layers / the score list hold a shape (its title is the piece's name); `.sc-picker-filters` = search (title, slug, tags) + a tag select by frequency (tags, not classes); `.sc-cards` with `gridArrows`, the `.sc-checker` background, mask shapes as an ink silhouette; a whole-tab empty library = one `.sc-empty` line. This is VECTORS §6 step 8's Shapes picker.
- Characters: `character-picker-sheet.svelte`'s body **moved** into `characters/character-grid.svelte` (the sheet = PickerSheet + CharacterGrid, same API for cast-editor and character-ref-field). A mood cell resolves `figureImage(char, { persona, look, mood, view: 'portrait' })` (collage-pick.ts: the fallback rule, but '' instead of the "?" substitute – a character with no picture, or only a known-missing one, adds nothing and a warning toast says so); the view is the inspector's Picture row. No cast filters here (`filters={false}`: neither "has missing states" nor "every mood of the vocabulary" – they led only to moods without a picture); a pick counts only while the sheet it came from is still open.
- Returns `PickedPiece { src /* CDN root */; ratio?; figure?; name }` – ratio from AssetSummary width / height, else `imageRatio(url)` (decode). The host converts the path (slices `toonAssetRef`, scores as is).
- As built: `lib/editor/collage-pick.ts` holds `PickedPiece`, `shapePick` / `imagePick` (the WebP rendition of a PNG / JPEG), `shapeTags` / `filterShapes`, `imageRatio` and `pickedToPiece(picked, toon, place)` (toon '' = a score's root path); a one-colour shape shows as `filter: brightness(0)` on its lazy `<img>` (masks don't load lazily); **Image …** sits behind the tabs (Tabs `after`), so both tabs reach it; a character pick never adds to the cast.

## 6. Rendering – `shared/reader/src/collage/collage-piece.svelte` (`CollagePieceView`, props `piece, src, left, top, width, z?`), builders in `collage-css.ts` (tested). No inline SVG: no `{@html}`, no id collisions. Colours are #rgb / #rrggbb at every guard (the schemas' `PieceColor` pattern, `CatalogValidator::collage`) and the builders drop anything else (`isPieceColor`, finite numbers only); the blend only from `BLEND_MODES`.
```
.piece  left/top/width % · aspect-ratio: ratio · translate: anchorTranslate · rotate · mix-blend-mode · opacity · container-type:inline-size · pointer-events:none · z-index (scores)
  .fx   inset 0 · filter: outline ×4 → shadow → blur   (cqw of the piece)
    PAINT = <img loading=lazy decoding=async draggable=false object-fit:contain>  |  .fill { mask: var(--src) center/contain no-repeat (+ -webkit-mask); background: colour | linear-gradient }
progressive blur – two bands instead of one .fx, both padded (a mask cuts at its box – also the sharp band's outline / shadow):
  .band (sharp)  inset −P cqw, mask linear-gradient(θ, #000 s%, transparent e%) remapped to the padded box > .fx at inset P cqw (outline, shadow)
  .band (soft)   the same box, the inverse mask, mix-blend-mode: plus-lighter > .fx at inset P cqw (outline, shadow, blur)   P = spill = outline + shadow (max |x|,|y| + 1.5 × blur) + 3 × amount
  .piece.ramp    isolation: isolate – the bands add (1 − t + t = 1) only with each other; over each other they'd give 1 − t + t² (75 % mid-ramp)
```
| Effect | Technique |
|---|---|
| Blend (16) | `mix-blend-mode` on `.piece` (an HTML box – Firefox, the owner's phone, never blends SVG children) |
| Opacity | `opacity` on `.piece` |
| Fill colour / gradient | the file's alpha as CSS mask on `.fill`, background = colour or one `linear-gradient` across the whole shape; URL through `style:--src` (Windows drive colon, as score-illumination) |
| Outline | 4 chained zero-blur `drop-shadow(±w cqw …)` on `.fx` (dilation; boxy corners on thick outlines) |
| Shadow | `drop-shadow(x y blur rgb(r g b / a))` after the outline – the outlined shape casts it; turns with the piece |
| Blur | `blur(a cqw)` last on `.fx` – never on the masked element (filter runs before mask) |
| Progressive blur | two bands; `blurBands(style, ratio)` maps from / to into the padded box (`gradientLine`): θ = atan2(dx·w, −dy·h), L = abs(w sinθ) + abs(h cosθ), stops = projections; the soft band added (`plus-lighter`) in the isolated piece. As built: identical in Chromium / Firefox / WebKit (probe: an opaque `fade` disc stays opaque down its middle, a `negative` ramp inverts there); WebKit rounds drop-shadow lengths to whole px |
| Rotation | `rotate` on `.piece`; shadows rotate with it |

**Limits** (validation warnings, not caps): slice 12 pieces / heavy weight 4; score 60 / 20; outline ≤ 8 %, blur ≤ 20 %, shadow blur ≤ 25 %. Pieces are static, mount near the viewport (slices) or on rendered pages (scores). WebKit software probe: blend 1.6×, drop-shadow + blend ~7×, blur ~5× a plain piece – phones still need an on-device check (owner's Firefox Android + an iPhone).

## 7. The piece inspector – `collage/piece-inspector.svelte` (+ `look-grid.svelte`, `look-preview.svelte`, `preset-line.svelte`, `look-fields.svelte`, `place-fields.svelte`, `gradient-field.svelte`, `points-field.svelte`, each ≤ 300 lines)
Props `doc, path, host: 'slice' | 'score', registry, moods, presets?, onremove` (`presets` = the `colorations` document, §12.4). Like the pilot dialog-inspector: head `h3` "Shape / Character / Image · name" + secondary **Remove**; the preset grid (`look-grid.svelte`, label Preset / Vorlage): As drawn, COLLAGE_LOOKS (labels `creator.look_<id>`), then the custom presets – each tile shows the selected piece's own file in that look (§12.4 "The grid") – a pick writes `preset` and drops `style` in one `doc.edit`; a re-click on the chosen look takes the piece back to it (its own values go; without any it is no edit); under it the preset line (Save as preset … · Update preset · •••, each only where it applies – §12.4); then `<details class="advanced">` (open when the piece has its own `style`). Plain fields `doc.set(path, v)` (undefined = default, coalesced per path: one scrub = one undo step); effect fields through `setEffect()` (collage-edit.ts: copy-on-write from the preset, clearing writes `null` only when the preset has that key). Placeholders show `pieceStyle()`. Every control `--control-height`, ≤ 3 per line.

| Row | Controls |
|---|---|
| Picture (figure; outside the fold) | PropSelect icon `smile` mood (the look's moods) · line: Segmented stretch avatar / portrait / full – one edit re-resolves `src` (`figureImage`) and `ratio`; nothing resolves → the piece stays, a warning toast |
| Position – slice | AnchorField bare · ScrubNumber tag X % · tag Y % |
| Position – score | Segmented square `music` / `file` (bar / page) · line: text input bar (#) · ScrubNumber tag St (staff, placeholder 1); line: tag X (% of the bar / page) · tag Y (sp / %). No anchor picker on scores |
| Size | ScrubNumber tag W (% / sp, min > 0) · ScrubNumber icon `rotate-cw` ° |
| Side (score) | Segmented stretch Over / Behind (fallback Over) |
| Blend | ScrubNumber icon `eye` opacity % (stored 0–1) · line: PropSelect icon `blend`, 16 modes (`creator.blend_<mode>`; its own line – the long names) |
| Fill | Segmented square none `slash` / colour `droplet` / gradient `gradient` · line: ColorField, or GradientField = ScrubNumber `compass` angle ° beside the kind · line: ColorField from · ColorField to (`.sc-fields data-cols="fit"`: half each where a whole #rrggbb fits, else a line each); a gradient from a light colour (luminance > 0.5) runs to ink, else to white |
| Outline | ColorField (placeholder none) · ScrubNumber icon `outline-width` % (only while a colour applies) |
| Shadow | ColorField (placeholder none) · ScrubNumber `eye` % · line (with a colour): tag X · tag Y · icon `blur` |
| Blur | ScrubNumber icon `blur` % · Segmented square even `circle` / ramp `gradient` · ramp: PointsField = lines [X1 · Y1] [X2 · Y2] (+ the canvas dots on slices) |

New icons in `shared/ui/src/icons-extra.ts` (Feather manner, 24 grid, strokes): `blend`, `gradient`, `shadow`, `blur`, `shapes` (the piece tool, a piece's Layers glyph). GradientField / PointsField live in the creator (one user); they move to @scutoons/ui when a second inspector needs them.

As built: the edits in `lib/editor/collage-edit.ts` – `setPreset`, `setPlain` (blend, opacity), `setEffect` (a whole value; clearing = `null` when the look has the key, else no key; an emptied `style` goes), `editEffect` (copy-on-write of the drawn effect; an effect that is off starts from what its field just cleared, else the look's, else `EFFECT_DEFAULTS`), `setEffectColor` (an outline's / shadow's ColorField sends `undefined` on every keystroke of an emptied text: empty = off and the inspector keeps the effect as drawn, per piece and look – a colour retyped brings it back with its width / offsets / blur / opacity; the fill's ColorField ignores an emptied text, only the kind None clears a fill; ColorField shows the kept value again on blur), `setFillKind` / `setBlurRamp` (a fill switched off over the look comes back as the look's when that kind is asked for, else from the look's colour; the kind a fill / blur already has is no edit – Segmented sends the chosen option again, a re-click must not rebuild a gradient, reset the ramp points or copy the look's value), `setFigure` + `figurePath` (a mood / view re-pick waits for the picture's ratio: it lands on its piece by id, never on whatever the inspector's path shows by then; a newer re-pick of the same piece carries the older one's changes), `layerRows` / `canMoveLayerRow` / `moveLayerRows`; words in `collage-words.ts`. The effect fields show the piece **as drawn** (`pieceStyle`) as their value, not as a placeholder – so a look's outline / shadow can be cleared (→ `null`); plain fields keep own value + the look's placeholder. The head's name is `pieceLabel` (a shape's library title, a character's name · mood). Side labels Over / Behind (the long words as tooltips). Slice pieces' AnchorField needs the canvas' drawn height (step 4 `canvas-pieces.ts` writes `drawnHeights`).

## 8. Build (the night of 2026-10-18) – done
Built in the planned order core → slices → scores, one step at a time on `main` with its own paths (pushed by the owner on 2026-10-18); nothing from the cut order was cut (the canvas' blur dots, Shift+V, the **Image …** hand-off, the stage's scale handle are all in). Where it lives:
- **Model** – `@scutoons/content` collage.ts (types, `COLLAGE_LOOKS`, `pieceStyle`, `collageProblems`), layers.ts (pieces first), validate.ts / containers.ts (slice + score checks), the schemas (both copies).
- **Readers** – `@scutoons/reader` collage/ (collage-css.ts, `CollagePieceView`, `SlicePicture`), score/ (score-layout.ts, score-bars.ts, score-anchors.ts – also `@scutoons/reader/score-anchors` – score-pieces.svelte, ScoreView's props); viewer toon/slice.svelte, items/score-actions.svelte; server `ProtectedFiles::score` (the pieces with the MusicXML), `CatalogValidator::collage`.
- **Creator** – components/collage/ (the picker, `ShapeGrid`, `PieceInspector` + its fields, the score stage: score-stage / score-frame / score-overlay / collage-list), characters/character-grid.svelte, slice/layer-list.svelte + canvas-point-handles.svelte; lib/editor collage-edit.ts, collage-pick.ts, collage-words.ts, shapes.svelte.ts, canvas-pieces.ts, score-collage.ts, screens.ts; the route `/catalog/items/[id]/collage`.
- **Checks** – tests: content collage / layers / validate / containers, reader collage-css / score-anchors / score-reflow, creator collage-edit / canvas-pieces / score-collage / clip / canvas-geometry; browser: `npm run check:creator-chapter -- --pieces` (the slice editor's pieces, pixels included) and `npm run check:creator-collage` (the stage on cloud9_en, 35 checks); both save drafts and restore them.

The owner opens in the morning: creator → a chapter → a slice → the piece tool; creator → Catalog → Items → Cloud 9 → More → Collage.

## 9. Deferred
Pieces over the lettering or reaching into the neighbour slice (a front plane after `.slice-layers` with z `var(--plane,1)` – no data change); pieces on video slices; mirroring; round outlines (SVG filter); radial gradients, > 2 stops in the UI, a canvas gradient gizmo; tint / duotone fills; the Images tab inside the sheet; lookahead prefetch of piece files; effect thinning on the `low` media quality; on-device measurements (the owner's Firefox Android + an iPhone); the stage: all pages stacked, rotate handle, copy / paste, nudge, phone bottom sheet, drag across pages, a part switch, signing a draft-only score file; re-resolving figure pieces when the registry changes (`src` is frozen; the Picture row re-resolves); MCP `search_assets` kind `shape` (its folder `catpope/vectors` finds them today); freezing a lab riot page into pieces.

## 10. Owner questions (built with the default – FEATURES 10.105)
1. ~~**Where do your own looks live** once "Save as preset" exists?~~ Answered 2026-10-18 with the default ("Next: 'Save as preset' – my own colourations as named presets … applied to any piece"): one platform-wide document `colorations`, edited from the piece inspector, shared by every toon's slices and every score – built, §12 (its own questions: §12.8).
2. **Slices: always on the picture?** Pieces lie under every text box, item and tap area, clipped to the picture – where every blend reaches the art and looks the same on every screen. Over the lettering, or into the neighbouring slice? *Default:* not now – a layer over the lettering can come later without a data change.
3. **Scores follow the music?** A piece sticks to a bar + staff, moves with that bar to whatever page and scales with the zoom; "Page" makes it a background on the page that holds the bar. *Default:* music, with the Bar / Page switch in the inspector.
4. **One collage per file:** the part readers open (the choir part when there is one, else the full score) of that item – so one per language edition? *Default:* yes; a part switch and copying a collage to another edition later.
5. **Rotate? Mirror?** ("no turning, no distortion" was said of the random lab collage.) *Default:* rotation yes (the canvas handle + a field), never stretched (only the width is stored), no mirroring.
6. **Effect sizes:** outline, shadow and blur relative to the piece's width, or a fixed thickness? *Default:* relative – the same look on phone and desktop.
7. **Blend modes:** the 16 standard CSS modes (the same in Chrome, Firefox, Safari) – plus-lighter as a 17th? *Default:* the 16.
8. **Readers' engraving:** the plain engraving of today, or the lab's Collage look? *Default:* plain – the stage shows the readers' engraving; the lab looks stay in the lab.
9. **Sensitive slices:** while a reader's picture is blurred, its pieces hidden or blurred too? *Default:* hidden until the reader shows the slice.
10. ~~When do the shapes go to production?~~ Answered 2026-10-18 ("import the 436 vectors to production"): they are there (research/VECTORS.md step 5). Pieces reach readers once `main` is pushed and a chapter / score with pieces is published.
11. **Video slices** (a blend over a playing video costs every frame)? *Default:* not now – pieces show on picture slices only; a video slice with pieces gets a warning.

## 11. Risks
- **Isolation is load-bearing:** any z-index / transform / opacity / filter / will-change / isolation / container-type on `.slice-pieces`, `.score-pieces`, `.pages` or `.leaf` silently changes blends – CSS comments + the pixel assertions in check:creator-chapter / check:creator-collage.
- **Verovio's SVG structure** (`g.measure[data-n] > g.staff[data-n] > 5 path`, `g.note > g.notehead > use`) and the MusicXML import's `dur.ppq` are pinned by the fixture and the live reflow test; a Verovio upgrade re-runs them.
- **Renumbered bars** move pieces musically; only a replaced file warns.
- **The stage signs the published file**; a draft's new file needs publishing first (production score items that were never published can't open in the stage yet).
- **ScoreView is live for readers**: changes stay additive; check a plain score and the lab looks by eye.
- **Phone cost** unmeasured; limits are warnings. Outline corners at thick widths; progressive blur is a crossfade (2× cost).
- **A piece's file must exist where readers look**: the 436 shapes are on production since 2026-10-18 under the same paths; a piece on an image or a character picture made only locally (local media is a dev copy) shows nothing there.
- **Two editors on one item** (catalog form + stage in two tabs) → the existing save conflict.

## 12. Presets (owner 2026-10-18)

Owner 2026-10-18: “Next: 'Save as preset' – my own colourations as named presets (COLLAGE.md §10 question 1, default: one platform-wide document), applied to any piece.” This answers §10 q1 with its default. Built 2026-10-18 in §12.7's six steps; below is how it is, the build's changes corrected in place. On the grid's first look the owner asked for previews (2026-10-18): “man könnte in dem Wort auf dem Selector zeigen, was das dann wird, wie das aussieht, dass man eine visuelle Vorstellung davon hat” – every tile shows its look on the piece itself (§12.4 “The grid”).

### 12.1 The decision
- **One platform-wide document, `colorations`**, language-neutral, holding `{ presets: CollagePreset[] }`. It is stored, versioned and published like `moods`. A preset is an id, a name and the six style keys. A piece names a preset in `preset` and can fine-tune it in `style`. Once published, a change to a preset restyles every piece that names it, on every toon's slices and every score (§2 “Presets by reference”).
- **The ten code looks stay in code and cannot be changed** (collage.ts COLLAGE_LOOKS, with their translated names, testids and tests). Custom presets share the code looks' id namespace, but a custom id can never be a code look's id or `none`. COLLAGE_LOOKS is final: any new look is a custom preset.
- **Readers never resolve a custom preset. The server bakes it** into the two answers readers already fetch: the chapter (Library::chapter) and a score's signed URL (ProtectedFiles::score).
  - A piece that names a custom preset arrives with `style` set to the piece as drawn, and without `preset`.
  - Baking happens per answer, not at publish. So publishing the presets restyles every published piece without republishing any chapter or item.
  - There is no new route and no new request, so “one request per chapter” (RULES) still holds.
  - Nothing changes in the viewer, shared/reader or the offline rules. Old PWA builds and saved chapters need no change either.
- **The creator draws from documents**, so it bakes the same way, using the newest draft.
  - The TS function is `bakePresets`; its PHP twin is `CollagePresets::bake`.
  - One golden fixture keeps the two equal; both test suites read it (precedent: ExprTest.php:13-18).
- **In the piece inspector**, the preset grid previews every look on the selected piece; custom presets follow the ten. Under the grid is one line with Save as preset … · Update preset · ••• (Rename … / Delete …). Each control shows only where it applies.

### 12.2 Data
shared/content/src/collage.ts (exported through index.ts):
```ts
/**
 * a custom preset's id: a slug, stable forever (pieces name it; a rename changes `name` only) – never a code look's id or
 * 'none' (the “As drawn” option's testid)
 * @pattern ^(?!(?:ink|red|cyan|negative|burn|acid|sticker|glow|ghost|fade|none)$)[a-z][a-z0-9-]{0,39}$
 */
export type PresetId = string;
export const PRESET_ID = /^(?!(?:ink|red|cyan|negative|burn|acid|sticker|glow|ghost|fade|none)$)[a-z][a-z0-9-]{0,39}$/;
/** a custom colouration (“Save as preset”) – drawn like a code look */
export interface CollagePreset extends PieceStyle {
	id: PresetId;
	/** its name in the creator – one string, readers never see it */
	name: string;
	/** deleted: no longer offered, still drawn by every piece that names it; its id stays taken */
	removed?: boolean;
}
/** entity `colorations` – platform-wide, language-neutral; in the grid's order (newest last) */
export interface Colorations extends AcceptedWarnings {
	presets: CollagePreset[];
}
export const emptyColorations = (): Colorations => ({ presets: [] });
```
- **How a preset id resolves (the same rule in TS and PHP).**
  - The *index* is the document's entries whose `id` matches PRESET_ID. If an id repeats, the first entry wins. Deleted entries still count.
  - `customPreset(id, presets)` is the index entry; `collageLook(id, presets?)` returns the code look if there is one, else the index entry. Code looks come first, so data can never redefine a built-in. A code look's id fails PRESET_ID anyway.
  - `pieceStyle(piece, presets?)` keeps its merge. It reads each of the six keys before its own-property test: on a Svelte `$state` proxy `Object.hasOwn` does not track a missing key, so a key added later (an opacity on a piece with a moved blur ramp, a key an Update gives a preset) never redrew the canvas or the stage (found in step 4).
  - `STYLE_KEYS` (:172) gets exported as `PIECE_STYLE_KEYS`.
- **`bakePresets(pieces, presets)`.**
  - A piece whose `preset` is in the index gets `style` set to a JSON copy of `pieceStyle(piece, presets)`, with nulls dropped and no key at all if nothing is left. The piece loses `preset`.
  - Every other piece passes through unchanged. If no piece changed, the same array comes back.
  - Invariant, tested over the fixture: `pieceStyle(baked) ≡ pieceStyle(piece, presets)`.
  - `preset` is removed so that a code look added later cannot leak into baked art.
- **The key is `colorations`**, the name COLLAGE.md, VECTORS.md and DECISIONS 2026-10-17 already use. `looks` already means a persona's looks, and `presets` the text presets.
- **No migration:** `kind` is a VARCHAR and nothing is stored yet.
- **Registration – these change together:**
  - entities.ts: EntityKind, SHAPES `[false, false, false, false]`, `entities.colorations`, and the header's platform-wide line (the MCP entity-keys resource prints it).
  - EntityKey::KINDS (and its comment) and ContentSchemas::BY_KIND `'colorations' => 'colorations'`, both directly after `map` (ContentSchemasTest:46-52 compares the key order).
  - ContentFiles PLATFORM_KINDS (:35, for `scutoons/_/colorations.json`) and the import help.
  - schema.js SCHEMAS `colorations: 'Colorations'` (both copies).
  - With the schema in place, the save guard refuses a bad colour, blend, number or id, a code look's id included, with no PHP logic. Repeated ids are an error in colorationProblems.
- **Who may change presets:** role creator saves. Platform-wide keys publish without review (Reviews::applies, Reviews.php:68-71). AI tokens save drafts only.
- **Delete always leaves a tombstone** (`removed: true`; values and id kept), even for a preset that was never published.
  - Why: a preset is a value inside one document. Removing it would free its id while draft pieces may still name it, and a later preset with the same name would silently take those pieces over.
  - This departs from DECISIONS 2026-10-12 “deleted when never published”, which was made for kinds stored one document per entry. See owner question 1.

### 12.3 Delivery – baked by the server
New `server/src/Domain/Content/CollagePresets.php`:
```php
final class CollagePresets
{
    public const KEY = 'colorations';
    /** = @scutoons/content PRESET_ID (CollagePresetsTest): the code looks' ids and 'none' never match; `D` = JS's `$` (end only, no trailing newline) */
    public const PRESET_ID = '/^(?!(?:ink|red|cyan|negative|burn|acid|sticker|glow|ghost|fade|none)$)[a-z][a-z0-9-]{0,39}$/D';
    /** = collage.ts PIECE_STYLE_KEYS (parity test) */
    public const STYLE_KEYS = ['blend', 'opacity', 'fill', 'outline', 'shadow', 'blur'];
    /** id → entry (PRESET_ID ids, the first of a repeated id, removed ones too) @return array<string, stdClass> */
    public static function index(mixed $doc): array;
    /** bakePresets: pieces as stdClass (chapters) or assoc arrays (catalog rows) – new copies, the input untouched */
    public static function bake(array $pieces, array $index): array;
    /** a copy of a chapter answer, groups[].slices[].pieces baked – an explicit walk (pickup's `pieces` is a count, content.ts:368) */
    public static function chapter(stdClass $chapter, array $index): stdClass;
}
```
- **Merging, per key:**
  - If the piece has its own key, use it. Test with `property_exists` / `array_key_exists`, never with `isset` or truthiness: opacity 0 is a real value, and an own `null` means the effect is off.
  - Otherwise use the preset's value.
  - Drop nulls. Copy values with a JSON round trip, in the piece's own form.
  - Then unset `preset`, and unset `style` if it is empty (an empty assoc `[]` would encode as a list).
- **Copy, never mutate.** Snapshot::doc caches decoded documents (Snapshot.php:107-116), withScenes clones only the chapter (Library.php:786-808), and NeutralDoc::slice passes `pieces` through by reference (NeutralDoc.php:180-202).
- **Chapters:**
  - Library::chapter (:355) returns `CollagePresets::chapter($this->withScenes(…), CollagePresets::index($ctx->snapshot->doc(CollagePresets::KEY)))`.
  - With an empty index the chapter comes back unchanged. When no document is stored, there is no query.
  - Readers get the published presets. Draft mode (`?draft=1`, used by the Reader dock) gets the newest draft. The Reader dock already reloads after a platform-wide save (reader-dock.svelte:60-67).
  - Snapshot::toon reads `toon = ? OR toon IS NULL` (Snapshot.php:32-35), so the fingerprint – cache key and ETag – already changes when the presets are published. The answer is `public, max-age=60` (LibraryController.php:139).
  - No FORMAT bump: until the first save the bake changes nothing.
- **Scores:** ProtectedFiles::score (:44) bakes `collagePieces()` (:66) with the published document, only when there are pieces.
  - It needs a new `EntityStore` dependency (autowired), for `published()` (EntityStore.php:63-66).
  - The answer is no-store (CatalogController.php:54, :66).
- **Offline:** a saved chapter is the chapter answer (ChapterFiles.php:54), and its ETag is the saved version (:120-121).
  - So publishing the presets flags saved chapters, and their documents download again; unchanged media is kept. A characters or map publish does the same today.
  - Presets carry no media, so ChapterFiles, saved-rules.ts and the service worker stay unchanged.
- **The answer contract changes:** a piece that names a custom preset reaches readers without `preset`. API.md and DATA.md must say so.

| the piece names | readers | creator |
|---|---|---|
| a code look | as today – the renderer resolves the ten | the same |
| a published custom preset | baked | the newest draft's values |
| a draft-only preset | as drawn + own values (the Publish lists offer the presets, §12.4) | drawn with it |
| a deleted preset | baked from the tombstone | drawn; option “Night (deleted)”; a warning |
| an unknown id | as drawn + own values | a warning once the presets are loaded |

### 12.4 Creator
- **The document:** `lib/editor/presets-doc.ts` holds `presetsDoc()` = `neutralDoc(entities.colorations)`, `ownPresets(doc)`, `judgedPresets(doc)` (undefined while it loads or after a failed load – then no custom id is judged) and `ensurePresets(doc)`, following the moods precedent (characters.ts:49, :59-64).
  - DocKind gets `'colorations'` plus a kindOfEntity case (doc.svelte.ts:38, :77-115). Without that case the kind falls through to 'group' and runs assignEventIds.
  - Labels: “Collage presets” / “Collage-Vorlagen” in entity-label.ts (EntityLabels, ENTITY_LABELS, NAMED) and in the creator's and the admin's labels.ts.
- **Chapter editor:**
  - It loads the document in onMount's Promise.all (:291).
  - `allDocs` includes the document while it is dirty (the worldMap precedent, :261-267), so Save, autosave, the leave guard and Problems cover it.
  - The problems context gets `presets: judgedPresets(colorations)`.
  - SliceCanvas gets `presets` (the list); SliceInspector passes the document on to PieceInspector.
- **Score stage:**
  - It loads the document in onMount (:46-49).
  - `MetaPage companions={() => [presets]}` saves it when dirty and puts its key into the 'files' Publish scope (doc-session.svelte.ts:123-129).
  - ScoreView gets `pieces={bakePresets(pieces, own)}` (:204). ScoreOverlay and CollageList keep the stored pieces.
  - `validate` (:163) passes the presets to stageProblems, and adds validateFile('colorations') while the document is dirty.
  - The screen block moved to lib/editor/screens.ts (`firstScreen` / `pickedScreen`), so score-stage.svelte stays at 300 lines.
- **Drawing and fields:**
  - slice-canvas computes `drawn = $derived(bakePresets(slice?.pieces, presets))` and gives it to SlicePicture (:603) and pieceItems (:158). The indices stay the same, and the ramp dots read the baked blur, so canvas-pieces.ts needs no change.
  - slice-canvas's moveRamp (:285-288) passes `presets` to editEffect.
  - In collage-edit, setEffect, lookValue, editEffect, setEffectColor, setFillKind and setBlurRamp (:77-177) take a trailing `presets` parameter, so copy-on-write starts from the custom preset.
  - look-fields :20 and :22 read `pieceStyle(piece, presets)` and `pieceStyle({ preset }, presets)`.
  - Copy with JSON only: the documents are $state proxies.
- **Publish:**
  - The gap today: the 'chapter' scope admits only chapter and scene keys. pendingFiles reads only `contentIndex(scope.toon)` (publish-tree.ts:38-53, :79-83), which never contains platform rows, because EntityStore::index with a toon is `WHERE toon = ?` (EntityStore.php:341-351).
  - Fix: a new `PLATFORM_ALONG = ['colorations']`.
    - inScope admits it for the 'toon', 'season' and 'chapter' scopes, and pendingFiles then also reads `contentIndex('')` for those scopes.
    - KIND_ORDER (:59) gets it after `character`.
  - The chapter, season, toon and score Publish lists then show “Collage presets” whenever it has a draft, ticked like every line. No level can ship pieces whose preset is still a draft (owner question 6).
- **The grid** (`collage/look-grid.svelte`, replacing the Segmented):
  - Its label changed from “Look” to “Preset” / “Vorlage”: in DESIGN.md's wording table, “Look” means a character's look.
  - Options: As drawn, the ten, then `presetChoices(own, piece.preset)`. These are the live custom presets in stored order, plus the deleted preset this piece names, shown as “Night (deleted)”, struck through.
  - Each custom option's label and `title` are its name; its testid is `piece-look-<id>`; the chosen tile is `aria-pressed`.
  - Picking runs `setPreset` as before; a re-click while the piece has its own values goes back to the preset.
  - **Previews** (the owner's request, §12 intro): each tile is `look-preview.svelte` over its name – the selected piece's own file in that look, drawn by the readers' `CollagePieceView` (a custom preset baked with `bakePresets`, as the canvas draws it), centred and fitted into a 3:2 `.sc-checker` tile that is its own blend group, the ratio kept; the effects are % of the piece's width, so the thumbnail reads like the canvas; As drawn = the file plain; the same URL the canvas loaded.
  - Tiles of at least 4.5rem (`--seg-min`): four a row in the 22rem panel, five on a 375px phone; As drawn + the ten take three rows, as tall as the old two-column text grid; ≥ 44px on touch. The name runs the tile's whole width (its transparent border included), at most 2 lines – “Wie gezeichnet” fits four a row while the panel shows its scroll bar.
  - From the 6th live custom preset on (`GRID_PRESETS = 5`, measured: As drawn + the ten + 5 = four rows at both widths; this section had guessed the 7th), all custom presets move out of the grid into one fold under it, “Custom (n)” / “Eigene (n)” – a `<details>`, not a PropSelect: a native select can't show the previews.
    - Its tiles are drawn only while it is open; it opens on a piece that names one of them and when the presets move into it; shut, its summary names the chosen one.
    - The re-click rule holds there too.
- **The preset line** (`collage/preset-line.svelte`; pure logic in `lib/editor/collage-presets.ts`; dialogs and toasts in `lib/editor/preset-actions.ts`):
  - It sits under the grid, outside Advanced: secondary buttons at `--control-height`, wrapping, sizes in rem only.
  - Each control is hidden – never disabled – where it does not apply, so most pieces show no line at all; there is no line while the presets document loads or failed to load. The controls:
    - **Save as preset …** – when the piece draws differently from what it names, or when it names a deleted preset. `ownDiffers` compares `pieceStyle(piece, P)` with `pieceStyle({ preset }, P)` deeply.
    - **Update preset** – when the piece names a live custom preset and differs from it.
    - **•••** – a round `more-horizontal` button whose `title` is the preset's name, shown when the piece names a live custom preset. It opens a `choiceDialog` titled with the name, offering Rename … · Delete ….
      - choiceDialog is already used for the route start and end. Menu is not used in any app yet.
- **Save as preset …:**
  1. `askDialog`: title Save as preset · field Name · placeholder “Preset {n}” · button Save.
     - The name is trimmed and cut to 40 characters; an empty name takes the placeholder.
     - A name already shown – a live preset's, As drawn's or a code look's, case-insensitive – gets “ 2”, “ 3”.
  2. `presetIdFor(name, stored)`: slugId, made to start with a letter (`p-…`), cut to 40, then `-2`, `-3` past every stored id (deleted ones included) and the reserved ids.
  3. `ensurePresets`, then `presets.edit(addPreset(f, { id, name, ...presetValues(piece, P) }))`.
     - `presetValues` = the drawn values, as a JSON copy with nulls dropped, so a shadow that was switched off stays off.
  4. On the host: `setPreset(t, id)`. The piece now names the preset and loses its own values; its pixels do not change.
  5. Toast “Saved as preset: Night” with Undo. Undo removes the preset and gives the piece back its `preset` and `style`; the first preset's Undo leaves no document behind (nothing to save or publish).
     - Both are found by id: `piecePath(file, path, id)` (collage-edit.ts; figurePath reuses it).
     - This is a forward edit, like the cast toast's Undo (chapter-editor.svelte:182-190).
     - Every presets edit that leaves the document as stored (an Undo, a rename back) also drops its local autosave copy (`settle`): the chapter editor autosaves the document only while it is dirty, so the last dirty copy would come back on a later load as “Collage presets: Recover”, dropping what was made since.
- **Update preset:**
  1. `confirmDialog`: title “Update ‘Night’?”, message “Every piece with this preset – on every toon's slices and every score – changes with it once the presets are published.”, button Update.
  2. `updatePreset(f, id, presetValues(piece, P))` replaces the six style keys and keeps id, name and removed.
  3. On the host: `setPreset(t, id)`.
  4. Toast with Undo, which puts the old values back, by id.
  - Update is the only way to change a preset's values: pick it, fine-tune in Advanced, then Update.
  - There is no usage count, because AssetUsages indexes media paths only.
- **Rename …:** an askDialog filled with the current name – @scutoons/ui's askDialog takes `value` (dialogs.svelte.ts; dialog-host.svelte starts the field with it).
  - The same name rules as Save apply; an empty or unchanged name changes nothing. It writes `name` only (the id stays), with no toast.
- **Delete …:** `confirmDelete` (ask.ts), message “‘Night’ leaves the list – pieces that use it keep drawing it.” It writes `removed: true`; toast with Undo.
- **Each action edits two documents.**
  - Ctrl+Z reaches only the host: the chapter editor undoes `[activeDoc, chapter]` (chapter-editor.svelte:472-474), MetaPage undoes `docs` (meta-page.svelte:87). Undoing there leaves an unused preset behind.
  - The toast's Undo takes back both documents.
- **Phone and desktop:**
  - The slice inspector is the bottom sheet; the score inspector sits below the stage.
  - The preset line has at most three controls and 44px targets. Dialogs are in-page and toasts sit above the bar; the choiceDialog shows over the bottom sheet (step 4's screenshots, en + de, 375 px and the 22rem panel).
  - Nothing goes on the PhoneBar, and nothing depends on hover or long-press.
  - Shift+V and paste across toons work unchanged, because preset ids are platform-wide.
- **Later:** a Universe “Presets” page (order, overview, History). Until then, History and restore of `colorations` go through the MCP (list_versions, restore_version).

### 12.5 Validation
- **`collageProblems(pieces, host, add, presets?)`** (:248):
  - A code look id is fine.
  - With a presets list:
    - an id in the index is fine;
    - a deleted one warns “the preset ‘Night’ was deleted – the piece still draws it; pick another”;
    - any other id warns “unknown preset ‘x’ – the piece is drawn without it”.
  - **Without a list, an id that is not a code look is not judged.** Today :266 warns on every unknown id, which would flag every custom id in validateCatalog and the admin.
  - Weight is counted through `pieceStyle(p, presets)` (:279).
- **Where the list comes from:**
  - `ValidateContext.presets?: readonly CollagePreset[]`, next to `moods` (validate.ts:112); checkSlice passes it on (:746).
  - `validateContainers(catalog, add, presets?)` passes it to checkCollage (containers.ts:157-168).
  - stageProblems gets `presets` (score-collage.ts:243).
- **New `colorationProblems(doc, add)`**, built on styleProblems (:318, whose comment becomes “a piece's own style or a custom preset”):
  - Errors:
    - the document is not `{ presets: [] }`;
    - an entry is not an object;
    - an id fails PRESET_ID (“reserved for a built-in look” for the ten and none) or repeats;
    - `name` is not a non-empty string;
    - `removed` is not a boolean;
    - the style errors.
  - Warnings: two shown presets with the same name; the size warnings.
  - validateFile `'colorations'` (validate.ts:1381-1400) dispatches it. Its callers: the chapter editor's allDocs loop, the score stage's validate, the MCP.
- **Server:** the schema guard only. CatalogValidator::collage keeps `preset` unchecked.

### 12.6 MCP
- These tools work as soon as EntityKey parses the key: read_entity, list_entities (toon “”), search_content, save_draft (drafts only), list_versions, restore_version.
- creatorHref stays null, as for moods.
- validateDoc (tools/mcp/src/server.ts):
  - `colorations: 'colorations'` is in the kind map, so validate and save_draft run colorationProblems with `validated: true` – a code look's id, a repeated id, no name or a bad style is an error, and save_draft refuses it.
  - The chapter / group branch loads `judgedPresets()` next to moods, into `ctx.presets` – like the creator's: the newest draft's list; nothing stored = [] (every custom id is unknown); a failed load = undefined (nothing judged).
- The slices rule (resources.ts) says, in its section “Custom presets (the platform-wide document `colorations`)”:
  - `preset` is one of the ten, or a custom preset of `colorations` (read_entity “colorations”; 404 = none saved yet);
  - a new preset is a save_draft adding `{ id, name, …style }`, with an id that matches PRESET_ID (printed from `PRESET_ID.source`), never a code look's or `none`, stable forever, and with `removed: true` instead of deleting;
  - a change restyles every piece on every toon and score once published; readers get custom presets baked into `style`, stored documents keep `preset`;
  - the member publishes from a chapter's, season's, toon's or score's Publish list; what validate warns about.
  - It ends with one ts block of PieceStyle, CollagePreset and Colorations. The workflow rule lists `colorations` with the platform-wide documents; agents/MCP.md names no platform-wide kinds, so it is unchanged.
- `npm run check:mcp` has a presets case (validate refuses `negative`; save_draft of a live and a deleted preset; list_entities toon “” lists colorations; a chapter check over four pieces – a custom preset and `ink` pass, a deleted one and `nope` warn); it deletes its colorations versions by note, also on failure, and refuses to run over someone's unpublished draft.

### 12.7 Build (one agent per step, in order; on `main`, own paths) – done 2026-10-18
Steps 1–4 went to production with the owner's push that evening; the autosave fix of step 4 (`settle`), step 5 and these docs are on `main`, not pushed. Green at each step: `npm run check` of the touched workspaces (content 311 tests), PHPUnit CollagePresetsTest|ContentSchemasTest|EntityStoreTest|LibraryTest|ProtectedFilesTest (74 tests) + PHPStan, the creator build, `check:creator-collage` 46/46, `check:creator-chapter -- --pieces` (its preset checks; the slice's two old misses), `check:mcp` 48/48 – every saved `colorations` version deleted again.
1. **Model – @scutoons/content:** the types, PRESET_ID, the resolver, bakePresets, validation, the key, the schema and the golden fixture.
2. **Server:** the kind, CollagePresets, the chapter and score bakes, and PHPUnit tests on the same fixture.
3. **Creator, reading:** the document, labels, drawing, fields, validation and the Publish lists. No new controls yet.
4. **Creator, the controls:** custom presets in the grid, the preset line, askDialog `value`, and the browser checks.
5. **MCP:** validation and the slices rule.
6. **Docs:** this section as built; DESIGN, DATA, CATALOG and API; DECISIONS; FEATURES 10.105; HANDOVER.

### 12.8 Owner questions (built with the default)
1. **Deleting a preset that pieces use.**
   - *Default:* always a tombstone. It leaves the list, every piece that names it keeps drawing it, and its id never comes back – even if it was never published. This departs from DECISIONS 2026-10-12.
   - *Alternative:* never-published presets are removed outright, and their id can come back.
2. **The ten built-in looks.**
   - *Default:* fixed in code. Save as preset from one of them makes your own copy.
   - *Alternative:* seed them into the document, so all are editable – but their translated names become plain names.
3. **Where your presets sit.**
   - *Default:* in the grid after the ten, each tile previewing its look on the piece; from the 6th (measured) all of them in one “Custom” fold under it.
   - *Alternative:* always the fold, or always the grid.
4. **Words.**
   - *Default:* the grid's label is “Preset / Vorlage” (not “Look”, which is a character's), the fold is “Custom / Eigene”, and the Publish line is “Collage presets / Collage-Vorlagen”.
5. **Who may make and publish presets.**
   - *Default:* every creator, published without review – platform-wide, like moods and characters.
   - *Alternative:* the owner and admins only, which needs a new server rule.
6. **Publish lists.**
   - *Default:* the chapter, season, toon and score pages list “Collage presets” whenever it has a draft, ticked.
   - *Alternative:* unticked, or listed only where the page's pieces use a custom preset.
7. **Names.**
   - *Default:* one plain name per preset, not one per language – readers never see it.

### 12.9 Risks · not verified
- **A global restyle is the point.** One Update, once published, changes published art on every toon and score, and every saved chapter refreshes. The Update dialog says so.
- **No publishing of single presets.** It is one document; DECISIONS 2026-10-12 moved places and characters to one document each for exactly this reason.
- **The TS / PHP twin.** Two guards keep them equal:
  - the fixture, with these cases: an own `null` over a preset key; opacity 0; a missing or empty `style`; a style that ends up empty; both piece forms; a code id; an unknown id; a deleted preset; a repeated id;
  - parity tests of PRESET_ID and STYLE_KEYS (precedent: BLEND_MODES, CatalogTest.php:129-131).
- **Cache churn.** Every presets publish changes every chapter's fingerprint: the server cache rebuilds and saved chapters refresh. Characters work the same way today.
- **Conflicts.** Two tabs, or a member's Claude, editing `colorations` get the 409 conflict bar on the whole document; “overwrite” can drop someone else's preset.
- **Dev DB hygiene.** Checks that save `colorations` must delete their versions by note afterwards, and must refuse to run while it holds someone's draft (precedent: creator-collage-check.mjs:53-76).
- **Deploy order.** If the creator goes live before the API, saving the new key fails with 400 for a few minutes (deploy.yml uploads one target at a time) – passed: the API and the creator went live with the same push (2026-10-18).
- **Verified (step 2):** opis/json-schema runs the lookahead in `pattern` – ContentSchemasTest refuses `negative`, `none` and `Night`.
- **Verified (step 4):** the grid's threshold (measured: the 6th), the German preset line at 375 px and in the 22rem panel, the choiceDialog over the bottom sheet – screenshots sent to the owner.
- **Not verified:** the previews' cost on a real phone (As drawn + the ten + up to five custom tiles drawn per inspector; the Custom fold's only while it is open); the MCP's presets case ran against the local API only.
