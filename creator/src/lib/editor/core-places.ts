// The core's jump places for the creator's jump targets: one holder, filled once by the root layout's
// CorePlacesProvider (the only place that loads them through the wasm) and asked by jump-places.svelte.ts.
export interface PlacesCore {
	/** a chapter's jump places from its `groups` */
	jumpLabels(groups: readonly unknown[]): string[];
}

let load: (() => Promise<PlacesCore>) | null = null;
let core: Promise<PlacesCore> | null = null;

export function providePlacesCore(loader: () => Promise<PlacesCore>): void {
	load = loader;
}

/** the core's jump places (loaded on first use, once per page) */
export function placesCore(): Promise<PlacesCore> {
	if (!load) return Promise.reject(new Error('the core is not loaded'));
	core ??= load().catch((e) => {
		core = null;
		throw e;
	});
	return core;
}
