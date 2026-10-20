Status 2026-10-20: EXPLORATORY (owner: rooms end up as looped video backgrounds (fallback) or built externally in Blender and dropped in – no in-house room pipeline as the goal; no deadline, not in the "Top Toy workshop" milestone); concept + a working one-room prototype (the cathedral from the owner's Unreal level, dev lab `/account/lab/room`, tools/rooms – §7); the owner OKed the look (2026-10-20); nothing reader-facing built; three more rooms 2026-10-20 (§7b: the station, the garden hut and the library as splats); next: the owner's look at them (issue #74). Not on chapter 3's critical path (#64).

# LOCATION-SCENES — a 3D room for every place on the map

Owner 2026-10-20: "i want to create each scene as a 3d scene draft … combine blender and unreal scene i already got and
create something that works with low resources … for each location on the map, a 3d scene, a static room, like home.
we can enter and pick items and objects from, and some characters can live in there. basically a 3d scene background
if we enter locations."

**In one line:** a place's room is a **home scene with a 3D model**. Homes already have rooms (research/CAVE.md, ITEMS.md
"Homes and furniture"), and CAVE.md's "Images first, 3D second" already plans a `.glb` per home scene. This doc
turns that second iteration into "every place can have a room", and the avatar's home is one of those rooms. It is not a
second system.

## 1. For readers
- **Entering:** a place's card on the map offers "Eintreten" / "Enter" when the place has a room and the reader's avatar
  is there. That is the same button and the same rule as the home's: walk there for free, ride for cucumbers (CAVE D2,
  D8). The story can open a room too: a VN / slice event `enterRoom { place }` (proposed), e.g. at the end of a chapter
  ("you are in the library now"). The VR headset stays the home-only shortcut.
- **The view:** a **static diorama, not a walk-around game**. One camera per room looks in, like the home scenes'
  strict central perspective (DECISIONS 2026-10-14). The reader can look around a little: the phone tilt pivots the
  view (±18° / ±4°, like the record stage), pinch zooms toward the room's back, and a tap on a hotspot moves the
  camera to it. Free walking would need a nav mesh, collision and a joystick, which is several times the work, the
  data and the GPU cost, and isn't what "a static room, like home" asks for.
- **Picking things up:** placed pickups work like placed items on a slice (ITEMS.md "Placed items"). An item stands
  at a spot in the room, a tap gives it with the same rules: respawn, the collection limit, "can't take it" shakes, and
  the server's once-per-ref grant (ref `room:<place>#<placement id>`). The item shows its own 3D `model` if it has one
  (ITEMS.md "3D item models"), otherwise its icon as a little standing card.
- **Objects:** a tap on part of the room itself (a named node in the `.glb`: the altar, a book shelf) can run events,
  like a placed item with `action: events`, e.g. "show text box", unlock a place, play a sample.
- **Characters who live there:** a character whose `home` is the place stands in its room, with no new data (the
  field exists, CAVE D1). It shows the persona's `.glb` if there is one (ITEMS / ASSETS "3D models", posed with
  figures.js's joint names, research/CHARACTERS3D.md), else its A-pose as a standing card (what HomeView draws today).
  A tap on a resident can open its character card or run events. The reader's own avatar stands in the room like at
  home.
- **Fallback:** no WebGL, reduced motion, the `none` tier or a failed load show the room's **still** (the home scene's
  1080p / 4K image, which a room keeps anyway – rendered from the same 3D scene), with the hotspots as tap areas in %.
  That is exactly today's 2.5D HomeView, so a room works on every device from day one.

## 2. Reuse, not a second system
| Need | Exists | What changes |
|---|---|---|
| A room's look | home scene (catalog `home_scenes`: name, 1080p / 4K still, video loop; creator / admin form `home-scene-form.svelte`) | + `model` (the `.glb`, per quality level), + `spots` (§4) |
| A place has a room | – | `MapPlace.room?: <home scene id>` (one field, the map builder sets it) |
| Residents | `character.home` (a place) | nothing; the room shows them |
| The full-screen view | `HomeView` / `home-room.svelte` (@scutoons/reader, live mode reuses it) | `home-room` gets a 3D renderer next to the 2.5D one; the panel below stays |
| Furniture | `inventory.home_x / home_y` in % of the room | the same numbers mapped onto the model's floor (the `floor` node's rectangle): no migration |
| 3D loading | `model-view.js` (three.js 0.186 loaded on demand), `warmModel` | + meshopt / KTX2 decoders (§3), one shared renderer per view |
| Pickups | `SliceItem` + `giveStoryItem` + respawn | the same placement shape with a spot instead of a `rect` |

