// GENERATED from deck/live-core/src/wire.rs by `npm run generate` (cargo run -p live-core --features codegen --bin codecs-ts) – do not edit.
// The wire format across the page / worker ↔ wasm boundary (#184, #372): binary records, no JSON. The layout tables are
// the Rust side's (SEND_FIELDS, Mode::ALL, BANDS and the frame's slots); deck/live-core/fixtures/wire.json holds records
// with their bytes and both sides' tests encode and decode every one of them.
//
// A record, little-endian, 4-byte aligned (the values can be read as a Float32Array in place): the header's fields at
// their offsets (op @ 0, mode @ 1, n @ 2, node @ 4, port @ 6, amount @ 8), then n f32 values (#189's float vectors); v[0] → port, v[1] → port + 1 …
// Node and port are indices from the load table (names are resolved once, at load – JSON stays at the edge).
// The analysis frame going out is a block of f32: level, bass, mid, high, then SPECTRUM bins, then the limiter's gain reduction (dB,
// the deepest since the last frame – #188), then (#282) the number of device channels and MAX_OUTPUTS peaks (0..1, each
// channel's largest sample, held and falling – deck-core's meters, #373), their places on the meter's dB scale (0..1 from
// the bottom), their zones, the limiter's bar (0..1), then (#320) the number of values and the values: every load-table
// row's inputs and outputs of the heard block, from the row's `at`. Device messages come in their own records
// (signals.ts).

export const HEADER = 12;
export const OP_SEND = 1;
/** deck-core's analysis::SPECTRUM – bin k = k · rate / 1024 Hz */
export const SPECTRUM = 512;
/** the frame's slot of the first spectrum bin (the bands come before it) */
export const SPECTRUM_AT = 4;
/** the frame's slot of the limiter's gain reduction (dB) */
export const REDUCTION = 516;
/** … of the number of device channels the engine renders (#282) */
export const OUTPUT_COUNT = 517;
/** … of the first channel's held peak */
export const PEAKS = 518;
/** the most channels a frame carries peaks for (deck-engine's device channel limit) */
export const MAX_OUTPUTS = 32;
/** … of the first channel's place on the meter's dB scale (#373) */
export const POSITIONS = 550;
/** … of the first channel's zone (an index into METER_ZONES) */
export const ZONES = 582;
/** … of the limiter's gain reduction as a 0..1 bar */
export const LIMITER_BAR = 614;
/** … of the number of values that follow */
export const VALUE_COUNT = 615;
/** … of the first value */
export const VALUES_AT = 616;
/** the frame's fixed part; the values follow it */
export const FRAME = 616;
/** the most values a frame carries (a row past them reads `at` = -1 in the table) */
export const MAX_FRAME_VALUES = 4096;
/** the most values one record carries (u16 n) */
export const MAX_VALUES = 65535;

/** a meter's zone, by the code the frame carries (deck-core meters::Zone) */
export type MeterZone = 'ok' | 'hot' | 'clip';
export const METER_ZONES: readonly MeterZone[] = ['ok', 'hot', 'clip'];
/** the meter scale's fixed marks as places 0..1 from the bottom: the 0.0 dB line and the zone edges (deck-core meters: -48 … +3 dB) */
export const METER_MARKS = { zero: 0.9411765, ok: 0.7058824, hot: 0.88235295 } as const;
/** the zone edges in dB, for the scale's labels */
export const METER_EDGES_DB = { ok: -12, hot: -3 } as const;

export type Mode = 'force' | 'blend' | 'release' | 'base';
export const MODES: readonly Mode[] = ['force', 'blend', 'release', 'base'];

export interface SendRecord {
	mode: Mode;
	node: number;
	port: number;
	/** blend: 0–1 of the sent value; force = 1 and release = 0 whatever is written here */
	amount: number;
	v: ArrayLike<number>;
}

export const recordSize = (n: number): number => HEADER + 4 * n;

const LITTLE = new Uint8Array(new Uint16Array([1]).buffer)[0] === 1;
const F = new Float32Array(1);
const FB = new Uint8Array(F.buffer);

/** an f32 as 4 little-endian bytes, without a DataView (no allocation on the hot path) */
function putF32(b: Uint8Array, at: number, x: number): void {
	F[0] = x;
	if (LITTLE) {
		b[at] = FB[0];
		b[at + 1] = FB[1];
		b[at + 2] = FB[2];
		b[at + 3] = FB[3];
	} else {
		b[at] = FB[3];
		b[at + 1] = FB[2];
		b[at + 2] = FB[1];
		b[at + 3] = FB[0];
	}
}

/**
 * Writes `r` at `at` into `bytes` (a wasm inbox, a ring's scratch) and returns the end offset. No allocation for short
 * vectors; a long one at an aligned offset is copied with one Float32Array.set.
 */
