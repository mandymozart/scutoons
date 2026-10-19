Status 2026-10-19: concept + plan; §8 steps 1–2 and §9 built and live (the endless band, format + migration, Swiper gone). Owner 2026-10-19: Q1 step 1 ships WITH step 2, Q2 drafts first, Q4 keep "Slices", Q7 canvas bands only in the strip editor, Q3 one document; "Szene" stays the group's name, slices name their storyboard shot (§10). Step 3 built on branch claude/slices-rework-fe9a8d (scenes inlined by the migration 2026-10-19-inline-scenes, `ref` / the `scene` kind out of the code, optional `slice.shot`), not pushed. Strip editor drafts (§4.1): D1 = A picked, phone D drawn (§4.1) for the owner's look; then §5 steps 1 + 2 together. GitHub #44 (FEATURES 10.100).

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
  a Swiper group = the slide on all four sides – until Swiper goes (§8); the endless strip = the band's top / bottom.

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

### 4.1 Drafts (2026-10-19) – agents/research/strip-editor/

Static HTML in the creator's tokens with real ch2 slices (index.html shows all; screenshots at 1440 × 900 and 390 × 844):
`desktop-a/b` (a vertical group, a box selected across a seam, a collision), `slices-a/b` (reorder, the slice menu,
files dropped on a seam, the Slice tab), `desktop-endless` (a horizontal / endless group), `phone.html#a|b[-open]`.
What both variants share:
- **One strip per group** at a zoom (fit = the area), the slice image numbers at the seams; the ActionBar's
  ‹ Slice n / m › scrolls the strip (n = the slice in the view's centre); the tool strip stays (text · item · piece ·
  sound · event).
- **The inspector:** Group (label, layout "Vertical · Horizontal · Endless", end, continues, the group's script) ·
  Layers · Audio · Events · Slice. Layers lists the group's stack **under slice headers** – the anchor is visible
  without a field; a selected overlay shows an anchor chip ("⚓ Slice 39") and the hint "moves with its picture; drag
  it across a seam = re-anchored, the id stays". The Slice tab = the slice in the view's centre or the one picked:
  picture slot + Replace … + cut, sensitive, music volume, whole-slice events.
- **Collisions** (the later slice's box over a crossing one): a dashed `--state-warning` outline + "⚠ covers the text
  of 39" on the canvas, the ⚠ in Layers, and in the covered box's inspector "Bring to front" (= re-anchor to the later
  slice).
- **The slice menu** (right-click / long-press / •••): Replace picture … · Insert before … · Insert after … · Replace
  from here (range) … · Cut a tall picture here … (step 4) · Copy · Paste · Remove slice …; files dropped on a seam
  insert there ("3 pictures after slice 40 → 41, 42, 43, the rest move on").
- **Horizontal / Endless:** the same editor laid sideways, screen high, Shift + wheel / ← →; Endless shows the first
  slices dimmed after the last – the wrap seam is open like any other; Horizontal (pinned) has its strip end there.
- **Pieces crossing a seam:** drawn in the drafts. In the reader a piece lives in its slice's blend group
  (`.slice-media`, isolated, clipped to the picture – DESIGN "Collage pieces"); letting it cross needs the group's
  pictures + pieces as ONE blend group – checked in step 1's build, else pieces stay clipped (the owner decides then).

**D1 – where slices are managed:** **A** a filmstrip of thumbnails under the open group in the sidebar (click =
scroll there, drag = reorder, files onto it = insert), seams across the art by a View toggle; **B** a seam gutter
beside the strip (per slice a block: number, grip, •••; a + on every seam), the tree lists groups only, the seam line
across the art only under the pointer. A keeps the art untouched and gives an overview of long groups; B keeps every
slice action where the slice is and needs no sidebar (phones, a folded sidebar).
**D2 – the phone:** **A** the tool strip at the left (as today), the strip ≈ 3.5rem narrower than read, the slice bar
‹ Slice n / m ›; **B** the strip at the phone's full width (= the reader's width), the tools move into the slice bar
(tools · undo · redo · ⚠), scrolling only by finger, the seam marks say where you are.

