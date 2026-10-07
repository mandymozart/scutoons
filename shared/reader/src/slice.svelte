<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { SliceLayerRef } from '@scutoons/content';
	import type { HTMLAttributes } from 'svelte/elements';
	import { DEFAULT_SLICE, type Slice, type SliceItem } from '@scutoons/content';
	import SliceLayers from './slice-layers.svelte';
	import type { ResolveSliceItem } from './items/slice-item.svelte';
	import type { DialogPlay, TextExpressions } from './types.ts';
	import type { Dialog as DialogModel } from '@scutoons/content';
	import SliceVideo from './slice-video.svelte';
	import type { PlayerLabels } from './player-overlay.svelte';

	/**
	 * One slice: the image (or its video) and its text boxes. The app passes the resolved URLs and the text language,
	 * and renders its own interactive layers (samples, VN triggers) in `before` (below the image in paint order, like
	 * in the reader) or `children` (after the text boxes). Other attributes (and `{@attach}`) go to the `.slice`
	 * element, e.g. the app's audio action via `fromAction(sliceAudio, …)`.
	 *
	 * Placed items (`slice.items`) and text boxes are ONE layer stack above the image (`SliceLayers`, their `layer`;
	 * none = items under the text boxes); items are drawn once the app resolves their icons (`itemInfo`); `itemHidden` /
	 * `onitemtap` are the app's (what the reader took, what a tap does). Click / hover areas (events with a rect, samples
	 * with an area – BACKLOG 10.54) join that stack when the app passes `area` (it draws one area per call, in the
	 * stack's order); without it the app draws them in `before` (the old way: under the stack in paint order).
	 *
	 * The image loads lazily (`eager` for the first slices) with `width` / `height` from the slice, so the box has its
	 * final height before the image arrives: no layout shift, and the slices below don't all intersect at once.
	 *
	 * `blurred` (a sensitive slice while the reader hides sensitive content, agents/ITEMS.md "Sensitive slices"): the image
	 * – a video slice's poster, the video itself is not loaded (no sound, nothing to reveal) – is drawn blurred, clipped
	 * to the slice; the app mutes the slice's sounds and puts its note on top (`children`).
	 */
	let {
		slice,
		src,
		videoSrc,
		lang,
		alt = 'Slice',
		eager = false,
		autoplayPlayer = false,
		playerLabels,
		skippable = false,
		itemInfo,
		itemHidden,
		onitemtap,
		dialogHidden,
		dialogPlay,
		expressions,
		blurred = false,
		area,
		before,
		children,
		...rest
	}: {
		slice: Slice;
		/** absolute image URL (the app resolves `slice.image` against its CDN); the poster of a video slice */
		src: string;
		/**
		 * absolute URL of the video file when the slice has a `video`: the app picks the file for the reader's media quality
		 * (`pickQualityVariant(slice.video, level)`) and resolves it; undefined = not known yet (the poster meanwhile)
		 */
		videoSrc?: string;
		/** language of the dialog text (hyphenation) */
		lang?: string;
		alt?: string;
		/** load right away (the first slices of a chapter), else lazily */
		eager?: boolean;
		/** a player video slice starts by itself once in view (the reader; `SliceVideo`) */
		autoplayPlayer?: boolean;
		/** the player video's texts (the app's language; `SliceVideo`) */
		playerLabels?: Partial<PlayerLabels>;
		/** a player video with controls shows "Überspringen" (the app handles the `sliceskip` event; `SliceVideo`) */
		skippable?: boolean;
		/** a placed item's icon URL + name (from the app's item catalog); none = placed items are not drawn */
		itemInfo?: ResolveSliceItem;
		/** a placed item the reader already took (not drawn) */
		itemHidden?: (item: SliceItem) => boolean;
		/** a tap on a placed item whose `action` isn't `none` */
		onitemtap?: (item: SliceItem, index: number) => void;
		/** a text box a showTextBox event reveals later (not drawn yet) */
		dialogHidden?: (dialog: DialogModel) => boolean;
		/** a text box being revealed by a showTextBox event right now */
		dialogPlay?: (dialog: DialogModel) => DialogPlay | null | undefined;
		/** the reader's answers for `{…}` in the text boxes (none: the texts as written – the creator's previews) */
		expressions?: TextExpressions;
		/** draw the image blurred and leave a video unloaded (a sensitive slice the reader hides) */
		blurred?: boolean;
		/** draws one click / hover area in the layer stack (`SliceLayers`); none = the areas are not in the stack */
		area?: Snippet<[SliceLayerRef]>;
		before?: Snippet;
		children?: Snippet;
	} & Omit<HTMLAttributes<HTMLDivElement>, 'children'> = $props();

	// slices from before the size was required fall back to the standard size (the real ratio wins once loaded)
	const width = $derived(slice.width > 0 ? slice.width : DEFAULT_SLICE.width);
	const height = $derived(slice.height > 0 ? slice.height : DEFAULT_SLICE.height);
</script>

<div {...rest} class="slice" class:full-width={!blurred && !!slice.video?.viewport}>
	{@render before?.()}
	{#if blurred}
		<div class="blurred">
			<img {src} alt="" {width} {height} loading={eager ? 'eager' : 'lazy'} decoding="async" />
		</div>
	{:else if slice.video}
		<SliceVideo
			src={videoSrc}
			poster={src}
			{width}
			{height}
			{alt}
			loop={slice.video.loop ?? true}
			autoplay={slice.video.autoplay ?? true}
			player={slice.video.player === true}
			controls={slice.video.controls}
			viewport={slice.video.viewport}
			frame={slice.video.frame}
			{autoplayPlayer}
			{playerLabels}
			{skippable}
			{eager}
		/>
	{:else}
		<img
			{src}
			{alt}
			{width}
			{height}
			loading={eager ? 'eager' : 'lazy'}
			decoding="async"
			fetchpriority={eager ? 'high' : 'auto'}
		/>
	{/if}
	<SliceLayers
		dialogs={slice.dialog}
		items={slice.items}
		{lang}
		hidden={dialogHidden}
		play={dialogPlay}
		{expressions}
		resolve={itemInfo}
		{itemHidden}
		ontap={onitemtap}
		events={area ? slice.events : undefined}
		samples={area ? slice.audio?.audio?.samples : undefined}
		{area}
	/>
	{@render children?.()}
</div>

<style>
	.slice {
		position: relative;
		width: 100%;
		min-height: 200px;
		/* dialog font sizes are in cqw = % of the slice width */
		container-type: inline-size;
		/* a text box reaching past the slice's side (or one shivering there) never widens the page – on phones a page
		   wider than the screen moves / re-zooms ("the page jumps", owner 2026-10-07, BACKLOG M.2); up / down it may
		   still overlap the next slice (clip: no scroll container) */
		overflow-x: clip;
	}
	/* …except a video placed over the whole viewport (`video.viewport`): it is as wide as the page, not the column –
	   clipped to the column it showed a strip of itself on wide screens, and its autoplay (half of it in view) never
	   started (2026-10-07, check-next-bar 1440) */
	.slice.full-width {
		overflow-x: visible;
	}

	.slice img {
		width: 100%;
		height: auto;
		display: block;
	}

	/* a sensitive slice: blurred, clipped to the slice, scaled a little so the edges stay filled */
	.blurred {
		overflow: hidden;
	}
	.blurred img {
		filter: blur(1.5rem);
		transform: scale(1.15);
	}
</style>
