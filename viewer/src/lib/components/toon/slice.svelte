<script lang="ts">
	import { fromAction } from 'svelte/attachments';
	import { pickQualityVariant, type Slice as SliceModel } from '@scutoons/content';
	import { Slice, sliceAudio } from '@scutoons/reader';
	import { page } from '$app/state';
	import { asset } from '$lib/utils/cdn.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { audioPlayer } from '$lib/actions/audio/index.ts';
	import { mediaQuality } from '$lib/media-quality.svelte.ts';
	import Samples from './audio/samples.svelte';
	import Triggers from './vn/triggers.svelte';

	let { slice, sliceIndex = 0, allSlices = [] }: { slice: SliceModel; sliceIndex?: number; allSlices?: SliceModel[] } =
		$props();

	const toonId = $derived(page.params.toonId);
	const imageUrl = $derived(asset(`${toonId}/${slice.image}`));
	// the file for the reader's media quality (lib/media-quality.svelte.ts); a video without variants is just its `src`
	const hasVariants = $derived(Object.values(slice.video?.variants ?? {}).some((file) => !!file));
	const videoUrl = $derived.by(() => {
		const video = slice.video;
		if (!video?.src) return undefined;
		if (!hasVariants) return asset(`${toonId}/${video.src}`);
		const level = mediaQuality.level;
		return level ? asset(`${toonId}/${pickQualityVariant(video, level)}`) : undefined;
	});
	// "Automatic" without the Network Information API: measure with the first media (its low variant)
	$effect(() => {
		const video = slice.video;
		if (video?.src && hasVariants && mediaQuality.needsMeasure) void mediaQuality.measure(asset(`${toonId}/${pickQualityVariant(video, 'low')}`));
	});
	/** the first slices load right away, the rest lazily (agents/PERFORMANCE.md P2) */
	const EAGER_SLICES = 2;
	// chapter files are loaded per locale, so the UI locale is the language of the text
	const lang = getLocale();
</script>

<Slice
	{slice}
	data-slice-index={sliceIndex}
	src={imageUrl}
	videoSrc={videoUrl}
	eager={sliceIndex < EAGER_SLICES}
	autoplayPlayer
	soundOnLabel={m['player.sound_on']()}
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
