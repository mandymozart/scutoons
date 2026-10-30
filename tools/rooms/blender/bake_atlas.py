# A scan room re-baked onto one atlas per quality level (agents/research/LOCATION-SCENES.md §7e, issue #74): an Unreal
# export (tools/rooms/unreal/export_level.py: level.glb + tex/ + manifest.json + lights.json) → per level a decimated
# mesh with its own UV atlas, Cycles-baked: the scans' colour × Unreal's lights (real shadows, a little bounce) + AO.
# tools/rooms/pipeline/build.mjs then reads it like any export (manifest `baked`: no vertex-light bake, no simplify).
# Headless, never saves a .blend:
#
#   "/c/Program Files/Blender Foundation/Blender 5.1/blender.exe" -b --factory-startup \
#     --python C:/Development/scutoons/tools/rooms/blender/bake_atlas.py -- rooms/<room>.json "<export dir>" "<out dir>" \
#     [--only <level>] [--samples N] [--cpu]
#
# 1. the export without room.json's `drop`, the scans' textures back on (manifest), everything joined into one mesh
# 2. the view cut (build.mjs's: room.json `view` + `cut`) – no atlas space for what the reader never sees
# 3. per level (room.json `levels.<l>.tris`, `.atlas` px, `.atlases` 1–n): a welded copy, Decimate (collapse) to the
#    triangle target, split into `atlases` parts (whole scans, by surface), Smart UV Project + pack per part (texel
#    density by area), then three selected-to-active bakes from the full mesh: DIFFUSE colour, DIFFUSE light (direct +
#    indirect, Unreal's lights in build.mjs's units), AO; texels whose ray missed are filled from their neighbours
# 4. the atlas = colour × (1 − e^−(ambient · AO + exposure · light)) – build.mjs's tone curve, so the mood stays
# → <out>/level.<level>.glb (atlas inside) + atlas.<level>[.<part>].png + manifest.json (`baked`) + lights.json
# room.json `bake` (all optional): samples (light, 128), ao { distance m, samples }, bounces, extrusion / ray (m, the
# selected-to-active cage of a decimated level; the full detail uses 0.01 / 0.05), margin (px), angle (Smart UV's
# limit, 66°), plainShare (the untextured faces' part of the atlas, 0.12 – one colour × light needs few texels).
# Not supported here: `glow`, `ground`, `scale` (none of the scan rooms has them).
import json
import math
import os
import shutil
import sys
import time

import bmesh
import bpy
import numpy as np
from mathutils import Vector

args = sys.argv[sys.argv.index('--') + 1:]
ROOM_FILE, SRC, OUT = args[0], args[1], args[2]
opt = lambda k, d=None: args[args.index(k) + 1] if k in args else d  # noqa: E731
room = json.load(open(ROOM_FILE, encoding='utf-8'))
cfg = {'samples': 128, 'ao': {'distance': 1.5, 'samples': 64}, 'bounces': 2, 'extrusion': 0.08, 'ray': 0.3,
       'margin': 16, 'angle': 66, 'plainShare': 0.12,
       **room.get('bake', {})}
SAMPLES = int(opt('--samples', cfg['samples']))
LEVELS = [opt('--only')] if opt('--only') else [k for k in ('high', 'medium', 'low') if k in room['levels']]
os.makedirs(OUT, exist_ok=True)
t0 = time.time()
log = lambda *a: print(f'[bake_atlas {time.time() - t0:6.1f}s]', *a, flush=True)  # noqa: E731

bpy.ops.wm.read_factory_settings(use_empty=True)
sc = bpy.context.scene
vl = bpy.context.view_layer
sc.tool_settings.use_uv_select_sync = True  # the UV operators work on the selected faces


def activate(*objs):
    bpy.ops.object.select_all(action='DESELECT')
    for o in objs:
        o.select_set(True)
    vl.objects.active = objs[-1]


# --- Cycles, on the GPU when Blender sees one -------------------------------------------------------------------------
sc.render.engine = 'CYCLES'
device = 'CPU'
if '--cpu' not in args:
    prefs = bpy.context.preferences.addons['cycles'].preferences
    for kind in ('OPTIX', 'CUDA'):
        try:
            prefs.compute_device_type = kind
        except TypeError:
            continue
        prefs.get_devices()
        if any(d.type == kind for d in prefs.devices):
            for d in prefs.devices:
                d.use = d.type == kind
            sc.cycles.device, device = 'GPU', kind
            break
