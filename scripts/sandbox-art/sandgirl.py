"""Sandgirl – the sandbox character, drawn by code (flat toon, transparent background).
full 600x1200, portrait 600x600 (18 moods), avatar 600x600."""
import math, os, sys
from PIL import Image, ImageDraw, ImageFilter

S = 4
W, H = 600, 1200
OUT = (74, 48, 28)
SKIN = (238, 203, 150); SKIN_SH = (216, 174, 120)
HAIR = (203, 152, 64); HAIR_L = (236, 196, 118); HAIR_D = (160, 112, 40)
DRESS = (47, 167, 160); DRESS_D = (30, 128, 124); DRESS_L = (120, 208, 200)
CORAL = (232, 96, 76); YEL = (245, 196, 60); YEL_D = (222, 160, 40); RED = (214, 62, 52); WOOD = (176, 122, 66)
WHITE = (255, 255, 255); MOUTH = (122, 43, 48); TONGUE = (240, 128, 128)
IRIS = (38, 140, 134); TEAR = (120, 196, 240)
OW = 4.5


def bez(p0, p1, p2, p3, n=28):
    out = []
    for i in range(n + 1):
        t = i / n; u = 1 - t
        out.append((u**3 * p0[0] + 3 * u * u * t * p1[0] + 3 * u * t * t * p2[0] + t**3 * p3[0],
                    u**3 * p0[1] + 3 * u * u * t * p1[1] + 3 * u * t * t * p2[1] + t**3 * p3[1]))
    return out


def arcpts(cx, cy, rx, ry, a0, a1, n=32):
    return [(cx + rx * math.cos(math.radians(a0 + (a1 - a0) * i / n)), cy + ry * math.sin(math.radians(a0 + (a1 - a0) * i / n))) for i in range(n + 1)]


class C:
    def __init__(s, img, ox=0, oy=0):
        s.img = img; s.d = ImageDraw.Draw(img); s.ox = ox; s.oy = oy

    def p(s, x, y):
        return ((x - s.ox) * S, (y - s.oy) * S)

    def ell(s, cx, cy, rx, ry, fill):
        a = s.p(cx - rx, cy - ry); b = s.p(cx + rx, cy + ry)
        s.d.ellipse([a[0], a[1], b[0], b[1]], fill=fill)

    def poly(s, pts, fill):
        s.d.polygon([s.p(*q) for q in pts], fill=fill)

    def line(s, pts, fill, width):
        q = [s.p(*t) for t in pts]
        s.d.line(q, fill=fill, width=max(1, int(width * S)), joint='curve')
        r = width * S / 2
        for t in (q[0], q[-1]):
            s.d.ellipse([t[0] - r, t[1] - r, t[0] + r, t[1] + r], fill=fill)

    def arc(s, cx, cy, rx, ry, a0, a1, fill, width):
        s.line(arcpts(cx, cy, rx, ry, a0, a1), fill, width)

    def chord(s, cx, cy, rx, ry, a0, a1, fill):
        s.poly(arcpts(cx, cy, rx, ry, a0, a1, 40), fill)


def outlined(base, fn, ow=OW):
    """draw fn on its own layer, put a dark outline around the layer's shape, composite on base"""
    lay = Image.new('RGBA', base.size, (0, 0, 0, 0))
    fn(C(lay))
    a = lay.getchannel('A')
    dil = a.filter(ImageFilter.GaussianBlur(ow * S / 1.55)).point(lambda v: 255 if v > 16 else 0)
    o = Image.new('RGBA', base.size, OUT + (255,)); o.putalpha(dil)
    base.alpha_composite(o); base.alpha_composite(lay)


# ---------------------------------------------------------------- the body (once)
def hair_back(c):
    c.ell(300, 250, 172, 160, HAIR)
    c.ell(148, 372, 54, 50, HAIR); c.ell(452, 372, 54, 50, HAIR)
    for cx in (148, 452):
        c.arc(cx, 380, 34, 26, 200, 340, HAIR_L, 5)
        c.arc(cx, 398, 30, 20, 205, 335, HAIR_D, 4)


def legs(c):
    for x0, x1, fx in ((264, 258, 244), (336, 342, 356)):
        c.line([(x0, 740), (x1, 1085)], SKIN, 40)
        c.ell(fx, 1122, 50, 13, CORAL)
        c.ell(fx, 1106, 46, 22, SKIN)
    c.line([(214, 1096), (268, 1088)], CORAL, 9); c.line([(332, 1088), (386, 1096)], CORAL, 9)


