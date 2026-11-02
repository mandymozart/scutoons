// The deck half's signal strip (#282, owner draft 2026-11-02): one vertical meter per audio output, one column per exposed
// knob, a MIDI / DMX / OSC activity monitor, one stats line. Pure – what the strip shows is worked out here from what the
// engine host reports; the DOM strip draws it today and the later canvas element (own issue: "super fast ui for audio
// analytic feedback") reads the same model. Tested in test/signal-strip.test.ts.

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

/** a bus meter's scale: dBFS from `floor` to `top` (a little headroom above the 0.0 dB mark, so a clip shows) */
export const METER_RANGE = { floor: -48, top: 3 } as const;
/** the zones' edges in dB: below `ok` green, up to `hot` amber, above red (digital full scale is 0.0) */
export const METER_ZONES = { ok: -12, hot: -3 } as const;

/** a linear level (0..1, 1 = full scale) in dB, floored at the scale's bottom (silence = the floor) */
export function levelDb(linear: number, range = METER_RANGE): number {
	if (!(linear > 0) || !Number.isFinite(linear)) return range.floor;
	return Math.max(range.floor, 20 * Math.log10(linear));
}

/** where a dB value sits on the meter, 0..1 from the bottom */
export function dbPosition(db: number, range = METER_RANGE): number {
	return clamp01((db - range.floor) / (range.top - range.floor));
}

export type MeterZone = 'ok' | 'hot' | 'clip';
export const meterZone = (db: number): MeterZone => (db >= METER_ZONES.hot ? 'clip' : db >= METER_ZONES.ok ? 'hot' : 'ok');

/** the scale's fixed marks as positions: the 0.0 dB line and the zone edges (the strip draws them on every bus meter) */
export const METER_MARKS = {
	zero: dbPosition(0),
	ok: dbPosition(METER_ZONES.ok),
	hot: dbPosition(METER_ZONES.hot)
} as const;

export interface OutputMeter {
	/** 1-based, as a patch bay numbers outputs */
	channel: number;
	/** linear 0..1 as the engine reports it */
	level: number;
	/** the same on the meter's dB scale, 0..1 from the bottom */
	position: number;
	zone: MeterZone;
	/** the engine renders this output (a slot beyond the count is only drawn dim) */
	active: boolean;
}

function meterOf(level: number, channel: number, active: boolean): OutputMeter {
	const db = levelDb(level);
	return { channel, level: clamp01(level), position: dbPosition(db), zone: meterZone(db), active };
}

/** the output meters: one per reported peak, padded with dim slots up to `slots` (a stereo pair at least) */
export function outputMeters(peaks: readonly number[], slots = 2): OutputMeter[] {
	const n = Math.max(peaks.length, slots);
	return Array.from({ length: n }, (_, i) => meterOf(i < peaks.length ? peaks[i] : 0, i + 1, i < peaks.length));
}

/** an audio input the engine listens to; `source` names where it comes from (the source types: not decided yet – the engine reports none today) */
export interface InputLike {
	id: string;
	source: string;
	label?: string;
	/** linear 0..1 */
	level: number;
}

export interface InputMeter extends OutputMeter {
	id: string;
	source: string;
	label: string;
}

/** the input meters – none when the engine reports no inputs (an empty group shows nothing) */
export function inputMeters(inputs: readonly InputLike[]): InputMeter[] {
	return inputs.map((x, i) => ({ ...meterOf(x.level, i + 1, true), id: x.id, source: x.source, label: x.label || x.id }));
}

/** the limiter's gain reduction (dB, 0 = untouched) as a 0..1 bar: full at `fullAt` dB */
export const reductionBar = (db: number, fullAt = 12): number => clamp01(Math.abs(db) / fullAt);

/** the stats the engine host reports (EngineSnapshot, structural) */
export interface StatsLike {
	phase: string;
	error?: string | null;
	transport: string | null;
	backend: string;
	fps: number;
	cpu: number;
	dropouts: number | null;
}

/** the state words and the performance, once each (the frame rate is listed ONCE): `running · shared · webgl2` / `60 fps · 4.2 ms · 0 drops` */
export function stripStatus(s: StatsLike): { state: string; perf: string } {
	const state = [s.phase === 'error' && s.error ? `error: ${s.error}` : s.phase, ...(s.transport ? [s.transport] : []), ...(s.backend ? [s.backend] : [])].join(' · ');
	const perf = [`${Math.round(s.fps)} fps`, `${s.cpu.toFixed(1)} ms`, ...(s.dropouts === null ? [] : [`${s.dropouts} drops`])].join(' · ');
	return { state, perf };
}

export type SignalKind = 'midi' | 'osc' | 'dmx';
export const SIGNAL_KINDS: readonly SignalKind[] = ['midi', 'osc', 'dmx'];

/** what the engine host's monitor keeps per kind (engine-host `SignalState`, structural) */
export interface SignalView {
	count: number;
	/** 1 on a message, falling to 0 */
	activity: number;
	/** the last message in a short line, '' = none */
	text: string;
}

export interface SignalRow {
	kind: SignalKind;
	/** the last message, '' = none yet */
	text: string;
	/** traffic just now – the lamp is lit */
	active: boolean;
}

/** a row per protocol that has carried a message (a protocol nothing uses shows nothing); lit while its activity is up */
export function signalRows(view: Readonly<Record<SignalKind, SignalView>> | null, lit = 0.25): SignalRow[] {
	if (!view) return [];
	return SIGNAL_KINDS.filter((kind) => view[kind].count > 0).map((kind) => ({ kind, text: view[kind].text, active: view[kind].activity >= lit }));
}
