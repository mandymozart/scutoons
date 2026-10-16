"""
Builds the 3D record (and its sleeve) of an LP item as binary glTF (.glb) from the print PDFs.

Owner 2026-10-16: "let the user turn the record ... a 3d asset with both sides of the record" – "We will turn all items
into 3d assets." First item: catpope `awakening_vinyl` (Cat Pope – Awakening, SCU001). ITEMS.md "3D item models".

    python tools/vinyl3d/build.py --label-a A.pdf --label-b B.pdf [--cover Cover.pdf [--front right]] --out <dir> [--name awakening_vinyl] [--tag v2]

Needs numpy, Pillow (with WebP), pymupdf. Writes <name>.<tag>.model.glb (the record) and, with --cover,
<name>.<tag>.sleeve.glb and <name>.<tag>.set.glb – the sleeve with the record inside (owner 2026-10-16: "The item should show sleeve with record inside. And
on starting to play it is pulled out and put on the record player"): node "sleeve" (as in .sleeve.glb, but open at +X)
and node "record" (as in .model.glb) standing in it, peeking PEEK out of the open edge (+X, opposite the spine). The
viewer's record player takes the "record" node out of the sleeve and onto its platter (record-stage.js) – keep the names.

The record (metres, +Y up, side A faces +Z, standing on its edge so a turn around Y shows side B – the way the tools'
model view turns it): Ø 302 mm, 2 mm thick, label Ø 100 mm (the label PDFs' trim box = the die cut), hole Ø 7.24 mm.
The vinyl face is a ring with POLAR uvs (u = angle, v = radius), so the grooves are a tiny 1D texture (a normal map:
fine groove noise, louder bands, smooth lead-in, track gaps and run-out; a roughness map makes the gaps glossier) – no
moire, ~2 KB (1024 rows); KHR_materials_anisotropy along the tangent (= around) gives the radial sheen
(viewers without it ignore it). The labels are rendered from the PDFs at 300 dpi, cropped to the trim box, LABEL_PX.

The sleeve: a spined sleeve 315 × 312 × 3 mm (specs/spinedsleeve_12inch_2019.pdf); the cover PDF's trim box is
back | spine | front (315 + 3 + 315 mm), front faces +Z, the spine on the left of the front. Which half is the front:
FRONT_IS_RIGHT_HALF (or --front left|right) – the right half, with the obi strip on its left (owner, 2026-10-16).

A test press (`--test-press --name <id>`): blank white labels in a generic, empty paper inner sleeve 310 × 310 mm –
<name>.<tag>.set.glb only (owner 2026-10-16: "generic empty inner sleeve and white label on record only"); one file serves
every test press (they look the same).

Small files (owner 2026-10-16: "up to 150 KB for the record sounds a lot for such a simple model"; the screen is never
much beyond 1080 px): SEG = 96 around, positions int16 / normals int8 (KHR_mesh_quantization, the node's scale = Q),
textures WebP (EXT_texture_webp, required – no fallback) at SLEEVE_PX / SLEEVE_BACK_PX / LABEL_PX, the groove maps PNG.
The v2 set: 252 KB (front 113, back 66, labels 21 + 14, geometry 26); a test press 32 KB (geometry 24). A changed build
= a new --tag (CDN paths never change their bytes). A next item: same pattern – artwork from the print files, real
dimensions in metres, standing so a turn shows the back, one self-contained .glb, as small (ASSETS.md "3D models").

The viewer now builds the record and sleeve in code (@scutoons/media/vinyl-model.js, the same geometry and groove maps):
an item gives only its pictures (`vinyl`, ITEMS.md "3D item models") – `--pictures` writes them,
<name>.{sleeve-front,sleeve-back,sleeve-spine,label-a,label-b}.webp. A change to the record here goes there too.
"""
import argparse
import io
import json
import math
import struct
from pathlib import Path

import numpy as np
import pymupdf
from PIL import Image

MM = 0.001
R_OUT = 151.0 * MM           # 302 mm
R_LABEL = 50.0 * MM          # label Ø 100 mm
R_HOLE = 3.62 * MM           # spindle hole Ø 7.24 mm
HALF_T = 1.0 * MM            # 2 mm thick
SEG = 96                     # segments around: a flat disc – at the stage's size a chord dips < 0.1 px (owner 2026-10-16:
                             # "150 KB for the record sounds a lot for such a simple model")
