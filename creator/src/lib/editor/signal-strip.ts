// The deck half's signal strip (#282, owner draft 2026-11-02): one vertical meter per audio output, one column per exposed
// knob, a MIDI / DMX / OSC activity monitor, one stats line. Pure – what the strip shows is arranged here from what the
// engine host reports (the meters' levels, positions and zones and the device lines are the core's – #373); the DOM strip
// draws it today and the later canvas element (own issue: "super fast ui for audio analytic feedback") reads the same model.
// Tested in test/signal-strip.test.ts.
import type { MeterZone } from '@scutoons/engine-host';

/** an exposed knob as the rig plays it (live-deck `RigKnob`, structural so this stays free of the engine's packages) */
export interface KnobLike {
	path: string;
	label?: Record<string, string>;
	min: number;
	max: number;
	log?: boolean;
}

export interface KnobColumn {
	path: string;
	label: string;
	/** where the value sits in its range, 0..1 (log knobs on their log scale) */
	position: number;
}

const clamp01 = (x: number): number => (Number.isFinite(x) ? Math.min(1, Math.max(0, x)) : 0);

/** the value's place in [min, max]; a `log` knob (a frequency) on its log scale */
export function knobPosition(value: number, min: number, max: number, log = false): number {
	if (!(max > min)) return 0;
	if (log && min > 0 && value > 0) return clamp01(Math.log(value / min) / Math.log(max / min));
	return clamp01((value - min) / (max - min));
}

/** a knob's label in `locale`, else English, else any, else the path's last segment */
export function knobName(k: KnobLike, locale = 'en'): string {
	const raw = k.label?.[locale] ?? k.label?.en ?? Object.values(k.label ?? {})[0];
	return raw && raw !== '' ? raw : (k.path.split('.').pop() ?? k.path);
}

/** one column per knob, in the rig's order; `values` = the deck's live values by path (a missing one reads as the minimum) */
export function knobColumns(knobs: readonly KnobLike[], values: Readonly<Record<string, number>>, locale = 'en'): KnobColumn[] {
	return knobs.map((k) => ({ path: k.path, label: knobName(k, locale), position: knobPosition(values[k.path] ?? k.min, k.min, k.max, k.log) }));
}

/** the same columns again? (the strip only redraws when something moved a visible step) */
export function sameColumns(a: readonly KnobColumn[], b: readonly KnobColumn[], step = 0.01): boolean {
	return a.length === b.length && a.every((c, i) => c.path === b[i].path && c.label === b[i].label && Math.abs(c.position - b[i].position) < step);
}

/** one output's reading as the core's analysis frame carries it (engine-host `OutputReading`, structural): the held peak, its
 *  place on the meter's dB scale and its zone – deck-core's meters (#373); the strip only draws them */
export interface ReadingLike {
	/** linear 0..1 */
	level: number;
	/** on the dB scale, 0..1 from the bottom */
	position: number;
	zone: MeterZone;
}

export interface OutputMeter extends ReadingLike {
	/** 1-based, as a patch bay numbers outputs */
	channel: number;
	/** the engine renders this output (a slot beyond the count is only drawn dim) */
	active: boolean;
}

const DIM: ReadingLike = { level: 0, position: 0, zone: 'ok' };

/** the output meters: one per reported reading, padded with dim slots up to `slots` (a stereo pair at least) */
export function outputMeters(readings: readonly ReadingLike[], slots = 2): OutputMeter[] {
	const n = Math.max(readings.length, slots);
	return Array.from({ length: n }, (_, i) => ({ ...(readings[i] ?? DIM), channel: i + 1, active: i < readings.length }));
}

/** an audio input the engine listens to; `source` names where it comes from (the source types: not decided yet – the engine reports none today) */
export interface InputLike extends ReadingLike {
	id: string;
	source: string;
	label?: string;
}

export interface InputMeter extends OutputMeter {
	id: string;
	source: string;
	label: string;
}

/** the input meters – none when the engine reports no inputs (an empty group shows nothing) */
export function inputMeters(inputs: readonly InputLike[]): InputMeter[] {
	return inputs.map((x, i) => ({ ...x, channel: i + 1, active: true, label: x.label || x.id }));
}

/** the stats the engine host reports (EngineSnapshot, structural) */
export interface StatsLike {
	phase: string;
	error?: string | null;
	transport: string | null;
	backend: string;
	/** #364: deck-core's monitor report (engine-host snapshot `stats`), null = none yet */
	stats: { bar: { label: string; value: string }[] } | null;
}

/** the state words and the performance, once each: `running · shared · webgl2` / the monitor's bar items as deck-core
 *  words them (#364: `FPS 60 · CPU 4.2/6.0MS · AUDIO 0/10M 2/5/9% +43MS · LIM 0.0DB`) */
export function stripStatus(s: StatsLike): { state: string; perf: string } {
	const state = [s.phase === 'error' && s.error ? `error: ${s.error}` : s.phase, ...(s.transport ? [s.transport] : []), ...(s.backend ? [s.backend] : [])].join(' · ');
	const perf = (s.stats?.bar ?? []).map((i) => `${i.label} ${i.value}`).join(' · ');
	return { state, perf };
}
