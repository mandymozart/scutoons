"""Black-and-white outline of a character image for the storyboard schematics (owner 2026-10-09: "more detailed bw
outlines style"). Lines from a difference of Gaussians on the luminance + the cut-out's silhouette edge; transparent
background, black lines. Pillow only (the cdn/ tools use it too).

    python scripts/storyboard/outline.py <source image> <out.png> [--height 800] [--mirror] [--crop x0,y0,x1,y1]

--crop takes fractions of the source (0…1), e.g. the eyes of a portrait for an extreme close-up.
"""
import sys

from PIL import Image, ImageChops, ImageFilter, ImageOps


def outline(src, height=800, mirror=False, crop=None):
    im = Image.open(src).convert('RGBA')
    if crop:
        w, h = im.size
        x0, y0, x1, y1 = crop
        im = im.crop((int(x0 * w), int(y0 * h), int(x1 * w), int(y1 * h)))
    if mirror:
        im = ImageOps.mirror(im)
    # work at a fixed scale so the line weight is the same for every figure, then scale to the target
    work = 1000
    im = im.resize((max(1, round(im.width * work / im.height)), work), Image.LANCZOS)
    flat = Image.new('RGBA', im.size, (255, 255, 255, 255))
    flat.alpha_composite(im)
    g = flat.convert('L')
    fine = g.filter(ImageFilter.GaussianBlur(1.2))
    wide = g.filter(ImageFilter.GaussianBlur(3.5))
    lines = ImageChops.subtract(wide, fine).point(lambda v: 255 if v > 7 else 0)
    alpha = im.split()[3].point(lambda v: 255 if v > 20 else 0)
    edge = alpha.filter(ImageFilter.FIND_EDGES).filter(ImageFilter.MaxFilter(5))
    ink = ImageChops.lighter(lines, edge).filter(ImageFilter.MedianFilter(3))
    # inside the figure a little white so it covers what is behind it (a cut-out, like the toon's collages)
    paper = alpha.filter(ImageFilter.MinFilter(3)).point(lambda v: 235 if v else 0)
    target_h = max(8, int(height))
    size = (max(1, round(im.width * target_h / work)), target_h)
    ink = ink.resize(size, Image.LANCZOS).point(lambda v: 255 if v > 90 else 0)
    paper = paper.resize(size, Image.LANCZOS)
    out = Image.new('RGBA', size, (255, 255, 255, 0))
    white = Image.new('RGBA', size, (255, 255, 255, 255))
    out = Image.composite(white, out, paper)
    black = Image.new('RGBA', size, (20, 20, 18, 255))
    out = Image.composite(black, out, ink)
    return out


def main(argv):
    if len(argv) < 2:
        print(__doc__)
        return 2
    src, dst = argv[0], argv[1]
    height, mirror, crop = 800, False, None
    rest = argv[2:]
    while rest:
        a = rest.pop(0)
        if a == '--height':
            height = int(rest.pop(0))
        elif a == '--mirror':
            mirror = True
        elif a == '--crop':
            crop = tuple(float(v) for v in rest.pop(0).split(','))
    outline(src, height, mirror, crop).save(dst, optimize=True)
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
