// The worker's side of the boundary (#184; #320: deck-engine inside): live_core.wasm (deck/live-core) behind views on
// its memory, and the pump that keeps the audio ahead of the speaker. No worker globals here – worker.ts wires it to
// messages and rings, the tests run it in Node with the real wasm.
//
// Hot path, no JSON, no glue: records go from the control ring straight into the wasm inbox (one copy, then
// `lc_apply`), audio blocks (the device channels, interleaved) and analysis frames come out as Float32Array views on
// wasm memory (one copy into the audio ring / the analysis slot). JSON only at the edge: the load table, once. The
// engine's control half steps inside its render – the pump only renders.
import { FRAME, REDUCTION, VALUE_COUNT } from './wire.ts';
import { BlockTimes } from './timing.ts';
import type { Ring } from './ring.ts';
import type { ClockOp } from './messages.ts';

export interface Exports {
	memory: WebAssembly.Memory;
	lc_init(rate: number, channels: number): void;
	lc_inbox(len: number): number;
	lc_outbox(): number;
	lc_load(len: number): number;
	/** #356: the inbox's scene played silently by a throwaway twin for `quanta` quanta (0 = the core's default); 0 or −len */
	lc_warm(len: number, quanta: number): number;
	lc_apply(len: number): number;
	lc_render(frames: number): number;
	lc_analyse(dt: number, lead: number): number;
	/** #282: the device messages since the last call – their length; the records are at `lc_signals_at()` */
	lc_signals(): number;
	lc_signals_at(): number;
	/** #321: the tempo clock (deck/live-core clock.rs) – one frame → the state block's address (`CLOCK_STATE` f64) */
	lc_clock_frame(nowMs: number): number;
	lc_clock_stamp(context: number, heardMs: number, skew: number): void;
	lc_clock_pong(t0Us: number, t1Ms: number, t2Ms: number): void;
	lc_clock_tempo(bpm: number, perBar: number, beat: number, at: number, running: number, arrivedMs: number): void;
	lc_clock_midi(status: number, d1: number, d2: number, atMs: number): void;
	lc_clock_joined(nowMs: number): void;
	lc_clock_choose(source: number, lead: number): void;
	lc_clock_key(key: number, atMs: number): void;
	/** #321: the clock's line (deck-core `Controller::status`, the native HUD's text) into the outbox → its length */
	lc_clock_line(nowMs: number): number;
	/** #321: the MIDI clock out (deck-core's `ClockOut`) on / off; its messages of the last frame: how many, where */
	lc_clock_out_on(on: number): void;
	lc_clock_out(): number;
	lc_clock_out_at(): number;
}

/** #321: the clock's state block, f64 (deck/live-core clock.rs `STATE`) */
export const CLOCK_STATE = 16;

/** One node of the load table: its inputs (`ports`, what records' `port` counts), its outputs, and where its values are
 * in the analysis frame's values (`at`: the inputs, then the outputs; -1 = not carried). */
export interface TableNode {
	id: string;
	kind: string;
	ports: string[];
	outs: string[];
	at: number;
}

/** The load table: the device channels, the scene's nodes in the order the records' `node` index counts, and what the
 * build did with the device in words (a fold, an outlet without channels). */
export interface Table {
	channels: number;
	nodes: TableNode[];
	notes: string[];
}

export class WasmCore {
	readonly x: Exports;
	readonly rate: number;
	/** the device channels the engine renders (interleaved) */
	readonly channels: number;
	private inboxPtr = 0;
	private inboxView = new Uint8Array(0);
	private buffer: ArrayBuffer | null = null;
	private renderView: Float32Array | null = null;
	loaded = false;

	constructor(exports: Exports, rate: number, channels: number) {
		this.x = exports;
		this.rate = rate;
		this.channels = channels;
		exports.lc_init(rate, channels);
	}

