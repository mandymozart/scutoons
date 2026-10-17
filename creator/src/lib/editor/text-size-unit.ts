// The text box's font size in the creator's units (owner 2026-10-17: "font size in percent makes no sense … rem as
// default, but one can switch to pt or px"). Stored unchanged as % of the slice width (`Dialog.size`, rendered as cqw
// – the reader scales the slice); the inspector shows it in px / rem / pt of the slice at its 1x size (390 px wide by
// default), 1rem = 16px, 1pt = 4/3 px. The chosen unit is a per-browser preference (localStorage).

export const TEXT_SIZE_UNITS = ['rem', 'pt', 'px'] as const;
export type TextSizeUnit = (typeof TEXT_SIZE_UNITS)[number];

const PX_PER: Record<TextSizeUnit, number> = { px: 1, rem: 16, pt: 4 / 3 };
/** a sensible scrub / arrow step per unit */
export const TEXT_SIZE_STEP: Record<TextSizeUnit, number> = { rem: 0.05, pt: 0.5, px: 0.5 };

const round = (v: number, decimals: number) => Math.round(v * 10 ** decimals) / 10 ** decimals;

/** % of the slice width → the unit (rounded for display: rem 3 decimals, pt / px 2) */
export function sizeToUnit(percent: number, unit: TextSizeUnit, sliceWidth: number): number {
	return round(((percent / 100) * sliceWidth) / PX_PER[unit], unit === 'rem' ? 3 : 2);
}

/** the unit → % of the slice width (4 decimals: a px value survives the round trip) */
export function unitToSize(value: number, unit: TextSizeUnit, sliceWidth: number): number {
	return round(((value * PX_PER[unit]) / sliceWidth) * 100, 4);
}
