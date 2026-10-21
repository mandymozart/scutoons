# Storyboard Studio

A local tool for the storyboard workflow (owner 2026-10-09: "just start the studio over night. i want to see where
this goes. we might dump it later anyhow if there is easier way"; 2026-10-10: "use our scutoons ui and design
dogmas", "a free moving camera in a 3d view"). Built like the tools: `@scutoons/ui` (AppShell, dock panels, rails,
Segmented, dialogs) and `@scutoons/tokens` (tools.css). Everything goes back into the repo's storyboard files.

```bash
npm run dev:studio
```

→ http://localhost:14000 (`shared/dev-ports.json` "studio"; `npm run dev -- studio` with the stack). A Vite + Svelte app
(no SvelteKit); its API (`api.mjs`) runs inside the dev server. Needs the local dev DB for the script text
(`npm run dev:db`) and the media dev copy (`media/`) for the character images.

## The layout

Owner 2026-10-19 (research/STUDIO-LAYOUT.md, concept C stage 1): **steps** over one layout that always looks the same –
"it makes sense for it to always look the same". The top bar's step tabs (keys **1–6**): **Script · Shots · Layout ·
Handheld · Generate · Export**. A step only changes which dock panels are open, the Shot panel's tab, the main view's
editor (Viewport · Variants), the timeline's view mode, its fold and the panels' widths – each step remembers its own (localStorage
`storyboard-studio:step:v2:<id>`, `storyboard-studio:<step>:<kind>`); the current and the picked shots are shared by
every step.

| Step | Opens the first time | Timeline (view mode) |
| --- | --- | --- |
| Script | Shot (tab Shot) + Script | folded (Shot) |
| Shots | Shot (tab Shot) | folded (Shot) |
| Layout | Shot (tab Camera) | folded (Shot) |
| Handheld | Handheld | open (Take – the take's trim) |
| Generate | Generate; the main view: the Variants board; the ActionBar's Review (Approve / Ask for a change) | open (Generated) |
| Export | Output | folded (Rendered) |

- **Sidebar – the shots** in the toon's order, by scene (owner 2026-10-19: "the left side is our timeline vertically, the
  toon order so to speak"): frame, id, format, view, review dot (in every step), variants, handheld. A click = only that
  shot; **Ctrl / ⌘ + click** adds it to the picked shots (or takes it off), **Shift + click** picks the range from the
  last clicked one. The last clicked is the **current** shot (inverted – the one the panels edit; unsaved changes ask
  first), the others picked show an ink frame with a bar; the timeline shows the picked ones. Resizable, folds to a rail
  (**T** or Ctrl + \).
- **Page – the main view** has two editors – a square switch at the start of its tool row, **V** outside text fields:
  **Viewport** (below) and **Variants** (the board, "The variants board" below); the 3D view stays mounted under the
  board. Play switches to the Viewport. The **Viewport** follows the clip under the timeline's playhead: the **3D view** for a still or the current
  shot's take (its camera at that moment), a **video** (a rendered / generated clip, at the playhead's time) or another
  shot's frame over it – the 3D view stays mounted underneath. **The 3D view**: the shot's scene with the frames' own renderer (`tools/storyboard-render/core.js`), live.
  Drag = orbit, right-drag = pan, wheel = dolly – the camera is written into the shot as a hand camera
  (`camera: { pos, target, fov }`); "Shot camera" goes back to the camera of size + angle. A figure is handled
  directly: drag it = move on the ground (Turn tool: drag sideways = turn), wheel on it = size, Alt + wheel = turn (about
  its feet); the picked figure's gizmo works too. Its place goes back once per gesture as the storyboard's x / y / h /
  turn – the exact inverse of the renderer's placing, so a rebuild (any field changed) leaves every figure where it was
  (a figure floating under a hand camera gets `ground: false`). The texts in the SCUTOONS presets on top ("Texts").
  Ctrl + Shift + \ folds it (the panels take its room).
