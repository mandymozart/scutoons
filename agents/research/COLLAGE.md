Status: decided design 2026-10-18 (judged "simplest that works" + grafts) – build plan §8 for the night of 2026-10-18; owner questions §10 open, the build uses their defaults; §8 steps 1–5 built (model, renderer + viewer, picker + inspector, the slice editor, score anchoring + delivery), 6–7 open.

# COLLAGE – pieces (vectors, characters, images) on slices and scores

Owner 2026-10-17: "an editor for the scores, so that, even though the scores are dynamically building depending on page size, I can still add these vector graphics on them and blend them … our avatars and portraits and full … placing vector graphics onto the slices … all of the available blending modes … drop shadow … outlines, color fill gradients, and blurring them … with a start and end point." Standing decisions: colour is never stored on a shape – a colouration is a named preset applied to any shape (VECTORS §3); shapes are CDN assets with tags (several per shape); the random lab collage is not the goal; vectors never distorted.

**One piece shape, one renderer, one picker, one inspector, two hosts** (the slice editor that exists, one new score stage page). Less UI is better UI.

## 1. Checked in the code (2026-10-18)
- `claude/editor-inputs-e870d0` is merged into main (ee723dc1, 4740c13c, 80b01804): ScrubNumber, PropRow, PropSelect, ColorField, LinkedPair, Segmented square / fallback, icons-extra.ts.
- Vector library is local: d53a041b (`asset_shapes`, `GET /api/assets?kind=shape`, `@scutoons/api` AssetShape) + d72e88d0 (436 shapes in `media/catpope/vectors/`). Production has none.
- CORS: `access-control-allow-origin: *` for WebP and SVG on the local CDN (:8001) and for WebP on cdn.scutoons.com (curl). CSS masks of character pictures are safe.
- Local score items: 6 published (cloud9_en …), full score only (`?part=choir` → no_part); `GET /api/items/cloud9_en/score/musicxml` signs `scores/cloud9_en/4971f460e393.musicxml` (200).
- Verovio 6.3.0 (`svgAdditionalAttribute: ['measure@n','staff@n']`, `adjustPageHeight:false`): outer `<svg>` = the page in px; `svg.definition-scale viewBox="0 0 pageWidth×10 pageHeight×10"`; `g.page-margin transform="translate(mx,my)"`; `g.measure[data-n] > g.staff[data-n]` whose first 5 `<path d="M x1 y L x2 y">` are the staff lines, 180 units apart.
- `.slice-layers` is `position:absolute; z-index:1` (slice-layers.svelte:80) – a stacking context; nothing inside blends with the picture. fx `inFrame` animates every child of `.slice` separately (cues.ts:228).
- `ProtectedFiles::publicItem` rebuilds `score` as an allowlist – "the public catalog never names their paths". `ScoreField`'s `set` / `setChoir` spread keep extra score fields.
- Any `character` key is walked by `ChapterFiles::characterRefs` (saves the whole persona offline) and `Library::referencedCharacters` (adds to the cast).
- Fall-throughs: chapter-editor.svelte:1277 (unknown tool → addSampleArea), slice-edit.ts removeSelected (→ events), slice-inspector rows (→ item). `slice-clip.ts LISTS: Record<kind>` and `makeClip(sel.kind)` make the clipboard type-check only once it knows `piece`.
- score-looks.css styles `.leaf > svg` (54, 175, 229, 261): the music SVG must stay a direct child of `.leaf`.
- `check:tools-css` already reports lines on main (score-view.svelte:358 Feather "x", map tabs, bar-tabs): green = no new line in a touched file.

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
/** a named colouration preset (tonight: COLLAGE_LOOKS in code) */
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
	 *  slice: % of the picture. score (music): x = % of the bar's width from its start, y = staff spaces below
	 *  the staff's top line, width = staff spaces. score (`at.page`): % of the page */
	rect: { x: number; y: number; width: number };
	anchor?: Anchor;   // default 'tl'; score pieces are written 'c'
	rotation?: number; // degrees around the centre
	preset?: string;   // a CollageLook id; none = as drawn
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

