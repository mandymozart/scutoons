<script lang="ts">
	import { mediaText, mediaUrl } from './setup.ts';
	import type { VinylLook } from '@scutoons/content';
	import { mountModelView, type ModelInfo } from './model-view.js';
	import { buildVinyl } from './vinyl-model.js';

	/**
	 * A live 3D view of a model (.glb) – turns slowly, drag to turn (owner 2026-10-14: "a preview of the 3d asset in the
	 * asset manager and in the slot where it is placed"). `fallback` marks a stand-in (the mannequin of a persona without
	 * its own model: "it needs to reflect in the 3d preview if it is the fallback"). Starts when it scrolls into view;
	 * three.js loads only then.
	 */
	let {
		path = '',
		vinyl = null,
		fallback = false,
		testid
	}: {
		/** CDN-root path (`characters/catpope/catpope-regular.glb`) or a full URL */
		path?: string;
		/** instead of a .glb: the shared record model with this look (vinyl-model.js) */
		vinyl?: VinylLook | null;
		fallback?: boolean;
		testid?: string;
	} = $props();

	const t = $derived(mediaText());
	let canvas = $state<HTMLCanvasElement>();
	let info = $state<ModelInfo | null>(null);
	let failed = $state(false);

	$effect(() => {
		const el = canvas;
		const look = vinyl;
		const url = look ? (THREE: unknown) => buildVinyl(THREE, look, mediaUrl) : /^https?:/.test(path) ? path : mediaUrl(path);
		if (!el || (!path && !look)) return;
		info = null;
		failed = false;
		let view: { dispose(): void } | null = null;
		const io = new IntersectionObserver((entries) => {
			if (!entries.some((e) => e.isIntersecting) || view) return;
			view = mountModelView(el, url, { onload: (i) => (info = i), onerror: () => (failed = true) });
		});
		io.observe(el);
		return () => {
			io.disconnect();
			view?.dispose();
		};
	});
</script>

<figure class="model-preview" class:fallback data-testid={testid} title={t.modelTurn}>
	<canvas bind:this={canvas}></canvas>
	{#if fallback}<span class="sc-chip badge">{t.modelPlaceholder}</span>{/if}
	<figcaption>{failed ? t.modelError : info ? t.modelInfo(info.tris, info.bones) : ' '}</figcaption>
</figure>

<style>
	.model-preview {
		position: relative;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		width: 12rem;
	}
	canvas {
		display: block;
		box-sizing: border-box; /* the frame inside the 12rem (it stuck out 4px) */
		width: 100%;
		aspect-ratio: 1;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--surface, transparent);
		cursor: grab;
		touch-action: none;
	}
	/* a stand-in: a dashed frame like an empty slot */
	.fallback canvas {
		border-style: dashed;
	}
	.badge {
		position: absolute;
		top: var(--space-05);
		left: var(--space-05);
	}
	figcaption {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		color: var(--muted);
		min-height: var(--type-caption-line);
	}
</style>
