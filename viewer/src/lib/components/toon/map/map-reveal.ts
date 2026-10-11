// The unlock reveal (agents/MAP.md §8.5a, owner 2026-10-11): the story's `unlockLocation` unlocks a place in the save
// game; the map (toon/map/map.svelte, mounted on the toon pages) shows a toast "<place> unlocked" and – when the run may
// open the map (a VN script, a click trigger, an item tap; never an intersect / hover trigger) – opens, flies to the
// place, shows it and closes again. The story waits for that. One reveal after another.
// A place that is not on the map (deleted, mistyped) does nothing at all – the event fizzles; the creator warns about it
// (owner 2026-10-11: "der Event verpufft – mit Warnung, dass was faul ist, nur im Creator").

export interface RevealRequest {
	id: string;
	/** the state only: no toast, no map */
	silent: boolean;
	/** open the map and fly there (else the toast only) */
	reveal: boolean;
	done: () => void;
}

/** the mounted map checks the place, unlocks it and shows it (it knows the places, names, images and the camera) */
type Handler = (request: RevealRequest) => void;

let handler: Handler | null = null;
/** unlocks that came while no map was mounted: handed over (without the map's flight) once one is */
const pending: RevealRequest[] = [];

/** the map registers while it is mounted; returns the unregister */
export function onPlaceUnlocked(next: Handler): () => void {
	handler = next;
	for (const request of pending.splice(0)) next(request);
	return () => {
		if (handler === next) handler = null;
	};
}

/**
 * The story's `unlockLocation`: the map unlocks the place when it exists (a set operation – again = nothing shown); not
 * `silent` and new: the toast, and with `reveal` the map's flight. Resolves when the reader can go on (the map closed).
 */
export function unlockStoryPlace(id: string, options: { silent?: boolean; reveal?: boolean } = {}): Promise<void> {
	if (!id) return Promise.resolve();
	if (!handler) {
		pending.push({ id, silent: !!options.silent, reveal: false, done: () => {} });
		return Promise.resolve();
	}
	const show = handler;
	return new Promise((resolve) => show({ id, silent: !!options.silent, reveal: !!options.reveal, done: resolve }));
}
