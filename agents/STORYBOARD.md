# STORYBOARD — script → stacked shots → schematic storyboard → chapter draft + image prompts (research, 2026-10-09)

Research only, nothing built. Illustration: [storyboard-example.html](storyboard-example.html) (one scene, hand-written
SVG, open it in a browser). Pipeline context: AUTOMATION.md (story → script → scenes → frames). Closest relative:
TREATMENTS.md (per-scene shot treatment for generators). Manual precursor: `workflows/scenes/generate-shots.md` (FLORA).

**Revision 2 (owner 2026-10-09):** "Webtoons have no panels – images are stacked one after another." So the unit is
the **shot = one image** (one slice), not a panel with gutters. Space, pauses and transitions live *inside* the
images. The owner's parameters so far: **viewport**, **vertical**, **horizontal loop (360°)**, and for tall shots the
**projections** perspective / **polar coordinates** / **isometric** / **top-down**, at aspects from **9:16 via 1:4 up
to 1:7** (beyond 1:4 most generators give up).

## Summary + recommendation

- **The storyboard is a list of shots, stacked.** Each shot = one image on the 390-wide canvas, delivered as one slice or
  cut into 625-high slices like chapters 1 and 2 (a 360° loop = one panorama, cut into the slides of a looping
  horizontal group). There is no gutter field, and none is needed: a pause
  is a shot (or part of one) with "air" – sky, a colour field, darkness – and the storyboard marks that air as zones,
  so the rhythm can be measured.
- **A shot is described by four things:** its **format** (viewport · vertical · loop360), its **aspect** (9:16 … 1:7),
  its **projection** (perspective · top-down · isometric · polar) and its **scroll move** (what the camera does while
  the reader scrolls through it: tilt, fall, travel over the ground, descent into the spiral …). On a vertical shot the
  reader's scroll *is* the camera move. That is the webtoon's real grammar, and the storyboard has to show it.
- **One intermediate JSON is still the core**; SVG and prompts are views of it. It stays close to `GroupDoc`: a group
  has `label`, `renderer`, `events`, `continues`; its `slices` become `shots` (layout + content instead of an image).
  Text boxes, items, events and sound are the chapter's own types. Compiling to a chapter is simple: stack the shots, cut
  for delivery.
- **Generators are the bottleneck for tall shots**, not the layout. Up to 9:16 (some 9:21) works natively; 1:4 rarely;
  1:7 nowhere. So every shot carries an **intended** aspect and a **generation plan** (native · segments + outpaint ·
  generate as a panorama and transform). The plan follows from the projection (§3).
- **Order: schematic first, prompts second.** The schematic SVGs, rasterized to 390-wide WebP, become the slice images
  of a draft chapter in the sandbox toon (with real text boxes, VN events, items). The **Reader dock** then shows the
  rhythm on a phone – stack, loops, VN – without new UI. The static HTML page is the offline / agent view.
- **Characters: registry ids + reference pack** (base views of persona + look, mood portrait) + the schematic as a
  "layout only" image. Multi-reference models in FLORA (Nano Banana Pro, GPT Image) work today; a LoRA (FLORA: Flux 2)
  only for the main cast, once the reference pack drifts.
- **Phase 1 is an agent workflow** on the existing MCP server (`tools/mcp`). A creator feature comes later and is
  mostly the rhythm rail in the Reader dock.
- **Everything runs in the cloud, only while there are jobs** (owner 2026-10-09: no own GPU). Open models on a
  serverless GPU (ComfyUI, scale to zero) cover the hard parts – no aspect cap, the schematic as a ControlNet input,
  seamless panoramas, own character LoRAs; FLORA for the closed models (§3 "Open models on a serverless GPU").
- **One budget, set in the admin, hard cut** (owner 2026-10-09): one amount of money; every paid job goes through the
  API, which books its cost and refuses the next job once the budget is used up, with a message (§7 "Budget").
- **The storyboard is also the work breakdown:** one shot = one work unit with a status, so the owner, artists, a
  generation worker and agents can work on different scenes / lanes at once (§7 "Parallel work").
- **Start with a diagnosis, not a build:** the rhythm measures already work on chapter 1 (see "Why") – 70 screens,
  text in walls of up to 89 words per screen next to 13 silent slices. That is the case for the tool.

## Why: chapter 1 measured (owner 2026-10-09: "still very packed – I had no idea about rhythm yet")

Goal of the workflow (owner): **better rhythm, images that can be planned, script → toon more efficiently.** Measured
on the repo's copy of chapter 1 "Leugnen" (`server/tests/fixtures/content/catpope/_/chapter1.json` – a test copy, it
may lag behind production; chapter 2 and the images could not be read here: `api.` / `cdn.scutoons.com` are blocked
in this cloud environment):

- **95 slices** (the key visual 390 × 845 with video + 94 × 390 × 625) = one long canvas cut into 625-high slices –
  **70.6 phone screens**, one group, no VN, no items.
- **70 text boxes, 894 words (de).** Words per slice, in reading order:
  `0 0 13 10 0 6 68 23 0 30 5 0 55 15 0 39 0 3 4 10 20 3 18 2 38 28 0 8 18 0 5 3 9 3 16 9 7 9 0 0 0 3 9 14 9 0 3 0 18 30 0 13 0 4 0 0 0 4 11 26 37 0 6 0 0 0 0 0 0 0 0 0 0 0 0 33 0 22 0 0 18 0 5 0 7 8 0 87 19 0 0 0 19 0 12`
- Per phone screen (a window of 844 sliding in quarter screens): median **9 words**, but **24 of 279 windows > 35
  words** (a "wall"), the worst **89**. Single slices with 68, 55, 87 words.
- The text is **lumpy, not paced**: the first third alternates walls and empties with no build-up; then **13 silent
  slices in a row** (63–75, ~10 screens) – either a deliberate action stretch or air that wasn't planned as such.
- Music changes at slices 1, 17, 21, 28, 32, 95 – the sound has a structure the text doesn't follow.

What this says for the tool: the measurements in §6 work on today's chapters already (no storyboard needed), so the
**rhythm rail is useful from day one** – for chapter 1/2 as a diagnosis, for chapter 3 as the plan. With the images
visible (owner: allow the hosts, or a screenshot export) the next step is to mark per slice: shot boundaries, air,
figures – the real "Ist" storyboard of chapter 1 as the reference to compare against.

