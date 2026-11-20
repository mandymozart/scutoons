// units.ts: deck-core's units – the catalogue and its conversions – for the web kit's unit field (@scutoons/ui
// `coreUnits` → ScrubNumber), space-web's free functions as they are (as wasm, no GPU). No conversion here.
import { spaceModule } from './space.ts';

/** structurally @scutoons/ui's `UnitConversions` */
export interface UnitsView {
	offered(dimension: string, refs: Float64Array): string[];
	convert(v: number, from: string, to: string, refs: Float64Array): number;
	pick(chosen: string, fallback: string, refs: Float64Array): string;
	word(unit: string): string;
	decimals(unit: string): number;
	step(unit: string): number;
}

/** space-web's `units_*` exports */
interface UnitsExports {
	units_offered: UnitsView['offered'];
	units_convert: UnitsView['convert'];
	units_pick: UnitsView['pick'];
	units_word: UnitsView['word'];
	units_decimals: UnitsView['decimals'];
	units_step: UnitsView['step'];
}

/** the core's units from the wasm at `base` (the same module the deck loads: loaded once per page) */
export async function loadUnits(base: string): Promise<UnitsView> {
	const mod = (await spaceModule(base)) as unknown as UnitsExports;
	return { offered: mod.units_offered, convert: mod.units_convert, pick: mod.units_pick, word: mod.units_word, decimals: mod.units_decimals, step: mod.units_step };
}
