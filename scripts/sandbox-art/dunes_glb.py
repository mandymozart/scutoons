"""The sandbox set `dunes` as .glb: a dune field (flat stage in the middle), a sandcastle, a palm, bucket, shovel,
starfish, a beach ball, a sky dome and a sun; spots as empty nodes. Flat-shaded vertex colours (COLOR_0, linear), no textures."""
import json, math, os, struct, sys


def lin(c):
    return tuple(((v / 255.0) ** 2.2) for v in c)


L = (-0.45, 0.78, 0.43)
V = []
C = []


def tri(a, b, c, col, up=None, shade=True):
    ux, uy, uz = b[0] - a[0], b[1] - a[1], b[2] - a[2]
    vx, vy, vz = c[0] - a[0], c[1] - a[1], c[2] - a[2]
    n = (uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx)
    l = math.sqrt(sum(x * x for x in n)) or 1
    n = tuple(x / l for x in n)
    if up is not None and sum(n[i] * up[i] for i in range(3)) < 0:
        b, c = c, b
        n = tuple(-x for x in n)
    k = (0.5 + 0.5 * max(0.0, sum(n[i] * L[i] for i in range(3)))) if shade else 1.0
    r, g, bl = lin(col)
    for p in (a, b, c):
        V.append(p)
        C.append((r * k, g * k, bl * k, 1.0))


def quad(a, b, c, d, col, up=None, shade=True):
    tri(a, b, c, col, up, shade)
    tri(a, c, d, col, up, shade)


def height(x, z):
    h = 1.6 * math.sin(x * 0.21 + 0.6) * math.cos(z * 0.17 - 0.4) + 0.9 * math.sin(x * 0.09 + z * 0.13 + 2.0) + 0.35 * math.sin(x * 0.6 + z * 0.45)
    h += 0.06 * math.sin(x * 3.1 + z * 0.7)
    d = math.hypot(x, z)
    flat = max(0.0, min(1.0, (d - 4.5) / 5.0))
    flat = flat * flat * (3 - 2 * flat)
    rim = max(0.0, min(1.0, (d - 14) / 14.0))
    return h * flat * (0.55 + 1.6 * rim)


SAND = [(214, 170, 104), (232, 194, 128), (244, 214, 156), (250, 228, 178)]


def sandcol(y):
    t = max(0.0, min(0.999, (y + 2.2) / 5.0))
    i = int(t * 3)
    f = t * 3 - i
    a, b = SAND[i], SAND[i + 1]
    return tuple(a[k] + (b[k] - a[k]) * f for k in range(3))


def terrain(n, size=44.0):
    s = size / n
    for i in range(n):
        for j in range(n):
            x0, z0 = -size / 2 + i * s, -size / 2 + j * s
            p = [(x, height(x, z), z) for x, z in ((x0, z0), (x0 + s, z0), (x0 + s, z0 + s), (x0, z0 + s))]
            col = sandcol(sum(q[1] for q in p) / 4)
            if (i + j) % 2:
                tri(p[0], p[1], p[2], col, (0, 1, 0)); tri(p[0], p[2], p[3], col, (0, 1, 0))
            else:
                tri(p[0], p[1], p[3], col, (0, 1, 0)); tri(p[1], p[2], p[3], col, (0, 1, 0))


def cyl(cx, cy, cz, r0, r1, h, col, seg=10, cap=True, tilt=(0, 0)):
    def ring(r, y):
        return [(cx + tilt[0] * y / h + r * math.cos(2 * math.pi * k / seg), cy + y, cz + tilt[1] * y / h + r * math.sin(2 * math.pi * k / seg)) for k in range(seg)]
    a, b = ring(r0, 0), ring(r1, h)
    top = (cx + tilt[0], cy + h, cz + tilt[1])
    for k in range(seg):
        k2 = (k + 1) % seg
        mid = ((a[k][0] + a[k2][0]) / 2 - cx, 0.15, (a[k][2] + a[k2][2]) / 2 - cz)
        if r1 > 0:
            quad(a[k], a[k2], b[k2], b[k], col, mid)
            if cap:
                tri(top, b[k], b[k2], col, (0, 1, 0))
        else:
            tri(a[k], a[k2], top, col, mid)


def box(cx, cy, cz, w, h, d, col):
    x0, x1, y0, y1, z0, z1 = cx - w / 2, cx + w / 2, cy, cy + h, cz - d / 2, cz + d / 2
    quad((x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1), col, (0, 0, 1))
    quad((x0, y0, z0), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0), col, (0, 0, -1))
    quad((x0, y0, z0), (x0, y0, z1), (x0, y1, z1), (x0, y1, z0), col, (-1, 0, 0))
    quad((x1, y0, z0), (x1, y0, z1), (x1, y1, z1), (x1, y1, z0), col, (1, 0, 0))
    quad((x0, y1, z0), (x1, y1, z0), (x1, y1, z1), (x0, y1, z1), col, (0, 1, 0))


