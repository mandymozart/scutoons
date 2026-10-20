# A room composed from what exists (owner 2026-10-20: "create a scene like picture 9 with the stuff you have"): the
# library hall (library-scene.blend, 14 vault tiles) with its central tile replaced by a round ceiling – a circular
# ornament of the vector library extruded under it (default ring-dial.svg), a round window with a cross and a light in
# the middle – then exported like any .blend (export_blend.py).
# Read-only on the .blend (nothing is saved). Headless:
#
#   blender.exe -b "F:/2024_Roar/FOOTAGE/MODELS/Library/library-scene.blend" \
#     --python C:/Development/scutoons/tools/rooms/blender/compose_rotunda.py -- "<export dir>" [triangle budget] [ornament.svg]
#
# The rotunda sits on the tile named CENTER (Blender units = the file's; the room's scale comes in build.mjs).
import math
import os
import sys

import bmesh
import bpy

CENTER = 'Plane.003'  # the middle tile of the 3 × 5 grid (x −4.33, y 0.95)
RADIUS = 1.55  # the ornament / ceiling inside the gap the tile leaves (tiles are 4 wide, 3.5 apart)
CEIL = 1.42  # the round ceiling – just above the vaults' crown (1.36)
OCULUS = 0.36  # the round window's radius
ORNAMENT = (sys.argv[sys.argv.index('--') + 3:] or [os.path.join(os.path.dirname(os.path.abspath(__file__)), '../../../media/catpope/vectors/ring-dial.svg')])[0]

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


# the round ceiling (owner 2026-10-20: "it uses an extruded version of one of the circular vectors in our ornament
# library"): a ceiling disc over the gap just above the vaults' crown, the ornament (ring-dial) extruded as relief beneath
# it – the pillars of the neighbouring tiles flare into it
bm = bmesh.new()
bmesh.ops.create_circle(bm, cap_ends=True, segments=64, radius=RADIUS + 0.35)
bmesh.ops.translate(bm, vec=(cx, cy, CEIL), verts=bm.verts)
bmesh.ops.reverse_faces(bm, faces=bm.faces[:])  # it faces down
project_uv(add('rotunda_ceiling', bm))
before = set(bpy.data.objects)
bpy.ops.import_curve.svg(filepath=ORNAMENT)
for o in set(bpy.data.objects) - before:
    if o.type != 'CURVE':
        continue
    o.data.extrude = 0.04
    o.data.fill_mode = 'BOTH'
    # the SVG comes in at its pixel size in metres / 1000-ish: fit it to the gap, centred, just under the ceiling
    bpy.context.view_layer.update()
    w = max(o.dimensions.x, o.dimensions.y) or 1
    o.scale = [v * (2 * RADIUS / w) for v in o.scale]
    bpy.context.view_layer.update()
    bb = [o.matrix_world @ __import__('mathutils').Vector(c) for c in o.bound_box]
    mid = sum(bb, __import__('mathutils').Vector()) / 8
    o.location.x += cx - mid.x
    o.location.y += cy - mid.y
    o.location.z = CEIL - 0.05
    mesh = bpy.data.meshes.new_from_object(o.evaluated_get(bpy.context.evaluated_depsgraph_get()))
    obj = bpy.data.objects.new('rotunda_ornament', mesh)
    obj.matrix_world = o.matrix_world
    coll.objects.link(obj)
    obj.data.materials.clear()
    if concrete:
        obj.data.materials.append(concrete)
    project_uv(obj)
    bpy.data.objects.remove(o, do_unlink=True)

# the oculus: a ring round the opening + a cross of two bars, and a glowing disc just above (the light's face)
bm = bmesh.new()
bmesh.ops.create_cone(bm, cap_ends=False, segments=48, radius1=OCULUS + 0.05, radius2=OCULUS + 0.05, depth=0.12)
bmesh.ops.translate(bm, vec=(cx, cy, CEIL - 0.1), verts=bm.verts)
project_uv(add('rotunda_oculus_ring', bm))
for i, (sx, sy) in enumerate([(OCULUS * 2, 0.05), (0.05, OCULUS * 2)]):
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1)
    bmesh.ops.scale(bm, vec=(sx, sy, 0.05), verts=bm.verts)
    bmesh.ops.translate(bm, vec=(cx, cy, CEIL - 0.1), verts=bm.verts)
    project_uv(add(f'rotunda_oculus_bar_{i}', bm))
glow = bpy.data.materials.new('rotunda_glow')
glow.diffuse_color = (1, 1, 1, 1)
bm = bmesh.new()
bmesh.ops.create_circle(bm, cap_ends=True, segments=48, radius=OCULUS + 0.04)
bmesh.ops.translate(bm, vec=(cx, cy, CEIL - 0.02), verts=bm.verts)
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
light.location = (cx, cy, CEIL - 0.25)
coll.objects.link(light)

# then the usual export
here = os.path.dirname(os.path.abspath(__file__))
exec(compile(open(os.path.join(here, 'export_blend.py'), encoding='utf8').read(), os.path.join(here, 'export_blend.py'), 'exec'))
