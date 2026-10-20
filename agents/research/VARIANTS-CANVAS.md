Status 2026-10-19: concept – owner to choose (§6 questions; static mockup, nothing built). Issue #62; the generators' side: #61 (research/GENERATORS.md).

# VARIANTS-CANVAS – every generation of a shot on one board (concept, 2026-10-19)

Owner 2026-10-19 (storyboard studio): "variants can be a huge open canvas with each generation next to the others, we can
scroll and compare generation to each other. and pick the one we like. the generator takes previous as reference so we
can improve on them, right="

Mockup (static HTML on the real tokens, 1440 × 900): [variants-canvas.html](studio-layout/variants-canvas.html).
Builds on research/STUDIO-LAYOUT.md (stage 2: the main's editor switch, §4 C "Compare"), research/TIMELINE.md (the view
modes), research/GENERATORS.md (#61: `transport` api / local / manual, the kinds, the Image | Video switch, the record's
`kind` + `take`), the studio README "The layout" / "Generators", DESIGN "Order is space, time is clips".
**In scope**: the board (pan / zoom, every generation of the current shot side by side, compare, pick) and "improve on
this". **Not here** (GENERATORS / #61): the kind split, caps, the Image | Video switch, new providers, the budget.

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
- **gen4_turbo / gen4.5** (image → video, 5 / 12 credits/s, 2–10 s): ONE still at position `first` + the prompt – no
  last frame on these two.
- **aleph2** (video → video, 28 credits/s, at least 56): the video (≤ 30 s) + 1–5 keyframes, each at `at` (0–1) or
  `seconds`, optionally a `range`; no references. runway.mjs sends one keyframe at 0 today.
- Both take a `seed` (0 – 2³² − 1, random when none is sent); the task (`GET /v1/tasks/{id}`) doesn't return it.
- **comfy-local** (Qwen-Image-Edit, free, ~10 min a shot): image 1 = the picture to edit, 2–3 = characters / style –
  "improve" = the previous generation as image 1 + an edit prompt.

## 2. What exists

- **Variants panel** (dock, 18rem, opens with Generate in the Generate step): the saved frame, the layout variants
  (Load / Take this), "Generated" = a drop slot (png / jpeg / webp only) and the files one under another – picture or
  player, the file name, "Approve as chosen" (`askDialog` → approve.mjs: review + dataset copy). No metadata, no lineage.
- **Generate panel** (22rem): generator, model, inputs (Still / Keyframe: the chosen image, the clean render, the saved
  frame, the generated images; Pass: the handheld render passes), prompt, length, the estimate, Generate (a paid one asks
  with the cost), the session's Jobs (progress, cancel). **"Luma – by hand"** is one entry in its generator list
  (luma-manual.mjs, GENERATORS' `transport: 'manual'`; uncommitted in this worktree): the prompt to copy, the result
  dropped onto Variants. Results: `_out/<ch>/generated/<shot>--g<n>.<ext>`.
- **Chosen per kind**: approve.mjs copies `chosen<ext>` – a shot has a chosen image AND a chosen video.
- **Timeline**: the picked shots' clips; the view mode Shot · Take · Rendered · Generated picks ONE clip per shot
  (Generated = the chosen video, else the newest) and what the main view shows; a click or seek on a clip makes its shot
  the current one.
- **Naming bug**: `POST /api/generated` (api.mjs) and comfy.mjs name by count + 1 – once a `g<n>` was deleted they
  overwrite a file (jobs.mjs `nextName` takes the highest n + 1 – the right way).

## 3. The concept

### 3.1 Where it lives: the main view's editor "Variants"
- The main gets stage 2's editor switch (STUDIO-LAYOUT §4 C: 3D · Frames · Compare) – a square `Segmented` in its tool
  row, key **V**: **Viewport · Variants**. Viewport = stage 2's "3D" (the 3D view, the take's camera or the clip's video,
  following the timeline); Frames joins later. The board takes the place of stage 2's Compare editor; Compare becomes a
  split inside it (3.4).
- **Generate opens Variants** (a step remembers its main view); every other step keeps the Viewport.
- **The Variants panel leaves the dock** – everything it holds moves onto the board (frame, layout variants, generated,
  drops); its grid icon moves from the top bar's panel group to the main's switch; stage 2's tab column has no Variants
  tab. Generate's dock = the Generate panel (22rem): at 1440 the board gets ~64rem.
- **Order stays space**: the shot list is the toon order; the board shows the **current** shot (the last clicked of the
  picked) and follows it (the list, ‹ › in the ActionBar, a click or seek on the timeline).
- **Two cuts of the same clips**: the timeline = ONE clip per picked shot along time; the board = ALL candidates of the
  current shot in space. Its sources are the view modes' clips (Shot = the render, Take / Rendered = the take and its
  passes); its generations are what Generated chooses from. **Play** needs a picture: it switches the main to Viewport
  (V goes back to the board).
- **Audition** (Q3): while the board is open, the current shot's clip in Generated mode is the **current card** (tagged
  "audition"), so Play shows it between its neighbours; the chosen again when the board closes. Approving stays a
  deliberate act.

