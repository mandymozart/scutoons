Status 2026-10-20: concept agreed – every owner question answered (§8); issues filed under #96: #107–#109 for chapter 3's first cut, the rest after them (chapters 4–5's storyboards on whole slices now, Q5). Nothing built yet. No stored format changes, no migration.

# SLICE-GRID — one slice size across the platform

Owner 2026-10-20: "i think we have start a big task enforcing multiples of 1:1.6 across storyboard studio reader and
creator and importer and exporter. prompts etc. it seems like this can be rather confusing other wise … already we got
issues here. some concept for the slices inside our platform allows for various heights of a slice, but export doesnt.
lets reconsider before we continue"

**In one line:** every slice in the reading column is one unit, 390 × 624 (1:1.6; at 3x it is 1170 × 1872, which is
one WEBTOON image). A taller picture is a whole number of units (1:3.2, 1:4.8, 1:6.4, 1:8) and becomes that many
slices that share its shot. The tools that make pictures snap to the unit: the studio offers only whole-slice aspects,
the cutter fills a group's last slice, generated pictures are cropped to size, and the importer cuts whole units. The
creator and the MCP warn on anything else. The reader changes nothing. Whatever is composed for the screen stays
outside the rule: horizontal frames, full-viewport videos and (recommended, Q6) the key visuals.

**What the grid buys, and what it doesn't.** Export works without it. The planned exporter takes a column of any
heights and cuts it between text boxes (WEBTOON.md §4 step 5). A cut through art is invisible on WEBTOON, because
WEBTOON shows images edge to edge. What the exporter can't do is keep a slice taller than 624 as one image. The grid
makes "one slice = one WEBTOON image" true everywhere, so Figma, the studio, the prompts and the creator all speak one
unit. That is the clarity the owner asked for. Its price is owner art time (§4, Q5–Q7), the scarcest thing until
February (WEBTOON.md §10).

Sources:
- The code at HEAD 24f0ce12. content.ts and validate.ts carry uncommitted edits from the #94 session, so their line
  numbers here are HEAD's.
- The production drafts (catpope ch0 v662, ch1–5 v733–v737, toptoy ch1 v722), read through the MCP.
- Files on disk.

Labels used below: **measured** = counted or viewed; **computed** = arithmetic only; **assumed** = not checked.

## 1. The unit and the rule
| k slices | ratio | 1x (low) | 2x (medium) | 3x (high) | master 6x | WEBTOON, 800 wide | export CSS, 400 wide |
|---|---|---|---|---|---|---|---|
| 1 | 1:1.6 | 390 × 624 | 780 × 1248 | 1170 × 1872 | 2340 × 3744 | 800 × 1280 = 1 image | 400 × 640 |
| 2 | 1:3.2 | 390 × 1248 | 780 × 2496 | 1170 × 3744 | 2340 × 7488 | 2 images | 400 × 1280 |
| 3 | 1:4.8 | 390 × 1872 | 780 × 3744 | 1170 × 5616 | 2340 × 11232 | 3 images | 400 × 1920 |
| 4 | 1:6.4 | 390 × 2496 | 780 × 4992 | 1170 × 7488 | 2340 × 14976 | 4 images | 400 × 2560 |
| 5 | 1:8 | 390 × 3120 | 780 × 6240 | 1170 × 9360 | 2340 × 18720 (PNG) | 5 images | 400 × 3200 |

1. **A column slice is exactly one unit:** 390 × 624 at 1x, with its `.2x` and `.3x` files at double and triple size.
   A column slice is a slice of a vertical group, or of a horizontal group's vertical version (WEBTOON.md §8).
2. **A picture is k whole units, k ≥ 1.** There are no half units and no leftovers. The picture becomes k consecutive
   slices with one `shot`:
   - the storyboard shot id; or else
   - the file stem as a slug: lower case, every character other than a–z, 0–9, `_` and `-` turned into `-`, at most
     64 characters.

   A raw file stem fails: `slice.shot` must match `/^[a-z0-9][a-z0-9_-]{0,63}$/` or it is an error (validate.ts:669,
   :680), and the owner's file names have capitals (`Vienna_Church_Aerial_1-48_…`). This is TEXT-ACROSS-SLICES.md §5
   step 4 ("tall pictures … cut into tiles") with the tile fixed at one unit. If Q3 picks "one slice of k units", rule
   1 reads "a whole multiple of 624", and §3.4 and §3.5 change where they mention Q3.
3. **Tolerance 0.5 %.** A picture within 0.5 % of k units is resampled to exactly k units (#78 did 625 → 624, a 0.16 %
   change). Anything further off is off the grid, and nothing resamples it: 681 → 624 would be 8.4 %, which shows.
