// @scutoons/live-core – the browser Live mode's engine (#184, DECISIONS 2026-10-30): deck-core as wasm in a Web
// Worker (graph + analysis + DSP), its audio played by an AudioWorklet through a SharedArrayBuffer ring.
//
//   const core = await LiveCore.start({ worker, wasm, worklet });   // after a user gesture (the AudioContext)
//   await core.load(sceneToml);                                      // the edge: TOML in, the node table out
//   core.set('lead', new Float32Array([440, 2, 1, 0.1]), { mode: 'blend', amount: 0.5 });
//   const off = core.onAnalysis((a) => draw(a.level, a.bass, a.mid, a.high, a.spectrum));
//
// The exchange format (deck/lab/results/formats.md): wire records (wire.ts) – no JSON across the boundary. With
// `crossOriginIsolated` (COOP / COEP) records, audio and analysis go through SharedArrayBuffer rings and no message
// is sent per value; without it the same bytes travel as transferred ArrayBuffers (`transport === 'transfer'`):
// it works, at one postMessage per send, per analysis frame and per 512 audio frames.
import { BLOCK, type Table } from './engine.ts';
import { Latest, Ring, ringBuffers, sharedAvailable } from './ring.ts';
import { analysisOf, FRAME, recordSize, writeSend, type Analysis, type Mode } from './wire.ts';
import type { FromWorker, FromWorklet, SharedSetup, ToWorker, Transport, WorkerStats, WorkletOptions } from './messages.ts';

export * from './wire.ts';
export { Ring, Latest, ringBuffers, sharedAvailable } from './ring.ts';
export type { Table } from './engine.ts';
export type { Transport, WorkerStats } from './messages.ts';

export interface StartOptions {
	/** the bundled worker.ts, started by the app (its bundler resolves the URL) */
	worker: Worker;
	/** live_core.wasm (deck/live-core, `npm run build:wasm -w shared/live-core`) */
	wasm: string | URL;
	/** the bundled worklet.ts module */
	worklet: string | URL;
	/** default: a new interactive AudioContext */
	context?: AudioContext;
	/** default: the context's destination */
	output?: AudioNode;
	/** 'auto' (default): shared when cross-origin isolated, else transfer */
	transport?: 'auto' | Transport;
	/** frames the worker keeps ahead of the speaker (default 1024 ≈ 21 ms at 48 kHz) – audio latency vs safety */
	lead?: number;
}

/** A resolved `node.port`: the indices a record carries. */
export interface Param {
	node: number;
	port: number;
	/** the inputs from `port` on (a vector's components land there in order) */
	ports: string[];
}

export interface SetOptions {
	/** force (default): the value replaces the input · blend: mixed in by `amount` · release: the input as built · base:
	 * the input's base – a linked input keeps its link on top (#188: a knob on a modulated input) */
	mode?: Mode;
	amount?: number;
}

export interface Stats extends WorkerStats {
	transport: Transport;
	underruns: number;
	/** underruns over the last minute (#188: the browser status shows it) */
	underrunsPerMinute: number;
	/** sends dropped because the control ring was full (shared transport) */
	dropped: number;
	/** #208: when the newest dropout reached the page (performance.now ms; -Infinity = none yet) */
	lastDropoutMs: number;
}

/** #208: how long the page's warning stays after a dropout (ms) */
export const ALERT_MS = 5000;

/** #208: a dropout within the last `hold` ms – the page shows its warning */
export function dropoutAlert(lastDropoutMs: number, now: number, hold = ALERT_MS): boolean {
	return now - lastDropoutMs < hold;
}

const CONTROL_BYTES = 1 << 16;
const AUDIO_FRAMES = 1 << 13;

export class LiveCore {
	readonly context: AudioContext;
	readonly node: AudioWorkletNode;
	readonly transport: Transport;
	table: Table | null = null;
	private readonly worker: Worker;
	private readonly control: Ring<Uint8Array> | null;
	private readonly latest: Latest | null;
	private readonly scratch = new Uint8Array(recordSize(4096));
	private readonly listeners = new Set<(a: Analysis) => void>();
	private readonly pending = new Map<number, { ok: (t: Table) => void; fail: (e: Error) => void }>();
	private nextId = 1;
	private raf = 0;
	private seen = 0;
	private frame: Float32Array = new Float32Array(FRAME);
	private stats_: Stats;
	/** the worklet's underrun count over the last minute: (ms, total) once a second */
	private readonly runs: { t: number; n: number }[] = [];

