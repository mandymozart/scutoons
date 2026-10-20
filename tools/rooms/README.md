# tools/rooms – a place's 3D room for the web

The prototype pipeline of agents/research/LOCATION-SCENES.md (issue #74): an Unreal level becomes one small `.glb` per
media quality, with the level's light baked in – or a Gaussian splat becomes a `.splat` per quality. Rooms (`rooms/`):
`cathedral` (Roar `/Game/Maps/Church`), `station` (Roar `/Game/Maps/Mercury` around the station scan), `garden-hut` and
`library` (splats, InnerScope 5.6 `/Game/XV3dGS/…`); the dev lab `/account/lab/room?room=<id>` shows them.

1. **Export from Unreal** (headless, read-only, ~10 s; UE 5.6 in `C:\Epic\UE_5.6`):
   ```bash
   MSYS_NO_PATHCONV=1 ROOM_OUT="<export dir>" ROOM_MAP=/Game/Maps/Church \
     "/c/Epic/UE_5.6/Engine/Binaries/Win64/UnrealEditor-Cmd.exe" "<project>.uproject" -run=pythonscript \
     -script="C:/Development/scutoons/tools/rooms/unreal/export_level.py" \
     -EnablePlugins=PythonScriptPlugin,GLTFExporter -unattended -nosplash -nullrhi -nop4
   ```
   → `level.glb`, `tex/*.png`, `manifest.json`, `lights.json` (unreal/export_level.py's header). A part of a big map:
   `ROOM_CENTER=x,y,z ROOM_RADIUS=cm` (Unreal cm). A splat (XV3dGS): `unreal/export_splat.py` with `GS_BASE=/Game/XV3dGS/<scene>/`
   → `pos / quat / scale / sh_0.exr`.
2. **Build** (`pipeline/` is its own npm project, not a workspace: `npm install` there once):
   `node pipeline/build.mjs rooms/<room>.json --source "<export dir>"` → `media/scutoons/rooms/<room>.<level>.glb` (the
   local CDN copy) + `<room>.report.json` (MB, triangles, draw calls, texture memory per level).
   Splats: `node pipeline/splat.mjs rooms/<room>.json --source "<export dir>"` → `<room>.<level>.splat` +
   `<room>.splat.json` (its view and spots).
3. **Look:** `npm run dev`, then `/account/lab/room` (dev only).

`rooms/<room>.json`: `view { at, look, fov }` (the reader's camera, metres, glTF axes – the lab's tap prints a point),
`spots [{ id, at }]` (empty nodes `spot_<id>` for pickups / residents), `cut { margin, far, aspect }` (drops what the
view never sees), `ambient` + `exposure` (the light bake), `levels { low | medium | high: { ratio, error, texture } }`,
`drop` (node name prefixes to leave out), `scale` (the level at real size – the bake stays in Unreal's), `ground { y, center,
size, color }` (a plain floor where the scan has none). Splat rooms: `splat { minOpacity, maxScale, radius, center?,
level?: "pca", levels { low | medium | high: splat count } }` (`level: "pca"` turns a scan without an upright level).