Q = 1e-5                     # positions are int16 in 0.01 mm (KHR_mesh_quantization; the node scales them back to metres)

# the vinyl ring, as radii in mm (outside → in): edge, lead-in, grooved area, run-out to the label
LEAD_IN = (150.0, 146.0)
GROOVES = (146.0, 60.0)
TRACK_GAPS = 3               # 3 tracks a side → 2 gaps (+ the lead-out spiral)

# the sleeve (specs/spinedsleeve_12inch_2019.pdf) and where the record sits in the set
SLEEVE_W, SLEEVE_H, SLEEVE_D = 315 * MM, 312 * MM, 3 * MM
PEEK = 25 * MM               # how far the record looks out of the sleeve's open edge in the set
INNER_W = 310 * MM           # a paper inner sleeve (square, a test press's only sleeve)
# the cover PDF is 315 mm | spine | 315 mm: the FRONT is the right half – the one with the obi strip on its left (owner
# 2026-10-16: "The front is the one with the obi slip on the left"). A change = new file names (CDN paths never change).
FRONT_IS_RIGHT_HALF = True
# texture sizes: the stage is ~375–600 px wide and the sleeve never full screen (owner 2026-10-16: the files too big)
# (owner 2026-10-16: "the screen never gets much beyond 1080 px wide") – WebP (EXT_texture_webp)
SLEEVE_PX = 1024             # the front (the stage shows it at most ~500 css px)
SLEEVE_BACK_PX = 768         # the back (seen when the item view turns it)
LABEL_PX = 512               # labels A / B
WEBP_Q = 80


# ---------------------------------------------------------------- artwork

def render_clip(pdf: Path, clip_mm=None, dpi=300, size=None) -> Image.Image:
    """the PDF's first page at dpi, clipped to its trim box (or to clip_mm = (x0, y0, x1, y1) inside the trim box)"""
    page = pymupdf.open(str(pdf))[0]
    trim = page.trimbox
    if clip_mm is not None:
        k = 72 / 25.4
        x0, y0, x1, y1 = clip_mm
        trim = pymupdf.Rect(trim.x0 + x0 * k, trim.y0 + y0 * k, trim.x0 + x1 * k, trim.y0 + y1 * k)
    pix = page.get_pixmap(dpi=dpi, clip=trim, colorspace=pymupdf.csRGB, alpha=False)
    img = Image.frombytes('RGB', (pix.width, pix.height), pix.samples)
    if size is not None:
        img = img.resize(size, Image.LANCZOS)
    return img


def webp(img: Image.Image, q=WEBP_Q) -> bytes:
    b = io.BytesIO()
    img.convert('RGB').save(b, 'WEBP', quality=q, method=6)
    return b.getvalue()


def png(img: Image.Image) -> bytes:
    b = io.BytesIO()
    img.save(b, 'PNG', optimize=True)
    return b.getvalue()


