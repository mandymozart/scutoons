"""Sandbox scenery, drawn by code: the chapter's slices, its thumbnail, the set's still, the toon avatar and the
map place (diorama + plaster)."""
import math, os, sys
from PIL import Image, ImageDraw, ImageFilter

OUT = (74, 48, 28)
CORAL = (232, 96, 76); YEL = (245, 196, 60); YEL_D = (222, 160, 40); RED = (214, 62, 52); WOOD = (176, 122, 66)
CAST = (232, 190, 122); CAST_D = (206, 160, 96)
GREEN = (58, 158, 108); GREEN_D = (40, 124, 92)
SANDS = [(250, 230, 184), (244, 214, 156), (236, 198, 132), (226, 180, 110), (212, 162, 92)]


class K:
    """a canvas in its own units (u = px at 1x), drawn at scale s"""
    def __init__(s, img, sc, ox=0, oy=0):
        s.img = img; s.d = ImageDraw.Draw(img); s.s = sc; s.ox = ox; s.oy = oy

    def p(s, x, y):
        return ((x + s.ox) * s.s, (y + s.oy) * s.s)

    def ell(s, cx, cy, rx, ry, fill):
        a = s.p(cx - rx, cy - ry); b = s.p(cx + rx, cy + ry)
        s.d.ellipse([a[0], a[1], b[0], b[1]], fill=fill)

    def poly(s, pts, fill):
        s.d.polygon([s.p(*q) for q in pts], fill=fill)

    def line(s, pts, fill, width):
        q = [s.p(*t) for t in pts]
        w = max(1, int(width * s.s))
        s.d.line(q, fill=fill, width=w, joint='curve')
        r = w / 2
        for t in (q[0], q[-1]):
            s.d.ellipse([t[0] - r, t[1] - r, t[0] + r, t[1] + r], fill=fill)


def outlined(base, sc, fn, ow=2.2, ox=0, oy=0):
    lay = Image.new('RGBA', base.size, (0, 0, 0, 0))
    fn(K(lay, sc, ox, oy))
    a = lay.getchannel('A')
    dil = a.filter(ImageFilter.GaussianBlur(ow * sc / 1.55)).point(lambda v: 255 if v > 16 else 0)
    o = Image.new('RGBA', base.size, OUT + (255,)); o.putalpha(dil)
    base.alpha_composite(o); base.alpha_composite(lay)


def castle(c, x, y, k=1.0):
    def P(dx, dy): return (x + dx * k, y + dy * k)
    c.poly([P(-34, 0), P(34, 0), P(34, -30), P(-34, -30)], CAST)
    for i in range(-3, 4, 2):
        c.poly([P(i * 9 - 4, -30), P(i * 9 + 4, -30), P(i * 9 + 4, -37), P(i * 9 - 4, -37)], CAST)
    for sx in (-1, 1):
        c.poly([P(sx * 34 - 11, 0), P(sx * 34 + 11, 0), P(sx * 34 + 9, -48), P(sx * 34 - 9, -48)], CAST_D)
        c.poly([P(sx * 34 - 14, -48), P(sx * 34 + 14, -48), P(sx * 34, -72)], CORAL)
    c.poly([P(-14, -30), P(14, -30), P(13, -58), P(-13, -58)], CAST)
    c.poly([P(-18, -58), P(18, -58), P(0, -88)], CORAL)
    c.line([P(0, -88), P(0, -104)], OUT, 1.6 * k)
    c.poly([P(0, -104), P(14, -99), P(0, -94)], YEL)
    c.poly([P(-7, 0), P(7, 0), P(7, -12), P(-7, -12)], (120, 84, 48)); c.ell(*P(0, -12), 7 * k, 7 * k, (120, 84, 48))


def palm(c, x, y, k=1.0):
    def P(dx, dy): return (x + dx * k, y + dy * k)
    trunk = [P(0, 0), P(4, -30), P(12, -60), P(24, -88)]
    c.line(trunk, WOOD, 9 * k)
    tx, ty = P(24, -88)
    for a, ln, g in ((200, 46, GREEN_D), (160, 40, GREEN), (250, 40, GREEN), (300, 44, GREEN_D), (340, 46, GREEN), (20, 40, GREEN_D)):
        r = math.radians(a)
        ex, ey = tx + ln * k * math.cos(r), ty + ln * k * math.sin(r) + 14 * k
        mx, my = tx + ln * k * 0.5 * math.cos(r), ty + ln * k * 0.5 * math.sin(r) - 8 * k
        nx, ny = -math.sin(r) * 7 * k, math.cos(r) * 7 * k
        c.poly([(tx, ty), (mx + nx, my + ny), (ex, ey), (mx - nx, my - ny)], g)
    c.ell(tx + 3 * k, ty + 5 * k, 4 * k, 4 * k, (120, 80, 44)); c.ell(tx - 4 * k, ty + 6 * k, 4 * k, 4 * k, (120, 80, 44))