4. **The UI says "slices"** ("1:4.8 · 3 slices"), never "grid". The creator's box snap is already called `GRID_SIZES`
   (chapter-state.svelte.ts:16).
5. **The width stays 390** (today's warning, validate.ts:678).
6. **How tall:**
   - generated pictures: plan up to 2 slices (`SAFE.vertical` plans up to 1:4, lib.mjs:48);
   - the owner's art: up to 5;
   - 1:8 is past what the generators have proved (1:7), so the storyboard warns on it, as it does today.

**Why one unit per slice, not one slice of k units (Q3):**
- **Reader.** Whole-slice events, music, samples and player autoplay wait until half the slice is visible (threshold
  0.5: slice-events.ts:109, music.ts:73, samples.ts:66, slice-video.svelte:200).
  - A 3-unit slice is 1872 px tall. Half of it (936) never fits on an 844-px phone (computed). A 2-unit slice would
    still fire.
  - `EAGER_SLICES = 2` counts slices, not pixels (viewer slice.svelte:116).
- **Export and creator.** One slice is always one image. The film strip already draws every slice as a 390 / 624 cell
  with `object-fit: cover` (slice-list.svelte:342-350).
- **The price.** A tall picture is replaced, flagged sensitive or deleted as a range of k slices. #44 step 4 already
  plans "replace range", and the cut copies `sensitive` to every slice.

One unit is 74 % of a 390 × 844 phone screen (computed). So "one screen" is not a unit, and things composed for one
screen stay outside the rule (§2).

## 2. Where it applies and where not
| Thing | Rule | Why |
|---|---|---|
| Slices of vertical groups and of vertical versions | on | they are the column that readers scroll and that WEBTOON gets |
| A framed video in the column (`video.frame`) | on, through its frame: 5:8 = 1 slice | the frame sets the slice size (`fillSliceSize`, creator images.ts:74-98); its fit decides crop or bars |
| Key visuals (ch1 s0, ch2 `keyvisual`: 390 × 845 = 6:13) | outside in the reader; the export crops a key visual composed with the centred 5:8 band to it (Q1: ch3 on); ch1 / ch2 per Q6 | composed for one phone screen. The coming-soon teaser shows the same media full screen (chapter-teaser.svelte:16, "Portrait windows crop to fill"). The export cuts each one once, invisibly |
| Full-viewport video (`video.viewport`, ch0) | outside | its box is the screen (content.ts:204-206); the stored height is only the poster's |
| Pinned-strip and endless-band frames | outside | drawn at the screen's height (viewer strip.svelte:12-14, `lvh`). Their ratio only counts how many screens wide they are, and they are never exported (the vertical version is). A 5:8 frame would be 1.35 screens wide on an 844 phone (computed) |
| Storyboard `loop360` and `strip` (the sideways pass) | outside | they become those frames; their slides stay 390 × 844 (compile.mjs:59-64) |
| Covers, thumbnails, map tiles, place images, item icons | outside | not slices; their sizes come from WEBTOON (1080², 1080 × 1920, 202 × 142) or from their own screens |
| toptoy's choice cards (390 × 390 and 390 × 780) | on, with a warning (Q4) | slices of a vertical group, though not in the WEBTOON export |

## 3. What changes, place by place
Each place gets one of three levels:
- **snap:** the tool only makes whole slices;
- **warning:** shown, and can be accepted per slice (RULES "Warnings can be accepted");
- **error:** blocks saving.

### 3.1 Storyboard: numbers, studio, validator (issue E, after Nov 15)
- **One source for the numbers.** `UNIT = 624`, `slicesOf(h)` and the slice aspects live in
  tools/storyboard-render/spec.js (plain ESM, so the studio and Node share it). lib.mjs re-exports them. Today 390,
  844 and 624 each have three copies (lib.mjs:19-21, spec.js:5-6, content.ts:195).
- **Studio (snap).**
  - Today the Aspect list offers 11 aspects, and none of them is whole slices (`VOCAB.aspect`, studio.svelte.js:10).
  - New for `viewport` and `vertical`: 1:1.6 · 1:3.2 · 1:4.8 · 1:6.4 · 1:8, each labelled with its slices.
    `viewport` = 1 slice; `vertical` = 2 or more.
  - `loop360` and `strip` keep their aspects.
  - A stored old value shows as "9:16 – not whole slices" until someone changes it.
  - studio.svelte.js is also #62's file, so coordinate with #62's branch.
- **Heights.** `shotHeight` stays as it is (lib.mjs:77-82). Decimals already parse (:70-73), so 1:3.2 gives exactly
  1248.
- **Validator (warning).**
  - storyboard.mjs warns on a viewport or vertical shot that isn't whole slices.
  - Its rule at :108 ("vertical … wider than a screen – a viewport?") becomes "under 2 slices – a viewport?"
    (threshold 1:3.2).
  - Its rule at :119 ("a viewport is 9:16") moves to 1:1.6.
  - `SAFE` stays. It records what the generators have proved (lib.mjs:42-48), and 1:8 is already beyond its 1:7.
- **compile stays** at one draft entry per shot. figma-chapter places one picture per entry (figma-chapter.mjs:71-79),
  and the cut makes the slices. A compile draft saved directly through `save_draft` would carry a k-unit slice; the
  validator warning (§3.5) names it.
- **Chapters 4–5 (Q5).** If the owner says yes:
  - a one-off script rewrites their 70 viewport and vertical shots;
  - the 3D frames are re-rendered ("All frames");
  - their Figma frames are placed again. Those frames exist, with the owner's pictures sorted beside them (f33cec1e,
    a6b6d698).

  Chapter 3 is not rewritten (§6).

