// The unlock reveal (agents/MAP.md §8.5a, owner 2026-10-11): the story's `unlockLocation` unlocks a place in the save
// game; the map (toon/map/map.svelte, mounted on the toon pages) shows a toast "<place> unlocked" and – when the run may
// open the map (a VN script, a click trigger, an item tap; never an intersect / hover trigger) – opens, flies to the
// place, shows it and closes again. The story waits for that. One reveal after another.
import { getCurrentWorld } from '$lib/contexts/world.svelte.ts';

export interface RevealRequest {
	id: string;
	/** open the map and fly there (else the toast only) */
	reveal: boolean;
	done: () => void;
}

/** what the mounted map does with a newly unlocked place (it knows the names, images and the camera) */
type Handler = (request: RevealRequest) => void;

let handler: Handler | null = null;

/** the map registers while it is mounted; returns the unregister */
export function onPlaceUnlocked(next: Handler): () => void {
	handler = next;
	return () => {
		if (handler === next) handler = null;
	};
}

/**
 * The story's `unlockLocation`: unlocks the place (a set operation – again = nothing shown); not `silent` and new: the
 * toast, and with `reveal` the map's flight. Resolves when the reader can go on (the map closed again).
 */
export function unlockStoryPlace(id: string, options: { silent?: boolean; reveal?: boolean } = {}): Promise<void> {
	const world = getCurrentWorld();
	if (!world || !id) return Promise.resolve();
	const added = world.unlock([id]);
	if (!added.length || options.silent || !handler) return Promise.resolve();
	const show = handler;
	return new Promise((resolve) => show({ id, reveal: !!options.reveal, done: resolve }));
}
