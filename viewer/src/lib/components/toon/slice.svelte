<script lang="ts">
	import type { Slice } from '$lib/models/content.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { page } from '$app/stores';
	import { audioEvents } from '$lib/actions/audio/index.ts';
	import Samples from './audio/samples.svelte';

	let { slice, sliceIndex = 0, allSlices = [] }: { slice: Slice, sliceIndex?: number, allSlices?: Slice[] } = $props();
	$inspect(slice);
	
	// Get toonId from route params and locale from paraglide
	const toonId = $derived($page.params.toonId);
	
	// Construct the full image path: toonId/locale/slice.image
	const imagePath = $derived(`${toonId}/${slice.image}`);
	const imageUrl = $derived(asset(imagePath));
	
	// Placeholder states for image preloading (to be implemented)
	let loading = false;
	let error = false;
</script>

<div class="slice" use:audioEvents={{ events: slice.audio, index: sliceIndex, slices: allSlices }}>
	{#if loading}
		<div class="slice-loading">Loading slice...</div>
	{/if}
	{#if error}
		<div class="slice-error">Error loading slice</div>
	{/if}
	{#if !loading && !error}
		<Samples samples={slice.audio?.audio?.samples} />
		<img src={imageUrl} alt="Slice" loading="eager"/>
	{/if}
</div>

<style>
	.slice {
		position: relative;
		width: 100%;
		min-height: 200px;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.slice-loading,
	.slice-error {
		padding: 2rem;
		text-align: center;
		color: var(--text-color, #666);
	}

	.slice-error {
		color: var(--error-color, #d32f2f);
	}

	.slice img {
		width: 100%;
		height: auto;
		display: block;
	}
</style>
