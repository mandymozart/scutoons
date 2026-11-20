"""Sandy – the sandbox's sandworm (a friendly cousin of the ones from Dune), drawn by code.
full 600x1200, portrait 600x600 (18 moods), avatar 600x600; plus a new chapter slice with Sandy."""
import math, os, sys
from PIL import Image
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import sandgirl as sg
import scenery as sc
from sandgirl import C, outlined, bez, arcpts, drop, star, S, OUT, WHITE, YEL, RED, TEAR

BODY = (198, 146, 88); BODY_L = (226, 180, 120); BODY_D = (160, 110, 62); BELLY = (240, 208, 156)
LIP = (232, 186, 126); MAW = (70, 26, 34); THROAT = (36, 12, 18); TOOTH = (255, 246, 222); TONGUE = (240, 128, 128)
SAND = (236, 198, 132); SAND_D = (212, 162, 92)
W, H = 600, 1200


def spine(t):
    """the body's centre line, t 0 (head) … 1 (in the sand)"""
    return (300 + 46 * math.sin(t * 3.4 + 0.3) * t, 330 + t * 760)


def body(c):
    for i in range(22, -1, -1):
        t = i / 22
        x, y = spine(t)
        r = 150 + 26 * math.sin(t * math.pi) + 10 * t
        c.ell(x, y, r, 62, BODY if i % 2 else BODY_L)
    for i in range(22, 0, -1):
        t = i / 22
        x, y = spine(t)
        r = 150 + 26 * math.sin(t * math.pi) + 10 * t
        c.ell(x, y + 6, r * 0.5, 50, BELLY)
        c.arc(x, y + 6, r - 6, 54, 20, 160, BODY_D, 4)


def head(c):
    c.ell(300, 300, 186, 190, BODY_L)
    c.ell(214, 150, 46, 46, BODY_L); c.ell(386, 150, 46, 46, BODY_L)
    for a in (205, 230, 310, 335):
        c.arc(300, 300, 172, 176, a - 7, a + 7, BODY_D, 5)


def mound(c):
    pts = [(40 + 520 * i / 40, 1150 - 86 * math.sin(math.pi * i / 40) ** 0.7) for i in range(41)]
    c.poly(pts + [(560, 1180), (40, 1180)], SAND)
    for j in (0.45, 0.7):
        c.line([(300 - 240 * j + 480 * j * i / 20, 1150 - 70 * j * math.sin(math.pi * i / 20) ** 0.7) for i in range(2, 19)], SAND_D, 5)
    for x, y, r in ((70, 1050, 12), (110, 1010, 8), (520, 1040, 13), (480, 1000, 7), (150, 1060, 6), (455, 1062, 6)):
        c.ell(x, y, r, r, SAND)


def base():
    img = Image.new('RGBA', (W * S, H * S), (0, 0, 0, 0))
    for fn in (body, mound, head):
        outlined(img, fn)
    return img


def maw(c, scale=0.8, sq=1.0, dx=0, dy=0, tongue=False):
    cx, cy = 300 + dx, 318 + dy
    R = 122 * scale
    ry = R * sq
    c.ell(cx, cy, R + 20, ry + 20, OUT); c.ell(cx, cy, R + 15.5, ry + 15.5, LIP)
    c.ell(cx, cy, R + 3, ry + 3, OUT); c.ell(cx, cy, R, ry, MAW)
    c.ell(cx, cy, R * 0.42, ry * 0.42, THROAT)
    for ring, n, ln, ph in ((1.0, 18, 0.26, 0), (0.7, 12, 0.22, 0.26)):
        for k in range(n):
            a = 2 * math.pi * k / n + ph
            da = math.pi / n * 0.62
            p0 = (cx + R * ring * math.cos(a - da), cy + ry * ring * math.sin(a - da))
            p1 = (cx + R * ring * math.cos(a + da), cy + ry * ring * math.sin(a + da))
            p2 = (cx + R * (ring - ln) * math.cos(a), cy + ry * (ring - ln) * math.sin(a))
            c.poly([p0, p1, p2], TOOTH)
    if tongue:
        c.ell(cx + R * 0.2, cy + ry * 0.75, R * 0.34, ry * 0.42, OUT); c.ell(cx + R * 0.2, cy + ry * 0.75, R * 0.28, ry * 0.36, TONGUE)