def arm_left(c):
    c.line([(214, 468), (182, 580), (170, 676)], SKIN, 36)
    c.line([(172, 640), (150, 806)], WOOD, 13)
    c.ell(146, 852, 34, 48, RED)
    c.line([(146, 822), (146, 880)], (236, 110, 96), 6)
    c.ell(170, 682, 25, 25, SKIN)


def arm_right(c):
    c.line([(386, 468), (418, 580), (432, 676)], SKIN, 36)
    c.arc(432, 742, 40, 44, 180, 360, OUT, 5)
    c.poly([(392, 742), (472, 742), (462, 826), (402, 826)], YEL)
    c.poly([(384, 734), (480, 734), (480, 752), (384, 752)], YEL_D)
    c.poly([(420, 770), (444, 770), (441, 800), (423, 800)], CORAL)
    c.ell(432, 688, 25, 25, SKIN)


def dress(c):
    c.poly([(238, 424), (362, 424), (394, 474), (434, 762), (166, 762), (206, 474)], DRESS)
    for i in range(7):
        c.ell(188 + i * 37.3, 762, 21, 21, DRESS)
    c.ell(214, 458, 36, 30, DRESS_D); c.ell(386, 458, 36, 30, DRESS_D)
    # a sand wave band and ripples
    wave = [(172 + i * 4, 700 + 9 * math.sin(i * 4 / 18.0)) for i in range(65)]
    c.line(wave, (250, 226, 170), 12)
    wave2 = [(180 + i * 4, 726 + 6 * math.sin(i * 4 / 14.0 + 1)) for i in range(61)]
    c.line(wave2, DRESS_L, 5)
    # a little sun-shell on the chest
    c.ell(300, 520, 24, 24, YEL)
    for a in range(200, 341, 35):
        c.line([(300, 540), (300 + 22 * math.cos(math.radians(a)), 540 + 34 * math.sin(math.radians(a)))], YEL_D, 3)
    # collar
    c.arc(300, 424, 46, 26, 10, 170, DRESS_L, 7)


def head(c):
    c.line([(300, 392), (300, 432)], SKIN_SH, 50)
    c.ell(300, 270, 140, 130, SKIN)


def hair_front(c):
    c.ell(300, 152, 146, 56, HAIR)
    c.ell(226, 180, 72, 42, HAIR); c.ell(300, 170, 78, 44, HAIR); c.ell(376, 180, 72, 42, HAIR)
    peak = bez((196, 172), (206, 90), (300, 44), (398, 32)) + bez((398, 32), (444, 28), (462, 64), (434, 84)) + \
        bez((434, 84), (416, 96), (398, 88), (402, 70)) + bez((402, 70), (372, 96), (384, 142), (410, 176))
    c.poly(peak, HAIR)
    # wind ripples
    c.line(bez((232, 160), (252, 104), (318, 70), (392, 56)), HAIR_L, 7)
    c.line(bez((270, 168), (286, 124), (330, 100), (380, 98)), HAIR_L, 6)
    c.line(bez((312, 172), (322, 146), (350, 132), (384, 136)), HAIR_D, 4)
    c.arc(226, 184, 50, 26, 30, 150, HAIR_D, 4)
    c.arc(300, 176, 54, 28, 30, 150, HAIR_D, 4)
    c.arc(376, 184, 50, 26, 30, 150, HAIR_D, 4)


def starfish(c):
    cx, cy = 184, 206
    pts = []
    for i in range(10):
        r = 27 if i % 2 == 0 else 12
        a = math.radians(-90 + i * 36 + 12)
        pts.append((cx + r * math.cos(a), cy + r * math.sin(a)))
    c.poly(pts, CORAL)
    c.ell(cx, cy, 5, 5, (250, 170, 150))


def body():
    base = Image.new('RGBA', (W * S, H * S), (0, 0, 0, 0))
    for fn in (hair_back, legs, arm_left, arm_right, dress, head, hair_front, starfish):
        outlined(base, fn)
    return base


# ---------------------------------------------------------------- faces
EX, EY, EDX = 300, 294, 56  # eye centre line


def brow(c, side, inner, outer):
    cx = EX + side * EDX
    c.line([(cx - side * 26, 240 + inner * 0.7), (cx + side * 4, 235 + (inner + outer) * 0.35 - 2), (cx + side * 30, 242 + outer * 0.7)], (122, 80, 28), 9)