sc.cycles.max_bounces = cfg['bounces'] + 1
sc.cycles.diffuse_bounces = cfg['bounces']
sc.cycles.glossy_bounces = sc.cycles.transmission_bounces = sc.cycles.volume_bounces = 0
sc.cycles.caustics_reflective = sc.cycles.caustics_refractive = False
sc.world = bpy.data.worlds.new('dark')
sc.world.use_nodes = True
sc.world.node_tree.nodes['Background'].inputs[1].default_value = 0.0  # the ambient comes in with the AO (step 4)
sc.world.light_settings.distance = cfg['ao']['distance']
log('device', device)

# --- 1. the export, the textures back on, one mesh ----------------------------------------------------------------------
bpy.ops.import_scene.gltf(filepath=os.path.join(SRC, 'level.glb'), merge_vertices=True)
drop = tuple(room.get('drop', []))
for o in list(sc.objects):
    if o.type != 'MESH' or (drop and o.name.startswith(drop)):
        bpy.data.objects.remove(o)
manifest = json.load(open(os.path.join(SRC, 'manifest.json'), encoding='utf-8'))
for actor in manifest.get('actors', []):
    o = bpy.data.objects.get(actor['name'])
    if not o or not actor.get('meshes'):
        continue
    params = manifest['materials'].get(actor['meshes'][0]['materials'][0], [])
    base = next((p for p in params if p.startswith('BaseColorTexture=')), None)
    if not base:
        continue
    mat = bpy.data.materials.new('scan_' + o.name)
    mat.use_nodes = True
    nt = mat.node_tree
    bsdf = nt.nodes['Principled BSDF']
    tex = nt.nodes.new('ShaderNodeTexImage')
    tex.image = bpy.data.images.load(os.path.join(SRC, 'tex', base.split('.')[-1] + '.png'))
    uv = nt.nodes.new('ShaderNodeUVMap')
    uv.uv_map = 'UVMap'  # every part's first layer is renamed so below
    nt.links.new(uv.outputs['UV'], tex.inputs['Vector'])
    nt.links.new(tex.outputs['Color'], bsdf.inputs['Base Color'])
    o.data.materials.clear()
    o.data.materials.append(mat)
    log('texture', o.name, tex.image.size[:])
for m in bpy.data.materials:  # a diffuse room: no metal, no shine
    b = m.node_tree.nodes.get('Principled BSDF') if m.use_nodes else None
    if b:
        b.inputs['Metallic'].default_value = 0.0
        b.inputs['Roughness'].default_value = 1.0
meshes = [o for o in sc.objects if o.type == 'MESH']
for o in meshes:  # one UV name, so the join keeps one layer
    if o.data.uv_layers:
        o.data.uv_layers[0].name = 'UVMap'
        while len(o.data.uv_layers) > 1:
            o.data.uv_layers.remove(o.data.uv_layers[-1])
activate(*meshes)
bpy.ops.object.join()
src = vl.objects.active
src.name = 'source'
bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
bm = bmesh.new()
bm.from_mesh(src.data)
bmesh.ops.triangulate(bm, faces=bm.faces[:])

