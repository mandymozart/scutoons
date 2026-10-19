Status 2026-10-19: the shared Timeline is built (`@scutoons/ui` timeline.ts + timeline.svelte); its first user is the storyboard studio (stage 1). The creator, scores and live below are a plan – nothing built there.

# TIMELINE – one time axis for the studio, the creator and the live mode (2026-10-19)

Owner 2026-10-19: "Actually the timeline underneath the other windows was also good" (the studio's layout research),
then "please consider the whole time. at some point this has to be compatible with creator and anything we plan for
live performance", "best place it is part of creator" → the timeline lives in the shared UI package, designed
creator-first; the studio uses it now, the creator adopts the same component later. Choices (owner, same day): a
**linear** time axis with zoom (no per-item minimum widths that bend the ruler), a shot's width = its **reading time**,
the handheld take sits **on the chapter's axis** at its shot's start (one playhead).

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
- **Studio** (workflows/storyboard-studio `src/lib/timeline-lanes.js`, `components/Timeline.svelte`): the Shots lane
  (reading seconds → ms, scenes as markers, the frame / review dot / handheld mark drawn by the studio), in Handheld the
  Take lane (the active take at its shot's start, its trim), Play for the take in the studio's header.

## 2. Later – in this order when each is wanted (nothing decided beyond the model)

1. **Studio stage 2** (research/STUDIO-LAYOUT.md): a Generated lane (the clips per shot, for Compare), the Frames view
   sharing the selection, an audio-sample lane, snapping to shot edges.
2. **Creator** – a chapter's time on the same component:
   - the bottom region held across navigation like the dock (`useBottom` in dock.svelte.ts, DOCKING "Panels across
     pages") – the creator's AppShell sits in its root layout (page mode today; the bottom region needs fill mode);
   - `@scutoons/content` gets the reading rhythm (moved from scripts/storyboard `RHYTHM` / `measureShot` – a move, not a
     copy) and the held music of a chapter;
   - an adapter (creator `lib/editor/chapter-timeline.ts`) builds lanes along the default path (the first option at
     each choice): Reading (slices / groups by reading time), Music, Samples, Events (instants);
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
