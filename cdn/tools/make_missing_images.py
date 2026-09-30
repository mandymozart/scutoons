"""Generate the missing-image substitutes for character images (owner, 2026-09-30).

One on-brand placeholder per base view: a neutral ink-on-paper silhouette with a "?" in the SCUTOONS style
(ink outline, paper fill, a hard ink offset shadow, transparent background). `resolveExpression` in
`shared/content/src/characters.ts` falls back to them as the very last step (after the legacy image) when nothing is
set or the resolved file is known to be missing.

Output: cdn/root/characters/_missing/missing_<view>.png
    avatar    192 x 192   bust
    portrait  600 x 600   bust
    full      600 x 1200  standing figure
    apose     768 x 1152  A-pose figure (same framing as the A-pose bodies)

Reproducible: the shapes are plain geometry (no fonts, no randomness), rendered by resvg.

Usage (from cdn/):
  .venv/Scripts/python tools/make_missing_images.py
Requirements: see tools/requirements.txt (pillow, resvg-py).
"""

from __future__ import annotations

import io
from pathlib import Path

import resvg_py
from PIL import Image

OUT_DIR = Path(__file__).resolve().parent.parent / "root" / "characters" / "_missing"

INK = "#000000"  # --ink
PAPER = "#ffffff"  # --paper


def question_mark(cx: float, cy: float, s: float) -> str:
    """A "?" drawn as strokes (hook + stem + dot), height ~ 1.3 s, centred on (cx, cy)."""
    w = 0.16 * s
    top = cy - 0.55 * s
    hook = (
        f"M {cx - 0.34 * s:.1f} {top + 0.3 * s:.1f} "
        f"C {cx - 0.34 * s:.1f} {top - 0.08 * s:.1f} {cx + 0.34 * s:.1f} {top - 0.08 * s:.1f} {cx + 0.34 * s:.1f} {top + 0.3 * s:.1f} "
        f"C {cx + 0.34 * s:.1f} {top + 0.55 * s:.1f} {cx:.1f} {top + 0.55 * s:.1f} {cx:.1f} {top + 0.85 * s:.1f}"
    )
    return (
        f'<path d="{hook}" fill="none" stroke="{INK}" stroke-width="{w:.1f}" stroke-linecap="round" stroke-linejoin="round"/>'
        f'<circle cx="{cx:.1f}" cy="{top + 1.15 * s:.1f}" r="{w * 0.62:.1f}" fill="{INK}"/>'
    )


class Figure:
    """Silhouette primitives; drawn as one outlined shape: every outline (ink) first, then every fill (paper)."""

    def __init__(self, t: float):
        self.t = t  # outline width
        self.items: list[tuple] = []

    def circle(self, cx, cy, r):
        self.items.append(("circle", cx, cy, r))

    def limb(self, x1, y1, x2, y2, w):
        self.items.append(("limb", x1, y1, x2, y2, w))

    def rect(self, x, y, w, h, r):
        self.items.append(("rect", x, y, w, h, r))

    def path(self, d):
        self.items.append(("path", d))

    def _svg(self, color: str, grow: float) -> str:
        out = []
        for it in self.items:
            kind = it[0]
            if kind == "circle":
                _, cx, cy, r = it
                out.append(f'<circle cx="{cx:.1f}" cy="{cy:.1f}" r="{r + grow:.1f}" fill="{color}"/>')
            elif kind == "limb":
                _, x1, y1, x2, y2, w = it
                out.append(
                    f'<line x1="{x1:.1f}" y1="{y1:.1f}" x2="{x2:.1f}" y2="{y2:.1f}" stroke="{color}" '
                    f'stroke-width="{w + 2 * grow:.1f}" stroke-linecap="round"/>'
                )
            elif kind == "rect":
                _, x, y, w, h, r = it
                out.append(
                    f'<rect x="{x - grow:.1f}" y="{y - grow:.1f}" width="{w + 2 * grow:.1f}" height="{h + 2 * grow:.1f}" '
                    f'rx="{r + grow:.1f}" fill="{color}"/>'
                )
            elif kind == "path":
                _, d = it
                stroke = f' stroke="{color}" stroke-width="{2 * grow:.1f}" stroke-linejoin="round"' if grow else ""
                out.append(f'<path d="{d}" fill="{color}"{stroke}/>')
        return "".join(out)

    def svg(self, shadow: float) -> str:
        ink = self._svg(INK, self.t)
        return f'<g transform="translate({shadow:.1f} {shadow:.1f})">{ink}</g>' + ink + self._svg(PAPER, 0)