# --- 2. the view cut: build.mjs's cone, in glTF axes (Blender x, y, z = glTF x, −z, y) ---------------------------------
if room.get('view') and room.get('cut'):
    v, cut = room['view'], room['cut']
    at, f = np.array(v['at']), np.array(v['look']) - np.array(v['at'])
    f /= np.linalg.norm(f)
    z = -f
    ax = np.array([z[2], 0, -z[0]]) / math.hypot(z[2], z[0])
    ay = np.cross(z, ax)
    deg = math.pi / 180
    vfov = v.get('fov', 55) * deg
    cap = lambda a: math.tan(min(a, 89 * deg))  # noqa: E731
    tan_h = cap(math.atan(math.tan(vfov / 2) * cut.get('aspect', 2)) + (18 + cut['margin']) * deg)
    tan_v = cap(vfov / 2 + (6 + cut['margin']) * deg)
    co = np.array([vt.co[:] for vt in bm.verts])
    c = np.stack([co[:, 0], co[:, 2], -co[:, 1]], 1) - at
    vz, vx, vy = c @ f, c @ ax, c @ ay
    with np.errstate(divide='ignore', invalid='ignore'):
        seen = (vz > 0.05) & (vz <= cut.get('far', 30)) & (np.abs(vx / vz) < tan_h) & (np.abs(vy / vz) < tan_v)
    bm.verts.index_update()
    gone = [fc for fc in bm.faces if not any(seen[vt.index] for vt in fc.verts)]
    all_tris = len(bm.faces)
    bmesh.ops.delete(bm, geom=gone, context='FACES')
    log(f'cut: {len(bm.faces)} of {all_tris} triangles in view')
bm.to_mesh(src.data)
bm.free()
SRC_TRIS = len(src.data.polygons)

# --- 3. the lights: lights.json (Unreal cm, Z up) → Blender; build.mjs's units (value = intensity / d², colour / 255) ---
lights_file = json.load(open(os.path.join(SRC, 'lights.json'), encoding='utf-8'))
K = {'POINT': 4 * math.pi ** 2, 'SPOT': 4 * math.pi ** 2, 'AREA': math.pi ** 2}  # measured: Cycles' diffuse per watt at 1 m
KIND = {'PointLightComponent': 'POINT', 'SpotLightComponent': 'SPOT', 'RectLightComponent': 'AREA',
        'DirectionalLightComponent': 'SUN'}
exposure = room.get('exposure', 1)
for row in lights_file['lights']:
    if not (room.get('allLights') or row.get('visible', True)) or row['class'] not in KIND:
        continue
    kind = KIND[row['class']]
    d = bpy.data.lights.new(row['name'], kind)
    d.color = [c / 255 for c in row['color']]
    if kind == 'SUN':
        d.energy = math.pi * row['intensity'] * room.get('sun', 0.1) * exposure
    else:
        d.energy = K[kind] * row['intensity'] * exposure
    if kind in ('POINT', 'SPOT'):
        d.shadow_soft_size = 0.05
    if kind == 'SPOT':
        outer = float(row['outer_cone_angle']) * math.pi / 180
        inner = float(row.get('inner_cone_angle') or 0) * math.pi / 180
        d.spot_size = min(math.pi, 2 * outer)
        d.spot_blend = max(0.0, min(1.0, 1 - inner / outer))
    if kind == 'AREA':
        d.shape = 'RECTANGLE'
        d.size, d.size_y = float(row.get('source_width', 64)) / 100, float(row.get('source_height', 64)) / 100
    r = float(row.get('attenuation_radius', 100000)) / 100
    if kind != 'SUN' and r < 1000:  # Unreal's radius window (1 − (d/r)⁴)², as build.mjs: the ray length is d
        d.use_nodes = True
        nt = d.node_tree
        em = next(n for n in nt.nodes if n.type == 'EMISSION')
        path = nt.nodes.new('ShaderNodeLightPath')
        q = nt.nodes.new('ShaderNodeMath')
        q.operation, q.inputs[1].default_value = 'DIVIDE', r
        p4 = nt.nodes.new('ShaderNodeMath')
        p4.operation, p4.inputs[1].default_value = 'POWER', 4
        one = nt.nodes.new('ShaderNodeMath')
        one.operation, one.inputs[0].default_value, one.use_clamp = 'SUBTRACT', 1, True
        sq = nt.nodes.new('ShaderNodeMath')
        sq.operation, sq.inputs[1].default_value = 'POWER', 2
        nt.links.new(path.outputs['Ray Length'], q.inputs[0])
        nt.links.new(q.outputs[0], p4.inputs[0])
        nt.links.new(p4.outputs[0], one.inputs[1])
        nt.links.new(one.outputs[0], sq.inputs[0])
        nt.links.new(sq.outputs[0], em.inputs['Strength'])
    o = bpy.data.objects.new(row['name'], d)
    o.location = Vector(row['pos']) * 0.01
    o.location.y = -o.location.y
    o.rotation_mode = 'QUATERNION'
    o.rotation_quaternion = Vector((row['dir'][0], -row['dir'][1], row['dir'][2])).to_track_quat('-Z', 'Y')
    sc.collection.objects.link(o)
    log('light', row['name'], kind, round(d.energy, 2), 'W')