	static async instantiate(wasm: WebAssembly.Module | BufferSource, rate: number, channels = 2): Promise<WasmCore> {
		const module = wasm instanceof WebAssembly.Module ? wasm : await WebAssembly.compile(wasm);
		const instance = await WebAssembly.instantiate(module, {});
		return new WasmCore(instance.exports as unknown as Exports, rate, channels);
	}

	/** A view on the inbox, at least `len` bytes; re-made when the inbox moved or the memory grew. */
	inbox(len: number): Uint8Array {
		if (len > this.inboxView.length || this.buffer !== this.x.memory.buffer) {
			this.inboxPtr = this.x.lc_inbox(len);
			this.buffer = this.x.memory.buffer;
			this.inboxView = new Uint8Array(this.buffer, this.inboxPtr, Math.max(len, this.inboxView.length));
			// the inbox may be longer than asked; the view covers what we asked for at least
		}
		return this.inboxView;
	}

	private text(n: number): string {
		return new TextDecoder().decode(new Uint8Array(this.x.memory.buffer, this.x.lc_outbox(), Math.abs(n)));
	}

	/** The scene (the deck's TOML) → the load table. A broken scene throws and keeps the running one. */
	load(scene: string): Table {
		const bytes = new TextEncoder().encode(scene);
		this.inbox(bytes.length).set(bytes);
		const n = this.x.lc_load(bytes.length);
		if (n < 0) throw new Error(this.text(n));
		this.loaded = true;
		return JSON.parse(this.text(n)) as Table;
	}

	/** #321: one clock frame at the page's `nowMs` → the state block (a view, valid until the next call into the core) */
	clockFrame(nowMs: number): Float64Array {
		return new Float64Array(this.x.memory.buffer, this.x.lc_clock_frame(nowMs), CLOCK_STATE);
	}

	/** #321: the MIDI clock out's messages of the last clock frame (a copy: [page ms, status, d1, d2] each), null = none */
	clockOut(): Float64Array | null {
		const n = this.x.lc_clock_out();
		return n ? new Float64Array(this.x.memory.buffer, this.x.lc_clock_out_at(), 4 * n).slice() : null;
	}

	/** #321: the clock's line at the page's `nowMs` – deck-core's words, the same the native HUD shows ("" = nothing to say) */
	clockLine(nowMs: number): string {
		return this.text(this.x.lc_clock_line(nowMs));
	}

	/** #321: something for the clock; `skew` = the worklet's (a stamp's context frame → the frames it played) */
	clock(o: ClockOp, skew: number): void {
		const x = this.x;
		switch (o.op) {
			case 'stamp':
				return x.lc_clock_stamp(o.context, o.heard, skew);
			case 'pong':
				return x.lc_clock_pong(o.t0, o.t1, o.t2);
			case 'tempo':
				return x.lc_clock_tempo(o.bpm, o.perBar, o.beat, o.at, o.running ? 1 : 0, o.arrived);
			case 'midi':
				return x.lc_clock_midi(o.bytes[0], o.bytes[1], o.bytes[2], o.at);
			case 'joined':
				return x.lc_clock_joined(o.now);
			case 'choose':
				return x.lc_clock_choose(o.source, o.lead ? 1 : 0);
			case 'key':
				return x.lc_clock_key(o.key, o.at);
			case 'out':
				return x.lc_clock_out_on(o.on ? 1 : 0);
		}
	}

	/** #356: the warm-up – `scene` rendered silently by a throwaway twin (the loaded core plays on untouched), so its
	 * code is compiled and optimised before it is heard; `quanta` 0 = the core's `WARM_QUANTA`. Throws on a broken scene. */
	warm(scene: string, quanta = 0): void {
		const bytes = new TextEncoder().encode(scene);
		this.inbox(bytes.length).set(bytes);
		const n = this.x.lc_warm(bytes.length, quanta);
		if (n < 0) throw new Error(this.text(n));
	}

	/** Applies the first `len` bytes of the inbox (records written there in place); throws with the core's reason. */
	apply(len: number): number {
		const n = this.x.lc_apply(len);
		if (n < 0) throw new Error(this.text(n));
		return n;
	}

