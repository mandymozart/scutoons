/**
 * Free speech-bubble tails (owner 2026-10-18, agents/DESIGN.md Part 2 "Tails"): a drawn bubble's tail runs from one
 * point on the bubble's outline (`base`: a side and a place 0 … 1 along it) to a free tip anywhere on – or past – the
 * slice (`tip`, % of the slice, may lie outside 0 … 100: an off-panel speaker; the reader cuts the tail at the slice's
 * edge). Two kinds: `speech` = the pointed wedge, `thought` = a chain of circles getting smaller towards the head.
 * The older fixed tails (`DIALOG_TAILS`: 'bottom-left' … 'none') are still read and drawn as before; the creator turns
 * one into a free tail on its first edit (`freeTailFrom`).
 *
 * The pixel geometry here is shared by the reader (draws it) and the creator (its handles): no Svelte, no DOM.
 */
import type { Dialog, DialogTail } from './content.ts';

export const TAIL_KINDS = ['speech', 'thought'] as const;
export type TailKind = (typeof TAIL_KINDS)[number];
export const TAIL_SIDES = ['top', 'right', 'bottom', 'left'] as const;
export type TailSide = (typeof TAIL_SIDES)[number];

/** A free tail (`Dialog.tail`): one base on the bubble's outline, one tip on the slice. */
export interface FreeTail {
	kind: TailKind;
	/** where the tail points (the speaker's mouth / head), % of the slice; outside 0 … 100 = off the slice */
	tip: { x: number; y: number };
	/** where it leaves the bubble: a side and the place along it (0 = its start: left / top, 1 = its end: right / bottom) */
	base: { side: TailSide; at: number };
}

/** the distance the tail's base keeps from the bubble's corners, in rem (the base never crosses a corner) */
export const TAIL_CORNER_INSET_REM = 1;
/** a speech tail's base width, in em of the bubble's font (capped so it fits between the corner insets) */
export const TAIL_BASE_EM = 1.5;
/**
 * A thought tail's chain (owner 2026-10-18: "more bubbles must come when it gets longer … the size grows with the
 * length"), radii in em of the font: the circle at the bubble grows from `first[0]` (a short tail, ≤ `lengthEm[0]`) to
 * `first[1]` (a long one, ≥ `lengthEm[1]`), the one on the tip from `tip[0]` to `tip[1]`; `count` circles at the least /
 * most; `pitch` = the centre distance in sums of the two neighbours' radii (1.4 = a gap of 0.4 × the pair: even, never
 * floating); `lift` = the first circle's gap off the outline, in its radius.
 */
export const THOUGHT_CHAIN = {
	first: [0.6, 0.9],
	tip: [0.2, 0.25],
	lengthEm: [4, 24],
	count: [3, 24],
	pitch: 1.4,
	lift: 0.3
} as const;

export function isFreeTail(tail: unknown): tail is FreeTail {
	if (!tail || typeof tail !== 'object') return false;
	const t = tail as Record<string, any>;
	return (
		(TAIL_KINDS as readonly string[]).includes(t.kind) &&
		!!t.tip && Number.isFinite(t.tip.x) && Number.isFinite(t.tip.y) &&
		!!t.base && (TAIL_SIDES as readonly string[]).includes(t.base.side) && Number.isFinite(t.base.at)
	);
}

/** a text box is a drawn bubble (the only kind with a tail): variant bubble + frame */
export function isDrawnBubble(d: Pick<Dialog, 'variant' | 'frame'> | null | undefined): boolean {
	return !!d && (d.variant ?? 'bubble') === 'bubble' && d.frame === true;
}

/**
 * The tail a text box draws: a free one, an old fixed one (its default: 'bottom-left' for a box, none for a wobbly
 * bubble) or null (none, or not a drawn bubble).
 */
export function dialogTail(d: Dialog | null | undefined): FreeTail | Exclude<DialogTail, 'none'> | null {
	if (!d || !isDrawnBubble(d)) return null;
	if (isFreeTail(d.tail)) return d.tail;
	const fixed = typeof d.tail === 'string' ? d.tail : d.shape === 'wobbly' ? 'none' : 'bottom-left';
	return fixed === 'none' ? null : (fixed as Exclude<DialogTail, 'none'>);
}

/** the inspector's choice: none / speech / thought (an old fixed tail is a speech tail) */
export function tailKindOf(d: Dialog | null | undefined): TailKind | 'none' {
	const t = dialogTail(d);
	return t === null ? 'none' : typeof t === 'string' ? 'speech' : t.kind;
}

const r2 = (v: number) => Math.round(v * 100) / 100;
const r3 = (v: number) => Math.round(v * 1000) / 1000;

/** a drawn box in % of the slice (its top-left corner and size, before its turn) */
export interface TailBox {
	left: number;
	top: number;
	width: number;
	height: number;
}

