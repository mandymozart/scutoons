// The units the creator's length fields offer (owner 2026-10-18: "switch between %, px, pt and rem with a dropdown on
// the unit … for ALL fields"; the text size came first, 2026-10-17). The stored numbers never change – a unit only
// converts them for display and input (@scutoons/ui ScrubNumber `units`, the choice remembered per kind in the browser):
//   % of a slice axis  (x, width, text size: of the width; y, height: of the height) → %, px, pt, rem of the 1x slice
//   em of the font size (text outline, letter spacing; line height as ×)               → em (×), px, pt, rem
// 1rem = 16px, 1pt = 4/3px, all at the slice's 1x size (390 px wide by default). Steps: 0.1 %, 1 px, 0.5 pt, 0.05 rem.
import type { UnitOption, Units } from '@scutoons/ui';

/** px in one of each absolute unit */
const PX = { px: 1, pt: 4 / 3, rem: 16 } as const;
const ABSOLUTE = [
	{ id: 'px', step: 1, decimals: 1 },
	{ id: 'pt', step: 0.5, decimals: 2 },
	{ id: 'rem', step: 0.05, decimals: 3 }
] as const;

/** px / pt / rem for a stored unit that is `pxPerStored` px */
function absolute(pxPerStored: number): UnitOption[] {
	return ABSOLUTE.map((a) => ({
		id: a.id,
		label: a.id,
		step: a.step,
		decimals: a.decimals,
		toShown: (v: number) => (v * pxPerStored) / PX[a.id],
		toStored: (n: number) => (n * PX[a.id]) / pxPerStored
	}));
}

const same = (id: string, label: string, step: number, decimals: number): UnitOption => ({ id, label, step, decimals, toShown: (v) => v, toStored: (n) => n });

/** a length stored in % of a slice axis that is `axisPx` px at 1x */
export function sliceLength(kind: string, axisPx: number, fallback = '%'): Units {
	return { kind, fallback, options: [same('%', '%', 0.1, 2), ...absolute(axisPx / 100)] };
}

/** a length stored in em of a font size that is `fontPx` px at 1x; `label` / `step` for the stored unit (line height: ×) */
export function fontLength(kind: string, fontPx: number, fallback = 'em', label = 'em', step = 0.01): Units {
	return { kind, fallback, options: [same('em', label, step, 3), ...absolute(fontPx)] };
}

/** a text box's font size in px at 1x (`Dialog.size` is % of the slice width, default 4) */
export const fontPx = (size: number | undefined, sliceWidth: number) => ((size ?? 4) / 100) * sliceWidth;
