"""Find text boxes (captions / speech bubbles) baked into comic slice images.

Heuristic, no OCR: a text box is a bright, roughly uniform region (white-ish caption box
or bubble) whose holes are dark glyphs. For every slice it reports

- ``box``: the enclosing bright region (bbox, px + % of slice), its background colour
  and rectangularity (1.0 = perfect rectangle, lower = rounded / cloud bubble)
- ``text``: tight bbox around the dark glyphs inside it, the detected text lines,
  mean line pitch / glyph height (px + % of slice width) and a guessed alignment

and optionally writes crops (for human transcription) and an overlay image per slice.

Usage (from repo root, with the cdn venv):

    cdn/.venv/Scripts/python cdn/tools/extract_bubbles.py \
        "cdn/root/catpope/images/3x/de/S1C1 - Slice *.jpg" \
        --json out.json --crops some/dir --overlay

Requires: numpy, pillow, opencv-python(-headless).
"""
from __future__ import annotations

import argparse
import glob
import json
import os
import re
import sys

import cv2
import numpy as np
from PIL import Image


def natural_key(path: str):
    return [int(t) if t.isdigit() else t.lower() for t in re.split(r"(\d+)", os.path.basename(path))]


def pct(v: float, total: float) -> float:
    return round(v / total * 100, 2)


def find_lines(text_mask: np.ndarray, min_gap: int = 4):
    """Group rows containing ink into text lines -> list of (y0, y1)."""
    rows = text_mask.sum(axis=1) > 0
    lines, start, gap = [], None, 0
    for y, on in enumerate(rows):
        if on:
            if start is None:
                start = y
            gap = 0
            end = y
        elif start is not None:
            gap += 1
            if gap > min_gap:
                lines.append((start, end))
                start = None
    if start is not None:
        lines.append((start, end))
    # merge tiny fragments (umlaut dots, commas) into neighbours
    merged = []
    for l in lines:
        if merged and (l[1] - l[0] < 8 or merged[-1][1] - merged[-1][0] < 8) and l[0] - merged[-1][1] < 14:
            merged[-1] = (merged[-1][0], l[1])
        else:
            merged.append(l)
    return merged


def guess_align(text_mask: np.ndarray, lines, tol: int = 12) -> str:
    if len(lines) < 2:
        return "left"
    lefts, rights = [], []
    for y0, y1 in lines:
        cols = np.where(text_mask[y0:y1 + 1].sum(axis=0) > 0)[0]
        if len(cols):
            lefts.append(cols[0])
            rights.append(cols[-1])
    if not lefts:
        return "left"
    l_spread = max(lefts) - min(lefts)
    r_spread = max(rights) - min(rights)
    centers = [(a + b) / 2 for a, b in zip(lefts, rights)]
    c_spread = max(centers) - min(centers)
    if l_spread <= tol:
        return "left"
    if r_spread <= tol:
        return "right"
    if c_spread <= tol * 1.5:
        return "center"
    return "left"


def grow_box(bright: np.ndarray, x0: int, y0: int, x1: int, y1: int, min_frac: float = 0.9, max_grow: int = 400):
    """Grow a bbox outward while the next row/column strip is (almost) all bright background."""
    H, W = bright.shape
    for _ in range(max_grow):
        moved = False
        if y0 > 0 and bright[y0 - 1, x0:x1 + 1].mean() >= min_frac:
            y0 -= 1; moved = True
        if y1 < H - 1 and bright[y1 + 1, x0:x1 + 1].mean() >= min_frac:
            y1 += 1; moved = True
        if x0 > 0 and bright[y0:y1 + 1, x0 - 1].mean() >= min_frac:
            x0 -= 1; moved = True
        if x1 < W - 1 and bright[y0:y1 + 1, x1 + 1].mean() >= min_frac:
            x1 += 1; moved = True
        if not moved:
            break
    return x0, y0, x1, y1