/**
 * A new free tail for a bubble drawn at `box` (owner 2026-10-18: base on the bottom at 0.3, the tip a bit below-left).
 * `ratio` = the slice's height / width (the tip's distance looks the same across and down).
 */
export function defaultTail(kind: TailKind, box: TailBox, ratio = 625 / 390): FreeTail {
	const at = 0.3;
	const x = box.left + box.width * at;
	const y = box.top + box.height;
	return { kind, tip: { x: r2(x - 5), y: r2(y + 5 / ratio + 4) }, base: { side: 'bottom', at } };
}

/**
 * An old fixed tail as a free one that looks the same (the creator converts on the first edit): the base where the old
 * tail sat, the tip about one em past the side. `size` = the font size in % of the slice width.
 */
export function freeTailFrom(fixed: Exclude<DialogTail, 'none'>, box: TailBox, size = 4, ratio = 625 / 390, kind: TailKind = 'speech'): FreeTail {
	const top = fixed.startsWith('top');
	const where = fixed.endsWith('left') ? 'left' : fixed.endsWith('right') ? 'right' : 'centre';
	const at = where === 'left' ? 0.25 : where === 'right' ? 0.75 : 0.5;
	const x = where === 'left' ? box.left + box.width * 0.22 : where === 'right' ? box.left + box.width * 0.78 : box.left + box.width / 2;
	const em = (size * 1.2) / ratio;
	return { kind, tip: { x: r2(x), y: r2(top ? box.top - em : box.top + box.height + em) }, base: { side: top ? 'top' : 'bottom', at } };
}

// ── pixel geometry (reader + creator) ────────────────────────────────────────────────────────────────────────────

export interface Pt {
	x: number;
	y: number;
}

/** a laid-out box on the slice, in px: its top-left corner (after the anchor's translate) and size, its turn and scale around its centre */
export interface TailFrame {
	left: number;
	top: number;
	width: number;
	height: number;
	/** degrees */
	rotation: number;
	scale: number;
}

const RAD = Math.PI / 180;

/** a slice point (px) in the box's own frame (px from its top-left corner, before turn and scale) */
export function toLocal(f: TailFrame, p: Pt): Pt {
	const cx = f.left + f.width / 2;
	const cy = f.top + f.height / 2;
	const a = f.rotation * RAD;
	const k = f.scale || 1;
	const dx = p.x - cx;
	const dy = p.y - cy;
	return { x: f.width / 2 + (Math.cos(a) * dx + Math.sin(a) * dy) / k, y: f.height / 2 + (-Math.sin(a) * dx + Math.cos(a) * dy) / k };
}

/** a point of the box's own frame (px) on the slice (px) */
export function toSlice(f: TailFrame, q: Pt): Pt {
	const a = f.rotation * RAD;
	const k = f.scale || 1;
	const lx = (q.x - f.width / 2) * k;
	const ly = (q.y - f.height / 2) * k;
	return { x: f.left + f.width / 2 + Math.cos(a) * lx - Math.sin(a) * ly, y: f.top + f.height / 2 + Math.sin(a) * lx + Math.cos(a) * ly };
}

export interface TailBase {
	/** the base's middle on the outline (box frame, px) */
	point: Pt;
	/** along the side (unit) */
	tangent: Pt;
	/** out of the box (unit) */
	normal: Pt;
	/** the side's length (px) */
	length: number;
	/** the place along the side after the corner inset (0 … 1) */
	at: number;
}

/**
 * The base on a `width` × `height` box (its own frame, px): `at` along the side, kept `inset` px from both corners (a
 * side shorter than two insets: its middle).
 */
export function tailBase(side: TailSide, at: number, width: number, height: number, inset: number): TailBase {
	const horizontal = side === 'top' || side === 'bottom';
	const length = horizontal ? width : height;
	const along = length <= 2 * inset ? length / 2 : Math.min(length - inset, Math.max(inset, (Number.isFinite(at) ? at : 0.5) * length));
	const clamped = length > 0 ? along / length : 0.5;
	switch (side) {
		case 'top':
			return { point: { x: along, y: 0 }, tangent: { x: 1, y: 0 }, normal: { x: 0, y: -1 }, length, at: clamped };
		case 'bottom':
			return { point: { x: along, y: height }, tangent: { x: 1, y: 0 }, normal: { x: 0, y: 1 }, length, at: clamped };
		case 'left':
			return { point: { x: 0, y: along }, tangent: { x: 0, y: 1 }, normal: { x: -1, y: 0 }, length, at: clamped };
		default:
			return { point: { x: width, y: along }, tangent: { x: 0, y: 1 }, normal: { x: 1, y: 0 }, length, at: clamped };
	}
}

