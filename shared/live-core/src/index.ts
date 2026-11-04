// @scutoons/live-core – the browser deck's sound (#184, DECISIONS 2026-10-30; #320): deck-engine – the native deck's
// engine – as wasm in a Web Worker, its audio (the device's channels) played by an AudioWorklet through a
// SharedArrayBuffer ring. TypeScript here only moves bytes: what plays and how is the core's.
//
//   const core = await LiveCore.start({ worker, wasm, worklet });   // after a user gesture (the AudioContext)
//   await core.load(sceneToml);                                      // the edge: TOML in, the node table out
//   core.onClock((c) => strobe(beatAt(c, nowMs())));                 // #321: the tempo clock (the show clock's)
//   core.set('lead.freq', new Float32Array([440, 2, 1, 0.1]), { mode: 'blend', amount: 0.5 });
//   const off = core.onAnalysis((a) => draw(a.level, a.bass, a.mid, a.high, a.spectrum));
//   core.value('sway.out');                                          // the heard block's value of a path
//
// The exchange format (deck/lab/results/formats.md): wire records (wire.ts) – no JSON across the boundary. With
// `crossOriginIsolated` (COOP / COEP) records, audio and analysis go through SharedArrayBuffer rings and no message
// is sent per value; without it the same bytes travel as transferred ArrayBuffers (`transport === 'transfer'`):
// it works, at one postMessage per send, per analysis frame and per 512 audio frames.
import type { Table } from './engine.ts';
import { Latest, Ring, ringBuffers, sharedAvailable } from './ring.ts';
import { readSignals, type DeviceSignal } from './signals.ts';
import { analysisOf, FRAME, MAX_OUTPUTS, MAX_FRAME_VALUES, recordSize, writeSend, type Analysis, type Mode } from './wire.ts';
import type { ClockOp, FromWorker, FromWorklet, SharedSetup, ToWorker, Transport, WorkletOptions } from './messages.ts';
import { ClockWindow, clockOf, type ClockState } from './clock.ts';

export * from './wire.ts';
export * from './clock.ts';
export type { ClockOp } from './messages.ts';
export * from './signals.ts';
export { Ring, Latest, ringBuffers, sharedAvailable } from './ring.ts';
export type { Table, TableNode } from './engine.ts';
export type { Transport } from './messages.ts';

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
	/** the device class the core's adaptive lead is chosen for (#365, deck-core `lead`: its floor, start and cap) – the
	 *  page knows it (a coarse pointer = a phone); default desktop */
	device?: DeviceKind;
	/** the device channels the engine renders (default: the destination's `maxChannelCount` – `deviceChannels`) */
	channels?: number;
}

/** #364: deck-core's monitor as the page holds it (deck/space-web `DeckMonitor`, no GPU): samples in – each animation
 *  frame (drawn or skipped by the fps cap), this core's audio window (`audioTo`), the dropouts – the report out. The
 *  numbers, their judgement and their words are the core's; the page feeds and draws. */
export interface MonitorView {
	/** the quality rung's fps cap (0 = none) */
	target(fps: number): void;
	frame(nowMs: number, workMs: number, drawn: boolean): void;
	audio(nowMs: number, words: Float64Array): void;
	/** the worklet's dropouts so far */
	underruns(nowMs: number, total: number): void;
	silent(): void;
	/** deck-core `monitor::Report` as JSON (`MonitorReport`) */
	report(nowMs: number): string;
	/** the sound's first seconds as the report's table */
	series(): string[];
	/** #373: a device message (`DeviceSignal` as JSON) – the core words it and keeps the lamp's activity (deck-core `devices`) */
	signal(nowMs: number, json: string): void;
	/** #373: the rows of the protocols that carried a message (JSON `DeviceRow[]`) as of `nowMs` */
	devices(nowMs: number): string;
	resetDevices(): void;
}

/** #364: a bar item as the core worded and judged it */
export interface MonitorItem {
	label: string;
	value: string;
	level: 'ok' | 'warn' | 'bad';
}

