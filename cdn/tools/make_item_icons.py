"""Generate inventory item icons (transparent cut-outs) for the SCUTOONS item catalog.

Output: cdn/root/<toon>/items/<item_id>.png and <item_id>_<variant>.png
        512x512, sRGB, transparent background, item trimmed + centred with ~6% padding.

Sources:
  - physical merch: product photos in cdn/root/catpope/merch/ (local shoot) and
    cdn/root/catpope/merch/bandcamp/ (originals from mandymozart.bandcamp.com, f4.bcbits.com)
  - virtual items: inline SVG drawn here (camera, cucumber, cucumber_camera)

Background removal: rembg (isnet-general-use for garments, birefnet-general for objects) for the fine alpha; for garments that are worn in the
photo, the alpha is additionally multiplied with the "upper cloth" mask of u2net_cloth_seg so head,
hands, legs and the stool disappear. Then: per-source exclusion boxes, keep only the main connected
components, fill pin holes, 1px alpha erosion + colour decontamination (no halos), trim, fit, save.

Usage (from cdn/):
  .venv/Scripts/python tools/make_item_icons.py            # all icons
  .venv/Scripts/python tools/make_item_icons.py dago_hoodie_black camera   # only these outputs
  .venv/Scripts/python tools/make_item_icons.py --sheet out.png            # + QA contact sheet

Requirements: see tools/requirements.txt (rembg[cpu], pillow, numpy, scipy, resvg-py).
Models are downloaded by rembg into ~/.rembg on first run.
"""

from __future__ import annotations

import io
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageCms, ImageDraw, ImageOps
from scipy import ndimage

CDN_ROOT = Path(__file__).resolve().parent.parent / "root"
MERCH = CDN_ROOT / "catpope" / "merch"
BANDCAMP = MERCH / "bandcamp"
OUT_DIR = CDN_ROOT / "catpope" / "items"

SIZE = 512
PADDING = 0.06  # per side
WORK_MAX = 1600  # working resolution (long edge) for segmentation

# ---------------------------------------------------------------------------------------------
# Source choice per output icon. Coordinates in "crop"/"exclude" are fractions of the (cropped)
# image: (left, top, right, bottom). mode "cloth" = fine alpha x upper-cloth mask (worn garments),
# "object" = fine alpha only. "copy" = base icon duplicates another output.
# ---------------------------------------------------------------------------------------------
PHOTO_ICONS: dict[str, dict] = {
    # Cat Pope Dago Hoodie — no flat lays exist, only worn shots. Front views leave a face-shaped hole
    # in the hood (the mask is cut out), so the back views on the stool are used: complete silhouette
    # incl. hood, and they show the signature back print.
    "dago_hoodie_black": {"src": MERCH / "1_catpope_dago_hoodie_black_web-32.jpg", "mode": "cloth"},
    "dago_hoodie_beige": {"src": MERCH / "1_catpope_dago_hoodie_beige_web-49.jpg", "mode": "cloth",
                          # legs/socks between the sleeves below the hem
                          "exclude": [(0.36, 0.73, 0.665, 1.0), (0.36, 0.68, 0.45, 0.73), (0.60, 0.68, 0.665, 0.73)]},
    "dago_hoodie_grey": {"src": MERCH / "1_catpope_dago_hoodie_grey_web-54.jpg", "mode": "cloth",
                         "exclude": [(0.368, 0.69, 0.625, 1.0)]},
    # Cat Pope Dago Heavy T — front views (back views: cloth mask eats the white back print)
    # Cat Pope Dago Heavy T
    "dago_heavy_t_black": {"src": MERCH / "1_catpope_dago_heavy_t_black_web-7.jpg", "mode": "cloth"},
    "dago_heavy_t_white": {"src": MERCH / "1_catpope_dago_heavy_t_white_web-1.jpg", "mode": "cloth"},
    # Bandcamp-only items
    "scorpion_amulet_purple": {"src": BANDCAMP / "scorpion_amulet_0046916043.jpg", "mode": "object", "model": "birefnet-general"},
    "awakening_vinyl": {"src": BANDCAMP / "awakening_vinyl_0046915732.jpg", "mode": "object", "model": "birefnet-general"},
    "awakening_test_press_1_of_2": {"src": BANDCAMP / "awakening_test_press_1_of_2_0046916239.jpg", "mode": "object", "model": "birefnet-general"},
    "awakening_test_press_2_of_2": {"src": BANDCAMP / "awakening_test_press_2_of_2_0046916279.jpg", "mode": "object", "model": "birefnet-general"},
    # flat on a bed, garment runs off the photo edges (only source available)
    "kangaroo_cat_hoodie": {"src": BANDCAMP / "kangaroo_cat_hoodie_0044221026.jpg", "mode": "object", "model": "birefnet-general"},
}

