<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { UnitConversions } from '@scutoons/ui';
	import { loadUnits } from '@scutoons/live-deck/units';
	import { provideCoreUnits } from '$lib/editor/core-units-context.ts';
	import { toast } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * Provides deck-core's units (through the deck's wasm) to every field below; the wasm loads when a field first asks –
	 * once: a failed load is said in the Log and not tried again (every field reads `core` on each draw).
	 */
	let { children }: { children: Snippet } = $props();

	let loaded = $state<UnitConversions | null>(null);
	let started = false;
	provideCoreUnits({
		get core() {
			if (!started) {
				started = true;
				void loadUnits('/live/').then(
					(c) => (loaded = c),
					(e) => toast(m['shell.log_core_failed']({ reason: e instanceof Error ? e.message : String(e) }), { type: 'error' })
				);
			}
			return loaded;
		}
	});
</script>

{@render children()}
