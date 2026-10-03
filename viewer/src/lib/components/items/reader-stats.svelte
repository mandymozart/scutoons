<script lang="ts">
	import { getInventoryContext, localized } from '$lib/contexts/inventory.svelte.ts';

	/**
	 * The reader's stats as plain brutalist bars ("Testo 62"), ITEMS.md "Substances": the current level (drifting back
	 * to the stat's baseline over time), no gendered wording. `include` / `exclude` pick the stats. The inventory shows
	 * them all (appetite, testo) small at the right of its tab row, so both tabs have them (owner 2026-10-03: "less
	 * massive"). `compact`: label and a thin bar on one line, in the surrounding text colour.
	 */
	let { include, exclude = [], compact = false }: { include?: string[]; exclude?: string[]; compact?: boolean } = $props();
	const inventory = getInventoryContext();
	const shown = $derived(inventory.stats.filter((s) => (!include || include.includes(s.id)) && !exclude.includes(s.id)));
</script>

{#if shown.length}
	<div class="stats" class:compact data-testid="reader-stats">
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
	/* stacked, one line each, right-aligned: fits next to the tabs on a phone */
	.compact {
		flex-direction: column;
		flex-wrap: nowrap;
		align-items: flex-end;
		gap: 0.25rem;
		margin: 0;
	}
	.compact .stat {
		flex: 0 0 auto;
		flex-direction: row;
		align-items: center;
		gap: 0.375rem;
		font-weight: 400;
		white-space: nowrap;
	}
	.compact .bar {
		width: 2.5rem;
		height: 0.5rem;
		border: 1px solid currentColor;
		background: transparent;
	}
	.compact .fill {
		background: currentColor;
	}
</style>