	private constructor(o: { context: AudioContext; node: AudioWorkletNode; worker: Worker; transport: Transport; shared: SharedSetup | null }) {
		this.context = o.context;
		this.node = o.node;
		this.worker = o.worker;
		this.transport = o.transport;
		this.control = o.shared ? Ring.bytes(o.shared.control) : null;
		this.latest = o.shared ? new Latest(o.shared.analysis) : null;
		this.stats_ = {
			transport: o.transport,
			underruns: 0,
			underrunsPerMinute: 0,
			dropped: 0,
			blocks: 0,
			records: 0,
			renderUs: 0,
			renderPeakUs: 0,
			budgetUs: 0,
			lastDropoutMs: -Infinity
		};
		this.runs.push({ t: globalThis.performance?.now() ?? 0, n: 0 });
	}

	/** Starts the worker (wasm compiled there), the worklet and the rings. Call it from a user gesture. */
	static async start(o: StartOptions): Promise<LiveCore> {
		const context = o.context ?? new AudioContext({ latencyHint: 'interactive' });
		const wanted = o.transport ?? 'auto';
		if (wanted === 'shared' && !sharedAvailable()) throw new Error('live-core: no SharedArrayBuffer – the page needs COOP / COEP (cross-origin isolated)');
		const transport: Transport = wanted === 'auto' ? (sharedAvailable() ? 'shared' : 'transfer') : wanted;
		const lead = o.lead ?? 1024;
		const shared: SharedSetup | null =
			transport === 'shared'
				? { control: ringBuffers(CONTROL_BYTES, 1, true), audio: ringBuffers(AUDIO_FRAMES, 4, true), analysis: Latest.buffers(FRAME, true) }
				: null;
		await context.audioWorklet.addModule(String(o.worklet));
		const options: WorkletOptions = { audio: shared?.audio ?? null, capacity: AUDIO_FRAMES };
		const node = new AudioWorkletNode(context, 'live-core', { numberOfInputs: 0, outputChannelCount: [2], processorOptions: options });
		node.connect(o.output ?? context.destination);
		const core = new LiveCore({ context, node, worker: o.worker, transport, shared });
		let audioPort: MessagePort | null = null;
		if (!shared) {
			const ch = new MessageChannel();
			node.port.postMessage({ feed: ch.port2 }, [ch.port2]);
			audioPort = ch.port1;
		}
		node.port.onmessage = (ev: MessageEvent<FromWorklet>) => core.underran(ev.data.underruns, performance.now());
		const ready = new Promise<void>((resolve, reject) => {
			o.worker.onmessage = (ev: MessageEvent<FromWorker>) => (ev.data.type === 'ready' ? resolve() : core.receive(ev.data));
			o.worker.onerror = (e) => reject(new Error(`live-core worker: ${e.message}`));
		});
		const wasm = new URL(String(o.wasm), globalThis.location?.href).href;
		core.post({ type: 'init', wasm, rate: context.sampleRate, lead: Math.max(lead, 4 * BLOCK), shared, audioPort }, audioPort ? [audioPort] : []);
		await ready;
		o.worker.onmessage = (ev: MessageEvent<FromWorker>) => core.receive(ev.data);
		if (context.state === 'suspended') await context.resume();
		return core;
	}

	private post(m: ToWorker, transfer: Transferable[] = []): void {
		this.worker.postMessage(m, transfer);
	}

	private receive(m: FromWorker): void {
		switch (m.type) {
			case 'loaded':
			case 'failed': {
				const p = this.pending.get(m.id);
				this.pending.delete(m.id);
				if (m.type === 'loaded') {
					this.table = m.table;
					p?.ok(m.table);
				} else p?.fail(new Error(m.error));
				break;
			}
			case 'analysis':
				this.frame = m.frame;
				this.emit();
				break;
			case 'stats':
				Object.assign(this.stats_, m.stats);
				break;
			case 'error':
				console.error('live-core:', m.error);
				break;
		}
	}

