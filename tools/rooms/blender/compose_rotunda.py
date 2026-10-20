# A room composed from what exists (owner 2026-10-20: "create a scene like picture 9 with the stuff you have"): the
# library hall (library-scene.blend, 14 vault tiles) with its central tile replaced by a rotunda – a ribbed dome with a
# round window (an oculus: a ring and a cross) and a light in it – then exported like any .blend (export_blend.py).
# Read-only on the .blend (nothing is saved). Headless:
#
#   blender.exe -b "F:/2024_Roar/FOOTAGE/MODELS/Library/library-scene.blend" \
#     --python C:/Development/scutoons/tools/rooms/blender/compose_rotunda.py -- "<export dir>" [triangle budget]
#
# The rotunda sits on the tile named CENTER (Blender units = the file's; the room's scale comes in build.mjs).
import math
import os
import sys

import bmesh
import bpy

CENTER = 'Plane.003'  # the middle tile of the 3 × 5 grid (x −4.33, y 0.95)
RADIUS = 1.55  # the dome inside the gap the tile leaves (tiles are 4 wide, 3.5 apart)
SPRING = 1.0  # where the dome starts – below the vaults' crown (1.36), so it meets them
OCULUS = 0.36  # the round window's radius

scene = bpy.context.scene
tile = bpy.data.objects[CENTER]
cx, cy = tile.matrix_world.translation.x, tile.matrix_world.translation.y
concrete = tile.material_slots[0].material if tile.material_slots else None
bpy.data.objects.remove(tile, do_unlink=True)
coll = scene.collection


def add(name, bm, material=concrete):
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    obj = bpy.data.objects.new(name, mesh)
    coll.objects.link(obj)
    if material:
        obj.data.materials.append(material)
    return obj


def project_uv(obj, size=4.0):
    # a plain planar UV so the concrete texture shows (the dome is new geometry)
    me = obj.data
    if not me.uv_layers:
        me.uv_layers.new()
    uv = me.uv_layers.active.data
    for poly in me.polygons:
        for li in poly.loop_indices:
            co = me.vertices[me.loops[li].vertex_index].co
            uv[li].uv = ((co.x + co.z) / size, (co.y + co.z) / size)


# the dome: a hemisphere shell from SPRING up, open at the top (the oculus), normals inward
bm = bmesh.new()
rings, segs = 14, 48
top = math.asin(OCULUS / RADIUS)
verts = []
for r in range(rings + 1):
    a = (math.pi / 2 - top) * r / rings  # 0 = springing line … up to the oculus' edge
    row = []
    for s in range(segs):
        t = 2 * math.pi * s / segs
        row.append(bm.verts.new((cx + RADIUS * math.cos(a) * math.cos(t), cy + RADIUS * math.cos(a) * math.sin(t), SPRING + RADIUS * math.sin(a))))
    verts.append(row)
for r in range(rings):
    for s in range(segs):
        bm.faces.new((verts[r][s], verts[r][(s + 1) % segs], verts[r + 1][(s + 1) % segs], verts[r + 1][s]))
# a drum from the floor's edge up to the springing line, between the neighbours' pillars
ground = [bm.verts.new((cx + RADIUS * math.cos(2 * math.pi * s / segs), cy + RADIUS * math.sin(2 * math.pi * s / segs), SPRING - 0.25)) for s in range(segs)]
for s in range(segs):
    bm.faces.new((ground[s], ground[(s + 1) % segs], verts[0][(s + 1) % segs], verts[0][s]))
bmesh.ops.reverse_faces(bm, faces=bm.faces[:])
dome = add('rotunda_dome', bm)
project_uv(dome)

# ribs: 8 half arches along the dome's inside, from the springing line to the oculus
for i in range(8):
    t = 2 * math.pi * i / 8
    bm = bmesh.new()
    prev = None
    for k in range(17):
        a = (math.pi / 2 - top) * k / 16
        rr = RADIUS - 0.06
        c = (cx + rr * math.cos(a) * math.cos(t), cy + rr * math.cos(a) * math.sin(t), SPRING + rr * math.sin(a))
        ring = bmesh.ops.create_circle(bm, segments=6, radius=0.045)['verts']
        # orient the little profile across the rib's path
        for v in ring:
            x, y = v.co.x, v.co.y
            v.co = (c[0] - y * math.sin(t), c[1] + y * math.cos(t), c[2] + x)
        if prev:
            for j in range(6):
                bm.faces.new((prev[j], prev[(j + 1) % 6], ring[(j + 1) % 6], ring[j]))
        prev = ring
    project_uv(add(f'rotunda_rib_{i}', bm))

# the oculus: a ring round the opening + a cross of two bars, and a glowing disc just above (the light's face)
bm = bmesh.new()
bmesh.ops.create_cone(bm, cap_ends=False, segments=48, radius1=OCULUS + 0.05, radius2=OCULUS + 0.05, depth=0.12)
bmesh.ops.translate(bm, vec=(cx, cy, SPRING + math.sqrt(RADIUS ** 2 - OCULUS ** 2)), verts=bm.verts)
project_uv(add('rotunda_oculus_ring', bm))
for i, (sx, sy) in enumerate([(OCULUS * 2, 0.05), (0.05, OCULUS * 2)]):
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1)
    bmesh.ops.scale(bm, vec=(sx, sy, 0.05), verts=bm.verts)
    bmesh.ops.translate(bm, vec=(cx, cy, SPRING + math.sqrt(RADIUS ** 2 - OCULUS ** 2) + 0.02), verts=bm.verts)
    project_uv(add(f'rotunda_oculus_bar_{i}', bm))
glow = bpy.data.materials.new('rotunda_glow')
glow.diffuse_color = (1, 1, 1, 1)
bm = bmesh.new()
bmesh.ops.create_circle(bm, cap_ends=True, segments=48, radius=OCULUS + 0.04)
bmesh.ops.translate(bm, vec=(cx, cy, SPRING + math.sqrt(RADIUS ** 2 - OCULUS ** 2) + 0.12), verts=bm.verts)
bmesh.ops.reverse_faces(bm, faces=bm.faces[:])  # it faces down, into the rotunda
add('glow_oculus', bm, glow)

# the floor where the tile was
bm = bmesh.new()
bmesh.ops.create_grid(bm, x_segments=16, y_segments=16, size=RADIUS + 0.3)
bmesh.ops.translate(bm, vec=(cx, cy, 0.0), verts=bm.verts)
project_uv(add('rotunda_floor', bm))

# the light: in the oculus, shining down – the room's key light
light = bpy.data.objects.new('rotunda_light', bpy.data.lights.new('rotunda_light', 'POINT'))
light.data.energy = 6000
light.data.color = (0.85, 0.92, 1.0)
light.location = (cx, cy, SPRING + math.sqrt(RADIUS ** 2 - OCULUS ** 2) - 0.1)
coll.objects.link(light)

# then the usual export
here = os.path.dirname(os.path.abspath(__file__))
exec(compile(open(os.path.join(here, 'export_blend.py'), encoding='utf8').read(), os.path.join(here, 'export_blend.py'), 'exec'))
