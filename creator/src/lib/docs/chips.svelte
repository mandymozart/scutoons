<script lang="ts">
	// A use case's where / who / how often, in one quiet line.
	import { OFTEN, WHERE, WHO, type Case } from '$lib/docs/catalog.ts';

	let { item, soon = false }: { item: Case; soon?: boolean } = $props();
</script>

<span class="chips">
	{#each item.where as where (where)}<span class="chip">{WHERE[where]}</span>{/each}
	<span class="who">{item.who.map((w) => WHO[w]).join(', ')}</span>
	<span class="often" title="How often: {OFTEN[item.often]}">{'●'.repeat(item.often)}</span>
	{#if item.partly}<span class="chip part" title="Built in part: the guide says what works today">In part</span>{/if}
	{#if soon}<span class="chip part">Guide soon</span>{/if}
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
	.chip {
		padding: 0.125rem 0.375rem;
		border: 1px solid var(--d-line);
		color: var(--d-ink);
	}
	.chip.part {
		border-style: dashed;
		color: var(--d-muted);
	}
	.often {
		margin-left: auto;
		color: var(--d-accent);
		letter-spacing: 0.1em;
	}
</style>