### 3.2 Figma and the production cut (issues A, B, J: before ch3's first scene cut)
**The slice lines are wrong today (measured).**
- figma-chapter.mjs:81-83 draws a slice line every 624 px from the frame top.
- figma-production cuts each vertical group from that group's own top (:119-121), and cuts a band as one 844-high row
  (:113-117).
- ch3 scene-25 is 7452 px = 11 × 624 + 588. So from scene-26 on, every line the owner lays ch3 out against misses the
  real cut, and each band shifts the lines again.
- figma-draft.mjs:54-56 draws one frame per scene, so its lines are off only for a group that follows a band.

**A: lines per group, drawn without touching the frame.** There is no guides-only run today. figma-chapter sends the
bridge's `draft` command (:91, :97), and that command:
- finds the frame by name (code.js:33). The default name carries the run's date (figma-chapter.mjs:29), so a re-run
  makes a new frame;
- resizes the frame and all four layers to the draft's height (:41-47; the frame clips its content, :38);
- re-stacks the layers (:53);
- deletes the `text` nodes of every shot it places (:77). Those are the texts as the owner edited them.

New: a command module, tools/figma-bridge/commands/slice-guides.js.
- It uses the runtime modules of d4b3b216. The plugin at HEAD (VERSION 2026-10-20.3) loads modules, so the plugin
  core needs no change; `ping` shows whether the running plugin does.
- It finds the frame by its exact name and redraws only the children of `guides` and the rail:
  - a line every 624 px from each vertical group's top;
  - an "end · +r px" line at each group's end;
  - the rail label "scene-25 · 11 slices + 588 px".
- It never resizes or re-stacks anything, and never touches `storyboard`, `text` or `layout`.
- figma-chapter and figma-draft get the same per-group lines for later runs.
- code.js, relay.mjs and cli.mjs carry another session's uncommitted edits. A module avoids touching them.

**B: the cutter's group end (figma-production.mjs:119-123), per Q2.** The recommended version:
- It still cuts every 624 px from the group's top.
- No more joining (`MIN_REST` 160, :57, :122). So no more 669–745 px slices, each of which is two WEBTOON images.
- The last piece (r < 624 px) is **dropped** when its pixels are one colour and no text top lies in it. The text check
  matters because the export hides `text` (:172), so the colour test alone would drop a box.
- Otherwise the piece is exported r high and **padded** below to 624 (in the PIL step, lib.mjs:454-462). The pad
  takes the bottom row's colour when that row is one colour, else white.
- Texts are measured against 624 (`dialogFor` divides by `slice.h`).
- A text of a scene group that lands on no slice is listed. Today it is skipped silently (:126-128).
- The run lists every pad and every drop.
- The header's "cut at shot boundaries" (:25) is wrong, because the cut runs straight through shots. It is corrected.
- What the owner can do in the main frame: leave the space under a group's last line empty, and it is dropped;
  otherwise it is padded. Painting into a pad would need per-slice frames, which were retired the same day
  (DECISIONS:16), so `--figma-slices` stays as it is.
- Fallback: today's cutter (the commit before B). The chapter is never blocked (STORYBOARD.md:625-626: "if a
  milestone slips, that chapter is made the chapter-2 way").

**What B means for ch3.** These figures are computed from the storyboard's group heights at 624; the final art
decides.
- The 11 vertical groups end on pieces of 588, 450, 528, 60, 294, 51, 354, 60, 363, 597 and 432 px.
- The two 60-px ends are white (measured on the 625 masters), so they are dropped.
- That leaves 9 pads of 27–573 px, 1959 px in all: about 2.3 phone screens of flat colour at scene changes.
- The worst case: scene-29 ends in 51 px of art and 573 px of pad.
- A pad is not the rhythm model's planned scene air. `airScene` (292–487 px) is used by no code (lib.mjs:36); a pad
  is just air at a scene change.