## What a chapter is today (the target format)

Verified in the code 2026-10-09. Types: `shared/content/src/content.ts`, `visual-novel.ts`, `audio.ts`, `fx.ts`,
`characters.ts`; stored language-neutral (`neutral.ts`, every text a locale map).

| Part | Today | For the storyboard |
|---|---|---|
| Image = slice | `Slice {image, width, height, …}`. Width 390 (reference), height free (no upper limit on images; default 625). Slices stack edge to edge (`renderer/vertical.svelte`). Rects in % of the slice, font size in % of the width | a shot = one image, one or more slices. 9:16 = 390 × 693, 1:4 = 390 × 1560, 1:7 = 390 × 2730 (3.2 phone screens) |
| Viewport | `Slice.video.viewport {fit, position}` (full-screen video); in-column video `frame {ratio}` clamped 4:1 … 1:4 | format `viewport`: one screen, a stop. A still viewport = a 9:16 image; a moving one = video |
| Vertical group | `renderer.type: 'vertical'` | format `vertical` |
| Horizontal | `horizontal` → Swiper (1 slide per view; `loop`, `freeMode`, `autoPlay`, `cube` / `flip`). `horizontal` + `fullWidth` → the pinned strip (`strip.svelte`: screen-high, moved sideways by the page scroll, travel computed). **The strip doesn't loop; the Swiper does** | `loop360` = the Swiper with `loop: true` + `freeMode: true` (continuous drag that wraps). A one-way sideways pass = the strip |
| VN | any group with `events` gets the overlay (`group.svelte`). Events: `joinScene` / `leaveScene`, `showMessage`, `addChoice` + `showChoices`, `askText`, `showTextBox`, `fx`, `jumpTo`, `label`, `divert`, `setVariable`, `giveItem` / `takeItem`, `if`. No x/y: the view places a character (avatar left, portrait right, full centre; `stage.ts`) | a VN group = a backdrop shot (no characters in it) + real `VNEvent`s |
| Elements | `Dialog` (text boxes, live text, `variant` bubble / caption / narration), `SliceItem` (pickup / events), slice `events` with `trigger` (intersect / hover / click), audio samples + music per slice, the `fx` event | reused 1:1, rects in % of the shot |
| Characters | registry Character → Persona → Look → Mood × View (avatar / portrait / full; apose). Files `characters/<char>[-<persona>]/<char>[-<persona>][_<look>]_<view>-<mood>.webp` | figures = `{character, persona, look, mood}` + framing; references by the same convention |
| Script | `script:<toon>:<l>` (creator only), `scriptOutline()` → scenes with line ranges. No script → chapter conversion exists | `source` = script key + scene + lines (de = master) |
| Builders | `creator/src/lib/editor/defaults.ts` (`newGroup`, `newSlice`, `newDialog`, `nextDialogId`), `scripts/make-sandbox.ts` → `import-content` | the compiler reuses them |

## 1. The shot vocabulary (owner's parameters, made explicit)

### Formats

| Format | Aspect | In the reader | Reading | Use |
|---|---|---|---|---|
| `viewport` | 9:16 (= the screen) | one image filling the screen; a video with `viewport` | a **stop**: the reader sees all of it at once | establishing, impact, a beat that must land whole, the key visual |
| `vertical` | 9:16 → 1:4 → 1:7 | one tall image, scrolled through | a **move**: the scroll is the camera. Height = duration | falls, climbs, reveals, travel, descent, long establishing pans |
| `loop360` | a seamless panorama (2:1 … 6:1), cut into N slides | horizontal group, `loop` + `freeMode` | **looking around**: the reader stops scrolling and drags; no end | a place to explore (room, square, crowd), hidden items, "where is …" |

Between them: a one-way **sideways pass** (the pinned strip, `fullWidth`) for chases and walks – a vertical shot
turned 90°.

### Projections (for `vertical` and `loop360`)

| Projection | What the scroll does | Typical aspect | Generation (§3) |
|---|---|---|---|
| `perspective` | **tilt / crane**: sky → horizon → ground, or a figure head → feet; with a horizon the image has a "top" | 9:16 – 1:4 | native up to 9:16; beyond: segments + outpaint downwards |
| `top-down` | **travel over the ground**: a street, a river, a table, from above; no horizon, no gravity | 1:4 – 1:7 | rotation-free: generate as a wide landscape panorama, turn 90° (or segments); seams are easy (ground texture) |
| `isometric` | **through a stack**: floors of a building, layers of a cliff, a tower; the camera moves without perspective | 1:4 – 1:7 | segments of a fixed grid (the iso grid makes them line up); or one floor per generation, composited |
| `polar` | **descent / spiral / rabbit hole**: the horizon bends into a circle; scrolling = going inwards / downwards; "tiny planet" at the bottom | 9:16 – 1:7 | generate a flat panorama (2:1 … 4:1, which generators do well), then **transform by code** (rectangular → polar); tall polar shots = the transform over a long strip, or a stack of tiny-planet segments |

Polar and loop360 share their source: **one panorama** of a place can be the 360° loop *and*, polar-transformed, the
tiny planet / descent shot of the same place – consistent by construction.

### Scroll moves (the camera vocabulary of a stacked webtoon)

`hold` (viewport) · `tilt-down` / `tilt-up` · `fall` (figure falls through the shot) · `climb` · `travel` (top-down
over ground) · `descend` (isometric floors / polar spiral) · `reveal` (air first, the subject at the bottom) ·
`look-around` (loop360) · `pass` (sideways strip). Shown in the schematic as a scroll arrow along the shot's side.

### Air (the pauses)

A pause is not a gap between images; it is a zone in an image (or a whole shot) with nothing to read: sky, darkness,
a colour field, a texture. The storyboard marks it as `air: [[from %, to %]]` per shot. It drives the rhythm
measurement (§4) and the prompt (`keep the top 40 % empty night sky`). A pure transition shot (one colour, a fade from
night to day) is a shot with `air: [[0, 100]]` – generated by code, not by an AI.

## 2. The intermediate format (storyboard JSON)

Principles:
- As close to `GroupDoc` as possible; only `slices` becomes `shots`. Everything a slice already knows (`dialog`,
  `items`, `events`, `audio`, `video`) uses the chapter's own types, rects in % of the shot.