- **Right dock – the step's panels** (the top bar's icon group toggles them for this step; each has the creator's head:
  title, fold to its rail, maximize – Ctrl + Space –, ✕; **N** folds / unfolds the step's panels), in fixed places from
  the view outwards: **Shot** – the shot's fields as a compact inspector, one tab at a time: Shot (action, image prompt,
  view, source, note), Camera (format, aspect, projection, scroll move, size, angle, the hand camera's lens), Light
  (from, height, source, shadow, a free direction in °, softness, accent + rim, note), Figures (cast, persona, pose,
  facing, framing, mood, x / y / h); **Script** – the scene's script lines + notes; **Handheld** – below;
  **Generate** – a generator per shot (below, "Generators"), with **Build from the shot** (the full prompt) and **Luma –
  by hand** (the prompt to copy, the render without texts, the references to attach – the former Luma panel, owner
  2026-10-19: "this should be somehow now be covered by the generate right?"); **Output** – the chapter's handheld
  videos and downloads (below). (The Variants panel left the dock: the frame, the layout variants, the generated files
  and the drops are on the Variants board – "The variants board" below.)
- **Timeline** along the bottom, under the main view and the panels (owner: "the timeline underneath the other windows
  was also good"; then "every scene is first of all spatial in the webtoon … depending on how long a camera is recorded
  and or trimmed to, thats the length of that clip for that shot"): only the **picked** shots, back to back in toon
  order, each ONE clip as long as that clip. The **view mode** in its head (like Blender's viewport shading – "in our
  case, it is shot/take/rendered/generated") says which clip: **Generated** = the shot's generated video (the approved
  one, else the newest the board doesn't know as rejected; while the Variants board is open, the current shot's: the
  board's current card – "The audition" below), **Rendered** = the newest handheld render of the active take
  (beauty pass), **Take** = the active take
  (its kept part; in Take mode the whole take with its in / out handles), **Shot** = a still – each falling
  back down generated → rendered → take → a still of 5 s; a clip's tag says which it is (a generated one: its status
  bullet ✓ / ? / ✕ and its id – g3). The clips come from
  `GET /api/clips` (clips.mjs). A clip shows its **frames one after another** (owner 2026-10-20: "should be not
  cover, but show the frame after frame"): a video as a filmstrip – tiles at its aspect along the clip, each the frame
  at its time, as many as fit (zoom in → more; ffmpeg makes one sprite per clip and zoom step, `GET /api/thumbs`,
  cached in `_out/<ch>/thumbs/`); a still or a take (only the shot's frame exists) shows that frame once at the left;
  without ffmpeg a video shows its first frame at the left. Hover a clip: its first frame next to the pointer. Click a clip = its shot current
  (← → the previous / next clip); click / drag the ruler = the playhead – in Shot / Take mode the shot under it becomes
  current on release (unsaved changes ask first), in the video modes the current shot stays. **Play** runs the
  sequence: a video plays (it is the clock), a take moves the 3D view's camera (its kept part), a still waits 5 s; in
  Shot / Take mode each shot it reaches becomes current (a refused change stops it); a seek, another step, mode or picked
  set, another current shot (the list, the stepper) or folding the timeline ends it. No Space key; I / O trim at the
  playhead (Take mode – "Trim" below). "Back to the shot
  camera" gives the 3D view back to the shot until the playhead moves. Ctrl + wheel or + / − zoom, 0 fits. Drag its top edge for the height
  (one height for all steps), its fold button folds it to a rail (per step). The shared `@scutoons/ui` Timeline
  (creator-ready: the creator and the live mode will use the same component – research/TIMELINE.md).
- **Top bar**: the chapter, the steps, the panels, **Chapter ▾** (Check, All frames, Reference packs, Chapter draft,
  Add a taste rule …).
- **ActionBar**: ‹ the shot › (n / N, through the whole toon), the keys; the storyboard check's notes (a click picks the
  shot a note names); Revert · Keep as variant · Review ▾ (Generate) · **Save + frame** (• = unsaved). Messages are
  toasts (bottom left, above the ActionBar).

## The variants board

Owner 2026-10-19: "variants can be a huge open canvas with each generation next to the others, we can scroll and compare
generation to each other. and pick the one we like" (agents/research/VARIANTS-CANVAS.md, issue #62). Stage 2 is built:
the main's editor **Variants** (Generate opens it) – every generation of the **picked** shots, a **band** each in toon
order (the timeline's clips, the same set and order). A band's head: the review dot, the id, the format, "n done · cost"
(euro; sticks while its band scrolls by; the current shot's inverted). In a band, laid out by the record
(`src/lib/board-layout.js`, node tests): column 0 = the **sources** – the render without texts (else the saved frame),
the kept **layout variants** (v1, v2 …, "Keep as variant": its saved frame + the note; Q5 A), one card per take with
a handheld render, a dropped file –; a parent's results = one row, oldest left; a generation's
results a row under it, one column right; several generated inputs → under the main one (`parents[0]`); a result
without a parent under the source its inputs name, else in the row **Earlier – no record** (files without a v2 record
too). Lines: a comb above each row, a tick into every result, a drop from the parent. One picture height for all bands
(9rem), the width by the shot's aspect. A card: the image or a video's first frame (`GET /api/thumbs`, one tile), a
video's length, id + model, the record as its tooltip, the **status** as a bullet – ✓ chosen · ? review · ✕ rejected.
A video **scrubs under the pointer** (x = time – one sprite of 32 frames from `GET /api/thumbs`, asked on its first
hover; the chip says the time there; without ffmpeg the card's own video seeks). A click in a band = its shot current;
on a card = the **current card** (its actions under it: a result's status ✓ ? ✕ and **Improve · Vary · More** – below; a
layout variant's **Load** – into the Viewport, unsaved – and **Take this** – after an in-page confirm it becomes the
shot, the shot as saved a variant with its saved frame, the frame and the render without texts are saved again; the
render and the takes take no status); **Ctrl / ⌘ + click** selects (A–D – Compare's sides, below); the board scrolls to the current
band when the list, ‹ ›, or the timeline changes it. Keys: ← → in the row, ↑ ↓ the rows (across bands – the shot becomes
current), **Enter** chosen (= approve: the review + the dataset, one chosen image and one chosen video per shot), **R**
review / none, **X** reject – with what hangs under it, one stamp; again: back (`POST /api/generated/mark`; the chosen
and the sources – a dropped file too – can't be rejected, a rejected card can't be chosen or put in review), Esc the
selection off (C and Esc also work from the tool row and the Divider). **The fold** (2b): a rejected card is a
**strip** in its place (the picture's height, 1.5rem wide, drawn like the Rail – ✕ and the id upwards, muted); with
nothing open under it its branch folds into that strip ("g3 +2"),
folded neighbours in a row share one; a row of nothing but strips **lies down** as one line. The strip stays current
after its X (X again: back); the arrow keys skip strips; a click on a strip (Enter when current) opens a **look** – it
unfolds with its branch, muted, until the current card leaves it or Esc. **Show rejected · n** (the tool row, hidden at
0) unfolds them all; rejected cards stay out of the selection. A band's head has a **fold** button: folded, the band is
its head line (a click on it unfolds it), loads nothing, the keys skip it; not remembered. **Pan / zoom**: the wheel
scrolls down, Shift + wheel sideways, a drag on the background pans, Ctrl + wheel / + / − zoom about the pointer (one
zoom for all bands, 25–200 %: the picture height in rem – `src/lib/board-zoom.js`), 0 fits the widest band, the %
button → 100 %; below 60 % the captions hide. A band fills its cards only once it comes near the view (or holds the
current card). **Drops**: images (png / jpeg / webp) or videos (mp4 / webm) dragged over a band outline it (2px dashed);
dropped, each is an upload for that band's shot (`POST /api/generated`, a source card in column 0) – the tool row's
round button picks files for the current shot; what can't be added is a toast. A **running** job of a picked shot (the
session's jobs, `GET /api/generate/jobs?ch`, polled while one runs) is a placeholder where its result will land – at the
end of its main input's row: the progress over the house bar and the estimate; its card replaces it, cancel and
failures stay in Generate's Jobs list.

**Improve on this · Vary · More** (stage 3, VARIANTS-CANVAS §3.6; `src/lib/improve.js` – node tests –,
`improve.svelte.js`): they **fill the Generate panel – nothing is sent**; Generate asks with the cost as always. Each
makes the card's shot current and opens Generate (unfolded). **Improve on this** (⚡, key **I**): Generate starts with
the line "Improve on g2" – the card's picture, how it goes in, ✕ = back to the shot's own inputs and the generator
before –, the card's prompt (editable: the new record keeps the edit) and the **Seed** New | the card's. The card goes
in where its kind fits, in the form's fields: an image → the still (Gen-4: the first frame; ComfyUI: image 1, an edit
– `--from`; Luma by hand: the reference), a video → Aleph's video + the chosen image as the keyframe at 0 (a video to
video model only – a frame of it is More ▾'s), a layout variant → its frame as the still (the shot stays), a take →
its render as the video, the render → the shot's own inputs. The pickers keep only the models that take the card;
when the current generator has none, the first configured one that has takes over (one that makes it before Luma by
hand) – the line says so. A still set afterwards (the Still select, More ▾) stays over the card's through generator
and model changes, and the line names it ("g3 as the first frame, set here – g1 is not in the form now"). Sent, its
placeholder lands in the card's child row (the job's main input is the card – `inputs[0]` / `parents`). **Vary** (↻):
the card's own inputs, prompt, generator, model and length with a new seed – a sibling in its row ("Vary g2"; not for
a dropped file or a file without a record); the chosen image among its inputs goes in as the file it was when the card
was made (its parent in the record – a later approval doesn't change what is repeated; gone → left out, said). **More
▾** (a dropdown from its button – no right-click menu; the focus back on it after an item, so the board's keys go on):
an image – **Use as start frame / keyframe** (the card becomes the still, the rest of the form stays; only where the
model takes it – Gen-4's first frame, Aleph's one keyframe at 0 –, else off with why); a video – **Still: the first
frame | the frame at <time> | the last frame** (the frame becomes the still, cut by ffmpeg – "The record"; the time =
where the card was clicked while it scrubbed – a line marks it; a time inside the last frame is the last frame); **Add
to / Remove from Compare** (the selection); **Show the file** (a new tab). A rejected card is refused (X first). The
form belongs to one shot of one chapter: another shot or chapter clears it before its inputs load (Generate and the
manual result slot wait for them; the server refuses a shot that isn't in the chapter and another chapter's inputs).
Without a render, ComfyUI and Luma by hand pre-fill no still (never an earlier result).

**Compare** (stage 4, VARIANTS-CANVAS §3.4; `src/lib/compare.js` – node tests –, `compare.svelte.js`,
`compare-clock.svelte.js`, `Compare.svelte`): **C** or **Compare · n** (the tool row, once a card is selected) splits the
main – the bands left, Compare right, a Divider between (normally 45 % of the room, at least 27rem; dragged, its width
is remembered – `storyboard-studio:compare`; Home / a double-click: back); ✕, Esc or C closes it (opened by the button,
the focus goes into Compare). Its sides are the **selection** (the letters on the cards): A the reference, the first
selected; fewer than two selected, the current shot's chosen (of the selected card's kind) comes in front as A –
nothing selected: beside the current card; one side leaves an empty B. Open, a **click on a card fills the first empty
side, else replaces the last** (A stays); Ctrl + click and More ▾ add / remove as before (at most 4 – a fifth replaces
D). A side taken out while Compare is open – **X** on it (a rejected card never enters: "X on side B folds it and
empties B"), Ctrl + click, More ▾, or its card gone from the board (a layout variant taken, a file's record gone) –
**stays empty**: a dashed slot, the other sides keep their letters (on the board too), the next click fills it; closed,
the selection closes up. The sides stand at **one height**, each in its shot's aspect, from any band (a side of another
shot than the current names it – "sb-25-1 · g4", narrow "25-1 · g4", narrower "25-1·g4" without the reference mark), as
large as the pane allows – nothing scrolls sideways (at 1440 beside Generate they scale down; T / N give the board +
Compare the window). Over each: the letter, the name, the status bullet (the cards' `StatusBullet` – a named image for
a screen reader); under each: model · length · seed · cost (each part whole; a narrow side leaves the seed out), what it
came from (its main input on the board + the other parents, how it went in), the prompt (3 lines); each line's whole
text in its tooltip. **Videos run on one clock** (Frame.io): one Play (**Space** – while Compare has the focus, and on
the board's cards while Compare is open), one scrubber (← → 0.1 s, Shift 1 s), the same moment in each – as long as the
longest; a shorter one holds its last frame; images stand. A webm without a duration (a recorder's file) is seeked to
its end once so its length is known (until then it plays along); a file that can't be played says so and leaves the
clock. At most 4 `<video>` elements, only while Compare is open – each lets go of its media (no src + `load()`) when its
side goes or Compare closes.

**The audition** (stage 4, VARIANTS-CANVAS §3.1 Q3 – owner 2026-10-20: "The current card"; `src/lib/audition.js` – node
tests –, `audition.svelte.js`): while the board is open, the current shot's **Generated** clip on the timeline is the
board's **current card** – the one the board marks current: the last clicked (the arrows' too), nothing clicked yet in
the current shot's band its chosen (the video first, else the image) –, tagged **audition** – a small ink tag beside
the clip's label. A rejected card, a source (the render, a layout variant, a take, a dropped file), a running job or a
card whose file is gone: the Generated mode's own clip, no tag (the approved video, else the newest – never a rejected
one: `GET /api/clips` returns each file's last mark from the log, so this holds before the board is opened too). An image auditions as a still of 5 s that shows the image (the Viewport shows it too).
**Play** shows it among the picked shots' clips: Play switches the main to the Viewport and the audition holds while
that Play runs (V back to the board); Play's end (or a pause), the board closed by hand (V, the switch, a source's Load)
or another step: the chosen again.
The timeline's Generated clips carry the **status bullet** (the cards' `StatusBullet`) and the generation's id ("g3")
in their tag: ✓ the chosen, ? review, ✕ rejected – from `GET /api/clips` (the log's marks); the board's own marks count
instead when it read the log after the clips did (`audition.js` `boardMarks` – a board closed earlier keeps its records,
a mark set since in another window or by hand comes with the clips). Only the Generated view mode; the lane takes the
board's marks and the pick as an override per shot (`clipsLane`, `markOf`).

## Files

- the shots: `storyboards/<toon>/<chapter>/scene-*.json` (written as 2-space JSON)
- variants: `storyboards/<toon>/<chapter>/variants/<shot>.json`, frames under `storyboards/_out/<chapter>/` (`frames/`,
  `clean/` without texts, `variants/`, `generated/` + `generated.jsonl` – the generators' results and their record,
  "The record" below)
- the generators: `generators/` (`jobs.mjs` the interface, `runway.mjs`, `comfy-local.mjs`, `luma-manual.mjs`,
  `others.mjs`, `rate.mjs` the euro rate; tests: `npm test -w workflows/storyboard-studio`); the rate's cache
  `.cache/eur-usd.json` (gitignored)

## Handheld camera (the phone as the shot's camera)

Owner 2026-10-19: film a shot's blocked scene with the phone, trim, render control videos for AI, download the camera
(research/HANDHELD-CAMERA.md: why WebXR, USB, the socket relay, the passes).

- **Per shot**: the **Handheld** step's panel → Handheld **On for <shot>** (stored in the shot's takes file, below).
- **Pairing**: "Connect a phone" shows a 4-digit code, a QR code and the phone link `/phone?code=1234` (a route of this
  app). The phone page shows the shot fullscreen through the shot camera (the studio look; "Plain" = a faster plain
  render). **Start (AR)** = WebXR `immersive-ar` (ARCore: position + rotation, Chrome on Android); **Start / Rotation
  only** = DeviceOrientation (rotation) + drag for position (one finger = move, two = up / down). The first pose starts
  at the shot camera; **Set origin** (phone or PC) = the phone's pose now is the shot camera's pose (heading matched,
  up stays up). The PC's 3D view follows live; **Smoothing** 0 = the raw handheld look … 1 = a slow steadicam;
  **Scale** = scene metres per phone metre (a miniature set: < 1).
- **Record / Stop** on the PC or the phone → a take: samples `{ t, position, quaternion, fov }` (~60 Hz, the scene's
  metres, Y up, glTF's camera convention) in `storyboards/<toon>/<chapter>/takes/<shot>.json` (`{ on, active, takes }`;
  raw samples + the take's smoothing, in / out). Several takes per shot, one active (the radio); delete = the bin.
- **Trim** (research/TIMELINE.md §4, way C): the timeline in Take mode (the Handheld step's) – the take is its shot's
  clip, the whole take, the cut parts hatched in place: drag its in / out (or ← → on them by a frame, Shift ×10, Alt ×0.1)
  or slip the kept part (both move, its length stays); **I / O** put the in / out of the take under the playhead at the
  playhead (I: the frame under it is the first one kept, O: the last one kept). The playhead goes to the moved point (the
  in, the last kept frame; a slip: the new in) and the 3D view shows the take's camera there; the takes file is written
  on release. While a trim runs the main view shows the **in / out pair** – the kept part's first and last frame side by
  side, the moved one marked (live; the other a still the 3D view renders at its moment) – one picture again 0.6 s after
  the release. Click / drag = the playhead; the timeline's Play = the kept parts. "Back to the shot camera" / "Shot
  camera" give the view back.
- **Render all handheld shots** (the Export step's **Output** panel): every shot with the camera on and an active take, along its
  trimmed, smoothed track, rendered in the studio's browser with the frames' renderer, at 24 / 25 / 30 fps, 1× or 2×.
  Passes: **Beauty** (the studio look – lines + hatch, also the lineart), **Depth** (white = near, inverse depth with
  one range per take), **Normals** (view space), **Grey** (plain lit). ffmpeg (on the PATH, or `FFMPEG=<path>`) writes
  `storyboards/_out/<chapter>/handheld/<shot>--<take>--<pass>.mp4` (H.264, yuv420p; + `.webm` VP9 when ticked) – the
  links appear under the button. These are the control videos for Runway / Luma / Wan VACE / ControlNet.
- **Downloads**: per take (the Handheld panel) "glTF" (`<shot>-<take>-camera.glb`: a perspective camera with its
  animation), "glTF + scene" (+ the blocked set and figures) and "JSON" (the trimmed, smoothed samples + the space it is
  in); the Output panel's "Download all (zip)" = both for every handheld shot + a README ("With the scene" there too).
  - **Blender**: File → Import → glTF 2.0 (.glb) – the camera comes with its action (Y up becomes Z up by itself).
    FBX: import the GLB in Blender, then File → Export → FBX.
  - **Unreal**: import the GLB (the glTF / Interchange importer), put the camera into a Level Sequence (Sequencer) –
    or import Blender's FBX as a camera track in Sequencer.
  - The lens is constant per take in the GLB (glTF animates no `yfov` without KHR_animation_pointer); the JSON has
    `fov` per sample.
- A take lives in the scene's world: the figures are placed through the shot camera, so changing the shot's size /
  angle / hand camera after recording moves them – record again.

### Connecting the phone

- **Wi-Fi (the default, no cable** – owner 2026-10-19: "i don t want to plug in a cable"): the studio opens a second
  server on the LAN, **`https://<this PC>:14443`** (the studio's port + 443; `phone-https.mjs`), and the Handheld panel's
  QR code points there – phone and PC in the same Wi-Fi, scan, done. The phone needs HTTPS for its sensors / WebXR: the
  studio makes a self-signed certificate for localhost + this PC's LAN addresses with `openssl` (Git for Windows has
  it; or `OPENSSL=<path>`) into `workflows/storyboard-studio/.cert/` (gitignored) – the phone warns once per
  certificate: **Advanced → Continue**. A new LAN address (another network) makes a new certificate (the old addresses
  stay in it). Windows asks once whether Node.js may use the network: allow **Private networks**. The PC itself keeps
  `http://localhost:14000` (127.0.0.1 only). Other machines get only GET / HEAD and the socket – every write stays on
  this PC; it reads only while the studio runs. `STUDIO_LAN=0 npm run dev:studio` keeps it off the LAN.
- **USB** (no Wi-Fi): phone → Developer options → USB debugging on; cable to the PC; in desktop Chrome
  `chrome://inspect/#devices` → **Port forwarding…** → `14000` → `localhost:14000`, tick "Enable port forwarding". The
  phone opens `http://localhost:14000/phone?code=…` (the panel's USB link) – localhost is a secure context, no
  certificate. (`adb reverse tcp:14000 tcp:14000` does the same.)
- Transport: the studio's own dev-server socket (`handheld-relay.mjs`, Vite's HMR WebSocket – over Wi-Fi tunnelled
  through the HTTPS server).
- Pairing (above): the QR / Wi-Fi link first, the USB link under it.

## Generators

Owner 2026-10-19: "Ja, generator pluggable ist smart. Erstmal soweit bauen, dass ich die Runway API anbinden kann."
The **Generate** panel sends a shot to a generator; the result lands on the Variants board
(`_out/<chapter>/generated/<shot>--g<n>.png|mp4`, recorded in `generated.jsonl` – "The record" below; a placeholder
while it runs), where ✓ / Enter approves it as the chosen. Each generator says what it takes; the inputs are
pre-filled from the shot (the approved image, else the render without texts – ComfyUI's image 1 is the render; the
newest handheld render pass; the shot's "Image (EN)" text as the prompt) – or from a board card ("Improve on this ·
Vary · More" above). **Build from the shot** next to the Prompt replaces it with everything set for
the shot – also unsaved changes – as one prompt (the story moment, perspective, figures, camera, light, empty zones,
the look, the script: `scripts/storyboard/luma.js` via `POST /api/prompt`); longer than Runway's 1000 characters, the
panel says that only the start is sent. Luma – by hand keeps its built prompt apart (read-only), so a prompt typed for
another generator stays – while a board card fills the form, it shows the card's prompt, editable. Every generator says how a job gets made – `transport` in `GET /api/generators`: `api` (a
provider's API), `local` (this PC) or `manual` (by hand: never started – the panel shows Copy prompt instead of
Generate; agents/research/GENERATORS.md §2).

| Generator | Transport | What | Cost |
|---|---|---|---|
| ComfyUI (this PC) – `comfy-local` | local | `scripts/storyboard/comfy.mjs`: Qwen-Image-Edit, image 1 = the still – the render without texts (its own default) or a generated image of the shot / the chosen (`--from`, an edit) – + character images → a still; its prompt is comfy.mjs's own | free |
| Runway – `runway` | api | **Gen-4 Turbo** / **Gen-4.5** image to video (the still + prompt, 2–10 s); **Aleph 2** video to video (the handheld render pass ≤ 30 s, restyled by the prompt; the still as keyframe) | 5 / 12 / 28 credits per second (Aleph at least 56); 1 credit = 0.01 USD (≈ 0,0089 €) |
| Luma – by hand – `luma-manual` | manual | no API: the built prompt (read-only) + **Copy prompt**, **Render (no texts)** (Luma's image reference, a download) and the references to attach (links) – made on lumalabs.ai; the result goes into the panel's **Result** slot (drop or pick: recorded as Luma – by hand's candidate, model `manual`, with the prompt, the Reference and the improved card as its parent – a drop on the board stays a source) | your Luma subscription |
| ComfyUI (cloud GPU) – `comfyui-remote` | api | the same graphs on RunPod / Comfy Cloud – not built yet (`generators/others.mjs`) | – |

Safety: the estimate is shown before sending; every paid call asks once more with the cost; nothing is retried by
itself (a failed or timed-out create call is reported, not sent again); the panel shows what this studio session has
spent (it resets when the studio restarts). Cancel stops the job and cancels the task at Runway.

Prices are shown in euro (owner 2026-10-20: "preise in euro"): the estimate, the confirmation ("Generate for 0,22 €"),
each job's cost and the session's spend, `de-DE` formatted; the tooltip of each figure shows the provider's own –
credits, USD – and the rate. The rate: the ECB euro reference rate (`generators/rate.mjs` – eurofxref-daily.xml, USD per
EUR → 1 / that), served with `GET /api/generators` and `GET /api/generate/jobs`, fetched at most once a day in the
background (a request never waits for it), cached in `.cache/eur-usd.json` (gitignored); offline → the last cached
rate, else a fixed fallback (ECB 2026-10-19: 1 USD = 0.8909 €), marked as such in the tooltip;
`STUDIO_EUR_PER_USD=<EUR per USD>` overrides it. Inside, the money stays in USD: the estimate, the jobs, the spend,
`generated.jsonl` and the confirm check of `POST /api/generate` (`confirm` = the USD behind the euro figure).

### The record (what made each generated file)

`_out/<chapter>/generated.jsonl` – one v2 line per generation, append-only (`scripts/storyboard/generated-log.mjs`, the
fields there and in agents/research/VARIANTS-CANVAS.md §3.7): id `<shot>--g<n>`, file, kind, take, generator, model,
mode, the provider's job, status, the inputs as keys with their roles (still · control · keyframe · reference ·
character · style), the parents (inputs under `generated/`; the approved image counts as the file it was approved
from), prompt, seed, ratio, duration, estimate, cost, date, the mark. Written by the jobs (a failed or cancelled job
too – an id, no file), drops (`generator: 'upload'`, png / jpeg / webp / mp4 / webm), the manual result slot
(`POST /api/generated` with the Generate form's `job` + `parent`: Luma – by hand's candidate, model `manual`, with the
prompt, the still as `reference` and the Improve-on card first in `parents`), comfy.mjs (`--from <shot>--g<n>.png`:
improve on a result) and gemini.mjs. A job's inputs are keys (`generators/job-fields.mjs`); a still can be a frame of a
video, `frame:out:<…video>#first|last|<ms>` – cut once by ffmpeg to `_out/<ch>/generated/frames/` (named by the
video's mtime + size: again when the video is replaced, also by an older copy; `POST /api/generate/frame` cuts it for a
thumbnail) and recorded as the video's key + `frame`, the
video its parent. `GET /api/generate/inputs` offers the shot's generated images as stills, its generated clips as
Aleph's video and their first / last frames as stills, the layout variants' frames; each record from `GET
/api/generated` carries its `key` (the card as an input). Names are always the highest n + 1 over the files and the
log, the file created exclusively (never written over). A model that takes a seed (Runway, ComfyUI) gets one picked
when none is given. The marks: `POST /api/generated/mark { ch, ids, mark: 'review' | 'rejected' | null, marked }` appends one line per
id, the last wins; approving appends `mark: null`. Chosen is no mark: approve.mjs writes `ch` and
`files.from: "<shot>--g<n>.<ext>"` into the dataset line – the chosen image and the chosen video (clips.mjs; an approval
from before 2026-10-20 is found by its copy's bytes). `GET /api/generated?ch=&ids=a,b` answers the picked shots' records
(generated.mjs); a file without a v2 record comes back as `earlier: true` (nothing guessed). Old lines are converted once,
by hand – a dry run with a report first:

    node workflows/storyboard-studio/scripts/convert-generated-log.mjs [--ch s1c3] [--write] [--json]

### Runway einrichten (für den Owner)

1. Auf **https://dev.runwayml.com** anmelden (das Entwicklerportal – nicht das normale Runway-Abo, die Credits sind
   getrennt), ein Projekt anlegen, Credits kaufen – Runway rechnet in Dollar: 1 Credit ≈ 0,0089 € (0,01 USD). Der
   Start-Tier erlaubt bis ≈ 89 € (100 USD) im Monat und wenige Generierungen gleichzeitig / pro Tag; mehr gibt es
   automatisch nach den ersten Käufen.
2. Unter **API Keys** einen Schlüssel erzeugen und kopieren (er wird nur einmal angezeigt).
3. In die Datei **`.env` im Repo-Ordner** (`C:\Development\scutoons\.env`, sie ist in `.gitignore`, wird nie
   committet) eine Zeile schreiben: `RUNWAY_API_KEY=key_…`. Ein Neustart des Studios ist nicht nötig.
4. Im Studio: Shot wählen → **Generate** → Generator „Runway“, Modell wählen, Bild / Video und Prompt prüfen →
   **Generate** → die Rückfrage zeigt den Preis in Euro → bestätigen. Das Video erscheint auf dem Variants-Board (V).

Alle Preise im Studio stehen in Euro – ungefähr (≈), umgerechnet zum Euro-Referenzkurs der EZB des Tages; fährt man mit
der Maus über einen Betrag, zeigt der Tooltip Runways eigene Zahlen (Credits, USD) und den Kurs. Abgerechnet wird bei
Runway in USD, darum kann der Euro-Betrag auf der Kreditkarte leicht abweichen.

Was ein Shot kostet (Preise laut docs.dev.runwayml.com, Stand 2026-10-19; in Euro zum EZB-Kurs vom 2026-10-19,
1 USD = 0,8909 €, gerundet – Runways Dollar in Klammern):

| Modell | pro Sekunde | 5-s-Shot | 10-s-Shot |
|---|---|---|---|
| Gen-4 Turbo (Bild → Video) | 5 Credits ≈ 0,04 € (0,05 USD) | ≈ 0,22 € | ≈ 0,45 € |
| Gen-4.5 (Bild → Video) | 12 Credits ≈ 0,11 € (0,12 USD) | ≈ 0,53 € | ≈ 1,07 € |
| Aleph 2 (Handheld-Video → Video) | 28 Credits ≈ 0,25 € (0,28 USD), min. ≈ 0,50 € (0,56 USD) | ≈ 1,25 € | ≈ 2,49 € |

Ohne Schlüssel zeigt der Generator „not configured“ mit dieser Anleitung in einer Zeile. Der Schlüssel bleibt auf dem
PC: er geht nie an den Browser und nie in Git.

For development: `STUDIO_FAKE_GENERATOR=1` adds a fake generator (fake credits, 1 credit = 0.01 USD); `RUNWAY_API_URL` points the adapter at
the mock (`node workflows/storyboard-studio/generators/test/runway-mock.mjs --port 14091`, key `mock-key`).

## Not yet

- joints posed by hand in 3D (poses come from the pose list), props placed by hand, undo;
- the look saved per chapter (the switches are a view setting; `render-shots.mjs --look hatch,inner` for files);
- the move into the creator, if the team works with it (agents/STORYBOARD.md §7 / phase 5).
