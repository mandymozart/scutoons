// The AudioWorklet processor `live-core` (#184): it renders nothing itself – it plays what the worker rendered with
// the wasm, read from the audio ring (shared transport: a SharedArrayBuffer, Atomics.notify after each read wakes the
// worker; transfer transport: a local ring fed with chunks over a MessagePort, the played count reported back).
// Mono from the core, copied to every output channel. An empty ring plays silence and counts an underrun.
// The app loads it with `context.audioWorklet.addModule(url)` – a module the bundler emits (vite: `?worker&url`).
import { Ring, ringBuffers } from './ring.ts';
import type { AudioFeed, FromWorklet, WorkletOptions } from './messages.ts';

declare const AudioWorkletProcessor: { new (): { readonly port: MessagePort } };
declare function registerProcessor(name: string, ctor: unknown): void;

const REPORT = 512;

class LiveCoreProcessor extends AudioWorkletProcessor {
	private readonly ring: Ring<Float32Array>;
	private readonly shared: boolean;
	private feed: MessagePort | null = null;
	private consumed = 0;
	private started = false;
	private underruns = 0;
	/** #208: [dropouts] shared with the worker (its adaptive lead); null in the transfer transport */
	private readonly status: Int32Array | null;
	/** the count last posted to the page */
	private reported = 0;
	private played = 0;
	private sinceReport = 0;

	constructor(options: { processorOptions: WorkletOptions }) {
		super();
		const o = options.processorOptions;
		this.shared = o.audio !== null;
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
		const first = out[0];
		const n = this.ring.read(first, this.shared);
		if (n > 0) this.started = true;
		if (n < first.length) {
			first.fill(0, n);
			if (this.started) {
				this.underruns++;
				if (this.status) Atomics.store(this.status, 0, this.underruns);
			}
		}
		for (let c = 1; c < out.length; c++) out[c].set(first);
		this.played += n;
		if (!this.shared && n > 0) {
			this.consumed += n;
			if (this.consumed >= REPORT && this.feed) {
				this.feed.postMessage({ consumed: this.consumed, underruns: this.underruns } satisfies AudioFeed);
				this.consumed = 0;
			}
		}
		this.sinceReport += first.length;
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
