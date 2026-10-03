<script lang="ts">
	import { fromAction } from 'svelte/attachments';
	import { pickQualityVariant, type Slice as SliceModel, type SliceItem } from '@scutoons/content';
	import { Slice, sliceAudio, type ResolveSliceItem } from '@scutoons/reader';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { getCurrentInventory } from '$lib/contexts/inventory.svelte.ts';
	import { getGameContext } from '$lib/contexts/game.svelte';
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import { pickedKey, pickUpSliceItem, runSliceItemEvents } from '$lib/visual-novel/slice-items.ts';
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

	// placed items (Slice.items): icons from the item catalog; a tap does what the creator chose (lib/visual-novel/slice-items.ts)
	const game = getGameContext();
	const vn = getVisualNovelContext();
	const itemInfo: ResolveSliceItem = (item) => {
		const inventory = getCurrentInventory();
		const def = inventory?.item(item.item);
		return def?.icon ? { src: asset(def.icon), label: inventory!.nameOf({ item_id: item.item, variant: null }) } : null;
	};
	const itemHidden = (item: SliceItem) => (item.action === 'pickup' ? !!game?.getVariable(pickedKey(slice, item)) : false);
	/** placements whose events already ran on this page view (`once`, default true) */
	const ran = new Set<string>();
	async function onItemTap(item: SliceItem) {
		if (!game) return;
		if (item.action === 'pickup') {
			await pickUpSliceItem(slice, item, game);
		} else if (item.action === 'events' && vn) {
			const once = item.once ?? true;
			if (once && ran.has(item.id)) return;
			if (once) ran.add(item.id);
			const destination = await runSliceItemEvents(item, game, vn);
			if (destination) goto(destination);
		}
	}
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
	{itemInfo}
	{itemHidden}
	onitemtap={onItemTap}
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