def groove_maps(h=1024, w=4, seed=12):
    """normal + roughness maps of the vinyl ring along the radius: v = 0 at the edge (R_OUT), v = 1 at the label.
    1024 rows = 0.1 mm a row: the groove pitch aliases into fine noise – what a screen pixel (≥ 0.3 mm) shows anyway"""
    rng = np.random.default_rng(seed)
    r = np.linspace(R_OUT / MM, R_LABEL / MM, h)          # mm per row
    dr = (R_OUT - R_LABEL) / MM / (h - 1)                    # mm per px (~0.1)
    grooved = (r <= GROOVES[0]) & (r >= GROOVES[1])
    # track gaps: evenly spread inside the grooved area (no real durations at hand), ~1.2 mm wide
    gaps = np.zeros(h, bool)
    for i in range(1, TRACK_GAPS):
        rc = GROOVES[0] - (GROOVES[0] - GROOVES[1]) * i / TRACK_GAPS
        gaps |= np.abs(r - rc) < 0.6
    # loudness: slow random envelope per track (the visible bands of a record)
    k = max(1, round(10 / dr))                              # a ~10 mm window
    env = np.convolve(rng.normal(0, 1, h), np.ones(k) / k, mode='same')
    env = 0.55 + 0.45 * (env - env.min()) / (np.ptp(env) + 1e-9)
    # grooves (~0.1 mm pitch, below a row): fine noise of the radius, times the loudness
    height = rng.normal(0, 1, h).clip(-2.5, 2.5) * env * 0.2 * dr    # mm (slope ~0.1)
    height[~grooved | gaps] = 0
    # run-out: a few wide spiral turns between the music and the label
    runout = (r < GROOVES[1]) & (r > R_LABEL / MM + 3)
    height[runout] = 0.01 * np.sin(2 * math.pi * r[runout] / 2.5) ** 8
    # the edge: a rounded bead
    edge = r > LEAD_IN[0]
    height[edge] = -0.02 * ((r[edge] - LEAD_IN[0]) / (R_OUT / MM - LEAD_IN[0])) ** 2
    # tangent space: T = around (u), B = image up = towards v = 0 = outwards → n = (0, -dh/dr, 1)
    slope = np.gradient(height, r)
    n = np.stack([np.zeros(h), -slope, np.ones(h)], 1)
    n /= np.linalg.norm(n, axis=1, keepdims=True)
    rgb = ((n * 0.5 + 0.5) * 255).round().astype(np.uint8)
    normal = Image.fromarray(np.repeat(rgb[:, None, :], w, 1), 'RGB')
    # roughness (G) – grooves scatter, gaps / lead-in / run-out are glossy; metallic (B) 0
    rough = np.where(grooved & ~gaps, 0.34 + 0.08 * env, 0.18)
    rough[edge] = 0.3
    mr = np.zeros((h, 3), np.uint8)
    mr[:, 0] = 255
    mr[:, 1] = (rough * 255).round().astype(np.uint8)
    mrough = Image.fromarray(np.repeat(mr[:, None, :], w, 1), 'RGB')
    return normal, mrough


# ---------------------------------------------------------------- geometry

def ring(r0, r1, z, facing, uv):
    """a flat ring between radii r0 < r1 at height z, facing +1 (+Z) or -1 (-Z); uv(x, y, r, a) → (u, v)"""
    pos, nor, tan, uvs, idx = [], [], [], [], []
    for j, r in enumerate((r0, r1)):
        for i in range(SEG + 1):
            a = 2 * math.pi * i / SEG
            x, y = r * math.cos(a), r * math.sin(a)
            pos.append((x, y, z))
            nor.append((0, 0, facing))
            uvs.append(uv(x, y, r, a))
            tan.append(None)
    for i in range(SEG):
        a, b, c, d = i, i + 1, SEG + 1 + i, SEG + 2 + i
        idx += [a, c, b, b, c, d] if facing > 0 else [a, b, c, b, d, c]
    return pos, nor, tan, uvs, idx


def tube(r, z0, z1, outward, rounds=0.0):
    """the side wall of a cylinder (the edge or the hole), normals outward (+1) or inward (-1)"""
    pos, nor, uvs, idx = [], [], [], []
    rows = [(z0, 0.0), (z1, 1.0)]
    for z, v in rows:
        for i in range(SEG + 1):
            a = 2 * math.pi * i / SEG
            pos.append((r * math.cos(a), r * math.sin(a), z))
            nor.append((outward * math.cos(a), outward * math.sin(a), 0))
            uvs.append((i / SEG, v))
    for i in range(SEG):
        a, b, c, d = i, i + 1, SEG + 1 + i, SEG + 2 + i
        idx += [a, b, c, b, d, c] if outward > 0 else [a, c, b, b, c, d]
    return pos, nor, None, uvs, idx


def merge(parts):
    pos, nor, uvs, idx, tan = [], [], [], [], []
    for p, n, t, u, i in parts:
        base = len(pos)
        pos += p; nor += n; uvs += u
        tan += t if t is not None else [None] * len(p)
        idx += [base + k for k in i]
    return pos, nor, (None if all(x is None for x in tan) else tan), uvs, idx