def bucket(c, x, y, k=1.0):
    c.poly([(x - 11 * k, y - 22 * k), (x + 11 * k, y - 22 * k), (x + 8 * k, y), (x - 8 * k, y)], YEL)
    c.poly([(x - 13 * k, y - 25 * k), (x + 13 * k, y - 25 * k), (x + 13 * k, y - 20 * k), (x - 13 * k, y - 20 * k)], YEL_D)


def shovel(c, x, y, k=1.0):
    c.line([(x, y), (x + 5 * k, y - 30 * k)], WOOD, 3.4 * k)
    c.ell(x - 1 * k, y + 2 * k, 7 * k, 9 * k, RED)


def starfish(c, x, y, r):
    pts = []
    for i in range(10):
        rr = r if i % 2 == 0 else r * 0.45
        a = math.radians(-90 + i * 36 + 10)
        pts.append((x + rr * math.cos(a), y + rr * math.sin(a) * 0.8))
    c.poly(pts, CORAL)


def ridge(w, base, amp, f, ph, n=120):
    return [(w * i / n, base + amp * math.sin(i / n * f + ph) + amp * 0.35 * math.sin(i / n * f * 2.3 + ph * 1.7)) for i in range(n + 1)]


def backdrop(w, h, sc, horizon=0.42, seed=0.0, sun=(0.74, 0.16)):
    """sky, sun, dune layers; returns the image and the ridges (far → near)"""
    img = Image.new('RGBA', (int(w * sc), int(h * sc)), (0, 0, 0, 255))
    d = ImageDraw.Draw(img)
    top = (86, 176, 196); mid = (178, 222, 214); hor = (253, 234, 194)
    H = int(h * sc)
    for y in range(H):
        t = min(1.0, y / (H * horizon))
        a, b, u = (top, mid, t / 0.6) if t < 0.6 else (mid, hor, (t - 0.6) / 0.4)
        d.line([(0, y), (int(w * sc), y)], fill=tuple(int(a[k] + (b[k] - a[k]) * u) for k in range(3)) + (255,))
    c = K(img, sc)
    sx, sy = w * sun[0], h * sun[1]
    halo = Image.new('RGBA', img.size, (0, 0, 0, 0)); hc = K(halo, sc)
    for r, al in ((0.2, 40), (0.15, 60), (0.11, 90)):
        hc.ell(sx, sy, w * r, w * r, (255, 244, 214, al))
    img.alpha_composite(halo)
    c.ell(sx, sy, w * 0.075, w * 0.075, (255, 248, 226))
    ridges = []
    for i in range(5):
        base = h * (horizon + (1 - horizon) * (i / 5.0) ** 1.25) - (h * 0.02 if i == 0 else 0)
        amp = h * (0.012 + 0.012 * i)
        r = ridge(w, base, amp, 5.0 + 2.1 * i, seed + i * 1.9)
        ridges.append(r)
        outlined(img, sc, lambda k, r=r, i=i: k.poly(r + [(w, h + 10), (0, h + 10)], SANDS[i]), 1.3)
        c = K(img, sc)
        for off, col in ((0.022, 0.93), (0.05, 0.9), (0.085, 0.94)):
            c.line([(x + w * 0.03, y + h * off * (0.6 + 0.35 * i)) for x, y in r[4:-6:2]], tuple(int(v * col) for v in SANDS[i]), 1.2 + 0.25 * i)
    return img, ridges


def y_at(r, x):
    for (x0, y0), (x1, y1) in zip(r, r[1:]):
        if x0 <= x <= x1:
            return y0 + (y1 - y0) * (x - x0) / max(1e-6, x1 - x0)
    return r[-1][1]


def paste_girl(img, sc, girl, x, foot_y, height):
    g = girl.copy()
    k = height * sc / g.height
    g = g.resize((int(g.width * k), int(g.height * k)), Image.LANCZOS)
    # the art's feet are at 94.5 % of its height
    px = int(x * sc - g.width / 2); py = int(foot_y * sc - g.height * 0.945)
    sh = Image.new('RGBA', img.size, (0, 0, 0, 0))
    K(sh, sc).ell(x, foot_y, height * 0.17, height * 0.028, (120, 80, 40, 110))
    img.alpha_composite(sh.filter(ImageFilter.GaussianBlur(sc * 1.5)))
    img.alpha_composite(g, (px, py))


