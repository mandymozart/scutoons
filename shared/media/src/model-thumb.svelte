<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { mediaUrl } from './setup.ts';
	import { modelThumbnail } from './model-view.js';

	/**
	 * A still picture of a 3D model (.glb) for a card (the asset picker, the admin's asset grid): rendered once by one
	 * shared renderer when the card scrolls into view (cached); the box icon until then or when it fails. `still`: rendered
	 * at that shape (width × height px) and handed to `onstill` (a card that stores it as the model's thumbnail).
	 */
	let {
		path,
		size = '2em',
		still,
		onstill
	}: { path: string; size?: string; still?: { width: number; height: number }; onstill?: (png: string) => void } = $props();

	let el = $state<HTMLElement>();
	let src = $state('');

	$effect(() => {
		const node = el;
		const url = mediaUrl(path);
		src = '';
		if (!node || !path) return;
		let live = true;
		const io = new IntersectionObserver((entries) => {
			if (!entries.some((e) => e.isIntersecting)) return;
			io.disconnect();
			modelThumbnail(url, still?.width, still?.height)
				.then((png) => {
					if (!live) return;
					src = png;
					onstill?.(png);
				})
				.catch(() => undefined);
		});
		io.observe(node);
		return () => {
			live = false;
			io.disconnect();
		};
	});
</script>

<span class="model-thumb" bind:this={el}>
	{#if src}<img {src} alt="" />{:else}<Icon name="box" {size} />{/if}
</span>

<style>
	.model-thumb {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 100%;
		height: 100%;
	}
	img {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}
</style>
