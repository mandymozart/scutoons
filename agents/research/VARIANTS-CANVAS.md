Status 2026-10-20: concept, all owner questions answered (§6; Q4 not as recommended: one band per picked shot now, 3.2) – being built in stages (owner 2026-10-20: "bitte gern schon anfangen mit dem Studio"): phase 1 (the record, §3.7 – studio README "The record") and stage 2 (the board, 2a + 2b – studio README "The variants board"; the Variants panel retired) built on the branch, the conversion of the owner's logs not run yet; next: phase 3 (Improve on this). Static mockup; statuses as icon bullets, set on the selected card (3.3). Issue #62; the generators' side: #61 (research/GENERATORS.md).

# VARIANTS-CANVAS – every generation of a shot on one board (concept, 2026-10-19)

Owner 2026-10-19 (storyboard studio): "variants can be a huge open canvas with each generation next to the others, we can
scroll and compare generation to each other. and pick the one we like. the generator takes previous as reference so we
can improve on them, right="

Mockup (static HTML, the real tokens, 1440 × 900): [variants-canvas.html](studio-layout/variants-canvas.html) – 3 bands.
Builds on research/STUDIO-LAYOUT.md (stage 2: the main's editor switch, §4 C "Compare"), research/TIMELINE.md (the view
modes), research/GENERATORS.md (#61: `transport` api / local / manual, the kinds, the Image | Video switch, the record's
`kind` + `take`), the studio README "The layout" / "Generators", DESIGN "Order is space, time is clips".
**In scope**: the board (a band per picked shot, pan / zoom, every generation side by side, compare, pick) and "improve
on this". **Not here** (GENERATORS / #61): the kind split, caps, the Image | Video switch, new providers, the budget.

## 1. The answer first – "the generator takes previous as reference … right?"

