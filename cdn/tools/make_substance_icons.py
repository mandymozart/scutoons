"""Placeholder icons of the substance items (agents/ITEMS.md "Substances") in the house style of make_item_icons.py
(thick ink outlines, paper/cyan accents, flat fills). Generic objects only – no real product look, no branding.

Output: cdn/root/scutoons/items/{checkit_camera,testo,testo_camera,psilocybin,lsd,semaglutide,weed,tuna_can}.png (512 px, transparent, trimmed + fitted
like every item icon). The owner replaces them later under the same names.
Also the players (agents/ITEMS.md "Players, albums, related items"): {record_player,tape_player,cd_player}.png - generic
devices, no brand look.

Usage (from the repo root, with the cdn venv):
  cdn/.venv/Scripts/python cdn/tools/make_substance_icons.py [--root <cdn root>] [name ...]
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from make_item_icons import CYAN, INK, PAPER, fit, render_svg  # noqa: E402

TUBE = "#e8ecef"
TUBE_CAP = "#f2c230"
MUSH = "#d8443a"
STEM = "#f3e9d2"
PEN = "#3b6fd6"
BAG = "#eef3f6"
BUD = "#5f9e3a"
TIN = "#c9cfd4"


def _tube(x: int, y: int, w: int, h: int, angle: float) -> str:
    """A plain squeeze tube (body, crimped end, cap), rotated around its centre."""
    cx, cy = x + w / 2, y + h / 2
    return f"""<g transform="rotate({angle} {cx} {cy})">
    <path d="M{x} {y + 30} L{x + w} {y + 30} L{x + w - 18} {y + h - 46} L{x + 18} {y + h - 46} Z" fill="{TUBE}" stroke="{INK}" stroke-width="18" stroke-linejoin="round"/>
    <rect x="{x - 8}" y="{y}" width="{w + 16}" height="34" fill="{TUBE}" stroke="{INK}" stroke-width="18"/>
    <path d="M{x + 14} {y + 17} H {x + w - 14}" stroke="{INK}" stroke-width="6" stroke-dasharray="10 10"/>
    <rect x="{x + w / 2 - 34}" y="{y + h - 52}" width="68" height="52" fill="{TUBE_CAP}" stroke="{INK}" stroke-width="18"/>
    <rect x="{x + 34}" y="{y + 90}" width="{w - 68}" height="{(h - 200) / 2}" fill="{CYAN}" stroke="{INK}" stroke-width="10"/>
    <text x="{cx}" y="{y + 90 + (h - 200) / 4 + 14}" font-family="monospace" font-weight="700" font-size="42" text-anchor="middle" fill="{INK}">T</text>
  </g>"""


def testo_svg() -> str:
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  {_tube(176, 40, 160, 430, -24)}
</svg>"""


def testo_camera_svg() -> str:
    # the starter camera (make_item_icons) with a tube strapped over it, like the cucumber camera's cucumber
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <rect x="150" y="146" width="130" height="58" fill="{PAPER}" stroke="{INK}" stroke-width="18"/>
  <rect x="40" y="186" width="432" height="232" fill="{PAPER}" stroke="{INK}" stroke-width="18"/>
  <rect x="352" y="160" width="62" height="26" fill="{INK}"/>
  <rect x="70" y="214" width="56" height="30" fill="{CYAN}" stroke="{INK}" stroke-width="10"/>
  <circle cx="256" cy="292" r="104" fill="{PAPER}" stroke="{INK}" stroke-width="18"/>
  <circle cx="256" cy="292" r="66" fill="{TUBE_CAP}" stroke="{INK}" stroke-width="14"/>
  <circle cx="256" cy="292" r="26" fill="{INK}"/>
  {_tube(330, 20, 110, 300, 38)}
</svg>"""


def psilocybin_svg() -> str:
    dots = "".join(f'<circle cx="{x}" cy="{y}" r="{r}" fill="{PAPER}" stroke="{INK}" stroke-width="8"/>' for x, y, r in [(170, 170, 26), (256, 128, 22), (340, 176, 28), (214, 222, 16), (302, 230, 14)])
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <path d="M206 270 C 196 360, 186 420, 176 470 H 336 C 326 420, 316 360, 306 270 Z" fill="{STEM}" stroke="{INK}" stroke-width="18" stroke-linejoin="round"/>
  <path d="M56 286 C 56 120, 456 120, 456 286 Z" fill="{MUSH}" stroke="{INK}" stroke-width="18" stroke-linejoin="round"/>
  {dots}
</svg>"""


