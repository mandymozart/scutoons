// The map builder's region brush (agents/MAP.md §8.10, owner 2026-10-12: "the brush can be scaled from 1 to 256 so it can
// go faster to paint regions"): which region's cover is being painted, add or remove, the brush's size. The region form
// and the phone bar switch it; the builder hands the canvas a bulk HexBrushTarget for it (paintRuns on the row runs).
import { BRUSH_MAX } from '@scutoons/content';
import { shared } from '$lib/utils/shared.ts';

class RegionBrush {
	/** the region whose cover is painted (null: no brush) */
	region = $state<string | null>(null);
	mode = $state<'add' | 'remove'>('add');
	/** a region is big: start at a quarter of the biggest brush */
	size = $state(Math.round(BRUSH_MAX / 4));

	toggle(region: string) {
		this.region = this.region === region ? null : region;
	}
	stop() {
		this.region = null;
	}
}

/** the builder's one region brush (shared(): one instance even when the module is loaded twice) */
export const regionBrush = shared('region-brush', () => new RegionBrush());
