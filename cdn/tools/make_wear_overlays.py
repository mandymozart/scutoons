"""Wear overlays for the reader's avatar wardrobe (agents/ITEMS.md "Wardrobe").

A worn clothing item is drawn as a full-body overlay aligned to the body's A-pose: same 768 x 1152 canvas, transparent
WebP, only the garment's pixels. This tool makes the PLACEHOLDER overlays of the default wearables from the A-pose art
that exists today; the owner replaces them later with the final branded artwork under the same names.

Outputs (from the CDN root, cdn/root is not in git - upload with cdn/sync.py):
  characters/<character>/wear/<item>.webp                    single figure (the item's slot instance = its type)
  characters/<character>/wear/<item>-<slot instance>.webp     multi-figure bodies (Top Toy: legs-left, legs-right)
  scutoons/items/<item>.png                                   the item's inventory icon (512 px cut-out, like
                                                              make_item_icons.py)

Methods (quality notes in agents/ITEMS.md "Wardrobe"):
  swim pants   = pixel difference between <character>_apose-swim and <character>_apose-naked (both normalized with one
                 transform, so they overlay exactly): the per-pixel colour distance (alpha-composited on grey) is
                 blurred (sigma 2, averages out fur/texture noise of the re-rendered body), hysteresis threshold
                 (strong 60 / weak 22, weak regions kept only when connected to a strong one - catches the low-contrast
                 white waistband), holes filled, specks < 1500 px dropped, dilated 2 px, feathered 1 px, cut to the swim
                 file's own alpha; colours = the swim file's pixels. Twins: the mask is split at the canvas centre.
  T-shirt      = colour segmentation of the dressed reference catpope_apose-example-dressed.webp: low saturation
                 (S < 0.075, the shirt is neutral grey/white, the sphynx skin is S 0.10-0.20 at hue 8-17 deg), V > 0.3,
                 inside the torso band, largest component, holes filled, closed, feathered.
  shorts       = the same reference, hue 190-250 deg and S > 0.15 (denim blue), inside the hip band.
  sandals      = the same reference, brown/cork (hue 5-45 deg, S > 0.22) below y 950, one blob per side, holes filled
                 (the reference's toes come along), each moved + scaled onto the naked A-pose's foot (see sandals()).
  The reference's upper body lines up with catpope_apose-naked (checked as an anaglyph: head, shoulders, arms within
  1-3 px), its legs don't (wider stance, feet turned out and higher) - the shorts hang over the naked thighs, which are
  narrower than the hems, so they read correctly; the sandals are re-placed per foot.

Usage (from the repo root, with the cdn venv):
  cdn/.venv/Scripts/python cdn/tools/make_wear_overlays.py [--root <cdn root>] [--sheet <png>]
"""

from __future__ import annotations

import argparse
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter
from scipy import ndimage

W, H = 768, 1152
ICON = 512
PADDING = 0.06

# what exists: (item id, character, slot instances or None for a single figure, method)
SWIM = [
    ("catpope", None),
    ("yellowmouse", None),
    ("toptoy", ("legs-left", "legs-right")),
]
REFERENCE = "catpope/catpope_apose-example-dressed.webp"


def load(root: Path, rel: str) -> np.ndarray:
    im = Image.open(root / "characters" / rel).convert("RGBA")
    if im.size != (W, H):
        raise SystemExit(f"{rel}: {im.size}, expected {W} x {H}")
    return np.asarray(im).astype(np.float32)


def on_grey(a: np.ndarray) -> np.ndarray:
    al = a[..., 3:4] / 255.0
    return a[..., :3] * al + 128.0 * (1 - al)


def hsv(a: np.ndarray) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    rgb = a[..., :3] / 255.0
    mx, mn = rgb.max(-1), rgb.min(-1)
    v = mx
    s = np.where(mx > 0, (mx - mn) / np.maximum(mx, 1e-6), 0)
    r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    d = np.maximum(mx - mn, 1e-6)
    h = np.where(mx == r, ((g - b) / d) % 6, np.where(mx == g, (b - r) / d + 2, (r - g) / d + 4)) * 60
    return h, s, v


def clean(mask: np.ndarray, min_area: int, keep_largest: int | None = None) -> np.ndarray:
    lab, n = ndimage.label(mask)
    if n == 0:
        return mask
    areas = ndimage.sum(mask, lab, range(1, n + 1))
    order = np.argsort(areas)[::-1]
    keep = [i + 1 for i in order if areas[i] >= min_area]
    if keep_largest:
        keep = keep[:keep_largest]
    return np.isin(lab, keep)


