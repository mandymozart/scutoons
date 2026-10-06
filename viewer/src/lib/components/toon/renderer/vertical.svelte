<script lang="ts">
	import type { Group } from '@scutoons/content';
	import Slice from '../slice.svelte';

	/** sliceOffset: the page index of the first slice (linked groups share one page) */
	let { group, sliceOffset = 0 }: { group: Group; sliceOffset?: number } = $props();
</script>

<div class="renderer renderer--vertical" data-group-label={group.label}>
	{#each group.slices as slice, index}
		<Slice {slice} sliceIndex={index} pageIndex={sliceOffset + index} allSlices={group.slices} />
	{:else}
		Slices missing
	{/each}
</div>

<style>
	/* a block formatting context (was `overflow-y: auto`, which never scrolled here – the column has no height – but
	   clipped the full-viewport video slices sideways: overflow-y auto turns overflow-x into auto) */
	.renderer {
		display: flow-root;
	}
</style>