# base icon (<item_id>.png) = most representative variant
BASE_COPIES: dict[str, str] = {
    "dago_hoodie": "dago_hoodie_beige",  # the colour that is still available / featured
    "dago_heavy_t": "dago_heavy_t_white",  # available + featured
    "scorpion_amulet": "scorpion_amulet_purple",
    "awakening_test_press": "awakening_test_press_1_of_2",
}

# The cucumber is drawn as SVG below in the same style as the camera icons (the old 124x124 VN art in
# images/collectables/ was too small and is no longer referenced).

INK = "#000000"
PAPER = "#ffffff"
CYAN = "#00d1ff"
CUKE_DARK = "#2f8f2a"
CUKE = "#5cc23a"
CUKE_FLESH = "#d8f5a2"


def _camera_svg(cucumber: bool) -> str:
    body_fill = CUKE if cucumber else PAPER
    top_fill = CUKE_DARK if cucumber else PAPER
    if cucumber:
        # cucumber slice lens: dark skin ring, pale flesh, seeds in a ring, cyan glint
        seeds = "".join(
            f'<ellipse cx="{256 + 38 * np.cos(a):.1f}" cy="{292 + 38 * np.sin(a):.1f}" rx="7" ry="12" '
            f'transform="rotate({np.degrees(a) + 90:.1f} {256 + 38 * np.cos(a):.1f} {292 + 38 * np.sin(a):.1f})" '
            f'fill="{PAPER}" stroke="{CUKE_DARK}" stroke-width="3"/>'
            for a in np.linspace(0, 2 * np.pi, 7)[:-1]
        )
        lens = (
            f'<circle cx="256" cy="292" r="104" fill="{CUKE_DARK}" stroke="{INK}" stroke-width="18"/>'
            f'<circle cx="256" cy="292" r="80" fill="{CUKE_FLESH}" stroke="{CUKE}" stroke-width="8"/>'
            f"{seeds}"
            f'<circle cx="256" cy="292" r="10" fill="{CUKE}"/>'
            f'<path d="M204 250 A64 64 0 0 1 240 226" fill="none" stroke="{CYAN}" stroke-width="12" stroke-linecap="round"/>'
        )
        # whole cucumber as the carrying strap, arching over the camera
        bumps = "".join(
            f'<circle cx="{x}" cy="{y}" r="5" fill="{CUKE_FLESH}"/>'
            for x, y in [(118, 118), (170, 84), (230, 68), (290, 68), (350, 84), (400, 116)]
        )
        strap = (
            f'<path d="M84 196 C 110 70, 402 50, 428 196" fill="none" stroke="{INK}" stroke-width="58" stroke-linecap="round"/>'
            f'<path d="M84 196 C 110 70, 402 50, 428 196" fill="none" stroke="{CUKE}" stroke-width="30" stroke-linecap="round"/>'
            f"{bumps}"
        )
    else:
        lens = (
            f'<circle cx="256" cy="292" r="104" fill="{PAPER}" stroke="{INK}" stroke-width="18"/>'
            f'<circle cx="256" cy="292" r="66" fill="{CYAN}" stroke="{INK}" stroke-width="14"/>'
            f'<circle cx="256" cy="292" r="26" fill="{INK}"/>'
            f'<path d="M214 262 A50 50 0 0 1 244 244" fill="none" stroke="{PAPER}" stroke-width="12" stroke-linecap="round"/>'
        )
        strap = (
            f'<path d="M60 206 L 60 170" stroke="{INK}" stroke-width="18" stroke-linecap="square"/>'
            f'<path d="M452 206 L 452 170" stroke="{INK}" stroke-width="18" stroke-linecap="square"/>'
        )
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  {strap}
  <rect x="150" y="146" width="130" height="58" fill="{top_fill}" stroke="{INK}" stroke-width="18" stroke-linejoin="miter"/>
  <rect x="40" y="186" width="432" height="232" fill="{body_fill}" stroke="{INK}" stroke-width="18" stroke-linejoin="miter"/>
  <rect x="352" y="160" width="62" height="26" fill="{INK}"/>
  <rect x="70" y="214" width="56" height="30" fill="{CYAN}" stroke="{INK}" stroke-width="10"/>
  <path d="M40 360 H 150 M362 360 H 472" stroke="{INK}" stroke-width="10"/>
  {lens}