def bust(size: int) -> str:
    """Head + shoulders, cropped at the bottom (avatar, portrait)."""
    W = float(size)
    f = Figure(0.028 * W)
    m = 0.06 * W  # keeps the outline + shadow inside the canvas at the sides
    f.path(
        f"M {m + 0.06 * W:.1f} {W:.1f} C {m + 0.06 * W:.1f} {0.74 * W:.1f} {0.3 * W:.1f} {0.66 * W:.1f} {0.5 * W:.1f} {0.66 * W:.1f} "
        f"C {0.7 * W:.1f} {0.66 * W:.1f} {W - m - 0.06 * W:.1f} {0.74 * W:.1f} {W - m - 0.06 * W:.1f} {W:.1f} Z"
    )
    f.rect(0.43 * W, 0.5 * W, 0.14 * W, 0.2 * W, 0.03 * W)  # neck
    f.circle(0.5 * W, 0.38 * W, 0.22 * W)
    return f.svg(0.035 * W) + question_mark(0.5 * W, 0.38 * W, 0.2 * W)


def standing(w: int, h: int) -> str:
    """Full figure, arms down (full view)."""
    W, H = float(w), float(h)
    f = Figure(0.022 * W)
    lw = 0.13 * W  # limb width
    f.limb(0.4 * W, 0.62 * H, 0.38 * W, 0.93 * H, lw)  # legs
    f.limb(0.6 * W, 0.62 * H, 0.62 * W, 0.93 * H, lw)
    f.limb(0.3 * W, 0.3 * H, 0.2 * W, 0.6 * H, lw * 0.8)  # arms
    f.limb(0.7 * W, 0.3 * H, 0.8 * W, 0.6 * H, lw * 0.8)
    f.rect(0.27 * W, 0.26 * H, 0.46 * W, 0.4 * H, 0.08 * W)  # torso
    f.rect(0.44 * W, 0.2 * H, 0.12 * W, 0.08 * H, 0.03 * W)  # neck
    f.circle(0.5 * W, 0.15 * H, 0.17 * W)
    return f.svg(0.025 * W) + question_mark(0.5 * W, 0.45 * H, 0.2 * W)


def apose(w: int, h: int) -> str:
    """A-pose figure: arms angled down and out, legs apart (same framing as the A-pose bodies)."""
    W, H = float(w), float(h)
    f = Figure(0.02 * W)
    lw = 0.1 * W
    f.limb(0.44 * W, 0.6 * H, 0.35 * W, 0.94 * H, lw * 1.1)  # legs
    f.limb(0.56 * W, 0.6 * H, 0.65 * W, 0.94 * H, lw * 1.1)
    f.limb(0.38 * W, 0.29 * H, 0.17 * W, 0.6 * H, lw * 0.8)  # arms
    f.limb(0.62 * W, 0.29 * H, 0.83 * W, 0.6 * H, lw * 0.8)
    f.rect(0.36 * W, 0.25 * H, 0.28 * W, 0.38 * H, 0.06 * W)  # torso
    f.rect(0.455 * W, 0.19 * H, 0.09 * W, 0.08 * H, 0.02 * W)  # neck
    f.circle(0.5 * W, 0.135 * H, 0.13 * W)
    return f.svg(0.02 * W) + question_mark(0.5 * W, 0.44 * H, 0.15 * W)


VIEWS: dict[str, tuple[int, int, str]] = {
    "avatar": (192, 192, bust(192)),
    "portrait": (600, 600, bust(600)),
    "full": (600, 1200, standing(600, 1200)),
    "apose": (768, 1152, apose(768, 1152)),
}


def render(w: int, h: int, body: str) -> Image.Image:
    svg = f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">{body}</svg>'
    png = resvg_py.svg_to_bytes(svg_string=svg, width=w, height=h)
    return Image.open(io.BytesIO(bytes(png))).convert("RGBA")


def main() -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    for view, (w, h, body) in VIEWS.items():
        path = OUT_DIR / f"missing_{view}.png"
        render(w, h, body).save(path, optimize=True)
        print(f"{path.relative_to(OUT_DIR.parent.parent.parent)}  {w}x{h}")


if __name__ == "__main__":
    main()
