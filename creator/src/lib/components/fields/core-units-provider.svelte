<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { UnitConversions } from '@scutoons/ui';
	import { loadUnits } from '@scutoons/live-deck/units';
	import { provideCoreUnits } from '$lib/editor/core-units-context.ts';

	/** Provides deck-core's units (through the deck's wasm) to every field below; the wasm loads when a field first asks. */
	let { children }: { children: Snippet } = $props();

	let loaded = $state<UnitConversions | null>(null);
	let started = false;
	provideCoreUnits({
		get core() {
			if (!started) {
				started = true;
				void loadUnits('/live/').then((c) => (loaded = c), () => (started = false));
			}
			return loaded;
		}
	});
</script>

{@render children()}
