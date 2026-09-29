<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import type { Slice } from '@scutoons/content';
	import Dialogs from './dialog/dialogs.svelte';

	/**
	 * One slice: the image and its text boxes. The app passes the resolved image URL and the text language, and
	 * renders its own interactive layers (samples, VN triggers) in `before` (below the image in paint order, like
	 * in the reader) or `children` (after the text boxes). Other attributes (and `{@attach}`) go to the `.slice`
	 * element, e.g. the app's audio action via `fromAction(sliceAudio, …)`.
	 */
	let {
		slice,
		src,
		lang,
		alt = 'Slice',
		before,
		children,
		...rest
	}: {
		slice: Slice;
		/** absolute image URL (the app resolves `slice.image` against its CDN) */
		src: string;
		/** language of the dialog text (hyphenation) */
		lang?: string;
		alt?: string;
		before?: Snippet;
		children?: Snippet;
	} & Omit<HTMLAttributes<HTMLDivElement>, 'children'> = $props();
</script>

<div {...rest} class="slice">
	{@render before?.()}
	<img {src} {alt} loading="eager" />
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