def by_material(meshes):
    """one primitive per material (the sleeve's four plain edges are one draw, not four)"""
    out = {}
    for m, p, n, t, u, i in meshes:
        out.setdefault(m, []).append((p, n, t, u, i))
    return [(m, *merge(parts)) for m, parts in out.items()]


def record_meshes():
    """→ list of (material name, positions, normals, tangents|None, uvs, indices)"""
    meshes = []

    # labels: planar uv over the label square (trim box = the Ø 100 mm circle's square); side B mirrored so it reads
    def label_uv(sign):
        return lambda x, y, r, a: (0.5 + sign * x / (2 * R_LABEL), 0.5 - y / (2 * R_LABEL))
    for side, z, f in (('label_a', HALF_T, 1), ('label_b', -HALF_T, -1)):
        p, n, t, u, i = ring(R_HOLE, R_LABEL, z, f, label_uv(f))
        meshes.append((side, p, n, None, u, i))

    # vinyl faces: polar uv (u around, v = 0 at the edge → 1 at the label); tangent = around, w so B = outwards
    def polar(x, y, r, a):
        return (a / (2 * math.pi), (R_OUT - r) / (R_OUT - R_LABEL))
    faces = []
    for z, f in ((HALF_T, 1), (-HALF_T, -1)):
        p, n, _, u, i = ring(R_LABEL, R_OUT, z, f, polar)
        # T = d(pos)/du = around (counter-clockwise); B (image up = towards v=0) = outwards; glTF B = cross(N, T) * w
        t = []
        for (x, y, _z) in p:
            r = math.hypot(x, y)
            tx, ty = -y / r, x / r
            # cross(N, T) with N = (0, 0, f): (-f*ty, f*tx, 0) = f * (-x, -y)/r... inward for f=1 → w = -f gives outward
            t.append((tx, ty, 0.0, float(-f)))
        faces.append((p, n, t, u, i))
    p, n, t, u, i = merge(faces)
    meshes.append(('vinyl', p, n, t, u, i))

    # edge + hole walls (plain vinyl, no grooves)
    p, n, t, u, i = merge([tube(R_OUT, -HALF_T, HALF_T, 1), tube(R_HOLE, -HALF_T, HALF_T, -1)])
    meshes.append(('vinyl_edge', p, n, None, u, i))
    return meshes


def box_meshes(w, h, d, open_edge=False):
    """a sleeve box centred on the origin: front (+Z), back (-Z, read from behind), spine (-X), the rest plain;
    open_edge: the +X side is the opening (no wall, plain paper inside) – the set with the record in it"""
    x0, x1, y0, y1, z0, z1 = -w / 2, w / 2, -h / 2, h / 2, -d / 2, d / 2

    def quad(name, a, b, c, dd, normal, uv=((0, 1), (1, 1), (1, 0), (0, 0))):
        return (name, [a, b, c, dd], [normal] * 4, None, list(uv), [0, 1, 2, 0, 2, 3])
    faces = [
        quad('front', (x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1), (0, 0, 1)),
        quad('back', (x1, y0, z0), (x0, y0, z0), (x0, y1, z0), (x1, y1, z0), (0, 0, -1)),
        quad('spine', (x0, y0, z0), (x0, y0, z1), (x0, y1, z1), (x0, y1, z0), (-1, 0, 0)),
        quad('edge', (x0, y1, z1), (x1, y1, z1), (x1, y1, z0), (x0, y1, z0), (0, 1, 0)),
        quad('edge', (x0, y0, z0), (x1, y0, z0), (x1, y0, z1), (x0, y0, z1), (0, -1, 0)),
    ]
    if not open_edge:
        return faces + [quad('edge', (x1, y0, z1), (x1, y0, z0), (x1, y1, z0), (x1, y1, z1), (1, 0, 0))]
    # the opening: the insides of front and back (a hair inside, facing in) – a look into it shows paper, not the void
    e = d * 0.02
    return faces + [
        quad('edge', (x1, y0, z1 - e), (x0, y0, z1 - e), (x0, y1, z1 - e), (x1, y1, z1 - e), (0, 0, -1)),
        quad('edge', (x0, y0, z0 + e), (x1, y0, z0 + e), (x1, y1, z0 + e), (x0, y1, z0 + e), (0, 0, 1)),
    ]