def eye(c, side, kind, look=(0, 0)):
    cx = EX + side * EDX; cy = EY
    if kind in ('open', 'wide', 'half', 'puppy'):
        rx, ry = (30, 38) if kind != 'wide' else (33, 43)
        c.ell(cx, cy, rx + 3.5, ry + 3.5, OUT); c.ell(cx, cy, rx, ry, WHITE)
        ir = {'open': 20, 'half': 20, 'wide': 12, 'puppy': 25}[kind]
        ix, iy = cx + look[0], cy + look[1] + (3 if kind != 'wide' else 0)
        c.ell(ix, iy, ir, ir, IRIS); c.ell(ix, iy, ir * 0.55, ir * 0.55, (20, 30, 34))
        c.ell(ix - ir * 0.35, iy - ir * 0.4, ir * 0.3, ir * 0.3, WHITE)
        if kind == 'puppy':
            c.ell(ix + ir * 0.35, iy + ir * 0.35, ir * 0.18, ir * 0.18, WHITE)
        if kind == 'half':
            c.chord(cx, cy, rx + 4, ry + 4, 196, 344, SKIN)
            c.line([(cx - rx - 2, cy - 11), (cx + rx + 2, cy - 11)], OUT, 4.5)
    elif kind == 'happy':
        c.arc(cx, cy + 10, 28, 26, 200, 340, OUT, 6)
    elif kind == 'soft':
        c.arc(cx, cy - 10, 28, 22, 20, 160, OUT, 6)
    elif kind == 'shut':
        c.line([(cx - 28, cy + 2), (cx + 28, cy + 2)], OUT, 6)
    elif kind == 'squint':
        c.line([(cx - side * 26, cy - 18), (cx + side * 18, cy), (cx - side * 26, cy + 18)], OUT, 6)
    elif kind == 'swirl':
        pts = [(cx + (3 + t * 1.5) * math.cos(t * 0.75) , cy + (3 + t * 1.5) * math.sin(t * 0.75)) for t in range(0, 19)]
        c.ell(cx, cy, 33, 36, WHITE)
        c.line(pts, OUT, 4.5)


def mouth(c, kind):
    x, y = 300, 356
    if kind == 'small':
        c.arc(x, y - 12, 26, 16, 25, 155, OUT, 5.5)
    elif kind == 'smile':
        c.arc(x, y - 16, 40, 24, 15, 165, OUT, 6)
    elif kind in ('grin', 'laugh'):
        rx, ry = (40, 30) if kind == 'grin' else (48, 44)
        c.chord(x, y - 14, rx + 4, ry + 4, 0, 180, OUT); c.poly([(x - rx - 4, y - 18), (x + rx + 4, y - 18), (x + rx + 4, y - 13), (x - rx - 4, y - 13)], OUT)
        c.chord(x, y - 14, rx, ry, 0, 180, MOUTH)
        c.poly([(x - rx * 0.8, y - 14), (x + rx * 0.8, y - 14), (x + rx * 0.74, y - 5), (x - rx * 0.74, y - 5)], WHITE)
        c.ell(x, y - 14 + ry * 0.68, rx * 0.42, ry * 0.26, TONGUE)
    elif kind == 'frown':
        c.arc(x, y + 14, 32, 20, 200, 340, OUT, 6)
    elif kind == 'tight':
        c.arc(x, y + 20, 36, 22, 215, 325, OUT, 7)
    elif kind == 'wail':
        c.chord(x, y + 16, 40, 34, 180, 360, OUT); c.poly([(x - 40, y + 14), (x + 40, y + 14), (x + 40, y + 20), (x - 40, y + 20)], OUT)
        c.chord(x, y + 14, 35, 28, 180, 360, MOUTH)
        c.ell(x, y + 6, 16, 7, TONGUE)
    elif kind == 'flat':
        c.line([(x - 24, y), (x + 24, y)], OUT, 6)
    elif kind == 'side':
        c.line([(x - 4, y + 2), (x + 34, y - 4)], OUT, 6)
    elif kind == 'grit':
        c.poly([(x - 42, y - 16), (x + 42, y - 16), (x + 36, y + 14), (x - 36, y + 14)], OUT)
        c.poly([(x - 36, y - 11), (x + 36, y - 11), (x + 31, y + 9), (x - 31, y + 9)], WHITE)
        for dx in (-18, 0, 18):
            c.line([(x + dx, y - 10), (x + dx, y + 8)], OUT, 3)
        c.line([(x - 33, y - 1), (x + 33, y - 1)], OUT, 3)
    elif kind == 'o':
        c.ell(x, y + 2, 20, 26, OUT); c.ell(x, y + 2, 15, 21, MOUTH)
    elif kind == 'oh':
        c.ell(x, y, 13, 14, OUT); c.ell(x, y, 8.5, 9.5, MOUTH)
    elif kind == 'wavy':
        c.line([(x - 36 + i * 3, y + 6 * math.sin(i * 3 / 7.0)) for i in range(25)], OUT, 5.5)
    elif kind == 'smirk':
        c.line(bez((x - 26, y + 2), (x - 6, y + 10), (x + 22, y + 6), (x + 38, y - 14), 16), OUT, 6)
    elif kind == 'bleh':
        c.arc(x, y + 12, 34, 20, 200, 340, OUT, 6)
        c.ell(x + 6, y + 8, 15, 15, OUT); c.ell(x + 6, y + 8, 11, 11, TONGUE)
    elif kind == 'softsmile':
        c.arc(x, y - 10, 30, 14, 20, 160, OUT, 5.5)


