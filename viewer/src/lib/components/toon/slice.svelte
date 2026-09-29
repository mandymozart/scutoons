<script lang="ts">
	import { fromAction } from 'svelte/attachments';
	import type { Slice as SliceModel } from '@scutoons/content';
	import { Slice, sliceAudio } from '@scutoons/reader';
	import { page } from '$app/state';
	import { asset } from '$lib/utils/cdn.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { audioPlayer } from '$lib/actions/audio/index.ts';
	import Samples from './audio/samples.svelte';
	import Triggers from './vn/triggers.svelte';

	let { slice, sliceIndex = 0, allSlices = [] }: { slice: SliceModel; sliceIndex?: number; allSlices?: SliceModel[] } =
		$props();

	const toonId = $derived(page.params.toonId);
	const imageUrl = $derived(asset(`${toonId}/${slice.image}`));
	// chapter files are loaded per locale, so the UI locale is the language of the text
	const lang = getLocale();
</script>

<Slice
	{slice}
	src={imageUrl}
	{lang}
	{@attach fromAction(sliceAudio, () => ({
		events: slice.audio,
		index: sliceIndex,
		slices: allSlices,
		player: audioPlayer()
	}))}
>
	{#snippet before()}
		<Samples samples={slice.audio?.audio?.samples} />
		<Triggers events={slice.events} />
	{/snippet}
</Slice>
