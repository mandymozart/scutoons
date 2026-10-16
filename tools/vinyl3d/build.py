"""
Builds the 3D record (and its sleeve) of an LP item as binary glTF (.glb) from the print PDFs.

Owner 2026-10-16: "let the user turn the record ... a 3d asset with both sides of the record" – "We will turn all items
into 3d assets." First item: catpope `awakening_vinyl` (Cat Pope – Awakening, SCU001). ITEMS.md "3D item models".

    python tools/vinyl3d/build.py --label-a A.pdf --label-b B.pdf [--cover Cover.pdf [--front right]] --out <dir> [--name awakening_vinyl]

Needs numpy, Pillow, pymupdf. Writes <name>.model.glb (the record) and, with --cover, <name>.sleeve.glb and
<name>.set.glb – the sleeve with the record inside (owner 2026-10-16: "The item should show sleeve with record inside. And
on starting to play it is pulled out and put on the record player"): node "sleeve" (as in .sleeve.glb, but open at +X)
and node "record" (as in .model.glb) standing in it, peeking PEEK out of the open edge (+X, opposite the spine). The
viewer's record player takes the "record" node out of the sleeve and onto its platter (record-stage.js) – keep the names.

The record (metres, +Y up, side A faces +Z, standing on its edge so a turn around Y shows side B – the way the tools'
model view turns it): Ø 302 mm, 2 mm thick, label Ø 100 mm (the label PDFs' trim box = the die cut), hole Ø 7.24 mm.
The vinyl face is a ring with POLAR uvs (u = angle, v = radius), so the grooves are a tiny 1D texture (a normal map:
fine groove noise, louder bands, smooth lead-in, track gaps and run-out; a roughness map makes the gaps glossier) – no
moire, a few KB; KHR_materials_anisotropy along the tangent (= around) gives the radial sheen
(viewers without it ignore it). The labels are rendered from the PDFs at 300 dpi, cropped to the trim box, 1024 px JPEG.

The sleeve: a spined sleeve 315 × 312 × 3 mm (specs/spinedsleeve_12inch_2019.pdf); the cover PDF's trim box is
back | spine | front (315 + 3 + 315 mm), front faces +Z, the spine on the left of the front. Which half is the front:
FRONT_IS_RIGHT_HALF (or --front left|right) – today the right half (the title side), the owner hasn't confirmed it.

Textures are JPEG / PNG (core glTF, no extension needed). A next item: same pattern – artwork from the print files,
real dimensions in metres, standing so a turn shows the back, one self-contained .glb ≤ ~1–2 MB (ASSETS.md "3D models").
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
SEG = 256                    # segments around

# the vinyl ring, as radii in mm (outside → in): edge, lead-in, grooved area, run-out to the label
LEAD_IN = (150.0, 146.0)
GROOVES = (146.0, 60.0)
TRACK_GAPS = 3               # 3 tracks a side → 2 gaps (+ the lead-out spiral)

# the sleeve (specs/spinedsleeve_12inch_2019.pdf) and where the record sits in the set
SLEEVE_W, SLEEVE_H, SLEEVE_D = 315 * MM, 312 * MM, 3 * MM
PEEK = 25 * MM               # how far the record looks out of the sleeve's open edge in the set
# the cover PDF is 315 mm | spine | 315 mm: which half is the sleeve's FRONT is not confirmed by the owner – today the
# right half (the title side). One flag: flip it here or pass --front left.
FRONT_IS_RIGHT_HALF = True


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


def jpeg(img: Image.Image, q=85) -> bytes:
    b = io.BytesIO()
    img.convert('RGB').save(b, 'JPEG', quality=q, optimize=True, progressive=False)
    return b.getvalue()


def png(img: Image.Image) -> bytes:
    b = io.BytesIO()
    img.save(b, 'PNG', optimize=True)
    return b.getvalue()


def groove_maps(h=4096, w=4, seed=12):
    """normal + roughness maps of the vinyl ring along the radius: v = 0 at the edge (R_OUT), v = 1 at the label"""
    rng = np.random.default_rng(seed)
    r = np.linspace(R_OUT / MM, R_LABEL / MM, h)          # mm per row
    dr = (R_OUT - R_LABEL) / MM / (h - 1)                    # mm per px (~0.025)
    grooved = (r <= GROOVES[0]) & (r >= GROOVES[1])
    # track gaps: evenly spread inside the grooved area (no real durations at hand), ~1.2 mm wide
    gaps = np.zeros(h, bool)
    for i in range(1, TRACK_GAPS):
        rc = GROOVES[0] - (GROOVES[0] - GROOVES[1]) * i / TRACK_GAPS
        gaps |= np.abs(r - rc) < 0.6
    # loudness: slow random envelope per track (the visible bands of a record)
    env = np.convolve(rng.normal(0, 1, h), np.ones(400) / 400, mode='same')
    env = 0.55 + 0.45 * (env - env.min()) / (np.ptp(env) + 1e-9)
    # grooves ~0.1 mm pitch: a sine of the radius with a jittered phase, times the loudness
    phase = np.cumsum(2 * math.pi * dr / (0.1 + 0.02 * rng.normal(0, 1, h).clip(-2, 2)))
    height = np.sin(phase) * env * 0.006                    # mm
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
    return pos, nor, tan, uvs, idx


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
        self.g['images'].append({'bufferView': self.view(data), 'mimeType': mime})
        self.g['textures'].append({'source': len(self.g['images']) - 1, 'sampler': sampler})
        return len(self.g['textures']) - 1

    def material(self, m):
        self.g['materials'].append(m)
        return len(self.g['materials']) - 1

    def primitive(self, pos, nor, tan, uvs, idx, material):
        attrs = {
            'POSITION': self.accessor(np.asarray(pos, np.float32), 'VEC3', 5126, 34962, True),
            'NORMAL': self.accessor(np.asarray(nor, np.float32), 'VEC3', 5126, 34962),
            'TEXCOORD_0': self.accessor(np.asarray(uvs, np.float32), 'VEC2', 5126, 34962),
        }
        if tan is not None:
            attrs['TANGENT'] = self.accessor(np.asarray(tan, np.float32), 'VEC4', 5126, 34962)
        ind = np.asarray(idx, np.uint32 if len(pos) > 65535 else np.uint16)
        return {'attributes': attrs, 'indices': self.accessor(ind, 'SCALAR', 5125 if ind.dtype == np.uint32 else 5123, 34963),
                'material': material}

    def node(self, name, prims, translation):
        """one mesh as its own root node"""
        self.g['meshes'].append({'name': name, 'primitives': prims})
        self.g['nodes'].append({'name': name, 'mesh': len(self.g['meshes']) - 1, 'translation': translation})

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

def record_prims(g: Glb, label_a: Path, label_b: Path, out: Path, name: str):
    """the record's materials + primitives in g (labels from the PDFs; previews next to the output)"""
    clamp = g.sampler(33071, 33071)
    around = g.sampler(10497, 33071)
    tex = {}
    for key, pdf in (('label_a', label_a), ('label_b', label_b)):
        img = render_clip(pdf, dpi=300, size=(1024, 1024))
        img.save(out / f'{name}.{key}.preview.jpg', quality=80)
        tex[key] = g.texture(jpeg(img, 86), 'image/jpeg', clamp)
    normal, mr = groove_maps()
    t_normal = g.texture(png(normal), 'image/png', around)
    t_mr = g.texture(png(mr), 'image/png', around)
    black = [0.012, 0.012, 0.014, 1]
    mats = {
        'label_a': g.material({'name': 'Label A', 'pbrMetallicRoughness': {'baseColorTexture': {'index': tex['label_a']}, 'metallicFactor': 0, 'roughnessFactor': 0.75}}),
        'label_b': g.material({'name': 'Label B', 'pbrMetallicRoughness': {'baseColorTexture': {'index': tex['label_b']}, 'metallicFactor': 0, 'roughnessFactor': 0.75}}),
        'vinyl': g.material({'name': 'Vinyl', 'pbrMetallicRoughness': {'baseColorFactor': black, 'metallicFactor': 1, 'roughnessFactor': 1,
                             'metallicRoughnessTexture': {'index': t_mr}}, 'normalTexture': {'index': t_normal, 'scale': 1},
                             # the grooves' radial sheen: anisotropy along the tangent (= around), no texture needed
                             'extensions': {'KHR_materials_anisotropy': {'anisotropyStrength': 0.75}}}),
        'vinyl_edge': g.material({'name': 'Vinyl edge', 'pbrMetallicRoughness': {'baseColorFactor': black, 'metallicFactor': 0, 'roughnessFactor': 0.35}}),
    }
    g.g['extensionsUsed'] = ['KHR_materials_anisotropy']
    return [g.primitive(p, n, t, u, i, mats[m]) for m, p, n, t, u, i in record_meshes()]


