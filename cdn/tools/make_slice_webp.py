"""Slice images as one WebP at 1x (owner 2026-09-30, agents/PERFORMANCE.md P2).

Slices are delivered and served at 1x only: 390 px wide (the standard slice is 390 x 625, the Naver webtoon size;
other heights are fine). This writes one WebP per source image:

    <out>/<slug>.webp     slug = the source name in lower case, runs of other characters -> "-"
                          ("S1C1 - Slice 7.jpg" -> "s1c1-slice-7.webp", "s1c2-slice-3.jpg" -> "s1c2-slice-3.webp")

Wider sources are scaled down to 390 px (Lanczos), narrower ones are never scaled up (a warning is printed).
The sources are left untouched. A WebP is (re)written only when it is missing or older than its source (`--force`
rewrites). Location (owner 2026-09-30): one folder per chapter code, `cdn/root/<toon>/images/webtoon/<sXcY>/`, next to
the delivered JPGs: `images/webtoon/s1c2/s1c2-slice-N.webp` (new names = cache-safe on the CDN; the configs point to
them, with width / height from `shared/content/scripts/slice-sizes.ts`).

Usage (from the repo root):
  cdn/.venv/Scripts/python cdn/tools/make_slice_webp.py <source files or globs...> --out=<folder>
      [--width=390] [--quality=85] [--force] [--dry-run]
  e.g. cdn/.venv/Scripts/python cdn/tools/make_slice_webp.py "cdn/root/catpope/images/webtoon/s1c2/*.jpg"
           --out=cdn/root/catpope/images/webtoon/s1c2
Requirements: pillow (cdn/tools/requirements.txt).
"""

from __future__ import annotations

import glob
import re
import sys
from pathlib import Path

from PIL import Image

WIDTH = 390
QUALITY = 85


def slug(stem: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", stem.lower()).strip("-")


def natural(path: Path) -> list:
    return [int(t) if t.isdigit() else t for t in re.split(r"(\d+)", path.name.lower())]


def main(argv: list[str]) -> int:
    opts = dict(a[2:].split("=", 1) if "=" in a else (a[2:], "1") for a in argv if a.startswith("--"))
    patterns = [a for a in argv if not a.startswith("--")]
    if not patterns or "out" not in opts:
        print(__doc__, file=sys.stderr)
        return 2
    out = Path(opts["out"])
    width = int(opts.get("width", WIDTH))
    quality = int(opts.get("quality", QUALITY))
    dry, force = "dry-run" in opts, "force" in opts

    sources = sorted({Path(p) for pattern in patterns for p in (glob.glob(pattern) or [pattern]) if Path(p).is_file()}, key=natural)
    if not sources:
        print("no source images found", file=sys.stderr)
        return 1
    if not dry:
        out.mkdir(parents=True, exist_ok=True)

    before = after = written = 0
    for source in sources:
        target = out / f"{slug(source.stem)}.webp"
        if not force and target.exists() and target.stat().st_mtime >= source.stat().st_mtime:
            continue
        with Image.open(source) as image:
            image = image.convert("RGBA" if "A" in image.getbands() else "RGB")
            w, h = image.size
            if w > width:
                image = image.resize((width, round(h * width / w)), Image.LANCZOS)
            elif w < width:
                print(f"warning: {source.name} is {w} px wide (< {width}), not scaled up", file=sys.stderr)
            if not dry:
                image.save(target, "WEBP", quality=quality, method=6)
        written += 1
        before += source.stat().st_size
        after += 0 if dry else target.stat().st_size
        print(f"{'would write' if dry else 'wrote'} {target.as_posix()}  {image.size[0]} x {image.size[1]}")
    print(f"{written} WebP {'to write' if dry else 'written'}" + ("" if dry or not written else f": {before / 1e6:.2f} MB -> {after / 1e6:.2f} MB"))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
