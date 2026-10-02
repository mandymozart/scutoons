"""Slice images as one WebP at 1x, as sharp as possible (owner 2026-09-30, agents/PERFORMANCE.md P2).

Slices are delivered and served at 1x only: 390 px wide (the standard slice is 390 x 625, the Naver webtoon size;
other heights are fine). This writes one WebP per source image:

    <out>/<slug>.webp     slug = the source name in lower case, runs of other characters -> "-"
                          ("S1C1 - Slice 7.jpg" -> "s1c1-slice-7.webp", "s1c2-slice-3.jpg" -> "s1c2-slice-3.webp")

Always start from the highest-resolution source there is (the delivered masters, e.g. 799 x 1280).
- Wider sources are scaled down to 390 px with Lanczos in sRGB (a linear-light resize was compared and thins dark
  line art and lettering, see PERFORMANCE.md "Slice sharpness"), then get a mild unsharp mask tuned for line art
  (radius 0.6, 60 %, threshold 2 - lifts the edges softened by the downscale, leaves flat areas and grain alone).
- Sources that are already 390 px wide are only re-encoded (no sharpening: it would lift the JPEG artefacts).
- Narrower sources are never scaled up (a warning is printed).
- Encoding: WebP quality 88, method 6. With `cwebp` (libwebp tools) on the PATH, or its path in the CWEBP environment
  variable, the encode also uses `-sharp_yuv` (sharper colour edges in the 4:2:0 chroma); Pillow does not expose
  that option, so without cwebp the encode is Pillow's (a note is printed).
The sources are left untouched. A WebP is (re)written only when it is missing or older than its source (`--force`
rewrites). Location (agents/ASSETS.md "Folder convention", 2026-10-02): the chapter's folder
`cdn/root/<toon>/s<n>/c<n>/`: `s1/c2/s1c2-slice-N.webp` (new names = cache-safe on the CDN; the configs point to them, with
width / height from `shared/content/scripts/slice-sizes.ts`); density variants as suffixes (`.2x`, `.3x`).

Usage (from the repo root):
  cdn/.venv/Scripts/python cdn/tools/make_slice_webp.py <source files or globs...> --out=<folder>
      [--width=390] [--quality=88] [--sharpen=0.6,60,2 | --sharpen=0] [--force] [--dry-run]
  e.g. chapter 2: cdn/.venv/Scripts/python cdn/tools/make_slice_webp.py "<delivered>/s1c2/*.jpg"
           --out=cdn/root/catpope/s1/c2 --force
Requirements: pillow (cdn/tools/requirements.txt); optional cwebp.
"""

from __future__ import annotations

import glob
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image, ImageFilter

WIDTH = 390
QUALITY = 88
SHARPEN = (0.6, 60, 2)  # unsharp mask radius, percent, threshold - applied after a downscale only


def slug(stem: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", stem.lower()).strip("-")


def natural(path: Path) -> list:
    return [int(t) if t.isdigit() else t for t in re.split(r"(\d+)", path.name.lower())]


def cwebp_path() -> str | None:
    return os.environ.get("CWEBP") or shutil.which("cwebp")


def encode(image: Image.Image, target: Path, quality: int, cwebp: str | None) -> None:
    if not cwebp:
        image.save(target, "WEBP", quality=quality, method=6)
        return
    with tempfile.TemporaryDirectory() as tmp:
        png = Path(tmp) / "slice.png"
        image.save(png, "PNG", compress_level=1)
        subprocess.run([cwebp, "-quiet", "-q", str(quality), "-m", "6", "-sharp_yuv", "-metadata", "none", str(png), "-o", str(target)], check=True)


def main(argv: list[str]) -> int:
    opts = dict(a[2:].split("=", 1) if "=" in a else (a[2:], "1") for a in argv if a.startswith("--"))
    patterns = [a for a in argv if not a.startswith("--")]
    if not patterns or "out" not in opts:
        print(__doc__, file=sys.stderr)
        return 2
    out = Path(opts["out"])
    width = int(opts.get("width", WIDTH))
    quality = int(opts.get("quality", QUALITY))
    sharpen = SHARPEN if "sharpen" not in opts else (None if opts["sharpen"] in ("0", "") else tuple(float(v) for v in opts["sharpen"].split(",")))
    dry, force = "dry-run" in opts, "force" in opts
    cwebp = cwebp_path()
    if not cwebp:
        print("note: cwebp not found (PATH or CWEBP) - encoding with Pillow, without sharp_yuv", file=sys.stderr)

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
        old = target.stat().st_size if target.exists() else 0
        with Image.open(source) as image:
            image = image.convert("RGBA" if "A" in image.getbands() else "RGB")
            w, h = image.size
            if w > width:
                image = image.resize((width, round(h * width / w)), Image.LANCZOS)
                if sharpen:
                    image = image.filter(ImageFilter.UnsharpMask(radius=sharpen[0], percent=int(sharpen[1]), threshold=int(sharpen[2])))
            elif w < width:
                print(f"warning: {source.name} is {w} px wide (< {width}), not scaled up", file=sys.stderr)
            if not dry:
                encode(image, target, quality, cwebp)
        written += 1
        before += old
        after += 0 if dry else target.stat().st_size
        print(f"{'would write' if dry else 'wrote'} {target.as_posix()}  {image.size[0]} x {image.size[1]}")
    print(f"{written} WebP {'to write' if dry else 'written'}" + ("" if dry or not written else f": {before / 1e6:.3f} MB (previous WebP) -> {after / 1e6:.3f} MB"))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
