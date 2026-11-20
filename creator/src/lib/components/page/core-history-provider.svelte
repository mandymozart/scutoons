<script lang="ts">
	import { loadHistory } from '@scutoons/live-deck/history';
	import { provideHistoryCore } from '$lib/editor/core-history.ts';

	/** Hands deck-core's history rules (through the deck's wasm) to the History; the wasm loads when the History first asks. */
	provideHistoryCore(() => loadHistory('/live/'));
</script>