# --- 4. per level: decimate, atlas, bake, composite, export -------------------------------------------------------------
def pixels(img):
    a = np.empty(img.size[0] * img.size[1] * 4, np.float32)
    img.pixels.foreach_get(a)
    return a.reshape(img.size[1], img.size[0], 4)


def bake(tgt, img, kind, passes, samples, cage):
    tgt.active_material.node_tree.nodes.active.image = img
    sc.cycles.samples = samples
    t = time.time()
    activate(src, tgt)
    bpy.ops.object.bake(type=kind, pass_filter=passes, use_selected_to_active=True, cage_extrusion=cage[0],
                        max_ray_distance=cage[1], margin=cfg['margin'], margin_type='EXTEND', target='IMAGE_TEXTURES',
                        uv_layer='atlas', use_clear=True)
    px = pixels(img)
    log(f'  bake {kind} {sorted(passes)} {samples} spp: {time.time() - t:.1f}s, mean {px[..., :3].mean():.4f}')
    return px


def fill_holes(a, known, steps=8):
    """texels whose cage ray missed the full mesh (a sliver, a thin ornament's rim) come back black: each step fills
    the unknown texels next to known ones with their mean, up to `steps` texels deep"""
    a = a * known[..., None]
    w = known.astype(np.float32)
    for _ in range(steps):
        if w.all():
            break
        acc, cnt = np.zeros_like(a), np.zeros_like(w)
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                if dx or dy:
                    acc += np.roll(a, (dy, dx), (0, 1))
                    cnt += np.roll(w, (dy, dx), (0, 1))
        new = (w == 0) & (cnt > 0)
        a[new] = acc[new] / cnt[new][:, None]
        w[new] = 1
    return a


def unwrap(obj, size, tag):
    """Smart UV Project (scans and plain faces apart, so no island mixes them), texel density by area, the plain faces
    down to their `plainShare`, packed with a margin of 6 px at the final texture size"""
    me = obj.data
    me.uv_layers.active = me.uv_layers.new(name='atlas')
    activate(obj)
    bpy.ops.object.mode_set(mode='EDIT')
    bm = bmesh.from_edit_mesh(me)
    flag = bm.faces.layers.bool.get('plain')
    for group in (False, True):
        for fc in bm.faces:
            fc.select_set(fc[flag] == group)
        bmesh.update_edit_mesh(me)
        bpy.ops.uv.smart_project(angle_limit=math.radians(cfg['angle']), island_margin=0.0, area_weight=0.0,
                                 correct_aspect=True, scale_to_bounds=False)
    bpy.ops.mesh.select_all(action='SELECT')
    bpy.ops.uv.average_islands_scale()
    bm = bmesh.from_edit_mesh(me)
    flag = bm.faces.layers.bool.get('plain')
    uvl = bm.loops.layers.uv['atlas']

    def uv_area(fc):
        u = [lp[uvl].uv for lp in fc.loops]
        return abs(sum(u[i].x * u[i - 1].y - u[i - 1].x * u[i].y for i in range(len(u)))) / 2
    area = {False: 0.0, True: 0.0}
    for fc in bm.faces:
        area[fc[flag]] += uv_area(fc)
    share = cfg['plainShare']
    k = min(1.0, math.sqrt(share / (1 - share) * area[False] / area[True])) if area[True] and area[False] else 1.0
    for fc in bm.faces:
        if fc[flag]:
            for lp in fc.loops:
                lp[uvl].uv *= k
    # all into 0..1 – the pack keeps them there (ORIGINAL_AABB)
    lo = Vector((min(lp[uvl].uv.x for fc in bm.faces for lp in fc.loops), min(lp[uvl].uv.y for fc in bm.faces for lp in fc.loops)))
    span = max(max(lp[uvl].uv.x - lo.x, lp[uvl].uv.y - lo.y) for fc in bm.faces for lp in fc.loops) or 1.0
    for fc in bm.faces:
        for lp in fc.loops:
            lp[uvl].uv = (lp[uvl].uv - lo) / span
    bmesh.update_edit_mesh(me)
    bpy.ops.uv.pack_islands(udim_source='ORIGINAL_AABB', rotate=True, scale=True, margin_method='FRACTION',
                            margin=6 / min(size, 2048), shape_method='CONCAVE')
    bpy.ops.object.mode_set(mode='OBJECT')
    # a big pack can spill over several tiles (seen with 100k unwelded triangles): back into 0..1, uniformly
    uv = np.empty(len(me.loops) * 2, np.float32)
    me.uv_layers['atlas'].data.foreach_get('uv', uv)
    lo, span = uv.min(), uv.max() - uv.min()
    if lo < 0 or uv.max() > 1:
        me.uv_layers['atlas'].data.foreach_set('uv', (uv - lo) / span * (1 - 2 / size) + 1 / size)
        log(tag, f'the pack spilled to {uv.max():.2f}: scaled back into the atlas')
    log(tag, f'unwrapped: {len(me.polygons)} triangles, plain faces {area[True] / (sum(area.values()) or 1):.0%} of the '
        f'surface → x{k:.2f}')