**Where ch3's slices come from.** All 11 groups are planned to go through this cutter and `production-upload.mjs`.
But #67 also names Import slices, which takes any height until G ships: a changed height only asks for confirmation
(slice-import.svelte:191-201). So a ch3 slice replaced by hand must be 1170 × 1872 (a line on #67).

**J: endless bands are 2340 px wide, not 2532 (a bug; bands are outside the rule).**
- compile cuts a panorama into n = round(w / 390) slides (compile.mjs:61). A 3:1 panorama at 844 high is 2532 wide,
  so it gets 6 slides = 2340 px (computed).
- figma-chapter draws the band 2340 wide (:65) and `FILL` centre-crops the picture into it (code.js:83). Each wrap
  edge loses 96 px, so the endless join no longer matches.
- The cut takes 6 frames (figma-production.mjs:113-117).
- All four bands are 3:1 (measured: sb-28-4, sb-32-1, sb-36-4, sb-38-11). ch3's scene-28-duel and scene-32-hall have
  6 slides each in chapter.draft.json.
- For ch3: look at one band on screen. Before the band art is made, tell the owner the band is 2340 × 844 at 1x
  (7020 × 2532 at 3x) and must wrap at that width.
- The compile fix lands before ch4–5's band art.

### 3.3 Generators and prompts (issue F, after Nov 15)
- **Today nothing fits a generated picture to its slot.**
  - figma-chapter places the picture at the slice's w × h (:76), and Figma `FILL` centre-crops it (code.js:83).
  - Luma gets the nearest of nine ratios by log distance (comfy.mjs:285-291).
  - Gemini defaults to `gemini-2.5-flash-image` (gemini.mjs:28), which has no tall ratio.
  - So a head or feet can be cut off without anyone noticing.
- **Fit step (snap, new).** When a picture is approved (in the studio or the generate scripts), it is cropped to
  exactly 1170 × 1872k, anchored by the shot's `air` / figure y, and then placed. Never `FILL`.
- **Routes for 1 and 2 slices** (output sizes measured in storyboards/_out/*/generated):

| k | Route | Result |
|---|---|---|
| 1 | Luma at 9:16 (1504 × 2672) → crop to 1500 × 2400 → × 0.78 | 1170 × 1872; 10 % of the height cut |
| 1 | Luma at 2:3 (what comfy.mjs already picks for 1:1.6; its size not measured) → crop about 6 % of the width | to be measured |
| 2 | Luma at 1:3 (1120 × 3360) → crop the width to 1050 → × 1.114 | 1170 × 3744, slightly upscaled |
| ≥ 3 | The owner's art. For a generated one, test calls decide: Luma `image_edit` keeps an exact-size source's size (documented); Gemini 3.1 has 1:4 and 1:8 (documented, never run here, pixel size unknown) | – |
| video | Runway 720:1280 (runway.mjs:26) in a 5:8 frame, "Fill (cropped)" | 10 % of the height cut, no re-encode |

- Luma's upscale reaches the 2340 master (owner 2026-10-20 for ch2's vertical pictures: "all need to be upscaled and
  normalized to 2340 pixel width at 1:4.8 ratio"); without that step a generated picture's master is its 3x. (RULES "Product rules" names masters
  for new art; generated pictures are the exception.)
- **Prompts speak in slices.**
  - Today prompts.mjs:84-88 says "Vertical phone frame 9:16" and "taller than Luma's native 9:16", although Luma's
    list goes down to 1:3.
  - New wording: "k slices of 1:1.6, read top to bottom".
  - The same wording goes into workflows/scenes/storyboard.md:60-66, storyboard-taste.md:57, generate-shots.md:62-73,
    storyboards/style/luma-skills/storyboard-frame-briefs.md, and STORYBOARD.md §1 and §3.
  - The docs disagree on the generators' limits today. They get one line: `SAFE` in lib.mjs, which STORYBOARD.md:40
    and prompts.mjs:87 then cite.
- **Segments** (STORYBOARD.md:295) overlap by 25 %, so their seams fall every 468 px, not on slice seams. The rule "no
  figure and no text box on a seam" then applies to both kinds of seam.
- **Test spend:** a handful of calls, under €1 in all (Luma lists $0.04–0.10 per image).
- ch3's gap prompts (#61) keep their slots' aspects: they fill the owner's layout, and that layout stays.

### 3.4 Creator: the importer and the strip editor (issues G, H)
- **A height is never typed in.** `fillSliceSize` writes the image's own size, or the frame's for a framed video
  (images.ts:74-98). So the creator acts on pictures, not on numbers.
- **Import slices** (slice-import.svelte; issue H = #44 step 4).
  - Each row says what a file becomes: "1170 × 5616 → 3 slices", or "1170 × 4717 → 2.52 slices, not whole".
  - Whole units (± 0.5 %) are cut into k slices with one `shot` (rule 2). Every density is cut from the top one, so
    the 3x tiles are exactly 1170 × 1872. Today a 1170 × 4715 file rounds to 780 × 3143 at 2x (media-upload.ts:9-11).
  - A picture that isn't whole slices is imported as one slice with the warning. It is never cut, padded or cropped:
    where a picture ends is the artist's call.
  - If Q3 picks "one slice of k units", there is no cut.
- **Replace.** Today a height change only asks for confirmation (slice-import.svelte:191-201) and keeps every overlay
  in %, so overlays drift by Δh / h. New: overlays keep their px offset from the picture's top (§5), and a taller
  replacement becomes k slices ("replace range").
- **Other ways in:** a slot drop, Add slice and the MCP's `save_draft` take any height. Only the warning (§3.5) names
  an off-grid slice that comes in this way.
- **Film strip and strip canvas: nothing to change.** The cells are already 390 / 624, the slices of one picture show
  as one shot (#44 step 2c), and a picture's seams are slice seams.
- **Frame presets** (content.ts:259-264):
  - 'slice' 5:8 stays;
  - 'keyvisual' 6:13 stays while key visuals are outside the rule (Q6);
  - 9:16 and square leave the list. Stored frames keep working, and the warning names them.
- **Size line** (slice-media.svelte:86-87): it says "390 × 845 · 1.35 slices – not whole slices" instead of
  "standard 390 × 624".
- **Seeing it** (owner, Q4): in the film strip and the strip canvas a slice shorter than whole units shows the missing
  part below it as an orange / red offset, and its slice marker (the number) gets the same colour; a slice taller than
  whole units that isn't a multiple only colours its marker. Accepting the warning keeps the marks (they show a fact,
  not a to-do). The colour comes from the tokens' warning colour; a design draft on the real page first.
- **Four texts say "other heights are fine".** They state the rule instead: content.ts:193, creator defaults.ts:5, the
  `format_hint` message (creator/messages/en.json:495 and de) and the MCP's resources.ts:245.

### 3.5 Validation, MCP, server (issue G)
- **Validator (warning).** Today `checkSlice` has no height rule (validate.ts:671-680).
  - New: a warning on every column slice that isn't 624 high: "slices are 390 × 624 – this one is 390 × 845 (1.35
    slices): cut a taller picture into whole slices, or fix the picture". If Q3 picks "one slice of k units", the
    warning is for a height that isn't a multiple of 624.
  - A framed video is judged by its frame (5:8).
  - Skipped: key visuals (a 6:13 frame or a video slice at 6:13, per Q6), viewport videos and horizontal frames.
  - The slices of a vertical version (`checkVertical`, WEBTOON §8) get the same rule.
- **A warning, not an error** (Q4). Once the makers snap, off-grid slices only come in by hand. An error would block
  toptoy (34 slices) and ch4–5's storyboard drafts (6 rests) until their new art arrives.
- **MCP.** `validate` gets the warning for free, because it uses the same validator. resources.ts:245 and the
  workflow rules gain one line: "a tall picture is k slices".
- **Server: nothing.** There is no height rule in server/schema/content, and the shared validator stays the one place
  for the rule.
- content.ts and validate.ts are open in the #94 session (uncommitted). G starts after that work lands, and deploys
  outside ch4's freeze (~Dec 10–12).

### 3.6 Reader: nothing changes
With every column slice one unit:
- the 0.5 triggers fire (§1);
- `EAGER_SLICES = 2` keeps covering about the first screen and a half;
- `.slice { min-height: 200px }` (shared/reader/src/slice.svelte:178) never applies;
- the image's natural size (`height: auto`, :196) equals the stored size, so nothing shifts after loading.

Strips and bands are outside the rule and unchanged.

### 3.7 Exporter (#77): nothing changes
WEBTOON.md §4 step 5 stays as planned. Its dynamic programming (fewest pieces; a cut on a seam costs 0):
- already returns exactly the seams for content on the grid;
- cuts elsewhere only where a box crosses a seam;
- keeps "a cut through a box never happens" for anything off the grid.

Two notes for WEBTOON.md:
- In export mode a framed video's poster needs the frame's fit. The slice is passed without `video`, and the image
  keeps `height: auto` (slice.svelte:196).
- On the grid an episode is whole slices, so the end card (§5 "Endings") is one more full unit.

### 3.8 Found on the way (real whatever the owner decides)
- The Figma lines miss the real cuts (A).
- figma-production's header is wrong, and the cutter drops a stray text silently (B).
- The endless bands lose 96 px at each wrap edge (J).
- A replace that changes the height leaves overlays in % (§3.4).
- The docs disagree on what the generators can make (§3.3).

## 4. Off-grid content today and its way onto the grid
Measured 2026-10-20 on all 490 slices (catpope ch0–5 and toptoy ch1; yellowmouse has no chapters).

| Category | Count | 1x height | Where | Way onto the grid | Who / when |
|---|---|---|---|---|---|
| Standard slice | 368 | 624 | ch1 94, ch2 86, ch3 86, ch4 61, ch5 41 | already on it, in drafts v733–v737; the published versions are still 625 (spot-checked ch3 v724, ch4 v577) | the owner publishes (#78) |
| Key visual (looping video) | 2 | 845 (6:13) | ch1 s0 (no frame), ch2 `keyvisual` (frame 6:13, cover) | outside the rule (recommended, Q6); else a 5:8 frame or new art | after Nov 15 (C) |
| Group-end rest | 12 | 188–589 | ch3 8, ch4 2, ch5 2 | storyboard placeholders: the cut of the final art fills them (B) | with each chapter's art |
| Joined rest | 5 | 669, 681 ×3, 745 | ch3 3, ch4 2 | the same | the same |
| Pinned-strip frame | 44 | 693 (9:16) | ch2 scenes 12–15, final art | outside the rule | – |
| Endless-band frame | 24 | 844 | ch3 2 bands, ch4 1, ch5 1 | outside the rule (width bug: J) | – |
| Full-viewport video | 1 | 206 | ch0 | outside the rule | – |
| toptoy card | 34 | 390 (32), 780 (2) | toptoy ch1 choices, not in the WEBTOON export | warned; new art at the workshop (Q4) | – |
| **Total** | **490** | | | **368 on, 71 outside (key visuals per Q6), 51 off** | |

**ch2's vertical pictures.** These are from the owner's zip `CAT_POPE_-_AWAKENING_S1C2_Anger(1).zip` (PNG headers
measured; not in the DB yet). The church and the alley came in several lengths:

| Picture | Versions delivered | On the grid | Way |
|---|---|---|---|
| Vienna_Church_Aerial (scene-12) | 1-32: 1170 × 3744 · 1-48: 1170 × 5616 · 1-74: 1170 × 8658 | 1-32 = 2 slices, 1-48 = 3; 1-74 = 4.625 | 1-48, upscaled to 2340 × 11232 (owner, Q7) |
| Sunbeam_Alley (scene-14) | 1-48: 1170 × 5616 · 1-74: 1170 × 8658 | 1-48 = 3 slices | 1-48, upscaled to 2340 × 11232 (owner, Q7) |
| Rooftop_Pool_Detail_Enhanced (scene-13) | 2048 × 8256 (1:4.03) | 2.52 slices | re-made at 1:4.8, 2340 × 11232 (owner, Q7) |
| Combo4 (scene-15) | 2048 × 8256 | 2.52 slices | re-made at 1:4.8, 2340 × 11232 (owner, Q7) |
| alley-pano.jpg | 5408 × 768 | – | a horizontal strip, so outside the rule. Its 768 rows are sharp only at 1x when drawn at screen height (a 3x phone needs about 2532, computed). That is a sharpness question, not a grid one |

The scene each picture belongs to is assumed from ch2's unlocks: 12 church square, 13 pool, 14 alley, 15 apartment
blocks.

**Storyboard shots** (repo files, measured): 130 viewport and vertical shots in ch3–5, none of them whole slices, plus
4 loop360 panoramas (outside the rule).

| Aspect today | ch3 | ch4 | ch5 | In ch4–5 it becomes (Q5) | Change |
|---|---|---|---|---|---|
| 9:16 (693) | 31 | 25 | 18 | 1:1.6 (624) | −10 % of the height |
| 1:3 (1170) | 20 | 9 | 5 | 1:3.2 (1248) | +6.7 % |
| 1:4 (1560 = 2.5 slices) | 8 | 8 | 5 | 1:3.2 or 1:4.8, chosen per shot | ±20 % |
| 1:5 (1950) | 1 | – | – | – (ch3 stays as it is) | – |

What the rewrite does to ch4–5 (computed with `shotHeight`, choosing 1:3.2 for the 1:4 shots, bands included):
- ch4 goes from 41,179 to 37,660 px (−8.5 %), and ch5 from 26,968 to 24,556 px (−8.9 %). `TARGET` is 60–80 screens
  (lib.mjs:40).
- ch4 and ch5 have 4 and 2 vertical groups, so the rewrite saves at most 6 pads.
- Its other gains: cuts fall on shot boundaries, and generated pictures are made to size.

## 5. Keeping text boxes, items and events on their art
- **Everything on a slice is placed in % of that slice:**
  - a box's `rect.y` and `height`, a tail tip, item rects and area-event rects (dialog-tail.ts:3-4,
    slice-edges.ts:110-121);
  - sound areas;
  - collage pieces, in % of the picture and clipped to it (content.ts:433-435, collage.ts:139-144).

  x, width and `cqw` sizes follow the width, and the width never changes.
- **One helper, moved rather than written again.** `convertPoint` / `convertRect`
  (creator/src/lib/editor/strip-anchor.ts:22-36) convert % between two slices' boxes. They move into
  `@scutoons/content` (RULES "Code": a move, not a copy). The tile cut and the replace both use them.
- **Per way onto the grid:**
  - **The cutter's pad: nothing to remap.** Texts are measured against 624 at the cut. The cutter writes only image,
    size, variants and dialog (figma-production.mjs:211-217), so ch3's items and unlocks (#69) are placed later in the
    creator, on the uploaded slices.
  - **Cutting a picture into slices:**
    - each overlay goes to the slice that holds its anchor, with its px offset from the picture's top kept;
    - an overlay may cross a seam (#44's paint rule);
    - an event without a rect stays on the first slice;
    - `sensitive` is copied to every slice;
    - a collage piece that crosses a seam is split into two pieces that meet there.
  - **Key visuals:** neither has any overlays (measured).
  - **ch4–5's rewrite:** there is no owner layout yet; figma-chapter lays the texts out again from the shots.
- **Never:** a resample of more than 0.5 %, a crop or pad nobody saw, or overlays kept in % across a height change.

## 6. Order of work around chapter 3
Chapter 3's dates (#64): first final scene cut about Oct 26; key visual and coming soon by Sun Nov 1 (#70); art freeze
Tue Nov 10; deploy freeze Thu Nov 12; release Sun Nov 15. The owner is laying ch3 out in Figma on the storyboard's
first throw now (#67).

**Chapter 3's slices follow the grid; its shots don't.**
- Every ch3 slice comes out 390 × 624 through B. By hand, only 1170 × 1872 files go in.
- The shots, the Figma frame and the owner's layers stay where they are. Re-gridding the shots would move every shot
  below a change, while the owner's `layout` and `text` layers stay put (figma-chapter.mjs:12; figma-production reads
  the texts from Figma).

| When | Step | Issue | Size |
|---|---|---|---|
| by Sun Oct 25 | Q1 (ch3 key visual) and Q2 (ch3 group ends) answered | #96 | – |
| before ch3's first scene cut (~Oct 26) | lines per group, through a guides-only module | A | S |
| before ch3's first scene cut | the cutter per Q2 | B | S |
| before ch3's band art (scene-28, scene-32) | the band width checked on screen, the owner told | J | XS |
| week of Oct 26 | ch3's key visual composed per Q1 (owner) | a line on #70 | – |
| when #96 is answered | the docs: DECISIONS lines (+ "superseded in part" on :21 and :24), RULES "Product rules", ASSETS "Slice resolutions", WEBTOON.md, TEXT-ACROSS-SLICES.md §5 step 4, STORYBOARD.md §1 and §3 | #96 | XS |
| after Nov 15, before ch4's layout | the storyboard on whole slices (Q5); the band fix in compile | E, J | M |
| after Nov 15 | generated pictures fitted, prompts in slices (test calls first) | F | M |
| after the #94 session's work lands, outside ch4's freeze | the rule in validate, the creator and the MCP | G | S |
| after G | tall pictures as whole slices (#44 step 4) | H | M |
| with the WEBTOON build (Nov 15 – Feb) | key visuals (Q6); ch2's vertical pictures (Q7) | C, I | XS |

- Before Nov 15: scripts and notes only. No deploy, nothing reader-facing, no ch3 shot moved.
- **Capacity after Nov 15.** E, F, G and H run alongside ch4 (Dec 15), ch5 (Jan 15), the WEBTOON build, performance
  mode from Nov 16 (DECISIONS:20) and #62. None of them is needed for the Feb 28 launch (the exporter cuts off-grid
  slices), so the owner orders them.

## 7. Left out, and why
- **Half units (312 px, 1:0.8) or a 39-px step.** They would absorb 1:4.03 (→ 1:4.0), 1:7.4 (→ 1:7.2) and the 21
  shots at 1:4. But the owner said multiples of 1:1.6, and a half unit is half a WEBTOON image. "One slice = one
  image" and episodes of whole 1280s would both end.
- **Slices of k units:** from 3 slices on, they break the reader's half-visible triggers (§1, Q3).
- **Horizontal frames on the grid:** they are measured in screens wide, not slices tall (§2). ch2's 44 frames also
  have no lossless masters (frames 109–152 are not in `_out/s1c2`; the CDN has 2x at most, 780 × 1387, measured).
- **Changing the exporter:** the planned one already cuts on seams (§3.7). A fixed "seams only" rule would fail on two
  neighbouring crossed seams, and would cut through boxes on an off-grid slice.
- **Cutting k-slice shots in compile:** figma-chapter places one picture per compile entry (§3.1).
- **Fixing off-grid pictures automatically in the creator** (pad, crop or stretch): composition is the artist's call.
  Only the cutter pads, because there the piece is air at a scene change.
- **Re-gridding ch3's shots or its Figma frame:** it would move the owner's layout three weeks before the freeze (§6).
- **Fixing the 17 storyboard rests now:** they are placeholders that the final art replaces. Fixing them now means
  about 2.7k px of pads, 12 boxes and 3 items to remap, and 3 new WebPs per slice (measured), all thrown away later.
- **A migration or a schema rule:** nothing stored changes, and the shared validator is the one place for the rule.
- **A 5:16 frame preset:** no 2-slice framed video exists. Add it when one does.
- **Painting into pads in Figma:** per-slice frames are retired (DECISIONS:16).
- **Generator routes for 3–5 slices:** generated pictures are planned at up to 2 slices. Test calls come when one is
  needed (§3.3).
- **Where masters live, and a 4x level (1560):** this isn't grid work (the grid works with either answer). It is split
  from #96 into its own issue (K).

## 8. Owner questions (with recommendations)
**By Sun Oct 25 (chapter 3):**
1. **Chapter 3's key visual** – answered (owner 2026-10-20: "i like the crop for export"): composed for the screen (6:13,
   390 × 845 at 1x) with everything that matters inside the centred 5:8 band (the middle 624 rows). The reader and the
   Nov 1 teaser show it whole; the WEBTOON export crops it to the band = exactly one WEBTOON image.
2. **Chapter 3's group ends at the cut.** On today's storyboard the 11 groups end on pieces of 51–597 px. Today's
   cutter keeps them as short slices and joins ends under 160 px into 669–745 px slices (two images each). Options:
   - (a) Never join, and keep the short ends. Each fits one WEBTOON image but isn't a whole slice, and one of them
     would be a 51-px slice.
   - (b) You end each group on a slice line. The new lines (A) show where, and an empty space under the last line is
     dropped.
   - (c) The cutter pads every end that has art to 624, in its bottom colour (else white). That is about 9 pads,
     1959 px: about 2.3 phone screens of flat colour at scene changes.
   - The fallback for all three is today's cutter.
   - Rec.: (c), with (b) wherever it suits the art. Every ch3 slice is then 390 × 624.
   - Answered (owner 2026-10-20: "Q2 b and c are fine"): (c), with (b) wherever it suits the art.

**The rest (answered the same day):**
3. **A tall picture** – answered (owner 2026-10-20: "agreed"): k slices of one unit each, sharing its shot.
4. **Warning or error** – answered (owner 2026-10-20: "agreed also maybe we can visualize it with a orange or red offset
   for if it is smaller and the same color on the slice marker. if it is longer same thing applies but just the slice
   marker gets the color indication"): a warning, accepted per slice, on every toon; in the creator a slice shorter than
   whole units shows the missing part as an orange / red offset and its slice marker in the same colour; a taller one
   that isn't whole units only colours its marker (§3.4).
5. **Chapters 4–5** – answered (owner 2026-10-20: "we do it now. layouting in figma hasnt started for sure."): their
   storyboards move to whole slices now, before their Figma layout (E).
6. **Key visuals** – answered (owner 2026-10-20: "key visuals stay outside the rule. key visual i anwsered with q1."):
   outside the rule in the reader (6:13, made for one screen); the WEBTOON export crops each to its centred 5:8 band
   (Q1). ch1's and ch2's weren't composed with the band: the crop preview shows what each loses (ch1: the figure at
   the top right or the cat at the bottom, viewed) – the owner looks before the first export (C).
7. **ch2's vertical pictures** – answered (owner 2026-10-20: "all need to be upscaled and normalized to 2340 pixel width
   at 1:4.8 ratio"): all four scenes at 1:4.8 = 3 slices each, masters 2340 × 11232 (Luma's upscale); church and alley
   from their 1-48 versions, the pool and Combo4 re-made at 1:4.8.