A home is a room with the reader's furniture in it. A place's room is the same view without furniture (read-only, like
a friend's visit), plus its pickups and residents.

## 3. Pipeline: Blender + Unreal → web
**Measured on the owner's disk (2026-10-20):**
- Unreal project `Documents/Roar Little Kitty - Video/Roar/Roar` (UE 5.6, `C:/Epic/UE_5.6`; Content 6.3 GB): maps
  `AltErlaa` (ch1 apartment blocks), `Church`, `Crypta` (ch4 crypt), `Downtown`, `Garden` (ch5), `Zentralfriedhof`
  (ch4 graveyard), `Mercury`, `Mercury_Full`, **`Mercury_Mountain_Path`** (ch3 mountain-path, 62 MB `.umap`: a
  landscape, outdoors). Models: `cabin` 111 MB, `kirchenruine` 176 MB, `mount-station` 58 MB, `station` 650 MB,
  `gravestones` 23 MB, `alley-way`, `brutalist-building`, `hongkong`, … Plugins: Postshot (Gaussian splats), Water,
  Niagara. Movie Render Queue sequences per place (`S_Crypta_*`, `S_Zentralfriedhof*`, `S_Garden`).
- **Gaussian splats** (Postshot) in `Content/Splats`: **`library-window-wall`** (ch3 library, 285 MB `.psht`),
  `zentralfriedhof_crypta` (720 MB), `alt-erlaa-church` (721 MB), `zentralfriedhof_chapel_wagner` (384 MB; its cropped
  `.ply`: **528,608 splats, SH degree 3, 125 MB**), `garden-hut`. Photogrammetry scans (Stephansdom, Altstadt): 16–103 MB
  textures each.
- Blender 5.1 is installed (glTF exporter built in). `Documents/Top Toy/…/hongkong.glb` (a Sketchfab city block,
  exported from Blender) was the test file for the web pipeline (gltf-transform 4.2.1):

| hongkong.glb | Size | Triangles | Draw calls | Textures | GPU memory (textures) |
|---|---|---|---|---|---|
| as exported | 94.4 MB | 369,465 | 489 | 50 (73.6 MB PNG / JPEG, up to 2400²) | ~390 MB |
| meshopt + join (`optimize`, textures kept) | 77.2 MB | 336,289 | 55 | 52 | ~390 MB |
| + textures ≤ 1024, WebP q80 | 8.8 MB | 336,289 | 55 | 52 (5.1 MB) | ~226 MB (RGBA) |
| + textures ≤ 512, WebP | 5.3 MB | 336,289 | 55 | 52 (1.6 MB) | ~61 MB |
| + simplify (ratio 0.25, error 0.1 %) | 4.5 MB | 245,611 | 55 | 52 | ~61 MB |

What this shows: **textures are 80 % of the download and nearly all of the GPU memory**. Geometry compresses well (meshopt),
and joining meshes takes 489 draw calls down to 55. But 50 separate textures stay too many for a phone even when small,
and WebP becomes full RGBA in the GPU (226 MB at 1024² – an Android phone with 4 GB kills the tab). So a room needs **few
textures and GPU-compressed ones (KTX2)**, i.e. a bake.

**The pipeline (proposal):**
1. **Compose in Unreal or Blender**, whichever has the place. From Unreal: `tools/rooms/unreal/export_level.py` (the
   glTF Exporter headless, without material baking – that needs a renderer; textures and lights come as files, §7).
   Landscapes: only the visible patch, as a mesh.
2. **Blender = the bake station:** cut to what the camera sees (a diorama has no backside), decimate to the budget,
   **one UV atlas + Cycles bake** of colour × light × AO into 1–2 textures ("baked lighting": the moody Unreal look
   becomes texture; at runtime a single unlit or lightly-lit material). Name the empties for spots (§4). Export glTF
   (+Y up, metres, front = +Z, like the items).
3. **gltf-transform** (a script in `tools/rooms/`, like `tools/vinyl3d`): weld, join, meshopt, textures → **KTX2**
   (ETC1S for colour / bake, UASTC for a normal map if any), per quality level (`low` 1024² atlas, `medium` 2048²,
   `high` 2048² + detail), plus the still renders (1080p / 4K) from the same camera. KTX2 needs `toktx` (KTX-Software)
   on the build machine – not installed yet.
4. **Upload** as assets (`.glb` only, ASSETS.md "3D models") to `scutoons/rooms/<scene>.<level>.v<n>.glb`. New file =
   new name (immutable CDN paths), the home scene row points at it.
- **meshopt over Draco:** three.js reads both. The meshopt decoder is ~20 KB of wasm (Draco's is ~300 KB), decodes faster,
  and gltf-transform makes it in one step. **KTX2** needs three's KTX2Loader + the Basis transcoder (~250 KB wasm,
  loaded once, from our CDN).
- **LODs:** a room is seen from one side and from close, so a room needs no LODs. The quality levels (one file per level,
  RULES) do the job LODs would. Props far away just get fewer triangles in the bake.
- **Splats** (the library window wall, the crypt, the chapel): beautiful and already made. The cropped chapel is 528k
  splats / 125 MB as `.ply`; compressed (`.spz`, SH degree 0–1) it would be roughly 8–15 MB (estimated, not measured).
  Rendering needs a per-frame depth sort that mid-range phones manage at roughly ≤ 300k splats. **Recommendation: meshes
  first; splats as an optional "backdrop" layer behind a mesh room later** (e.g. the library's window wall as the far
  wall), tried in a second prototype. A three.js splat renderer (e.g. Spark) would load only for such a room.

## 4. Budgets per room (mid-range Android, e.g. a Galaxy A5x – proposals, checked on a device in the prototype)
| | low | medium | high |
|---|---|---|---|
| Download (`.glb`, gzip doesn't help – already compressed) | ≤ 1.5 MB | ≤ 3 MB | ≤ 6 MB |
| Triangles | ≤ 50k | ≤ 100k | ≤ 150k |
| Draw calls | ≤ 30 | ≤ 50 | ≤ 50 |
| Textures | 1–2 × 1024² KTX2 | 2–4 × 2048² KTX2 | + detail |
| GPU memory (textures) | ≤ 8 MB | ≤ 24 MB | ≤ 48 MB |
| Frame rate / pixel ratio | 30 fps / DPR ≤ 1.5 | 30 fps / DPR ≤ 2 | 60 fps desktop |

Plus per pickup / resident: an item `.glb` ≤ 150 KB (ITEMS.md), a character ≤ 1 MB. On a 4G phone (20 Mbit/s) medium
loads in ~1.5 s; the still shows at once and the 3D fades in over it. **Lazy per place:** nothing 3D loads with the
map. A place card that has a room warms its file in idle time (`warmModel`, LOADING.md's prefetch rules), and only the
room the reader enters is loaded. `none` tier / save-data: the still only.

## 5. Data – as small as possible
- `MapPlace.room?: string`: the home scene id of the place's room (map builder: a select "Room"). None = the place
  can't be entered (as now).
- Home scene + `model?: { src, variants? }`: the `.glb` per quality level (`QualityVariants`, like `image`), CDN root.
- Home scene + `spots?: RoomSpot[]`: what a screen edits and code reads.
  `{ id, node?, at?: [x, y, z], items?: { item, qty?, respawn?, action? }[], events?: VNEvent[] }`. A spot is either
  a named node in the `.glb` (`node`: the Blender empty "spot_altar", so its position lives in the model) or a point
  picked in the editor (`at`, metres). Items and events reuse the placed-item shapes; respawn and the once-per-ref
  grant are the same.
- The camera: from the `.glb` (a camera node named `view`; its limits are the defaults above), so no field. A room
  that wants different limits gets a field then, not now.
- Residents: none (character `home`). Furniture: none (existing `home_x / home_y` onto the `floor` node).
- Reader state: picked spots in the game state like slice pickups (`picked:room:<place>#<spot>`), synced with progress.
- Validation (TS + PHP): `room` is a known home scene, `model` paths are `.glb`, every `node` exists in the model (the
  creator checks it when the model loads – a warning, since the server can't read the `.glb`), `items` are catalog items.

## 6. The editor side
- **Creator / admin home scene form** (today: name, still 1080p / 4K, video): + a model slot per level (`FileSlot
  kind="model"`) with the live `ModelPreview`, + a **Spots** list. Tap a spot row to see it in the preview, tap the
  model to set a spot's point (a raycast onto the surface), "Choose the item …" opens the existing item drawer, events
  use the VN event editor. One screen, everything in place.
- **Map builder:** the place inspector gets "Room" (a select of home scenes, "None", "+ New room" → the home scene
  form). The place card preview shows "Eintreten".
- **Story:** the event `enterRoom { place }` in the VN / slice event editors (a place picker, as for `unlockLocation`).
- The storyboard studio's 3D viewport is a separate tool (shots); later it could load a room `.glb` to frame
  storyboard shots in the real place.

## 7. Prototype (built 2026-10-20 – owner: "go try")
**The room:** the Roar project's `/Game/Maps/Church` → the map's place `cathedral` (ch2): an interior composed in Unreal
from 4 Stephansdom photo scans (nave, side altar, pews, outer wall) + roses, the incense burner, 9 lights. Chosen over
the church ruin / cabin because it is the "combine what exists in Unreal" case and already a room.

**Pipeline** (tools/rooms/README.md): `unreal/export_level.py` runs headless in the owner's UE 5.6 (`-nullrhi`, ~10 s,
read-only) → `level.glb` (158k triangles, 16 draw calls, no textures – the glTF exporter can't bake materials without a
renderer) + the 4 scan textures as PNG (8k–10k², 217 MB) + the actors / materials / lights as json →
`pipeline/build.mjs rooms/cathedral.json`: textures back onto the scans; **Unreal's 9 lights baked into vertex colours**
(inverse square, Unreal's radius window, spot cones, no shadows) and every material unlit, normals / tangents dropped;
the room's own eye-level `view`; `spot_<id>` nodes; **a view cut** (the triangles the view can't see while the reader looks
around go: 158k → 129k); per level simplify, WebP, quantize, meshopt. Whole build ~20 s.

| cathedral | low | medium | high |
|---|---|---|---|
| Download | 1.82 MB | **2.43 MB** | 3.78 MB |
| Triangles | 84,760 | 99,241 | 129,205 |
| Draw calls (room) | 6 | 6 | 6 |
| Texture memory | 5.6 MB | 22.4 MB | 81.6 MB |

Medium is inside §4's budget; low is 0.3 MB over (the scans' UV seams stop the simplifier – a Blender re-bake onto one
atlas would fix both); high is over on texture memory (4 × 2048² WebP = RGBA in the GPU – KTX2 needed, `toktx` not
installed yet).

**Lab** `/account/lab/room` (dev only; room-stage.js – own three.js stage with the meshopt decoder): the room, the
Awakening LP (its item `.glb`) on the altar, a cucumber (its icon as a standing card) on the floor, Cat Pope (her
persona `.glb`, 1.1 m) as a resident; drag / tilt pivots (±18° / ±6°), wheel / pinch moves in, a tap takes a pickup ("ist
jetzt in deiner Tasche") or greets the resident, a tap on the room prints the point (= how the editor would set a spot).
Portrait phones keep ≥ 60° across. Measured on this PC (dev server, local CDN): fetch 16 ms + parse 45 ms + things
60 ms; 47 fps at 1024 × 768 (with things: 101k triangles, 15 draw calls). On 4G (20 Mbit/s) medium ≈ 1 s download.

**Learned:**
- Unreal → web works headless and needs no Blender for scan-based rooms: the scans' textures already carry their light,
  the vertex bake adds Unreal's mood for free at runtime. Blender stays the step for modelled (non-scan) rooms: one
  atlas, a real light bake (shadows), better simplification.
- The level's own camera is a cinematic top-down shot – a room needs its reader view set by hand (`view` in room.json;
  the lab's tap gives the points).
- Scan rooms have holes where the scanner didn't look; the view cut + a fixed camera hide most of them – a free walk
  would show them all (another reason for the diorama).
- Gaps: not measured on a phone yet (none attached – the owner's Android over USB + `chrome://inspect` gives fps / memory);
  no still fallback rendered yet; KTX2; the incense smoke (Niagara) is not exported.

### 7b. Three more rooms (2026-10-20 – owner: "try to build the garden hut … also the train station")
| room | source | kind | medium | |
|---|---|---|---|---|
| `station` (ch2 `station`) | Roar `/Game/Maps/Mercury`, the part around the Mount Royal station scan (`ROOM_CENTER` / `ROOM_RADIUS`) + its 9 lights | mesh | **0.73 MB**, 7.5k tris, 2 draw calls | Unreal shows it at 200× – `scale` 0.35 = its real ~60 m; the scan has no forecourt: a plain baked `ground`; a forecourt view, not an interior; close up the 4.4k-triangle scan shows its coarseness |
| `garden-hut` (ch5 `garden`) | InnerScope 5.6 `/Game/XV3dGS/SCENE_8_-_GARDEN_HUT_-_Gate(1)` | splat | **9.6 MB**, 300k of 1.49M splats (low 150k 4.8 MB, high 600k 19.2 MB) | the Roar project's Postshot original is on drive E: (not connected); the XV3dGS copy = 4 float data textures (pos, quat, scale, colour + opacity – no view-dependent SH) → EXR → `.splat`; already level; reads well at eye height (a garden path, posts, the hedge) |
| `library` (ch3 `library` / `monastery`) | InnerScope 5.6 `/Game/XV3dGS/SCENE_6_-_MONASTARY_-_LIBRARY_-_Terminals_(1)` | splat | 9.6 MB, 300k of 978k | a scanned **window wall**, not a room; no upright in the data (placed in no level) – `level: "pca"` turns it ~18°, still bent; a halo of big background splats dropped (> 25 cm). Owner 2026-10-20: "the library is in a blender file actually … on my external in the ROAR project folder footage perhapse" – not on C: / D: (searched every .blend); the external E: (`E:4_Roar\FOOTAGE\…`) wasn't connected → the mesh library waits for it |

- **Splats render in the lab without a library** (room-splat.js: data textures + one instanced quad a splat, a worker's
  counting sort when the view turns). A splat library (Spark …) would bring its own three.js, and the viewer's npm lock
  is shared – for the real thing pick one then. Pickups and residents stand in a splat like in a mesh room (depth-tested).
- **Splat costs:** 32 B a splat as `.splat` – 9.6 MB for 300k is ~4× a mesh room's budget (§4); `.spz` (quantized +
  gzip) is roughly 3–4× smaller (estimate). The sort costs CPU per view change (300k ≈ tens of ms, in a worker).
  → Mesh rooms stay the default; a splat is the look for a scanned place, at 150k (4.8 MB) on phones.
- Not measured: fps of the splat rooms (the browser pane was hidden – its frames are throttled); a phone.

## 8. Owner questions (with recommendations)
1. ~~The cathedral's look~~ – OK (2026-10-20). **Next room:** a ch3 interior (the monastery or the library) – the owner
   names the Unreal map or Blender file; then the same pipeline.
2. **Static diorama view (tilt, zoom, tap-to-focus) or free walking?** Rec.: diorama. "Static room, like home" fits
   it; walking costs a lot of time and data.
3. **Splats:** keep them for later (a backdrop behind mesh rooms), or is a scanned place (library wall, crypt) the
   look you want first? Rec.: later; meshes first.
4. **Every place, or the ones that matter?** Rec.: a room is optional per place (`room`). Start with the places
   a chapter sends readers into; the rest keep their map card.
5. **Who bakes?** Rec.: the bake runs as a Blender script on your machine (the GPU) – you compose and pick the camera,
   the script does cut / bake / compress / upload. Or an artist delivers to the §4 budget.
6. **The home's 3D:** does the avatar's home (cave, tatami room) become 3D the same way, so the home and the places
   share one renderer? Rec.: yes, once the first place room works (CAVE's second iteration).
7. **Chapter 3:** none of this is needed for 2026-11-15. Should the prototype wait until after the release, or run
   now on the side? Rec.: after the art freeze (2026-11-10), unless you want a ch3 room as a launch extra.
