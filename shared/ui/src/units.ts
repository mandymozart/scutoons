// Units of a number field (owner 2026-10-18: "switch between %, px, pt and rem with a dropdown on the unit … for ALL
// fields"): the stored value never changes – a unit only converts it for display and input. The app builds the options
// (it knows what the stored number is relative to: % of the slice, em of the font size …); ScrubNumber shows them as a
// dropdown in the field and remembers the choice per kind of field (unit-prefs.svelte.ts).

export interface UnitOption {
	/** the id that is remembered (`px`, `rem` …) */
	id: string;
	/** shown in the dropdown and after the number */
	label: string;
	/** one arrow / scrub step in this unit (1 px, 0.5 pt, 0.05 rem, 0.1 %) */
	step: number;
	/** the shown value is rounded to these decimals */
	decimals: number;
	/** stored → shown, and back (both increase together) */
	toShown: (stored: number) => number;
	toStored: (shown: number) => number;
}

export interface Units {
	/** the kind of field the choice is remembered for (positions, sizes, strokes, text size …) */
	kind: string;
	options: UnitOption[];
	/** the unit without a remembered choice */
	fallback: string;
}

export const roundTo = (v: number, decimals: number) => Math.round(v * 10 ** decimals) / 10 ** decimals;