	/** Copies `bytes` into the inbox and applies them (the transfer path; the shared path reads its ring in place). */
	applyBytes(bytes: Uint8Array): number {
		this.inbox(bytes.length).set(bytes);
		return this.apply(bytes.length);
	}

	/** `frames` rendered frames, the device channels interleaved – a view, valid until the next call into the core. The
	 * view is kept while the core renders into the same place (#208: no garbage per block in the worker – a GC pause
	 * there is a dropout). */
	render(frames: number): Float32Array {
		const ptr = this.x.lc_render(frames);
		const len = frames * this.channels;
		const v = this.renderView;
		if (v && v.buffer === this.x.memory.buffer && v.byteOffset === ptr && v.length === len) return v;
		return (this.renderView = new Float32Array(this.x.memory.buffer, ptr, len));
	}

	/** The analysis frame (wire.ts: FRAME f32, then its values) of what the speaker plays now (`lead` = frames queued
	 * ahead) – a view. */
	analyse(dt: number, lead: number): Float32Array {
		const ptr = this.x.lc_analyse(dt, lead);
		const count = new Float32Array(this.x.memory.buffer, ptr, FRAME)[VALUE_COUNT];
		return new Float32Array(this.x.memory.buffer, ptr, FRAME + count);
	}

	/** #282: the scene's `[outputs]` messages since the last call, as signal records (signals.ts) – a view (empty when
	 * there are none), valid until the next call into the core. */
	signals(): Uint8Array {
		const n = this.x.lc_signals();
		return n === 0 ? EMPTY : new Uint8Array(this.x.memory.buffer, this.x.lc_signals_at(), n);
	}
}

const EMPTY = new Uint8Array(0);

/** the engine's quantum (frames) – the worker renders in it */
export const BLOCK = 128;

export interface EngineOptions {
	/** frames kept ahead of the speaker */
	lead: number;
	/** an analysis frame per this many rendered frames (default ≈ 60 per second) */
	analysisEvery?: number;
	onAnalysis(frame: Float32Array): void;
	/** #282: the device messages since the last analysis frame, as signal records (a view – copy it); only called when
	 * there are some */
	onSignals?(records: Uint8Array): void;
}

/** A second of the pump (`Engine.stats`): µs per 128-frame block against its budget. */
export interface BlockStats {
	blocks: number;
	records: number;
	/** the mean, the median, the 99th percentile and the longest block (µs): a control step + the render + the copy out */
	renderUs: number;
	renderP50Us: number;
	renderP99Us: number;
	renderPeakUs: number;
	/** #356: the longest analysis frame + signals (µs) – between two blocks */
	analysisPeakUs: number;
	/** a block's duration (2667 µs at 48 kHz) */
	budgetUs: number;
	/** #356: the output limiter's gain reduction over the second's analysis frames: the deepest and the mean (dB), and
	 *  the share of frames it held the sound down by more than `LIMITING_DB` (%) */
	limitPeakDb: number;
	limitMeanDb: number;
	limitingPct: number;
}

/** #356: a gain reduction past this (dB) counts as limiting – below it the limiter only shaves peaks unheard */
export const LIMITING_DB = 1;

/** The pump: control records in, audio blocks out, an analysis frame now and then. Lead and fills count frames; the
 * audio ring holds samples (frames × channels). */
export class Engine {
	readonly core: WasmCore;
	/** frames kept ahead of the speaker – the worker adapts it (#208, lead.ts) */
	lead: number;
	private readonly every: number;
	private readonly onAnalysis: (frame: Float32Array) => void;
	private readonly onSignals: ((records: Uint8Array) => void) | undefined;
	private sinceAnalysis = 0;
	/** records applied since the last `stats()` */
	private records = 0;
	/** #356: each block's render time since the last `stats()` (p50 / p99 / the longest) */
	private readonly times = new BlockTimes();
	/** #356: the longest analysis frame (+ the signals) since the last `stats()` (ms) – between blocks, not in them */
	private analysisPeakMs = 0;
	/** #356: the output limiter's gain reduction per analysis frame since the last `stats()` (dB): the deepest, the sum,
	 *  the frames, the frames it held the sound down by more than `LIMITING_DB` – pumping can sound like a dropout */
	private limitPeakDb = 0;
	private limitSumDb = 0;
	private limitFrames = 0;
	private limitingFrames = 0;