def lsd_svg() -> str:
    # a blotter square with a printed spiral (a generic pattern)
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <g transform="rotate(-12 256 256)">
    <rect x="86" y="86" width="340" height="340" fill="{PAPER}" stroke="{INK}" stroke-width="18"/>
    <path d="M86 199 H 426 M86 312 H 426 M199 86 V 426 M312 86 V 426" stroke="{INK}" stroke-width="5" stroke-dasharray="12 10"/>
    <path d="M256 256 m0 -18 a18 18 0 1 1 -18 18 a40 40 0 1 1 40 40 a64 64 0 1 1 -64 -64 a92 92 0 1 1 92 92" fill="none" stroke="{CYAN}" stroke-width="16" stroke-linecap="round"/>
    <path d="M256 256 m0 -18 a18 18 0 1 1 -18 18 a40 40 0 1 1 40 40 a64 64 0 1 1 -64 -64 a92 92 0 1 1 92 92" fill="none" stroke="{MUSH}" stroke-width="6" stroke-linecap="round"/>
  </g>
</svg>"""


def semaglutide_svg() -> str:
    # a generic injection pen: body, window, dose dial, cap – no product look
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <g transform="rotate(-40 256 256)">
    <rect x="40" y="206" width="300" height="100" rx="18" fill="{PAPER}" stroke="{INK}" stroke-width="18"/>
    <rect x="96" y="232" width="120" height="48" fill="{CYAN}" stroke="{INK}" stroke-width="10"/>
    <path d="M120 232 V 280 M150 232 V 280 M180 232 V 280" stroke="{INK}" stroke-width="6"/>
    <rect x="340" y="222" width="70" height="68" fill="{PEN}" stroke="{INK}" stroke-width="18"/>
    <rect x="410" y="236" width="62" height="40" fill="{INK}"/>
    <path d="M40 256 H 12" stroke="{INK}" stroke-width="14" stroke-linecap="round"/>
  </g>
</svg>"""


def checkit_camera_svg() -> str:
    # the starter camera with a small test tube on the body - a generic mark, no real logo
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <rect x="150" y="146" width="130" height="58" fill="{PAPER}" stroke="{INK}" stroke-width="18"/>
  <rect x="40" y="186" width="432" height="232" fill="{PAPER}" stroke="{INK}" stroke-width="18"/>
  <rect x="352" y="160" width="62" height="26" fill="{INK}"/>
  <circle cx="230" cy="302" r="98" fill="{PAPER}" stroke="{INK}" stroke-width="18"/>
  <circle cx="230" cy="302" r="62" fill="{CYAN}" stroke="{INK}" stroke-width="14"/>
  <circle cx="230" cy="302" r="24" fill="{INK}"/>
  <g transform="rotate(18 400 300)">
    <path d="M378 226 V 342 A22 22 0 0 0 422 342 V 226" fill="{PAPER}" stroke="{INK}" stroke-width="14"/>
    <path d="M378 292 H 422 V 342 A22 22 0 0 1 378 342 Z" fill="{MUSH}"/>
    <path d="M366 226 H 434" stroke="{INK}" stroke-width="14" stroke-linecap="square"/>
  </g>
</svg>"""


def weed_svg() -> str:
    # a small zip bag with a few green buds (a generic game object, no branding)
    buds = "".join(f'<ellipse cx="{x}" cy="{y}" rx="{rx}" ry="{ry}" fill="{BUD}" stroke="{INK}" stroke-width="10"/>' for x, y, rx, ry in [(196, 330, 52, 40), (290, 352, 58, 42), (246, 280, 46, 36), (330, 292, 38, 30)])
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <g transform="rotate(-8 256 256)">
    <rect x="96" y="96" width="320" height="360" fill="{BAG}" stroke="{INK}" stroke-width="18" stroke-linejoin="round"/>
    <rect x="96" y="96" width="320" height="40" fill="{CYAN}" stroke="{INK}" stroke-width="18"/>
    <path d="M120 170 H 392" stroke="{INK}" stroke-width="6" stroke-dasharray="14 10"/>
    {buds}
  </g>
</svg>"""


def tuna_can_svg() -> str:
    # a generic flat tin with a ring pull and a fish outline on the label - no brand
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <path d="M56 214 V 330 A200 66 0 0 0 456 330 V 214" fill="{TIN}" stroke="{INK}" stroke-width="18" stroke-linejoin="round"/>
  <path d="M56 236 V 308 A200 66 0 0 0 456 308 V 236 A200 66 0 0 1 56 236 Z" fill="{CYAN}" stroke="{INK}" stroke-width="10"/>
  <path d="M200 326 C 234 298, 284 298, 310 326 C 284 354, 234 354, 200 326 Z M310 326 L 338 306 V 346 Z" fill="{PAPER}" stroke="{INK}" stroke-width="10" stroke-linejoin="round"/>
  <ellipse cx="256" cy="214" rx="200" ry="66" fill="{TIN}" stroke="{INK}" stroke-width="18"/>
  <ellipse cx="256" cy="214" rx="168" ry="50" fill="none" stroke="{INK}" stroke-width="6"/>
  <ellipse cx="300" cy="196" rx="40" ry="18" fill="none" stroke="{INK}" stroke-width="12"/>
  <path d="M262 204 L 236 214" stroke="{INK}" stroke-width="12" stroke-linecap="round"/>