# ---------------------------------------------------------------- glb writer

class Glb:
    def __init__(self):
        self.bin = bytearray()
        self.g = {'asset': {'version': '2.0', 'generator': 'scutoons tools/vinyl3d'}, 'scene': 0,
                  'scenes': [{'nodes': [0]}], 'nodes': [], 'meshes': [], 'materials': [], 'textures': [],
                  'images': [], 'samplers': [], 'accessors': [], 'bufferViews': []}

    def view(self, data: bytes, target=None):
        while len(self.bin) % 4:
            self.bin.append(0)
        bv = {'buffer': 0, 'byteOffset': len(self.bin), 'byteLength': len(data)}
        if target:
            bv['target'] = target
        self.bin += data
        self.g['bufferViews'].append(bv)
        return len(self.g['bufferViews']) - 1

    def accessor(self, arr: np.ndarray, kind, comp, target, minmax=False):
        v = self.view(arr.tobytes(), target)
        acc = {'bufferView': v, 'componentType': comp, 'count': int(arr.shape[0]), 'type': kind}
        if minmax:
            acc['min'] = arr.min(0).tolist()
            acc['max'] = arr.max(0).tolist()
        self.g['accessors'].append(acc)
        return len(self.g['accessors']) - 1

    def sampler(self, wrap_s=10497, wrap_t=33071):
        s = {'magFilter': 9729, 'minFilter': 9987, 'wrapS': wrap_s, 'wrapT': wrap_t}
        if s not in self.g['samplers']:
            self.g['samplers'].append(s)
        return self.g['samplers'].index(s)

    def texture(self, data: bytes, mime, sampler):
        """a WebP image goes in through EXT_texture_webp (required – no fallback: every browser we serve decodes WebP)"""
        self.g['images'].append({'bufferView': self.view(data), 'mimeType': mime})
        src = len(self.g['images']) - 1
        if mime == 'image/webp':
            self.g['textures'].append({'sampler': sampler, 'extensions': {'EXT_texture_webp': {'source': src}}})
            self.require('EXT_texture_webp')
        else:
            self.g['textures'].append({'source': src, 'sampler': sampler})
        return len(self.g['textures']) - 1

    def require(self, ext):
        for k in ('extensionsUsed', 'extensionsRequired'):
            if ext not in self.g.setdefault(k, []):
                self.g[k].append(ext)

    def textured(self, mi):
        m = self.g['materials'][mi]
        return 'baseColorTexture' in m['pbrMetallicRoughness'] or 'normalTexture' in m

    def material(self, m):
        self.g['materials'].append(m)
        return len(self.g['materials']) - 1

    def padded(self, arr: np.ndarray, kind, comp, stride, normalized=False, minmax=False):
        """a vertex attribute whose element is padded to `stride` bytes (glTF: 4-byte aligned elements)"""
        n, c = arr.shape
        row = np.zeros((n, stride), np.uint8)
        row[:, :arr.itemsize * c] = arr.view(np.uint8).reshape(n, -1)
        v = self.view(row.tobytes(), 34962)
        self.g['bufferViews'][v]['byteStride'] = stride
        acc = {'bufferView': v, 'componentType': comp, 'count': n, 'type': kind}
        if normalized:
            acc['normalized'] = True
        if minmax:
            acc['min'] = arr.min(0).tolist()
            acc['max'] = arr.max(0).tolist()
        self.g['accessors'].append(acc)
        return len(self.g['accessors']) - 1

    def primitive(self, pos, nor, tan, uvs, idx, material, textured=True):
        """KHR_mesh_quantization: positions int16 in Q (the node's scale), normals / tangents int8, uvs uint16 – a
        third of float32; uvs only for a textured material"""
        snorm8 = lambda a: np.round(np.clip(np.asarray(a, np.float64), -1, 1) * 127).astype(np.int8)
        attrs = {
            'POSITION': self.padded(np.round(np.asarray(pos) / Q).astype(np.int16), 'VEC3', 5122, 8, minmax=True),
            'NORMAL': self.padded(snorm8(nor), 'VEC3', 5120, 4, normalized=True),
        }
        if textured:
            uv = np.round(np.clip(np.asarray(uvs, np.float64), 0, 1) * 65535).astype(np.uint16)
            attrs['TEXCOORD_0'] = self.accessor(uv, 'VEC2', 5123, 34962)
            self.g['accessors'][-1]['normalized'] = True
        if tan is not None:
            attrs['TANGENT'] = self.padded(snorm8(tan), 'VEC4', 5120, 4, normalized=True)
        ind = np.asarray(idx, np.uint32 if len(pos) > 65535 else np.uint16)
        return {'attributes': attrs, 'indices': self.accessor(ind, 'SCALAR', 5125 if ind.dtype == np.uint32 else 5123, 34963),
                'material': material}

    def node(self, name, prims, translation):
        """one mesh as its own root node; its scale turns the quantized positions back into metres"""
        self.g['meshes'].append({'name': name, 'primitives': prims})
        self.g['nodes'].append({'name': name, 'mesh': len(self.g['meshes']) - 1, 'translation': translation, 'scale': [Q, Q, Q]})
        self.require('KHR_mesh_quantization')

    def write(self, path: Path):
        self.g['scenes'] = [{'nodes': list(range(len(self.g['nodes'])))}]
        self.g['buffers'] = [{'byteLength': len(self.bin)}]
        for k in ('textures', 'images', 'samplers'):
            if not self.g[k]:
                del self.g[k]
        js = json.dumps(self.g, separators=(',', ':')).encode()
        js += b' ' * (-len(js) % 4)
        bn = bytes(self.bin) + b'\0' * (-len(self.bin) % 4)
        total = 12 + 8 + len(js) + 8 + len(bn)
        out = struct.pack('<4sII', b'glTF', 2, total) + struct.pack('<I4s', len(js), b'JSON') + js + struct.pack('<I4s', len(bn), b'BIN\0') + bn
        path.write_bytes(out)
        return len(out)


