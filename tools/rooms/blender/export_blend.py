# A place's room from a Blender scene, step 1 (agents/research/LOCATION-SCENES.md §3, issue #74): the .blend → the same
# files tools/rooms/pipeline/build.mjs reads from an Unreal export (level.glb + manifest.json + lights.json + tex/).
# Read-only – the .blend is never saved. Headless:
#
#   "/c/Program Files/Blender Foundation/Blender 5.1/blender.exe" -b "<scene>.blend" \
#     --python C:/Development/scutoons/tools/rooms/blender/export_blend.py -- "<export dir>" [triangle budget]
#
# - every mesh gets a Decimate (collapse) so the scene lands near the budget (default 150 000 triangles) – in memory only
# - glTF with its textures inside (+Y up, metres), its cameras; lights don't go into the .glb
# - lights.json in the Unreal export's convention (cm, Z up, left-handed: x, −y, z), so build.mjs bakes them alike;
#   `visible` = the light renders (hide_render off); a sun = 'DirectionalLightComponent'
import json
import os
import sys

import bpy

args = sys.argv[sys.argv.index('--') + 1:]
OUT = args[0]
BUDGET = int(args[1]) if len(args) > 1 else 150000
os.makedirs(OUT + '/tex', exist_ok=True)
scene = bpy.context.scene

meshes = [o for o in scene.objects if o.type == 'MESH' and not o.hide_render]
tris = {o.name: sum(len(p.vertices) - 2 for p in o.data.polygons) for o in meshes}
total = sum(tris.values()) or 1
ratio = min(1.0, BUDGET / total)
for o in meshes:
    if ratio < 1.0 and tris[o.name] > 500:
        m = o.modifiers.new('rooms_decimate', 'DECIMATE')
        m.decimate_type = 'COLLAPSE'
        m.ratio = ratio

ue = lambda v: [v[0] * 100, -v[1] * 100, v[2] * 100]  # noqa: E731 – Blender m → Unreal cm (the export's convention)
KIND = {'POINT': 'PointLightComponent', 'SPOT': 'SpotLightComponent', 'AREA': 'RectLightComponent', 'SUN': 'DirectionalLightComponent'}
lights = []
for o in scene.objects:
    if o.type != 'LIGHT':
        continue
    d = o.data
    fwd = o.matrix_world.to_3x3() @ __import__('mathutils').Vector((0, 0, -1))  # a Blender light shines down its −Z
    row = {'name': o.name, 'class': KIND.get(d.type, d.type), 'visible': not o.hide_render, 'pos': ue(o.matrix_world.translation),
           'dir': [fwd.x, -fwd.y, fwd.z], 'color': [c * 255 for c in d.color], 'intensity': d.energy,
           'attenuation_radius': str(d.cutoff_distance * 100 if getattr(d, 'use_custom_distance', False) else 100000)}
    if d.type == 'SPOT':
        row['outer_cone_angle'] = str(d.spot_size * 90 / 3.141592653589793)
        row['inner_cone_angle'] = str(d.spot_size * (1 - d.spot_blend) * 90 / 3.141592653589793)
    lights.append(row)
json.dump({'lights': lights}, open(OUT + '/lights.json', 'w'), indent=1)

bpy.ops.export_scene.gltf(filepath=OUT + '/level.glb', export_format='GLB', export_apply=True, export_cameras=True,
                          export_lights=False, export_yup=True, use_visible=False, use_renderable=True)
info = {'blend': bpy.data.filepath, 'trianglesBefore': total, 'decimate': ratio, 'actors': [], 'materials': {},
        'cameras': [o.name for o in scene.objects if o.type == 'CAMERA'], 'camera': scene.camera.name if scene.camera else None}
json.dump(info, open(OUT + '/manifest.json', 'w'), indent=1)