### 3.2 The layout on the board – rows of results (Q2)
Laid out by the data – nobody drags or wires (FLORA / Krea / Weave node graphs are the anti-pattern for one shot):
- **Column 0 = the sources**: the render without texts (what the generators get; the saved frame is the same card), one
  card per take (its beauty pass), the layout variants (Q5), dropped images.
- **A parent's results form one row**, oldest left, newest right – a new run lands at the row's end. A source's results
  sit on its own row; a generation's results open a **new row under it, one column right of it**. So alternatives sit
  side by side (the comparison you make most), improvements walk down and right.
- A generation with several generated inputs (Aleph's video + a keyframe) hangs under its **main input** (the still or
  the video); the others are named in its tooltip – one line into every card.
- **Lines** (2px ink): a comb above each row with a tick into every result, a drop from the parent into it. Child rows
  follow right under their parent's row, the rightmost parent's first – no line crosses.
- **One card size** per shot – its aspect (another ratio is letterboxed). At 100 % a picture is 9rem tall.
- **Pan / zoom** (the Timeline's keys): wheel scrolls, Shift + wheel sideways, a drag on the background pans, Ctrl + wheel /
  + / − zoom about the pointer, **0** fits all; the zoom % in the tool row; below ~60 % the captions hide. No minimap
  (≤ ~50 cards per shot).
- **Keys** (board focused): ← → in the row, ↑ ↓ the row above / below (nearest column), Ctrl + click adds to the
  selection (as in the shot list), **Enter** approve, **I** improve on this, **C** compare, **Esc** closes Compare, then
  clears the selection.

### 3.3 The cards
- Picture: the image, or a video's poster (its first frame, cut once by ffmpeg, cached by mtime); a video **scrubs under
  the pointer** (x = time), its length as a chip (the chip marks a video); only the hovered card plays.
- Caption: id + model; the full record as its tooltip (generator, model, seed, cost, time, inputs, parent).
- **Chosen**: a ✓ chip + a 0.5rem cyan left rule (the shot list's approved colour) – one image, one video.
- **Selected** (2–4, for Compare): the caption inverted with its letter A–D. The **current** card (the last clicked)
  gets one square icon group under it, drawn above the row below: Approve · Improve on this · Vary · More ▾ (first /
  last / this frame → still, use as start frame, use as keyframe, show the file, delete with an `askDialog`).
  Right-click = the same as a menu.
- **Running**: a placeholder where the result will land – the house progress bar and the estimate. Cancel and failures
  stay in the Generate panel's Jobs list (it also holds the other shots' jobs) – the board shows results only.

### 3.4 Comparing
- Select 2–4, then **C** or "Compare · n": the main splits – the board left, **Compare** right (a `Divider`, its width
  remembered; ✕ / Esc closes). T folds the shot list, N the step's panels – the board gets the window.
- The cards side by side at the same height. **A is the reference** (the first selected, by default the chosen): a click
  on another card replaces the last side – walk the board against a fixed reference.
- Videos on **one clock** (Frame.io): one Play, one scrubber, the same moment in each; a shorter one holds its last frame.
- Under each side: model · length · seed · cost, what it came from, the prompt.

### 3.5 Picking
- **Approve** (Enter / ✓): today's `askDialog` (review + dataset); the card gets ✓, the previous chosen of that kind
  loses it and stays on the board. Generated mode plays the chosen video.
- approve.mjs records `from: "<shot>--g<n>.<ext>"` (GENERATORS §3: the chosen per kind) → clips.mjs stops comparing
  bytes (kept for old approvals).
- Nothing is hidden: the board keeps every generation; delete only under More ▾ (phase 5 may fold away rejected ones).

### 3.6 "Improve on this"
- **I** / ⚡ on a card fills the Generate panel, which then starts with "Improve on g2" and the card's thumbnail (✕ = back
  to the shot's own inputs), the card's prompt (editable; the edit is kept with the result) and the seed **New** (or the
  card's). Nothing is sent: Generate asks with the cost as today; once sent, the running placeholder appears in the
  card's child row.
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

### 3.7 Data + server – what each generation records
- **Lineage record**: `generated.jsonl` stays the log; each line becomes v2 –
  `{ v: 2, id: "sb-25-2--g6", file, kind: "image" | "video", take: "<take id>" | null, generator, model, mode, job,
  status, parents: ["sb-25-2--g3"], inputs: [{ role: "still" | "control" | "keyframe" | "reference" | "character" |
  "style", key: "out:s1c3/generated/…", frame?: "first" | "last" | <ms>, at? }], prompt, seed, ratio, duration,
  estimate, cost, date }`. `kind` + `take` as GENERATORS §3 (take = the control video's or the handheld frame's take,
  null for a pure image → video). `parents` = every input under `generated/` (a frame counts as its video), the main
  input first.
- **Writers**: jobs.mjs keeps full keys + roles (not basenames); the server **picks a seed** (0 … 2³² − 1) when the model
  takes one and none is given; failed / cancelled jobs write their line too (status, cost); `POST /api/generated` writes
  `{ generator: "upload" }` and takes mp4 too; comfy.mjs takes `--from <generated file>` (image 1) and writes `parents`;
  both count + 1 writers take the highest n + 1; approve.mjs writes `from`.
- **Manual** ("Luma – by hand", `transport: 'manual'`): its result dropped on the manual form's slot in Generate is
  recorded as that generator's candidate – with the form's prompt, inputs and parent; a drop on the board is a source.
- `GET /api/generated?ch&id` returns records – `[{ file, id, kind, take, parents, generator, model, prompt, seed, cost,
  date, duration, approved }]`: enough to lay out the board and draw its lines.
- **Video → still**: `inputs()` offers the shot's generated videos as Aleph's video and `frame:<key>#first|last|<ms>`
  stills; `fileOf` cuts the frame with ffmpeg (api.mjs has it) into `_out/<ch>/generated/frames/`, cached by mtime. The
  board's posters the same way.
- **The existing log**: one conversion script (basenames resolved inside the chapter, roles from the generator's mode,
  parents where an input is a generated file) with a report of what it couldn't resolve; then only v2 is read – no silent
  load-time conversion (RULES "Stored format changes" in spirit; a local log, not DB data). What stays unresolved hangs in
  a row "earlier – no record" under the sources, never under a guessed parent.

## 4. Phases – smallest useful first

1. **Record the lineage now** (~1 day, = GENERATORS §6 step 2): invisible, but every generation from then on carries its
   history – old runs can't get it back. v2 lines (+ `kind`, `take`), seeds, failed jobs, uploads, `from` on approve,
   the naming fix, the conversion script, GET returns records; node tests (jobs, api, clips).
2. **The board** (~2.5–3 days) – "a huge open canvas … pick the one we like": Viewport · Variants in the main, Generate
   opens it, the Variants panel retires; sources, rows of results + lines (the layout a pure module with node tests),
   posters + hover scrub, pan / zoom / fit, select + keys, approve, running placeholders, drops, layout variants as
   source cards. Board, card and lines as separate components (≤ 300 lines each), the maths in `lib/`.
3. **Improve on this** (~1.5 days): the pre-fill, Vary, use as start frame / keyframe, generated videos as Aleph's
   video, frame → still, comfy `--from`, the manual result slot.
4. **Compare** (~1–1.5 days): the split, A pinned, one clock; the audition on the timeline (Q3).
5. **Later, when wanted**: a wipe between two and the prompt's differences marked; folding away rejected cards; one band
   per picked shot (Q4); a minimap past ~50 cards.

Effort 1–4: ~6–7 agent days with checks (`npm run check`, `check:tools-css`, the studio's node tests, the Generate step in
the browser at 1440 and 1920). Not counted (GENERATORS rollout, #61): caps and the switch, Aleph keyframes ≤ 5,
gen4_image, the Luma API, the budget.

## 5. Risks

- **Room at 1440**: board + Compare + Generate (22rem) is the tight case – the mockup is that case (cards at 90 %); T / N
  give the board the window.
- **Many videos**: posters only, one live `<video>` (the hovered card) and ≤ 4 in Compare – never 50 playing at once.
- **Old generations** have no parents and partly unknown inputs – an honest "no record" row, no guessed lines.
- **Improving a video costs**: Aleph 2 ≈ 1,25 € per 5 s against ≈ 0,22 € for gen4_turbo from a frame (ECB 2026-10-19). The estimate + the
  confirm stay per job; the budget gate is GENERATORS §6 step 3 – it comes before anything sends several jobs at once.
- **Files moved by hand** leave records without files (a "missing" card) and files without records ("no record" row).
- **Aleph 2 takes ≤ 30 s**: a handheld render or a gen-4 result fits; a longer take is trimmed first.
- **Stage 2 overlap**: the main's switch is stage 2 – the board can come first with Viewport · Variants; Frames and the
  inspector's tab column follow.

## 6. Questions for the owner

1. **Where** – the board as the main view's editor (Viewport · Variants, in the place of stage 2's Compare; Generate
   opens it; the Variants panel retires), or a dock panel you maximize? → Recommended: the main – it needs the room, and
   Generate must stay beside it.
2. **Layout** – rows of results (a parent's results side by side; improving on one opens a row under it, one column
   right; laid out automatically), or each run a row in time order, or a free canvas you arrange by hand? →
   Recommended: rows of results, no dragging.
3. **Audition** – while the board is open, does the timeline's Generated clip of the current shot play the current card
   (tagged), so you see it between its neighbours before approving – or always the chosen? → Recommended: audition.
4. **Several picked shots** – only the current shot's board, or one band per picked shot in toon order (continuity across
   shots)? → Recommended: the current shot now; bands in phase 5.
5. **Layout variants** ("Keep as variant") as source cards on the board – one place for everything a shot tried – or into
   the Shot panel (Layout step)? → Recommended: source cards on the board.
