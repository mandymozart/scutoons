<script lang="ts">
	import type { Snippet } from 'svelte';
	import { MediaQuery } from 'svelte/reactivity';
	import { mountModelView, webglAvailable } from '@scutoons/media/model-view';
	import type { ModelSource } from '@scutoons/content';
	import { modelLoader } from './model-source.ts';

	/**
	 * An item's 3D model in its detail view (FEATURES 10.102 – owner 2026-10-16: "The item should show sleeve with record
	 * inside"): a .glb or the shared record model with the item's look (model-source.ts) – a live view that turns slowly, drag to turn (`mountModelView`, @scutoons/media – three.js loads only when
	 * one is shown). Without WebGL, with reduced motion, while loading and when the model fails: `fallback` (the icon).
	 */
	let { path, label, fallback }: { path: ModelSource; label: string; fallback: Snippet } = $props();

	const reduced = new MediaQuery('prefers-reduced-motion: reduce');
	let canvas = $state<HTMLCanvasElement>();
	let ready = $state(false);
	let failed = $state(false);
	const shown = $derived(!!path && !failed && !reduced.current && webglAvailable());

	$effect(() => {
		const el = canvas;
		if (!el || !path) return;
		ready = false;
		failed = false;
		const view = mountModelView(el, modelLoader(path), { onload: () => (ready = true), onerror: () => (failed = true) });
		return () => view.dispose();
	});
</script>

{#if shown}
	<div class="item-model" class:ready>
		<canvas bind:this={canvas} aria-hidden="true" title={label} data-testid="item-model"></canvas>
		{#if !ready}<div class="under">{@render fallback()}</div>{/if}
	</div>
{:else}
	<div class="flat">{@render fallback()}</div>
{/if}

<style>
	/* the whole width it is given (the item detail: the panel's, edge to edge – owner 2026-10-16: a record pulled out of
	   its sleeve must not be cut off at the sides), a sensible height */
	.item-model {
		position: relative;
		width: 100%;
		height: min(24rem, 70vw);
	}

	canvas {
		display: block;
		width: 100%;
		height: 100%;
		cursor: grab;
		touch-action: pan-y;
		opacity: 0;
		transition: opacity 0.3s ease;
	}

	.ready canvas {
		opacity: 1;
	}

	/* the icon while the model loads (and without a 3D view): at an icon's size in the middle */
	.under {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.under > :global(*),
	.flat {
		width: min(15rem, 80%);
		margin-inline: auto;
	}
</style>
