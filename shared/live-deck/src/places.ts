// places.ts: deck-core's jump places (a chapter's group labels) from the wasm the page loads (the same module the deck
// loads: once per page, as wasm, no GPU). No rule here.
import { spaceModule } from './space.ts';

export interface PlacesView {
	/** a chapter's jump places from its `groups` */
	jumpLabels(groups: readonly unknown[]): string[];
}

interface PlacesExports {
	jump_labels(groups: string): string[];
}

/** the core's jump places from the wasm at `base` */
export async function loadPlaces(base: string): Promise<PlacesView> {
	const mod = (await spaceModule(base)) as unknown as PlacesExports;
	return { jumpLabels: (groups) => mod.jump_labels(JSON.stringify(groups)) };
}