def sleeve_prims(g: Glb, cover: Path, out: Path, name: str, front_right=FRONT_IS_RIGHT_HALF, open_edge=False):
    """the sleeve's materials + primitives in g; the cover trim box: 315 mm | spine 315–318 | 315 mm (312 high)"""
    clamp = g.sampler(33071, 33071)
    right = render_clip(cover, (318, 0, 633, 312), dpi=300, size=(2048, 2028))
    left = render_clip(cover, (0, 0, 315, 312), dpi=300, size=(2048, 2028))
    front, back = (right, left) if front_right else (left, right)
    spine = render_clip(cover, (315, 0, 318, 312), dpi=300, size=(32, 2028))
    front.save(out / f'{name}.sleeve.front.preview.jpg', quality=80)
    t = {k: g.texture(jpeg(img, 82), 'image/jpeg', clamp) for k, img in (('front', front), ('back', back), ('spine', spine))}
    mats = {k: g.material({'name': f'Sleeve {k}', 'pbrMetallicRoughness': {'baseColorTexture': {'index': v}, 'metallicFactor': 0, 'roughnessFactor': 0.7}}) for k, v in t.items()}
    mats['edge'] = g.material({'name': 'Sleeve edge', 'pbrMetallicRoughness': {'baseColorFactor': [0.85, 0.83, 0.78, 1], 'metallicFactor': 0, 'roughnessFactor': 0.9}})
    return [g.primitive(p, n, tn, u, i, mats[m]) for m, p, n, tn, u, i in box_meshes(SLEEVE_W, SLEEVE_H, SLEEVE_D, open_edge)]


