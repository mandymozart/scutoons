/**
 * The full-width strip's scroll (strip.svelte): the page scrolls `range` px while the strip is pinned, the track moves
 * `travel` px sideways in that time. The range is fixed by CSS from the slices' ratios and the large viewport height
 * (`lvh`) – it never follows the mobile toolbar (`dvh`), so the page's height doesn't change when the toolbar slides in
 * or out (owner 2026-10-10: scrolling back up jumped – 4 strips in catpope ch2 grew / shrank ≈ 1600 px with the toolbar).
 * The track itself is drawn at the visible height (dvh, no black bar), so travel and range differ a little while the
 * toolbar shows; the scroll is mapped onto the travel.
 */

/** the sum of the frames' width / height ratios – the strip's width in screen heights */
export function ratioSum(frames: { width: number; height: number }[]): number {
	return frames.reduce((sum, f) => sum + (f.width > 0 && f.height > 0 ? f.width / f.height : 0), 0);
}

/** the track's offset for a page scroll of `scrolled` px into the strip */
export function stripOffset(scrolled: number, range: number, travel: number): number {
	if (range <= 0 || travel <= 0) return 0;
	return (Math.min(Math.max(scrolled, 0), range) / range) * travel;
}

/** the page scroll (px into the strip) that shows the track at `offset` */
export function stripScroll(offset: number, range: number, travel: number): number {
	if (range <= 0 || travel <= 0) return 0;
	return (Math.min(Math.max(offset, 0), travel) / travel) * range;
}
