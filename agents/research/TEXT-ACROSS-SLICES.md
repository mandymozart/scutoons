Status 2026-10-15: concept, researched (code, real data, three browser engines, horizontal groups §7); nothing built – waits for the owner's Q1–Q7 (§6, §7). FEATURES 10.100.

# Slices as background: the strip editor

Owner 2026-10-15: "sometimes boxes … are cutting over the end of slices" · "having text boxes pass is good as a tool" ·
"eventually i want to not have to worry about slices, they will become a background thing" · "i really need super good
design and concept that will last and be able to migrate the current slice editor – scenes and slices are still a bit
clunky" · "not important for right now".

## 1. What was actually wrong (measured, not assumed)

- **The reader already paints a box past its slice's bottom over the next slice** – tested on the real local reader
  (chapter 1, a box pushed to 96 % / 99 %) in Chromium 140 + 152, Firefox 141 and WebKit 26: the box stays on top. The
  old note "text overflowing a slice's bottom is painted under the next slice" (archive/MEMORY.md) was true when
  `container-type` implied layout containment (a stacking context per `.slice`); the CSS WG dropped that in 2024 –
  Chrome 129, Firefox 131, Safari Technology Preview 206. **Older Safari (iOS ≤ 18, still in use) may still cut the
  box** – every iPhone browser is WebKit. So today it depends on the reader's device. Rule now (implicit): images
  below, text boxes / items / areas above all images; between two slices' overlays the later slice is on top.
