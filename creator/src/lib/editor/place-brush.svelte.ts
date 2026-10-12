// The map builder's ground brush for a place (owner 2026-10-12: "a brush that adds or removes hexagons to the place's
// base"): which place is being painted, add or remove, the brush's size. The place form (place-props.svelte) and the
// phone bar switch it; the builder hands the canvas a HexBrushTarget for it (map-builder.svelte → hex-brush.svelte).
import { BRUSH_MIN } from '@scutoons/content';

class PlaceBrush {
	/** the place whose ground is painted (null: no brush) */
	place = $state<string | null>(null);
	mode = $state<'add' | 'remove'>('add');
	size = $state(BRUSH_MIN);

	toggle(place: string) {
		this.place = this.place === place ? null : place;
	}
	stop() {
		this.place = null;
	}
}

export const placeBrush = new PlaceBrush();