def footprints(img, sc, pts):
    c = K(img, sc)
    for i, (x, y, r) in enumerate(pts):
        c.ell(x + (3 if i % 2 else -3) * r / 4, y, r, r * 0.5, (200, 152, 88))


def save_variants(img3, out, name, w, h):
    """img3 = the 3x render (or larger); writes name.webp (1x), .2x, .3x"""
    for sfx, m in (('', 1), ('.2x', 2), ('.3x', 3)):
        img3.convert('RGB').resize((w * m, h * m), Image.LANCZOS).save(f'{out}/{name}{sfx}.webp', 'WEBP', quality=88, method=6)


def slices(out, girl_dir):
    full = Image.open(f'{girl_dir}/sandgirl_full-neutral.webp').convert('RGBA')
    W, H, sc = 390, 624, 6
    # 1: the dunes, far castle, footprints
    img, r = backdrop(W, H, sc, 0.40, 0.3)
    outlined(img, sc, lambda c: castle(c, 96, y_at(r[1], 96) + 6, 0.62), 1.4)
    outlined(img, sc, lambda c: palm(c, 300, y_at(r[2], 300) + 6, 0.9), 1.6)
    footprints(img, sc, [(210 - i * 16, 600 - i * 34, 9 - i * 0.7) for i in range(8)])
    save_variants(img, out, 's1c1-slice-2', W, H)
    # 2: Sandgirl on the dune
    img, r = backdrop(W, H, sc, 0.34, 1.4, (0.2, 0.13))
    outlined(img, sc, lambda c: castle(c, 300, y_at(r[2], 300) + 8, 0.95), 1.6)
    outlined(img, sc, lambda c: (bucket(c, 318, 560, 1.5), shovel(c, 352, 566, 1.5)), 2.0)
    outlined(img, sc, lambda c: starfish(c, 64, 590, 16), 1.8)
    paste_girl(img, sc, full, 170, 592, 400)
    save_variants(img, out, 's1c1-slice-3', W, H)
    # 3: close on her, happy
    img, r = backdrop(W, H, sc, 0.5, 2.6, (0.8, 0.2))
    outlined(img, sc, lambda c: palm(c, 52, y_at(r[2], 52) + 8, 1.5), 2.0)
    happy = Image.open(f'{girl_dir}/sandgirl_portrait-happy.webp').convert('RGBA')
    g = happy.resize((int(430 * sc), int(430 * sc)), Image.LANCZOS)
    img.alpha_composite(g, (int(-20 * sc), int((H - 425) * sc)))
    c = K(img, sc)
    for i in range(26):
        x = (i * 97) % 390; y = 60 + (i * 53) % 300
        c.ell(x, y, 1.2 + (i % 3) * 0.6, 1.2 + (i % 3) * 0.6, (255, 240, 200, 255))
    save_variants(img, out, 's1c1-slice-4', W, H)
    # the thumbnail (square) and the toon avatar
    img, r = backdrop(400, 400, sc, 0.46, 0.9, (0.78, 0.2))
    outlined(img, sc, lambda c: castle(c, 316, y_at(r[2], 316) + 6, 0.8), 1.6)
    neutral = Image.open(f'{girl_dir}/sandgirl_portrait-neutral.webp').convert('RGBA')
    g = neutral.resize((int(330 * sc), int(330 * sc)), Image.LANCZOS)
    img.alpha_composite(g, (int(8 * sc), int(74 * sc)))
    img.convert('RGB').resize((600, 600), Image.LANCZOS).save(f'{out}/s1c1-thumbnail-2.webp', 'WEBP', quality=88, method=6)
    img, r = backdrop(400, 400, sc, 0.5, 2.0, (0.5, 0.26))
    av = Image.open(f'{girl_dir}/sandgirl_avatar-neutral.webp').convert('RGBA')
    g = av.resize((int(372 * sc), int(372 * sc)), Image.LANCZOS)
    img.alpha_composite(g, (int(14 * sc), int(30 * sc)))
    img.convert('RGB').resize((600, 600), Image.LANCZOS).save(f'{out}/avatar.webp', 'WEBP', quality=88, method=6)
    # the set's still, 16:9
    img, r = backdrop(1920, 1080, 2, 0.46, 0.6, (0.3, 0.2))
    outlined(img, 2, lambda c: castle(c, 1010, y_at(r[2], 1010) + 20, 3.2), 4.5)
    outlined(img, 2, lambda c: palm(c, 1560, y_at(r[2], 1560) + 20, 3.6), 4.5)
    outlined(img, 2, lambda c: (bucket(c, 1330, 930, 4.2), shovel(c, 1420, 944, 4.2)), 4.5)
    outlined(img, 2, lambda c: starfish(c, 420, 960, 46), 4.5)
    img.convert('RGB').resize((1920, 1080), Image.LANCZOS).save(f'{out}/dunes.still.v1.webp', 'WEBP', quality=86, method=6)


