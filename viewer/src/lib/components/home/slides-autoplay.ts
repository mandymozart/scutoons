/**
 * The home slider's autoplay rules (slides-pager.ts; owner 2026-10-17 – TEXT-ACROSS-SLICES.md §9), pure so
 * test/slides-autoplay.test.ts can hold them: on unless reduced motion; the countdown runs only while nothing holds it
 * (a mouse over the slides, keyboard focus in the slider, a finger on it, a drag, a hidden tab) and starts anew once the
 * hold ends – there is no pause control (owner 2026-10-17: no buttons under the picture).
 */

/** what holds the autoplay for a while */
export interface AutoplayHolds {
	/** a mouse is over the slides */
	hover: boolean;
	/** keyboard focus is in the slider */
	focus: boolean;
	/** a finger (or pen) is on the slides */
	touch: boolean;
	dragging: boolean;
	/** the tab is hidden */
	hidden: boolean;
}

/** autoplay is set (ms > 0) and the reader doesn't prefer reduced motion */
export function autoplayOn(ms: number, reduced: boolean): boolean {
	return ms > 0 && !reduced;
}

/** whether the countdown to the next slide runs now */
export function autoplayRuns(on: boolean, movable: boolean, holds: AutoplayHolds): boolean {
	return on && movable && !holds.hover && !holds.focus && !holds.touch && !holds.dragging && !holds.hidden;
}
