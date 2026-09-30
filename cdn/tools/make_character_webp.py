"""WebP versions of the character images (agents/PERFORMANCE.md P3, owner 2026-09-30).

The owner's image supplier delivers PNGs into the convention folders
`cdn/root/characters/<character>[-<persona>]/`. This writes, next to them:

    <name>[_<look>]_portrait-<mood>.webp     from the portrait PNG (600 x 600), quality 85, alpha kept
    <name>[_<look>]_full-<mood>.webp         from the full PNG (600 x 1200)
    <name>[_<look>]_avatar-<mood>.webp       the 192 px avatar display copy (below)
    <name>_transition-<from>-to-<to>.webp    from the transition PNG
    <name>_apose-<look>.webp                 from an A-pose PNG (768 x 1152)

Avatars (owner rules 2026-09-30, workflows/characters/base-views.md):
- A **delivered** avatar (its own view, `_avatar-<mood>.png` of at least 600 x 600) is downscaled to 192 px.
- Otherwise the avatar is **cropped from that mood's portrait**: a square head-with-neck crop (the framing of the avatar
  view: head with neck, facing the viewer, fully inside the frame), scaled to 192 px. Smaller avatar PNGs (the old
  192 / 96 px copies) are not deliveries and are ignored.
- The crop box is derived once per `<name>[_<look>]` from its **neutral** portrait and used for every mood of it, so all
  moods stay aligned. Method (`head_crop`): alpha mask (> 64); top = first row of the figure; neck = the narrowest row
  (widths smoothed over 9 rows) between 25 % of the figure height and 75 % of the canvas; the head = rows top..neck,
  centred on its alpha centroid; side = max(1.4 x head height, 1.2 x the head's 90th-percentile row width); then widened / raised so the
  head (rows above that neck) of every mood of the name fits (highest top, widest extent, 2 % margin); the top edge 6 %
  of the side above the highest head top. The box may reach outside the canvas (transparent padding).
- The boxes are stored in `cdn/tools/avatar-crops.json` (`{"<name>[_<look>]": {"box": [x, y, side], "source": "auto"}}`,
  portrait pixels). `"source": "manual"` entries are the override: never recomputed (edit the box by hand, set manual,
  re-run). Auto entries are recomputed from the neutral portrait on every run; when a box changes, that name's avatars
  are rewritten.

The PNGs stay (the released viewer and older store versions point to them). The missing-image substitutes
(`_missing/`) are left alone. A WebP is (re)written only when it is missing or older than its PNG (or its crop box
changed), so re-running is cheap; `--force` rewrites all.

Run by the characters import (`shared/content/scripts/migrate-characters.ts`) before it scans the folders, or by hand
(from the repo root):
  cdn/.venv/Scripts/python cdn/tools/make_character_webp.py [--root=cdn/root] [--dry-run] [--force] [--sheet=<png>]
`--sheet` writes a contact sheet of every avatar WebP (crop on the portrait + the 192 px result) for a visual check.
Requirements: pillow, numpy (cdn/tools/requirements.txt).
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

QUALITY = 85
AVATAR_SIZE = 192
AVATAR_MASTER = 600  # a delivered avatar view is 600 x 600 (spec); smaller avatar PNGs are old derived copies
EXPRESSION = re.compile(r"^(?P<prefix>.+)_(?P<view>portrait|avatar|full)-(?P<mood>[a-z0-9-]+)\.png$")
TRANSITION = re.compile(r"^.+_transition-[a-z0-9]+-to-[a-z0-9]+\.png$")
APOSE = re.compile(r"^.+_apose-[a-z0-9-]+\.png$")
SKIP_FOLDERS = {"_missing"}
CROPS_FILE = Path(__file__).resolve().parent / "avatar-crops.json"


def save_webp(image: Image.Image, target: Path) -> None:
    image = image.convert("RGBA") if image.mode not in ("RGB", "RGBA") else image
    image.save(target, "WEBP", quality=QUALITY, method=6)


def outdated(target: Path, source: Path) -> bool:
    return not target.exists() or target.stat().st_mtime < source.stat().st_mtime


def _mask(image: Image.Image) -> np.ndarray:
    return np.array(image.convert("RGBA").getchannel("A")) > 64


def head_crop(image: Image.Image, moods: list[Image.Image] | None = None) -> list[int]:
    """Square head-with-neck box [x, y, side] (portrait pixels) – see the module docstring. `image` is the neutral
    portrait (it sets the neck and the centre); the box then grows so the head of every mood (`moods`) fits in it too
    (tilted heads, raised ears), still one box for all of them."""
    a = _mask(image)
    height, width = a.shape
    w = a.sum(1).astype(float)
    rows = np.nonzero(w >= max(3, width * 0.01))[0]
    if not len(rows):
        return [0, 0, min(width, height)]
    top = int(rows[0])
    smooth = np.convolve(w, np.ones(9) / 9, mode="same")
    lo, hi = int(top + 0.25 * (height - top)), int(height * 0.75)
    neck = lo + int(np.argmin(smooth[lo:hi])) if hi > lo else height
    head = a[top:neck]
    ys, xs = np.nonzero(head)
    cx = float(xs.mean()) if len(xs) else width / 2
    widths = np.sort(head.sum(1))
    wide = float(widths[int(len(widths) * 0.9)]) if len(widths) else width
    # every mood's head (rows above the neutral neck): the highest top and the widest extent
    x_lo, x_hi = (float(xs.min()), float(xs.max())) if len(xs) else (0.0, float(width))
    for mood in moods or []:
        m = _mask(mood)
        if m.shape != a.shape:
            continue
        r = np.nonzero(m.sum(1) >= max(3, width * 0.01))[0]
        if not len(r) or r[0] >= neck:
            continue
        top = min(top, int(r[0]))
        mx = np.nonzero(m[int(r[0]):neck].any(0))[0]
        x_lo, x_hi = min(x_lo, float(mx.min())), max(x_hi, float(mx.max()))
    side = max((neck - top) * 1.4, wide * 1.2, (x_hi - x_lo) * 1.04)
    x0 = min(max(cx - side / 2, x_hi + 0.02 * side - side), x_lo - 0.02 * side)
    return [round(x0), round(top - 0.06 * side), round(side)]


def crop_square(image: Image.Image, box: list[int]) -> Image.Image:
    x, y, side = box
    src = image.convert("RGBA")
    out = Image.new("RGBA", (side, side), (0, 0, 0, 0))
    part = src.crop((max(x, 0), max(y, 0), min(x + side, src.width), min(y + side, src.height)))
    out.alpha_composite(part, (max(-x, 0), max(-y, 0)))
    return out


def avatar_from_portrait(portrait: Path, box: list[int]) -> Image.Image:
    return crop_square(Image.open(portrait), box).resize((AVATAR_SIZE, AVATAR_SIZE), Image.LANCZOS)


def delivered_avatar(path: Path) -> bool:
    with Image.open(path) as image:
        return min(image.size) >= AVATAR_MASTER


def load_crops() -> dict:
    try:
        return json.loads(CROPS_FILE.read_text(encoding="utf8"))
    except FileNotFoundError:
        return {}


def contact_sheet(entries: list[tuple[str, Path, list[int] | None, Path]], target: Path) -> None:
    """One tile per avatar: the portrait with the crop box, and the 192 px WebP on dark and light."""
    cols, tile_w, tile_h = 6, 420, 236
    rows = (len(entries) + cols - 1) // cols
    sheet = Image.new("RGB", (cols * tile_w, max(rows, 1) * tile_h), "white")
    draw = ImageDraw.Draw(sheet)
    for i, (label, source, box, webp) in enumerate(entries):
        x, y = (i % cols) * tile_w, (i // cols) * tile_h
        draw.text((x + 4, y + 2), label[:64], fill="black")
        src = Image.open(source).convert("RGBA")
        k = 192 / src.width
        thumb = src.resize((192, round(src.height * k)), Image.LANCZOS)
        bg = Image.new("RGBA", thumb.size, (200, 200, 200, 255))
        bg.alpha_composite(thumb)
        if box:
            ImageDraw.Draw(bg).rectangle([box[0] * k, box[1] * k, (box[0] + box[2]) * k, (box[1] + box[2]) * k], outline=(255, 0, 0), width=2)
        sheet.paste(bg.convert("RGB"), (x + 4, y + 18))
        avatar = Image.open(webp).convert("RGBA")
        for j, colour in enumerate(((30, 30, 30, 255), (240, 240, 240, 255))):
            tile = Image.new("RGBA", avatar.size, colour)
            tile.alpha_composite(avatar)
            if j == 0:
                sheet.paste(tile.convert("RGB"), (x + 200, y + 18))
            else:
                sheet.paste(tile.convert("RGB").resize((64, 64), Image.LANCZOS), (x + 200 + 192 - 64, y + 18 + 192 + 2 - 64 - 2))
    sheet.save(target)


def main(argv: list[str]) -> int:
    args = dict(a[2:].split("=", 1) if "=" in a else (a[2:], "1") for a in argv if a.startswith("--"))
    root = Path(args.get("root", Path(__file__).resolve().parent.parent / "root")) / "characters"
    dry, force = "dry-run" in args, "force" in args
    if not root.is_dir():
        print(f"no characters folder: {root}", file=sys.stderr)
        return 1

    crops = load_crops()
    old_crops = json.loads(json.dumps(crops))
    written: list[tuple[Path, int, int]] = []
    sheet: list[tuple[str, Path, list[int] | None, Path]] = []
    computed: set[str] = set()  # auto boxes derived in this run (once per name)

    def write(image_source: Path, target: Path, make, again: bool = False) -> None:
        if not force and not again and not outdated(target, image_source):
            return
        if not dry:
            save_webp(make(), target)
        written.append((target, image_source.stat().st_size, 0 if dry else target.stat().st_size))

    for folder in sorted(p for p in root.iterdir() if p.is_dir() and p.name not in SKIP_FOLDERS):
        files = {f.name for f in folder.iterdir() if f.is_file()}
        for name in sorted(files):
            source = folder / name
            if TRANSITION.match(name) or APOSE.match(name):
                write(source, source.with_suffix(".webp"), lambda s=source: Image.open(s))
                continue
            m = EXPRESSION.match(name)
            if not m:
                continue
            if m["view"] == "avatar":
                if delivered_avatar(source):
                    # a delivered avatar is its own view (owner 2026-09-30): the WebP is its 192 px display copy
                    write(source, source.with_suffix(".webp"), lambda s=source: Image.open(s).convert("RGBA").resize((AVATAR_SIZE, AVATAR_SIZE), Image.LANCZOS))
                    target = source.with_suffix(".webp")
                    if target.exists():
                        sheet.append((f"{folder.name}/{target.name} (delivered)", source, None, target))
                continue
            write(source, source.with_suffix(".webp"), lambda s=source: Image.open(s))
            if m["view"] != "portrait":
                continue
            avatar_png = folder / f"{m['prefix']}_avatar-{m['mood']}.png"
            if avatar_png.name in files and delivered_avatar(avatar_png):
                continue
            # no delivered avatar: crop the head with neck from this mood's portrait, with the name's (neutral) box
            key = m["prefix"]
            entry = crops.get(key)
            if key not in computed and (not entry or entry.get("source") != "manual"):
                computed.add(key)
                neutral = folder / f"{key}_portrait-neutral.png"
                pattern = re.compile(rf"^{re.escape(key)}_portrait-[a-z0-9-]+\.png$")
                moods = [Image.open(folder / f) for f in sorted(files) if pattern.match(f)]
                box = head_crop(Image.open(neutral if neutral.exists() else source), moods)
                entry = crops[key] = {"box": box, "source": "auto"}
            box = entry["box"]
            changed = old_crops.get(key, {}).get("box") != box
            target = folder / f"{m['prefix']}_avatar-{m['mood']}.webp"
            write(source, target, lambda s=source, b=box: avatar_from_portrait(s, b), again=changed)
            sheet.append((f"{folder.name}/{target.name}", source, box, target))

    if not dry and crops != old_crops:
        lines = [f"\t{json.dumps(key)}: {json.dumps(value)}" for key, value in sorted(crops.items())]
        CROPS_FILE.write_text("{\n" + ",\n".join(lines) + "\n}\n", encoding="utf8", newline="\n")
        print(f"crop boxes updated: {CROPS_FILE.name}")
    if "sheet" in args and not dry:
        contact_sheet(sheet, Path(args["sheet"]))
        print(f"contact sheet: {args['sheet']} ({len(sheet)} avatars)")

    before = sum(b for _, b, _ in written)
    after = sum(a for _, _, a in written)
    for target, b, a in written:
        print(f"{'would write' if dry else 'wrote'} {target.relative_to(root.parent).as_posix()}  {b // 1024} KB -> {a // 1024} KB")
    print(f"{len(written)} WebP {'to write' if dry else 'written'}" + ("" if dry or not written else f": {before / 1e6:.1f} MB PNG -> {after / 1e6:.1f} MB WebP"))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