- **The ch2 boxes** (group 4): published v633 has six boxes past a slice's bottom (slice images 39, 45, 47, 61, 84,
  92) – shown on current browsers, cut on an old iPhone. In drafts v637 – v640 slice-40 got its own box (8 – 18 %)
  right where slice-39's box (98.9 → 126.9 %) reached: the later slice's box painted over it – a **collision**,
  hidden on every device. The others collide with nothing; the likely place the owner saw them cut is **the creator,
  which shows one slice** (a box above / left of the canvas is cut and can't be scrolled to: 61 px at fit).
- **Real content (dev DB = prod seed + drafts, latest versions; the scratch scripts measured it):** 14 documents, 276
  slices, all 390 wide (625 × 230, 693 × 44 in strips, 845, 206). 88 real text boxes (+ 25 FX test boxes in the ch1
  draft), all with ids, 8 targeted by `showTextBox`. 7 boxes cross a top edge (chapter 1: 5, 6, 12, 13, 15, 75, 87 –
  the lower slice owns them), the 3 ch2 boxes crossed a bottom edge, 2 bubbles cross a side. 14 placed items, 23 event
  rects, 3 sample rects – none crosses. **No stored document points at a slice by index**; text boxes are found by id.
  Scenes: only ch2 uses `ref` groups (3 strips); scene docs 7, 15, 16 are used by no chapter.

So the reader needs only a small, explicit fix. The real problem is **authoring**: the creator thinks in slices.

## 2. Why the slice editor feels clunky (creator, read from the code)

1. One slice at a time (`slice-canvas.svelte`): no seam, no neighbour, collisions invisible; a box that should cross
   needs a hand-set negative y; moving a box to the next slice = cut → next slice → paste (new id, `showTextBox`
   targets break, slice-clip.ts).
2. Too many stops: tree → group form → slice list → slice → canvas; the desktop shows no picture of the group (the
   strip preview was removed 2026-10-08; the Reader dock only reloads after a saved draft).
3. Everything carries the slice: box ids from the slice image number, slice events may only target boxes on their own
   slice (validate.ts), item pickups remembered by `slice.image` (so images may never be renamed, ASSETS.md),
   "+ slice" guesses the next file name, no split / merge / replace-a-range.
4. Scenes: a `ref` group is a second document with its own undo stack, a status chip, special cases in the validator,
   clipboard and locales – for 3 strips in one chapter. The default group label is still `scene-N`.

What the industry does: authors draw one long canvas; the platform cuts it into tiles (WEBTOON auto-slices anything
over 800 × 1280; Clip Studio exports "divide vertically"). Lettering is baked, so the cut never matters to them. Ours is
live text over the art – so the text must belong to the art, not to the cut.

## 3. The model that lasts

**A group is one continuous strip. Slices are its background tiles. Overlays (text boxes, items, click / hover
areas, sample areas) are anchored to a slice but may reach anywhere, and paint above every slice of the group.**

- **Anchor + offset, not strip coordinates.** An overlay keeps living in its slice's arrays with `rect` in % of that
  slice (exactly today's format – values outside 0…100 simply allowed). Why not one strip coordinate system: inserting,
  removing or replacing a picture would shift every box below it; anchoring makes text move with its art – what
  layout tools do (anchored objects, CSS anchor positioning). It also keeps the DOM nesting the reader relies on (fx
  `inFrame`, the slam shake on `closest('.slice')`, hit tests, per-slice audio) – no reader rewrite.
- **The anchor is chosen by the editor, never by the author:** on drop, the slice under the box's centre. Moving a box
  across a seam re-parents it (rect converted, **id kept**, layer on top of its new stack).
- **Paint order, explicit:** slice images < all overlays of the group; overlays by slice order, then by `layer`. "Bring
  to front" over a box of a later slice re-anchors to that slice. Both directions: §7.
- **Units stay:** % of the anchor slice, font in % of the width. (y in % of width was weighed: no gain while slices are
  fixed-ratio tiles, and a migration plus renderer change.)
- **Scope:** a text box is addressed by its id within the group (`showTextBox` from a slice event, an item or the group
  script) – re-anchoring never breaks a target.
- **Only the layout's window cuts** (§7): vertical = the column's sides; the pinned strip = the screen's top / bottom;
  a Swiper group = the slide on all four sides (paged on purpose – to cross, use Full width).

## 4. The strip editor (creator)

```
┌ tree ───────────┬ strip (the group, at reading width, zoomable) ──────┬ inspector ───────────┐
│ Meta  Audio     │                                                      │ nothing selected:    │
│ Groups          │   ┌──────────────────────────┐                      │  the group: label,   │
│  ▸ intro        │   │      picture 38           │                      │  layout, end, script │
│  ▾ cloud-9      │   │   ┌──────────────┐        │                      │ a box / item / area: │
│    [▤][▤][▤]    │ ··│···│ narration     │·······│·· seam (on hover)    │  its fields (today's)│
│    [▤][▤][▤] ←  │   │   └──────────────┘        │                      │ "Picture" tab: the   │
│    filmstrip =  │   │      picture 39           │                      │  slice under the     │
│    minimap      │   └──────────────────────────┘                      │  view's centre       │
└─────────────────┴──────────────────────────────────────────────────────┴──────────────────────┘
```

- **One screen per group** instead of group form + slice list + slice canvas: the strip scrolls like the reader
  (virtualised – only the pictures near the view are mounted), tools as today (text, item, sample, event), drag a box
  anywhere, arrow keys cross seams. Seams show as a hairline on hover or with a toggle; collisions show an outline.
- **The slice list becomes a filmstrip** of thumbnails under the open group: click = scroll there, drag = reorder,
  drop files = insert / replace, multi-pick = replace a range with one tall upload (the creator cuts it into tiles).
- **Picture properties** (image / video, sensitive, music volume, whole-picture events) move to a "Picture" tab for
  the slice under the view's centre or the one picked in the filmstrip.
- **The group's script** stays in the inspector; the box picker lists boxes by their text ("WO BIST DU, MAMA?! ·
  picture 2"), not "Slice n".
- **Phone:** the same strip at fit width; tap selects, the bar holds the tools, a selected box moves by drag (an
  unselected one never steals the scroll). Horizontal groups: the strip laid out sideways; a Swiper group edits slide
  by slide.
- The Reader dock stays the "how it reads" view; the strip is the editing surface itself, not a second preview.

## 5. Migration path (each step ships alone)

| Step | What | Size | Data |
|---|---|---|---|
| 1 Reader paint rule | §7: `container-type` moves from `.slice` to `.slice-layers` (not to the group – that breaks strip fonts); the strip drops the slice's side clip, a Swiper clips at the slice-layers; strip travel from the frames; fx clip only the media; `sliceEdges` validator messages (window cuts + collisions, nothing for an open edge – owner: a tool); canvas bands in the one-slice canvas (Q7); a check script (both directions × vertical / strip / Swiper × Chromium, Firefox, WebKit + an old-iOS pass), then one real iPhone on iOS 17 / 18. DESIGN.md states the paint order | S–M | none |
| 2 Strip editor | §4; re-anchoring on drop; ids from a chapter counter, not the image number; validator: `showTextBox` targets in the group, not only the own slice; the reader's hidden-box set per group (today per slice, viewer slice.svelte `sliceRefs`) | L | none |
| 3 Data hygiene | slices get a stable `id` (migration); pickups remembered by slice id (one-time localStorage helper), so images may be renamed; scene `ref` groups inlined into their chapter (migration; 3 in ch2) and `ref` support removed (validator, clipboard, creator, locales, sandbox generator); default group label without "scene" | M | 2 migrations |
| 4 Tall pictures | upload one tall image (a storyboard shot up to 1:7) → cut into tiles = consecutive slices; "replace range"; the storyboard compile no longer moves cuts off text boxes (research/STORYBOARD.md §2) | S–M | none |

Later, only if needed: a "picture" made of several tiles as one unit (one sensitive flag, one replace) – the anchor
model above doesn't change for it.

## 6. Open for the owner
- **Q1** Step 1 now (small; fixes crossing boxes on older iPhones, makes the rule explicit) – or with step 2?
- **Q2** The strip editor replaces the one-slice canvas (§4) – drafts on the real page (phone + desktop) first?
- **Q3** Inline the scenes into their chapters and drop scene documents (step 3)?
- **Q4** In the UI, keep calling them "slices", or "pictures" (Bilder)?
- **Q5 – Q7**: §7.

## 7. Horizontal groups (strip, swiper) and the previous slice

Owner 2026-10-15: "vertical scroll, the next one it works … But horizontal scroll creates issues" · "also the previous
one will be an issue, so if there isn't a box on the top, it will cut off". Measured in Chromium 140, Firefox 141, WebKit
26 at 390 × 844 and 1440 × 900 – real local viewer (vertical, strip) plus replicas; Swiper on a standalone page with the
repo's Swiper 12.1.3 (the local DB lacks ch3–5's Swipers); old iOS emulated (`contain: layout`), not a real iPhone;
pixels and hit tests agree. A workflow of 11 agents (map, 3 designs + judge, experiment + code skeptic).

| today, a box crossing … | vertical | strip (`fullWidth`) | Swiper slide / loop / freeMode | Swiper cube / flip |
|---|---|---|---|---|
| into the previous slice | shows (old iOS too) | cut by its own `.slice` | cut | cut |
| into the next slice's picture | shows; old iOS: covered | cut by its own `.slice` | cut | cut |
| into the next slice's box | covered | cut | cut | cut |
| past the window | sides: cut at the column | top / bottom: cut by `.pin` | all sides: `.swiper` | top / bottom spill out |

- **In the strip both directions are cut by each slice's `overflow-x: clip`** (shared/reader/src/slice.svelte:164) –
  not by paint order; `.pin` (strip.svelte:242) already clips at the screen. Without that clip both directions show
  (3 engines, real ch2 strip, also fullscreen). Removing only the clip leaves old iOS lopsided (left shows, right
  covered by the next picture: `container-type` = a stacking context per slice there) – so the unit moves too.
- **"If it isn't on top" = the forward direction:** a box reaching into the next slice lies under that slice's boxes in
  every engine and layout (old iOS: under its picture too).
- **Production:** 8 horizontal groups – 4 strips (ch2 + its scenes, 390 × 693), 4 loop + freeMode Swipers (ch3–5,
  390 × 844); 16 boxes, 6 items, **no crossing**. So the cuts were seen in the creator or while trying.

**Paint contract (both directions, every layout):** pictures < all overlays of the group; between two slices the later
slice's overlay plane wins; inside a slice, `layer`. **Backward is the safe direction** (over the previous picture and
its boxes); **forward** lies over the next picture but under the next slice's boxes – a collision: the validator names
it, the editor resolves it by re-anchoring (§3). Positioned pieces outside the plane (old-style trigger areas z 2, sample
areas z 1, the sensitive note z 2, the fx flash z 4) share the group's stacking order once `.slice` isn't one: they may
paint over a neighbour's crossing box (e.g. "Show hints" outlines) – move them into the plane in step 1, or accept.
Only the layout's **window** cuts:
- **Vertical:** the column – sides cut, up / down free; above the first / below the last slice is outside the group.
- **Strip:** the screen (`.pin`) – top / bottom cut, plus the strip's start and end; left / right free.
- **Swiper (slide, loop, autoPlay, freeMode):** the slice on all four sides – paged on purpose; to cross: Full width.
  (The cut is the `.slice` box: mixed slice heights cut at the shorter one, a slice taller than 100dvh at `.swiper`.)
