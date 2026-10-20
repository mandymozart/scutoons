<script lang="ts">
	// The page's main view: the current shot. For now its schematic (@scutoons/content/storyboard shotSvg – figures,
	// texts, the scroll move, the air) at the phone canvas's width; the live 3D view replaces this component in #124 (one
	// boundary: it gets the page state and draws the current shot).
	import { shotSvg } from '@scutoons/content/storyboard';
	import { m } from '$lib/paraglide/messages.js';
	import { svgSource } from '$lib/editor/storyboard-shots.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';

	let { sb }: { sb: StoryboardPage } = $props();

	const frame = $derived(sb.sel ? shotSvg(sb.sel.shot, sb.sel.scene) : null);
</script>

<div class="canvas" data-testid="shot-canvas">
	{#if frame && sb.sel}
		<img src={svgSource(frame.svg)} alt={m['storyboard.view_label']({ id: sb.sel.id })} style:aspect-ratio="{frame.w} / {frame.h}" />
	{:else if sb.entries.length}
		<p class="sc-empty">{m['storyboard.no_shot']()}</p>
	{/if}
</div>

<style>
	.canvas {
		box-sizing: border-box;
		height: 100%;
		overflow: auto;
		padding: var(--space-1);
		display: flex;
		justify-content: center;
		align-items: flex-start;
		background: var(--surface-panel, var(--paper));
	}
	/* the canvas's width (390) at most 24rem – a tall shot scrolls, as the reader would */
	img {
		display: block;
		box-sizing: border-box;
		width: min(100%, 24rem);
		height: auto;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
	}
</style>
