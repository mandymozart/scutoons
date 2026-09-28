<script lang="ts">
	import type { Slice } from '$lib/models/content.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { audioEvents } from '$lib/actions/audio/index.ts';
	import Samples from './audio/samples.svelte';
	import Triggers from './vn/triggers.svelte';
	import Dialogs from './dialog/dialogs.svelte';

	let { slice, sliceIndex = 0, allSlices = [] }: { slice: Slice, sliceIndex?: number, allSlices?: Slice[] } = $props();
	
	const toonId = $derived(page.params.toonId);
	
	const imagePath = $derived(`${toonId}/${slice.image}`);
	const imageUrl = $derived(asset(imagePath));
	
	let loading = false;
	let error = false;
</script>

<div 
	class="slice" 
	use:audioEvents={{ events: slice.audio, index: sliceIndex, slices: allSlices }}
>
	{#if loading}
		<div class="slice-loading">Loading slice...</div>
	{/if}
	{#if error}
		<div class="slice-error">Error loading slice</div>
	{/if}
	{#if !loading && !error}
		<Samples samples={slice.audio?.audio?.samples} />
		<Triggers events={slice.events} />
		<img src={imageUrl} alt="Slice" loading="eager"/>
		<Dialogs dialogs={slice.dialog} />
	{/if}
</div>

<style>
	.slice {
		position: relative;
		width: 100%;
		min-height: 200px;
		/* dialog font sizes are in cqw = % of the slice width */
		container-type: inline-size;
		/* display: flex;
		align-items: center;
		justify-content: center; */
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