- New fields only for what the storyboard decides and the chapter doesn't store (PLAN.md "keep the configuration
  small"): format, aspect, projection, move, air, framing, figures, location, action, generation plan. They stay in
  the storyboard (creator-only) and drive the schematic, the prompts and the compile; none go into the reader's chapter.
- Text = locale maps, de first. Ids stable: `sb-<scene>-<group>-<n>` = the shot id of the prompts and stills.

Example (JSONC; the same scene as [storyboard-example.html](storyboard-example.html)):

```jsonc
{
  "storyboard": 2,                              // format version
  "toon": "catpope",
  "source": { "script": "script:catpope:de", "chapter": 3, "scene": 1, "lines": [412, 468] },
  "chapter": 3,                                 // target chapter (draft); null = sandbox only
  "style": "style:catpope",                     // the toon's style block (TREATMENTS.md open decision)
  "cast": [
    { "ref": "catpope", "character": "catpope", "persona": "regular", "look": "default" },
    { "ref": "boy",     "character": "boy",     "persona": "regular" }          // "boy" = an example id
  ],
  "locations": [ { "ref": "roof", "place": "vatican-roof", "time": "night" } ],  // map place (MAP.md §8.3)

  "groups": [
    {
      "label": "s1-wake",
      "renderer": { "type": "vertical" },
      "continues": true,
      "shots": [
        {
          "id": "sb-3-1-1",
          "format": "viewport", "aspect": "9:16",          // a stop: one screen
          "projection": "perspective", "move": "hold",
          "shot": { "size": "wide", "angle": "high" },     // size: ecu cu mcu medium full wide ews · angle: eye high low bird worm dutch
          "location": "roof",
          "action": { "de": "Der Katzenpabst steht klein auf dem Dachfirst. Mond rechts oben, Rom darunter." },
          "air": [[0, 30]],                                // the top 30 %: night sky, nothing to read
          "figures": [
            // y = position ALONG the shot (0 top … 1 bottom), x across; h = figure height as a fraction of the SCREEN
            { "cast": "catpope", "framing": "full", "mood": "neutral", "facing": "right", "x": 0.52, "y": 0.45, "h": 0.08 }
          ],
          "dialog": [ { "variant": "caption", "rect": { "x": 5, "y": 82, "width": 44 }, "text": { "de": "Rom. 3 Uhr nachts." } } ],
          "gen": { "plan": "native" }
        },
        {
          "id": "sb-3-1-2",
          "format": "vertical", "aspect": "1:4",           // 390 × 1560: ~1.8 screens of scrolling
          "projection": "perspective", "move": "fall",
          "shot": { "size": "full", "angle": "high" },
          "action": { "de": "Er rutscht ab und fällt am Glockenturm vorbei in die Tiefe." },
          "air": [[0, 15], [80, 100]],
          "figures": [
            { "cast": "catpope", "framing": "full", "mood": "scared", "pose": "falling", "x": 0.4, "y": 0.35, "h": 0.25 }
          ],
          "dialog": [ { "id": "sfx-aaah", "variant": "caption", "font": "lettering", "rect": { "x": 55, "y": 30, "width": 40 },
                        "text": { "de": "AAAH" } } ],
          "events": [ { "trigger": { "type": "intersect" }, "type": "showTextBox",
                        "params": { "dialog": "sfx-aaah", "effect": "shiver", "sound": "whoosh" } } ],
          "gen": { "plan": "segments", "segment": "9:16", "overlap": 0.25 }   // 1:4 = 3 segments, outpainted downwards
        },
        {
          "id": "sb-3-1-3",
          "format": "vertical", "aspect": "1:7",           // 390 × 2730: ~3.2 screens, the long descent
          "projection": "polar", "move": "descend",
          "shot": { "size": "ews", "angle": "bird" },
          "action": { "de": "Die Stadt krümmt sich zu einem Kreis; er fällt spiralförmig hinein, unten der kleine Planet: der Hof." },
          "air": [[0, 10]],
          "figures": [
            { "cast": "catpope", "framing": "full", "mood": "scared", "pose": "falling", "x": 0.5, "y": 0.55, "h": 0.06 }
          ],
          "items": [ { "item": "cucumber", "action": "pickup", "rect": { "x": 62, "y": 88, "width": 10 } } ],  // = SliceItem
          "gen": { "plan": "panorama-polar", "source": "pano-roof" }  // from the place's panorama (see loop360 below)
        }
      ]
    },
    {
      "label": "s1-courtyard",
      "renderer": { "type": "horizontal", "params": { "loop": true, "freeMode": true } },  // the 360° loop
      "shots": [
        {
          "id": "sb-3-2-1",
          "format": "loop360", "aspect": "4:1",            // the whole panorama; cut into 4 slides of 1:1 (or screen-high 9:16 × n)
          "projection": "perspective", "move": "look-around",
          "id_panorama": "pano-roof",                      // shared with the polar shot above: same place, same pixels
          "action": { "de": "Der Hof bei Nacht rundum: Brunnen, Tor, das erleuchtete Fenster, Mülltonnen." },
          "figures": [
            { "cast": "boy", "framing": "full", "mood": "neutral", "facing": "left", "x": 0.62, "y": 0.6, "h": 0.3 }  // x = position in the 360 (0…1 = 0…360°)
          ],
          "items": [ { "item": "cucumber", "action": "pickup", "rect": { "x": 91, "y": 70, "width": 3 } } ],  // hidden behind you
          "audio": { "samples": [ { "id": "fountain", "loop": true } ] },
          "gen": { "plan": "panorama", "seam": "wrap" }    // left and right edge must meet (§3)
        }
      ]
    },
    {
      "label": "s1-meet",
      "renderer": { "type": "vertical" },
      "shots": [                                           // the VN backdrop: one viewport plate, no characters in it
        { "id": "sb-3-3-1", "format": "viewport", "aspect": "9:16", "projection": "perspective", "move": "hold",
          "plate": true, "location": "roof", "shot": { "size": "wide", "angle": "eye" },
          "action": { "de": "Dachfenster, warmes Licht von innen, sonst Nacht." }, "gen": { "plan": "native" } }
      ],
      "events": [                                          // = GroupDoc.events: real VNEvents
        { "type": "joinScene",   "params": { "characters": [ { "character": "boy", "persona": "regular", "variant": "portrait" } ] } },
        { "type": "showMessage", "params": { "text": { "de": "Ein Junge steht im Dachfenster." }, "wait": true } },
        { "type": "joinScene",   "params": { "characters": [ { "character": "catpope", "persona": "regular", "variant": "avatar" } ] } },
        { "type": "showMessage", "params": { "character": "catpope", "mood": "confused", "text": { "de": "Du … kannst mich sehen?" }, "wait": true } },
        { "type": "showMessage", "params": { "character": "boy", "mood": "happy", "text": { "de": "Komm mit!" } } },
        { "type": "addChoice",   "params": { "label": { "de": "Ihm folgen" }, "onSelect": { "divert": { "to": "follow" } } } },
        { "type": "addChoice",   "params": { "label": { "de": "Gurke geben" },
                                             "onSelect": [ { "take": { "item": "cucumber" } }, { "divert": { "to": "gift" } } ] } },
        { "type": "showChoices" }
      ]
    }
  ]
}
```

