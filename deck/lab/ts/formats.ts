// #184: the exchange formats across the JS ↔ wasm boundary, shared by Node (ts/bench-formats.ts) and the browser
// worker (www/formats-worker.ts). Every candidate does the whole trip a worker would: encode in TS → cross into wasm →
// decode → apply to the same loaded scene (live-core's Core::send), or for the analysis frame the way out: wasm →
// JS values. Rows: µs for the TS encode alone and for the whole trip (median of 7 runs, suite.ts `time`).
//
// Candidates:
// - JSON: JSON.stringify → wasm-bindgen &str (TextEncoder into wasm memory) → serde_json (the cue: protocol::Envelope)
// - postcard: a TS encoder (below) → wasm-bindgen &[u8] (copied) → postcard + serde (the cue: a typed mirror)
// - wire, copied: live-core's records (shared/live-core wire.ts) → wasm-bindgen &[u8] (copied) → wire.rs
// - wire, in place: the records written straight into live_core.wasm's inbox (a view on wasm memory) → lc_apply
import { FRAME, writeSend, type Mode, type SendRecord } from '../../../shared/live-core/src/wire.ts';
import type { WasmCore } from '../../../shared/live-core/src/engine.ts';
import { ENVELOPE, time } from './suite.ts';

export type FormatRow = { host: string; workload: string; format: string; encodeUs: number | null; totalUs: number };

/** The shape of core_wasm.js's format functions (core-wasm/src/formats.rs). */
export type FormatsWasm = {
	fmt_init(): number;
	fmt_scene(): string;
	fmt_json_send(text: string): number;
	fmt_json_frame(): string;
	fmt_json_cue(text: string): number;
	fmt_pc_send(bytes: Uint8Array): number;
	fmt_pc_frame(): Uint8Array;
	fmt_pc_cue(bytes: Uint8Array): number;
	fmt_pc_check(send: Uint8Array, cue: Uint8Array): string;
	fmt_wire_send(bytes: Uint8Array): number;
	fmt_wire_frame(): Float32Array;
};

// ------------------------------------------------------------------ postcard in TypeScript (what a generator emits)

const enc = new TextEncoder();

/** postcard's wire rules: varint (LEB128) integers above u8, raw u8 / bool, f32 LE, length-prefixed strings and
 *  sequences, Option = 0 | 1 + value, enum = varint variant index + fields. */
export class PostcardWriter {
	readonly b = new Uint8Array(4096);
	private readonly v = new DataView(this.b.buffer);
	at = 0;
	reset(): this {
		this.at = 0;
		return this;
	}
	u8(x: number): void {
		this.b[this.at++] = x;
	}
	varint(x: number): void {
		while (x >= 0x80) {
			this.b[this.at++] = (x % 0x80) | 0x80;
			x = Math.floor(x / 0x80);
		}
		this.b[this.at++] = x;
	}
	f32(x: number): void {
		this.v.setFloat32(this.at, x, true);
		this.at += 4;
	}
	str(s: string): void {
		const r = enc.encodeInto(s, this.b.subarray(this.at + 1));
		if (r.written < 0x80) {
			this.b[this.at] = r.written;
			this.at += 1 + r.written;
		} else {
			const bytes = enc.encode(s);
			this.varint(bytes.length);
			this.b.set(bytes, this.at);
			this.at += bytes.length;
		}
	}
	opt<T>(x: T | undefined, f: (x: T) => void): void {
		if (x === undefined) this.u8(0);
		else {
			this.u8(1);
			f(x);
		}
	}
	bytes(): Uint8Array {
		return this.b.subarray(0, this.at);
	}
}

const MODE_INDEX: Record<Mode, number> = { force: 0, blend: 1, release: 2 };

export function postcardSend(w: PostcardWriter, r: SendRecord): Uint8Array {
	w.reset();
	w.u8(MODE_INDEX[r.mode]);
	w.varint(r.node);
	w.varint(r.port);
	w.f32(r.amount);
	w.varint(r.v.length);
	for (let i = 0; i < r.v.length; i++) w.f32(r.v[i]);
	return w.bytes();
}

