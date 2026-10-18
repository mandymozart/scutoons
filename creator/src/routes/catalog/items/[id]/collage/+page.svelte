<script lang="ts">
	import { page } from '$app/state';
	import ScoreStage from '$lib/components/collage/score-stage.svelte';

	// a score item's collage stage (research/COLLAGE.md §4.4) – the catalog editor's "Collage" on a score item
	const id = $derived(page.params.id ?? '');
</script>

{#key id}
	<ScoreStage {id} />
{/key}