# ---------------------------------------------------------------- builds

TAG = ''                              # '.v2' → <name>.v2.set.glb (--tag)
WHITE_LABEL = [0.93, 0.92, 0.89, 1]   # a test press's blank label (plain paper)
INNER_SLEEVE = [0.95, 0.94, 0.91, 1]  # a generic, empty paper inner sleeve


def record_prims(g: Glb, label_a: Path | None, label_b: Path | None, out: Path, name: str):
    """the record's materials + primitives in g (labels from the PDFs; previews next to the output); no PDF = a blank
    white label (a test press – owner 2026-10-16: "white label on record only")"""
    clamp = g.sampler(33071, 33071)
    around = g.sampler(10497, 33071)
    labels = {}
    for key, pdf, title in (('label_a', label_a, 'Label A'), ('label_b', label_b, 'Label B')):
        if pdf is None:
            labels[key] = g.material({'name': title, 'pbrMetallicRoughness': {'baseColorFactor': WHITE_LABEL, 'metallicFactor': 0, 'roughnessFactor': 0.8}})
            continue
        img = render_clip(pdf, dpi=300, size=(LABEL_PX, LABEL_PX))
        img.save(out / f'{name}.{key}.preview.jpg', quality=80)
        t = g.texture(webp(img), 'image/webp', clamp)
        labels[key] = g.material({'name': title, 'pbrMetallicRoughness': {'baseColorTexture': {'index': t}, 'metallicFactor': 0, 'roughnessFactor': 0.75}})
    normal, mr = groove_maps()
    t_normal = g.texture(png(normal), 'image/png', around)
    t_mr = g.texture(png(mr), 'image/png', around)
    black = [0.012, 0.012, 0.014, 1]
    mats = {
        **labels,
        'vinyl': g.material({'name': 'Vinyl', 'pbrMetallicRoughness': {'baseColorFactor': black, 'metallicFactor': 1, 'roughnessFactor': 1,
                             'metallicRoughnessTexture': {'index': t_mr}}, 'normalTexture': {'index': t_normal, 'scale': 1},
                             # the grooves' radial sheen: anisotropy along the tangent (= around), no texture needed
                             'extensions': {'KHR_materials_anisotropy': {'anisotropyStrength': 0.75}}}),
        'vinyl_edge': g.material({'name': 'Vinyl edge', 'pbrMetallicRoughness': {'baseColorFactor': black, 'metallicFactor': 0, 'roughnessFactor': 0.35}}),
    }
    g.g.setdefault('extensionsUsed', []).append('KHR_materials_anisotropy')
    return [g.primitive(p, n, t, u, i, mats[m], g.textured(mats[m])) for m, p, n, t, u, i in record_meshes()]


