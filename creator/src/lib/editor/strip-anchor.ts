// Re-anchoring across a seam (agents/research/TEXT-ACROSS-SLICES.md §3, §5 step 2): a text box or a placed item dropped
// with its centre on another slice of the strip moves there – the entry leaves its slice's list and joins the other's,
// its id kept (showTextBox events, pickups by placement id), its place on screen kept: the anchor point, the size and a
// free tail's tip (all % of the slice) converted between the two slices' drawn boxes. Pure – the strip measures.

/** a slice's drawn box on screen (px) */
export interface DrawnBox {
	left: number;
	top: number;
	width: number;
	height: number;
}

/** what moves: text boxes (`dialog`) and placed items (`items`) – areas and collage pieces stay with their slice */
export type AnchoredList = 'dialog' | 'items';

type Obj = Record<string, any>;
const isNum = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);
const round2 = (v: number) => Math.round(v * 100) / 100;

/** a point in % of slice `from` → % of slice `to` (the same spot on screen) */
export function convertPoint(p: { x: number; y: number }, from: DrawnBox, to: DrawnBox): { x: number; y: number } {
	return {
		x: round2(((from.left + (p.x / 100) * from.width - to.left) / to.width) * 100),
		y: round2(((from.top + (p.y / 100) * from.height - to.top) / to.height) * 100)
	};
}

/** a rect (`x` / `y` = the anchor point, `width`, `height` optional – % of the slice) → the other slice's % */
export function convertRect<R extends Obj>(rect: R, from: DrawnBox, to: DrawnBox): R {
	const out: Obj = { ...rect };
	if (isNum(rect.x) && isNum(rect.y)) Object.assign(out, convertPoint({ x: rect.x, y: rect.y }, from, to));
	if (isNum(rect.width)) out.width = round2((rect.width * from.width) / to.width);
	if (isNum(rect.height)) out.height = round2((rect.height * from.height) / to.height);
	return out as R;
}

/**
 * Moves entry `index` of `list` from slice `from` to slice `to` of `slices` (the group's, edited in place), converting its
 * geometry from the box `fromBox` to `toBox`. Returns its index on the new slice (appended: the front of that slice's
 * stack), or -1 when nothing moved.
 */
export function moveAcrossSeam(slices: Obj[], list: AnchoredList, from: number, index: number, to: number, fromBox: DrawnBox, toBox: DrawnBox): number {
	if (from === to || !slices[from] || !slices[to] || !toBox.width || !toBox.height) return -1;
	const source = slices[from][list];
	if (!Array.isArray(source) || !source[index]) return -1;
	const [entry] = source.splice(index, 1);
	if (!source.length) delete slices[from][list];
	if (entry.rect && typeof entry.rect === 'object') entry.rect = convertRect(entry.rect, fromBox, toBox);
	// a drawn bubble's free tail points at a spot of the slice (dialog-tail.ts): the same spot
	if (list === 'dialog' && entry.tail && typeof entry.tail === 'object' && entry.tail.tip && isNum(entry.tail.tip.x) && isNum(entry.tail.tip.y)) {
		entry.tail = { ...entry.tail, tip: convertPoint(entry.tail.tip, fromBox, toBox) };
	}
	const target = Array.isArray(slices[to][list]) ? slices[to][list] : (slices[to][list] = []);
	target.push(entry);
	return target.length - 1;
}

/** the slice whose drawn box holds `point` along the strip's axis (vertical: y, horizontal: x), -1 = none */
export function sliceAt(boxes: (DrawnBox | null)[], point: { x: number; y: number }, axis: 'vertical' | 'horizontal'): number {
	return boxes.findIndex((b) => !!b && (axis === 'vertical' ? point.y >= b.top && point.y < b.top + b.height : point.x >= b.left && point.x < b.left + b.width));
}
