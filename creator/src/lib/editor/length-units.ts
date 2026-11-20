// The units the creator's length fields offer (owner 2026-10-18: "switch between %, px, pt and rem with a dropdown on
// the unit … for ALL fields"): the catalogue and every conversion are deck-core's `units` (through the wasm); this only
// says what a stored number is a part of. The stored numbers never change – a unit only converts them for display and
// input. `core` is null until the wasm is there: the field shows its stored unit meanwhile.
import { coreUnits, type UnitConversions } from '@scutoons/ui/core-units';
import type { Units } from '@scutoons/ui';

/** a length stored in % of a slice axis that is `axisPx` px at 1x */
export function sliceLength(core: UnitConversions | null, kind: string, axisPx: number, fallback = 'percent'): Units | undefined {
	return core ? coreUnits(core, { kind, dimension: 'length', stored: 'percent', fallback, refs: { pixels: axisPx, web: true } }) : undefined;
}

/** a length stored in em of a font size that is `fontPx` px at 1x; `label` for the em (line height: ×) */
export function fontLength(core: UnitConversions | null, kind: string, fontPx: number, fallback = 'em', label?: string): Units | undefined {
	return core ? coreUnits(core, { kind, dimension: 'length', stored: 'em', fallback, refs: { pixels: fontPx, web: true, em: true }, labels: label ? { em: label } : undefined }) : undefined;
}

/** a text box's font size in px at 1x (`Dialog.size` is % of the slice width, default 4) */
export const fontPx = (size: number | undefined, sliceWidth: number) => ((size ?? 4) / 100) * sliceWidth;
