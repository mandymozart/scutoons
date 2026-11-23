<script lang="ts">
	import { loadIdleProblems, loadModelFacts } from '@scutoons/live-deck/model';
	import { provideIdleCore, provideModelCore } from '$lib/editor/core-model.ts';

	/** Hands deck-space's reading of a model file and its idle rule (through the deck's wasm) to the pages; the wasm loads when it first asks. */
	provideModelCore(() => loadModelFacts('/live/'));
	provideIdleCore(() => loadIdleProblems('/live/'));
</script>
