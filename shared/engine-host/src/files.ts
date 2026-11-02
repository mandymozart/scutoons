// files.ts (#298): live-core's worker and worklet as the app's bundler resolves them (`?worker`), and the wasm next to
// the 3D's. The one place that needs the bundler – the controller takes this as a function, so its tests need none.
import LiveWorker from '@scutoons/live-core/worker?worker';
import workletUrl from '@scutoons/live-core/worklet?worker&url';
import type { EngineFiles } from '@scutoons/live-deck';

/** a fresh worker per start (a stopped core terminates its worker); `wasmBase` ends with `/` */
export function engineFiles(wasmBase: string): EngineFiles {
	return { worker: new LiveWorker(), wasm: new URL(`${wasmBase}live_core.wasm`, location.href), worklet: workletUrl };
}