**Picked 2026-10-19:** D1 = **A** (the filmstrip; desktop as drafted). D2 leaning to **B**, with changes ("the bottom
row is already nicer in the current state … only TWO rows", "save and publish could move to the TOP, together with
problems … a menu like in the reader that goes away when scrolling", "the undo buttons can be bottom right", "every
menu with submenus can be reached in the one row by long scrolling") → **phone C** (`phone-c.html`, `#open`, `#add`):
- the strip at full width; zoom = two fingers (pinch around the fingers, two-finger drag pans while zoomed; one finger
  always scrolls or moves the selected thing); while zoomed a "140 % · tap = width" chip bottom left;
- a top bar ☰ · the path · ⚠ n · Save (dot while dirty) · Publish · ••• (history, versions, the reader dock), hidden
  while scrolling down and back on scrolling up (the reader's header);
- two bottom rows: **row 1, the context row** – a round ink + first (Text · Item · Piece … · Sound · Event · Insert
  slices … after the slice in view · Paste), then what the selection offers (Edit text, Bring to front, Cut, Copy,
  Delete …; nothing selected: the slice in view's), scrolling sideways; undo / redo as small round buttons at its
  right end (they don't scroll); **row 2, the sheet's tab row** as today (Group · Layers · Audio · Events · Slice).
  One merged row was weighed: + and the five tabs and undo / redo just fit 390 px, but a selection's actions would
  have no place – two rows it is.

**Phone D** (owner 2026-10-19 on C: "buttons alle als icons ohne borders, unten auch", "Stücke heißen Ornamente",
"das Menü kann komplett einzeilig sein", "unter jedem Icon im Submenü neben dem Öffnen des Inspectors auch … hinzufügen
als auch die Settings – wir werden so kontextueller") → `phone-d.html` (`#pick`, `#open`, `#more`):
- every icon button borderless (top and bottom), so they sit closer and more fit at once; the top bar as in C (☰ ·
  the path · ⚠ n · Save · Publish · •••; ••• = history and versions, the reader preview, show seams, copy / paste);
- ONE bottom row of kinds – Group · Slice · Text · Item · Ornament · Sound · Event (scrolls sideways if more come), the
  selection's kind inverted – undo / redo fixed at its right; no + of its own, no separate tab row;
- each icon opens its submenu above it: the selected one's actions (Text: edit text, bring to front, copy, delete),
  then Open inspector, Add a text, and the kind's settings (text presets; Ornament: colour presets; Slice: replace the
  picture, insert slices …); the inspector sheet opens over the row with the kind as its head;
- collage pieces are called **Ornamente** in the UI (labels without "…").

## 5. Migration path (each step ships alone)

| Step | What | Size | Data |
|---|---|---|---|
| 1 Reader paint rule | §7: `container-type` moves from `.slice` to `.slice-layers` (not to the group – that breaks strip fonts); the strip drops the slice's side clip, a Swiper clips at the slice-layers; strip travel from the frames; fx clip only the media; `sliceEdges` validator messages (window cuts + collisions, nothing for an open edge – owner: a tool); canvas bands in the one-slice canvas (Q7); a check script (both directions × vertical / strip / Swiper × Chromium, Firefox, WebKit + an old-iOS pass), then one real iPhone on iOS 17 / 18. DESIGN.md states the paint order | S–M | none |
| 2 Strip editor | §4; the film strip by shots (§10); re-anchoring on drop; ids from a chapter counter, not the image number; validator: `showTextBox` targets in the group, not only the own slice; the reader's hidden-box set per group (today per slice, viewer slice.svelte `sliceRefs`) | L | none |
| 3 Data hygiene | **built 2026-10-19:** scene `ref` groups inlined into their chapter (migration 2026-10-19-inline-scenes – production: ch2's 3 strips; the scene documents copied to `entity_versions_legacy`), `ref` and the `scene` entity kind removed (content, validator, schemas, server, creator, MCP, sandbox generator, scripts); the optional `slice.shot` (§10) – written by the storyboard compile. The group keeps "scene" in its default label (owner: "Szene" is the group's name). **Instead of slice ids:** pickups keyed by the placement id alone (already unique; a migration of the `picked:` keys in member progress + a one-time localStorage helper) – a small push of its own, after this one | M | 1 migration (+1 for the pickups) |
| 4 Tall pictures | upload one tall image (a storyboard shot up to 1:7) → cut into tiles = consecutive slices sharing its `shot`; "replace range"; the storyboard compile no longer moves cuts off text boxes (research/STORYBOARD.md §2) | S–M | none |

Later, only if needed: a "picture" made of several tiles as one unit (one sensitive flag, one replace) – the anchor
model above doesn't change for it.

## 6. Open for the owner
- **Q1** answered 2026-10-19: step 1 does not ship alone – together with step 2.
- **Q2** answered 2026-10-19: drafts first (phone + desktop) – §4.1; **D1, D2** there wait for the owner's pick.
- **Q3** answered 2026-10-19 ("ja, ein Dokument"): the scene `ref` groups are inlined into their chapters and scene
  documents / `ref` support go – §5 step 3 (built); the data change is a DB migration that runs with the deploy. Later
  the same day: "Szene" stays the name of a group (storyboard and creator), the default label keeps "scene"; he likes
  the shot referencing – §10.
- **Q4** answered 2026-10-19: the UI keeps "Slices".
- **Q5 – Q7**: §7 (Q7 answered 2026-10-19: the canvas bands come only with the strip editor).

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
(The last line only while Swiper lives; it goes with horizontal.svelte – §8.)
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

Found on the way: the fullscreen strip jumping during a screen shake and `sliceInView` picking strip frame 0 – fixed
2026-10-15 (6e4ebf45); the Swiper ones (home `product-slides` leaking `:global(.swiper-slide)`, a slice over 100dvh,
cube / flip observers, Firefox's cube page width) go with Swiper (§8). Untested: any real old iPhone; Safari < 16
ignores `overflow: clip`.

- **Q5, Q6** answered 2026-10-15: Swiper goes; every horizontal group is our strip – pinned or endless (§8).
- **Q7** answered 2026-10-19: no bands in today's one-slice canvas – the seams come with the strip editor (step 2).

## 8. Endless strip instead of Swiper (owner 2026-10-15)

Owner 2026-10-15: "Ditch Swiper. Give our own strip an endless drag mode (loop + momentum, no 3D effects) for the four
ch3–5 groups; research it and write the plan into 10.100 before building" · "since we only use swiping right and left,
we can ditch it altogether". Researched by a workflow (inventory, strip internals, production via MCP, CDN images,
technique sources, a looper prototype; 3 plans + judge; a prototype stress test + a completeness skeptic) in Chromium
140, Firefox 141, WebKit 26 at 390 × 844, 1440 × 900 and landscape / fractional-DPR cases; no real iPhone / Android.
Prototype + tests: the session scratchpad `endless-plan/` (index.html, wrap / seams2 / shift / decode / pinch2 .mjs).

**The four** (ch3 v632 g4 / g9, ch4 v577 g2, ch5 v578 g1): `{loop, freeMode}` only, `continues`, between vertical
groups, 6 slices 390 × 844 = one 2340 × 844 picture, no events / audio / items, 12 boxes inside their slice; ch3 g4 + g9
share a page. Today's Swiper: in the 500 px column, cut at 100dvh, traps the vertical wheel, moves DOM nodes.

| group | reads | wrap seam 6 → 1 (CDN images compared) |
|---|---|---|
| ch3 g4 duel | standoff across a snowy clearing | nearly (horizon matches, a ridge starts) |
| ch3 g9 hall | scene opener, heading on slice 1 | hard cut (a rack) |
| ch4 g2 cat | graveyard, the cat "kept appearing" | hard cut (white ground ↔ gate) |
| ch5 g1 garden | "5 things I see": intro, 5 items at y 70 % | seamless (drawn 360°) |

**Reader** – new `renderer/strip-endless.svelte` + pure `strip-loop.ts` + attachment `strip-drag.ts` (unit tests, listed
in viewer/package.json `test`); frames shared via `strip-scroll.ts`; strip.svelte (already 303 lines) stays the pinned
strip – one component, one job.
- **Where:** in the page flow, full viewport width (`--viewport-width`), ink behind, `min(100svh, 100lvh − 4rem)` high
  (with strip.svelte's vh fallback line; never dvh – the 2026-10-10 jump). Starts with slice 1 at the left; a resize
  keeps the place and stops a running glide / ease (else the old scale is written back).
- **Loop without clones or DOM moves** (Embla's looper technique, no library code): one `translate3d` on the track; a
  frame wraps by `left: k·T` (a transform would be a stacking context that covers crossing boxes) only while it and its
  boxes' overhang are off screen, decided on the device-rounded position. **T and k·T snapped to device px** (T is
  fractional, 2317.78 px: unsnapped, every full turn shifts the picture 1–3 device px – measured), T from the frame
  rects, never `scrollWidth`; rAF only while moving. Measured: frame wraps without a jump (234 positions × case),
  IntersectionObserver exactly 3 entries per frame over 3 turns, no page overflow, CLS 0.
- **Seams:** at fractional DPR (Windows 125–175 %, browser zoom) Chromium and WebKit show hairlines between frames –
  today's strip too (a static flex row), Firefox never; rounding doesn't cure it. Frames overlap by one hairline (a
  token width, like a border) so ink can't show through on light art (ch3 snow).
- **Fits:** Σratio 2.77, overhang 0.46 → it loops while window ≤ 2.31 : 1, the window = viewport width ÷ band height:
  portrait phones and 1440 × 900 (1.72) loop; landscape phones (≈ 2.6), a 1366 × 625 laptop window (2.44) and 21:9 get
  the band narrowed to T − max(W + overhang), centred on ink, still endless (measured: no jump, overlays fine); T ≤
  window: whole and still. Frames on screen: 1.08 (390 × 844), 1.35 (375 × 667), 3.73 (1440 × 900).
- **Drag:** `touch-action: pan-y pinch-zoom` (plain `pan-y` blocks pinch-zoom on the band – measured; box text there
  is ≈ 12 px), 0.5rem dead zone, then 1:1; a swipe whose first ≈ 10 px go sideways belongs to the band for that gesture
  (inherent, Swiper too). After a drag the next click is swallowed (`detail === 0` passes). Focus slides a frame in only
  for keyboard focus, minimally, and any drag stops a running ease (a mouse-down on a button fought the drag – measured).
- **Glide:** last 100 ms velocity, x₀ + vτ(1 − e^(−t/τ)) with **t clamped ≥ 0** (rAF stamps precede `performance.now()`
  in Chromium / Firefox: the first frame stepped backwards), τ 325 ms, time-based, |v| ≤ T/τ (one turn per fling), no
  snap; a touch / wheel / key stops it. Measured: monotone, stops in 2–2.7 s, idle rAF 0. Reduced motion: drag + glide
  stay (like native scrolling); steps and arrivals jump.
- **Wheel:** only a deltaX-led gesture (or Shift) moves it 1:1, axis latched per 150 ms burst; vertical always scrolls
  the page (the Swiper trap goes). The 4rem slack is less than a 100 px wheel notch: about 1 in 3 approaches has no
  stop showing the whole band – accepted (it's a band, not a page). **Keys:** ← → ease to the next frame edge when the
  band has focus. No fullscreen, arrows, icon, Home / End.
- `overflow: clip` (hidden let focus / `scrollIntoView` scroll it); `role=region` + `tabindex=0` + an aria-label (a new
  viewer message); DOM order 1…N. **Images:** all band frames load once it nears, at normal priority (slice.svelte ties
  eager to `fetchpriority=high` – split them), and are `decode()`d then (Chromium painted decoded-async frames black for
  one frame on first show – measured; Firefox / WebKit never). ≈ 71 MB decoded at 3x for 12 frames on the ch3 page.

**Seams and the page**
- At the wrap seam `.slice-layers { z-index: var(--plane) }` in screen rank keeps "later slice wins" (measured 60 / 60);
  inner seams: DOM order. Crossing boxes show once §7 step 1 lands; until then they cut at frame sides, as today.
- No clones: `data-slice-index` unique; events, audio, fx fire once per real entry. `sliceInView` (hit test, 6e4ebf45)
  and fx `inFrame` stay; `holdClip` (cues.ts:80) skips band frames like the strip's.
- **Reading progress:** track-reading's tall-slice rule (`top < 0 && bottom <= viewport`) counts any band frame read
  when the band's top is above the screen – a 3 px sliver counted (measured): band frames use a ratio-only rule.
- **The band's end:** next-bar's `reachedEnd` and a following slice-less script wait for the last slice, which sits off
  screen left of frame 1 – the band counts as passed when its bottom scrolls above the screen (not live for the four:
  each is followed by a group with slices).
- `arriveAt` (arrival, resume, showSpot, Reader dock; +page.svelte ~103–117) gets a `[data-endless]` branch: align the
  band, a `strip-show` event sets offset = −S_i. next-bar `strip` = pinned only; skip keeps `scrollIntoView`.
  `html[data-vn-blocking]`: no drag / wheel / keys. `music.ts` / `samples.ts` rAF fallbacks test left / right too.
  scripts/dev/creator-reader-check.mjs:143 (`viewerScrolledTo`) gets the endless branch.

**Data:** `RendererParams { endless?: boolean }`; autoPlay, loop, flip, keyboard, cube, freeMode, fullWidth leave the
type, the schemas (RendererParams `additionalProperties: false` – schema.js:54 makes every schema open today, so a stale
creator tab or member's MCP copy could write old keys back), `flow.ts` (`'vertical' | 'strip' | 'endless'` +
`rendererOf`) – no aliases. Migration `2026-10-1x-endless-strip.mysql.sql`, every chapter / scene version, per group
index + scene `$.renderer`, JSON paths only (`loop` is also audio / video):
- read booleans as `JSON_UNQUOTE(JSON_EXTRACT(…)) = 'true'` (or `JSON_TYPE`): **MariaDB 10.11's `JSON_VALUE` returns
  `1` for a JSON true** (checked on the dev container 10.11.14) – a `= 'true'` test would match nothing in production;
- only `renderer.type = 'horizontal'`: `fullWidth` true → params gone (**ch2 g0 v633 / v646 and scene-14 v635 carry
  `loop: true` next to `fullWidth` – fullWidth wins**); else `loop` true → `{"endless": true}`; else params gone; any
  other type: params gone. Writes wrapped in `JSON_COMPACT` (JSON_SET / JSON_REMOVE re-serialise with spaces; LIKE
  checks rely on compact JSON, as 2026-10-11 / 2026-10-13 do). Only rows with an old key → a 2nd run changes 0;
- PHPUnit (incl. an untouched audio / video `loop`), dev dry run (the dev DB lacks the four – only production
  `plan_migrations` shows their rows: expect ch3–5 histories, all 40 ch2 versions, scenes 14 / 17 / 18; scene-14 v195
  `{freeMode, loop}` becomes endless in history), then `apply_migrations` (it empties the library cache itself).
  `entity_versions_legacy` keeps old-form rows: an undo of the 2026-10-11 purge would bring old keys back.

**Creator:** one Layout select "Vertical · Horizontal · Endless" (one `doc.set` = one undo; switching to vertical clears
the params) replaces Full width + the 7-field Swiper block and their messages; flow-texts.ts `layoutIcon` (uses
`group_full_width`); `repair-booleans.ts` drops autoPlay `enabled`, flip, cube, keyboard, freeMode, keeps `loop` (audio
+ video) – its test too; canvas bands: slice 1's left neighbour is the last. Rebuild the public MCP bundle
(`npm run build:mcp`, it carries `groupLayout` + validate). **Storyboard:** the four sources (s1c3 scene-28 / 32, s1c4
scene-36, s1c5 scene-38) → `{endless: true}`; compile.mjs:75 stops spreading old params, the in-slide clamp goes;
storyboard.mjs:117 text; make-sandbox gets an endless group. **Validator:** error for a params key other than
`endless`; warn on `endless` off horizontal and when the picture is too narrow to loop on a phone; the `sliceEdges`
part (endless sides all `open` incl. the wrap seam, `slide` gone) comes with §7 step 1.

**Removed with the chapter part:** horizontal.svelte, its gesture svg, the dead `utils/icons.ts`; ≈ −25 kB (−7 kB gz)
per chapter page (rolldown in the scratchpad, not a viewer build). The package left with step 4 (§9): the home
`product-slides` and the ScoreView were its last users (the ScoreView had pulled Swiper core onto every toon page).

| Step | What | Done when |
|---|---|---|
| 1 | `strip-loop.ts`, `strip-drag.ts` + unit tests | green incl. snapped T, the sliver, narrowing, t ≥ 0, equal glide at 60 / 120 Hz |
| 2 | **one deploy:** the band + page wiring (progress, end, arrival, VN block, holdClip), Swiper renderer out, format (type, schemas closed, flow, validator, creator select, storyboard, sandbox, MCP bundle) + the migration, docs (DESIGN 371–382 / 492, DATA 459, STORYBOARD 94 / 789) | `check:endless` (ports the prototype checks) 3 engines × 2 sizes + fractional DPR; check:fx, check:resume, check:creator-reader; check, PHPUnit / PHPStan, builds; dry run rows, then 0; the owner tried it on phone + desktop |
| 3 | §7 step 1 (paint rule) + the crossing part of check:endless + `sliceEdges` | inner + wrap-seam crossing per the contract; one real iPhone + Android |
| 4 | after Q12 / Q13: home slider + ScoreView off Swiper, package out | no `swiper` import, no Swiper chunk |

(Reader and format ship together: separately, the same document changes layout between two deploys and the creator
shows Swiper fields the reader ignores.) **Risks:** one lane at a time on strip.svelte / strip-scroll.ts / cues.ts
(the linked fix session touched them); no real device yet (pan-y pinch-zoom on iOS, the left-edge back swipe); 71 MB
decoded; an old cached bundle shows migrated ch2 strips as a Swiper until reload.

- **Q8** Hall and cat cut hard once per turn: keep Endless, make them Horizontal (pinned), or redraw the seam?
- **Q9** Full width, screen high (box text ≈ 12 px at size 4 on a 390 phone): OK, or raise those boxes' sizes?
- **Q10** answered 2026-10-16 ("a nudge is fine. Also use FX … deactivated when hints are turned off"), built: with
  "Show hints" on, once per band and page view (0.7 s at ≥ 60 % in view; not under `html[data-vn-blocking]`, not while
  it moves; a pointer on it cancels, moving it spends it) a hint line (the trails' colours, blend, glow) sweeps right
  to left across the first slice (arrived at another slice: the most visible one), then the screen bounces sideways (fx `bounceCue`); no icon, the grab cursor stays.
  Reduced motion: the line stands and fades (a fade, like the tap glow, is no travel), no bounce. renderer/
  strip-nudge.ts (when, unit-tested) + strip-nudge-cue.ts; check:endless 13.
- **Q11** Creator: one Layout select "Vertical · Horizontal · Endless" – OK?
- **Q12** answered 2026-10-17 (DECISIONS): the home slider keeps loop + autoplay, on our utility – built §9.
- **Q13** answered 2026-10-17 (DECISIONS): score pages on CSS scroll-snap (replaces 2026-10-14 "use the swiper we got already") – built §9.

## 9. Swiper elsewhere: payload and reuse (2026-10-16)

**Status 2026-10-17: built** (owner: "Go: Swiper raus – Partitur lazy laden, Partiturseiten auf CSS Scroll-Snap,
Startseiten-Slider auf unser Werkzeug."). ScoreView behind `import('@scutoons/reader/score')`, pages on scroll-snap
(`score-pages.ts` ignores in-between pages while a turn is pending); the home slider on strip-loop / strip-drag +
`slides-pager.ts`; `swiper` out of both package.json files. Measured after: see the end of this section.

**How I measured.** `viewer/build` at ed4c780d, no rebuild. Each route's eager set is its static closure from `.vite/manifest.json` plus the `app.js` dictionary. Per file, Node zlib gzip -9 / brotli q11. Our code bundled with rolldown rc.12 (minified) in the scratchpad. My Swiper bundle came within 0.3% of the build's chunk `CZ-exddm.js` (67,976 B, 20.7 KB gz).

**Payload.**
- **Home:** 24.8 KB gz / 22.1 KB br eager, about 9% of the page's JS+CSS. That is core 20.7 + Nav/Autoplay 2.5 inlined in node 3 + core CSS 1.1 + navigation CSS about 0.5.
- **Every toon page:** 21.8 KB gz / 19.5 KB br eager through layout node 2: toon-shell → inventory → item-detail → score-actions → ScoreView. That is 6.6% of the toon page and 6.0% of the chapter. Opening the score adds 0 Swiper bytes; it only lazy-loads Verovio.
- **Ours:** the pager prototype + band code is 3.5 KB gz / 3.2 KB br. With every export of strip-loop, strip-drag and strip-dom it is 5.1 / 4.4. On chapter pages the pager adds about 1.5 KB gz over the band code already there. Ours is about 6× smaller.

**Reuse.**
- **Slides:** the band loop + a snap layer serves them. Prototype `pager.ts`: 242 lines, passed a CDP smoke test. perView, gap, buttons and autoplay must stay in the component, not the utility (that is the clone line).
- **Score:** it does not fit. Bounded mode uses only drag + ease, and a per-page `change(i)` would re-render Verovio and restart the ease on repeat jumps while playing (`follow()` → `show()`).
- **Score instead:** scroll-snap (`scroll-snap-stop:always`, instant `scrollTo` for jumps, `overscroll-behavior-x:contain`). About 0 B, native nested vertical scroll, loses only desktop mouse drag (keys, buttons and pedal already exist). It still needs a guard so smooth scrolling doesn't report intermediate pages.

**Plan.**
- **Now:** dynamic `import()` of ScoreView in score-actions (−21.8 KB gz per toon page). Drop product-slides' `:global(.swiper*)` rules: build CSS 3.BSR_asuy.css carries an unscoped `.swiper-slide{display:flex}` that reaches the score slides after SPA navigation (from the CSS, not seen in a browser).
- **Then, size M:**
  - strip-drag gets `blocked()` in place of `data-vn-blocking`, a wheel settle hook, and keys separate from touch.
  - strip-loop gets `settle()` with a short-swipe rule, `indexAt`, and a settle-once event.
  - product-slides: autoplay held on hover / touch / drag / focus + off with reduced motion, `user-select:none`, a
    width-only ResizeObserver; no buttons – the owner took the control row out after seeing it live (2026-10-17), ← / → on
    the focusable slider instead.
  - check-landing probes rewritten. The utility stays in viewer/.
- **Answers Q12** (loop + autoplay kept, on our utility, not scroll-snap) **and Q13** (scroll-snap, not Swiper or the utility). §8 step 4 then removes the package.
- **Risks:** no real touch, iOS or wheel device tested yet. The slider gap: 16 px from 768 px (as Swiper), 1rem below –
  14 px on phones ≤ 480 px (the root's 14 px), 16 px above.
- **Measured after** (2026-10-17 build on a0743045 + the working tree, same method: start + app + the route's nodes' static
  closure, gzip -9): home 247.5 KB gz (was 265.4), toon page 306.3, chapter 341.5 (was 362.4); no eager file mentions
  Swiper; the score view's own lazy part 9.7 KB gz (7.3 JS + 2.4 CSS) beyond the chapter's set. The deltas also hold
  the commits since ed4c780d (nudge, packing).


## 10. Scene · Shot · Slice (owner 2026-10-19)

Owner 2026-10-19 (relayed): he likes the shot referencing – "Szene" stays the name of a group in the storyboard and the
creator; slices name the storyboard shot they came from; both ways visible.

- **Scene** = a chapter group (`label`, by default `scene-<n>`; the storyboard's `INT. … – NACHT` heading has the same
  number). **Shot** = a storyboard shot (`sb-25-4`: scene 25, shot 4) – one picture of the storyboard, rendered /
  generated / handheld as takes (research/STORYBOARD.md, HANDHELD-CAMERA.md). **Slice** = a tile of the reader's strip
  (390 wide; a tall shot is cut into several).
- **Data (step 3, built):** `slice.shot?: string` (content.ts `Slice`, the schemas, the validator: an id like
  `sb-25-4`). Consecutive slices may share a shot. Optional – hand-made slices have none; the reader ignores it. Written
  by the storyboard compile (scripts/storyboard/compile.mjs: every slice of a shot, the panorama's parts too); the
  Figma paths (figma-reslice / figma-production) will write it once they cut a frame by its shots' rail (they know each
  shot's y from figma-chapter's rail) – open.
- **Creator (step 2):** the film strip (§4.1, D1 = A) groups thumbnails by shot: a shot = one tile row with its id
  (`sb-25-4 · take 2`), its slices under it; hand-made slices stand alone. Replacing a shot (a new take, a generation,
  a re-slice) replaces its slices as a whole: the new picture is cut again (step 4), the overlays keep their anchors – an
  overlay on a slice that no longer exists moves to the shot's slice under its centre (rect converted, id kept, §3). The
  Slice tab names the shot and links to it in the storyboard studio.
- **Storyboard → chapter:** the studio shows where a shot landed: chapter, group, slice numbers (a lookup over the
  chapter's slices by `shot`), and a link into the creator at that slice.
- **Step 4** (tall pictures) cuts by shot: one tall image per shot → its slices, all carrying the shot id.

Sources: WEBTOON Creators Resource Handbook (800 × 1280 per image, auto-slice); Clip Studio "Export Webtoon"; Chrome 129
container-query change (dev.to/michaelcharles); WebKit commit 284730@main "Drop layout containment from container-type"
(Safari TP 206), WebKit bug 292266 (April 2025, stable Safari still affected).