/**
 * The base for a pointer in the box's own frame (the creator's base handle): the nearest side – it jumps sides at the
 * corners – and the place along it, kept `inset` px from the corners; `at` rounded to 3 places.
 */
export function nearestBase(q: Pt, width: number, height: number, inset: number): { side: TailSide; at: number } {
	const cx = Math.min(width, Math.max(0, q.x));
	const cy = Math.min(height, Math.max(0, q.y));
	const candidates: [TailSide, number][] = [
		['top', Math.hypot(q.x - cx, q.y)],
		['bottom', Math.hypot(q.x - cx, q.y - height)],
		['left', Math.hypot(q.x, q.y - cy)],
		['right', Math.hypot(q.x - width, q.y - cy)]
	];
	candidates.sort((a, b) => a[1] - b[1]);
	const side = candidates[0][0];
	const horizontal = side === 'top' || side === 'bottom';
	const raw = horizontal ? (width > 0 ? cx / width : 0.5) : height > 0 ? cy / height : 0.5;
	return { side, at: r3(tailBase(side, raw, width, height, inset).at) };
}

/** the half width of a speech tail's base: `TAIL_BASE_EM` of the font, never past the corner inset or half the side */
export function tailHalfWidth(fontPx: number, inset: number, length: number): number {
	return Math.max(1, Math.min((TAIL_BASE_EM * fontPx) / 2, inset * 0.9, length / 2 - 1));
}

const add = (p: Pt, v: Pt, k: number): Pt => ({ x: p.x + v.x * k, y: p.y + v.y * k });
const f1 = (n: number) => Math.round(n * 10) / 10;
const pt = (p: Pt) => `${f1(p.x)} ${f1(p.y)}`;

/** the tip lies out of the box past the base's side (else the tail would run through the bubble: not drawn) */
export function tipOutside(base: TailBase, tip: Pt): boolean {
	return (tip.x - base.point.x) * base.normal.x + (tip.y - base.point.y) * base.normal.y > 0.5;
}

/** how far `tip` lies out past `side` of a `width` × `height` box (its own frame, px; ≤ 0 = on the bubble's side) */
function pastSide(side: TailSide, tip: Pt, width: number, height: number): number {
	return side === 'top' ? -tip.y : side === 'bottom' ? tip.y - height : side === 'left' ? -tip.x : tip.x - width;
}

/**
 * The side facing the tip (owner 2026-10-18, "Basis springt mit"): of the sides the tip lies out past, the one whose
 * outward direction best matches the direction from the box's centre to the tip; null = the tip is inside the bubble.
 * `tip` in the box's own frame (px), like `tipOutside`.
 */
export function facingSide(tip: Pt, width: number, height: number): TailSide | null {
	const dx = tip.x - width / 2;
	const dy = tip.y - height / 2;
	const len = Math.hypot(dx, dy) || 1;
	const outward: Record<TailSide, number> = { top: -dy / len, bottom: dy / len, left: -dx / len, right: dx / len };
	let best: TailSide | null = null;
	for (const side of TAIL_SIDES) if (pastSide(side, tip, width, height) > 0.5 && (best === null || outward[side] > outward[best])) best = side;
	return best;
}

/** the side a tail is drawn from: the stored one while the tip lies out past it, else the side facing the tip (or null) */
export function reachingSide(side: TailSide, tip: Pt, width: number, height: number): TailSide | null {
	return pastSide(side, tip, width, height) > 0.5 ? side : facingSide(tip, width, height);
}

/**
 * The tail with its base on the side that reaches its tip (the creator stores it after a tip drag or a bubble move, so
 * the data matches what is drawn): the same object while the stored side works or the tip is inside the bubble, else a
 * copy with the facing side and the same `at` (`tailBase` keeps it off the corners). `slice` = the slice in px.
 */
export function followTip(tail: FreeTail, frame: TailFrame, slice: { w: number; h: number }): FreeTail {
	if (!slice.w || !slice.h || !frame.width || !frame.height) return tail;
	const tip = toLocal(frame, { x: (tail.tip.x / 100) * slice.w, y: (tail.tip.y / 100) * slice.h });
	const side = reachingSide(tail.base.side, tip, frame.width, frame.height);
	return side === null || side === tail.base.side ? tail : { ...tail, base: { side, at: tail.base.at } };
}

/**
 * A speech tail (box frame, px): `fill` = the wedge reaching `depth` px into the bubble, so its paper covers the outline
 * between the base's two points (no line across the join); `stroke` = only its two sides, base → tip → base.
 */
