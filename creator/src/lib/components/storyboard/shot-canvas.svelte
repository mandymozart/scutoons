<script lang="ts">
	// The page's main view: the current shot. The live 3D view (#124, shot-3d-view – three.js, loaded only here, with a
	// dynamic import: the rest of the creator doesn't grow); its schematic (@scutoons/content/storyboard shotSvg –
	// figures, texts, the scroll move, the air) while the 3D view loads and where WebGL is missing. One boundary: it gets
	// the page state and draws the current shot.
	import { shotSvg } from '@scutoons/content/storyboard';
	import { m } from '$lib/paraglide/messages.js';
	import { svgSource } from '$lib/editor/storyboard-shots.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';

	let { sb }: { sb: StoryboardPage } = $props();

	/** WebGL here? (a throwaway context, released at once) */
	function webgl(): boolean {
		try {
			const gl = document.createElement('canvas').getContext('webgl2');
			gl?.getExtension('WEBGL_lose_context')?.loseContext();
			return !!gl;
		} catch {
			return false;
		}
	}
	const view3d = webgl() ? import('$lib/components/storyboard/shot-3d-view.svelte') : Promise.reject(new Error('no WebGL'));
	const frame = $derived(sb.sel ? shotSvg(sb.sel.shot, sb.sel.scene) : null);
</script>

{#snippet schematic(title?: string)}
	<div class="canvas" data-testid="shot-canvas">
		{#if frame && sb.sel}
			<img src={svgSource(frame.svg)} alt={m['storyboard.view_label']({ id: sb.sel.id })} {title} style:aspect-ratio="{frame.w} / {frame.h}" />
		{:else if sb.entries.length}
			<p class="sc-empty">{m['storyboard.no_shot']()}</p>
		{/if}
	</div>
{/snippet}

{#await view3d}
	{@render schematic()}
{:then { default: Shot3dView }}
	{#if sb.sel}
		<Shot3dView {sb} />
	{:else}
		{@render schematic()}
	{/if}
{:catch}
	{@render schematic(m['storyboard.view3d_no_webgl']())}
{/await}

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