/** #364: deck-core `monitor::Report` – the fields the pages read */
export interface MonitorReport {
	fps: number;
	sound: boolean;
	glitches: number;
	alert: string;
	alert_line: string;
	warm_ms: number;
	bar: MonitorItem[];
	lines: string[];
}

/** #365: the device classes the core's lead knows a browser by (deck-core `DeviceClass` codes) */
export type DeviceKind = 'phone' | 'desktop';
const DEVICE_CODE: Record<DeviceKind, number> = { phone: 0, desktop: 1 };

/** The device channels a destination offers (plan 2.6: its `maxChannelCount` – 2 on a phone; 0 (an old Safari) or
 * nothing = stereo), at most the engine's 32. The core decides how a scene's outlets fit them. */
export function deviceChannels(destination: { maxChannelCount?: number } | undefined): number {
	const n = Math.floor(destination?.maxChannelCount ?? 0);
	return n >= 1 ? Math.min(n, MAX_OUTPUTS) : 2;
}

/** The smallest power of two ≥ n (the rings' capacities). */
export const pow2At = (n: number): number => 2 ** Math.ceil(Math.log2(Math.max(2, n)));

/** Where `path` – `node.input`, `node.output`, or `node` (its first output) – is in an analysis frame's values; -1 =
 * nowhere (no such node or port, or past the frame's room). Inputs win over outputs of the same name. */
