Status 2026-10-15: concept, researched (code, real data, three browser engines); nothing built – waits for the owner's Q1–Q4 at the end. FEATURES 10.100.

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
- **The three ch2 boxes the owner saw** (draft 641, group 4): slice-39's box ran 98.9 → 126.9 % straight into
  slice-40's own narration box (8 – 18 %) – the later slice's box painted over it: a **collision**, hidden on every
  device. Slice-84's (79.4 → 129 %) and slice-92's (94.2 → 109.2 %) collide with nothing: fine on current browsers,
  cut on an old iPhone, and – the likely place the owner saw them – **cut off in the creator, which shows one slice**.
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
  to front" over a box of a later slice re-anchors to that slice.
- **Units stay:** % of the anchor slice, font in % of the width. (y in % of width was weighed: no gain while slices are
  fixed-ratio tiles, and a migration plus renderer change.)
- **Scope:** a text box is addressed by its id within the group (`showTextBox` from a slice event, an item or the group
  script) – re-anchoring never breaks a target.
- **Where crossing can't work:** a horizontal Swiper group is paged (each slide alone, cube / flip / loop) – a box past
  a slide's edge is cut: the validator warns there only. The pinned strip (`fullWidth`) is one sideways strip: the
  same rule sideways (font unit = % of the slice's own width, from the screen height × ratio).

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
| 1 Reader paint rule | `container-type` + `overflow-x: clip` move from each `.slice` to the group's renderer element (vertical: same width, cqw unchanged); `.slice` stays `position: relative`. Crossing boxes then render on every engine incl. old iPhones. A check script with a crossing box in Chromium, Firefox and WebKit (the playwright browsers are installed). Validator: warn when a box is covered by a box of a later slice (the slice-39 case) and past a Swiper slide's edge – no bottom warning (owner: a tool). DESIGN.md states the paint order | S | none |
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

Sources: WEBTOON Creators Resource Handbook (800 × 1280 per image, auto-slice); Clip Studio "Export Webtoon"; Chrome 129
container-query change (dev.to/michaelcharles); WebKit commit 284730@main "Drop layout containment from container-type"
(Safari TP 206), WebKit bug 292266 (April 2025, stable Safari still affected).