type Envelope = {
	seq: number;
	at?: number;
	from: string;
	to?: { role?: string; group?: string; client?: string };
	p: { id: string; actions: Record<string, Record<string, unknown>>[] };
};

/** formats.rs CueP: the cue envelope with its actions typed (say / stage / send). */
export function postcardCue(w: PostcardWriter, e: Envelope): Uint8Array {
	w.reset();
	w.varint(e.seq);
	w.opt(e.at, (x) => w.varint(x));
	w.str(e.from);
	w.opt(e.to, (to) => {
		w.opt(to.role, (s) => w.str(s));
		w.opt(to.group, (s) => w.str(s));
		w.opt(to.client, (s) => w.str(s));
	});
	w.str(e.p.id);
	w.varint(e.p.actions.length);
	for (const a of e.p.actions) {
		if (a.say) {
			const s = a.say as { outlet: string; text: Record<string, string>; haptic: boolean };
			w.varint(0);
			w.str(s.outlet);
			const t = Object.entries(s.text);
			w.varint(t.length);
			for (const [k, v] of t) {
				w.str(k);
				w.str(v);
			}
			w.u8(s.haptic ? 1 : 0);
		} else if (a.stage) {
			const s = a.stage as { outlet: string; layer: string; show: boolean };
			w.varint(1);
			w.str(s.outlet);
			w.str(s.layer);
			w.u8(s.show ? 1 : 0);
		} else {
			const s = a.send as { outlet: string; midi: { type: string; ch: number; n: number; v: number } };
			w.varint(2);
			w.str(s.outlet);
			w.str(s.midi.type);
			w.u8(s.midi.ch);
			w.u8(s.midi.n);
			w.u8(s.midi.v);
		}
	}
	return w.bytes();
}

function postcardFrame(b: Uint8Array, dst: Float32Array): number {
	let n = 0;
	let shift = 1;
	let at = 0;
	for (;;) {
		const x = b[at++];
		n += (x & 0x7f) * shift;
		if (x < 0x80) break;
		shift *= 0x80;
	}
	const v = new DataView(b.buffer, b.byteOffset, b.byteLength);
	for (let i = 0; i < n; i++) dst[i] = v.getFloat32(at + 4 * i, true);
	return n;
}

// ------------------------------------------------------------------ the workloads

const FM = 300; // the first FM voice of the bench scene (300 control nodes before it)
const vec = (n: number) => Float32Array.from({ length: n }, (_, i) => 0.1 + i * 0.25);
const SENDS: { workload: string; r: SendRecord; iters: number }[] = [
	{ workload: 'control: one scalar (force)', r: { mode: 'force', node: FM, port: 3, amount: 1, v: vec(1) }, iters: 100_000 },
	{ workload: 'parameter vector, 4 floats (blend)', r: { mode: 'blend', node: FM, port: 0, amount: 0.5, v: vec(4) }, iters: 100_000 },
	{ workload: 'parameter vector, 64 floats (blend)', r: { mode: 'blend', node: FM, port: 0, amount: 0.5, v: vec(64) }, iters: 30_000 }
];
const FRAME_OUT = 'analysis frame out (4 bands + 512 bins)';
const CUE = 'cue / cell in (the 330 B relay envelope)';

let sink = 0;
export const formatsSink = (): number => sink;

