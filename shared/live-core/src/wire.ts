// The wire format across the page / worker ↔ wasm boundary (#184, DECISIONS 2026-10-30): binary records, no JSON.
// The Rust twin is deck/live-core/src/wire.rs; deck/live-core/fixtures/wire.json holds records with their bytes and
// both sides' tests encode and decode every one of them.
//
// A record, little-endian, 4-byte aligned (the values can be read as a Float32Array in place):
//   0 u8 op (1 = send) · 1 u8 mode (0 force, 1 blend, 2 release) · 2 u16 n · 4 u16 node · 6 u16 port · 8 f32 amount
//   12 f32 × n – the values (#189's float vectors); v[0] → port, v[1] → port + 1 …
// Node and port are indices from the load table (names are resolved once, at load – JSON stays at the edge).
// The analysis frame going out is a fixed block of FRAME f32: level, bass, mid, high, then SPECTRUM bins.

export const HEADER = 12;
export const OP_SEND = 1;
/** deck-core's analysis::SPECTRUM – bin k = k · rate / 1024 Hz */
export const SPECTRUM = 512;
export const FRAME = 4 + SPECTRUM;
/** the most values one record carries (u16 n) */
export const MAX_VALUES = 0xffff;

export type Mode = 'force' | 'blend' | 'release';
export const MODES: readonly Mode[] = ['force', 'blend', 'release'];

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
	bytes[at + 3] = n >> 8;
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

/** The analysis frame as named values; `spectrum` is a view on `frame` (no copy). */
export interface Analysis {
	level: number;
	bass: number;
	mid: number;
	high: number;
	spectrum: Float32Array;
}

export const analysisOf = (frame: Float32Array): Analysis => ({
	level: frame[0],
	bass: frame[1],
	mid: frame[2],
	high: frame[3],
	spectrum: frame.subarray(4, FRAME)
});
