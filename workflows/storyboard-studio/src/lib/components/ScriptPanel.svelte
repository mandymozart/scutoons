<script>
	// The script lines of the selected shot's scene (script:<toon>:de, the local dev DB) and the scene's notes.
	import { find, studio } from '../studio.svelte.js';
	const sc = $derived(studio.sel ? find(studio.sel.id)?.sc : null);
</script>

<div class="panel">
	{#if studio.data?.scriptError}<p class="sc-error">{studio.data.scriptError}</p>{/if}
	{#if sc}
		<h3>{sc.sb.source?.scene} {sc.sb.source?.heading ?? ''}</h3>
		{#if sc.script}
			<pre>{sc.script.lines.join('\n')}</pre>
		{:else}
			<p class="hint">No script text for this scene.</p>
		{/if}
		{#if (sc.sb.notes ?? []).length}
			<h4>Notes</h4>
			<ul>
				{#each sc.sb.notes as n}<li>{n}</li>{/each}
			</ul>
		{/if}
	{/if}
</div>

<style>
	.panel {
		padding: var(--space-1);
	}
	h3,
	h4 {
		margin: 0 0 var(--space-05);
		font-size: var(--type-caption-size);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	h4 {
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
		margin: 0;
		padding-left: var(--space-1);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		display: grid;
		gap: var(--space-05);
	}
	.hint {
		font-size: var(--type-caption-size);
	}
</style>