CAST = (226, 184, 116); CAST_D = (200, 156, 92); CORAL = (232, 96, 76); YEL = (245, 196, 60); RED = (214, 62, 52)


def castle(cx, cz, detail):
    y = height(cx, cz) - 0.1
    box(cx, y, cz, 4.4, 1.3, 3.4, CAST)
    for k in range(-4, 5, 2):
        box(cx + k * 0.5, y + 1.3, cz + 1.55, 0.42, 0.34, 0.3, CAST)
        box(cx + k * 0.5, y + 1.3, cz - 1.55, 0.42, 0.34, 0.3, CAST)
    for sx in (-1, 1):
        for sz in (-1, 1):
            cyl(cx + sx * 2.2, y, cz + sz * 1.7, 0.72, 0.62, 2.3, CAST_D, detail)
            cyl(cx + sx * 2.2, y + 2.3, cz + sz * 1.7, 0.86, 0, 1.1, CORAL, detail)
    box(cx, y + 1.3, cz, 1.9, 1.5, 1.6, CAST)
    cyl(cx, y + 2.8, cz, 1.25, 0, 1.5, CORAL, detail)
    cyl(cx, y + 4.3, cz, 0.04, 0.04, 0.9, (90, 60, 36), 5)
    tri((cx, y + 5.2, cz), (cx + 0.7, y + 5.0, cz), (cx, y + 4.8, cz), YEL, None)
    box(cx, y, cz + 1.72, 0.8, 0.9, 0.06, (120, 84, 48))


def palm(cx, cz, detail):
    y = height(cx, cz) - 0.1
    x, z = cx, cz
    for k in range(7):
        cyl(x, y, z, 0.26 - 0.02 * k, 0.24 - 0.02 * k, 0.82, (150, 104, 60) if k % 2 else (172, 122, 72), max(6, detail - 2), True, (0.16 + 0.05 * k, 0.05))
        x += 0.16 + 0.05 * k; z += 0.05; y += 0.8
    for k in range(7):
        a = 2 * math.pi * k / 7
        dx, dz = math.cos(a), math.sin(a)
        p0 = (x, y, z)
        p1 = (x + dx * 1.4 - dz * 0.5, y + 0.35, z + dz * 1.4 + dx * 0.5)
        p2 = (x + dx * 1.4 + dz * 0.5, y + 0.35, z + dz * 1.4 - dx * 0.5)
        p3 = (x + dx * 3.0, y - 0.9, z + dz * 3.0)
        g = (58, 158, 108) if k % 2 else (44, 132, 96)
        tri(p0, p1, p2, g, (0, 1, 0)); tri(p1, p3, p2, g, (0, 1, 0))
    for k in range(3):
        cyl(x + 0.25 * math.cos(k * 2.1), y - 0.3, z + 0.25 * math.sin(k * 2.1), 0.2, 0.0, 0.3, (120, 80, 44), 6)


