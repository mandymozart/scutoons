Status 2026-10-19: the shared Timeline is built (`@scutoons/ui` timeline.ts + timeline.svelte); its first user is the storyboard studio – since the owner's second look a clip sequence of the picked shots (§1). The creator, scores and live below are a plan – nothing built there. The trim (§4, 2026-10-20): three ways to try in a prototype + a recommendation for the owner – not built beyond §1.

# TIMELINE – one time axis for the studio, the creator and the live mode (2026-10-19)

Owner 2026-10-19: "Actually the timeline underneath the other windows was also good" (the studio's layout research),
then "please consider the whole time. at some point this has to be compatible with creator and anything we plan for
live performance", "best place it is part of creator" → the timeline lives in the shared UI package, designed
creator-first; the studio uses it now, the creator adopts the same component later. A **linear** time axis with zoom
(no per-item minimum widths that bend the ruler). First the studio laid the whole chapter on it by reading time; the
owner then (same day): "the timeline makes a little no sense. since every scene is first of all spatial in the webtoon.
time is relative and only will play a role in the live performance mode or if videos get rendered as shot in the
webtoon … depending on how long a camera is recorded and or trimmed to, thats the length of that clip for that shot",
"the left side is our timeline vertically, the toon order so to speak. the other timeline underneath the clip only shows
those shots that are selected" → **the webtoon's order is spatial (the shot list); time exists only for clips** – a
recorded / trimmed take, a rendered or a generated video, and later live.

How it was decided: a research + design workflow (four readers: live, creator, studio, shared architecture; three
designs: a seam for later, shared-first, live / creator-risk-first; a judge) – this file keeps its result.

## 1. What exists (2026-10-19)

- **Model** `shared/ui/src/timeline.ts` (pure TS, no runes / DOM; also the subpath `@scutoons/ui/timeline` for node and a
  later `@scutoons/live`): the axis unit is **ms**; lanes of items (`start`, `length` – none = an instant, a cue –,
  `state`, `seq` for ties, `trim { in, out, min }` = the kept part of a clip / take); markers (`at`, `label`); `Place
  { id, at }` = a fraction into an item (live's `{ row, slice }`, a score's fraction of a bar); maths: `sequence`,
  `itemAt`, `placeOf` / `msOf`, `frac` / `timeAt`, `ticks` (1-2-5), `zoomAround`, `clampTrim`, `formatTime`.
- **View** `shared/ui/src/timeline.svelte` + `timeline-lane.svelte`, placed in the AppShell's bottom region
  (agents/DOCKING.md "Bottom region"): ruler with ticks + markers, the playhead, lanes (a tall one with pictures),
  zoom (Ctrl + wheel, + / − / 0), reveal a range, follow the playhead; callbacks `onseek(t, final)`,
  `onselect(lane, id)`, `ontrim(lane, id, { in, out }, final)`. **Read-only without callbacks** (a conductor's view).
  **No transport inside**: no Play, no clock, no Space, never PageUp / PageDown (the score's pedal keys) – the app owns
  playback and passes `playhead`.
- **Studio** (workflows/storyboard-studio `src/lib/timeline-lanes.js`, `timeline.svelte.js`, `components/Timeline.svelte`,
  `ClipView.svelte`; the clips from `GET /api/clips`, clips.mjs): ONE clips lane – the PICKED shots (multi-select in the
  shot list) back to back, each as long as its clip in the **view mode** (owner: "like in blender, rendered view port, vs.
  shaded, vs. wireframe vs box. in our case, it is shot/take/rendered/generated"): Shot · Take · Rendered · Generated,
  falling back generated → rendered → take → a still of 5 s ("those without a recorded or rendered video will need a
  default length"); the take keeps its trim; the main view follows the clip under the playhead (3D, the take's camera,
  or the video); Play runs the sequence (the studio's transport); a hover shows the clip's first frame next to the cursor
  (the Timeline's `preview` snippet).

## 2. Later – in this order when each is wanted (nothing decided beyond the model)

1. **Studio stage 2** (research/STUDIO-LAYOUT.md): Compare (the generated candidates of a shot side by side), the Frames
   view sharing the picked set, an audio-sample lane, snapping to clip edges, an export of the picked sequence.
2. **Creator** – a chapter's time on the same component:
   - the bottom region held across navigation like the dock (`useBottom` in dock.svelte.ts, DOCKING "Panels across
     pages") – the creator's AppShell sits in its root layout (page mode today; the bottom region needs fill mode);
   - only where a chapter really has time (owner: time "only will play a role in the live performance mode or if videos
     get rendered as shot in the webtoon"): its videos (player / video slices), its music and samples, its timed events –
     an adapter (creator `lib/editor/chapter-timeline.ts`) builds those lanes along the default path (the first option
     at each choice); no reading-time lane (the reading order is spatial – the chapter's slices);
   - the playhead follows the Reader panel read-only; a seek sends `scutoons:reader-show` (DOCKING "The Reader panel");
   - one `formatTime` for every duration shown (the player, the playlist).
3. **Scores**: lanes from the Verovio timemap (bars as markers, `ScoreAt` as places), the synth's time as the playhead.
4. **Live** (research/LIVE.md, after the February 2027 restart): adapters in `@scutoons/live` – the Show's rows as
   markers on the played path, one lane per outlet, cues as instants at anchor + `wait`, layers as regions from `on`
   to `off`; the manual cues stay in the cue list; rehearsal replays a recorded run read-only with `follow`; dense
   markers bunch, long shows virtualise.
   Musical time (bars, beats, tempo) comes as tick / format functions next to the score's `barTime`, never as a second
   axis.

## 3. Open (decide when the user is built)

- Waveforms (peaks from the creator's `decodeAudio`) – per lane item, drawn by the app's snippet or a shared one.
- Editing items by drag (move / resize) – not needed by the studio; the creator's Events lane might want it.

## 4. Trim (2026-10-20) – way C is built

Owner 2026-10-20: "show me in dept how the trim will work? like on a mobile app, or in ableton?" → a prototype to try:
`studio-layout/trim-prototype.html` (opens via file://, keys 1–3 switch the ways; one drawn 6 s take at 24 fps, the
frame number on each picture; the right column says how each way behaves, its pros and cons).

**Built (§1).** Take mode shows each picked shot's whole take; its kept part `trim { in, out, min }` (ms, `min` =
TRIM_MIN 100 ms) – the cut parts hatched in place (timeline-lane.svelte `.cut`). With `ontrim` the in / out lines are
sliders: a 3px ink bar in a 0.75rem grip zone the lane's height, dragged by how far it moved (no jump), snapped to the
lane's `step` (the studio: a frame at 24 fps); ← → a step, Shift ×10, Alt ×0.1, Home / End; `clampTrim` keeps it in
0..length and ≥ `min`, the dragged edge stops at the other. The studio (timeline.svelte.js `trim`) puts the playhead on
the moved point – the main view shows the take's camera there – and writes the take's in / out (s) into the takes file,
saved on release; Play plays the kept parts (`kept()`); Ctrl + wheel / + − 0 zoom.

**Three ways** (the prototype):
- **A · Mobile** (iPhone Photos, CapCut, Instagram): the whole take as one strip, a frame with two thick grips over it,
  outside dimmed; the big picture shows the dragged grip's frame; holding a grip still zooms the strip to single frames
  (let go: back); pinch / Ctrl + wheel; a tap plays the kept part. + one thumb, nothing to learn. − a mode of its own
  (one take, no neighbours, Cancel / Done), a frame is a sliver at full width, a mouse and keys get no more.
- **B · Ableton** (arrangement + clip view): a clip is a window onto its take – drag an edge (a bracket cursor) and the
  picture stays put in time; drag = move, Alt-drag = slip (the window stays, the take moves under it); edges snap to a
  grid that follows the zoom, Alt = single frames; the clip view shows the whole take with start / end markers, outside
  greyed. + precise, slip. − the cut parts vanish from the arrangement (against "Order is space"), a trim leaves a gap,
  two places for one cut, modifiers a phone lacks, a lot of UI.
- **C · Studio** – the recommendation: the built timeline, grown by the best of both.

**Recommendation – C.** Keep what is built (whole take, cut parts hatched in place, frame steps, the keys) and add:
1. Ableton's edge: the bracket cursors on the in / out line (already grabbable along its height) + a tab at its top,
   inside the kept part; on a coarse pointer the zone and the tab finger-sized (2.75rem).
2. The in / out pair in the main view while trimming (Final Cut's two-up): both frames side by side, the moved one
   marked; one picture again shortly after release. The dragged side live, the other a still taken when the drag starts.
3. Slip: a bar under the kept part's middle (works by finger) or Alt-drag on the kept picture – the kept part moves
   through the take, its length stays; ← → slip a frame on the focused bar.
4. I / O: the in / out of the clip under the playhead, at the playhead (I: the frame under it is the first one kept, O:
   the last one kept); no Space key – Play stays the studio's button (§1).
5. Hold a grip still (450 ms): the timeline zooms to a frame per picture around it; let go: the zoom before (A's best idea).
6. Readouts while trimming: in / out next to their lines (outside the kept part; at the lane's end inside, a row
   lower), the kept length in its middle.
On phones the timeline is hidden (DOCKING "Bottom region"): the phone page would get A's strip on the same model and
keys – one trim, two views. The frame-after-frame pictures in the clips are the owner's other 2026-10-20 point.

**What it takes** (about 2 days with tests; no data change – the takes file keeps in / out):
- `shared/ui/src/timeline.ts`: `clampTrim` for a slip (both edges, the length kept inside 0..length) + node tests.
- `shared/ui/src/timeline-lane.svelte` (212 lines – the trim part may become its own `timeline-trim.svelte`): the tab,
  the cursors, coarse-pointer sizes, the slip bar (a third slider; `ontrim` with both values), Alt-drag on the kept part,
  the readout chips; a `timelineSlip` label in ui-labels.
- `shared/ui/src/timeline.svelte`: hold-to-zoom around a time and back (`zoomAround` exists), panning while a held grip
  sits at the strip's end.
- `workflows/storyboard-studio/src/lib/timeline.svelte.js` + the main view: slip in `trim()`, I / O (free in App.svelte's
  keys – check the 3D view's), a trimming state with both edges for the pair, the other edge's still (one render to a
  canvas at drag start; a video clip: a seek).
- Docs: DESIGN "Steps and the timeline", §1 here; the studio's timeline check.

**Owner 2026-10-20: "Build trim way C (Studio) as recommended"** – built (issue #62): the shared Timeline's trim
(`timeline-trim.svelte`: the bracket cursors + the tab, finger-sized on a coarse pointer, the slip bar + Alt-drag, hold
a grip 450 ms = zoom to frames and back, the readouts while trimming; `ontrim(lane, id, { in, out }, final, edge)` with
edge in / out / slip – a gesture = final false … one final true, Escape / pointercancel = the start values;
`clampTrim(…, 'slip')`, node tests) and the studio's (`trim()` with the edge, the in / out pair in the main view –
`TrimPair.svelte`, the other edge a still rendered at the gesture's start – I / O at the playhead in Take mode).
Not built: the phone page's A strip; ZOOM_MAX 64 caps hold-to-zoom on long sequences (> ~32 s picked).
