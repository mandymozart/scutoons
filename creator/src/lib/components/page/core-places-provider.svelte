<script lang="ts">
	import { loadPlaces } from '@scutoons/live-deck/places';
	import { providePlacesCore } from '$lib/editor/core-places.ts';

	/** Hands deck-core's jump places (through the deck's wasm) to the jump targets; the wasm loads when they first ask. */
	providePlacesCore(() => loadPlaces('/live/'));
</script>