export function speechTail(base: TailBase, tip: Pt, halfWidth: number, depth: number): { fill: string; stroke: string } {
	const a = add(base.point, base.tangent, -halfWidth);
	const b = add(base.point, base.tangent, halfWidth);
	const ai = add(a, base.normal, -depth);
	const bi = add(b, base.normal, -depth);
	return { fill: `M${pt(ai)} L${pt(a)} L${pt(tip)} L${pt(b)} L${pt(bi)} Z`, stroke: `M${pt(a)} L${pt(tip)} L${pt(b)}` };
}

/** one circle of a thought tail's chain: its radius and its centre's distance from the base along the line (px) */
export interface ThoughtCircle {
	r: number;
	d: number;
}

/**
 * The thought chain's layout for a tail `length` px long (base → tip) at `fontPx` (owner 2026-10-18, DESIGN.md
 * "Tails"): the count follows the length (`THOUGHT_CHAIN.count`, as near to an even `pitch` as the count allows), the
 * radii shrink steadily (geometric) from the circle at the bubble to the one on the tip, both growing a little with the
 * length; neighbours keep a gap in proportion to their sizes; the first circle sits `lift` × its radius off the
 * outline, the last one's centre on the tip. `lead` ≥ 1 stretches the first circle's distance (a tail leaving its side
 * at a slant: the circle still clears the outline). A tail too short for three full circles shrinks them all.
 */
export function thoughtLayout(length: number, fontPx: number, lead = 1): ThoughtCircle[] {
	const C = THOUGHT_CHAIN;
	const em = fontPx > 0 ? fontPx : 16;
	const L = Math.max(0, length);
	const t = Math.min(1, Math.max(0, (L / em - C.lengthEm[0]) / (C.lengthEm[1] - C.lengthEm[0])));
	const r0 = (C.first[0] + (C.first[1] - C.first[0]) * t) * em;
	const rn = (C.tip[0] + (C.tip[1] - C.tip[0]) * t) * em;
	const k = Math.max(1, lead);
	const radiiFor = (n: number) => Array.from({ length: n }, (_, i) => r0 * Math.pow(rn / r0, i / (n - 1)));
	// the pitch a count needs so the chain ends on the tip: (L − first centre) / Σ (neighbour pairs)
	const pitchFor = (radii: number[]) => {
		let pairs = 0;
		for (let i = 1; i < radii.length; i++) pairs += radii[i - 1] + radii[i];
		return (L - r0 * (1 + C.lift) * k) / pairs;
	};
	let best = radiiFor(C.count[0]);
	let bestPitch = pitchFor(best);
	for (let n = C.count[0] + 1; n <= C.count[1]; n++) {
		const radii = radiiFor(n);
		const p = pitchFor(radii);
		if (Math.abs(p - C.pitch) < Math.abs(bestPitch - C.pitch)) {
			best = radii;
			bestPitch = p;
		}
	}
	let radii = best;
	let pitch = bestPitch;
	// too short for three full circles at an even pitch: all shrink until they fit
	if (pitch < C.pitch && radii.length === C.count[0]) {
		let pairs = 0;
		for (let i = 1; i < radii.length; i++) pairs += radii[i - 1] + radii[i];
		const scale = L / (r0 * (1 + C.lift) * k + C.pitch * pairs);
		radii = radii.map((r) => r * scale);
		pitch = C.pitch;
	}
	const out: ThoughtCircle[] = [];
	let d = radii[0] * (1 + C.lift) * k;
	for (let i = 0; i < radii.length; i++) {
		if (i > 0) d += pitch * (radii[i - 1] + radii[i]);
		out.push({ r: radii[i], d });
	}
	out[out.length - 1].d = L;
	return out;
}

/**
 * A thought tail (box frame, px): the chain of `thoughtLayout` on the straight line from the base to the tip – the
 * largest just off the bubble, the smallest on the tip.
 */
export function thoughtTail(base: TailBase, tip: Pt, fontPx: number): { cx: number; cy: number; r: number }[] {
	const dx = tip.x - base.point.x;
	const dy = tip.y - base.point.y;
	const len = Math.hypot(dx, dy) || 1;
	const u = { x: dx / len, y: dy / len };
	// the share of the line that leaves the side (1 = straight out): a slanted tail moves its first circle further along
	const out = u.x * base.normal.x + u.y * base.normal.y;
	const lead = 1 / Math.max(0.2, out);
	return thoughtLayout(len, fontPx, lead).map(({ r, d }) => {
		const c = add(base.point, u, d);
		return { cx: f1(c.x), cy: f1(c.y), r: f1(r) };
	});
}

/** a rectangle as a path (for clip paths) */
export function rectPath(x: number, y: number, w: number, h: number): string {
	return `M${f1(x)} ${f1(y)} H${f1(x + w)} V${f1(y + h)} H${f1(x)} Z`;
}

/** a polygon as a path */
export function polygonPath(points: Pt[]): string {
	return points.length ? `M${points.map(pt).join(' L')} Z` : '';
}
