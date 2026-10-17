/**
 * The 9-point anchor of a text box / placed item (owner 2026-10-17, like Figma's 3×3 reference point – not its
 * constraints; agents/DESIGN.md Part 2 "Anchor", agents/ITEMS.md "Placed items"). `rect.x` / `rect.y` are the anchor
 * point's position in % of the slice, `width` / `height` the box's size; the box grows away from that point when its
 * drawn size differs from the stored one (more lines in another language, an auto height, an icon's own ratio).
 * No anchor = 'tl' = the top-left corner, exactly the old meaning (no migration).
 */
export const ANCHORS = ['tl', 't', 'tr', 'l', 'c', 'r', 'bl', 'b', 'br'] as const;
export type Anchor = (typeof ANCHORS)[number];

export interface AnchorFactors {
	/** 0 = left, 0.5 = centre, 1 = right */
	ax: number;
	/** 0 = top, 0.5 = middle, 1 = bottom */
	ay: number;
}

export function isAnchor(value: unknown): value is Anchor {
	return typeof value === 'string' && (ANCHORS as readonly string[]).includes(value);
}

/** where the anchor sits in the box (an unknown / absent one = top-left) */
export function anchorFactors(anchor: unknown): AnchorFactors {
	if (!isAnchor(anchor) || anchor === 'tl') return { ax: 0, ay: 0 };
	const ax = anchor === 't' || anchor === 'c' || anchor === 'b' ? 0.5 : anchor.endsWith('r') ? 1 : 0;
	const ay = anchor === 'l' || anchor === 'c' || anchor === 'r' ? 0.5 : anchor.startsWith('b') ? 1 : 0;
	return { ax, ay };
}

/** the anchor at these factors (0 / 0.5 / 1) */
export function anchorAt(ax: number, ay: number): Anchor {
	const col = ax < 0.25 ? 0 : ax > 0.75 ? 2 : 1;
	const row = ay < 0.25 ? 0 : ay > 0.75 ? 2 : 1;
	return ANCHORS[row * 3 + col];
}

export interface PercentBox {
	x: number;
	y: number;
	width: number;
	height?: number;
}

/**
 * The box's top-left corner and size in % of the slice, from its stored rect and anchor. `height` = the drawn height
 * when the rect has none (an auto-height text box, an item at its icon's ratio); without either the vertical anchor
 * counts with 0 (the top stays).
 */
export function anchoredRect(rect: PercentBox, anchor: unknown, height?: number): { left: number; top: number; width: number; height: number } {
	const { ax, ay } = anchorFactors(anchor);
	const h = rect.height ?? height ?? 0;
	return { left: rect.x - rect.width * ax, top: rect.y - h * ay, width: rect.width, height: h };
}

/** the stored x / y for a box whose top-left corner is at `left` / `top` (the inverse of `anchoredRect`) */
export function anchorPoint(left: number, top: number, width: number, height: number, anchor: unknown): { x: number; y: number } {
	const { ax, ay } = anchorFactors(anchor);
	return { x: left + width * ax, y: top + height * ay };
}

/**
 * x / y for another anchor that keeps the box where it is: x' = x + width·(ax' − ax), y' = y + height·(ay' − ay);
 * `height` = the drawn height when the rect has none. Rounded to 2 places.
 */
export function reanchor(rect: PercentBox, from: unknown, to: unknown, height?: number): { x: number; y: number } {
	const a = anchorFactors(from);
	const b = anchorFactors(to);
	const h = rect.height ?? height ?? 0;
	const r = (v: number) => Math.round(v * 100) / 100;
	return { x: r(rect.x + rect.width * (b.ax - a.ax)), y: r(rect.y + h * (b.ay - a.ay)) };
}

/** CSS for the reader: `left` / `top` at the anchor point, `translate` pulls the drawn box back by its own size */
export function anchorTranslate(anchor: unknown): string | null {
	const { ax, ay } = anchorFactors(anchor);
	return ax === 0 && ay === 0 ? null : `${-ax * 100}% ${-ay * 100}%`;
}

/**
 * The anchor a box dragged near a slice edge or corner could take (within `margin` % of it) – a hint only, nothing is
 * changed by it; null = not near an edge, or it already has that anchor.
 */
export function suggestedAnchor(box: { left: number; top: number; width: number; height: number }, current: unknown, margin = 3): Anchor | null {
	const nearL = Math.abs(box.left) <= margin;
	const nearR = Math.abs(box.left + box.width - 100) <= margin;
	const nearT = Math.abs(box.top) <= margin;
	const nearB = Math.abs(box.top + box.height - 100) <= margin;
	if (!nearL && !nearR && !nearT && !nearB) return null;
	const cur = anchorFactors(current);
	const ax = nearL && !nearR ? 0 : nearR && !nearL ? 1 : cur.ax;
	const ay = nearT && !nearB ? 0 : nearB && !nearT ? 1 : cur.ay;
	const next = anchorAt(ax, ay);
	return next === anchorAt(cur.ax, cur.ay) ? null : next;
}