export function writeSend(bytes: Uint8Array, at: number, r: SendRecord): number {
	const n = r.v.length;
	if (n > MAX_VALUES) throw new RangeError(`wire: ${n} values – at most ${MAX_VALUES} per record`);
	const end = at + HEADER + 4 * n;
	if (end > bytes.length) throw new RangeError(`wire: ${end - at} bytes do not fit at ${at} of ${bytes.length}`);
	const mode = MODES.indexOf(r.mode);
	if (mode < 0) throw new RangeError(`wire: mode ${r.mode}`);
	bytes[at] = OP_SEND;
	bytes[at + 1] = mode;
	bytes[at + 2] = n & 0xff;
	bytes[at + 3] = (n >> 8) & 0xff;
	bytes[at + 4] = r.node & 0xff;
	bytes[at + 5] = (r.node >> 8) & 0xff;
	bytes[at + 6] = r.port & 0xff;
	bytes[at + 7] = (r.port >> 8) & 0xff;
	putF32(bytes, at + 8, r.amount);
	const v = at + HEADER;
	if (n > 16 && LITTLE && (bytes.byteOffset + v) % 4 === 0) new Float32Array(bytes.buffer, bytes.byteOffset + v, n).set(r.v);
	else for (let i = 0; i < n; i++) putF32(bytes, v + 4 * i, r.v[i]);
	return end;
}

export function encodeSend(r: SendRecord): Uint8Array {
	const out = new Uint8Array(recordSize(r.v.length));
	writeSend(out, 0, r);
	return out;
}

export class WireError extends Error {
	readonly at: number;
	readonly what: string;
	constructor(at: number, what: string) {
		super(`wire: ${what} at byte ${at}`);
		this.at = at;
		this.what = what;
	}
}

/** The records of `bytes`, in order (the values copied out); a malformed one throws a WireError with its offset. */
export function* readRecords(bytes: Uint8Array): Generator<SendRecord & { v: Float32Array }> {
	const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
	let at = 0;
	while (at < bytes.length) {
		if (bytes.length - at < HEADER) throw new WireError(at, 'a truncated header');
		const n = view.getUint16(at + 2, true);
		if (view.getUint8(at) !== OP_SEND) throw new WireError(at, 'an unknown op');
		const mode = MODES[view.getUint8(at + 1)];
		if (!mode) throw new WireError(at, 'an unknown mode');
		const end = at + recordSize(n);
		if (end > bytes.length) throw new WireError(at, 'truncated values');
		const v = new Float32Array(n);
		for (let i = 0; i < n; i++) v[i] = view.getFloat32(at + HEADER + 4 * i, true);
		yield { mode, node: view.getUint16(at + 4, true), port: view.getUint16(at + 6, true), amount: view.getFloat32(at + 8, true), v };
		at = end;
	}
}

/** One audio output's meter reading from the frame (#373): the held peak, its place on the dB scale and its zone. */
export interface OutputReading {
	/** the held peak, linear 0..1 */
	level: number;
	/** where it sits on the meter's dB scale, 0..1 from the bottom */
	position: number;
	zone: MeterZone;
}

/** The analysis frame as named values; `spectrum` is a view on `frame` (no copy). */
export interface Analysis {
	level: number;
	bass: number;
	mid: number;
	high: number;
	spectrum: Float32Array;
	/** the output limiter's gain reduction (dB, 0 = untouched) – the being's red face (#188) */
	reduction: number;
	/** #282, #373: one peak (0..1) per device channel – the largest sample, held and falling; a view on `frame`, as many as
	 * the engine renders */
	outputs: Float32Array;
	/** #373: where each output sits on the meter's dB scale, 0..1 from the bottom – a view, as many as `outputs` */
	positions: Float32Array;
	/** #373: each output's zone as a code – an index into `METER_ZONES` – a view, as many as `outputs` */
	zones: Float32Array;
	/** #373: the limiter's gain reduction as a 0..1 bar (full at 12 dB) */
	limiterBar: number;
	/** #320: the heard block's values of every load-table row (its inputs, then its outputs, from its `at`) – a view */
	values: Float32Array;
}

/** a count slot of the frame as a slice length: whole, within 0 … `max` */
const counted = (x: number | undefined, max: number): number => Math.max(0, Math.min(max, Math.round(x || 0)));

export const analysisOf = (frame: Float32Array): Analysis => ({
	level: frame[0],
	bass: frame[1],
	mid: frame[2],
	high: frame[3],
	spectrum: frame.subarray(SPECTRUM_AT, SPECTRUM_AT + SPECTRUM),
	reduction: frame[REDUCTION] ?? 0,
	outputs: frame.subarray(PEAKS, PEAKS + counted(frame[OUTPUT_COUNT], MAX_OUTPUTS)),
	positions: frame.subarray(POSITIONS, POSITIONS + counted(frame[OUTPUT_COUNT], MAX_OUTPUTS)),
	zones: frame.subarray(ZONES, ZONES + counted(frame[OUTPUT_COUNT], MAX_OUTPUTS)),
	limiterBar: frame[LIMITER_BAR] ?? 0,
	values: frame.subarray(VALUES_AT, VALUES_AT + counted(frame[VALUE_COUNT], MAX_FRAME_VALUES))
});
