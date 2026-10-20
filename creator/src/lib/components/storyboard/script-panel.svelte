<script lang="ts">
	// The current shot's scene in the toon's script (`script:<toon>:<source language>`, read only – the Script page edits
	// it) and the scene's notes from the storyboard (decisions / questions for the owner).
	import { m } from '$lib/paraglide/messages.js';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';

	let { sb }: { sb: StoryboardPage } = $props();

	const scene = $derived(sb.sel?.scene ?? null);
	const lines = $derived(sb.sceneScript(scene?.source?.scene));
</script>

<div class="panel" data-testid="script-panel">
	{#if sb.scriptError}<p class="sc-error">{m['storyboard.script_error']({ error: sb.scriptError })}</p>{/if}
	{#if scene}
		<h3>{scene.source?.scene ?? ''} {scene.source?.heading ?? ''}</h3>
		{#if lines}<pre data-testid="script-lines">{lines.join('\n')}</pre>{/if}
		{#if scene.notes?.length}
			<h3 class="notes">{m['storyboard.notes']()}</h3>
			<ul>
				{#each scene.notes as note, i (i)}<li>{note}</li>{/each}
			</ul>
		{/if}
	{/if}
</div>

<style>
	.panel {
		padding: var(--space-1);
		box-sizing: border-box;
	}
	h3 {
		margin: 0 0 var(--space-05);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	h3.notes {
		margin-top: var(--space-1);
	}
	pre {
		margin: 0;
		white-space: pre-wrap;
		font: inherit;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	ul {
		display: grid;
		gap: var(--space-05);
		margin: 0;
		padding-left: var(--space-1);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
</style>