</svg>"""


def _cucumber_svg() -> str:
    warts = "".join(
        f'<circle cx="{x}" cy="{y}" r="7" fill="{CUKE_FLESH}" stroke="{CUKE_DARK}" stroke-width="3"/>'
        for x, y in [(150, 330), (196, 280), (246, 238), (300, 196), (352, 158), (200, 330), (252, 290),
                     (304, 246), (356, 206), (130, 378)]
    )
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <g transform="rotate(-38 256 256)">
    <rect x="46" y="186" width="420" height="140" rx="70" fill="{CUKE}" stroke="{INK}" stroke-width="18"/>
    <path d="M110 222 H 400" stroke="{CUKE_FLESH}" stroke-width="14" stroke-linecap="round" opacity="0.8"/>
    <path d="M96 290 H 410" stroke="{CUKE_DARK}" stroke-width="12" stroke-linecap="round"/>
    <ellipse cx="452" cy="256" rx="16" ry="30" fill="{CUKE_FLESH}" stroke="{INK}" stroke-width="10"/>
    <path d="M46 256 H 22" stroke="{INK}" stroke-width="18" stroke-linecap="square"/>
  </g>
  {warts}
</svg>"""


SVG_ICONS: dict[str, str] = {
    "cucumber": _cucumber_svg(),
    "camera": _camera_svg(cucumber=False),
    "cucumber_camera": _camera_svg(cucumber=True),
}

# ---------------------------------------------------------------------------------------------

_sessions: dict[str, object] = {}


def _session(name: str):
    if name not in _sessions:
        from rembg import new_session

        _sessions[name] = new_session(name)
    return _sessions[name]


SRGB = ImageCms.createProfile("sRGB")


def load_rgb(path: Path) -> Image.Image:
    im = Image.open(path)
    im = ImageOps.exif_transpose(im)
    icc = im.info.get("icc_profile")
    if icc:
        try:
            src = ImageCms.ImageCmsProfile(io.BytesIO(icc))
            im = ImageCms.profileToProfile(im.convert("RGB"), src, SRGB, outputMode="RGB")
        except Exception:  # broken profile -> assume sRGB
            pass
    return im.convert("RGB")


def frac_box(box, w, h):
    l, t, r, b = box
    return int(l * w), int(t * h), int(r * w), int(b * h)


def cutout(spec: dict) -> Image.Image:
    from rembg import remove

    im = load_rgb(spec["src"])
    if "crop" in spec:
        im = im.crop(frac_box(spec["crop"], *im.size))
    im.thumbnail((WORK_MAX, WORK_MAX), Image.Resampling.LANCZOS)
    w, h = im.size

    alpha = np.asarray(remove(im, session=_session(spec.get("model", "isnet-general-use")), only_mask=True), np.float32) / 255
    if spec["mode"] == "cloth":
        cloth = _session("u2net_cloth_seg").predict(im, cc="upper")[0]
        cloth = np.asarray(cloth, np.float32) / 255
        cloth = ndimage.gaussian_filter(cloth, 1.2)
        alpha = alpha * np.clip(cloth * 1.15, 0, 1)
        # legs (jeans) sometimes leak into the upper mask -> subtract the lower-cloth mask
        lower = np.asarray(_session("u2net_cloth_seg").predict(im, cc="lower")[0], np.float32) / 255
        lower = ndimage.gaussian_filter(ndimage.grey_dilation(lower, size=(9, 9)), 2)
        alpha = alpha * (1 - np.clip(lower, 0, 1))
    for box in spec.get("exclude", []):
        l, t, r, b = frac_box(box, w, h)
        alpha[t:b, l:r] = 0

    # keep the main connected components only
    solid = alpha > 0.5
    labels, n = ndimage.label(solid)
    if n:
        areas = ndimage.sum(solid, labels, range(1, n + 1))
        keep_ids = [i + 1 for i, a in enumerate(areas) if a >= 0.04 * areas.max()]
        keep = np.isin(labels, keep_ids)
        # fill pin holes (< 0.2% of the object), leave big holes (hood openings, record holes) alone
        holes = ndimage.binary_fill_holes(keep) & ~keep
        hl, hn = ndimage.label(holes)
        if hn:
            hareas = ndimage.sum(holes, hl, range(1, hn + 1))
            small = np.isin(hl, [i + 1 for i, a in enumerate(hareas) if a < 0.002 * keep.sum()])
            keep |= small
            alpha = np.where(small, 1.0, alpha)
        near = ndimage.binary_dilation(keep, iterations=3)
        alpha = alpha * near

    # anti-halo: 1px erosion of the soft edge
    alpha = np.minimum(alpha, ndimage.grey_erosion(alpha, size=(3, 3)) * 0.5 + alpha * 0.5)
    alpha[alpha < 0.03] = 0

    # colour decontamination: edge pixels take the colour of the nearest solid pixel
    rgb = np.asarray(im, np.float32)
    solid = alpha > 0.9
    if solid.any():
        _, (iy, ix) = ndimage.distance_transform_edt(~solid, return_indices=True)
        inner = rgb[iy, ix]
        t = np.clip((alpha - 0.3) / 0.6, 0, 1)[..., None]  # 0 -> use inner colour, 1 -> original
        rgb = rgb * t + inner * (1 - t)

    out = np.dstack([np.clip(rgb, 0, 255), alpha * 255]).astype(np.uint8)
    return Image.fromarray(out, "RGBA")


