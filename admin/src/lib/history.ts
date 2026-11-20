// The History of an admin-edited document (#687): the shared state over the content store. deck-core's history rules are
// handed in once by the layout's CoreHistoryProvider (the only place that loads them through the wasm served at /live/).
import { createHistory } from '@scutoons/api';
import type { HistoryView } from '@scutoons/content';
import { content } from '$lib/api.ts';

let load: (() => Promise<HistoryView>) | null = null;
let core: Promise<HistoryView> | null = null;

export function provideHistoryCore(loader: () => Promise<HistoryView>): void {
	load = loader;
}

export const history = createHistory(content, () => {
	if (!load) return Promise.reject(new Error('the core is not loaded'));
	core ??= load().catch((e) => {
		core = null;
		throw e;
	});
	return core;
});