export function valueIndex(table: Table, path: string): number {
	const dot = path.lastIndexOf('.');
	const [id, port] = dot < 0 ? [path, ''] : [path.slice(0, dot), path.slice(dot + 1)];
	const n = table.nodes.find((x) => x.id === id);
	if (!n || n.at < 0) return -1;
	if (!port) return n.outs.length ? n.at + n.ports.length : -1;
	const i = n.ports.indexOf(port);
	if (i >= 0) return n.at + i;
	const o = n.outs.indexOf(port);
	return o >= 0 ? n.at + n.ports.length + o : -1;
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

export interface Stats {
	transport: Transport;
	/** the worklet's dropouts so far (the page's monitor counts them per window, #364) */
	underruns: number;
	/** sends dropped because the control ring was full (shared transport) */
	dropped: number;
}

const CONTROL_BYTES = 1 << 16;
const AUDIO_FRAMES = 1 << 13;

export class LiveCore {
	readonly context: AudioContext;
	readonly node: AudioWorkletNode;
	readonly transport: Transport;
	/** the device channels the engine renders */
	readonly channels: number;
	table: Table | null = null;
	/** the newest analysis frame's named values (`value`) */
	private analysis: Analysis | null = null;
	private readonly at = new Map<string, number>();
	private readonly worker: Worker;
	private readonly control: Ring<Uint8Array> | null;
	private readonly latest: Latest | null;
	private readonly scratch = new Uint8Array(recordSize(4096));
	private readonly listeners = new Set<(a: Analysis) => void>();
	private readonly signalListeners = new Set<(s: DeviceSignal) => void>();
	private readonly pending = new Map<number, { ok: (t: Table) => void; fail: (e: Error) => void }>();
	private nextId = 1;
	private raf = 0;
	private seen = 0;
	private frame: Float32Array = new Float32Array(FRAME + MAX_FRAME_VALUES);
	private stats_: Stats;
	/** #364: the monitor that gets the core's audio window once a second */
	private monitor: Pick<MonitorView, 'audio'> | null = null;
	/** #321: the core's tempo clock as its last frame left it (null before the first) */
	clock: ClockState | null = null;
	/** #321: the core's clock line as the worker last posted it */
	private clockLine = '';
	/** #321: the show clock's error over the last minute (the sound report's line) */
	readonly clockWindow = new ClockWindow();
	private readonly clockListeners = new Set<(c: ClockState) => void>();
	private readonly midiOutListeners = new Set<(msgs: Float64Array) => void>();
	private stampTimer: ReturnType<typeof setInterval> | null = null;

	private constructor(o: { context: AudioContext; node: AudioWorkletNode; worker: Worker; transport: Transport; shared: SharedSetup | null; channels: number }) {
		this.context = o.context;
		this.node = o.node;
		this.worker = o.worker;
		this.transport = o.transport;
		this.channels = o.channels;
		this.control = o.shared ? Ring.bytes(o.shared.control) : null;
		this.latest = o.shared ? new Latest(o.shared.analysis) : null;
		this.stats_ = { transport: o.transport, underruns: 0, dropped: 0 };
	}

	/** Starts the worker (wasm compiled there), the worklet and the rings. Call it from a user gesture. */
	static async start(o: StartOptions): Promise<LiveCore> {
		const context = o.context ?? new AudioContext({ latencyHint: 'interactive' });
		const wanted = o.transport ?? 'auto';
		if (wanted === 'shared' && !sharedAvailable()) throw new Error('live-core: no SharedArrayBuffer – the page needs COOP / COEP (cross-origin isolated)');
		const transport: Transport = wanted === 'auto' ? (sharedAvailable() ? 'shared' : 'transfer') : wanted;
		// the device's channels (plan 2.6): the engine renders them all, the worklet has as many outputs, the ring carries
		// them interleaved; the destination takes them one to one (no up / down-mix by the browser)
		const destination = context.destination;
		const channels = Math.min(Math.max(1, Math.floor(o.channels ?? deviceChannels(destination))), MAX_OUTPUTS);
		if (!o.output && channels !== destination.channelCount) {
			destination.channelCount = channels;
			destination.channelInterpretation = 'discrete';
		}
		const capacity = pow2At(AUDIO_FRAMES * channels);
		const shared: SharedSetup | null =
			transport === 'shared'
				? {
						control: ringBuffers(CONTROL_BYTES, 1, true),
						audio: ringBuffers(capacity, 4, true),
						analysis: Latest.buffers(FRAME + MAX_FRAME_VALUES, true),
						status: new SharedArrayBuffer(16)
					}
				: null;
		await context.audioWorklet.addModule(String(o.worklet));
		const options: WorkletOptions = { audio: shared?.audio ?? null, capacity, channels, status: shared?.status ?? null };
		const node = new AudioWorkletNode(context, 'live-core', { numberOfInputs: 0, outputChannelCount: [channels], processorOptions: options });
		node.connect(o.output ?? destination);
		const core = new LiveCore({ context, node, worker: o.worker, transport, shared, channels });
		let audioPort: MessagePort | null = null;
		if (!shared) {
			const ch = new MessageChannel();
			node.port.postMessage({ feed: ch.port2 }, [ch.port2]);
			audioPort = ch.port1;
		}
		node.port.onmessage = (ev: MessageEvent<FromWorklet>) => core.underran(ev.data.underruns);
		// #321: when the context's samples are heard (the clock's time base) – getOutputTimestamp, ten times a second
		core.stampTimer = setInterval(() => {
			const ts = context.getOutputTimestamp?.();
			if (ts?.contextTime && ts.performanceTime) core.clockOp({ op: 'stamp', context: ts.contextTime, heard: performance.timeOrigin + ts.performanceTime });
		}, 100);
		const ready = new Promise<void>((resolve, reject) => {
			o.worker.onmessage = (ev: MessageEvent<FromWorker>) => (ev.data.type === 'ready' ? resolve() : core.receive(ev.data));
			o.worker.onerror = (e) => reject(new Error(`live-core worker: ${e.message}`));
		});
		const wasm = new URL(String(o.wasm), globalThis.location?.href).href;
		core.post({ type: 'init', wasm, rate: context.sampleRate, channels, device: DEVICE_CODE[o.device ?? 'desktop'], shared, audioPort }, audioPort ? [audioPort] : []);
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
					this.at.clear();
					this.analysis = null;
					for (const note of m.table.notes) console.info('live-core:', note);
					p?.ok(m.table);
				} else p?.fail(new Error(m.error));
				break;
			}
			case 'analysis':
				this.frame = m.frame;
				this.emit();
				break;
			case 'signals':
				for (const s of readSignals(new Uint8Array(m.bytes))) for (const cb of this.signalListeners) cb(s);
				break;
			case 'stats':
				this.monitor?.audio(performance.now(), m.words);
				break;
			case 'clock': {
				const c = clockOf(m.state);
				c.line = this.clockLine;
				this.clock = c;
				this.clockWindow.add(performance.timeOrigin + performance.now(), c);
				for (const cb of this.clockListeners) cb(c);
				break;
			}
			case 'clockLine':
				this.clockLine = m.line;
				if (this.clock) this.clock.line = m.line;
				break;
			case 'midiOut':
				for (const cb of this.midiOutListeners) cb(m.msgs);
				break;
			case 'error':
				console.error('live-core:', m.error);
				break;
		}
	}

	/** A scene in the deck's TOML (the modules the browser carries: deck-core `BROWSER_MODULES`) → its node table. Its `tempo`
	 * node sets the clock; a scene without one plays on the clock as it is (the show clock's, #321). */
	load(scene: string): Promise<Table> {
		const id = this.nextId++;
		return new Promise((ok, fail) => {
			this.pending.set(id, { ok, fail });
			this.post({ type: 'load', id, scene });
		});
	}

	/** #321: something for the core's tempo clock (a pong, an anchor, a MIDI message, the picker, a key) */
	clockOp(op: ClockOp): void {
		this.post({ type: 'clock', op });
	}

	/** #321: every clock frame's state (≈ 40 a second). Returns the unsubscribe. */
	onClock(cb: (c: ClockState) => void): () => void {
		this.clockListeners.add(cb);
		return () => this.clockListeners.delete(cb);
	}

	/** #321: the MIDI clock out's messages as the core hands them out ([page ms, status, d1, d2] each, NaN = no such
	 *  byte) – for Web MIDI, sent with that timestamp. Returns the unsubscribe. */
	onMidiOut(cb: (msgs: Float64Array) => void): () => void {
		this.midiOutListeners.add(cb);
		return () => this.midiOutListeners.delete(cb);
	}

	/** The heard block's value of `path` (`node.input`, `node.output`, `node` = its first output; an audio output is its
	 * block's peak) from the newest analysis frame; NaN before one came or for a path the scene lacks. */
	value(path: string): number {
		const a = this.analysis;
		if (!a || !this.table) return NaN;
		let i = this.at.get(path);
		if (i === undefined) this.at.set(path, (i = valueIndex(this.table, path)));
		return i >= 0 && i < a.values.length ? a.values[i] : NaN;
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

	/**
	 * #282: every device message the scene's `[outputs]` resolves to (MIDI, DMX, OSC – shown, not played: the browser has
	 * no port), a few a second at most, in engine time order. Returns the unsubscribe.
	 */
	onSignal(cb: (s: DeviceSignal) => void): () => void {
		this.signalListeners.add(cb);
		return () => this.signalListeners.delete(cb);
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
		this.analysis = a;
		for (const cb of this.listeners) cb(a);
	}

	stats(): Stats {
		return { ...this.stats_ };
	}

	/** #364: the core's audio window once a second to the page's monitor (null = to none) */
	audioTo(monitor: Pick<MonitorView, 'audio'> | null): void {
		this.monitor = monitor;
	}

	/** the worklet's running dropout count */
	underran(n: number): void {
		this.stats_.underruns = n;
	}

	async stop(): Promise<void> {
		if (this.raf) cancelAnimationFrame(this.raf);
		this.raf = 0;
		if (this.stampTimer) clearInterval(this.stampTimer);
		this.post({ type: 'stop' });
		this.node.disconnect();
		this.worker.terminate();
		await this.context.close();
	}
}