def fit(im: Image.Image) -> Image.Image:
    im = im.convert("RGBA")
    bbox = im.getchannel("A").point(lambda v: 255 if v > 6 else 0).getbbox()
    if bbox:
        im = im.crop(bbox)
    inner = round(SIZE * (1 - 2 * PADDING))
    scale = inner / max(im.size)
    new = (max(1, round(im.width * scale)), max(1, round(im.height * scale)))
    im = im.convert("RGBa").resize(new, Image.Resampling.LANCZOS).convert("RGBA")
    canvas = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    canvas.alpha_composite(im, ((SIZE - new[0]) // 2, (SIZE - new[1]) // 2))
    # fully transparent pixels -> black RGB (smaller files, no stray colours)
    arr = np.asarray(canvas).copy()
    arr[arr[..., 3] == 0] = 0
    return Image.fromarray(arr, "RGBA")


def save(im: Image.Image, name: str) -> Path:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    path = OUT_DIR / f"{name}.png"
    im.save(path, optimize=True, compress_level=9)
    return path


def render_svg(svg: str) -> Image.Image:
    import resvg_py

    png = resvg_py.svg_to_bytes(svg_string=svg, width=SIZE * 4, height=SIZE * 4)
    return Image.open(io.BytesIO(bytes(png))).convert("RGBA")


def build(only: set[str] | None = None) -> list[Path]:
    made: list[Path] = []
    wanted = lambda n: not only or n in only  # noqa: E731
    for name, spec in PHOTO_ICONS.items():
        if wanted(name) or any(wanted(b) for b, s in BASE_COPIES.items() if s == name):
            icon = fit(cutout(spec))
            made.append(save(icon, name))
            print("photo ", name, "<-", spec["src"].relative_to(CDN_ROOT))
            for base, src in BASE_COPIES.items():
                if src == name:
                    made.append(save(icon, base))
                    print("base  ", base, "=", name)
    for name, svg in SVG_ICONS.items():
        if wanted(name):
            made.append(save(fit(render_svg(svg)), name))
            print("svg   ", name)
    return made


def make_web_copies() -> None:
    """Shop-size JPGs of the Bandcamp originals (4000px, several MB) -> merch/bandcamp/web/<name>.jpg."""
    out_dir = BANDCAMP / "web"
    out_dir.mkdir(exist_ok=True)
    for src in sorted(BANDCAMP.glob("*.jpg")):
        out = out_dir / src.name
        if out.exists() and out.stat().st_mtime >= src.stat().st_mtime:
            continue
        im = load_rgb(src)
        im.thumbnail((1350, 1350), Image.Resampling.LANCZOS)
        im.save(out, quality=85, optimize=True, progressive=True)
        print("web   ", out.relative_to(CDN_ROOT))


def contact_sheet(paths: list[Path], out: Path) -> None:
    cell, label = 180, 18
    paths = sorted(set(paths))
    sheet = Image.new("RGB", (cell * 3 + 260, (cell + 4) * len(paths)), "white")
    d = ImageDraw.Draw(sheet)
    checker = Image.new("RGB", (cell, cell), "white")
    cd = ImageDraw.Draw(checker)
    for y in range(0, cell, 12):
        for x in range(0, cell, 12):
            if (x // 12 + y // 12) % 2:
                cd.rectangle([x, y, x + 11, y + 11], fill="#cccccc")
    for row, p in enumerate(paths):
        icon = Image.open(p).convert("RGBA").resize((cell, cell), Image.Resampling.LANCZOS)
        y = row * (cell + 4)
        for col, bg in enumerate([checker, Image.new("RGB", (cell, cell), "white"), Image.new("RGB", (cell, cell), "black")]):
            tile = bg.convert("RGBA")
            tile.alpha_composite(icon)
            sheet.paste(tile.convert("RGB"), (col * cell, y))
        kb = p.stat().st_size // 1024
        d.text((cell * 3 + 10, y + cell // 2 - label), p.stem, fill="black")
        d.text((cell * 3 + 10, y + cell // 2), f"{kb} KB", fill="#6b6b6b")
    sheet.save(out)
    print("sheet ", out)


if __name__ == "__main__":
    args = sys.argv[1:]
    sheet = None
    if "--sheet" in args:
        i = args.index("--sheet")
        sheet = Path(args[i + 1])
        del args[i : i + 2]
    made = build(set(args) or None)
    make_web_copies()
    if sheet:
        contact_sheet(sorted(OUT_DIR.glob("*.png")), sheet)