def finish(mask: np.ndarray, src: np.ndarray, dilate: int = 2, feather: float = 1.0) -> Image.Image:
    """mask → RGBA overlay: dilated, feathered, cut to the source alpha, colours from the source."""
    m = ndimage.binary_fill_holes(mask)
    if dilate:
        m = ndimage.binary_dilation(m, iterations=dilate)
    alpha = Image.fromarray((m * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(feather))
    a = np.asarray(alpha).astype(np.float32) / 255.0 * src[..., 3]
    out = np.dstack([src[..., :3], a]).clip(0, 255).astype(np.uint8)
    out[out[..., 3] < 2] = 0
    return Image.fromarray(out, "RGBA")


def swim_pants(root: Path, character: str) -> np.ndarray:
    s = load(root, f"{character}/{character}_apose-swim.webp")
    n = load(root, f"{character}/{character}_apose-naked.webp")
    # compare the blurred images (fur / skin texture of the re-rendered body averages out, a garment's colour doesn't)
    blur = lambda a: np.dstack([ndimage.gaussian_filter(on_grey(a)[..., c], 2.5) for c in range(3)])  # noqa: E731
    d = np.abs(blur(s) - blur(n)).max(-1)
    strong, weak = d > 60, d > 24
    lab, _ = ndimage.label(weak)
    seeds = np.unique(lab[strong & (lab > 0)])
    mask = np.isin(lab, seeds[seeds > 0]) & (s[..., 3] > 40)
    # thin attachments (a tail drawn slightly differently, outline drift) go: open wide, keep the biggest blob per
    # figure half, then grow back over the weak pixels near it (restores the garment's edge, not the tail)
    core = ndimage.binary_opening(mask, iterations=5)
    halves = []
    for lo, hi in ((0, W // 2), (W // 2, W)):
        part = np.zeros_like(core); part[:, lo:hi] = core[:, lo:hi]
        halves.append(clean(part, 1500, keep_largest=1))
    core = halves[0] | halves[1]
    mask = mask & ndimage.binary_dilation(core, iterations=6)
    mask = clean(ndimage.binary_fill_holes(mask), 1500)
    return mask, s


def reference_masks(root: Path) -> tuple[dict[str, np.ndarray], np.ndarray]:
    d = load(root, REFERENCE)
    h, s, v = hsv(d)
    opaque = d[..., 3] > 128
    rows = np.arange(H)[:, None] * np.ones((1, W))
    shirt = opaque & (s < 0.075) & (v > 0.3) & (rows > 170) & (rows < 520)
    shirt = ndimage.binary_closing(ndimage.binary_opening(shirt, iterations=2), iterations=3)
    shirt = clean(ndimage.binary_fill_holes(shirt), 2000, keep_largest=1)
    shorts = opaque & (h > 190) & (h < 250) & (s > 0.15) & (rows > 420) & (rows < 760)
    shorts = ndimage.binary_closing(ndimage.binary_opening(shorts, iterations=1), iterations=3)
    shorts = clean(ndimage.binary_fill_holes(shorts), 2000, keep_largest=2)
    return {"scutoons_tshirt": shirt, "scutoons_shorts": shorts}, d


def sandals(root: Path) -> Image.Image:
    """The reference's sandals (straps, footbed, sole; brown/cork, below the ankles), each one moved and scaled onto
    the naked A-pose's foot: the reference stands wider, its feet turned out and 20 px higher, so a plain cut would
    float beside the feet. Per side: the sandal's box is scaled to the naked foot's width + 24 px, centred on the foot,
    its sole on the foot's sole. The holes between the straps are filled, so the reference's own toes come along and
    cover the cat's claws (reads as a foot in a sandal; the sole edge keeps a few ragged shadow pixels)."""
    d = load(root, REFERENCE)
    n = load(root, "catpope/catpope_apose-naked.webp")
    h, s, v = hsv(d)
    rows = np.arange(H)[:, None] * np.ones((1, W))
    brown = (d[..., 3] > 128) & (h >= 5) & (h <= 45) & (s > 0.22) & (v < 0.85) & (rows > 950)
    brown = ndimage.binary_closing(ndimage.binary_opening(brown, iterations=1), iterations=2)
    out = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    for lo, hi in ((0, W // 2), (W // 2, W)):
        part = np.zeros_like(brown); part[:, lo:hi] = brown[:, lo:hi]
        part = clean(part, 800, keep_largest=1)
        if not part.any():
            continue
        sandal = finish(part, d, dilate=1)
        box = Image.fromarray((part * 255).astype(np.uint8)).getbbox()
        sandal = sandal.crop(box)
        foot = np.zeros((H, W), bool); foot[960:, lo:hi] = n[960:, lo:hi, 3] > 128
        fx0, fy0, fx1, fy1 = Image.fromarray((foot * 255).astype(np.uint8)).getbbox()
        scale = (fx1 - fx0 + 24) / sandal.width
        size = (round(sandal.width * scale), round(sandal.height * scale))
        sandal = sandal.convert("RGBa").resize(size, Image.Resampling.LANCZOS).convert("RGBA")
        cx = (fx0 + fx1) // 2
        out.alpha_composite(sandal, (cx - size[0] // 2, fy1 - size[1] + 2))
    return out


def icon_of(im: Image.Image) -> Image.Image:
    bbox = im.getchannel("A").point(lambda v: 255 if v > 6 else 0).getbbox()
    im = im.crop(bbox)
    inner = round(ICON * (1 - 2 * PADDING))
    scale = inner / max(im.size)
    size = (max(1, round(im.width * scale)), max(1, round(im.height * scale)))
    im = im.convert("RGBa").resize(size, Image.Resampling.LANCZOS).convert("RGBA")
    canvas = Image.new("RGBA", (ICON, ICON), (0, 0, 0, 0))
    canvas.alpha_composite(im, ((ICON - size[0]) // 2, (ICON - size[1]) // 2))
    arr = np.asarray(canvas).copy()
    arr[arr[..., 3] == 0] = 0
    return Image.fromarray(arr, "RGBA")


def save_webp(im: Image.Image, path: Path) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    im.save(path, "WEBP", quality=90, method=6, exact=False)
    return path


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default=str(Path(__file__).resolve().parent.parent / "root"))
    ap.add_argument("--sheet", default=None, help="QA sheet: naked A-pose + overlays per body")
    args = ap.parse_args()
    root = Path(args.root)
    made: dict[str, list[Path]] = {}

    for character, twins in SWIM:
        mask, src = swim_pants(root, character)
        if twins:
            left = mask.copy(); left[:, W // 2:] = False
            right = mask.copy(); right[:, : W // 2] = False
            parts = {twins[0]: left, twins[1]: right}
        else:
            parts = {None: mask}
        for inst, m in parts.items():
            name = f"scutoons_swim_pants{'-' + inst if inst else ''}.webp"
            p = save_webp(finish(m, src), root / "characters" / character / "wear" / name)
            made.setdefault(character, []).append(p)
            print(f"{p.relative_to(root)}  {int(m.sum())} px  {p.stat().st_size // 1024} KB")

    masks, ref = reference_masks(root)
    for item, m in masks.items():
        p = save_webp(finish(m, ref, dilate=1), root / "characters" / "catpope" / "wear" / f"{item}.webp")
        made.setdefault("catpope", []).append(p)
        print(f"{p.relative_to(root)}  {int(m.sum())} px  {p.stat().st_size // 1024} KB")

    p = save_webp(sandals(root), root / "characters" / "catpope" / "wear" / "scutoons_sandals.webp")
    made.setdefault("catpope", []).append(p)
    print(f"{p.relative_to(root)}  {p.stat().st_size // 1024} KB")

    # icons: the catpope overlays cut out (the garment alone)
    icons = root / "scutoons" / "items"
    icons.mkdir(parents=True, exist_ok=True)
    for item in ("scutoons_tshirt", "scutoons_swim_pants", "scutoons_shorts", "scutoons_sandals"):
        overlay = Image.open(root / "characters" / "catpope" / "wear" / f"{item}.webp").convert("RGBA")
        p = icons / f"{item}.png"
        icon_of(overlay).save(p, optimize=True, compress_level=9)
        print(f"{p.relative_to(root)}  {p.stat().st_size // 1024} KB")

    if args.sheet:
        order = ["scutoons_swim_pants", "scutoons_shorts", "scutoons_sandals", "scutoons_tshirt"]
        tiles = []
        for character, files in made.items():
            base = Image.open(root / "characters" / character / f"{character}_apose-naked.webp").convert("RGBA")
            combos = [["scutoons_swim_pants"], ["scutoons_swim_pants", "scutoons_tshirt"], ["scutoons_shorts", "scutoons_sandals", "scutoons_tshirt"]]
            for combo in combos:
                layers = [f for o in order if o in combo for f in files if f.stem.split("-")[0] == o]
                if not layers:
                    continue
                t = Image.new("RGBA", (W, H), (205, 225, 245, 255))
                t.alpha_composite(base)
                for f in layers:
                    t.alpha_composite(Image.open(f).convert("RGBA"))
                ImageDraw.Draw(t).text((10, 10), f"{character}: {' + '.join(combo)}", fill="black")
                tiles.append(t.resize((W // 2, H // 2)))
        sheet = Image.new("RGB", (W // 2 * len(tiles), H // 2), "white")
        for i, t in enumerate(tiles):
            sheet.paste(t.convert("RGB"), (i * W // 2, 0))
        sheet.save(args.sheet)
        print("sheet", args.sheet)


if __name__ == "__main__":
    main()