def detect(path: str, bright_thr: int = 200, sat_thr: int = 45, dark_thr: int = 120,
           glyph_max_h: int = 80, glyph_max_w: int = 420, glyph_min_h: int = 4):
    """Text = clusters of small dark holes ("glyphs") inside bright regions."""
    img = np.array(Image.open(path).convert("RGB"))
    H, W = img.shape[:2]
    gray = cv2.cvtColor(img, cv2.COLOR_RGB2GRAY)
    sat = img.max(axis=2).astype(int) - img.min(axis=2).astype(int)
    bright = ((gray > bright_thr) & (sat < sat_thr)).astype(np.uint8)
    dark = gray < dark_thr

    # holes of bright regions = candidate glyphs (only keep glyph-sized ones)
    contours, hier = cv2.findContours(bright, cv2.RETR_CCOMP, cv2.CHAIN_APPROX_SIMPLE)
    ink = np.zeros((H, W), np.uint8)
    if hier is not None:
        for c, h in zip(contours, hier[0]):
            if h[3] == -1:
                continue  # outer boundary, not a hole
            x, y, w, hh = cv2.boundingRect(c)
            if glyph_min_h <= hh <= glyph_max_h and w <= glyph_max_w and w * hh >= 12:
                cv2.drawContours(ink, [c], -1, 1, thickness=-1)
    ink &= dark.astype(np.uint8)
    # cluster glyphs into text blocks (words -> lines -> paragraphs)
    blob = cv2.dilate(ink, cv2.getStructuringElement(cv2.MORPH_RECT, (41, 29)))
    n, lab, stats, _ = cv2.connectedComponentsWithStats(blob, connectivity=8)
    # background with glyphs filled in, so growing a box passes through the lettering
    paper = (bright | ink).astype(np.uint8)
    # 1) raw clusters -> (label ids, text bbox, grown background box)
    raw = []
    for i in range(1, n):
        bx, by, bw, bh = stats[i, :4]
        sub = ink[by:by + bh, bx:bx + bw] * (lab[by:by + bh, bx:bx + bw] == i)
        ng = cv2.connectedComponents(sub.astype(np.uint8), connectivity=8)[0]
        if ng - 1 < 3 or sub.sum() < 150:
            continue
        ys, xs = np.where(sub > 0)
        t = [bx + xs.min(), by + ys.min(), bx + xs.max(), by + ys.max()]
        g = grow_box(paper, max(0, t[0] - 4), max(0, t[1] - 4), min(W - 1, t[2] + 4), min(H - 1, t[3] + 4))
        raw.append({"ids": [i], "t": t, "g": list(g)})

    # 2) merge clusters that sit in the same background box (words of one line, lines of one caption)
    def overlap(g1, g2):
        ix = min(g1[2], g2[2]) - max(g1[0], g2[0])
        iy = min(g1[3], g2[3]) - max(g1[1], g2[1])
        if ix <= 0 or iy <= 0:
            return 0.0
        a1 = (g1[2] - g1[0]) * (g1[3] - g1[1])
        a2 = (g2[2] - g2[0]) * (g2[3] - g2[1])
        return ix * iy / float(min(a1, a2))

    def near(t1, t2):
        hgap = max(t1[0], t2[0]) - min(t1[2], t2[2])
        vgap = max(t1[1], t2[1]) - min(t1[3], t2[3])
        hmin = min(t1[3] - t1[1], t2[3] - t2[1])
        same_line = vgap < -0.5 * hmin and hgap < 90
        next_line = hgap < 0 and 0 <= vgap < 45
        return same_line or next_line

    merged = True
    while merged:
        merged = False
        for p_ in range(len(raw)):
            for q in range(p_ + 1, len(raw)):
                if overlap(raw[p_]["g"], raw[q]["g"]) > 0.6 or near(raw[p_]["t"], raw[q]["t"]):
                    A, B = raw[p_], raw[q]
                    A["ids"] += B["ids"]
                    A["t"] = [min(A["t"][0], B["t"][0]), min(A["t"][1], B["t"][1]), max(A["t"][2], B["t"][2]), max(A["t"][3], B["t"][3])]
                    A["g"] = [min(A["g"][0], B["g"][0]), min(A["g"][1], B["g"][1]), max(A["g"][2], B["g"][2]), max(A["g"][3], B["g"][3])]
                    raw.pop(q)
                    merged = True
                    break
            if merged:
                break

    # 3) stats per merged block
    results = []
    for r in raw:
        ax0, ay0, ax1, ay1 = (int(v) for v in r["t"])
        tm = (ink[ay0:ay1 + 1, ax0:ax1 + 1] > 0) & np.isin(lab[ay0:ay1 + 1, ax0:ax1 + 1], r["ids"])
        tm = tm.astype(np.uint8)
        lines = find_lines(tm)
        heights = [b - a + 1 for a, b in lines]
        pitches = [lines[k + 1][0] - lines[k][0] for k in range(len(lines) - 1)]
        med_h = float(np.median(heights))
        if not (18 <= med_h <= 75):
            continue
        gx0, gy0, gx1, gy1 = (int(v) for v in r["g"])
        region = bright[gy0:gy1 + 1, gx0:gx1 + 1] > 0
        bg = np.median(img[gy0:gy1 + 1, gx0:gx1 + 1][region], axis=0).astype(int) if region.any() else np.array([255, 255, 255])
        bw_, bh_ = gx1 - gx0 + 1, gy1 - gy0 + 1
        tb = [ax0, ay0, ax1 - ax0 + 1, ay1 - ay0 + 1]
        results.append({
            "box": {
                "px": [gx0, gy0, bw_, bh_],
                "pct": [pct(gx0, W), pct(gy0, H), pct(bw_, W), pct(bh_, H)],
                "bg": "#%02x%02x%02x" % tuple(bg),
                "touches_edge": bool(gx0 == 0 or gy0 == 0 or gx1 == W - 1 or gy1 == H - 1),
            },
            "text": {
                "px": tb,
                "pct": [pct(tb[0], W), pct(tb[1], H), pct(tb[2], W), pct(tb[3], H)],
                "lines": len(lines),
                "line_boxes_px": [[int(ay0 + a), int(b - a + 1)] for a, b in lines],
                "mean_glyph_height_px": round(float(np.mean(heights)), 1),
                "mean_line_pitch_px": round(float(np.median(pitches)), 1) if pitches else None,
                "size_pct_width": round(float(np.median(pitches)) / W * 100 / 1.15, 2) if pitches else None,
                "align": guess_align(tm, lines),
            },
        })
    results.sort(key=lambda r: (r["text"]["px"][1], r["text"]["px"][0]))
    return {"file": os.path.basename(path), "width": W, "height": H, "candidates": results}


