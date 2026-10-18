/**
 * The score view's page layout (research/SCORES.md §4, agents/research/COLLAGE.md §4.3): the zoom steps and Verovio's
 * options for a page exactly the size of what the reader sees. Pure – moved out of score-view.svelte so the creator's
 * score stage lays out the same pages, and so it is unit-tested (test/score-anchors.test.ts).
 */
import type { ScoreLook } from './score-looks.ts';

/** Verovio scales in percent; the steps − / + walk through */
export const ZOOMS = [20, 25, 30, 35, 40, 45, 50, 60, 70, 85, 100];
export const DEFAULT_ZOOM = 40;
/** a phone (narrower than 40rem) starts smaller: a few bars per line instead of one */
export const PHONE_ZOOM = 30;

/** what collage pieces anchor to (score-anchors.ts): a bar's number and a staff's number as `data-n` in the SVG */
export const ANCHOR_ATTRIBUTES = ['measure@n', 'staff@n'];

/** a look's margin (rem) in Verovio's units at `scale` %; without one Verovio's 40 */
export function margin(rem: number | undefined, scale: number, remPx: number): number {
	if (rem === undefined) return 40;
	return Math.round((rem * remPx * 100) / scale);
}

/** the next zoom step from `zoom` (`by` −1 / +1), clamped; an unknown zoom steps from the default */
export function stepZoom(zoom: number, by: number): number {
	const i = ZOOMS.indexOf(zoom);
	return ZOOMS[Math.min(Math.max(0, (i < 0 ? ZOOMS.indexOf(DEFAULT_ZOOM) : i) + by), ZOOMS.length - 1)];
}

/**
 * Verovio's options for a `w` × `h` (CSS px) page at `zoom` %: its page units at that scale are the sheet's pixels (its
 * tutorial's way to fit a page to a box). `remPx` = the root font size (a look's margins are in rem). A look's own
 * `svgAdditionalAttribute` (note@pname …) is kept; the anchors' measure@n / staff@n are added.
 */
export function scoreOptions(w: number, h: number, zoom: number, look: ScoreLook | null | undefined, remPx: number): Record<string, unknown> {
	const own = look?.options?.svgAdditionalAttribute;
	const attributes = [...new Set([...(Array.isArray(own) ? (own as string[]) : []), ...ANCHOR_ATTRIBUTES])];
	return {
		...look?.options,
		pageWidth: Math.round((w * 100) / zoom),
		pageHeight: Math.round((h * 100) / zoom),
		scale: zoom,
		adjustPageHeight: false,
		breaks: 'auto',
		header: 'auto',
		footer: 'none',
		pageMarginTop: margin(look?.margin?.top, zoom, remPx),
		pageMarginBottom: margin(look?.margin?.bottom, zoom, remPx),
		pageMarginLeft: margin(look?.margin?.side, zoom, remPx),
		pageMarginRight: margin(look?.margin?.side, zoom, remPx),
		svgAdditionalAttribute: attributes
	};
}