The compile (storyboard → chapter draft), deterministic:
1. **viewport / vertical:** the shots are stacked on one 390-wide canvas (as chapters 1 and 2 are made: one long
   canvas, cut into 625-high slices). Two delivery options, the storyboard doesn't care: **one shot → one slice**
   (`height = 390 × h/w`), or **the canvas cut into 625-high slices** like today (lazy loading, smaller files; the cut
   moves off text boxes and items). Dialog / items / events / audio are converted from % of the shot to % of the
   slice. `image` = the rasterized schematic, later the still.
2. **loop360:** the panorama → N slides (slices) of equal width, the group gets `loop` + `freeMode`. Items / dialogs
   are assigned to the slide they fall in (a rect over a cut is moved, with a warning).
3. **VN:** plate shots → the group's slices; `events` copied, ids `<label>-<n>`.
4. Ids via `defaults.ts` / `nextDialogId`; `validateFile` before `save_draft` (the MCP server does it). Unknown
   characters / moods / items → the wanted list. The panel id may go into the slice's optional `_label`.

## 3. Generation: tall shots, loops, projections

The hard part is the aspect, not the layout. State Oct 2026 (to re-check per model): FLORA's default model accepts
`21:9 … 9:16` (TREATMENTS.md); most models top out at 9:16 – 9:21; 1:4 is rare; 1:7 is not offered anywhere. So
`gen.plan` decides how a shot is made, from its projection:

| Plan | How | For | Risks |
|---|---|---|---|
| `native` | one generation at the aspect | 9:16 viewport, plates | – |
| `segments` | n segments of 9:16 top → bottom; each next one gets the previous bottom 25 % as its top (outpaint / "extend down"), the same references, the same seed area of the prompt; composited with a feathered seam | perspective tilts / falls (1:4 – 1:7) | drift over many segments (light, scale); a figure on a seam. Rule: **no figure and no text box on a seam**; the schematic shows the seams |
| `panorama-rotate` | generate a wide landscape (21:9 or wider, stitched) of a top-down / isometric scene, turn it 90° | top-down, isometric | isometric has a direction (the grid must be drawn "upright" after the turn → say so in the prompt) |
| `panorama` (`seam: wrap`) | wide panorama; check the wrap: roll by 50 %, inpaint the now-central seam, roll back | loop360 | perspective lines bend in an equirectangular image; a cylindrical panorama (straight verticals) reads better on a phone |
| `panorama-polar` | the panorama → polar transform by code (x → angle, y → radius; "tiny planet"); a tall polar shot = the transform over a long strip, or a stack of tiny-planet segments; optional AI pass over the result to clean the centre | polar | the centre smears (radius → 0): cover it with the subject or a plate; text never in the transform |
| `code` | no AI: colour fields, fades, gradients (pure air shots) | transitions | – |

Practical notes:
- **The schematic is the layout reference for every segment** (cropped to the segment), so the composition across
  segments is planned once.
- Dedicated panorama tools (360° skyboxes, e.g. Blockade Labs Skybox) are an option for `loop360` / `polar`; not in
  FLORA (to check).
- **Video viewports:** a still viewport can become a short loop (image → video models in FLORA); the slice's `video`
  with `viewport`, the still as poster.

### Open models on a serverless GPU (owner 2026-10-09: no own GPU, everything in the cloud, only on while jobs run)

Open models (run by us on a rented GPU) fit this workflow better than the closed services in three places: **any aspect** (no 9:16 cap; tall
shots via tiled / outpainted generation), **hard layout control** (ControlNet: the schematic SVG itself becomes the
control image) and **seamless wrap** (circular padding = loop360 seamless by construction). The price is setup and
GPU seconds. State of my knowledge (mid-2026) – versions, licences, GPU sizes **to verify when setting up**:

| Piece | Recommendation | Why |
|---|---|---|
| Runner | **ComfyUI** | node graphs saved as JSON (versionable in `workflows/`), an HTTP API + queue (`/prompt`) → a job is submitted, the GPU wakes, works, scales back to zero; the largest ecosystem for ControlNet / LoRA / tiling |
| Base models | **Qwen-Image** + **Qwen-Image-Edit** (Apache 2.0; edit takes several reference images), **FLUX.1 / FLUX.2 dev** (open weights, *non-commercial model licence – check what it says about outputs*), FLUX.1 schnell (Apache 2.0, fast, weaker), the SDXL family (Illustrious / NoobAI for comic / anime styles – the biggest LoRA + ControlNet choice, runs on less VRAM) | pick by the toon's style; test 2–3 on the same shot list |
| Consistency | a **LoRA per persona + look**, trained on the same serverless GPU (ai-toolkit, kohya_ss or OneTrainer – one paid job per LoRA) from the base views + approved stills; plus reference editing (Qwen-Image-Edit) or IP-Adapter / PuLID for faces | once trained, a LoRA makes every image cheaper (no reference pack, fewer retries) |
| Layout | **ControlNet** (union / lineart / scribble / depth) fed with the rasterized schematic; **OpenPose** from the figures (the storyboard can emit a pose skeleton per figure later) | the schematic stops being a loose hint |
| Tall shots | tiled generation (MultiDiffusion / tiled diffusion) or outpainting downwards (FLUX Fill / inpaint models) in 9:16 segments | 1:4 – 1:7 without a cap |
| Loop360 / polar | circular padding in X (seamless-tiling nodes), panorama LoRAs; the polar transform as a node or our own script | the wrap seam disappears |
| Upscale | tiled upscalers to 1170 wide (the 3x level, PLAN.md "one file per quality level") | – |
| Video | Wan 2.x (Apache 2.0), LTX-Video | viewport loops from stills |