/** The rows for one host. `core` = live_core.wasm with the same scene loaded (the in-place candidate). */
export function runFormats(host: string, w: FormatsWasm, core: WasmCore): FormatRow[] {
	const rows: FormatRow[] = [];
	const push = (workload: string, format: string, encodeUs: number | null, totalUs: number) => rows.push({ host, workload, format, encodeUs, totalUs });
	w.fmt_init();
	const pw = new PostcardWriter();
	const scratch = new Uint8Array(4096);
	const inbox = core.inbox(4096);
	for (const { workload, r, iters } of SENDS) {
		const json = () => JSON.stringify({ mode: r.mode, node: r.node, port: r.port, amount: r.amount, v: Array.from(r.v) });
		push(workload, 'JSON', time(iters, () => (sink += json().length)), time(iters, () => (sink += w.fmt_json_send(json()))));
		push(workload, 'postcard', time(iters, () => (sink += postcardSend(pw, r).length)), time(iters, () => (sink += w.fmt_pc_send(postcardSend(pw, r)))));
		push(
			workload,
			'wire, copied',
			time(iters, () => (sink += writeSend(scratch, 0, r))),
			time(iters, () => (sink += w.fmt_wire_send(scratch.subarray(0, writeSend(scratch, 0, r)))))
		);
		push(workload, 'wire, in place', time(iters, () => (sink += writeSend(inbox, 0, r))), time(iters, () => (sink += core.apply(writeSend(inbox, 0, r)))));
	}
	const dst = new Float32Array(FRAME);
	push(FRAME_OUT, 'JSON', null, time(10_000, () => (sink += (JSON.parse(w.fmt_json_frame()) as number[])[3])));
	push(FRAME_OUT, 'postcard', null, time(10_000, () => (sink += postcardFrame(w.fmt_pc_frame(), dst))));
	push(FRAME_OUT, 'wire, copied', null, time(10_000, () => (sink += w.fmt_wire_frame()[3])));
	const ptr = core.x.lc_analyse(1 / 60, 0);
	push(FRAME_OUT, 'wire, in place', null, time(10_000, () => {
		dst.set(new Float32Array(core.x.memory.buffer, ptr, FRAME));
		sink += dst[3];
	}));
	const env = JSON.parse(ENVELOPE) as Envelope;
	push(CUE, 'JSON', time(50_000, () => (sink += JSON.stringify(env).length)), time(50_000, () => (sink += w.fmt_json_cue(JSON.stringify(env)))));
	push(CUE, 'postcard', time(50_000, () => (sink += postcardCue(pw, env).length)), time(50_000, () => (sink += w.fmt_pc_cue(postcardCue(pw, env)))));
	push(CUE, 'TS only (JSON.parse – it stays in TS)', null, time(50_000, () => (sink += (JSON.parse(ENVELOPE) as Envelope).seq)));
	return rows;
}

/** Checks the TS postcard encoder against the Rust decoder (the bytes must decode to the values). */
export function checkPostcard(w: FormatsWasm): string {
	const pw = new PostcardWriter();
	const send = postcardSend(pw, SENDS[1].r).slice();
	return w.fmt_pc_check(send, postcardCue(pw, JSON.parse(ENVELOPE) as Envelope));
}

/** Median and spread per (host, workload, format) over several runs. */
export function summarise(runs: FormatRow[][]): (FormatRow & { min: number; max: number; runs: number })[] {
	const key = (r: FormatRow) => `${r.host}|${r.workload}|${r.format}`;
	const groups = new Map<string, FormatRow[]>();
	for (const run of runs) for (const r of run) groups.set(key(r), [...(groups.get(key(r)) ?? []), r]);
	const med = (xs: number[]) => [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)];
	return [...groups.values()].map((g) => {
		const totals = g.map((r) => r.totalUs);
		const enc = g.map((r) => r.encodeUs).filter((x): x is number => x !== null);
		return { ...g[0], encodeUs: enc.length ? med(enc) : null, totalUs: med(totals), min: Math.min(...totals), max: Math.max(...totals), runs: g.length };
	});
}

export function table(rows: ReturnType<typeof summarise>): string {
	const f = (x: number | null) => (x === null ? '–' : x.toFixed(3));
	const lines = ['| workload | format | TS encode µs | whole trip µs | spread (min – max) |', '|---|---|---|---|---|'];
	for (const r of rows) lines.push(`| ${r.workload} | ${r.format} | ${f(r.encodeUs)} | **${f(r.totalUs)}** | ${f(r.min)} – ${f(r.max)} |`);
	return lines.join('\n');
}
