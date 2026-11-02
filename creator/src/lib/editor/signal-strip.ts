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

export interface OutputMeter {
	/** 1-based, as a patch bay numbers outputs */
	channel: number;
	/** 0..1 */
	level: number;
	/** the engine renders this output (a slot beyond the count is only drawn dim) */
	active: boolean;
}

/** the output meters: one per reported peak, padded with dim slots up to `slots` (a stereo pair at least) */
export function outputMeters(peaks: readonly number[], slots = 2): OutputMeter[] {
	const n = Math.max(peaks.length, slots);
	return Array.from({ length: n }, (_, i) => ({ channel: i + 1, level: i < peaks.length ? clamp01(peaks[i]) : 0, active: i < peaks.length }));
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

export type SignalKind = 'midi' | 'dmx' | 'osc';
export const SIGNAL_KINDS: readonly SignalKind[] = ['midi', 'dmx', 'osc'];

/** the last message of a kind, already as short text (`CC74 · 90`, `U1 · 12 ch`), and when it was seen (ms) */
export interface SignalSeen {
	text: string;
	at: number;
}

export interface SignalRow {
	kind: SignalKind;
	/** the last message, '' = none yet */
	text: string;
	/** traffic within the last `hold` ms – the dot is lit */
	active: boolean;
}

/** a row per protocol: its last message and whether it is lit now (`hold` = how long a dot stays on after traffic) */
export function signalRows(last: Partial<Record<SignalKind, SignalSeen>>, now: number, hold = 250): SignalRow[] {
	return SIGNAL_KINDS.map((kind) => {
		const seen = last[kind];
		return { kind, text: seen?.text ?? '', active: !!seen && now - seen.at >= 0 && now - seen.at < hold };
	});
}

/** MIDI bytes (status first) as short text: `CC74 · 90`, `note on 60 · 100`, `bend 8192` – anything else as hex */
export function midiText(bytes: readonly number[]): string {
	const [status = 0, a = 0, b = 0] = bytes;
	const channel = (status & 0x0f) + 1;
	switch (status & 0xf0) {
		case 0xb0:
			return `CC${a} · ${b}${channel > 1 ? ` · ch${channel}` : ''}`;
		case 0x90:
			return `${b === 0 ? 'note off' : 'note on'} ${a} · ${b}`;
		case 0x80:
			return `note off ${a}`;
		case 0xe0:
			return `bend ${a | (b << 7)}`;
		default:
			return bytes.map((x) => x.toString(16).padStart(2, '0')).join(' ');
	}
}
