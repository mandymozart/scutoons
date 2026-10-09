# Workflow: script scene → storyboard (first throw)

The rules an agent (or a person) follows to turn one script scene into a storyboard: a list of **shots**, stacked. Why
and the full concept: `agents/STORYBOARD.md` (§1 formats, §2 the JSON, §6 rhythm, §9 build strategy). Tools:
`scripts/storyboard/` (`validate.mjs`, `render.mjs`, `rhythm-report.mjs`, `figma-draft.mjs`).

**Terms (owner 2026-10-09).** A **shot** is one image used for layouting the toon. A **shooting** is a photo session at
one location (the Figma library is sorted by shooting). Never mix the two.

## Where things go

- One storyboard per script scene: `storyboards/<toon>/s<season>c<chapter>/scene-<n>.json` (e.g.
  `storyboards/catpope/s1c3/scene-25.json`). JSON, no comments. Owner decision 2026-10-09 (§10 Q6): files in the repo
  for chapter 3; the creator entity comes later.
- Source: the script entity `script:<toon>:de` (local dev DB: `readEntity` in `scripts/storyboard/lib.mjs`), the scene
  heading `NN  INT./EXT. …`. German first; other locales are added by translation later.
- Check: `node scripts/storyboard/validate.mjs storyboards/catpope/s1c3` · page: `node scripts/storyboard/render.mjs
  storyboards/catpope/s1c3` → `storyboards/_out/s1c3.html` (schematic column + rhythm rail).

## The flow this serves (owner 2026-10-09)

1. Scene → storyboard (this file). 2. Match each shot against the owner's **Figma selection** on the chapter's page
(`S1C3`), the shooting pages only for gaps. 3. The first throw = the storyboard with the matched images, placed as a
**draft frame** in Figma (390 wide, a locked `storyboard` group, guides, `text`, `layout` on top). 4. Back into the
creator as the chapter's draft. 5. **Prompt only the gaps** (manual, Luma) or plan an Unreal camera / a direct shot.
So every shot says **what it needs to show**, clearly enough to pick a photo for it or to prompt it.

## The visual language of catpope (chapters 1 + 2, looked at 2026-10-09)

