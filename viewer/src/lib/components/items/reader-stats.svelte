<script lang="ts">
	import { getInventoryContext, localized } from '$lib/contexts/inventory.svelte.ts';

	/**
	 * The reader's stats as plain brutalist bars ("Testo 62"), ITEMS.md "Substances": the current level (drifting back
	 * to the stat's baseline over time), no gendered wording.
	 */
	const inventory = getInventoryContext();
</script>

{#if inventory.stats.length}
	<div class="stats" data-testid="reader-stats">
		{#each inventory.stats as s (s.id)}
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
