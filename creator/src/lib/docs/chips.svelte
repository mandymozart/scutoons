<script lang="ts">
	// A use case's where / who / how often, in one quiet line.
	import { Chip } from '@scutoons/ui/editor';
	import { OFTEN, WHERE, WHO, type Case } from '$lib/docs/catalog.ts';

	let { item, soon = false }: { item: Case; soon?: boolean } = $props();
</script>

<span class="chips">
	{#each item.where as where (where)}<Chip label={WHERE[where]} on />{/each}
	<span class="who">{item.who.map((w) => WHO[w]).join(', ')}</span>
	<span class="often" title="How often: {OFTEN[item.often]}">{'●'.repeat(item.often)}</span>
	{#if item.partly}<Chip label="In part" title="Built in part: the guide says what works today" />{/if}
	{#if soon}<Chip label="Guide soon" />{/if}
</span>

<style>
	.chips {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.25rem 0.5rem;
		font-size: 0.75rem;
		color: var(--d-muted);
	}
	.often {
		margin-left: auto;
		color: var(--d-accent);
		letter-spacing: 0.1em;
	}
</style>
