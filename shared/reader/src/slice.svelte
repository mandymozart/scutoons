<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { DEFAULT_SLICE, type Slice } from '@scutoons/content';
	import Dialogs from './dialog/dialogs.svelte';
	import SliceVideo from './slice-video.svelte';

	/**
	 * One slice: the image (or its video) and its text boxes. The app passes the resolved URLs and the text language,
	 * and renders its own interactive layers (samples, VN triggers) in `before` (below the image in paint order, like
	 * in the reader) or `children` (after the text boxes). Other attributes (and `{@attach}`) go to the `.slice`
	 * element, e.g. the app's audio action via `fromAction(sliceAudio, …)`.
	 *
	 * The image loads lazily (`eager` for the first slices) with `width` / `height` from the slice, so the box has its
	 * final height before the image arrives: no layout shift, and the slices below don't all intersect at once.
	 */
	let {
		slice,
		src,
		videoSrc,
		videoSources = [],
		lang,
		alt = 'Slice',
		eager = false,
		autoplayPlayer = false,
		soundOnLabel,
		before,
		children,
		...rest
	}: {
		slice: Slice;
		/** absolute image URL (the app resolves `slice.image` against its CDN); the poster of a video slice */
		src: string;
		/** absolute video URL when the slice has a `video` (the app resolves `slice.video.src`) */
		videoSrc?: string;
		/** `slice.video.sources` with absolute URLs (the app resolves them like `videoSrc`) */
		videoSources?: { src: string; media?: string }[];
		/** language of the dialog text (hyphenation) */
		lang?: string;
		alt?: string;
		/** load right away (the first slices of a chapter), else lazily */
		eager?: boolean;
		/** a player video slice starts by itself once in view (the reader; `SliceVideo`) */
		autoplayPlayer?: boolean;
		/** the label of the player's "Sound on" button (the app's language) */
		soundOnLabel?: string;
		before?: Snippet;
		children?: Snippet;
	} & Omit<HTMLAttributes<HTMLDivElement>, 'children'> = $props();

	// slices from before the size was required fall back to the standard size (the real ratio wins once loaded)
	const width = $derived(slice.width > 0 ? slice.width : DEFAULT_SLICE.width);
	const height = $derived(slice.height > 0 ? slice.height : DEFAULT_SLICE.height);
</script>

<div {...rest} class="slice">
	{@render before?.()}
	{#if slice.video && videoSrc}
		<SliceVideo
			src={videoSrc}
			sources={videoSources}
			poster={src}
			{width}
			{height}
			{alt}
			loop={slice.video.loop ?? true}
			autoplay={slice.video.autoplay ?? true}
			player={slice.video.player === true}
			{autoplayPlayer}
			{soundOnLabel}
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
	<Dialogs dialogs={slice.dialog} {lang} />
	{@render children?.()}
</div>

<style>
	.slice {
		position: relative;
		width: 100%;
		min-height: 200px;
		/* dialog font sizes are in cqw = % of the slice width */
		container-type: inline-size;
	}

	.slice img {
		width: 100%;
		height: auto;
		display: block;
	}
</style>