Photographic, collaged, graphic: real photos (the owner's shootings, PLF played by a person in the white robe), cut out
and combined with flat colour fields, white line-drawn clouds and shapes, black-and-white ink graphics, big SFX
lettering ("KAWUMS!", "QUETSCH", "ROARRR"), monochrome tints (ch2's green cathedral). Text sits in white boxes with a
monospaced typewriter face; scene headings appear as such boxes ("2 INT. SCHLAFZIMMER – NACHT"). A slice often holds
**several framed photos on a dark or coloured ground** (panels *inside* the image) – a shot may do that too: say so in
`action` and mark the ground as air. Chapter 2 ends with wide panoramas. Write shots in that language; never ask for a
"comic style" or "anime" look.

## A shot

```json
{
  "id": "sb-25-1",
  "format": "viewport", "aspect": "9:16", "projection": "perspective", "move": "hold",
  "shot": { "size": "wide", "angle": "eye" },
  "location": "village",
  "action": { "de": "Was das Bild zeigt – konkret genug, um ein Foto dafür zu wählen oder es zu prompten." },
  "air": [[0, 30]],
  "figures": [ { "cast": "plf", "framing": "full", "mood": "cold", "facing": "right", "x": 0.5, "y": 0.6, "h": 0.3 } ],
  "dialog": [ { "variant": "narration", "rect": { "x": 6, "y": 70, "width": 60 }, "text": { "de": "PLF stieg aus." } } ],
  "source": { "type": "schematic", "want": "photo" },
  "status": "draft",
  "note": "optional: why, alternatives, a question for the owner"
}
```

- `id`: `sb-<scene>-<n>`, n counts through the scene, never reused (it names the photo / prompt / still later).
- `format` / `aspect`: **viewport** `9:16` (a stop: one screen) · **vertical** `9:16`…`1:7` (a move: the scroll is
  the camera, height = duration) · **loop360** `N:1` (a panorama the reader drags; group renderer horizontal + loop) ·
  **strip** (a one-way sideways pass). Aspect = width:height for viewport / vertical (`1:4` = 390 × 1560), the
  panorama's width:height for loop360.
- **Safe formats (owner: the restriction for what we suggest and prompt):** vertical up to **1:7**, plan ≤ **1:4**;
  loop360 up to **13 screens** wide (Luma's proven maximum), plan ≤ **8**. Taller / wider: split into several shots,
  or take a source without the limit (Unreal render, a photo stitch) and say so in `source.want`.
- `projection`: perspective · top-down · isometric · polar. `move`: hold · tilt-down · tilt-up · fall · climb ·
  travel · descend · reveal · look-around · pass. `shot.size`: ecu cu mcu medium full wide ews; `shot.angle`: eye high
  low bird worm dutch.
- `light` (owner 2026-10-09: light is decided here, in the script → shot step, not later): **extremely hard light,
  never diffuse** – one key light, pure black and white, hard cast shadows (the storyboard's Sin City look, and the
  instruction for the photo / the shooting / the prompt). Fields:
  `{ "key": "top-left", "height": "low", "source": "sun", "shadow": "long", "rim": true, "accent": { "color": "red", "on": "die Beeren" }, "note": { "de": "…" } }`
  - `key`: where the light comes from, seen from the camera – `left` · `right` · `top` · `bottom` · `front` · `back`
    (backlight, silhouette) · `top-left` · `top-right` · `bottom-left` · `bottom-right`.
  - `height`: `below` (from under the face – menace) · `low` (grazing, long shadows) · `eye` · `high` · `overhead`
    (noon, eyes in shadow).
  - `source`: what makes it in the story – `sun` · `moon` · `snow` (bounce off snow) · `window` · `door` · `candle` ·
    `screen` · `neon` · `fire` · `lightning` · `flash` · `energy` (the ice ball) · `none` (a black frame).
  - `shadow`: `long` · `short` · `none`; `rim`: a white edge on the figures against black (backlight from behind).
  - `accent`: the one colour allowed in the frame and on what (sparingly – a scene has at most one).
  - `note`: what the light does dramatically, in a sentence ("Gegenlicht: PLF nur Umriss vor dem Schneesturm").
  A storyboard can set a scene default in `light` (top level); a shot only lists what differs. Light continuity: the
  key stays on the same side within a scene unless the story turns it (a door opens, the screens go dark).
- `air`: `[[from %, to %], …]` along the shot – zones with nothing to read (sky, snow, darkness, a colour field). A pure
  transition shot is `[[0, 100]]`.
- `figures`: `cast` = a ref from the storyboard's `cast`; x across, y along the shot (0…1), h = figure height as a
  fraction of the **screen**.
- `figures[].pose` (the 3D mannequin of the schematics takes it): `stand` · `walk` · `run` · `jump` · `fall` · `slip` ·
  `crouch` · `kneel` · `crawl` · `sit` · `lie` · `reach` · `point` · `throw` · `cast` (both hands forward – the energy
  ball) · `arms-up` (scared / surrender) · `hug-self` (cold) · `stamp` · `push` · `look-back` · `pray`.
  `facing`: `front` · `away` (back to the camera) · `left` · `right` · `three-quarter-left` · `three-quarter-right`.
- `dialog`: the chapter's own text boxes (`Dialog`, shared/content/src/content.ts). `variant`: `narration` (the white
  typewriter box, `font: "mono"`), `bubble` (speech; `tail` where it points), `caption` (a free line); **SFX** =
  `caption` + `font: "lettering"` ("ROARRR"). `rect` in % of the shot. The script's
  prose becomes narration – **shortened**: a box ≤ 25 words, a screen ≤ 35 words (the rhythm rule). Speech in the
  script (`PLF`, `{ROARRR}`) becomes bubbles / SFX.
- `source`: `type` = where the image comes from now (`schematic` until matched); `want` = what it should come from:
  `photo` (a shooting exists or is likely), `unreal`, `direct` (to be photographed), `generated` (prompt it),
  `art` (drawn / graphic). Later the match adds `figma` (the node id).
- `prompt`: `{ "en": "…" }` – only for shots that want `generated`: the image in English, concrete (what, where, who,
  light), without text or lettering; `prompts.mjs` builds the full prompt around it (format, framing, figures, air,
  the toon's style block).
- `items`, `events`, `audio`: the chapter's own types (SliceItem, VNEvent, audio) – only where the script asks.

## The storyboard file

```json
{
  "storyboard": 2, "toon": "catpope", "chapter": 3,
  "source": { "script": "script:catpope:de", "scene": 25, "heading": "EXT. WINTERLICHES BERGDORF - TAG" },
  "cast": [ { "ref": "plf", "character": "catpope", "persona": "human" } ],
  "locations": [ { "ref": "village", "place": "mountain-village", "time": "day" } ],
  "groups": [ { "label": "scene-25", "renderer": { "type": "vertical" }, "continues": true, "shots": [ … ] } ],
  "notes": [ "decisions for the owner" ]
}
```

- `cast`: registry ids (`cast:catpope` season 1: catpope `regular | human | cat | mountaintiger`, boy, crows).
  PLF in the robe = `catpope` / `human`; the TIGER = `catpope` / `mountaintiger` (the registry's reading – say in
  `notes` if the script means another being). A figure the registry lacks (e.g. YPLF, young PLF): `"character": null`,
  `"new": "YPLF – young PLF"` – it goes to the Wanted list later.
- `locations`: MAP.md's place ids (chapter 3: `mountain-village`, `beyond-the-crossroads`, `mountain-path`,
  `forest-clearing`, `monastery`, `library`).
- Groups: one `vertical` group per scene by default (label `scene-<n>`, `continues: true` so the scenes run on); a
  `horizontal` group with `loop: true, freeMode: true` for a loop360; a VN group (one plate shot + `events`) only where
  the script has a real dialogue with a choice.

## Rhythm rules (calibrated 2026-10-09 on chapters 1 + 2)

- A chapter ≈ **60–80 screens, 15–20 minutes** careful reading (ch1 70.6 screens / 17.5 min, ch2 81.7). The images
  carry the time (~10 s per screen of looking); text adds little. Chapter 3 has 10 scenes → **~6–8 screens per scene**
  on average; a big action scene (28, the tiger) gets more, a transition (26) less.
- **No walls:** ≤ 35 words and ≤ 3 boxes per screen. Chapter 1's walls (up to 94 words on one screen) are what we fix.
- **Air** ≥ 0.25 × W (~100 px, ~12 % of a screen) between two beats, 0.75–1.25 × W at a scene change – inside the shots.
- **Stops and moves alternate:** not more than 4 vertical shots without a viewport; not more than 3 viewports in a row.
- **One beat per screen**; an action beat (a fall, a run, an attack) = a vertical shot whose move *is* the action.
- Silence is fine when it is **planned**: a silent stretch is a run of images with a reason (the chase, the storm) –
  write the reason into `note`.
- VN: ≤ 6 messages without a choice / a plate change.

## Checklist before handing over

1. `validate.mjs` passes (warnings explained in `note`).
2. Every script beat of the scene is in a shot or consciously cut (`notes`: "cut: …").
3. Every shot has an `action` someone can pick a photo for, and a `source.want`.
4. The rail (`render.mjs`) shows no wall; the scene's screens fit its share of the chapter.
5. Open questions go into `notes` – never guess an owner decision.
