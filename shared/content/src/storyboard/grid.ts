// The storyboard's canvas and the slice grid (from tools/storyboard-render/spec.js + lib.mjs shotHeight, #111 / #119):
// the phone canvas W × S, one slice UNIT, the slice aspects, whole-slice checks. agents/research/SLICE-GRID.md §1.
// content.ts DEFAULT_SLICE is the platform's own slice size; this is the storyboard's planning grid.

/** the phone canvas: width W, one screen S */
export const W = 390;
export const S = 844;

/**
 * One slice, 390 × 624 = 1:1.6 (#78: its 3x = WEBTOON's 800 × 1280). A viewport / vertical shot is k whole slices,
 * one slice each when cut (SLICE-GRID.md §1); chapters 1–5 were cut at 625.
 */
export const UNIT = 624;

/** the aspects of 1 … 5 slices (SLICE-GRID.md §1: generated up to 2, the owner's art up to 5) */
export const SLICE_ASPECTS = ['1:1.6', '1:3.2', '1:4.8', '1:6.4', '1:8'] as const;

export interface Aspect {
	w: number;
	h: number;
}

/** a height in px → k when it is k whole slices (within 0.5 %, SLICE-GRID.md §1 rule 3), else null: 1248 → 2, 1560 → null */
export function slicesOf(h: number): number | null {
	const k = Math.round(h / UNIT);
	return k >= 1 && Math.abs(h - k * UNIT) <= 0.005 * k * UNIT ? k : null;
}

/** "1:4.8" → 3; "9:16", "1:4" → null (not whole slices) */
export function aspectSlices(aspect: unknown): number | null {
	const a = parseAspect(aspect);
	return a ? slicesOf((W * a.h) / a.w) : null;
}

/** "1:4" → { w: 1, h: 4 }; "9:16" → { w: 9, h: 16 }; "6:1" → { w: 6, h: 1 }; anything else → null */
export function parseAspect(a: unknown): Aspect | null {
	const m = /^(\d+(?:\.\d+)?):(\d+(?:\.\d+)?)$/.exec(String(a ?? ''));
	return m ? { w: Number(m[1]), h: Number(m[2]) } : null;
}

/** height in px of a shot of this aspect on the 390-wide canvas (loop360: the height of one slide row; no aspect: S) */
export function shotHeight(aspect: unknown, format?: string): number {
	const a = parseAspect(aspect);
	if (!a) return S;
	if (format === 'loop360') return S; // a panorama is read as one screen-high stop, dragged sideways
	return Math.round((W * a.h) / a.w);
}
