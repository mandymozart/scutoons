# A place's room, step 1 (agents/research/LOCATION-SCENES.md §3, issue #74): an Unreal level → files the web pipeline
# (tools/rooms/pipeline/build.mjs) reads. Read-only – nothing in the project is saved. Runs headless, no GPU needed:
#
#   set ROOM_OUT=C:\…\out   &   set ROOM_MAP=/Game/Maps/Church   (Git Bash: also MSYS_NO_PATHCONV=1)
#   "C:\Epic\UE_5.6\Engine\Binaries\Win64\UnrealEditor-Cmd.exe" "<project>.uproject" -run=pythonscript
#       -script="<this file>" -EnablePlugins=PythonScriptPlugin,GLTFExporter -unattended -nosplash -nullrhi -nop4
#
# Writes to ROOM_OUT: level.glb (the visible static mesh actors + cameras, no material bake – that needs a renderer),
# tex/<texture>.png (each material's textures), manifest.json (actors, materials → texture parameters, cameras),
# lights.json (every light: position, direction, colour, intensity, radius, cones). Units / axes stay Unreal's in the
# json (cm, Z up); the .glb is metres, Y up (the exporter's mapping: x, z, y).
import json
import os

import unreal

OUT = os.environ['ROOM_OUT']
MAP = os.environ.get('ROOM_MAP', '/Game/Maps/Church')
os.makedirs(OUT + '/tex', exist_ok=True)
info = {'map': MAP, 'actors': [], 'materials': {}, 'cameras': []}
lights = {'lights': []}


def save():
    json.dump(info, open(OUT + '/manifest.json', 'w'), indent=1, default=str)
    json.dump(lights, open(OUT + '/lights.json', 'w'), indent=1, default=str)


def textures_of(material):
    """a material's textures: an instance's texture parameters ('Name=/Game/…'), a material's used textures"""
    if isinstance(material, unreal.MaterialInstance):
        return [str(v.parameter_info.name) + '=' + v.parameter_value.get_path_name()
                for v in material.get_editor_property('texture_parameter_values') if v.parameter_value]
    return [t.get_path_name() for t in unreal.MaterialEditingLibrary.get_used_textures(material)]


if not MAP.startswith('/Game/'):
    # Git Bash rewrites /Game/… into a Windows path – set MSYS_NO_PATHCONV=1 there
    raise SystemExit(f'ROOM_MAP must be a /Game/… path, got {MAP}')
world = unreal.EditorLoadingAndSavingUtils.load_map(MAP)
if not world:
    raise SystemExit(f'no such map: {MAP}')
selected = set()
for actor in unreal.GameplayStatics.get_all_actors_of_class(world, unreal.Actor):
    cls = actor.get_class().get_name()
    t = actor.get_actor_transform()
    row = {'name': actor.get_actor_label(), 'class': cls, 'loc': str(t.translation), 'rot': str(t.rotation.rotator()),
           'scale': str(t.scale3d), 'hidden': actor.is_hidden_ed()}
    meshes = []
    for c in actor.get_components_by_class(unreal.StaticMeshComponent):
        if not c.static_mesh:
            continue
        mats = c.get_materials()
        meshes.append({'mesh': c.static_mesh.get_path_name(), 'materials': [m.get_path_name() if m else None for m in mats]})
        for m in mats:
            if m and m.get_path_name() not in info['materials']:
                try:
                    info['materials'][m.get_path_name()] = textures_of(m)
                except Exception as e:  # noqa: BLE001 – keep going, the manifest says what failed
                    info['materials'][m.get_path_name()] = ['ERR ' + str(e)]
    if meshes:
        row['meshes'] = meshes
        if not actor.is_hidden_ed():
            selected.add(actor)
    if 'Camera' in cls:
        info['cameras'].append(row)
        selected.add(actor)
    for c in actor.get_components_by_class(unreal.LightComponent):
        lt = c.get_world_transform()
        f = c.get_forward_vector()
        col = c.get_editor_property('light_color')
        light = {'name': row['name'], 'class': c.get_class().get_name(), 'visible': c.is_visible(),
                 'pos': [lt.translation.x, lt.translation.y, lt.translation.z], 'dir': [f.x, f.y, f.z],
                 'color': [col.r, col.g, col.b], 'intensity': c.get_editor_property('intensity')}
        for k in ['attenuation_radius', 'inner_cone_angle', 'outer_cone_angle', 'intensity_units', 'source_width', 'source_height']:
            try:
                light[k] = str(c.get_editor_property(k))
            except Exception:  # noqa: BLE001 – not every light has every property
                pass
        lights['lights'].append(light)
    info['actors'].append(row)
save()

# the textures as PNG (full resolution; the web pipeline resizes per quality level)
done = set()
for params in info['materials'].values():
    for p in params:
        if p.startswith('ERR'):
            continue
        path = p.split('=')[-1]
        if path in done or path.startswith('/Engine'):
            continue
        done.add(path)
        task = unreal.AssetExportTask()
        task.object = unreal.load_asset(path)
        task.filename = OUT + '/tex/' + path.split('.')[-1] + '.png'
        task.automated = True
        task.prompt = False
        task.replace_identical = True
        info.setdefault('tex_export', {})[path] = unreal.Exporter.run_asset_export_task(task)
save()

options = unreal.GLTFExportOptions()
options.set_editor_property('bake_material_inputs', unreal.GLTFMaterialBakeMode.DISABLED)
options.set_editor_property('export_cameras', True)
options.set_editor_property('export_lights', False)
options.set_editor_property('export_vertex_colors', False)
info['gltf'] = str(unreal.GLTFExporter.export_to_gltf(world, OUT + '/level.glb', options, selected))
info['selected'] = len(selected)
save()