Pure helpers in collage.ts (tested): `COLLAGE_LOOKS`, `collageLook(id)`, `pieceStyle(piece)` = `{ ...look, ...piece.style }` minus nulls, `pieceHeight(width, ratio, boxRatio)` = `width / (ratio × boxRatio)`, `heavyWeight(style)` (outline 1, shadow 1, blur 1, progressive blur 2), `pieceName(piece)`, `COLLAGE_LIMITS`, `collageProblems(pieces, host: 'slice' | 'score', add)`.

**Presets by reference.** A piece names a preset (`preset: 'negative'`) and may replace single keys in `style` (a whole shadow, or `null` = off). Renderer and inspector both read `pieceStyle()`, so a preset change reaches every piece that has not replaced that key. Ten code presets from the lab's INKS / GRADIENTS / BLENDS: `ink` fill #111111 · `red` fill #ff1900 · `cyan` fill #00d1ff · `negative` fill #ffffff + difference · `burn` #ff1900→#b300ff + multiply · `acid` #00d1ff→#37ff00 + exclusion · `sticker` outline #ffffff 3 + shadow #111111 2/2/0 @0.8 · `glow` shadow #00d1ff 0/0/6 · `ghost` opacity 0.5 + blur 3 · `fade` blur 6 from 50/0 to 50/100. Owner-made presets later live in one id namespace (no migration).

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
- Tool `piece` (Feather `scissors`) in the tool strip: text · item · piece · sound · event. chapter-state: `TOOL_TAB.piece = 'layers'`, `KINDS.layers += 'piece'`, `select()` / `setTab()` treat it like item; `pickingPiece`, `pieceSpot`, `pickPiece(spot)`.
- Click → the picker → the piece centred at the spot (anchor 'c', width 30 %); a drag → anchor 'tl' at the rect, its width kept. `chapter-editor` gets an explicit `Record<Tool, …>` create map (no ternary fall-through).
- Canvas: `ItemKind += 'piece'`; items from a pure `lib/editor/canvas-pieces.ts` (path `[...slice,'pieces',i,'rect']`, label `✂ name`, rotation, anchor factors, height `pieceHeight(width, ratio, sliceRatio)`, writes `drawnHeights` for AnchorField's reanchor); `transformable`; `handlesOf(piece) = []` (move, rotate, uniform scale – never stretched; `scaledRect` writes no height); centre snap, grid, nudge, undo per drag come free. Blur ramp: two round dots on the selected piece (`canvas-point-handles.svelte`, rotation-aware `pointInBox` in canvas-geometry.ts) – first cut line. A piece's box takes the pointer only under the scissors tool or while selected (`pieceTakesPointer`): art is often as large as the slice, so another tool draws over it (the text tool drags a text box, the item tool places); picking another tool lets a selected piece go.
- Layers list (moved into `slice/layer-list.svelte`, slice-inspector is at 332 lines): piece rows at the bottom (thumb = the file, glyph scissors), a top rule marks the boundary; `moveLayerRows()` (collage-edit.ts): piece ↔ piece reorders `pieces[]` and answers where each piece went (`followPieces` keeps the selected piece selected – the stack keeps its indices; undo / redo follow it by id, `piecesMoved`), stack ↔ stack = `setSliceLayers`, a drag across is refused. `+ piece` in `.adds`; selected piece → `PieceInspector host="slice"`.
- `removeSelected`: explicit `piece` and `event` branches. Phone bar label `piece`.
- Clipboard: `LISTS.piece = ['pieces']`, `ClipKind += 'piece'`, `clipWhat`; paste = new id, `offsetRect`, `src` rebased across toons; `rebaseSliceMedia` / `foreignFiles` walk `pieces[].src`; Shift+V `applyPieceLook` copies `preset` + `style` (cut line).
- Built 2026-10-18 (step 4), nothing cut: the canvas draws the pieces through `SlicePicture always eager` around its own `<img>`; `slice-edit.ts` `SLICE_LISTS` (one kind → list map for remove, copy, paste) + `addPiece`; `clip.ts` `offsetRect` (moved) / `preparePiece` / `applyPieceLook`; a piece's canvas label is its file / figure ids (`pieceName` – the canvas has no registry), the list and the inspector name the character. A Layers drag across the boundary changes nothing and a toast says why ("Pieces always lie under the text boxes and items"). `check:creator-chapter -- --pieces` runs only the piece case (+ save, reload, restore) on a chapter / group / slice of `SCUTOONS_CHAPTER` / `_GROUP` / `_SLICE` – catpope 1 and 2 carry unpublished drafts in the dev DB, catpope 3 group 2 has inline slices.

**Validation** (`checkSlice` → `collageProblems(s.pieces, 'slice', …)`): errors – not an array / object, missing or duplicate id, `src` without a media extension, `ratio ≤ 0`, rect not numbers or width ≤ 0, a `height`, bad anchor / rotation, blend not in BLEND_MODES, opacity outside 0…1, colours not #rgb/#rrggbb, gradient < 2 stops or `at` outside 0…100, only one of from / to, `figure.view` unknown, `at` / `behind` on a slice. Warnings – unknown preset; > 12 pieces; heavy weight > 4 ("slow on phones"); outline > 8, blur > 20, shadow blur > 25 (% of the piece); pieces on a video slice ("shown on picture slices only"). Schemas regenerate (chapter, chapter-doc, group, group-doc, catalog – both copies); the server guard then enforces the enums with no PHP change.

## 4. Scores

### 4.1 Storage and delivery
`score.collage` / `score.parts.<name>.collage` in the `item:<id>` document: the creator already edits it with drafts, history, undo and Publish (MetaPage); each collage is bound to its file by construction; `publicItem` strips it (no vault path, no bulk in every reader's `/api/items`). Readers get it with the signed URL they already request: `ProtectedFiles::score(…, 'musicxml', part)` answers `{ url, expires, collage?: { pieces } }` (never `file`); `@scutoons/api` `scoreFile` widens its type; viewer `score-actions.svelte` passes `pieces` + `pieceSrc={asset}` to ScoreView. Server guard: `CatalogValidator::container` checks each collage structurally (`file` under `scores/`, pieces a list, unique string ids, `src` media extension, `ratio > 0`, rect numbers, `at.bar` non-empty string, `style.blend` in `BLEND_MODES`) + a CatalogTest parity test of BLEND_MODES (like FX_SOUNDS). TS `validateContainers` runs `collageProblems(…, 'score')` (at required; > 60 pieces / heavy > 20 warn) and warns when `collage.file` ≠ the part's musicxml ("made on another file – check the bars"). The stage edits the part readers open (`readerPart`: choir if present, else the full score – locally the full score).
Built 2026-10-18 (step 5): `ProtectedFiles::score` adds `collage: { pieces }` to the MusicXML answer when the opened score / part has pieces (`collagePieces`; an empty list = none); `CatalogValidator::collage` runs on `score.collage` and every `parts.<name>.collage` at a catalog write (the published item); the viewer passes `pieceSrc={asset}` (catalog paths from the CDN root).

### 4.2 Anchoring (survives page size, zoom and reflow)
- Music (default): reference point = bar `at.bar` (`nth`), staff `at.staff`: x = bar left + `rect.x` % × bar width (bar widths vary 2–3× between layouts – a piece at 80 % of a bar stays at 80 %), y = the staff's top line + `rect.y` staff spaces, width = `rect.width` staff spaces (scales with zoom like the music). The piece's anchor (written 'c') sits there.
- Page (`at.page`): `rect` in % of the page that holds the bar (backgrounds).
- A bar that moves to another page takes its pieces along (geometry is read from every rendered page). A bar missing from the file: readers skip the piece, the stage lists it. A replaced file: the stage's notice "made on another file – check the bars" + **Checked** (writes `file`). Renumbered bars move pieces musically without notice (known limit).
- `shared/reader/src/score/score-anchors.ts` (pure, node-tested against a fixture SVG from a real 6.3.0 render): `barIndex(mei)` (measure id → `{n, nth}` from `tk.getMEI()`, 5–25 ms on the local files; ids stay through `redoLayout`); `pageGeometry(svg, bars)` → `{ width, height, sp, bars: { id, n, nth, left, right, staves: { n, top, bottom }[] }[], frame }` in viewBox units (margin translate included; sp = the median line distance); `placeScorePiece(piece, geo)` → `{ left, top, width }` in % of the page or null; `anchorScorePiece(point, geo, page?)` → `{ at, x, y }` (the bar whose system band holds the point, else the nearest; the nearest staff; x rounded to 0.01 %, y to 0.001 sp; staff 1 / nth 1 not written); `findBar(bars, at)`. No DOM measuring, no px, no rem. The leaf is not exactly the SVG's box: Verovio writes the element's px height rounded up and centres the viewBox in it (`xMidYMid meet`, up to half a pixel) – `frame` is the viewBox in % of the element's width and `pageFrame(geo)` the layer's CSS (width, aspect-ratio, margins – no transform), so % of the page are % of the music (measured: ≤ 0.4 px in Chromium / Firefox / WebKit at 390 and 1280).

### 4.3 Rendering in ScoreView (behaves as today without pieces)
- Move, not copy: `score/score-layout.ts` takes `ZOOMS`, `DEFAULT_ZOOM`, `PHONE_ZOOM`, `margin()`, `scoreOptions(w, h, zoom, look, remPx)` (merges `measure@n` / `staff@n` into the look's `svgAdditionalAttribute`) out of score-view.svelte. `ScoreToolkit += getMEI`.
- New props: `pieces?`, `pieceSrc?`, `embedded?` (no portal, no window keys, no stored zoom, absolute + `isolation:isolate` in its box), `zoom` (`$bindable`), `page` (`$bindable`), `follow?: ScoreAt` (the layout anchor and "turn to it" instead of the first bar), `onlayout?(bars)`, `onclose?` optional (no close button / Escape without it), `decor?: Snippet<[number, PageGeometry | null]>` (the lab's one-argument snippets still fit). The header's Feather "x" becomes `CloseButton` (the file's tools-css line).
- Per page: `{@html svg}` → `<ScorePieces>` (`.score-pieces { position:absolute; pointer-events:none; overflow:hidden }` on `pageFrame`, no z / transform / isolation – overflow clips a piece at the page's edge so the leaf never scrolls for it; each piece carries its own z: behind 0, over 2) → age → decor. With pieces: `.sheet[data-pieces] .leaf > :global(svg) { position:relative; z-index:1 }` and `.score-age` z 2. The paper is `.sheet`'s white inside `.score-view`'s stacking context; blends reach it across `.pages` / `.leaf` (tested). Only rendered pages (current ±1) draw pieces. The PDF stays clean.
- Built 2026-10-18 (step 5): the bars are read (one `getMEI` per load) only with `pieces` (even empty – the stage), `follow` or `onlayout`, so a plain score does no extra work; `onlayout(bars)` after every layout; `decor(n, geometry)` gets null without bars, and with pieces the music is z 1 – a decor over it brings its own z (the stage's overlay: 3). `rendered` holds `{ svg, geometry }` per page. The player's rAF loop was renamed `keepUp` (the prop is `follow`).

### 4.4 The score stage (creator)
Catalog → Items → a score → secondary **Collage** (catalog-editor actions, `category === 'score'`) → `/catalog/items/[id]/collage` (`/catalog/items` itself still goes to `[kind]`). `score-stage.svelte` = `MetaPage toon="" fill help="catalog" docs={[neutralDoc(catalogKey('items', id))]}` (Save draft, History, Publish, autosave, leave guard, Ctrl+S / Z), crumbs via `usePageNav`; the file signed with `authApi.scoreFile(id, 'musicxml', part)`; registry + moods docs for character rows.
- Main: a square Segmented `smartphone` / `monitor` → `score-frame.svelte`: the Reader dock's screens (390 × 844 / 1440 × 810 CSS px – simulated screens, `SCREENS` moved to `lib/editor/screens.ts`) scaled to the column with `transform: scale(k)` – the paper is inside the group, so blends equal the reader. Inside: `<ScoreView embedded>` with its own header, zoom ± and page footer: page breaks equal the reader's by construction. Zoom per frame: 30 / 40.
- `decor` → `score-overlay.svelte` per page: boxes with the same `pieceBox()` CSS as the pieces, `pointer-events:auto; touch-action:none` only on boxes (empty paper still swipes). Move = re-anchor on every pointer move (`anchorScorePiece`, one checkpoint per drag; pointer % from the overlay's `getBoundingClientRect`, so the frame's scale cancels out); a round corner handle scales `rect.width`; Delete / Backspace removes (outside fields), Esc deselects. Rotation in the inspector.
- Inspector region (Divider + aside, `panelWidth('sc-panel-width:score-stage', 22)`, below the stage under 1280 px and on phones): `collage-list.svelte` (over pieces front first, a rule, behind pieces; drag within a side; ⚠ on a missing bar; a click selects and `follow` turns to its page) + `+ piece` (lands at the centre of the current page, width 12 sp) + `PieceInspector host="score"`. The first piece writes `collage = { file: <part musicxml>, pieces: [] }`; the last removed takes `collage` away.
- `validate` = collageProblems + file check + missing bars (from `onlayout`).

## 5. The picker – `collage/collage-picker-sheet.svelte`
Creator `PickerSheet` (Sheet right, widthKey `picker`), Tabs **Shapes · Characters**, a secondary **Image …** in the filter row hands over to `AssetPickerSheet` (kind image; the WebP rendition of a PNG + WebP twin preferred).
- Shapes (`shape-grid.svelte`, list cached in `lib/editor/shapes.svelte.ts`): `assetsApi.list({ kind: 'shape', folder: 'catpope', page_size: 500 })` once per session; `.sc-picker-filters` = search (title, slug, tags) + a tag select by frequency (tags, not classes); `.sc-cards` with `gridArrows`, checker background, mask shapes as an ink silhouette; a whole-tab empty library = one `.sc-empty` line. This is VECTORS §6 step 8's Shapes picker.
- Characters: `character-picker-sheet.svelte`'s body **moved** into `characters/character-grid.svelte` (the sheet = PickerSheet + CharacterGrid, same API for cast-editor and character-ref-field). A mood cell resolves `figureImage(char, { persona, look, mood, view: 'portrait' })` (collage-pick.ts: the fallback rule, but '' instead of the "?" substitute – a character with no picture, or only a known-missing one, adds nothing and a warning toast says so); the view is the inspector's Picture row.
- Returns `PickedPiece { src /* CDN root */; ratio?; figure?; name }` – ratio from AssetSummary width / height, else `imageRatio(url)` (decode). The host converts the path (slices `toonAssetRef`, scores as is).
- Built 2026-10-18 (step 3): `lib/editor/collage-pick.ts` holds `PickedPiece`, `shapePick` / `imagePick` (the WebP rendition of a PNG / JPEG), `shapeTags` / `filterShapes`, `imageRatio` and `pickedToPiece(picked, toon, place)` (toon '' = a score's root path); a one-colour shape shows as `filter: brightness(0)` on its lazy `<img>` (masks don't load lazily); **Image …** sits behind the tabs (Tabs `after`), so both tabs reach it; a character pick never adds to the cast.

## 6. Rendering – `shared/reader/src/collage/collage-piece.svelte` (`CollagePieceView`, props `piece, src, left, top, width, z?`), builders in `collage-css.ts` (tested). No inline SVG: no `{@html}`, no id collisions, nothing to sanitise.
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
| Progressive blur | two bands; `blurBands(style, ratio)` maps from / to into the padded box (`gradientLine`): θ = atan2(dx·w, −dy·h), L = abs(w sinθ) + abs(h cosθ), stops = projections; the soft band added (`plus-lighter`) in the isolated piece. Built 2026-10-18 (step 2), identical in Chromium / Firefox / WebKit (probe: an opaque `fade` disc stays opaque down its middle, a `negative` ramp inverts there); WebKit rounds drop-shadow lengths to whole px |
| Rotation | `rotate` on `.piece`; shadows rotate with it |

**Limits** (validation warnings, not caps): slice 12 pieces / heavy weight 4; score 60 / 20; outline ≤ 8 %, blur ≤ 20 %, shadow blur ≤ 25 %. Pieces are static, mount near the viewport (slices) or on rendered pages (scores). WebKit software probe: blend 1.6×, drop-shadow + blend ~7×, blur ~5× a plain piece – phones still need an on-device check (owner's Firefox Android + an iPhone).

## 7. The piece inspector – `collage/piece-inspector.svelte` (+ `look-fields.svelte`, `place-fields.svelte`, `gradient-field.svelte`, `points-field.svelte`, each ≤ 300 lines)
Props `doc, path, host: 'slice' | 'score', registry, moods, onremove`. Like the pilot dialog-inspector: head `h3` "Shape / Character / Image · name" + secondary **Remove**; a `Segmented grid` of looks ("As drawn" + COLLAGE_LOOKS, labels `creator.look_<id>`) – a pick writes `preset` and drops `style` in one `doc.edit`; then `<details class="advanced">` (open when the piece has its own `style`). Plain fields `doc.set(path, v)` (undefined = default, coalesced per path: one scrub = one undo step); effect fields through `setEffect()` (collage-edit.ts: copy-on-write from the preset, clearing writes `null` only when the preset has that key). Placeholders show `pieceStyle()`. Every control `--control-height`, ≤ 3 per line.

| Row | Controls |
|---|---|
| Picture (figure; outside the fold) | PropSelect icon `smile` mood (the look's moods) · line: Segmented stretch avatar / portrait / full – one edit re-resolves `src` (`figureImage`) and `ratio`; nothing resolves → the piece stays, a warning toast |
| Position – slice | AnchorField bare · ScrubNumber tag X % · tag Y % |
| Position – score | Segmented square `music` / `file` (bar / page) · line: text input bar (#) · ScrubNumber tag St (staff, placeholder 1); line: tag X (% of the bar / page) · tag Y (sp / %). No anchor picker on scores |
| Size | ScrubNumber tag W (% / sp, min > 0) · ScrubNumber icon `rotate-cw` ° |
| Side (score) | Segmented stretch Over / Behind (fallback Over) |
| Blend | PropSelect icon `blend`, 16 modes (`creator.blend_<mode>`) · ScrubNumber icon `eye` opacity % (stored 0–1) |
| Fill | Segmented square none `slash` / colour `droplet` / gradient `gradient` · line: ColorField, or GradientField = ColorField from · ColorField to · ScrubNumber angle ° |
| Outline | ColorField (placeholder none) · ScrubNumber icon `outline-width` % (only while a colour applies) |
| Shadow | ColorField (placeholder none) · ScrubNumber `eye` % · line (with a colour): tag X · tag Y · icon `blur` |
| Blur | ScrubNumber icon `blur` % · Segmented square even `circle` / ramp `gradient` · ramp: PointsField = lines [X1 · Y1] [X2 · Y2] (+ the canvas dots on slices) |

New icons in `shared/ui/src/icons-extra.ts` (Feather manner, 24 grid, strokes): `blend`, `gradient`, `shadow`, `blur`. GradientField / PointsField live in the creator (one user); they move to @scutoons/ui when a second inspector needs them.

Built 2026-10-18 (step 3): the edits in `lib/editor/collage-edit.ts` – `setPreset`, `setPlain` (blend, opacity), `setEffect` (a whole value; clearing = `null` when the look has the key, else no key; an emptied `style` goes), `editEffect` (copy-on-write of the drawn effect; an effect that is off starts from what its field just cleared, else the look's, else `EFFECT_DEFAULTS`), `setEffectColor` (an outline's / shadow's ColorField sends `undefined` on every keystroke of an emptied text: empty = off and the inspector keeps the effect as drawn, per piece and look – a colour retyped brings it back with its width / offsets / blur / opacity; the fill's ColorField ignores an emptied text, only the kind None clears a fill; ColorField shows the kept value again on blur), `setFillKind` / `setBlurRamp` (a fill switched off over the look comes back as the look's when that kind is asked for, else from the look's colour; the kind a fill / blur already has is no edit – Segmented sends the chosen option again, a re-click must not rebuild a gradient, reset the ramp points or copy the look's value), `setFigure` + `figurePath` (a mood / view re-pick waits for the picture's ratio: it lands on its piece by id, never on whatever the inspector's path shows by then; a newer re-pick of the same piece carries the older one's changes), `layerRows` / `canMoveLayerRow` / `moveLayerRows`; words in `collage-words.ts`. The effect fields show the piece **as drawn** (`pieceStyle`) as their value, not as a placeholder – so a look's outline / shadow can be cleared (→ `null`); plain fields keep own value + the look's placeholder. The head names a character by name · mood. Side labels Over / Behind (the long words as tooltips). Slice pieces' AnchorField needs the canvas' drawn height (step 4 `canvas-pieces.ts` writes `drawnHeights`).

## 8. Build plan for tonight
Order core → slices → scores: the shared model, renderer, picker and inspector are ~60 % of the work and the slice editor (selection, handles, undo, Layers, phone sheet, Reader dock) proves them fastest; the stage is built on finished parts. **Timebox:** step 5 starts by ~55 % of the night whatever state step 4 is in. **Cut order:** blur dots on the canvas → Shift+V → Image … hand-off → the stage's scale handle. Commit each step on main with its own paths only, no Claude attribution line (workspace CLAUDE.md), never push; leave other sessions' files alone (logs/, tools/sketch3d/, storyboards/…, anything the vector session touches: score-riot, the lab page, VECTORS.md, ASSETS.md, vectors scripts).

0. **Preconditions** – baseline per-workspace checks and the existing `check:tools-css` lines noted; dev stack up; signed in as agent@scutoons.test.
1. **Model, layer stack, validation, schemas** (S) – collage.ts, Slice.pieces, ScoreFiles / ScorePart.collage, layers.ts, validate.ts, validateContainers, tests, schemas.
2. **Renderer + picture group + viewer** (M) – collage-css.ts, collage-piece.svelte, slice-picture.svelte, Slice, viewer slice; three-engine probe.
3. **Picker + inspector** (L) – CharacterGrid move, CollagePickerSheet, ShapeGrid, PieceInspector + fields, collage-edit.ts, icons, messages.
4. **Slice editor wiring** (L) – tool, canvas, Layers list move, add / remove / create map, clipboard, phone label, blur dots; check:creator-chapter piece case (incl. the ramp's middle: an opaque `fade` piece stays opaque there).
5. **Score anchoring, ScoreView, server delivery** (M) – score-layout.ts move, score-anchors.ts + fixture, score-pieces.svelte, ScoreView props, ProtectedFiles / CatalogValidator / tests, api type, viewer score-actions.
6. **Score stage** (L) – route, ScoreStage, ScoreFrame, ScoreOverlay, CollageList, score-collage.ts, screens.ts move, Collage button, check:creator-collage.
7. **Docs, green, handover** (S) – this doc updated to what was built, DESIGN / DATA / CATALOG / GOTCHAS / FEATURES / MCP format line, HANDOVER "Last run".

The owner opens in the morning: creator → a chapter → a slice → the scissors tool; creator → Catalog → Items → Cloud 9 → Collage.

## 9. Deferred
Owner-made presets + "Save as preset" (the `colorations` document); pieces over the lettering or reaching into the neighbour slice (a front plane after `.slice-layers` with z `var(--plane,1)` – no data change); pieces on video slices; mirroring; round outlines (SVG filter); radial gradients, > 2 stops in the UI, a canvas gradient gizmo; tint / duotone fills; the Images tab inside the sheet; lookahead prefetch of piece files; effect thinning on the `low` media quality; on-device measurements; the stage: all pages stacked, rotate handle, copy / paste, nudge, phone bottom sheet, drag across pages, a part switch, signing a draft-only score file; re-resolving figure pieces when the registry changes (`src` is frozen; the Picture row re-resolves); MCP `search_assets` kind `shape`; freezing a lab riot page into pieces; the production vector import (owner).

## 10. Owner questions (the build uses the default)
See the structured `owner_questions` – presets' home; slices: under the lettering, clipped to the picture; scores follow the music; one collage per file / readers' part; rotation yes, mirror no; effect sizes relative to the piece; 16 blend modes; plain engraving for readers; sensitive slices hide pieces; nothing to production; picture slices only.

## 11. Risks
- **Isolation is load-bearing:** any z-index / transform / opacity / filter / will-change / isolation / container-type on `.slice-pieces`, `.score-pieces`, `.pages` or `.leaf` silently changes blends – CSS comments + the pixel assertions in check:creator-chapter / check:creator-collage.
- **Verovio's SVG structure** (`g.measure[data-n] > g.staff[data-n] > 5 path`) is pinned by the fixture test; a Verovio upgrade re-runs it.
- **Renumbered bars** move pieces musically; only a replaced file warns.
- **The stage signs the published file**; a draft's new file needs publishing first (production score items that were never published can't open in the stage yet).
- **ScoreView is live for readers**: changes stay additive; check a plain score and the lab looks by eye.
- **Phone cost** unmeasured; limits are warnings. Outline corners at thick widths; progressive blur is a crossfade (2× cost).
- **Production has no shapes**: never push or publish pieces before the production import.
- **Two editors on one item** (catalog form + stage in two tabs) → the existing save conflict.