- **Swiper cube / flip:** the same; nothing crosses a 90° turn (the turning slide itself still projects mid-turn).

**CSS (replaces §5 step 1's "container on the group": on `.track` the font collapses to 0 px, on `.strip` it becomes
1 % of the viewport; a clip on the renderer cuts the full-viewport videos again, bc0787b3):**
```css
.slice { position: relative; overflow-x: clip; }                    /* shared slice.svelte: container-type removed */
.slice-layers { position: absolute; inset: 0; z-index: 1; container-type: inline-size; }   /* slice-layers.svelte */
.frame :global(.slice) { min-height: 0; overflow-x: visible; }     /* strip.svelte: .pin is the window */
.renderer--horizontal :global(.slice-layers) { overflow: clip; }   /* horizontal.svelte: the slide is the window */
```
- `.slice-layers` is the slice's own box, so 1cqw stays % of the slice width everywhere (column, strip frame, slide,
  creator canvas – `.creator-canvas` keeps its own container): font px identical in 3 engines (real boxes too), and with
  old iOS emulated on `.slice-layers` all verdicts hold – the containment lands on an element that is a stacking
  context anyway. Vertical keeps the slice's side clip (it only ever cut at the column); `.full-width` stays.
- **Ships together, or it regresses:** strip travel from the frames (first frame's left → last frame's right − pin
  width; strip.svelte:54, chapter +page.svelte:115–116 incl. `arriveAt`), not `track.scrollWidth` – else a box past
  the last frame runs the strip onto black (+136 px) and fits mode flickers; and the **fx**: `inFrame` (cues.ts:226)
  clip-paths the whole slice for the shake / punch (default 410–480 ms, an fx event up to 3 s) – it clips a new
  `.slice-media` wrapper around img / video / `.blurred`, the plane gets the same keyframes unclipped (measured: no
  gaps, the crossing part stays); `holdClip` (cues.ts:35, nudge 750 ms, lift until the grant answers) skips the strip;
  fireflies clamp to the window.