def brow(c, side, inner, outer):
    cx = 300 + side * 86
    c.line([(cx - side * 28, 98 + inner * 0.7), (cx + side * 2, 92 + (inner + outer) * 0.35), (cx + side * 30, 100 + outer * 0.7)], (112, 72, 36), 9)


#            brows L, R                eyes     look     maw(scale, squash, dx, dy, tongue)  extras
MOODS = {
    'neutral':     ((0, 0), (0, 0), 'open', (0, 0), (0.72, 1.0, 0, 0, False), []),
    'happy':       ((-8, -6), (-8, -6), 'open', (0, 0), (0.95, 1.0, 0, 0, False), ['sparkle']),
    'laughing':    ((-12, -8), (-12, -8), 'happy', (0, 0), (1.12, 1.0, 0, 6, True), ['joytear']),
    'sad':         ((-14, 8), (-14, 8), 'half', (0, 8), (0.5, 0.7, 0, 20, False), []),
    'crying':      ((-16, 10), (-16, 10), 'soft', (0, 0), (0.85, 1.1, 0, 14, False), ['tears']),
    'angry':       ((16, -8), (16, -8), 'half', (0, 2), (0.82, 0.8, 0, 6, False), []),
    'furious':     ((22, -12), (22, -12), 'wide', (0, 0), (1.18, 1.0, 0, 8, False), ['vein', 'rage']),
    'surprised':   ((-22, -20), (-22, -20), 'wide', (0, 0), (1.05, 1.15, 0, 10, False), ['pop']),
    'scared':      ((-18, 4), (-18, 4), 'wide', (0, 2), (0.62, 1.25, 0, 14, False), ['sweat']),
    'disgusted':   ((12, -4), (-6, 2), 'half', (-8, 4), (0.5, 0.8, -14, 16, True), []),
    'confused':    ((10, -2), (-18, -14), 'open', (6, -4), (0.42, 1.0, 26, 14, False), ['question']),
    'thinking':    ((4, 0), (-16, -14), 'open', (9, -10), (0.34, 0.9, 34, 16, False), ['dots']),
    'smug':        ((2, 0), (-14, -12), 'half', (8, 2), (0.5, 0.6, 22, 10, False), []),
    'embarrassed': ((-12, 6), (-12, 6), 'open', (-9, 8), (0.36, 1.0, 0, 16, False), ['blush', 'sweat']),
    'denial':      ((-10, -10), (-10, -10), 'shut', (0, 0), (0.3, 0.8, 0, 16, False), []),
    'bargaining':  ((-18, 2), (-18, 2), 'puppy', (0, -3), (0.44, 1.0, 0, 14, False), ['sparkle']),
    'acceptance':  ((-4, 0), (-4, 0), 'soft', (0, 0), (0.56, 0.9, 0, 8, False), ['blush']),
    'dizzy':       ((-10, 0), (-10, 0), 'swirl', (0, 0), (0.8, 1.0, 0, 6, True), ['stars']),
}


