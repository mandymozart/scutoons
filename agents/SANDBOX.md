# The sandbox toon – real data for demos and tests (#642, #643)

**Rule:** anything a session needs for a demo, a test or "let me try this" lives in the toon `sandbox`, as real
drafts – never in the other toons, never as a special case in code. Nothing here is ever published by a worker;
the owner publishes in the editor / creator. (Created in production 2026-11-19 through the admin.)

## What is in it (all drafts, 2026-11-20)

| what | key / id | notes |
|---|---|---|
| the toon | `toon:sandbox` | de + en, source de, status draft; `sandbox/avatar.webp` |
| character | `character:sandgirl` | a girl with dune hair, starfish clip, teal dress, shovel and bucket. 18 mood portraits, neutral avatar + full (`characters/sandgirl/`) |
| character | `character:sandy` | the sandworm ("a wurm from like dune", owner 2026-11-20), friendly. 18 mood portraits, neutral avatar, full neutral + happy (`characters/sandy/`) |
| rigs | `rig:character.sandgirl` · `rig:character.sandy` | instruments `sand_song` (FM + walk), `sand_wind` (pink noise), `worm_rumble` (two low oscillators); presets `sand_song.base` / `.bright`, `sand_wind.base`, `worm_rumble.base`; moods set |
| place | `place:sandbox-dunes` | map place, toons `["sandbox"]`, hidden, x 1300 / y 400 (an arbitrary spot); image + plaster 1x/2x/3x (`sandbox/map/places/`) |
| set | `dunes` | `sandbox/map/sets/dunes.{low,medium,high}.v1.glb` (vertex colours, no textures: dune field with a flat stage, sandcastle, palm, bucket, shovel, starfish, ball, sky dome) + `dunes.still.v1.webp`. Spots: `spot_centre` `spot_left` `spot_right` `spot_back` `spot_home` `spot_castle` `spot_palm` `view` |
| story | `chapter:sandbox:1` "Dünen / Dunes" | group `dunes`, 4 slices (`s1c1-slice-2 … 5`, 1x/2x/3x); unlocks the place, Sandgirl, a choice (sets `sandbox_way`, adds to `sandbox_laps`), `divert` to label `end`, a screen shake, Sandy joins |
| scenes | `scene:sandy-dunes` · `sandy-dunes-sandgirl` · `sandy-dunes-sandy` · `sandy-dunes-stage` | all on set `dunes`. Empty / Sandgirl alone / Sandgirl + Sandy with their rigs / two open holders (`left`, `right`, any character, join) |
| shows | `show:demo-far-nodes` · `demo-new-scene` · `demo-values-and-scenes` · `demo-vote-and-question` | the four editor demos from `deck/demos/*.show.json`, ids prefixed `demo-`, toon `sandbox` |

## How the art was made
No image generator was available (Luma: no key / budget, ComfyUI not running), so everything is **drawn by code**
(flat toon style, not the 3D-render look of the other characters) – a starting point, replace it when real art exists.
The generators: `scripts/sandbox-art/` – `sandgirl.py`, `worm.py`, `scenery.py` (slices, thumbnail, avatar, still, place),
`dunes_glb.py` (the set). Python + Pillow only: `python sandgirl.py <out>`; `python scenery.py <out> <sandgirl-out>`;
`python worm.py <sandy-out> <scenery-out>`; `python dunes_glb.py <out>`. The CDN never overwrites: a new build = a new
file name (`.v2`, `-slice-6` …), then a `save_draft` that points to it.

## Not verified / gaps
- **Nothing was played on a deck or in the reader**: the set's look (colours are linear vertex colours, the sky is a dome
  inside the model), the cameras (`space.at`, `distance` are guesses), that the server finds `dunes` under
  `sandbox/map/sets/`, and the rigs' sound are untested. The rigs were saved as drafts, so the Rust core's check
  (it runs on publish) has not seen them.
- No 3D character models (`.glb`) for Sandgirl or Sandy.
- The chapter's events have no ids yet: open it once in the creator and save – it assigns them.
- The first placeholder files `sandbox/s1/c1/s1c1-slice-1.webp` and `s1c1-thumbnail.webp` are unused now (cleanup: the owner's).
- The story has no season document; the toon says `chapters: 0` until the creator counts it.

## How to work in it
- Via the SCUTOONS MCP: `list_shows` / `read_show` / `save_show_draft` (toon `sandbox`), `list_scenes` / `read_scene` /
  `save_scene_draft`, `save_draft` for `character:`, `place:`, `rig:`, `instrument:`, `preset:`, `chapter:sandbox:N`,
  `upload_asset` for media. Never use the owner's token outside the MCP.
- Name new demo things `demo-<name>` (shows) or `sandy-…` / `sandbox-…` / `sand_…` (scenes, places, instruments).
- Platform-wide kinds (`character:`, `place:`, `scene:`, `rig:` …) have no toon in their key: the sandbox ones carry
  `toons: ["sandbox"]` / the tag `sandbox`.
- Cleanup is never automatic: delete a demo only on the owner's word.

Refs #642 #643
