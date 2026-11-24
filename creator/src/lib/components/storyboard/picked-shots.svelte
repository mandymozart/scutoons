<script lang="ts">
	// The stage with several shots picked (lap 2 C3, the owner's "go with the flow" on STORYBOARD.md point 3): the picked
	// shots side by side in the toon's order, as the Timeline lays out their clips – each its schematic in its own shape,
	// the current one ringed in the text colour. A press makes one the current shot (the picked set stays).
	import type { Snippet } from 'svelte';
	import { shotSvg } from '@scutoons/content/storyboard';
	import { m } from '$lib/paraglide/messages.js';
	import { svgSource } from '$lib/editor/storyboard-shots.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';

	let { sb, lead }: { sb: StoryboardPage; lead?: Snippet } = $props();

	const shots = $derived(
		sb.entries
			.filter((e) => sb.picked.includes(e.id))
			.map((e) => {
				const f = shotSvg(e.shot, e.scene);
				return { id: e.id, src: svgSource(f.svg), w: f.w, h: f.h };
			})
	);
</script>

<div class="picked" data-testid="picked-shots">
	{#if lead}<div class="lead">{@render lead()}</div>{/if}
	<div class="row" role="group" aria-label={m['storyboard.side_by_side']({ n: shots.length })}>
		{#each shots as s (s.id)}
			<button type="button" class="shot" class:current={s.id === sb.current} aria-current={s.id === sb.current ? 'true' : undefined} data-control-exempt="a shot on the stage" title={s.id} onclick={() => sb.focus(s.id)} data-testid="picked-{s.id}">
				<img src={s.src} alt={m['storyboard.view_label']({ id: s.id })} style:aspect-ratio="{s.w} / {s.h}" />
				<span class="id">{s.id}</span>
			</button>
		{/each}
	</div>
</div>

<style>
	.picked {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		min-height: 0;
	}
	.lead {
		display: flex;
		align-items: center;
		padding: var(--space-025) var(--space-1);
	}
	.row {
		flex: 1;
		min-height: 0;
		display: flex;
		align-items: flex-start;
		justify-content: safe center;
		gap: var(--space-1);
		padding: var(--space-1);
		overflow: auto;
	}
	.shot {
		flex: none;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-025);
		height: 100%;
		margin: 0;
		padding: 0.25rem;
		border: 0;
		border-radius: var(--editor-radius);
		background: transparent;
		color: var(--tool-muted);
		font: inherit;
		cursor: pointer;
	}
	.shot img {
		display: block;
		flex: 1;
		min-height: 0;
		height: 100%;
		max-width: 24rem;
		object-fit: contain;
		box-sizing: border-box;
		border: var(--editor-edge) solid var(--tool-line);
	}
	.shot.current {
		color: var(--tool-text);
	}
	.shot.current img {
		outline: 2px solid var(--kit-thumb-on-ring);
		outline-offset: 1px;
	}
	@media (hover: hover) {
		.shot:hover:not(.current) img {
			border-color: var(--kit-thumb-hover-border);
		}
	}
	.shot:focus-visible {
		outline: 2px solid var(--kit-focus-rest-ring);
		outline-offset: -2px;
	}
	.id {
		flex: none;
		font-family: var(--editor-sans);
		font-size: var(--editor-small);
	}
</style>