def sleeve_prims(g: Glb, cover: Path, out: Path, name: str, front_right=FRONT_IS_RIGHT_HALF, open_edge=False):
    """the sleeve's materials + primitives in g; the cover trim box: 315 mm | spine 315–318 | 315 mm (312 high)"""
    clamp = g.sampler(33071, 33071)
    right = render_clip(cover, (318, 0, 633, 312), dpi=300, size=(SLEEVE_PX, SLEEVE_PX * 312 // 315))
    left = render_clip(cover, (0, 0, 315, 312), dpi=300, size=(SLEEVE_PX, SLEEVE_PX * 312 // 315))
    front, back = (right, left) if front_right else (left, right)
    back = back.resize((SLEEVE_BACK_PX, SLEEVE_BACK_PX * 312 // 315), Image.LANCZOS)
    spine = render_clip(cover, (315, 0, 318, 312), dpi=300, size=(16, SLEEVE_PX * 312 // 315))
    front.save(out / f'{name}.sleeve.front.preview.jpg', quality=80)
    t = {k: g.texture(webp(img), 'image/webp', clamp) for k, img in (('front', front), ('back', back), ('spine', spine))}
    mats = {k: g.material({'name': f'Sleeve {k}', 'pbrMetallicRoughness': {'baseColorTexture': {'index': v}, 'metallicFactor': 0, 'roughnessFactor': 0.7}}) for k, v in t.items()}
    mats['edge'] = g.material({'name': 'Sleeve edge', 'pbrMetallicRoughness': {'baseColorFactor': [0.85, 0.83, 0.78, 1], 'metallicFactor': 0, 'roughnessFactor': 0.9}})
    return [g.primitive(p, n, tn, u, i, mats[m], g.textured(mats[m]))
            for m, p, n, tn, u, i in by_material(box_meshes(SLEEVE_W, SLEEVE_H, SLEEVE_D, open_edge))]


def inner_sleeve_prims(g: Glb):
    """a generic, empty paper inner sleeve (a test press – owner 2026-10-16: "generic empty inner sleeve"): plain paper
    all round, open at +X, no spine – the same nodes as the printed sleeve"""
    paper = g.material({'name': 'Inner sleeve', 'pbrMetallicRoughness': {'baseColorFactor': INNER_SLEEVE, 'metallicFactor': 0, 'roughnessFactor': 0.85}})
    meshes = [('paper', *m[1:]) for m in box_meshes(INNER_W, INNER_W, SLEEVE_D, open_edge=True)]
    return [g.primitive(p, n, tn, u, i, paper, False) for _m, p, n, tn, u, i in by_material(meshes)]


def build_test_press(out: Path, name: str):
    """a test press: blank white labels, in a plain inner sleeve – <name>.set.glb (nodes sleeve, record)"""
    g = Glb()
    g.node('sleeve', inner_sleeve_prims(g), [0, INNER_W / 2, 0])
    g.node('record', record_prims(g, None, None, out, name), [INNER_W / 2 + PEEK - R_OUT, INNER_W / 2, 0])
    return g.write(out / f'{name}{TAG}.set.glb')


def build_record(label_a: Path, label_b: Path, out: Path, name: str):
    g = Glb()
    # the disc is built in XY (side A +Z); its centre at y = R_OUT, so it stands on its edge on y = 0
    g.node(name, record_prims(g, label_a, label_b, out, name), [0, R_OUT, 0])
    return g.write(out / f'{name}{TAG}.model.glb')


def build_sleeve(cover: Path, out: Path, name: str, front_right=FRONT_IS_RIGHT_HALF):
    g = Glb()
    g.node(f'{name}_sleeve', sleeve_prims(g, cover, out, name, front_right), [0, SLEEVE_H / 2, 0])
    return g.write(out / f'{name}{TAG}.sleeve.glb')


def build_set(label_a: Path, label_b: Path, cover: Path, out: Path, name: str, front_right=FRONT_IS_RIGHT_HALF):
    """the sleeve standing on y = 0 (open edge +X) with the record inside, PEEK out of the opening: nodes sleeve, record"""
    g = Glb()
    g.node('sleeve', sleeve_prims(g, cover, out, name, front_right, open_edge=True), [0, SLEEVE_H / 2, 0])
    g.node('record', record_prims(g, label_a, label_b, out, name), [SLEEVE_W / 2 + PEEK - R_OUT, SLEEVE_H / 2, 0])
    return g.write(out / f'{name}{TAG}.set.glb')


def write_pictures(label_a: Path, label_b: Path, cover: Path, out: Path, name: str, front_right=FRONT_IS_RIGHT_HALF):
    """the pictures of the item's `vinyl` look (the shared record model, @scutoons/media/vinyl-model.js – owner
    2026-10-16: "Shared 3D-Modelle"): <name>.{sleeve-front,sleeve-back,sleeve-spine,label-a,label-b}.webp, same sizes"""
    h = SLEEVE_PX * 312 // 315
    right = render_clip(cover, (318, 0, 633, 312), dpi=300, size=(SLEEVE_PX, h))
    left = render_clip(cover, (0, 0, 315, 312), dpi=300, size=(SLEEVE_PX, h))
    front, back = (right, left) if front_right else (left, right)
    pics = {
        'sleeve-front': front,
        'sleeve-back': back.resize((SLEEVE_BACK_PX, SLEEVE_BACK_PX * 312 // 315), Image.LANCZOS),
        'sleeve-spine': render_clip(cover, (315, 0, 318, 312), dpi=300, size=(16, h)),
        'label-a': render_clip(label_a, dpi=300, size=(LABEL_PX, LABEL_PX)),
        'label-b': render_clip(label_b, dpi=300, size=(LABEL_PX, LABEL_PX)),
    }
    for k, img in pics.items():
        data = webp(img)
        (out / f'{name}.{k}.webp').write_bytes(data)
        print(f'{name}.{k}.webp', len(data), 'bytes')


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--label-a', type=Path)
    ap.add_argument('--label-b', type=Path)
    ap.add_argument('--pictures', action='store_true', help="only the vinyl look's WebP pictures (the shared record model)")
    ap.add_argument('--test-press', action='store_true', help='blank white labels in a plain inner sleeve: <name>.set.glb only')
    ap.add_argument('--cover', type=Path)
    ap.add_argument('--out', type=Path, required=True)
    ap.add_argument('--name', default='awakening_vinyl')
    ap.add_argument('--tag', default='v2', help='file version: <name>.<tag>.set.glb – a changed file gets a new name (CDN paths never change their bytes)')
    ap.add_argument('--front', choices=('left', 'right'), default='right' if FRONT_IS_RIGHT_HALF else 'left',
                    help="the cover PDF's half that is the sleeve's front (default: FRONT_IS_RIGHT_HALF)")
    a = ap.parse_args()
    global TAG
    TAG = f'.{a.tag}' if a.tag else ''
    a.out.mkdir(parents=True, exist_ok=True)
    if a.pictures:
        if not (a.label_a and a.label_b and a.cover):
            ap.error('--pictures needs --label-a, --label-b and --cover')
        return write_pictures(a.label_a, a.label_b, a.cover, a.out, a.name, a.front == 'right')
    if a.test_press:
        print('test press set', build_test_press(a.out, a.name), 'bytes')
        return
    if not (a.label_a and a.label_b):
        ap.error('--label-a and --label-b are needed (or --test-press)')
    print('record', build_record(a.label_a, a.label_b, a.out, a.name), 'bytes')
    if a.cover:
        front_right = a.front == 'right'
        print('sleeve', build_sleeve(a.cover, a.out, a.name, front_right), 'bytes')
        print('set', build_set(a.label_a, a.label_b, a.cover, a.out, a.name, front_right), 'bytes')


if __name__ == '__main__':
    main()
