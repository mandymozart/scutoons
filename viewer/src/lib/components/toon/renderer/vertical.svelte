<script lang="ts">
	import type { Group } from '$lib/models/content.ts';
	import Slice from '../slice.svelte';

	let { group }: { group: Group } = $props();
</script>

<div class="renderer renderer--vertical" data-group-label={group.label}>
	{#each group.slices as slice, index}
		<Slice {slice} sliceIndex={index} allSlices={group.slices} />
	{:else}
		Slices missing
	{/each}
</div>

<style>
	.renderer {
		overflow-y: auto;
	}
</style>