GPU (rough): **VRAM decides the GPU class.** 24 GB (L4 / A10 / 4090 class) runs FLUX-class models in fp8 +
ControlNet; 48–80 GB (L40S / A100 / H100) for the largest models at higher precision, faster, dearer per second.
Order of magnitude: tens of seconds per 1-megapixel image on a 24 GB GPU.

Hosting (to compare when setting up): **serverless GPU** platforms that bill per second and scale to zero, with
ComfyUI as a container or a ready "ComfyUI API" endpoint (e.g. RunPod Serverless, Modal, Replicate, fal, Comfy's own
cloud API). Requirements: scale to zero, per-second billing, an idle timeout, the model weights on one volume (the only
standing cost: GB-months), an HTTPS endpoint + key, the job's cost readable afterwards (for the budget booking).

Integration: the cloud worker session submits the ComfyUI workflow per `gen.plan` to the endpoint, waits for the
result, uploads it into the asset manager (MCP `upload_asset`) under the shot id, and books the cost (§7 "Budget").
FLORA stays for quick tries and the closed models (Nano Banana Pro, GPT Image).

## 4. Output

### a) Schematic SVG (rhythm)

- The reader's coordinates: width 390, every shot at its aspect's height, stacked edge to edge like in the reader.
  A loop360 shot shown as its panorama strip, scrollable sideways with the wrap marked; a phone frame shows the slide.
- **Per shot:** an outer frame + a label (`sb-3-1-2 · VERTICAL 1:4 · PERSPECTIVE · FALL`), the **scroll move as an
  arrow** down the side, **air zones** hatched, the projection drawn as guide lines (horizon for perspective, a grid
  for isometric, concentric circles + radials for polar, a ground pattern for top-down), **figures as silhouettes** in
  the registry `color`, labelled `id · framing · mood`, a gaze / movement arrow, text boxes with the real text,
  dashed item boxes, and – for `segments` – the **seams** as dotted lines.
- Clean line art, light tints only: usable later as a sketch / scribble control input.
- **VN:** one small phone frame per beat of the event list, the stage by the real rules.
- **Pages:** a static HTML per scene (agent output); the real target is the draft chapter in the Reader dock.

### b) Prompts per shot (content)

Assembled by code from canonical blocks (TREATMENTS.md), never written freely by the AI:

```
[STYLE]      the toon's style block, verbatim
[SHOT]       {format} {aspect}, {projection} projection, {size} shot, {angle} angle; the viewer scrolls {move}
[SETTING]    location block (place + time of day), verbatim
[CHARACTERS] per figure: "@{ref} = {name}, {persona/look block}, {mood}, {pose}, facing {facing},
             at {x} across and {y} along the image, about {h} of a screen tall"
[ACTION]     the shot's action line (en, from de)
[AIR]        "keep {from}–{to} % of the height empty: {sky / darkness / …}"
[LAYOUT]     "Image 1 is a LAYOUT SKETCH ONLY: keep the composition and figure positions, ignore its style and labels.
             Images 2..n are character references: keep identity exactly."
[NEGATIVE]   no text, no letters, no speech bubbles, no panel borders, no watermark; calm areas where text boxes sit
[SEGMENT]    (segments) "This continues the image above: match its bottom edge exactly" + the previous segment
[PANORAMA]   (loop360 / polar) "seamless 360° panorama, left and right edges continue each other"
```

References (role → asset): `layout` = the rendered schematic (or its segment), `character` = the persona + look's
`full` / `portrait` view + the mood portrait, `location` = the place's approved plate / panorama, `previous` = the
segment above or the last approved shot of the group.

Rules: no text in images ever (text boxes are live and translatable). VN plates have no characters (sprites come from
the registry; missing moods go to the moods workflow). The intended aspect is kept; the generator profile maps it.

### c) Both from one JSON: yes

| | Schematic / draft chapter | Prompts |
|---|---|---|
| Tests | rhythm, scroll length, air, text amount, branching, interaction placement | content, look, consistency |
| Deterministic | yes | no |
| Cost | none | ~$0.004–0.04 per generation (FLORA), × segments × retries |
| Build effort | renderer ~1–2 days, compiler ~1–2 days (simpler now: shot = slice) | assembly ~1 day; segment / panorama / polar tooling ~2 days |
| Order | first | second, after the rhythm gate |

## 5. Characters for image generators

- **Fixed ids:** `character` + `persona` + `look` (registry); in prompts `@catpope` (the LTX "Elements" / Runway `@tag`
  pattern). Descriptions come from the registry, assembled once per persona + look.
- **Reference images:** the base views (`workflows/characters/base-views.md`: avatar, portrait, full, A-pose) are the
  character sheet. Missing views → the wanted list before generation. Tall shots show figures from above / below and
  falling: a **turnaround** (front / ¾ / side / back, plus one from above) would help most. Proposal: an optional
  generator-only view `turn` for the main cast (owner decision).
- **Small figures in huge shots** (1:7 polar, top-down) are where references matter least and silhouettes most: the
  persona must read from its shape (cat ears, robe). The schematic's silhouettes should carry that marker.
- **Layout control:** the schematic PNG as a "layout only" reference. FLORA has no ControlNet / pose models; hard
  control needs Scenario (sketch ControlNet + LoRA), Leonardo (pose / depth / edge guidance) or ComfyUI. Not needed at
  the start.
- **Generators:** (1) reference pack, no training: Nano Banana Pro (up to 14 references, up to 5 people) or GPT Image
  (up to 16 inputs), both in FLORA; ≤ 2 characters per shot reliable. (2) LoRA for the main cast: FLORA trains Flux 2
  and Krea 2 Turbo LoRAs (10–30 approved images per persona + look). (3) Hard layout: Scenario / ComfyUI. (4) **Open models on a serverless GPU**:
  ComfyUI + open models + own LoRAs + ControlNet from the schematic – §3.
