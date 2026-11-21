// The core's history rules for the creator's History (#687): one holder, filled once by the root layout's
// CoreHistoryProvider (the only place that loads them through the wasm) and asked by lib/editor/history.svelte.ts.
import type { HistoryView } from '@scutoons/content';

let load: (() => Promise<HistoryView>) | null = null;
let core: Promise<HistoryView> | null = null;

export function provideHistoryCore(loader: () => Promise<HistoryView>): void {
	load = loader;
}

/** the core's history rules (loaded on first use, once per page) */
export function historyCore(): Promise<HistoryView> {
	if (!load) return Promise.reject(new Error('the core is not loaded'));
	core ??= load().catch((e) => {
		core = null;
		throw e;
	});
	return core;
}
