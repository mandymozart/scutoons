<script lang="ts">
	import { loadLog } from '@scutoons/live-deck/log';
	import { log } from '$lib/log/log.svelte.ts';

	/** Hands deck-ui's Log (through the deck's wasm) to the creator's Log store; what was posted before it is there waits. */
	log.provide(() => loadLog('/live/'));
</script>