def bake_atlas(obj, size, tag, cage):
    """colour, light and AO from the full mesh onto `obj`'s atlas (selected-to-active through `cage` = extrusion, ray
    distance) → atlas.<tag>.png, set as its base colour"""
    mat = bpy.data.materials.new('atlas_' + tag)
    mat.use_nodes = True
    node = mat.node_tree.nodes.new('ShaderNodeTexImage')
    mat.node_tree.nodes.active = node
    obj.data.materials.append(mat)
    buf = bpy.data.images.new('bake_' + tag, size, size, alpha=False, float_buffer=True)
    colour = bake(obj, buf, 'DIFFUSE', {'COLOR'}, 16, cage)[..., :3]
    light = bake(obj, buf, 'DIFFUSE', {'DIRECT', 'INDIRECT'}, SAMPLES, cage)[..., :3]
    ao = bake(obj, buf, 'AO', set(), cfg['ao']['samples'], cage)[..., :1]
    bpy.data.images.remove(buf)
    known = colour.max(-1) > 0  # every surface here has some colour; 0 = the ray found nothing
    log(tag, f'holes filled: {(~known).mean():.1%} of the atlas (gaps included)')
    colour, light, ao = np.split(fill_holes(np.concatenate([colour, light, ao], -1), known), [3, 6], -1)
    # build.mjs's tone curve over the light, × the scans' colour (linear), → sRGB bytes
    lin = colour * (1 - np.exp(-(np.array(room.get('ambient', [0, 0, 0])) * ao + light)))
    srgb = np.where(lin <= 0.0031308, lin * 12.92, 1.055 * np.power(np.clip(lin, 0, 1), 1 / 2.4) - 0.055)
    out = bpy.data.images.new('atlas_' + tag, size, size, alpha=False)
    out.pixels.foreach_set(np.concatenate([np.clip(srgb, 0, 1), np.ones_like(ao)], 2).astype(np.float32).ravel())
    out.filepath_raw = os.path.join(OUT, f'atlas.{tag}.png')
    out.file_format = 'PNG'
    out.save()
    node.image = out  # the exported material: the atlas as base colour (build.mjs makes it unlit)
    mat.node_tree.links.new(node.outputs['Color'], mat.node_tree.nodes['Principled BSDF'].inputs['Base Color'])


