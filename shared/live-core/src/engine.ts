// The worker's side of the boundary (#184): live_core.wasm (deck/live-core) behind views on its memory, and the pump
// that keeps the audio ahead of the speaker. No worker globals here – worker.ts wires it to messages and rings, the
// tests run it in Node with the real wasm.
//
// Hot path, no JSON, no glue: records go from the control ring straight into the wasm inbox (one copy, then
// `lc_apply`), audio blocks and analysis frames come out as Float32Array views on wasm memory (one copy into the
// audio ring / the analysis slot). JSON only at the edge: the load table, once.
import { FRAME } from './wire.ts';
import type { Ring } from './ring.ts';

export interface Exports {
	memory: WebAssembly.Memory;
	lc_init(rate: number): void;
	lc_inbox(len: number): number;
	lc_outbox(): number;
	lc_load(len: number): number;
	lc_apply(len: number): number;
	lc_step(dt: number): void;
	lc_render(frames: number): number;
	lc_analyse(dt: number, lead: number): number;
}

/** The load table: the scene's nodes with their input ports, in the order the records' `node` index counts. */
export interface Table {
	nodes: { id: string; kind: string; ports: string[] }[];
}

export class WasmCore {
	readonly x: Exports;
	readonly rate: number;
	private inboxPtr = 0;
	private inboxView = new Uint8Array(0);
	private buffer: ArrayBuffer | null = null;
	loaded = false;

	constructor(exports: Exports, rate: number) {
		this.x = exports;
		this.rate = rate;
		exports.lc_init(rate);
	}

	static async instantiate(wasm: WebAssembly.Module | BufferSource, rate: number): Promise<WasmCore> {
		const module = wasm instanceof WebAssembly.Module ? wasm : await WebAssembly.compile(wasm);
		const instance = await WebAssembly.instantiate(module, {});
		return new WasmCore(instance.exports as unknown as Exports, rate);
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

	/** The scene (deck/stage's TOML) → the load table. A broken scene throws and keeps the running one. */
	load(scene: string): Table {
		const bytes = new TextEncoder().encode(scene);
		this.inbox(bytes.length).set(bytes);
		const n = this.x.lc_load(bytes.length);
		if (n < 0) throw new Error(this.text(n));
		this.loaded = true;
		return JSON.parse(this.text(n)) as Table;
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

	step(dt: number): void {
		this.x.lc_step(dt);
	}

	/** `frames` rendered samples – a view, valid until the next call into the core. */
	render(frames: number): Float32Array {
		return new Float32Array(this.x.memory.buffer, this.x.lc_render(frames), frames);
	}

	/** The analysis frame (wire.ts FRAME) of what the speaker plays now (`lead` = frames queued ahead) – a view. */
	analyse(dt: number, lead: number): Float32Array {
		return new Float32Array(this.x.memory.buffer, this.x.lc_analyse(dt, lead), FRAME);
	}
}

export const BLOCK = 128;

export interface EngineOptions {
	/** frames kept ahead of the speaker */
	lead: number;
	/** an analysis frame per this many rendered frames (default ≈ 60 per second) */
	analysisEvery?: number;
	onAnalysis(frame: Float32Array): void;
}

/** The pump: control records in, audio blocks out (control stepped once per block), an analysis frame now and then. */
export class Engine {
	readonly core: WasmCore;
	readonly lead: number;
	private readonly every: number;
	private readonly onAnalysis: (frame: Float32Array) => void;
	private sinceAnalysis = 0;
	/** blocks rendered, records applied, µs spent rendering (since the last `stats()`) */
	private blocks = 0;
	private records = 0;
	private renderMs = 0;

	constructor(core: WasmCore, o: EngineOptions) {
		this.core = core;
		this.lead = o.lead;
		this.every = o.analysisEvery ?? Math.round(core.rate / 60);
		this.onAnalysis = o.onAnalysis;
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
	 * One block: a control step, the audio handed to `sink` (a view – copy it there), then maybe an analysis frame.
	 * `fill` = frames queued ahead of the speaker before this block.
	 */
	block(fill: number, sink: (audio: Float32Array) => void): void {
		const t = performance.now();
		this.core.step(BLOCK / this.core.rate);
		sink(this.core.render(BLOCK));
		this.renderMs += performance.now() - t;
		this.blocks++;
		this.sinceAnalysis += BLOCK;
		if (this.sinceAnalysis >= this.every) {
			const frame = this.core.analyse(this.sinceAnalysis / this.core.rate, fill + BLOCK);
			this.sinceAnalysis = 0;
			this.onAnalysis(frame);
		}
	}

	/** Shared transport: renders into the audio ring until `lead` frames are queued; returns the blocks rendered. */
	fill(audio: Ring<Float32Array>): number {
		if (!this.core.loaded) return 0;
		let n = 0;
		while (audio.available() + BLOCK <= this.lead && audio.space() >= BLOCK) {
			this.block(audio.available(), (b) => audio.write(b));
			n++;
		}
		return n;
	}

	stats(): { blocks: number; records: number; renderUs: number } {
		const s = { blocks: this.blocks, records: this.records, renderUs: this.blocks ? (this.renderMs * 1000) / this.blocks : 0 };
		this.blocks = this.records = this.renderMs = 0;
		return s;
	}
}
