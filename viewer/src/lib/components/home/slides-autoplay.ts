/**
 * The home slider's autoplay rules (slides-pager.ts; WCAG 2.2.2, owner 2026-10-17 – TEXT-ACROSS-SLICES.md §9), pure so
 * test/slides-autoplay.test.ts can hold them: on from the start unless reduced motion; the countdown runs only while
 * nothing holds it; any interaction stops it until the control starts it again.
 */

/** what holds a playing autoplay without stopping it */
export interface AutoplayHolds {
	/** a mouse is over the slides */
	hover: boolean;
	/** keyboard focus is in the slider (not on the pause / play control itself) */
	focus: boolean;
	dragging: boolean;
	/** the tab is hidden */
	hidden: boolean;
}

/** a page view starts playing: autoplay is set (ms > 0) and the reader doesn't prefer reduced motion */
export function autoplayStarts(ms: number, reduced: boolean): boolean {
	return ms > 0 && !reduced;
}

/** whether the countdown to the next slide runs now */
export function autoplayRuns(playing: boolean, movable: boolean, holds: AutoplayHolds): boolean {
	return playing && movable && !holds.hover && !holds.focus && !holds.dragging && !holds.hidden;
}

/**
 * what playing becomes: the control toggles it (only with more slides than fit); an interaction (a drag, a wheel, a
 * key, prev / next, a tap that stopped a slide) or reduced motion switched on stops it
 */
export function autoplayAfter(playing: boolean, event: 'toggle' | 'interact' | 'reduced', movable: boolean): boolean {
	return event === 'toggle' ? !playing && movable : false;
}