Only partly, today (checked in workflows/storyboard-studio/generators/*.mjs, 2026-10-19):

| Improve on … | Today | What it needs |
|---|---|---|
| a generated **image** | Runway: yes – Generate offers the shot's generated images as the Still (`inputs()`, index.mjs), gen4_turbo / gen4.5 take it as the first frame. comfy-local: no – comfy.mjs picks its own inputs, image 1 is always `clean/<shot>.png` | comfy.mjs `--from`; the record |
| a generated **video** | No – `inputs()` offers only images as stills and only the handheld passes as Aleph's video | generated videos as Aleph's video; first / last frame → a still |
| knowing **what made a card** | No – `generated.jsonl` keeps input basenames (no root, no role, no parent); the UI never sends a seed and Runway's task doesn't return the one it picked; failed jobs and dropped files write no line; approve.mjs doesn't say which `g<n>` (clips.mjs compares bytes) | a record per generation (§3.7) |

So "improve on them" needs two things besides the board: **frames out of videos** and **a record per generation** of its
inputs, prompt, seed and parent. Nothing records lineage today beyond the generators' `generated.jsonl`.

What "reference" means for the models built (docs.dev.runwayml.com/api.md, read 2026-10-19; prices runway.mjs,
1 credit = 0.01 USD ≈ 0,0089 € – the studio shows euro, owner 2026-10-20; all other providers: GENERATORS §5):
- **gen4_turbo / gen4.5** (image → video, 5 / 12 credits/s, 2–10 s): ONE still at `first` + the prompt, no last frame.
- **aleph2** (video → video, 28 credits/s, at least 56): the video (≤ 30 s) + 1–5 keyframes, each at `at` (0–1) or
  `seconds`, optionally a `range`; no references. runway.mjs sends one keyframe at 0 today.
- Both take a `seed` (0 – 2³² − 1, random when none is sent); the task (`GET /v1/tasks/{id}`) doesn't return it.
- **comfy-local** (Qwen-Image-Edit, free, ~10 min a shot): image 1 = the picture to edit, 2–3 = characters / style –
  "improve" = the previous generation as image 1 + an edit prompt.

## 2. What existed before the board (2026-10-19)

- **Variants panel** (dock, 18rem, opens with Generate in the Generate step): the saved frame, the layout variants
  (Load / Take this), "Generated" = a drop slot (png / jpeg / webp only) and the files one under another – picture or
  player, the file name, "Approve as chosen" (`askDialog` → approve.mjs: review + dataset copy). No metadata, no lineage.
- **Generate panel** (22rem): generator, model, inputs (Still / Keyframe: the chosen image, the clean render, the saved
  frame, the generated images; Pass: the handheld render passes), prompt, length, the estimate, Generate (a paid one asks
  with the cost), the session's Jobs (progress, cancel). **"Luma – by hand"** is one entry in its generator list
  (luma-manual.mjs, GENERATORS' `transport: 'manual'`): the prompt to copy, the result dropped onto Variants.
  Results: `_out/<ch>/generated/<shot>--g<n>.<ext>`.
- **Chosen per kind**: approve.mjs copies `chosen<ext>` – a shot has a chosen image AND a chosen video.
- **Timeline**: the picked shots' clips; the view mode Shot · Take · Rendered · Generated picks ONE clip per shot (Generated
  = the chosen video, else the newest) and what the main view shows; a click or seek on a clip makes its shot current.
- **Naming bug** (fixed in phase 1): `POST /api/generated`, comfy.mjs and gemini.mjs named by count + 1 – once a `g<n>`
  was deleted they overwrote a file; all writers now take the highest n + 1 over the files and the log (generated-log.mjs).

## 3. The concept

### 3.1 Where it lives: the main view's editor "Variants" (Q1, owner 2026-10-20: as recommended)
- The main gets stage 2's editor switch (STUDIO-LAYOUT §4 C) – a square `Segmented` in its tool row, key **V**:
  **Viewport · Variants**. Viewport = stage 2's "3D" (the 3D view, the take's camera or the clip's video, following the
  timeline); Frames joins later. The board takes stage 2's Compare editor's place; Compare becomes a split in it (3.4).
- **Generate opens Variants** (a step remembers its main view); every other step keeps the Viewport.
- **The Variants panel leaves the dock** – everything it holds moves onto the board (frame, layout variants (Q5: on the board),
  generated, drops); its grid icon moves from the top bar's panel group to the main's switch; stage 2's tab column has
  no Variants tab. Generate's dock = the Generate panel (22rem): at 1440 the board gets ~64rem.
- **Order stays space**: the shot list is the toon order; the board shows **every picked shot**, a band each in that
  order (3.2), and scrolls to the **current** one (the last clicked) when the list, ‹ ›, the timeline or a band moves it.
- **Two cuts of the same clips**: the timeline = ONE clip per picked shot along time; the board = ALL candidates of the
  picked shots in space – the same set, the same order (band n = clip n). Its sources are the view modes' clips (Shot =
  the render, Take / Rendered = the take and its passes); its generations are what Generated chooses from. **Play**
  needs a picture: it switches the main to Viewport (V goes back to the board).
- **Audition** (Q3): while the board is open, the **current shot's** Generated clip is the **current card** (tagged
  "audition"; a rejected one or a source: the chosen) – Play shows it among the picked shots' clips; the chosen on close.

### 3.2 The layout – a band per picked shot, rows of results in it (Q4, Q2)
- **Bands** (Q4, owner 2026-10-20: "One band per shot NOW"): one horizontal band per picked shot, stacked in toon order
  – the board scrolls down through them, continuity reads top to bottom; one shot picked = one band, as before.
- **A band's head**, one line: fold button (round, 1rem from the left, as the bottom region's rail), review dot, id,
  format, "n done · cost"; it sticks to the top while its band scrolls by. The **current** band (the shot the panels
  edit) has its head inverted, as the shot list's row; a click in a band makes its shot current (a pan doesn't).
- **Folding** (DOCKING's fold to a rail, lying down): a band folds to its head line, a click on the line unfolds it;
  not remembered (a newly picked shot opens unfolded, as a dock panel). A folded band loads nothing; keys skip it.
- **In a band**, laid out by the data (Q2, owner 2026-10-20: rows of results, as recommended) – nobody drags or wires
  (FLORA / Krea / Weave node graphs are the anti-pattern for one shot):
  - **Column 0 = the sources**: the render without texts (what the generators get; the saved frame is the same card),
    one card per take (its beauty pass), the layout variants (Q5: A), dropped images.
  - **A parent's results form one row**, oldest left, newest right – a new run lands at the row's end. A source's
    results sit on its own row; a generation's results open a **new row under it, one column right of it**. So
    alternatives sit side by side (the comparison you make most), improvements walk down and right.
  - A generation with several generated inputs (Aleph's video + a keyframe) hangs under its **main input** (the still
    or the video); the others are named in its tooltip – one line into every card.
  - **Lines** (2px ink): a comb above each row with a tick into every result, a drop from the parent into it. Child
    rows follow right under their parent's row, the rightmost parent's first – no line crosses.
- **One card size** per shot – its aspect (another ratio is letterboxed); **one picture height in all bands** (9rem at
  100 %) – a 1:3 beside a 9:16 compares at one size; a column stays a caption wide (a narrow picture leaves room).
- **Pan / zoom** (the Timeline's keys): wheel scrolls (down through the bands), Shift + wheel sideways, a drag on the
  background pans, Ctrl + wheel / + / − zoom about the pointer (one zoom for all bands), **0** fits the widest band's
  width; the zoom % in the tool row; below ~60 % the captions hide. No minimap (≤ ~50 cards per shot; bands fold).
- **Keys** (board focused): ← → in the row, ↑ ↓ the row above / below (nearest column; over a band's edge = the next
  band, its shot current), Ctrl + click adds to the selection (any band; the current shot stays), **Enter** chosen
  (= approve), **R** review / none, **X** reject / unreject (3.5), **I** improve, **C** compare, **Esc** steps back.

### 3.3 The cards
- Picture: the image, or a video's poster (its first frame, cut once by ffmpeg, cached by mtime); a video **scrubs under
  the pointer** (x = time); a length chip marks a video – under the pointer it shows the time there; only that card plays.
- Caption: id + model; the full record as its tooltip (generator, model, seed, cost, time, inputs, parent).
- **Status** (owner 2026-10-20: "… bullets with icons … the icon is enough"): one square bullet, icon only – top left on
  the picture, beside the letter in Compare, on the timeline's clip: **✓ chosen** (badge-published) · **? review**
  (badge-warning = the shot list's "change"; ink glyphs, 11.56 / 7.15 : 1) · **✕ rejected** muted, as its strip; none =
  nothing. No chip text, no side bar. **? not an eye**: in editors an eye = shown / hidden (Blender, layers panels).
- **Selected** (2–4, for Compare): the caption inverted with its letter A–D. The **current** card (the last clicked) gets
  its actions under it, above the row below – two square groups: the **status** ✓ · ? · ✕ (pressed = its status; again
  = none) and Improve on this · Vary · More ▾ (Add to / Remove from Compare, Still First | <time> | Last for a video or
  Use as start frame / keyframe, Show the file, Delete). **No right-click menu** (owner 2026-10-20: "the right click is
  when you select. that is better … my mistake the design was better").
- **Running**: a placeholder where the result will land – the house progress bar and the estimate. Cancel and failures
  stay in the Generate panel's Jobs list (it also holds the other shots' jobs) – the board shows results only.

### 3.4 Comparing
- Select 2–4, then **C** or "Compare · n": the main splits – the board left, **Compare** right (a `Divider`, its width
  remembered; ✕ / Esc closes). T folds the shot list, N the step's panels – the board gets the window.
- The cards side by side at the same height, each in its aspect – **from any bands** (continuity across shots: the
  previous shot's chosen beside this one's candidate; a side names its shot then), letter + status bullet over each.
  **A is the reference** (the first selected; default: the current shot's chosen): a click elsewhere replaces the last side.
- Videos on **one clock** (Frame.io): one Play, one scrubber, the same moment in each; a shorter one holds its last frame.
- Under each side: model · length · seed · cost, what it came from, the prompt.

### 3.5 Picking – one status per card: chosen · review · rejected · none
- **Chosen = approve** (Enter / ✓): today's `askDialog` (the shot's review + the dataset); the previous chosen of that kind
  goes to none, stays on the board – one per shot and kind, replaced, never cleared. Generated mode plays it. approve.mjs
  records `from: "<shot>--g<n>.<ext>"` (GENERATORS §3) → clips.mjs compares bytes only for old approvals.
- **Review** (R / ?): look at it again / a change asked – it stays open (Compare, the audition, Improve, Vary); ✓ chooses
  it, X rejects it, R again = none. Not the shot's review (approve.mjs's `review`, the shot list's dot).
- **Reject folds to make room** (owner 2026-10-20: "wenn rejected the need to fold somehow so we can make space"): the
  house's fold-to-rail (DOCKING) on the board. Nothing is deleted – files and records stay; delete = More ▾.
  - **Reject** (X / ✕): the card folds to a **strip** in its place – the picture's height, 1.5rem wide, drawn like the
    `Rail` (muted frame, ✕ and id muted, the id written upwards); the row closes up, its line stays. What hangs under it
    is rejected with it (not the chosen); with nothing open under it the branch is ONE strip ("g3 +2": +n = the cards
    folded in; folded neighbours share a strip too). A row of nothing but strips **lies down** as one line (the bottom
    region's rail) – height back, not only width.
  - The strip **stays current** (an ink frame): X again brings back what that X folded, each to its mark before (one
    stamp, 3.7); the arrow keys skip strips. **A look, not a verdict**: a click on a strip opens it with its branch
    (muted, ✕ bullets; ✕ pressed in its status group) until the current card leaves it, or Esc – owner 2026-10-20: "a
    rejected tree is folded. unfolding makes it always possible to dive in that tree and explore if nessecary"; inside it
    the arrows walk the tree; a result wanted back: X on it. **Show rejected · n** (a square toggle in the tool row;
    hidden at 0, not remembered) opens all, in every band – arrows visit them.
  - Rejected cards **stay out**: not in Compare (X on side B folds it and empties B), not the audition, no Approve /
    Improve / Vary. Not rejectable: the chosen (approve another first) and the sources.

### 3.6 "Improve on this"
- **I** / ⚡ on a card makes its shot current and fills the Generate panel, which then starts with "Improve on g2" and the
  card's thumbnail (✕ = back to the shot's own inputs), the card's prompt (editable; the edit is kept with the result)
  and the seed **New** (or the card's). Nothing is sent: Generate asks with the cost as today; once sent, a placeholder
  lands in its child row – in the card's own band.
- The card goes in as the input its kind fits – in today's panel fields; with #61 the Image | Video switch decides what
  comes out and which of these the panel offers (GENERATORS §4, not redesigned here):
  - **image** → the still (gen4_turbo / gen4.5: the first frame) or comfy-local's image 1 (an edit, `--from`);
  - **video** → Aleph 2's video (+ the chosen image as the keyframe at 0, if there is one);
  - **source** → the shot's defaults (today's pre-fill).
- **Vary**: the card's own inputs, prompt and model with a new seed – a sibling in the card's row.
- **Use as start frame / keyframe**: adds the card to the form instead of replacing it (Aleph: up to 5 keyframes – the
  adapter's part is GENERATORS §6 step 5).
- **Frame → still**: the first / last / scrubbed frame of a video becomes an image input. A last frame → image to video
  makes a NEW clip from where the old one ends, not an extension.

### 3.7 Data + server – what each generation records (the bands change only the GET)
- **Lineage record**: `generated.jsonl` stays the log; each line becomes v2 –
  `{ v: 2, id: "sb-25-2--g6", file, kind: "image" | "video", take: "<take id>" | null, generator, model, mode, job,
  status, parents: ["sb-25-2--g3"], inputs: [{ role: "still" | "control" | "keyframe" | "reference" | "character" |
  "style", key: "out:s1c3/generated/…", frame?: "first" | "last" | <ms>, at? }], prompt, seed, ratio, duration,
  estimate, cost, date, mark: "review" | "rejected" | null, marked: "<date-time>" | null }`. `kind` + `take` as
  GENERATORS §3 (take = the control video's or the handheld frame's take, null for a pure image → video). `parents` =
  every input under `generated/` (a frame counts as its video, the approved copy as its `from`), the main input first.
  `mark` (not `status` – the job's); `marked` = one X's stamp on its card + branch, the fold derived (3.5). **Chosen is
  no mark**: approve.mjs's `from` (the chosen per kind) stays its one truth – status = chosen if `from`, else `mark`,
  else none; approving appends `mark: null`.
- **Writers**: jobs.mjs keeps full keys + roles (not basenames); the server **picks a seed** (0 … 2³² − 1) when the model
  takes one and none is given; failed / cancelled jobs write their line too (status, cost); `POST /api/generated` writes
  `{ generator: "upload" }` and takes mp4 too; comfy.mjs takes `--from <generated file>` (image 1) and writes `parents`;
  both count + 1 writers take the highest n + 1; approve.mjs writes `from`; `POST /api/generated/mark { ch, ids,
  mark, marked }` appends `{ v: 2, id, mark, marked }` lines, the last per id wins (append-only – no rewrite racing comfy.mjs).
- **Manual** ("Luma – by hand", `transport: 'manual'`): its result dropped on the manual form's slot in Generate is
  recorded as that generator's candidate – with the form's prompt, inputs and parent; a drop on the board is a source.
- `GET /api/generated?ch&ids=…` returns the picked shots' records in one request – `[{ file, id, kind, take, parents,
  generator, model, prompt, seed, cost, date, duration, approved, mark, marked }]` (approved from `from`): the board.
- **Video → still**: `inputs()` offers the shot's generated videos as Aleph's video and `frame:<key>#first|last|<ms>`
  stills; `fileOf` cuts it with ffmpeg (api.mjs has it) to `_out/<ch>/generated/frames/`, cached by mtime; posters too.
- **The existing log**: one conversion script (basenames resolved inside the chapter, roles from the generator's mode,
  parents where an input is a generated file) with a report of what it couldn't resolve; then only v2 is read – no silent
  load-time conversion (RULES "Stored format changes" in spirit; a local log, not DB data). What stays unresolved hangs in
  a row "earlier – no record" under the sources, never under a guessed parent.

## 4. Phases – smallest useful first

1. **Record the lineage now** (~1 day, = GENERATORS §6 step 2): invisible, but every generation from then on carries its
   history – old runs can't get it back. v2 lines (+ `kind`, `take`, `mark`), seeds, failed jobs, uploads, `from` on
   approve, the naming fix, the conversion script, GET returns records; node tests (jobs, api, clips).
2. **The board** (~4.5–5 days, the bands ~1.5 of it – Q4) – "a huge open canvas … pick the one we like": Viewport ·
   Variants in the main, Generate opens it, the Variants panel retires; **a band per picked shot** (head, fold, the
   current band + the scroll to it, keys across bands, posters for the bands in view); sources, rows of results + lines
   (a pure layout module with node tests – strips, branch strip, lying row, the bands' stacking), posters + hover scrub,
   pan / zoom / fit, select + keys, the status group + bullets, the fold + Show rejected, running placeholders, drops,
   layout variants as source cards (Q5: A). Board, band, card, strip, lines ≤ 300 lines each, maths in `lib/`.
3. **Improve on this** (~1.5 days): the pre-fill, Vary, use as start frame / keyframe, generated videos as Aleph's
   video, frame → still, comfy `--from`, the manual result slot.
4. **Compare** (~1.25–1.75 days): the split, A pinned, one clock, sides from any band; the audition on the timeline (Q3).
5. **Later, when wanted**: a wipe between two and the prompt's differences marked; a cut view (A's last frame beside B's
   first); "clean up rejected" (their files); a minimap past ~50 cards.

Effort 1–4: ~8.25–9.25 agent days with checks (was ~6.5–7.5 – the bands add ~1.75; `npm run check`, `check:tools-css`,
the studio's node tests, the Generate step in the browser at 1440 and 1920). Not counted (GENERATORS rollout, #61): caps
and the switch, Aleph keyframes ≤ 5, gen4_image, the Luma API, the budget.

## 5. Risks

- **Room at 1440**: board + Compare + Generate (22rem) is the tight case – the mockup is that case (cards at 80 %, three
  bands, the board scrolls); T / N give the board the window.
- **Many picked shots × many cards** (10 × 50 = 500): posters only, for the bands in view (lazy; a folded band loads
  nothing, far bands are drawn as their height), one live `<video>` (the hovered card), ≤ 4 in Compare, one GET.
- **Old generations** have no parents and partly unknown inputs – an honest "no record" row, no guessed lines.
- **Improving a video costs**: Aleph 2 ≈ 1,25 € per 5 s against ≈ 0,22 € for gen4_turbo from a frame (ECB 2026-10-19). The estimate + the
  confirm stay per job; the budget gate is GENERATORS §6 step 3 – it comes before anything sends several jobs at once.
- **Files moved by hand** leave records without files (a "missing" card) and files without records ("no record" row).
- **A wrong X folds a branch**: X again undoes exactly that (one stamp); the board keeps the strip under the pointer.
- **Aleph 2 takes ≤ 30 s**: a handheld render or a gen-4 result fits; a longer take is trimmed first.
- **Stage 2 overlap**: the main's switch is stage 2 – the board can come first with Viewport · Variants; Frames and the
  inspector's tab column follow.

## 6. Questions for the owner

1. ~~Where~~ – **answered** (owner 2026-10-20, as recommended): the main view's editor, Viewport · Variants (3.1).
2. ~~Layout~~ – **answered** (owner 2026-10-20, as recommended): rows of results, laid out automatically, no dragging.
3. ~~Audition~~ – **answered** (owner 2026-10-20, as recommended): the shot's Generated clip plays the current card, tagged (3.1).
4. ~~Several picked shots~~ – **answered** (owner 2026-10-20: "One band per shot NOW" – over the recommendation (the
   current shot first, bands later)): a band per picked shot in toon order (3.2), built in the board's phase (§4).
5. ~~Layout variants~~ – **answered** (owner 2026-10-20, after the comparison
   [variants-canvas-q5.html](studio-layout/variants-canvas-q5.html)): **A – source cards on the board**, as recommended.
6. ~~Rejecting a card with results~~ – **answered** (owner 2026-10-20): its tree folds with it, unfolding explores (3.5).
