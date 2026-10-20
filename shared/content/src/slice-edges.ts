// What may cross a slice's edge (agents/research/TEXT-ACROSS-SLICES.md §3, §7): a group is one continuous strip, its
// slices are the background tiles; a text box, an item or an area belongs to (is anchored on) a slice but may reach over
// its edges. Only the layout's WINDOW cuts it:
//   vertical  the column – left / right cut; up / down open, except above the group's first and below its last slice
//   strip     (horizontal, pinned) the screen – top / bottom cut; left / right open, except the strip's start and end
//   endless   (horizontal, endless) the screen – top / bottom cut; left / right always open (the band wraps)
// An open edge is a tool, never a warning (owner 2026-10-15: "having text boxes pass is good as a tool"). Paint order:
// pictures < all overlays of the group; between two slices the later slice's overlays win – a box reaching into a later
// slice lies UNDER that slice's boxes (a collision). Shared by the validator and the creator's strip editor.
import { anchoredRect } from './anchor.ts';
import { dialogFit, estimatedWidth } from './dialog-fit.ts';

/** how a slice's edge behaves: open (crossable), or the window that cuts there */
export type SliceEdge = 'open' | 'column' | 'screen' | 'strip-end' | 'group-end';
export type EdgeSide = 'top' | 'right' | 'bottom' | 'left';
export type SliceEdges = Record<EdgeSide, SliceEdge>;
/** the group's axis: vertical stacks the slices, a strip / endless band lays them side by side */
export type GroupAxis = 'vertical' | 'strip' | 'endless';

type Obj = Record<string, any>;
const isObj = (v: unknown): v is Obj => !!v && typeof v === 'object' && !Array.isArray(v);
const isNum = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);

/** a group's axis from its renderer (`horizontal` + `params.endless` = the band) */
export function groupAxis(group: unknown): GroupAxis {
	const r = isObj(group) && isObj(group.renderer) ? group.renderer : null;
	if (r?.type !== 'horizontal') return 'vertical';
	return isObj(r.params) && r.params.endless === true ? 'endless' : 'strip';
}

/** the edges of slice `index` of `group` (its slices in order) */
export function sliceEdges(group: unknown, index: number): SliceEdges {
	const count = isObj(group) && Array.isArray(group.slices) ? group.slices.length : 0;
	const first = index <= 0;
	const last = index >= count - 1;
	switch (groupAxis(group)) {
		case 'endless':
			return { top: 'screen', bottom: 'screen', left: 'open', right: 'open' };
		case 'strip':
			return { top: 'screen', bottom: 'screen', left: first ? 'strip-end' : 'open', right: last ? 'strip-end' : 'open' };
		default:
			return { top: first ? 'group-end' : 'open', bottom: last ? 'group-end' : 'open', left: 'column', right: 'column' };
	}
}

/** an overlay's box in % of its slice (left / top / width; height null = unknown – an auto-height text box) */
export interface EdgeBox {
	left: number;
	top: number;
	width: number;
	height: number | null;
}

/** a text box's drawn box (its anchor; auto width: the estimated text width), null when its rect is incomplete */
export function dialogBox(d: unknown): EdgeBox | null {
	if (!isObj(d) || !isObj(d.rect) || !isNum(d.rect.x) || !isNum(d.rect.y)) return null;
	const fit = dialogFit(d as any);
	const width = fit === 'width' ? estimatedWidth(d as any) : isNum(d.rect.width) ? d.rect.width : null;
	if (width === null) return null;
	const height = fit === 'height' || !isNum(d.rect.height) ? null : d.rect.height;
	const box = anchoredRect({ x: d.rect.x, y: d.rect.y, width, ...(height !== null ? { height } : {}) } as any, d.anchor);
	return { left: box.left, top: box.top, width, height };
}

/** a placed item's / an area's box (`rect` with width, height optional), null when incomplete */
export function rectBox(rect: unknown, anchor?: unknown): EdgeBox | null {
	if (!isObj(rect) || !isNum(rect.x) || !isNum(rect.y) || !isNum(rect.width)) return null;
	const height = isNum(rect.height) ? rect.height : null;
	const box = anchoredRect({ x: rect.x, y: rect.y, width: rect.width, ...(height !== null ? { height } : {}) } as any, anchor);
	return { left: box.left, top: box.top, width: rect.width, height };
}