	constructor(core: WasmCore, o: EngineOptions) {
		this.core = core;
		this.lead = o.lead;
		this.every = o.analysisEvery ?? Math.round(core.rate / 60);
		this.onAnalysis = o.onAnalysis;
		this.onSignals = o.onSignals;
	}

	/** Moves every complete record of the control ring into the inbox and applies them; returns how many. */
	drain(control: Ring<Uint8Array>): number {
		const n = control.available();
		if (n === 0 || !this.core.loaded) return 0;
		const got = control.read(this.core.inbox(n).subarray(0, n));
		const applied = this.core.apply(got);
		this.records += applied;
		return applied;
	}

	applyBytes(bytes: Uint8Array): number {
		if (!this.core.loaded) return 0;
		const applied = this.core.applyBytes(bytes);
		this.records += applied;
		return applied;
	}

	/**
	 * One block: the audio handed to `sink` (a view – copy it there), then maybe an analysis frame. `fill` = frames
	 * queued ahead of the speaker before this block.
	 */
	block(fill: number, sink: (audio: Float32Array) => void): void {
		const t = performance.now();
		sink(this.core.render(BLOCK));
		const t1 = performance.now();
		this.times.push((t1 - t) * 1000);
		this.sinceAnalysis += BLOCK;
		if (this.sinceAnalysis >= this.every) {
			const frame = this.core.analyse(this.sinceAnalysis / this.core.rate, fill + BLOCK);
			this.sinceAnalysis = 0;
			const db = frame[REDUCTION];
			if (db > this.limitPeakDb) this.limitPeakDb = db;
			this.limitSumDb += db;
			this.limitFrames++;
			if (db > LIMITING_DB) this.limitingFrames++;
			this.onAnalysis(frame);
			if (this.onSignals) {
				const records = this.core.signals();
				if (records.length) this.onSignals(records);
			}
			const ms = performance.now() - t1;
			if (ms > this.analysisPeakMs) this.analysisPeakMs = ms;
		}
	}

	/** Shared transport: renders into the audio ring until `lead` frames are queued; returns the blocks rendered. */
	fill(audio: Ring<Float32Array>): number {
		if (!this.core.loaded) return 0;
		const ch = this.core.channels;
		let n = 0;
		const sink = (b: Float32Array) => audio.write(b); // one closure per fill, not per block
		while (audio.available() / ch + BLOCK <= this.lead && audio.space() >= BLOCK * ch) {
			this.block(audio.available() / ch, sink);
			n++;
		}
		return n;
	}

	/** since the last call: blocks, records, the average, (#356) p50 / p99 and (#208) the longest block's render against a
	 * block's budget, the longest analysis frame */
	stats(): BlockStats {
		const t = this.times;
		const s = {
			blocks: t.count,
			records: this.records,
			renderUs: t.mean,
			renderP50Us: t.quantile(0.5),
			renderP99Us: t.quantile(0.99),
			renderPeakUs: t.max,
			analysisPeakUs: this.analysisPeakMs * 1000,
			budgetUs: (BLOCK / this.core.rate) * 1e6,
			limitPeakDb: this.limitPeakDb,
			limitMeanDb: this.limitFrames ? this.limitSumDb / this.limitFrames : 0,
			limitingPct: this.limitFrames ? (100 * this.limitingFrames) / this.limitFrames : 0
		};
		t.reset();
		this.records = this.analysisPeakMs = 0;
		this.limitPeakDb = this.limitSumDb = this.limitFrames = this.limitingFrames = 0;
		return s;
	}
}
