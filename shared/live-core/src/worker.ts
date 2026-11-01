// The Web Worker that hosts live_core.wasm (#184). The app builds it with its bundler and hands it to `LiveCore.start`
// (vite: `new Worker(new URL('@scutoons/live-core/worker', import.meta.url), { type: 'module' })`).
//
// Shared transport: the pump wakes when the worklet has read (Atomics.waitAsync on the audio ring's read counter;
// a 2 ms timer where waitAsync is missing), drains the control ring into the wasm inbox, renders until `lead` frames
// are queued, publishes analysis frames into the `Latest` slot. Transfer transport: the worklet reports what it
// played over a MessagePort, the worker answers with transferred chunks; records and analysis frames are messages.
import { Engine, WasmCore, BLOCK } from './engine.ts';
import { Latest, Ring } from './ring.ts';
import { AdaptiveLead } from './lead.ts';
import type { AudioFeed, FromWorker, ToWorker } from './messages.ts';

/** #208: the adaptive lead's cap and step (frames) – 6144 ≈ 128 ms leaves room in the 8192-frame audio ring */
const LEAD_CAP = 6144;
const LEAD_STEP = 512;

const scope = self as unknown as {
	onmessage: ((ev: MessageEvent<ToWorker>) => void) | null;
	postMessage(m: FromWorker, transfer?: Transferable[]): void;
};
const post = (m: FromWorker, transfer: Transferable[] = []) => scope.postMessage(m, transfer);

let engine: Engine | null = null;
let control: Ring<Uint8Array> | null = null;
let audio: Ring<Float32Array> | null = null;
let feed: MessagePort | null = null;
/** transfer transport: frames sent to the worklet and not yet played */
let inFlight = 0;
let stopped = false;
let ticking = false;
let statsTimer: ReturnType<typeof setInterval> | null = null;
/** #208: the lead grows on the worklet's dropouts (shared: the status word; transfer: its feed messages) */
let adaptive: AdaptiveLead | null = null;
let status: Int32Array | null = null;
let dropouts = 0;
/** the last pump and the longest pause between two in this second (ms) – a stalled worker shows here */
let lastTick = 0;
let gapMs = 0;

function adapt(): void {
	if (!engine || !adaptive) return;
	if (status) dropouts = Atomics.load(status, 0);
	engine.lead = adaptive.update(performance.now(), dropouts);
}

type WaitAsync = (a: Int32Array, i: number, v: number, t?: number) => { async: boolean; value: Promise<string> | string };
const waitAsync = (Atomics as unknown as { waitAsync?: WaitAsync }).waitAsync;

function tick(): void {
	if (stopped || !engine || !audio || !control) return;
	const now = performance.now();
	if (lastTick) gapMs = Math.max(gapMs, now - lastTick);
	lastTick = now;
	adapt();
	try {
		engine.drain(control);
		engine.fill(audio);
	} catch (e) {
		post({ type: 'error', error: String((e as Error).message ?? e) });
	}
	const seen = Atomics.load(audio.state, 1);
	if (waitAsync) {
		const w = waitAsync(audio.state, 1, seen, 20);
		if (w.async) (w.value as Promise<string>).then(tick);
		else setTimeout(tick, 0);
	} else setTimeout(tick, 2);
}

/** Transfer transport: tops the worklet up to `lead` frames, in chunks of 4 blocks. */
function topUp(): void {
	if (!engine || !feed || !engine.core.loaded) return;
	while (inFlight + 4 * BLOCK <= engine.lead) {
		const chunk = new Float32Array(4 * BLOCK);
		for (let b = 0; b < 4; b++) engine.block(inFlight + b * BLOCK, (a) => chunk.set(a, b * BLOCK));
		inFlight += chunk.length;
		feed.postMessage({ chunk } satisfies AudioFeed, [chunk.buffer]);
	}
}

scope.onmessage = async (ev) => {
	const m = ev.data;
	try {
		switch (m.type) {
			case 'init': {
				const module = await WebAssembly.compileStreaming(fetch(m.wasm));
				const core = await WasmCore.instantiate(module, m.rate);
				if (m.shared) {
					const latest = new Latest(m.shared.analysis);
					control = Ring.bytes(m.shared.control);
					audio = Ring.floats(m.shared.audio);
					engine = new Engine(core, { lead: m.lead, onAnalysis: (f) => latest.publish(f) });
				} else {
					feed = m.audioPort;
					engine = new Engine(core, { lead: m.lead, onAnalysis: (f) => {
							const frame = f.slice();
							post({ type: 'analysis', frame }, [frame.buffer]);
						}
					});
					if (feed) {
						feed.onmessage = (e: MessageEvent<AudioFeed>) => {
							if ('consumed' in e.data) {
								inFlight = Math.max(0, inFlight - e.data.consumed);
								dropouts = e.data.underruns;
							}
							const now = performance.now();
							if (lastTick) gapMs = Math.max(gapMs, now - lastTick);
							lastTick = now;
							adapt();
							topUp();
						};
					}
				}
				status = m.shared ? new Int32Array(m.shared.status) : null;
				adaptive = new AdaptiveLead(m.lead, Math.max(LEAD_CAP, m.lead), LEAD_STEP);
				statsTimer = setInterval(() => {
					if (!engine) return;
					post({ type: 'stats', stats: { ...engine.stats(), lead: engine.lead, gapMs } });
					gapMs = 0;
				}, 1000);
				post({ type: 'ready' });
				break;
			}
			case 'load': {
				if (!engine) throw new Error('load before init');
				const table = engine.core.load(m.scene);
				post({ type: 'loaded', id: m.id, table });
				if (audio && !ticking) {
					ticking = true;
					tick();
				}
				topUp();
				break;
			}
			case 'records':
				engine?.applyBytes(new Uint8Array(m.bytes));
				break;
			case 'stop':
				stopped = true;
				if (statsTimer) clearInterval(statsTimer);
				feed?.close();
				break;
		}
	} catch (e) {
		const error = String((e as Error).message ?? e);
		if (m.type === 'load') post({ type: 'failed', id: m.id, error });
		else post({ type: 'error', error });
	}
};