def drop(c, x, y, r, col=TEAR):
    c.poly([(x, y - r * 1.9), (x + r * 0.85, y - r * 0.3), (x - r * 0.85, y - r * 0.3)], col)
    c.ell(x, y, r, r, col)
    c.ell(x - r * 0.3, y - r * 0.2, r * 0.28, r * 0.28, WHITE)


def star(c, cx, cy, r, col):
    pts = []
    for i in range(8):
        rr = r if i % 2 == 0 else r * 0.32
        a = math.radians(-90 + i * 45)
        pts.append((cx + rr * math.cos(a), cy + rr * math.sin(a)))
    c.poly(pts, col)


MOODS = {
    #            brows L(inner,outer) R(inner,outer)   eyes        look     mouth        extras
    'neutral':     ((0, 0), (0, 0), 'open', (0, 0), 'small', []),
    'happy':       ((-8, -6), (-8, -6), 'open', (0, 0), 'grin', ['sparkle']),
    'laughing':    ((-12, -8), (-12, -8), 'happy', (0, 0), 'laugh', ['joytear', 'blush']),
    'sad':         ((-14, 8), (-14, 8), 'half', (0, 8), 'frown', []),
    'crying':      ((-16, 10), (-16, 10), 'soft', (0, 0), 'wail', ['tears']),
    'angry':       ((16, -8), (16, -8), 'half', (0, 2), 'tight', []),
    'furious':     ((22, -12), (22, -12), 'wide', (0, 0), 'grit', ['vein', 'rage']),
    'surprised':   ((-22, -20), (-22, -20), 'wide', (0, 0), 'o', ['pop']),
    'scared':      ((-18, 4), (-18, 4), 'wide', (0, 2), 'wavy', ['gloom', 'sweat']),
    'disgusted':   ((12, -4), (-6, 2), 'half', (-8, 4), 'bleh', ['gloom']),
    'confused':    ((10, -2), (-18, -14), 'open', (6, -4), 'oh', ['question']),
    'thinking':    ((4, 0), (-16, -14), 'open', (9, -10), 'side', ['dots']),
    'smug':        ((2, 0), (-14, -12), 'half', (8, 2), 'smirk', []),
    'embarrassed': ((-12, 6), (-12, 6), 'open', (-9, 8), 'wavy', ['blush', 'sweat']),
    'denial':      ((-10, -10), (-10, -10), 'shut', (0, 0), 'frown', ['huff']),
    'bargaining':  ((-18, 2), (-18, 2), 'puppy', (0, -3), 'oh', ['sparkle']),
    'acceptance':  ((-4, 0), (-4, 0), 'soft', (0, 0), 'softsmile', ['blush']),
    'dizzy':       ((-10, 0), (-10, 0), 'swirl', (0, 0), 'wavy', ['stars']),
}