- **Judge pass:** generate-shots.md's free i2t judge (reference + still, "describe both first"); the owner approves per
  scene. For segments also a **seam check** (the judge compares the overlap).
- **FLORA gotcha:** generate-shots.md: `image_url` singular for generation, a plural array silently ignored; the
  catalog lists multi-image models with `image_urls[]`. Per model → test with 2 references before a batch.

## 6. Rhythm: what can be measured

Without gutters the rhythm is in the **images' lengths, their air and their text**. No study ties scroll distance to
reading time; practitioners' numbers (WEBTOON creators: ≥ 200 px between beats, 600–1000 px at scene changes on an
800 px canvas = 0.25 W and 0.75–1.25 W) now translate into **air inside shots**. Panel viewing time is proportional to
the amount of text (Ikuta et al., Sci. Rep. 2023). The model below is a starting point to calibrate.

Units: W = 390, a screen S = 844 (phone 390 × 844).

| Measure | Formula | Starting values / warnings |
|---|---|---|
| Length of a shot | `h / S` screens | viewport = 1 (a stop) · 1:4 = 1.8 · 1:7 = 3.2 |
| Scroll time | `1.2 s × h / S` (empty scrolling) | assumption to calibrate |
| Reading time | `0.8 s + words / 4 + 0.3 s × elements` per text cluster | ~240 wpm; elements = figures, items, SFX |
| Air | Σ air zones in px | ≥ 0.25 W between two beats, 0.75–1.25 W at a scene change (inside the shots or as a transition shot) |
| Density | words per screen | > 35 words or > 3 boxes per screen = "wall" |
| Stop vs. move | sequence of formats | ≥ 4 vertical shots without a viewport = no rest; ≥ 3 viewports in a row = flat |
| One beat per screen | beat changes per screen | > 2 = crowded |
| Seams | text box / figure on a segment seam | error for generation (warning in the draft) |
| Loop | items / text in a loop360 | a pickup only findable by dragging needs a hint (FX "hint.nudge") |
| VN | per message `0.8 s + words / 4` + a click | > 6 messages without a choice / plate change = "talking heads" |
| Episode | screens, minutes, shots | measure the existing chapters as the reference |

**How the tool shows it:** a **reading-time rail** next to the stacked column (each shot a bar at its height, labelled
with seconds; air hatched; dense stretches in the accent colour); a **screen ruler** (a faint band every 844); a
summary line per chapter (shots, screens, minutes, longest wall, longest air, loops); warnings in the Problems-panel
style once it is a creator feature. Calibration needs time per slice from readers (owner decision: privacy).

## 7. Workflow vs. tool

| Step | Who / where | Gate |
|---|---|---|
| 1. Pick the scene | owner, script page (`scriptOutline` scene numbers) | – |
| 2. Scene → storyboard JSON | **agent** (Claude Code + the SCUTOONS MCP: `read_entity` script, `read_characters`, `read_map_places`) with the workflow's rules (format / projection / move choice, air, VN for dialogue with choices) | – |
| 3. Validate + schematic page + rhythm rail | **script** `scripts/storyboard/` (Node, deterministic) | **G1 rhythm** (free) |
| 4. Compile → draft chapter (sandbox or the target chapter) | script + MCP `upload_asset` + `save_draft` | **G2 reading on the phone** (Reader dock: stack, loops, VN, items) |
| 5. Prompts, generation plans, reference lock | script + agent (FLORA assets) | **G3 references** |
| 6. Generate per scene (segments / panoramas / transforms by code) | agent in FLORA + code for stitching / polar | **G4 stills + seams** |
| 7. Stills → slices | a still replaces its schematic slice 1:1 (same shot id), or the artist paints over (Figma) | publish as always |

- **Agent workflow now:** steps 2–6; the JSON is the review surface (diffable).
- **Creator later, only if used:** (a) the rhythm rail + screen ruler in the Reader dock / chapter editor (useful for
  every chapter, also hand-made ones); (b) a creator-only `storyboard:<toon>:<scene>` entity (like `script`), merging
  the `treatment:` idea; (c) "Storyboard → draft" as an MCP action for the owner's local Claude (owner 2026-10-09: the in-app Ask panel is too
  expensive to run and will be removed from all services; production work goes through local Claude Code + the
  SCUTOONS MCP); (d) small tools in the asset manager: stitch
  segments, wrap-check a panorama, polar transform. No separate storyboard editor.
- Art always wins (AUTOMATION.md): a still or an artist's image replaces a schematic one, never the other way round.

### Where the workflow runs: cloud sessions (owner 2026-10-09)

The workflow's **workers** (generation, the agents that run jobs) run as **cloud services**, only while there are jobs.
The owner's own work happens as far as possible **in the SCUTOONS creator** or **through Claude on the computer with the
SCUTOONS MCP** (owner 2026-10-09: "then it's optimal"), never in a cloud session. What the workers need:

- **Network:** the **worker environment only** (owner 2026-10-09: yes, only for the worker cloud instances) allows
  `api.scutoons.com` (content, the MCP server's API), `cdn.scutoons.com` (images to look at / compare) and the GPU
  endpoint's host. Dev sessions like the one that wrote this document keep the default policy (both blocked
  2026-10-09). FLORA works through its connector (no host needed).
- **The SCUTOONS MCP server in the cloud:** `tools/mcp` is a stdio server that calls the API with a creator token
  (`SCUTOONS_API`, `SCUTOONS_TOKEN`, tools/mcp/README.md). In a cloud session it is started from the checkout; the token goes into
  the environment's secrets, never into the repo. **A separate creator account for the agents** (owner 2026-10-09: yes):
  its drafts show as the agent's, its rights can be narrower, its token can be revoked alone. Publish stays off.
