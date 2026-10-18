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
/** a thought tail's circles, largest first (radius in em of the font): they get smaller towards the head */
export const THOUGHT_RADII_EM = [0.5, 0.34, 0.22] as const;

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

/**
 * A thought tail (box frame, px): three circles from the outline to the tip, getting smaller towards it – the largest
 * just off the bubble, the smallest on the tip, the middle one between.
 */
export function thoughtTail(base: TailBase, tip: Pt, fontPx: number, gap = 0.2): { cx: number; cy: number; r: number }[] {
	const [r1, r2, r3] = THOUGHT_RADII_EM.map((r) => r * fontPx);
	const dx = tip.x - base.point.x;
	const dy = tip.y - base.point.y;
	const len = Math.hypot(dx, dy) || 1;
	const u = { x: dx / len, y: dy / len };
	const first = r1 + gap * fontPx;
	const c1 = add(base.point, u, first);
	const c3 = tip;
	// the middle circle a little nearer the bubble than half way (the chain bunches towards the bubble)
	const mid = first + Math.max(0, len - first) * 0.45;
	const c2 = add(base.point, u, Math.max(mid, first + r1 + r2 + gap * fontPx * 0.5));
	return [
		{ cx: f1(c1.x), cy: f1(c1.y), r: f1(r1) },
		{ cx: f1(c2.x), cy: f1(c2.y), r: f1(r2) },
		{ cx: f1(c3.x), cy: f1(c3.y), r: f1(r3) }
	];
}

/** a rectangle as a path (for clip paths) */
export function rectPath(x: number, y: number, w: number, h: number): string {
	return `M${f1(x)} ${f1(y)} H${f1(x + w)} V${f1(y + h)} H${f1(x)} Z`;
}

/** a polygon as a path */
export function polygonPath(points: Pt[]): string {
	return points.length ? `M${points.map(pt).join(' L')} Z` : '';
}