def props(detail):
    cyl(3.6, 0, -1.6, 0.26, 0.36, 0.62, YEL, detail)
    cyl(3.6, 0.62, -1.6, 0.4, 0.4, 0.06, (222, 160, 40), detail)
    box(4.4, 0, -1.2, 0.07, 0.9, 0.07, (176, 122, 66))
    box(4.4, 0.0, -1.2, 0.34, 0.34, 0.05, RED)
    pts = [(-3.2 + (0.5 if k % 2 == 0 else 0.2) * math.cos(2 * math.pi * k / 10), 0.03, 1.6 + (0.5 if k % 2 == 0 else 0.2) * math.sin(2 * math.pi * k / 10)) for k in range(10)]
    for k in range(10):
        tri((-3.2, 0.09, 1.6), pts[k], pts[(k + 1) % 10], CORAL, (0, 1, 0))
    for (x, z, r) in ((-6.5, -3.0, 0.5), (-7.3, -2.4, 0.3), (7.5, 2.5, 0.42)):
        cyl(x, height(x, z) - 0.1, z, r, r * 0.5, r * 0.7, (168, 150, 130), 7)
    seg = max(8, detail)
    bx, by, bz, br = -4.6, 0.45, -1.4, 0.45

    def P(t, a):
        return (bx + br * math.sin(t) * math.cos(a), by + br * math.cos(t), bz + br * math.sin(t) * math.sin(a))
    for i in range(seg // 2):
        for k in range(seg):
            t0, t1 = math.pi * i / (seg // 2), math.pi * (i + 1) / (seg // 2)
            a0, a1 = 2 * math.pi * k / seg, 2 * math.pi * (k + 1) / seg
            col = [(250, 250, 250), CORAL, (250, 250, 250), (47, 167, 160)][k * 4 // seg]
            n = (math.sin((t0 + t1) / 2) * math.cos((a0 + a1) / 2), math.cos((t0 + t1) / 2), math.sin((t0 + t1) / 2) * math.sin((a0 + a1) / 2))
            quad(P(t0, a0), P(t0, a1), P(t1, a1), P(t1, a0), col, n)


def sky(seg=24, R=70.0):
    top = (86, 176, 196); mid = (176, 220, 214); hor = (252, 232, 190)
    rings = [(0.0, hor), (0.12, hor), (0.45, mid), (1.0, top)]

    def P(e, a):
        return (R * math.cos(e) * math.cos(a), R * math.sin(e) - 3.0, R * math.cos(e) * math.sin(a))
    for i in range(len(rings) - 1):
        e0, c0 = rings[i]
        e1, c1 = rings[i + 1]
        e0 *= math.pi / 2 * 0.98; e1 *= math.pi / 2 * 0.98
        col = tuple((c0[k] + c1[k]) / 2 for k in range(3))
        for k in range(seg):
            a0, a1 = 2 * math.pi * k / seg, 2 * math.pi * (k + 1) / seg
            quad(P(e0, a0), P(e0, a1), P(e1, a1), P(e1, a0), col, None, False)
    cx, cy, cz = -16.0, 20.0, -58.0
    for r, col, dz in ((6.0, (255, 236, 190), 0.0), (4.0, (255, 246, 220), 0.3)):
        for k in range(20):
            a0, a1 = 2 * math.pi * k / 20, 2 * math.pi * (k + 1) / 20
            tri((cx, cy, cz + dz), (cx + r * math.cos(a0), cy + r * math.sin(a0), cz + dz), (cx + r * math.cos(a1), cy + r * math.sin(a1), cz + dz), col, None, False)


def build(level, path):
    V.clear(); C.clear()
    n, detail = {'low': (36, 8), 'medium': (64, 12), 'high': (96, 18)}[level]
    terrain(n); castle(0.5, -9.5, detail); palm(7.0, -5.5, detail); props(detail); sky()
    pos = b''.join(struct.pack('<3f', *p) for p in V)
    col = b''.join(struct.pack('<4f', *c) for c in C)
    mn = [min(p[i] for p in V) for i in range(3)]
    mx = [max(p[i] for p in V) for i in range(3)]
    spots = {'spot_centre': (0, 0, 0), 'spot_left': (-2, 0, 0), 'spot_right': (2, 0, 0), 'spot_back': (0, 0, -3), 'spot_home': (0, 0, 0.5),
             'spot_castle': (0.5, height(0.5, -7.2), -7.2), 'spot_palm': (5.6, height(5.6, -4.6), -4.6), 'view': (0, 1.5, 7.5)}
    doc = {"asset": {"version": "2.0", "generator": "scutoons sandbox dunes_glb.py"}, "scene": 0,
           "scenes": [{"name": "dunes", "nodes": list(range(1 + len(spots)))}],
           "nodes": [{"name": "dunes", "mesh": 0}] + [{"name": k, "translation": [float(x) for x in v]} for k, v in spots.items()],
           "meshes": [{"name": "dunes", "primitives": [{"attributes": {"POSITION": 0, "COLOR_0": 1}, "material": 0, "mode": 4}]}],
           "materials": [{"name": "sand", "doubleSided": True, "pbrMetallicRoughness": {"baseColorFactor": [1, 1, 1, 1], "metallicFactor": 0, "roughnessFactor": 1}}],
           "accessors": [{"bufferView": 0, "componentType": 5126, "count": len(V), "type": "VEC3", "min": mn, "max": mx},
                         {"bufferView": 1, "componentType": 5126, "count": len(V), "type": "VEC4"}],
           "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": len(pos), "target": 34962},
                           {"buffer": 0, "byteOffset": len(pos), "byteLength": len(col), "target": 34962}],
           "buffers": [{"byteLength": len(pos) + len(col)}]}
    js = json.dumps(doc, separators=(',', ':')).encode()
    js += b' ' * ((4 - len(js) % 4) % 4)
    bin_ = pos + col
    bin_ += b'\0' * ((4 - len(bin_) % 4) % 4)
    out = struct.pack('<4sII', b'glTF', 2, 12 + 8 + len(js) + 8 + len(bin_)) + struct.pack('<I4s', len(js), b'JSON') + js + struct.pack('<I4s', len(bin_), b'BIN\0') + bin_
    open(path, 'wb').write(out)
    print(level, len(V) // 3, 'tris', len(out), 'bytes')


if __name__ == '__main__':
    os.makedirs(sys.argv[1], exist_ok=True)
    for lv in ('low', 'medium', 'high'):
        build(lv, f'{sys.argv[1]}/dunes.{lv}.v1.glb')
