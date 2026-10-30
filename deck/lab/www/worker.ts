// The Web Worker of the #182 lab page: deck-core as wasm next to the TypeScript versions, off the main thread – where
// the browser Live mode would run its control graph, analysis and codec. Messages: `run` → the suite's rows; `echo`
// → the same payload back (the main thread measures the round trip, copied or transferred).
import { runTs, runWasm, type CoreWasm } from './suite.js';
// @ts-expect-error – wasm-bindgen's generated modules, copied next to this file by ts/build.ts
import init, * as core from './pkg/core_wasm.js';
// @ts-expect-error – the same crate built with simd128
import initSimd, * as coreSimd from './pkg-simd/core_wasm.js';

type In = { type: 'run' } | { type: 'echo'; buf: Float32Array; transfer: boolean };

self.onmessage = async (ev: MessageEvent<In>) => {
	const m = ev.data;
	if (m.type === 'echo') {
		(self as unknown as Worker).postMessage({ type: 'echo', buf: m.buf }, m.transfer ? [m.buf.buffer] : []);
		return;
	}
	const t = performance.now();
	await init();
	const initMs = performance.now() - t;
	await initSimd();
	const rows = [
		{ path: 'wasm (Chromium worker)', workload: 'fetch + compile + instantiate', us: initMs * 1000 },
		...runTs('TypeScript (Chromium worker)'),
		...runWasm('wasm (Chromium worker)', core as unknown as CoreWasm),
		...runWasm('wasm simd128 (Chromium worker)', coreSimd as unknown as CoreWasm)
	];
	(self as unknown as Worker).postMessage({ type: 'rows', rows });
};
