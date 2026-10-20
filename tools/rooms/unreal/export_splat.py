# A splat room, step 1 (agents/research/LOCATION-SCENES.md §3, issue #74): an XV3dGS Gaussian splat (the plugin's data
# textures pos / quat / scale / sh_0 – float, 1 texel a splat) → EXR files tools/rooms/pipeline/splat.mjs reads.
# Read-only, headless (UE 5.6, the project with the XV3dGS plugin – e.g. Unreal Projects/InnerScope 5.6):
#
#   MSYS_NO_PATHCONV=1 ROOM_OUT="<export dir>" GS_BASE="/Game/XV3dGS/SCENE_8_-_GARDEN_HUT_-_Gate(1)/" \
#     "/c/Epic/UE_5.6/Engine/Binaries/Win64/UnrealEditor-Cmd.exe" "<project>.uproject" -run=pythonscript \
#     -script="C:/Development/scutoons/tools/rooms/unreal/export_splat.py" -EnablePlugins=PythonScriptPlugin \
#     -unattended -nosplash -nullrhi -nop4
#
# Postshot splats (`.psht`, the Roar project) export with Postshot itself (postshot-cli export … → .ply) – not here.
import json
import os

import unreal

OUT = os.environ['ROOM_OUT']
BASE = os.environ['GS_BASE']
if not BASE.startswith('/Game/'):
    raise SystemExit(f'GS_BASE must be a /Game/… folder, got {BASE} (Git Bash: MSYS_NO_PATHCONV=1)')
os.makedirs(OUT, exist_ok=True)
res = {}
for name in ['pos', 'quat', 'scale', 'sh_0']:
    texture = unreal.load_asset(BASE + name)
    if not texture:
        raise SystemExit(f'no {BASE}{name}')
    task = unreal.AssetExportTask()
    task.object = texture
    task.filename = f'{OUT}/{name}.exr'
    task.automated = True
    task.prompt = False
    task.replace_identical = True
    res[name] = {'ok': unreal.Exporter.run_asset_export_task(task), 'size': [texture.blueprint_get_size_x(), texture.blueprint_get_size_y()]}
json.dump(res, open(OUT + '/export.json', 'w'), indent=1)
