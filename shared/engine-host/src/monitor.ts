// monitor.ts (#282): the scene's device messages (MIDI / OSC / DMX – what its `[outputs]` resolve to, live-core's signals)
// as a monitor reads them: the last message per kind, how many came, and an activity that jumps to 1 on a message and
// decays. Pure – no DOM, no wasm; the caller brings the clock (ms).
import type { DeviceSignal } from '@scutoons/live-core';

export type SignalKind = DeviceSignal['kind'];
export const SIGNAL_KINDS: readonly SignalKind[] = ['midi', 'osc', 'dmx'];

/** an activity lamp loses 63 % per this many seconds after a message */
export const ACTIVITY_SECONDS = 0.4;

/** one kind's state */
export interface SignalState {
	/** messages since the monitor started (or was reset) */
	count: number;
	/** 1 on a message, falling to 0 (`ACTIVITY_SECONDS`) */
	activity: number;
	/** the last message, null = none yet */
	last: DeviceSignal | null;
	/** the last message in a short line (`describeSignal`), '' = none */
	text: string;
	/** the clock (ms) it arrived at, null = none */
	at: number | null;
}

export type SignalsView = Record<SignalKind, SignalState>;

const hex = (b: readonly number[]): string => b.map((x) => x.toString(16).padStart(2, '0')).join(' ');

/** One message in a short line: `CC 74 = 127 · ch 1`, `note on 60 · vel 127 · ch 1`, `DMX 20–21 = 255 255`, `/fx/level 0.5` */
export function describeSignal(s: DeviceSignal): string {
	if (s.kind === 'osc') return [s.address, ...s.args.map((a) => (a.kind === 'string' ? JSON.stringify(a.p) : String(a.p)))].join(' ');
	if (s.kind === 'dmx') {
		const to = s.ch + s.values.length - 1;
		return `DMX ${s.values.length > 1 ? `${s.ch}–${to}` : s.ch} = ${s.values.join(' ')}${s.flash ? ' · flash' : ''}`;
	}
	const [status = 0, a = 0, b = 0] = s.bytes;
	const ch = (status & 0x0f) + 1;
	const to = s.port === 'conductor' ? ' → conductor' : '';
	switch (status & 0xf0) {
		case 0x80:
			return `note off ${a} · ch ${ch}${to}`;
		case 0x90:
			return b === 0 ? `note off ${a} · ch ${ch}${to}` : `note on ${a} · vel ${b} · ch ${ch}${to}`;
		case 0xb0:
			return `CC ${a} = ${b} · ch ${ch}${to}`;
		case 0xc0:
			return `program ${a} · ch ${ch}${to}`;
		case 0xe0:
			return `bend ${(b << 7) | a} · ch ${ch}${to}`;
		default:
			return `MIDI ${hex(s.bytes)}${to}`;
	}
}

const empty = (): SignalState => ({ count: 0, activity: 0, last: null, text: '', at: null });

/** what `push` keeps per kind – the message and a count; the text is made when someone asks (`view`) */
interface Kept {
	count: number;
	last: DeviceSignal | null;
	at: number | null;
	text: string;
	textOf: DeviceSignal | null;
}
const kept = (): Kept => ({ count: 0, last: null, at: null, text: '', textOf: null });

export class SignalMonitor {
	private states: Record<SignalKind, Kept> = { midi: kept(), osc: kept(), dmx: kept() };

	/** A message that arrived at `now` (ms). Allocation-free: the message is kept, its wording made only by `view` / `text`. */
	push(s: DeviceSignal, now: number): void {
		const st = this.states[s.kind];
		st.count++;
		st.last = s;
		st.at = now;
	}

	/** messages of `kind` so far (no allocation) */
	count(kind: SignalKind): number {
		return this.states[kind].count;
	}

	/** the last message of `kind`, null = none (no allocation) */
	last(kind: SignalKind): DeviceSignal | null {
		return this.states[kind].last;
	}

	/** the last message of `kind` in a line, '' = none – made once per message */
	text(kind: SignalKind): string {
		const st = this.states[kind];
		if (st.textOf !== st.last) {
			st.text = st.last ? describeSignal(st.last) : '';
			st.textOf = st.last;
		}
		return st.text;
	}

	/** The activity of `kind` at `now` (ms): 1 at a message, then falling; 0 before the first. */
	activity(kind: SignalKind, now: number): number {
		const at = this.states[kind].at;
		return at === null ? 0 : Math.exp(-Math.max(0, now - at) / 1000 / ACTIVITY_SECONDS);
	}

	/** every kind's state as of `now` (ms) – a copy */
	view(now: number): SignalsView {
		const out = {} as SignalsView;
		for (const k of SIGNAL_KINDS) out[k] = { count: this.states[k].count, last: this.states[k].last, at: this.states[k].at, text: this.text(k), activity: this.activity(k, now) };
		return out;
	}

	reset(): void {
		this.states = { midi: kept(), osc: kept(), dmx: kept() };
	}
}

export const emptySignals = (): SignalsView => ({ midi: empty(), osc: empty(), dmx: empty() });
