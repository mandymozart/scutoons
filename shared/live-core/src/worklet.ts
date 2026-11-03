// The AudioWorklet processor `live-core` (#184): it renders nothing itself – it plays what the worker rendered with
// the wasm, read from the audio ring (shared transport: a SharedArrayBuffer, Atomics.notify after each read wakes the
// worker; transfer transport: a local ring fed with chunks over a MessagePort, the played count reported back).
// The ring holds the device channels interleaved (#320: the engine renders them all – a channel per output channel of
// this node, `outputChannelCount`); it only de-interleaves. An empty ring plays silence and counts an underrun.
// The app loads it with `context.audioWorklet.addModule(url)` – a module the bundler emits (vite: `?worker&url`).
import { Ring, ringBuffers } from './ring.ts';
import type { AudioFeed, FromWorklet, WorkletOptions } from './messages.ts';

declare const AudioWorkletProcessor: { new (): { readonly port: MessagePort } };
/** the context frame at the start of this quantum (AudioWorkletGlobalScope) */
declare const currentFrame: number;
declare function registerProcessor(name: string, ctor: unknown): void;

const REPORT = 512;
/** the render quantum the scratch is made for (a different one makes a new scratch once) */
const QUANTUM = 128;

class LiveCoreProcessor extends AudioWorkletProcessor {
	private readonly ring: Ring<Float32Array>;
	private readonly shared: boolean;
	private readonly channels: number;
	/** one quantum of interleaved frames, read from the ring before it is spread over the outputs */
	private scratch: Float32Array;
	private feed: MessagePort | null = null;
	/** frames played since the last report to the worker (transfer transport) */
	private consumed = 0;
	private started = false;
	private underruns = 0;
	/** #208: [dropouts] shared with the worker (its adaptive lead); null in the transfer transport */
	private readonly status: Int32Array | null;
	/** the count last posted to the page */
	private reported = 0;
	private played = 0;
	private sinceReport = 0;
	/** #321: the context frame minus the frames played before it – where the engine's frames are heard (the clock's
	 * time base: `getOutputTimestamp` says when a context frame is heard) */
	private skew = 0;

	constructor(options: { processorOptions: WorkletOptions }) {
		super();
		const o = options.processorOptions;
		this.shared = o.audio !== null;
		this.channels = Math.max(1, o.channels);
		this.scratch = new Float32Array(QUANTUM * this.channels);
		this.ring = Ring.floats(o.audio ?? ringBuffers(o.capacity, 4, false));
		this.status = o.status ? new Int32Array(o.status) : null;
		this.port.onmessage = (ev: MessageEvent<{ feed?: MessagePort }>) => {
			if (!ev.data.feed) return;
			this.feed = ev.data.feed;
			this.feed.onmessage = (e: MessageEvent<AudioFeed>) => {
				if ('chunk' in e.data) this.ring.write(e.data.chunk);
			};
		};
	}

	process(_inputs: Float32Array[][], outputs: Float32Array[][]): boolean {
		const out = outputs[0];
		const frames = out[0].length;
		const ch = this.channels;
		if (this.scratch.length !== frames * ch) this.scratch = new Float32Array(frames * ch);
		const n = this.ring.read(this.scratch, this.shared) / ch;
		if (n > 0) {
			this.started = true;
			this.skew = currentFrame - this.played;
			if (this.status) Atomics.store(this.status, 1, this.skew);
		}
		for (let c = 0; c < out.length; c++) {
			const o = out[c];
			if (c < ch) for (let f = 0; f < n; f++) o[f] = this.scratch[f * ch + c];
			o.fill(0, c < ch ? n : 0);
		}
		if (n < frames && this.started) {
			this.underruns++;
			if (this.status) Atomics.store(this.status, 0, this.underruns);
		}
		this.played += n;
		if (!this.shared && n > 0) {
			this.consumed += n;
			if (this.consumed >= REPORT && this.feed) {
				this.feed.postMessage({ consumed: this.consumed, underruns: this.underruns, skew: this.skew } satisfies AudioFeed);
				this.consumed = 0;
			}
		}
		this.sinceReport += frames;
		// once a second, and (#208) a new dropout at once (at most every 100 ms at 48 kHz) – the page's warning flashes
		if (this.sinceReport >= 48_000 || (this.underruns !== this.reported && this.sinceReport >= 4_800)) {
			this.reported = this.underruns;
			this.sinceReport = 0;
			this.port.postMessage({ underruns: this.underruns, played: this.played } satisfies FromWorklet);
		}
		return true;
	}
}

registerProcessor('live-core', LiveCoreProcessor);
