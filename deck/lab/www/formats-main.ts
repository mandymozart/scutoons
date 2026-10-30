// formats.html (#184): how a value gets from the page into the worker (postMessage with an object, postMessage with
// transferred record bytes, the SharedArrayBuffer ring) and – in the worker – across into wasm in each candidate
// format (ts/formats.ts). Medians; `window.__results` holds everything for a script (ts/run-browser.ts reads it).
import { encodeSend } from '../../../shared/live-core/src/wire.ts';
import { Ring, ringBuffers } from '../../../shared/live-core/src/ring.ts';
import { summarise, table, type FormatRow } from '../ts/formats.ts';

type Crossing = { probe: string; us: number; runs: number[] };

const out = document.querySelector('#out') as HTMLElement;
const say = (s: string) => (out.textContent += s + '\n');
const worker = new Worker(new URL('./formats-worker.js', import.meta.url), { type: 'module' });
const next = <T>(type: string): Promise<T> =>
	new Promise((resolve) => {
		const on = (ev: MessageEvent) => {
			if (ev.data.type !== type) return;
			worker.removeEventListener('message', on);
			resolve(ev.data as T);
		};
		worker.addEventListener('message', on);
	});
const median = (xs: number[]) => [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)];
const RECORD = { mode: 'force' as const, node: 300, port: 3, amount: 1, v: [0.05] };
type WaitAsync = (a: Int32Array, i: number, v: number, t?: number) => { async: boolean; value: Promise<string> | string };
const waitAsync = (Atomics as unknown as { waitAsync: WaitAsync }).waitAsync;

/** one-way: µs per message, N sent back to back, until the worker has counted them all */
async function oneWay(n: number, send: () => void): Promise<number> {
	worker.postMessage({ type: 'count', n });
	const done = next('counted');
	const t = performance.now();
	for (let i = 0; i < n; i++) send();
	await done;
	return ((performance.now() - t) * 1000) / n;
}

async function ringOneWay(n: number): Promise<number> {
	const a = ringBuffers(1 << 16, 1, true);
	const b = ringBuffers(1 << 16, 1, true);
	const ring = Ring.bytes(a);
	worker.postMessage({ type: 'ring', a, b, mode: 'drain', n });
	await new Promise((r) => setTimeout(r, 50)); // the worker is waiting on the ring
	const rec = encodeSend(RECORD);
	const done = next('ring-done');
	const t = performance.now();
	for (let i = 0; i < n; i++) {
		ring.writeAll(rec);
		Atomics.notify(ring.state, 0);
	}
	await done;
	return ((performance.now() - t) * 1000) / n;
}

async function postRoundTrip(n: number): Promise<number> {
	const times: number[] = [];
	for (let i = 0; i < n; i++) {
		const pong = next('pong');
		const t = performance.now();
		worker.postMessage({ type: 'ping', t });
		await pong;
		times.push((performance.now() - t) * 1000);
	}
	return median(times);
}

async function ringRoundTrip(n: number): Promise<number> {
	const a = ringBuffers(1 << 16, 1, true);
	const b = ringBuffers(1 << 16, 1, true);
	const ra = Ring.bytes(a);
	const rb = Ring.bytes(b);
	worker.postMessage({ type: 'ring', a, b, mode: 'pingpong', n });
	await new Promise((r) => setTimeout(r, 50));
	const rec = encodeSend(RECORD);
	const back = new Uint8Array(64);
	const times: number[] = [];
	for (let i = 0; i < n; i++) {
		const w = Atomics.load(rb.state, 0);
		const t = performance.now();
		ra.writeAll(rec);
		Atomics.notify(ra.state, 0);
		const wait = waitAsync(rb.state, 0, w, 1000);
		if (wait.async) await wait.value;
		rb.read(back);
		times.push((performance.now() - t) * 1000);
	}
	await next('ring-done');
	return median(times);
}

async function main() {
	say(`crossOriginIsolated: ${self.crossOriginIsolated} · ${navigator.userAgent.match(/Chrome\/[\d.]+/)?.[0] ?? navigator.userAgent}`);
	const REPEAT = 5;
	const probes: [string, () => Promise<number>][] = [
		['postMessage, an object { mode, node, port, amount, v } (one-way, 4000 back to back)', () => oneWay(4000, () => worker.postMessage({ type: 'msg', ...RECORD }))],
		[
			'postMessage, 16 record bytes transferred (one-way, 4000 back to back)',
			() =>
				oneWay(4000, () => {
					const b = encodeSend(RECORD).buffer as ArrayBuffer;
					worker.postMessage({ type: 'bytes', b }, [b]);
				})
		],
		['SharedArrayBuffer ring, a 16-byte record (one-way, 4000 back to back)', () => ringOneWay(4000)],
		['postMessage round trip (median of 300)', () => postRoundTrip(300)],
		['SharedArrayBuffer ring round trip, Atomics.waitAsync both ways (median of 300)', () => ringRoundTrip(300)]
	];
	const crossings: Crossing[] = [];
	for (const [probe, run] of probes) {
		const runs: number[] = [];
		for (let i = 0; i < REPEAT; i++) runs.push(await run());
		crossings.push({ probe, us: median(runs), runs });
		say(`${probe}: ${median(runs).toFixed(2)} µs (${runs.map((x) => x.toFixed(2)).join(' / ')})`);
	}
	say('the formats in the worker (5 runs) …');
	worker.addEventListener('message', (ev) => ev.data.type === 'progress' && say(`  run ${ev.data.run}`));
	const done = next<{ runs: FormatRow[][] }>('formats');
	worker.postMessage({ type: 'formats', runs: REPEAT });
	const { runs } = await done;
	const rows = summarise(runs);
	say(table(rows));
	(window as unknown as { __results: unknown }).__results = { isolated: self.crossOriginIsolated, ua: navigator.userAgent, crossings, rows };
	document.title = 'done';
}

main().catch((e) => {
	say(`failed: ${e}`);
	document.title = 'failed';
});