def build_record(label_a: Path, label_b: Path, out: Path, name: str):
    g = Glb()
    # the disc is built in XY (side A +Z); its centre at y = R_OUT, so it stands on its edge on y = 0
    g.node(name, record_prims(g, label_a, label_b, out, name), [0, R_OUT, 0])
    return g.write(out / f'{name}.model.glb')


def build_sleeve(cover: Path, out: Path, name: str, front_right=FRONT_IS_RIGHT_HALF):
    g = Glb()
    g.node(f'{name}_sleeve', sleeve_prims(g, cover, out, name, front_right), [0, SLEEVE_H / 2, 0])
    return g.write(out / f'{name}.sleeve.glb')


def build_set(label_a: Path, label_b: Path, cover: Path, out: Path, name: str, front_right=FRONT_IS_RIGHT_HALF):
    """the sleeve standing on y = 0 (open edge +X) with the record inside, PEEK out of the opening: nodes sleeve, record"""
    g = Glb()
    g.node('sleeve', sleeve_prims(g, cover, out, name, front_right, open_edge=True), [0, SLEEVE_H / 2, 0])
    g.node('record', record_prims(g, label_a, label_b, out, name), [SLEEVE_W / 2 + PEEK - R_OUT, SLEEVE_H / 2, 0])
    return g.write(out / f'{name}.set.glb')


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--label-a', type=Path, required=True)
    ap.add_argument('--label-b', type=Path, required=True)
    ap.add_argument('--cover', type=Path)
    ap.add_argument('--out', type=Path, required=True)
    ap.add_argument('--name', default='awakening_vinyl')
    ap.add_argument('--front', choices=('left', 'right'), default='right' if FRONT_IS_RIGHT_HALF else 'left',
                    help="the cover PDF's half that is the sleeve's front (default: FRONT_IS_RIGHT_HALF)")
    a = ap.parse_args()
    a.out.mkdir(parents=True, exist_ok=True)
    print('record', build_record(a.label_a, a.label_b, a.out, a.name), 'bytes')
    if a.cover:
        front_right = a.front == 'right'
        print('sleeve', build_sleeve(a.cover, a.out, a.name, front_right), 'bytes')
        print('set', build_set(a.label_a, a.label_b, a.cover, a.out, a.name, front_right), 'bytes')


if __name__ == '__main__':
    main()