</svg>"""


VINYL = "#1d1d1d"
SILVER = "#d9dee2"


def record_player_svg() -> str:
    # a turntable: plinth, platter with a record (cyan label), tonearm on the right
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <rect x="28" y="96" width="456" height="340" fill="{PAPER}" stroke="{INK}" stroke-width="18"/>
  <circle cx="214" cy="266" r="146" fill="{VINYL}" stroke="{INK}" stroke-width="18"/>
  <circle cx="214" cy="266" r="112" fill="none" stroke="{SILVER}" stroke-width="4"/>
  <circle cx="214" cy="266" r="84" fill="none" stroke="{SILVER}" stroke-width="4"/>
  <circle cx="214" cy="266" r="44" fill="{CYAN}" stroke="{INK}" stroke-width="10"/>
  <circle cx="214" cy="266" r="8" fill="{INK}"/>
  <circle cx="420" cy="150" r="30" fill="{SILVER}" stroke="{INK}" stroke-width="14"/>
  <path d="M420 150 L 420 300 L 330 352" fill="none" stroke="{INK}" stroke-width="16" stroke-linecap="square" stroke-linejoin="round"/>
  <rect x="304" y="338" width="40" height="28" transform="rotate(-30 324 352)" fill="{INK}"/>
  <rect x="396" y="388" width="56" height="22" fill="{CYAN}" stroke="{INK}" stroke-width="8"/>
</svg>"""


def tape_player_svg() -> str:
    # a portable tape player: body, a cassette window with two reels, buttons on top
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <rect x="96" y="70" width="60" height="36" fill="{SILVER}" stroke="{INK}" stroke-width="12"/>
  <rect x="172" y="70" width="60" height="36" fill="{CYAN}" stroke="{INK}" stroke-width="12"/>
  <rect x="248" y="70" width="60" height="36" fill="{SILVER}" stroke="{INK}" stroke-width="12"/>
  <rect x="40" y="104" width="432" height="320" fill="{PAPER}" stroke="{INK}" stroke-width="18"/>
  <rect x="84" y="150" width="344" height="196" fill="{CYAN}" stroke="{INK}" stroke-width="14"/>
  <rect x="134" y="196" width="244" height="92" fill="{PAPER}" stroke="{INK}" stroke-width="10"/>
  <circle cx="190" cy="242" r="30" fill="{VINYL}" stroke="{INK}" stroke-width="8"/>
  <circle cx="322" cy="242" r="30" fill="{VINYL}" stroke="{INK}" stroke-width="8"/>
  <circle cx="190" cy="242" r="10" fill="{PAPER}"/>
  <circle cx="322" cy="242" r="10" fill="{PAPER}"/>
  <path d="M84 376 H 428" stroke="{INK}" stroke-width="10"/>
</svg>"""


def cd_player_svg() -> str:
    # a portable CD player seen from above: a round body, the disc through the lid window, a few buttons
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <rect x="44" y="44" width="424" height="424" rx="96" fill="{PAPER}" stroke="{INK}" stroke-width="18"/>
  <circle cx="256" cy="242" r="156" fill="{SILVER}" stroke="{INK}" stroke-width="14"/>
  <path d="M256 242 m-120 0 a120 120 0 0 1 120 -120" fill="none" stroke="{CYAN}" stroke-width="22" stroke-linecap="round"/>
  <path d="M256 242 m120 0 a120 120 0 0 1 -120 120" fill="none" stroke="{CYAN}" stroke-width="12" stroke-linecap="round"/>
  <circle cx="256" cy="242" r="38" fill="{PAPER}" stroke="{INK}" stroke-width="12"/>
  <circle cx="256" cy="242" r="12" fill="{INK}"/>
  <rect x="150" y="420" width="52" height="22" fill="{INK}"/>
  <rect x="230" y="420" width="52" height="22" fill="{CYAN}" stroke="{INK}" stroke-width="6"/>
  <rect x="310" y="420" width="52" height="22" fill="{INK}"/>
</svg>"""


ICONS = {
    "checkit_camera": checkit_camera_svg,
    "testo": testo_svg,
    "testo_camera": testo_camera_svg,
    "psilocybin": psilocybin_svg,
    "lsd": lsd_svg,
    "semaglutide": semaglutide_svg,
    "weed": weed_svg,
    "tuna_can": tuna_can_svg,
    "record_player": record_player_svg,
    "tape_player": tape_player_svg,
    "cd_player": cd_player_svg,
}


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default=str(Path(__file__).resolve().parent.parent / "root"))
    ap.add_argument("names", nargs="*", help="only these icons (default: all)")
    args = ap.parse_args()
    out = Path(args.root) / "scutoons" / "items"
    out.mkdir(parents=True, exist_ok=True)
    for name, svg in ICONS.items():
        if args.names and name not in args.names:
            continue
        p = out / f"{name}.png"
        fit(render_svg(svg())).save(p, optimize=True, compress_level=9)
        print(f"{p.relative_to(Path(args.root))}  {p.stat().st_size // 1024} KB")


if __name__ == "__main__":
    main()