- Rect intersect areas at a strip frame's edge fire a little differently once the clip is gone (ratio 0.5 → 0.63).
- A full-viewport video slice has no side clip: its sides count as the screen (warn), the page width stays guarded.

**Validator** (validate.ts, warnings only): a pure `sliceEdges(group, i)` gives each edge one of
`open | column | screen | strip-end | group-end | slide`, shared with the creator; `checkGroup` (≈858) passes it to
boxes, items and areas, replacing the fixed x / y ranges (195–196) and the side warning (203–205). One message per
window ("…past the slide's top – a Swiper group shows one slide at a time; Full width lets boxes cross"); **nothing for
an open edge** (owner: a tool); collision ("covered by '<first words>' on slice n – the later slice paints on top")
when both boxes have a height; old-style VN click / hover areas: "only the part on its own slice reacts"; old-style
sample areas: "the part over slice n takes its taps" (they still react there). Auto-height boxes (13 of the 16
horizontal ones), items without height and rotated / scaled boxes escape rect checks – the canvas shows those. On
today's published and draft content: no new warning (the slice-39 / 40 collision lived only in drafts v637 – v640).

**Creator:** interim (one-slice canvas): the editor passes `sliceEdges` and the neighbours; an open edge gets a band
with the neighbour's picture dimmed at the seam and its boxes as outlines, a cut edge a hatch with its reason; fit
counts the bands and the area scrolls to them (fixes today's cut top / left: `.canvas-scroll` can't scroll to negative
overflow); a box over a cut edge gets an `--error` outline, a collision `--warning`; no clamping; a strip's phone fit
follows the height. Strip editor (§4) with an axis: vertical = y, strip = x (re-anchoring converts sizes by the width
ratio, wheel sideways), Swiper = slide by slide with four hard edges. Switching a group's layout lists the boxes the new
window would cut.

**Steps (§5):** step 1 = the CSS + travel + fx + `sliceEdges` + canvas bands + the check script, then one real iPhone
(iOS 17 / 18). Step 2 = the editor's axis, re-anchoring across seams (takeable items keep their anchor until slice ids:
pickups are keyed by `slice.image`), the hidden-box set per group, the Reader dock sliding a Swiper to its slide.

Found on the way (not this feature): the fullscreen strip jumps during a screen shake (the shake transforms
`.chapter-view`, the fixed pin's new containing block); `sliceInView` picks strip frame 0 for slice fx; home
`product-slides` leaks `:global(.swiper-slide)`; a Swiper slice over 100dvh loses its bottom; cube / flip fire every
slide's observers at once; Firefox on a phone widens the page mid cube swipe. Untested: the Swiper cube in WebKit
(headless draws it blank), any real old iPhone; Safari < 16 ignores `overflow: clip`.

- **Q5** "Horizontal" = the pinned strip (crossing will work both ways) or a Swiper group (cut by design; way out: Full
  width)?
- **Q6** The four Swipers (ch3–5) are loop + freeMode and read like a strip – keep them paged, or make them strips?
- **Q7** The canvas bands in today's one-slice canvas with step 1 – or only the strip editor in step 2?

Sources: WEBTOON Creators Resource Handbook (800 × 1280 per image, auto-slice); Clip Studio "Export Webtoon"; Chrome 129
container-query change (dev.to/michaelcharles); WebKit commit 284730@main "Drop layout containment from container-type"
(Safari TP 206), WebKit bug 292266 (April 2025, stable Safari still affected).
