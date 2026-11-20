// The core's units for ScrubNumber (decisions/deck.md [design]): the catalogue and the conversions are deck-core's
// `units`, reached through the wasm the app loads (@scutoons/live-deck `loadUnits`); this only lays them out as the
// field's options. A unit the core cannot offer with what is known (frames without a frame rate) is left out.
import type { UnitOption, Units } from './units.ts';

/** deck-core `units` as the wasm hands it (unit and dimension ids: `s`, `ms`, `px`, `percent`, `db` …; `time`, `length` …) */
export interface UnitConversions {
	offered(dimension: string, refs: Float64Array): string[];
	/** NaN when it cannot be */
	convert(v: number, from: string, to: string, refs: Float64Array): number;
	pick(chosen: string, fallback: string, refs: Float64Array): string;
	word(unit: string): string;
	decimals(unit: string): number;
	step(unit: string): number;
}

/** what a conversion is read against (none = not known) */
export interface UnitRefs {
	fps?: number;
	bpm?: number;
	perBar?: number;
	/** the reference a length is a part of, in pixels and in metres */
	pixels?: number;
	metres?: number;
}

/** the core's wire form of `refs` (deck-core `Refs::from_wire`) */
export const unitRefs = (r: UnitRefs = {}) => Float64Array.from([r.fps, r.bpm, r.perBar, r.pixels, r.metres].map((v) => v ?? NaN));

/** A value of `dimension` stored in the unit `stored`: the units the core offers for it, the choice remembered per `kind`
 *  (ScrubNumber), `fallback` (default: the stored unit) when nothing is chosen. */
export function coreUnits(core: UnitConversions, o: { kind: string; dimension: string; stored: string; fallback?: string; refs?: UnitRefs }): Units {
	const refs = unitRefs(o.refs);
	const options: UnitOption[] = core.offered(o.dimension, refs).map((id) => ({
		id,
		label: core.word(id),
		step: core.step(id),
		decimals: core.decimals(id),
		toShown: (v: number) => core.convert(v, o.stored, id, refs),
		toStored: (n: number) => core.convert(n, id, o.stored, refs)
	}));
	return { kind: o.kind, options, fallback: core.pick('', o.fallback ?? o.stored, refs) };
}
