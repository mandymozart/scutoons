<script lang="ts">
	import { getInventoryContext, localized } from '$lib/contexts/inventory.svelte.ts';

	/**
	 * The reader's stats as plain brutalist bars ("Testo 62"), ITEMS.md "Substances": the current level (drifting back
	 * to the stat's baseline over time), no gendered wording. `include` / `exclude` pick the stats: the testo bar lives in
	 * the Avatar tab (owner 2026-10-01: "the testo is part of the avatar"), the others (appetite) in the Items tab.
	 */
	let { include, exclude = [] }: { include?: string[]; exclude?: string[] } = $props();
	const inventory = getInventoryContext();
	const shown = $derived(inventory.stats.filter((s) => (!include || include.includes(s.id)) && !exclude.includes(s.id)));
</script>

{#if shown.length}
	<div class="stats" data-testid="reader-stats">
		{#each shown as s (s.id)}
			<div class="stat" data-stat={s.id} data-level={s.level}>
				<span class="label">{localized(s.def.label) || s.id} {s.level}</span>
				<span class="bar" role="meter" aria-valuemin="0" aria-valuemax="100" aria-valuenow={s.level} aria-label={localized(s.def.label) || s.id}>
					<span class="fill" style="width: {s.level}%"></span>
				</span>
			</div>
		{/each}
	</div>
{/if}

<style>
	.stats {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem 1rem;
		margin: 0 0 1rem;
	}
	.stat {
		flex: 1 1 8rem;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: 0.75rem;
		font-weight: 700;
	}
	.bar {
		display: block;
		height: 0.75rem;
		border: 2px solid var(--ink);
		background: var(--paper);
	}
	.fill {
		display: block;
		height: 100%;
		background: var(--ink);
	}
</style>