- **Scripts:** `scripts/storyboard/` must run in the container without extra services: Node + the pre-installed
  Chromium (rasterizing the SVG schematics to WebP works there – this document's example was checked that way).
- **No local paths:** the workflow reads and writes through the API / asset manager only; nothing on `C:\Development`.
- **Generation:** everything in the cloud (owner 2026-10-09: no own GPU): closed models through FLORA, open models on
  a **serverless GPU** running ComfyUI behind HTTPS – its host on the worker environment's allow list, its key in the
  secrets. Both only through the budget gate (§7 "Budget").
- **Only on when there are jobs – no silent costs** (owner 2026-10-09). Rules:
  - **Event-driven, not polling.** A job (a storyboard reaching "refs locked", a "generate" click) *starts* the work:
    the creator / API fires a routine (`fire_trigger`) or a webhook. No cron routine that wakes every hour to find an
    empty queue (each wake costs a session's tokens).
  - **GPU = serverless, scale to zero.** Billed per second while a job runs, zero replicas when idle; an idle timeout
    of a few minutes; never a "pod" that runs until someone stops it. Accept the cold start (model load, ~1–3 min) as
    the price.
  - **The hidden standing costs, named:** model weights on a persistent volume (storage per GB-month – keep one
    volume, delete unused models), stored generations at the provider (download, then delete there), a minimum
    replica > 0 (forbidden), reserved / always-on endpoints (forbidden), egress. The asset manager (bunny.net) is the
    only place results stay.
  - **Cost before firing, a cap per run:** every batch states its cost first (generate-shots.md: "7 shots ≈ $0.03,
    approve?"); the gate is a person. Provider-side spending limits set to a monthly maximum; a run stops at its cap.
  - **Cloud sessions end with the job.** A session that waits for a person's gate is idle (no tokens while nobody
    writes); the next step starts when the gate is answered, not by a session watching in a loop.
  - **One cost line per scene** in the storyboard (generations, GPU seconds, tokens if known), so the owner sees what
    a scene cost.
- **Parallel sessions:** one cloud session per scene / lane (next section); the shot status in the storyboard is the
  shared state, the entity versioning refuses stale saves. Sessions never share a checkout, so nothing goes through
  files in the repo – everything through the API.
- **Persistence:** a cloud container is ephemeral. Storyboards live as entities (or at least drafts) in the database,
  not as files in the container; repo changes (workflow docs, scripts) are committed and pushed to a branch.

### Budget (owner 2026-10-09: "set in the admin, very simple: just how much money. Then a hard cut with a message")

The budget is how the owner steers the business (grants, investment plans): the teams work within it.

- **One setting in the admin:** `generation_budget` = an amount of money (the existing admin settings page,
  `admin/src/routes/settings`, `settings` table). Nothing else to configure. Next to it, read-only: spent, left.
- **Every paid job goes through the API** – never an agent calling a paid service on its own. The API holds the
  provider keys (GPU host, FLORA if its API allows a server key) and is the only door; that is what makes the cut
  *hard*. (A FLORA connector inside an agent session bypasses the gate → for paid generation the worker uses the
  API's job endpoint; the connector only for free reads / i2t judging. To check: can FLORA be called server-side?)
- **A ledger:** one row per job: who (account / agent), toon, scene / shot id, provider, model, estimated cost,
  actual cost, time. Before a job: `spent + reserved + estimate ≤ budget`, else refused. After it: the actual cost
  from the provider replaces the reservation.
- **Hard cut with a message:** over the budget the API answers `402 budget_exhausted { spent, budget }`; the creator /
  admin shows one notice ("Budget used up: 412 € of 400 € – ask the owner to raise it"), the worker stops and reports
  the same. A job already running finishes (it is paid either way); no new one starts. Raising the amount in the admin
  opens the gate again at once.
- **Costs outside the gate** (stated, not booked): the Claude subscription / tokens of the agent sessions, storage of
  model weights, bunny.net. Shown as a note under the budget, not part of the cut.

### Parallel work: several people and agents (owner 2026-10-09)

The storyboard is also the **work breakdown**: one shot = one unit of work with a stable id, so people and agents can
work side by side without touching the same thing.

- **Status per shot** (in the storyboard, the only new field for this): `draft → layout ok (G1) → refs locked (G3) →
  generating → still ok (G4) → in chapter`, plus `who` (a person, an agent, `gpu`). The schematic page and the
  rail colour shots by status: the scene's progress at a glance.
- **Unit of ownership = the scene's storyboard.** One person or agent writes a storyboard at a time; the entity
  versioning already refuses a stale save (`save_draft` with a base version → 409, DATA.md). Different scenes run in
  parallel.
- **Lanes that don't collide:**
  | Lane | Works on | Touches |
  |---|---|---|
  | writer / owner | script, storyboard decisions, gates | `script:`, the storyboard |
  | storyboard agent | scene → storyboard JSON, rhythm fixes | the storyboard (one scene) |
  | generation worker (serverless GPU / FLORA) | shots in "refs locked" | assets under the shot id |
  | artist(s) | paint over / replace stills, characters, moods | assets (artist wins, AUTOMATION.md) |
  | assembly agent | stills → chapter draft, text boxes, events | the chapter draft |
  | translator | text boxes' other locales | the chapter's locale maps |
- **Queues that already exist:** the Wanted list (asset requests) for missing characters / moods / plates; the
  creator's review flow (submit for review → publish) for the chapter. The storyboard adds only the shot status.
- **Agents** follow the repo's rule (HANDOVER.md): one after another or at most two lanes with separate files /
  entities, each saving only its own; a gate is always a person.

## 8. Existing tools: what to take, what is missing

| Tool | Take | Missing for SCUTOONS |
|---|---|---|
| Storyboarder (Wonder Unit; dormant since 2022, license unclear) | the open board JSON (`uid`, `shot`, `dialogue` / `action` / `notes`, `duration`, layered images), Fountain import, the 3D pose layer as a guide | film frames, linear, no scroll |
| StudioBinder | shot vocabulary (size, angle, movement) + arrow notation | – |
| Boords / Krock / Previs Pro | time per frame (animatic), comments on a frame | time in ms, never scroll length |
| Clip Studio Paint (webtoon canvas) | one long canvas, exported in chunks; on-screen-area phone preview; 3D pose models | one column, no loops, no metrics |
| Katalist / LTX Studio | script → shots, an entity registry (`@Element`) | closed, no JSON export, linear |
| Dashtoon / LlamaGen / Anifusion / Lore Machine | character libraries, LoRAs | linear pages; consistency claims are marketing |
| 360° skybox / panorama generators | seamless panoramas for loop360 / polar | no characters, no references (plates only) |

**What no tool has:** stacked shots with projections and scroll moves as the grammar, tall aspects beyond 1:4 as a
planned generation, 360° loops inside a scroll, a VN stage with persistent sprites, choices and items in the board,
and a time model tied to scroll length. That justifies owning the schema; it stays small by reusing the chapter's types.

## 9. Phases + effort

| Phase | What | Effort (agent days) | Needs |
|---|---|---|---|
| 0 | this document + `storyboard-example.html` | done | – |
| 1 | `workflows/scenes/storyboard.md` (rules: formats, projections, moves, air) + `scripts/storyboard/` (types, validate, schematic page + rhythm rail). Fixture: catpope de, Scene 1 | 2–3 | owner answers 1–5 |
| 2 | compiler → draft chapter (schematic WebPs, dialogs, items, VN events, loop groups), viewed in the Reader dock | 1–2 | MCP `upload_asset` / `save_draft` (exist) |
| 1a | rhythm measure as a script over existing chapters (chapter 1 + 2): the rail + summary, as a report | 1 | read access to the chapters (+ images for the "Ist" storyboard) |
| 3 | prompts + reference lock + FLORA for one scene; code for segment stitching, wrap check, polar transform | 3–4 + generation cost | style block, the cast's base views |
| 3a | **budget**: the admin setting, the cost ledger, the gate in the API, the message in creator / admin (§7 "Budget") – before any paid job runs through the workflow | 2–3 | – |
| 3b | serverless GPU: ComfyUI endpoint (scale to zero), 2–3 candidate models tested on the same shot list, a LoRA for catpope (regular), the worker session | 3–5 | a GPU host account, the budget (3a) |
| 4 | creator: rhythm rail in the Reader dock | 2–3 | phases 1–2 used on ≥ 2 scenes |
| 5 | creator: `storyboard:` entity, MCP action (not Ask – removed, owner 2026-10-09), asset-manager tools (stitch / wrap / polar) | 5–8 | phase 4 |
| later | LoRA for the main cast, hard pose control, calibration with reader data | open | owner decisions |

**Where it belongs:** AUTOMATION.md's pipeline, between "scenes" and "frames / slices" – it is the "Scene → shot list →
stills" action with the webtoon's own layout added. PLAN.md (the chapter-2 / platform plan) has no section for it.
Suggested: a new BACKLOG item **10.59** ("Storyboard: script scene → stacked shots → schematic draft chapter +
prompts"), a side line after 10.28 (MCP), and a one-line pointer in AUTOMATION.md "Pieces". TREATMENTS.md merges in,
if the owner agrees.

## 10. Open questions for the owner

1. **The formats:** viewport · vertical · loop360 (+ the one-way sideways strip) – complete? Is "horizontal loop 360"
   the Swiper with `loop` + `freeMode` (drag, wraps), or should the pinned strip learn to loop?
2. **Projections:** perspective · top-down · isometric · polar – more (fisheye, Dutch tilt as its own, split-screen)?
3. **1:7 as the upper limit**, or longer shots split into several shots by the storyboard?
4. **Pauses:** always air inside the shots / transition shots made by code – never a real spacing field in the
   chapter? (recommended: yes, no new field)
5. **Storyboard = treatment?** One document, with the treatment as its prompt export?
6. **Where a storyboard lives in phases 1–2:** files in the repo, a sandbox draft, or straight as a draft of the target
   chapter? (With cloud sessions: not files in a container – a sandbox draft or the `storyboard:` entity early.) One storyboard per script scene (recommended) or per chapter?
7. **The style block:** who writes it, per toon or per season?
8. **A turnaround view** (incl. from above) for the main cast, generator-only?
9. **Generators:** reference pack in FLORA first, LoRA later. A budget per scene? Tools outside FLORA (panorama /
   skybox, Scenario, ComfyUI) acceptable?
10. **Calibration:** may the reader measure time per slice (anonymous) for the rhythm constants?
11. **Licences:** commercial use of the outputs is required? (FLUX dev's licence must be checked; Apache-2.0 models
    like Qwen-Image are safe.)
12. **Chapter 1 + 2 as the reference:** can this environment get read access (allow `api.scutoons.com` +
    `cdn.scutoons.com`, or a screenshot / image export), so the "Ist" storyboard and the rhythm report cover both chapters with their images?
13. **People:** who else works on a scene (artists, translators), and should the shot status show names?
14. ~~Cloud~~ – answered 2026-10-09: an agent account yes; `api.` + `cdn.` allowed for the worker instances only;
    no own GPU, everything cloud; the budget = one amount in the admin, hard cut with a message.
15. **Budget details:** one pot that is topped up (the amount = what may be spent in total), or an amount per month
    that renews? In euros (providers bill in dollars – converted at the day's rate when booked)? One budget for
    everything, or later one per team / toon?

## Research notes (2026-10-09, sources)

- Storyboarder: github.com/wonderunit/storyboarder (last commit 2022-06-30; `.storyboarder` = JSON + PNGs).
- Camera notation: studiobinder.com/blog/storyboard-camera-movement, /types-of-camera-shots.
- WEBTOON CANVAS: images ≤ 800 × 1280, ≤ 2 MB per file (uploads are cut into such files – the reader still sees one
  stack); Creator's Resource Handbook, webtoons-static.pstatic.net/creator101. Spacing: tips.clip-studio.com/en-us/articles/4143.
- Reading time: Ikuta, Wöhler, Aizawa, Sci. Rep. 13 (2023), doi 10.1038/s41598-023-47120-w. 240 wpm: Brysbaert 2019
  (not re-checked).
- Generators: Nano Banana Pro (up to 14 references), OpenAI images/edits (up to 16 inputs), FLUX.1 Kontext
  (arXiv 2506.15742), Midjourney `--oref`, Runway Gen-4 references, Scenario sketch-to-render, Leonardo image guidance,
  Krea sketch, Anifusion / ComicsMaker LoRA, Lore Machine v3. FLORA's catalog via the connector 2026-10-09: Nano Banana
  Pro / 2, GPT Image 1–2.5, FLUX 2, Kontext Max, Seedream 5, Ideogram 3–4.5, Qwen Image, Recraft V4, Krea 2; LoRA:
  Flux 2 + Krea 2 Turbo; no ControlNet / pose image models. Aspect limits per model: not verified one by one (to do
  in phase 3).
