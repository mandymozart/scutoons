// The story's `migrate` (agents/MAP.md §8.10, owner 2026-10-12): the reader's avatar or a character moves to a region of
// the world map. The mounted map (toon/map/map.svelte – it has the map's regions) registers it in the save game and plays
// the welcome: the region's sign, its representative saying the greeting – or the narrator. The story waits for it.
// A region that is not on the map does nothing (the creator warns). Moves that come while no map is mounted are handed
// over once one is – registered, without the welcome.

export interface MigrateRequest {
	/** a character id, or `READER_RESIDENT` for the reader's own avatar */
	who: string;
	region: string;
	/** the registration only: no welcome */
	silent: boolean;
	done: () => void;
}

type Handler = (request: MigrateRequest) => void;

let handler: Handler | null = null;
const pending: MigrateRequest[] = [];

/** the map registers while it is mounted; returns the unregister */
export function onMigrate(next: Handler): () => void {
	handler = next;
	for (const request of pending.splice(0)) next(request);
	return () => {
		if (handler === next) handler = null;
	};
}

/** The story's `migrate`: resolves when the reader can go on (the welcome closed) */
export function migrateStory(who: string, region: string, options: { silent?: boolean } = {}): Promise<void> {
	if (!region) return Promise.resolve();
	if (!handler) {
		pending.push({ who, region, silent: true, done: () => {} });
		return Promise.resolve();
	}
	const run = handler;
	return new Promise((resolve) => run({ who, region, silent: !!options.silent, done: resolve }));
}