report = {}
for level in LEVELS:
    lc = room['levels'][level]
    size, count = int(lc.get('atlas', 4096)), int(lc.get('atlases', 1))
    t_level = time.time()
    me = src.data.copy()
    tgt = bpy.data.objects.new(room['name'], me)
    sc.collection.objects.link(tgt)
    target = int(lc.get('tris', SRC_TRIS))
    # `plain`: faces without a scan texture (the roses, one colour) – they get a fraction of the atlas (`plainShare`);
    # `part`: the source material, for splitting into `atlases`
    scan = [bool(m and m.name.startswith('scan_')) for m in me.materials]
    me.attributes.new('plain', 'BOOLEAN', 'FACE').data.foreach_set('value', [not scan[p.material_index] for p in me.polygons])
    me.attributes.new('part', 'INT', 'FACE').data.foreach_set('value', [p.material_index for p in me.polygons])
    while me.uv_layers:
        me.uv_layers.remove(me.uv_layers[0])
    me.materials.clear()
    bm = bmesh.new()  # one surface: the scans' UV seams were split vertices
    bm.from_mesh(me)
    bmesh.ops.remove_doubles(bm, verts=bm.verts[:], dist=1e-5)
    bm.to_mesh(me)
    bm.free()
    # the full detail keeps the source's surface: a tight cage (a thin ornament would catch its own back otherwise)
    cage = (cfg['extrusion'], cfg['ray']) if target < SRC_TRIS else (0.01, 0.05)
    if target < SRC_TRIS:
        dec = tgt.modifiers.new('decimate', 'DECIMATE')
        dec.decimate_type, dec.ratio, dec.use_collapse_triangulate = 'COLLAPSE', target / SRC_TRIS, True
        activate(tgt)
        bpy.ops.object.modifier_apply(modifier=dec.name)
    tris = len(me.polygons)
    log(level, f'{tris} triangles (target {target}, source {SRC_TRIS})')
    parts = [tgt]
    if count > 1:
        # whole scans into `count` atlases of about equal surface (the biggest first); the plain faces join the smallest
        part = [0] * tris
        me.attributes['part'].data.foreach_get('value', part)
        plain = [False] * tris
        me.attributes['plain'].data.foreach_get('value', plain)
        surface, plain_of = {}, {}
        for p in me.polygons:
            surface[part[p.index]] = surface.get(part[p.index], 0.0) + p.area
            plain_of[part[p.index]] = plain[p.index]
        bins, load = {}, [0.0] * count
        for k in sorted((k for k in surface if not plain_of[k]), key=lambda k: -surface[k]):
            bins[k] = load.index(min(load))
            load[bins[k]] += surface[k]
        for k in surface:
            bins.setdefault(k, load.index(min(load)))
        parts = []
        for i in range(count):
            m = me.copy()
            bm = bmesh.new()
            bm.from_mesh(m)
            layer = bm.faces.layers.int.get('part')
            bmesh.ops.delete(bm, geom=[f for f in bm.faces if bins[f[layer]] != i], context='FACES')
            bm.to_mesh(m)
            bm.free()
            o = bpy.data.objects.new(f"{room['name']}_{i + 1}", m)
            sc.collection.objects.link(o)
            parts.append(o)
        bpy.data.objects.remove(tgt)
    for o in parts:  # the bake shades the full mesh only: no part may cast onto it
        for flag in ('visible_camera', 'visible_diffuse', 'visible_glossy', 'visible_transmission', 'visible_shadow',
                     'visible_volume_scatter'):
            setattr(o, flag, False)
    for i, o in enumerate(parts):
        tag = level if count == 1 else f'{level}.{i + 1}'
        unwrap(o, size, tag)
        bake_atlas(o, size, tag, cage)
    activate(*parts)
    bpy.ops.export_scene.gltf(filepath=os.path.join(OUT, f'level.{level}.glb'), export_format='GLB', use_selection=True,
                              export_apply=True, export_yup=True, export_cameras=False, export_lights=False,
                              export_normals=True, export_tangents=False, export_attributes=False,
                              export_image_format='AUTO')
    report[level] = {'tris': tris, 'atlas': size, 'atlases': count, 'seconds': round(time.time() - t_level, 1)}
    log(level, 'done', report[level])
    for o in parts:
        bpy.data.objects.remove(o)

json.dump({'baked': True, 'from': os.path.abspath(SRC), 'device': device, 'samples': SAMPLES, 'bake': cfg,
           'sourceTris': SRC_TRIS, 'levels': report, 'actors': [], 'materials': {}},
          open(os.path.join(OUT, 'manifest.json'), 'w'), indent=1)
shutil.copy(os.path.join(SRC, 'lights.json'), os.path.join(OUT, 'lights.json'))
log('all done', report)