	/** A scene in deck/stage's TOML (the batch kinds: lfo, smooth, mul, add, osc, fm) → its node table. */
	load(scene: string): Promise<Table> {
		const id = this.nextId++;
		return new Promise((ok, fail) => {
			this.pending.set(id, { ok, fail });
			this.post({ type: 'load', id, scene });
		});
	}

	/** `node.port` (or `node` = its first input) → the indices records carry; resolve once, send often. */
	param(name: string): Param {
		if (!this.table) throw new Error('live-core: load a scene first');
		const [id, port] = name.split('.', 2);
		const node = this.table.nodes.findIndex((n) => n.id === id);
		if (node < 0) throw new Error(`live-core: no node '${id}'`);
		const ports = this.table.nodes[node].ports;
		const p = port === undefined ? 0 : ports.indexOf(port);
		if (p < 0) throw new Error(`live-core: node '${id}' has no input '${port}' (${ports.join(', ')})`);
		return { node, port: p, ports: ports.slice(p) };
	}

	/**
	 * Sends a float vector onto a node's inputs (#189): v[0] → the param's port, v[1] → the next input … (components
	 * past the node's last input are dropped). Shared transport: written into the control ring, applied before the next
	 * block; false = the ring was full (counted in `stats().dropped`).
	 */
	set(param: string | Param, values: ArrayLike<number>, o: SetOptions = {}): boolean {
		const p = typeof param === 'string' ? this.param(param) : param;
		const mode = o.mode ?? 'force';
		const r = { mode, node: p.node, port: p.port, amount: mode === 'blend' ? (o.amount ?? 0.5) : mode === 'force' ? 1 : 0, v: values };
		if (this.control) {
			const end = writeSend(this.scratch, 0, r);
			if (this.control.writeAll(this.scratch.subarray(0, end))) return true;
			this.stats_.dropped++;
			return false;
		}
		const bytes = new Uint8Array(recordSize(values.length));
		writeSend(bytes, 0, r);
		this.post({ type: 'records', bytes: bytes.buffer }, [bytes.buffer]);
		return true;
	}

	/** Every new analysis frame (≈ 60 per second), on the page's animation frames. Returns the unsubscribe. */
	onAnalysis(cb: (a: Analysis) => void): () => void {
		this.listeners.add(cb);
		if (this.latest && !this.raf) this.poll();
		return () => {
			this.listeners.delete(cb);
			if (!this.listeners.size && this.raf) {
				cancelAnimationFrame(this.raf);
				this.raf = 0;
			}
		};
	}

	private poll = (): void => {
		this.raf = requestAnimationFrame(this.poll);
		const s = this.latest!.take(this.frame, this.seen);
		if (s !== this.seen) {
			this.seen = s;
			this.emit();
		}
	};

	private emit(): void {
		const a = analysisOf(this.frame);
		for (const cb of this.listeners) cb(a);
	}

	stats(): Stats {
		return { ...this.stats_ };
	}

	/** #208: a dropout in the last few seconds – the page's warning (a live tool may not crackle unnoticed) */
	glitching(now = performance.now()): boolean {
		return dropoutAlert(this.stats_.lastDropoutMs, now);
	}

	/** the worklet's running count `n` at `t` ms → the total and the last minute's */
	underran(n: number, t: number): void {
		if (n > this.stats_.underruns) this.stats_.lastDropoutMs = t;
		this.stats_.underruns = n;
		this.stats_.underrunsPerMinute = perMinute(this.runs, n, t);
	}

	async stop(): Promise<void> {
		if (this.raf) cancelAnimationFrame(this.raf);
		this.raf = 0;
		this.post({ type: 'stop' });
		this.node.disconnect();
		this.worker.terminate();
		await this.context.close();
	}
}

/** Underruns over the last minute from a running count sampled about once a second (`runs` keeps the window). */
export function perMinute(runs: { t: number; n: number }[], n: number, t: number): number {
	runs.push({ t, n });
	while (runs.length > 1 && t - runs[0].t > 60_000) runs.shift();
	return Math.max(0, n - runs[0].n);
}