def face(img, mood, ox=0, oy=0):
    bl, br, ek, look, mw, ex = MOODS[mood]
    lay = Image.new('RGBA', img.size, (0, 0, 0, 0))
    c = C(lay, ox, oy)
    if 'rage' in ex:
        c.ell(300, 300, 170, 172, (232, 90, 60, 80))
    if 'blush' in ex:
        for sx in (-1, 1):
            c.ell(300 + sx * 150, 250, 26, 15, (240, 120, 100, 200))
    if 'tears' in ex:
        for sx in (-1, 1):
            c.line(bez((300 + sx * 96, 186), (300 + sx * 150, 230), (300 + sx * 176, 300), (300 + sx * 168, 372), 16), TEAR, 13)
    sg.EX, sg.EY, sg.EDX = 300, 152, 86
    sg.SKIN = BODY_L
    sg.eye(c, -1, ek, look); sg.eye(c, 1, ek, look)
    brow(c, -1, *bl); brow(c, 1, *br)
    maw(c, *mw)
    if 'tears' in ex:
        drop(c, 130, 392, 10); drop(c, 470, 400, 9)
    if 'joytear' in ex:
        drop(c, 150, 180, 9); drop(c, 450, 180, 9)
    if 'sweat' in ex:
        drop(c, 470, 110, 12)
    if 'vein' in ex:
        for a in (0, 90, 180, 270):
            x0 = 476 + 16 * math.cos(math.radians(a + 45)); y0 = 96 + 16 * math.sin(math.radians(a + 45))
            c.arc(x0, y0, 9, 9, a + 135, a + 315, RED, 4.5)
    if 'sparkle' in ex:
        star(c, 478, 108, 16, YEL); star(c, 112, 132, 11, YEL); star(c, 500, 150, 8, WHITE)
    if 'pop' in ex:
        for x0, y0, x1, y1 in ((478, 96, 504, 74), (492, 132, 524, 126), (122, 96, 96, 74)):
            c.line([(x0, y0), (x1, y1)], OUT, 5)
    if 'question' in ex:
        q = arcpts(490, 78, 17, 17, 180, 400) + [(490, 106), (490, 118)]
        c.line(q, OUT, 12); c.line(q, YEL, 6.5)
        c.ell(490, 138, 7.5, 7.5, OUT); c.ell(490, 138, 4.5, 4.5, YEL)
    if 'dots' in ex:
        for x, y, r in ((468, 118, 6), (490, 92, 9), (518, 60, 13)):
            c.ell(x, y, r + 3, r + 3, OUT); c.ell(x, y, r, r, WHITE)
    if 'stars' in ex:
        for i in range(5):
            a = math.radians(200 + i * 35)
            star(c, 300 + 190 * math.cos(a), 110 + 60 * math.sin(a), 13 if i % 2 == 0 else 9, YEL)
    img.alpha_composite(lay)


def main(out, scene_out):
    os.makedirs(out, exist_ok=True)
    b = base()
    PORT = (30, 40, 570, 580); AV = (80, 50, 520, 490)
    for mood in MOODS:
        if mood == 'neutral':
            im = b.copy(); face(im, mood)
            im.resize((W, H), Image.LANCZOS).save(f'{out}/sandy_full-neutral.webp', 'WEBP', quality=92, method=6)
            im.crop([v * S for v in AV]).resize((600, 600), Image.LANCZOS).save(f'{out}/sandy_avatar-neutral.webp', 'WEBP', quality=92, method=6)
            p = im.crop([v * S for v in PORT])
        elif mood == 'happy':
            im = b.copy(); face(im, mood)
            im.resize((W, H), Image.LANCZOS).save(f'{out}/sandy_full-happy.webp', 'WEBP', quality=92, method=6)
            p = im.crop([v * S for v in PORT])
        else:
            p = b.crop([v * S for v in PORT]); face(p, mood, PORT[0], PORT[1])
        p.resize((600, 600), Image.LANCZOS).save(f'{out}/sandy_portrait-{mood}.webp', 'WEBP', quality=92, method=6)
        print(mood)
    sheet = Image.new('RGB', (6 * 300, 3 * 300), (40, 44, 52))
    for i, mood in enumerate(MOODS):
        t = Image.open(f'{out}/sandy_portrait-{mood}.webp').resize((300, 300), Image.LANCZOS)
        sheet.paste(t, ((i % 6) * 300, (i // 6) * 300), t)
    sheet.save(f'{out}/_sheet.png')
    f = Image.open(f'{out}/sandy_full-neutral.webp')
    bg = Image.new('RGB', f.size, (40, 44, 52)); bg.paste(f, (0, 0), f); bg.save(f'{out}/_full.png')
    # a chapter slice: Sandy rises behind Sandgirl
    Wd, Hd, k = 390, 624, 6
    img, r = sc.backdrop(Wd, Hd, k, 0.36, 3.3, (0.82, 0.12))
    worm = Image.open(f'{out}/sandy_full-happy.webp').convert('RGBA')
    g = worm.resize((int(330 * k), int(660 * k)), Image.LANCZOS)
    img.alpha_composite(g, (int(80 * k), int(40 * k)))
    girl = Image.open(f'{os.path.dirname(out)}/sandgirl/sandgirl_full-neutral.webp').convert('RGBA')
    sc.paste_girl(img, k, girl, 72, 606, 250)
    sc.save_variants(img, scene_out, 's1c1-slice-5', Wd, Hd)
    Image.open(f'{scene_out}/s1c1-slice-5.webp').save(f'{out}/_slice5.png')


if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2])