def save_crops(path: str, det: dict, out_dir: str, stem: str, margin: int = 10):
    img = Image.open(path).convert("RGB")
    for i, c in enumerate(det["candidates"], 1):
        x, y, w, h = c["box"]["px"]
        img.crop((max(0, x - margin), max(0, y - margin), x + w + margin, y + h + margin)).save(
            os.path.join(out_dir, f"{stem}-{i}.jpg"), quality=90)


def save_overlay(path: str, det: dict, out_dir: str, stem: str):
    img = np.array(Image.open(path).convert("RGB"))
    for i, c in enumerate(det["candidates"], 1):
        x, y, w, h = c["box"]["px"]
        cv2.rectangle(img, (x, y), (x + w, y + h), (0, 160, 255), 4)
        x, y, w, h = c["text"]["px"]
        cv2.rectangle(img, (x, y), (x + w, y + h), (255, 0, 0), 3)
        cv2.putText(img, str(i), (x, max(30, y - 8)), cv2.FONT_HERSHEY_SIMPLEX, 1.2, (255, 0, 0), 3)
    Image.fromarray(img).save(os.path.join(out_dir, f"{stem}-overlay.jpg"), quality=80)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("images", nargs="+", help="image paths or glob patterns")
    ap.add_argument("--json", help="write all detections to this JSON file")
    ap.add_argument("--crops", help="directory for per-box crops (and overlays with --overlay)")
    ap.add_argument("--overlay", action="store_true", help="also save an overlay per slice into --crops")
    ap.add_argument("--bright", type=int, default=200, help="min gray level of box background")
    ap.add_argument("--dark", type=int, default=120, help="max gray level of glyph ink")
    args = ap.parse_args(argv)

    paths = []
    for p in args.images:
        paths.extend(glob.glob(p) or [p])
    paths = sorted(set(paths), key=natural_key)
    if args.crops:
        os.makedirs(args.crops, exist_ok=True)

    out = []
    for p in paths:
        det = detect(p, bright_thr=args.bright, dark_thr=args.dark)
        m = re.search(r"(\d+)\D*$", os.path.splitext(os.path.basename(p))[0])
        stem = f"s{int(m.group(1)):02d}" if m else os.path.splitext(os.path.basename(p))[0]
        det["stem"] = stem
        out.append(det)
        if args.crops:
            save_crops(p, det, args.crops, stem)
            if args.overlay:
                save_overlay(p, det, args.crops, stem)
        summary = ", ".join(
            f"[{c['text']['pct'][0]},{c['text']['pct'][1]} {c['text']['lines']}L bg{c['box']['bg']}]"
            for c in det["candidates"])
        print(f"{stem}: {len(det['candidates'])} {summary}")
    if args.json:
        with open(args.json, "w", encoding="utf-8") as f:
            json.dump(out, f, indent=1)
    return 0


if __name__ == "__main__":
    sys.exit(main())
