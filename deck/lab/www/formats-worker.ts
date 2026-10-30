// The worker of formats.html (#184): runs the exchange-format suite (ts/formats.ts) where the Live engine would run,
// and answers the page's crossing probes – postMessage (object, transferred bytes) and the SharedArrayBuffer ring.
import { WasmCore } from '../../../shared/live-core/src/engine.ts';
import { Ring, type RingBuffers } from '../../../shared/live-core/src/ring.ts';
import { runFormats, type FormatRow, type FormatsWasm } from '../ts/formats.ts';

/** wasm-bindgen's generated module, copied next to the bundle by ts/build.ts (loaded by URL: the bundler leaves it) */
type Pkg = FormatsWasm & { default(o: { module_or_path: URL }): Promise<unknown> };
const pkgUrl = new URL('./pkg/core_wasm.js', import.meta.url).href;

type In =
	| { type: 'formats'; runs: number }
	| { type: 'count'; n: number }
	| { type: 'msg' }
	| { type: 'bytes'; b: ArrayBuffer }
	| { type: 'ping'; t: number }
	| { type: 'ring'; a: RingBuffers; b: RingBuffers; mode: 'drain' | 'pingpong'; n: number };

const scope = self as unknown as { onmessage: ((ev: MessageEvent<In>) => void) | null; postMessage(m: unknown): void };
type WaitAsync = (a: Int32Array, i: number, v: number, t?: number) => { async: boolean; value: Promise<string> | string };
const waitAsync = (Atomics as unknown as { waitAsync: WaitAsync }).waitAsync;
const waitChange = async (a: Int32Array, i: number, v: number) => {
	const w = waitAsync(a, i, v, 1000);
	if (w.async) await w.value;
};

let expected = 0;
let got = 0;

scope.onmessage = async (ev) => {
	const m = ev.data;
	switch (m.type) {
		case 'formats': {
			const pkg = (await import(pkgUrl)) as Pkg;
			await pkg.default({ module_or_path: new URL('./pkg/core_wasm_bg.wasm', import.meta.url) });
			const core = await WasmCore.instantiate(await (await fetch(new URL('./live_core.wasm', import.meta.url))).arrayBuffer(), 48_000);
			core.load(pkg.fmt_scene());
			const runs: FormatRow[][] = [];
			for (let i = 0; i < m.runs; i++) {
				runs.push(runFormats('Chromium worker', pkg, core));
				scope.postMessage({ type: 'progress', run: i + 1 });
			}
			scope.postMessage({ type: 'formats', runs });
			break;
		}
		case 'count':
			expected = m.n;
			got = 0;
			break;
		case 'msg':
		case 'bytes':
			if (++got === expected) scope.postMessage({ type: 'counted' });
			break;
		case 'ping':
			scope.postMessage({ type: 'pong', t: m.t });
			break;
		case 'ring': {
			const a = Ring.bytes(m.a);
			const b = Ring.bytes(m.b);
			const inbox = new Uint8Array(1 << 16);
			let records = 0;
			while (records < m.n) {
				const w = Atomics.load(a.state, 0);
				const n = a.read(inbox);
				if (n === 0) {
					await waitChange(a.state, 0, w);
					continue;
				}
				const k = n / 16; // the probe's records are 16 bytes (one scalar)
				records += k;
				if (m.mode === 'pingpong') {
					b.writeAll(inbox.subarray(0, n));
					Atomics.notify(b.state, 0);
				}
			}
			scope.postMessage({ type: 'ring-done', records });
			break;
		}
	}
};
