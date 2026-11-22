<script lang="ts">
	import { loadModelFacts } from '@scutoons/live-deck/model';
	import { provideModelCore } from '$lib/editor/core-model.ts';

	/** Hands deck-space's reading of a model file (through the deck's wasm) to the 3D place; the wasm loads when it first asks. */
	provideModelCore(() => loadModelFacts('/live/'));
</script>
