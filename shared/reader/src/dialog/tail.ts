// A drawn bubble's free tail (owner 2026-10-18, agents/DESIGN.md Part 2 "Tails"): measured from the laid-out box and
// the slice, drawn in px of the box's own frame (before its turn and scale – the SVG sits inside the box, so it turns,
// scales and plays its effects with it). The geometry is @scutoons/content dialog-tail.ts (the creator's handles use it
// too); this file only reads the DOM.
import {
	TAIL_CORNER_INSET_REM,
	anchorFactors,
	polygonPath,
	rectPath,
	speechTail,
	tailBase,
	tailHalfWidth,
	thoughtTail,
	tipOutside,
	toLocal,
	type Dialog,
	type FreeTail,
	type Pt,
	type TailBase,
	type TailFrame
} from '@scutoons/content';
import { wobbleAmplitude } from './wobbly.ts';

export interface TailDrawing {
	kind: FreeTail['kind'];
	/** the box's border edge → the SVG's origin (the padding edge) */
	offset: Pt;
	/** the slice's box (the tail is cut at its edge: an off-panel speaker) */
	slice: string;
	/** the slice minus the bubble's inside (evenodd): the wedge's lines stop at the outline */
	outside: string;
	speech: { fill: string; stroke: string } | null;
	circles: { cx: number; cy: number; r: number }[];
	/** the outline's width (px), as the bubble's border */
	line: number;
}

/** the laid-out root box on the slice (px): its top-left after the anchor's translate, its turn and scale */
export function tailFrame(root: HTMLElement, dialog: Dialog): TailFrame {
	const { ax, ay } = anchorFactors(dialog.anchor);
	const width = root.offsetWidth;
	const height = root.offsetHeight;
	return { left: root.offsetLeft - ax * width, top: root.offsetTop - ay * height, width, height, rotation: dialog.rotation ?? 0, scale: dialog.scale ?? 1 };
}

/**
 * The tail for a bubble box (`.box` inside the dialog `root`), or null while it can't be measured (not laid out) or its
 * tip lies on the bubble's side of the base (a tail through the bubble). `wobblyD` = a wobbly bubble's outline path.
 */
export function measureTail(root: HTMLElement, box: HTMLElement, dialog: Dialog, tail: FreeTail, wobblyD: string | null): TailDrawing | null {
	const slice = root.offsetParent as HTMLElement | null;
	const w = box.offsetWidth;
	const h = box.offsetHeight;
	if (!slice || !slice.clientWidth || !slice.clientHeight || !w || !h) return null;
	const frame = tailFrame(root, dialog);
	// the box's place inside the root (effects may wrap it)
	let ox = 0;
	let oy = 0;
	for (let el: HTMLElement | null = box; el && el !== root; el = el.offsetParent as HTMLElement | null) {
		ox += el.offsetLeft;
		oy += el.offsetTop;
	}
	const local = (p: Pt): Pt => {
		const q = toLocal(frame, p);
		return { x: q.x - ox, y: q.y - oy };
	};
	const S = { w: slice.clientWidth, h: slice.clientHeight };
	const tip = local({ x: (tail.tip.x / 100) * S.w, y: (tail.tip.y / 100) * S.h });
	const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
	const fontPx = parseFloat(getComputedStyle(box).fontSize) || rem;
	const inset = (TAIL_CORNER_INSET_REM * rem) / (frame.scale || 1);
	const border = box.clientLeft;
	const offset = { x: box.clientLeft, y: box.clientTop };
	// where the outline runs: a box's border (the lines start at its inner edge, so they overlap it); a wobbly outline
	// lies within twice its amplitude of the box's edge
	const edge = wobblyD ? 2 * wobbleAmplitude(w, h) : border;
	const inner = (b: TailBase): TailBase => ({ ...b, point: { x: b.point.x - b.normal.x * edge, y: b.point.y - b.normal.y * edge } });
	const outer = tailBase(tail.base.side, tail.base.at, w, h, inset);
	if (!tipOutside(outer, tip)) return null;
	const sliceCorners = [local({ x: 0, y: 0 }), local({ x: S.w, y: 0 }), local({ x: S.w, y: S.h }), local({ x: 0, y: S.h })];
	const slicePath = polygonPath(sliceCorners);
	const insidePath = wobblyD ?? rectPath(border, box.clientTop, w - 2 * border, h - 2 * box.clientTop);
	const line = wobblyD ? 2.5 : Math.max(1, border);
	const drawing: TailDrawing = { kind: tail.kind, offset, slice: slicePath, outside: `${slicePath} ${insidePath}`, speech: null, circles: [], line };
	if (tail.kind === 'speech') {
		const half = tailHalfWidth(fontPx, inset, outer.length);
		drawing.speech = speechTail(inner(outer), tip, half, 1);
	} else {
		drawing.circles = thoughtTail(outer, tip, fontPx);
	}
	return drawing;
}