/** the sides a box reaches past (0.5 % of slack), each with its edge */
export function crossedEdges(box: EdgeBox, edges: SliceEdges): { side: EdgeSide; edge: SliceEdge }[] {
	const out: { side: EdgeSide; edge: SliceEdge }[] = [];
	if (box.left < -0.5) out.push({ side: 'left', edge: edges.left });
	if (box.left + box.width > 100.5) out.push({ side: 'right', edge: edges.right });
	if (box.top < -0.5) out.push({ side: 'top', edge: edges.top });
	if (box.height !== null && box.top + box.height > 100.5) out.push({ side: 'bottom', edge: edges.bottom });
	return out;
}

/** what a cut edge is, for a message: "… past the slice's left side – <why>" */
export function edgeReason(edge: SliceEdge, side: EdgeSide): string {
	switch (edge) {
		case 'column':
			return `past the slice's ${side} side – the reader's column cuts it there`;
		case 'screen':
			return `past the slice's ${side} – a horizontal group is one screen high, the screen cuts it`;
		case 'strip-end':
			return `past the strip's ${side === 'left' ? 'start' : 'end'} – nothing is there, the reader cuts it`;
		case 'group-end':
			return `${side === 'top' ? "above the group's first slice" : "below the group's last slice"} – outside the group, the reader cuts it`;
		default:
			return '';
	}
}

/** a box in the group's coordinates (vertical: % of the width down the group; a strip / band: % of the height along it) */
interface GroupBox {
	slice: number;
	label: string;
	a0: number;
	a1: number;
	b0: number;
	b1: number;
}

/** the slices' extents along the axis: start + length per slice, in % of the cross size */
function extents(slices: Obj[], axis: GroupAxis): { start: number; length: number }[] {
	let at = 0;
	return slices.map((s) => {
		const w = isNum(s?.width) && s.width > 0 ? s.width : 390;
		const h = isNum(s?.height) && s.height > 0 ? s.height : 625;
		const length = axis === 'vertical' ? (h / w) * 100 : (w / h) * 100;
		const out = { start: at, length };
		at += length;
		return out;
	});
}

/** a text as one line: a string, a string[] (paragraphs) or a locale map's first text */
function lineOf(v: unknown): string {
	if (typeof v === 'string') return v;
	if (Array.isArray(v)) return v.filter((x) => typeof x === 'string').join(' ');
	if (isObj(v)) for (const x of Object.values(v)) if (typeof x === 'string' && x.trim()) return x;
	return '';
}

/** the first words of a text box (for a message) */
function firstWords(d: Obj): string {
	const t = lineOf(d.text).replace(/\s+/g, ' ').trim();
	return t.length > 24 ? `${t.slice(0, 22)}…` : t || (typeof d.id === 'string' ? d.id : 'a text box');
}

export interface Collision {
	/** the covered box: slice index + dialog index */
	slice: number;
	dialog: number;
	/** the covering box (a later slice's) */
	bySlice: number;
	byLabel: string;
}

/**
 * Text boxes of the group that lie under a later slice's text box (both with a known height): the later slice's plane
 * paints on top, so the earlier box is covered where they overlap.
 */
export function dialogCollisions(group: unknown): Collision[] {
	if (!isObj(group) || !Array.isArray(group.slices)) return [];
	const slices = group.slices as Obj[];
	const axis = groupAxis(group);
	const ext = extents(slices, axis);
	const boxes: (GroupBox & { dialog: number })[] = [];
	slices.forEach((s, si) => {
		(Array.isArray(s?.dialog) ? s.dialog : []).forEach((d: Obj, di: number) => {
			const b = dialogBox(d);
			if (!b || b.height === null) return;
			const len = ext[si].length;
			// along the axis: vertical = y (in % of the width: × length / 100), strip = x (in % of the height)
			const along0 = axis === 'vertical' ? ext[si].start + (b.top / 100) * len : ext[si].start + (b.left / 100) * len;
			const along1 = axis === 'vertical' ? along0 + (b.height / 100) * len : along0 + (b.width / 100) * len;
			const across0 = axis === 'vertical' ? b.left : b.top;
			const across1 = axis === 'vertical' ? b.left + b.width : b.top + b.height;
			boxes.push({ slice: si, dialog: di, label: firstWords(d), a0: along0, a1: along1, b0: across0, b1: across1 });
		});
	});
	const out: Collision[] = [];
	for (const x of boxes) {
		for (const y of boxes) {
			if (y.slice <= x.slice) continue;
			const overlap = Math.min(x.a1, y.a1) - Math.max(x.a0, y.a0) > 0.5 && Math.min(x.b1, y.b1) - Math.max(x.b0, y.b0) > 0.5;
			if (overlap) out.push({ slice: x.slice, dialog: x.dialog, bySlice: y.slice, byLabel: y.label });
		}
	}
	return out;
}
