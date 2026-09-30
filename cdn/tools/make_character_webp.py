"""WebP versions of the character images (agents/PERFORMANCE.md P3, owner 2026-09-30).

The owner's image supplier delivers PNGs into the convention folders
`cdn/root/characters/<character>[-<persona>]/`. This writes, next to them:

    <name>[_<look>]_portrait-<mood>.webp     from the portrait PNG (600 x 600), quality 85, alpha kept
    <name>[_<look>]_avatar-<mood>.webp       192 px: downscaled from the delivered avatar PNG when there is one, else from the portrait
                                             (192 x 192 downscale, the derived view)
    <name>_transition-<from>-to-<to>.webp    from the transition PNG

The PNGs stay (the released viewer and older store versions point to them). The missing-image substitutes
(`_missing/`) and the A-poses (already WebP) are left alone. A WebP is (re)written only when it is missing or older
than its PNG, so re-running is cheap; `--force` rewrites all.

Run by the characters import (`shared/content/scripts/migrate-characters.ts`) before it scans the folders, or by hand
(from the repo root):
  cdn/.venv/Scripts/python cdn/tools/make_character_webp.py [--root=cdn/root] [--dry-run] [--force]
Requirements: pillow (cdn/tools/requirements.txt).
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

from PIL import Image

QUALITY = 85
AVATAR_SIZE = 192
EXPRESSION = re.compile(r"^(?P<prefix>.+)_(?P<view>portrait|avatar)-(?P<mood>[a-z0-9-]+)\.png$")
TRANSITION = re.compile(r"^.+_transition-[a-z0-9]+-to-[a-z0-9]+\.png$")
SKIP_FOLDERS = {"_missing"}


def save_webp(image: Image.Image, target: Path) -> None:
    image = image.convert("RGBA") if image.mode not in ("RGB", "RGBA") else image
    image.save(target, "WEBP", quality=QUALITY, method=6)


def outdated(target: Path, source: Path) -> bool:
    return not target.exists() or target.stat().st_mtime < source.stat().st_mtime


def main(argv: list[str]) -> int:
    args = dict(a[2:].split("=", 1) if "=" in a else (a[2:], "1") for a in argv if a.startswith("--"))
    root = Path(args.get("root", Path(__file__).resolve().parent.parent / "root")) / "characters"
    dry, force = "dry-run" in args, "force" in args
    if not root.is_dir():
        print(f"no characters folder: {root}", file=sys.stderr)
        return 1

    written: list[tuple[Path, int, int]] = []

    def write(image_source: Path, target: Path, make) -> None:
        if not force and not outdated(target, image_source):
            return
        if not dry:
            save_webp(make(), target)
        written.append((target, image_source.stat().st_size, 0 if dry else target.stat().st_size))

    for folder in sorted(p for p in root.iterdir() if p.is_dir() and p.name not in SKIP_FOLDERS):
        files = {f.name for f in folder.iterdir() if f.is_file()}
        for name in sorted(files):
            source = folder / name
            if TRANSITION.match(name):
                write(source, source.with_suffix(".webp"), lambda s=source: Image.open(s))
                continue
            m = EXPRESSION.match(name)
            if not m:
                continue
            if m["view"] == "avatar":
                # a delivered avatar is its own view (owner 2026-09-30): the WebP is its 192 px display copy
                write(source, source.with_suffix(".webp"), lambda s=source: Image.open(s).convert("RGBA").resize((AVATAR_SIZE, AVATAR_SIZE), Image.LANCZOS))
            else:
                write(source, source.with_suffix(".webp"), lambda s=source: Image.open(s))
            if m["view"] == "portrait":
                avatar_png = f"{m['prefix']}_avatar-{m['mood']}.png"
                if avatar_png not in files:
                    # a new delivery without an avatar: the avatar is the derived view (192 px downscale)
                    target = folder / f"{m['prefix']}_avatar-{m['mood']}.webp"
                    write(source, target, lambda s=source: Image.open(s).convert("RGBA").resize((AVATAR_SIZE, AVATAR_SIZE), Image.LANCZOS))

    before = sum(b for _, b, _ in written)
    after = sum(a for _, _, a in written)
    for target, b, a in written:
        print(f"{'would write' if dry else 'wrote'} {target.relative_to(root.parent).as_posix()}  {b // 1024} KB -> {a // 1024} KB")
    print(f"{len(written)} WebP {'to write' if dry else 'written'}" + ("" if dry or not written else f": {before / 1e6:.1f} MB PNG -> {after / 1e6:.1f} MB WebP"))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