def face(img, mood, ox=0, oy=0):
    bl, br, ek, look, mk, ex = MOODS[mood]
    lay = Image.new('RGBA', img.size, (0, 0, 0, 0))
    c = C(lay, ox, oy)
    if 'rage' in ex:
        c.ell(300, 318, 126, 78, (232, 110, 80, 70))
    if 'gloom' in ex:
        for i, x in enumerate(range(236, 372, 15)):
            c.line([(x, 228), (x, 244 + (i % 2) * 8)], (110, 130, 170, 170), 3.5)
    strong = 'blush' in ex
    for sx in (-1, 1):
        c.ell(300 + sx * 96, 334, 30 if strong else 25, 17 if strong else 13, (240, 130, 110, 200 if strong else 110))
        if strong:
            for k in (-10, 0, 10):
                c.line([(300 + sx * 96 + k - 4, 342), (300 + sx * 96 + k + 4, 326)], (214, 84, 76, 230), 3)
        for dx, dy in ((-12, -2), (0, 6), (12, -3)):
            c.ell(300 + sx * 96 + dx, 322 + dy - (0 if not strong else 16), 2.6, 2.6, (184, 134, 72, 255))
    if 'tears' in ex:
        for sx in (-1, 1):
            c.line(bez((300 + sx * 60, 304), (300 + sx * 70, 340), (300 + sx * 84, 370), (300 + sx * 78, 392), 14), TEAR, 13)
    brow(c, -1, *bl); brow(c, 1, *br)
    eye(c, -1, ek, look); eye(c, 1, ek, look)
    c.arc(300, 314, 9, 7, 30, 150, (190, 140, 96), 4.5)
    mouth(c, mk)
    if 'tears' in ex:
        drop(c, 212, 368, 9); drop(c, 392, 380, 8)
    if 'joytear' in ex:
        drop(c, 216, 300, 8); drop(c, 384, 300, 8)
    if 'sweat' in ex:
        drop(c, 414, 226, 11)
    if 'vein' in ex:
        for a in (0, 90, 180, 270):
            x0 = 404 + 9 * math.cos(math.radians(a + 45)); y0 = 208 + 9 * math.sin(math.radians(a + 45))
            c.arc(x0 + 7 * math.cos(math.radians(a + 45)), y0 + 7 * math.sin(math.radians(a + 45)), 9, 9, a + 135, a + 315, RED, 4.5)
    if 'sparkle' in ex:
        star(c, 428, 232, 15, YEL); star(c, 170, 262, 10, YEL); star(c, 446, 268, 8, WHITE)
    if 'pop' in ex:
        for a in (-60, -90, -120):
            c.line([(300 + 180 * math.cos(math.radians(a)) * 0.0 + 150 * math.cos(math.radians(a)) + 150 * 0, 0)], OUT, 0.1) if False else None
        for x0, y0, x1, y1 in ((452, 180, 476, 160), (462, 214, 492, 208), (148, 180, 124, 160)):
            c.line([(x0, y0), (x1, y1)], OUT, 5)
    if 'question' in ex:
        c.line(arcpts(446, 150, 17, 17, 180, 400) + [(446, 178), (446, 190)], OUT, 12)
        c.line(arcpts(446, 150, 17, 17, 180, 400) + [(446, 178), (446, 190)], YEL, 6.5)
        c.ell(446, 210, 7.5, 7.5, OUT); c.ell(446, 210, 4.5, 4.5, YEL)
    if 'dots' in ex:
        for i, (x, y, r) in enumerate(((432, 196, 6), (452, 170, 9), (478, 136, 13))):
            c.ell(x, y, r + 3, r + 3, OUT); c.ell(x, y, r, r, WHITE)
    if 'huff' in ex:
        for x0, y0 in ((440, 372), (452, 392)):
            c.arc(x0, y0, 16, 9, 200, 400, OUT, 4.5)
    if 'stars' in ex:
        for i in range(5):
            a = math.radians(200 + i * 35)
            star(c, 300 + 150 * math.cos(a), 150 + 70 * math.sin(a) + 20, 13 if i % 2 == 0 else 9, YEL)
    img.alpha_composite(lay)


def main(out):
    os.makedirs(out, exist_ok=True)
    base = body()
    PORT = (50, 10, 550, 510); AV = (100, 30, 500, 430)
    for mood in MOODS:
        if mood == 'neutral':
            im = base.copy(); face(im, mood)
            im.resize((W, H), Image.LANCZOS).save(f'{out}/sandgirl_full-neutral.webp', 'WEBP', quality=92, method=6)
            im.crop([v * S for v in AV]).resize((600, 600), Image.LANCZOS).save(f'{out}/sandgirl_avatar-neutral.webp', 'WEBP', quality=92, method=6)
            p = im.crop([v * S for v in PORT])
        else:
            p = base.crop([v * S for v in PORT]); face(p, mood, PORT[0], PORT[1])
        p.resize((600, 600), Image.LANCZOS).save(f'{out}/sandgirl_portrait-{mood}.webp', 'WEBP', quality=92, method=6)
        print(mood)
    # a contact sheet to look at
    sheet = Image.new('RGB', (6 * 300, 3 * 300), (40, 44, 52))
    for i, mood in enumerate(MOODS):
        t = Image.open(f'{out}/sandgirl_portrait-{mood}.webp').resize((300, 300), Image.LANCZOS)
        sheet.paste(t, ((i % 6) * 300, (i // 6) * 300), t)
    sheet.save(f'{out}/_sheet.png')
    f = Image.open(f'{out}/sandgirl_full-neutral.webp')
    bg = Image.new('RGB', f.size, (40, 44, 52)); bg.paste(f, (0, 0), f); bg.save(f'{out}/_full.png')


if __name__ == "__main__":
    main(sys.argv[1])