def place(out):
    """the map place: a little sand island (1024 → 3x, 512 → 2x, 256 → 1x) and its plaster twin"""
    sc = 2; N = 1024
    img = Image.new('RGBA', (N * sc, N * sc), (0, 0, 0, 0))

    def side(c):
        pts = []
        for i in range(73):
            a = 2 * math.pi * i / 72
            wob = 1 + 0.035 * math.sin(a * 9) + 0.02 * math.sin(a * 17 + 1)
            pts.append((512 + 430 * wob * math.cos(a), 690 + 215 * wob * math.sin(a)))
        c.poly(pts, (176, 126, 70))
        c.poly([(x, y - 62) for x, y in pts], SANDS[2])
    outlined(img, sc, side, 5)
    c = K(img, sc)
    for i, (cx, cy, rx, ry) in enumerate(((512, 640, 380, 170), (512, 632, 300, 128), (512, 626, 210, 86))):
        c.ell(cx, cy, rx, ry, SANDS[2 - i] if i else (240, 206, 144))
    # dunes on the island
    def mound(cx, cy, rx, ry, col, ripple):
        def f(k):
            pts = [(cx - rx + 2 * rx * i / 40, cy - ry * math.sin(math.pi * i / 40) ** 0.8) for i in range(41)]
            k.poly(pts, col)
        outlined(img, sc, f, 4)
        k = K(img, sc)
        for j in (0.35, 0.6, 0.82):
            k.line([(cx - rx * j + 2 * rx * j * i / 24, cy - ry * j * math.sin(math.pi * i / 24) ** 0.8 - 2) for i in range(2, 23)], ripple, 5)
    mound(330, 600, 190, 150, SANDS[1], SANDS[2])
    mound(700, 590, 170, 120, SANDS[0], SANDS[1])
    outlined(img, sc, lambda k: castle(k, 520, 640, 3.3), 5)
    mound(430, 760, 200, 100, SANDS[1], SANDS[3])
    outlined(img, sc, lambda k: palm(k, 760, 700, 3.3), 5)
    outlined(img, sc, lambda k: (bucket(k, 610, 800, 3.0), shovel(k, 676, 806, 3.0)), 5)
    outlined(img, sc, lambda k: starfish(k, 300, 800, 34), 5)
    big = img.resize((1024, 1024), Image.LANCZOS)
    for sfx, n in (('', 256), ('.2x', 512), ('.3x', 1024)):
        big.resize((n, n), Image.LANCZOS).save(f'{out}/sandbox-dunes{sfx}.webp', 'WEBP', quality=90, method=6)
    # plaster: a light clay model of the same shape
    l = big.convert('L')
    pl = Image.merge('RGBA', (l.point(lambda v: int(196 + v * 0.22)), l.point(lambda v: int(192 + v * 0.22)), l.point(lambda v: int(184 + v * 0.22)), big.getchannel('A')))
    for sfx, n in (('', 256), ('.2x', 512), ('.3x', 1024)):
        pl.resize((n, n), Image.LANCZOS).save(f'{out}/sandbox-dunes-plaster{sfx}.webp', 'WEBP', quality=90, method=6)
    bg = Image.new('RGB', (2048, 1024), (40, 44, 52)); bg.paste(big, (0, 0), big); bg.paste(pl, (1024, 0), pl); bg.save(f'{out}/_place.png')


if __name__ == '__main__':
    out, girl = sys.argv[1], sys.argv[2]
    os.makedirs(out, exist_ok=True)
    slices(out, girl); place(out)
    sheet = Image.new('RGB', (390 * 3 + 600 * 2, 624), (40, 44, 52))
    for i in (2, 3, 4):
        sheet.paste(Image.open(f'{out}/s1c1-slice-{i}.webp'), ((i - 2) * 390, 0))
    sheet.paste(Image.open(f'{out}/s1c1-thumbnail-2.webp'), (1170, 12)); sheet.paste(Image.open(f'{out}/avatar.webp'), (1770, 12))
    sheet.save(f'{out}/_slices.png')
