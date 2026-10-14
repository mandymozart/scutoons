<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { mediaUrl } from './setup.ts';
	import { modelThumbnail } from './model-view.js';

	/**
	 * A still picture of a 3D model (.glb) for a card (the asset picker, the admin's asset grid): rendered once by one
	 * shared renderer when the card scrolls into view (cached); the box icon until then or when it fails.
	 */
	let { path, size = '2em' }: { path: string; size?: string } = $props();

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
			modelThumbnail(url).then((png) => live && (src = png)).catch(() => undefined);
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
