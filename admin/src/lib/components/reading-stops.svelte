<script lang="ts">
	/**
	 * Where the unfinished readers of a chapter stand (BACKLOG 10.69): one small bar per 10 % step, the tallest step full
	 * height; the counts are in the title and the label.
	 */
	let { stops, label }: { stops: number[]; label: string } = $props();
	const max = $derived(Math.max(1, ...stops));
</script>

<span class="stops" role="img" aria-label={label} title={label} data-testid="reading-stops">
	{#each stops as n, i (i)}
		<span class="step" class:empty={n === 0} style:height="{n === 0 ? 0 : Math.max(12, (n / max) * 100)}%"></span>
	{/each}
</span>

<style>
	.stops {
		display: inline-flex;
		align-items: flex-end;
		gap: 0.125rem;
		height: 1.5rem;
		padding-bottom: var(--border-width-hairline);
		border-bottom: var(--border-width-hairline) solid var(--border-muted);
	}
	.step {
		width: 0.5rem;
		background: var(--ink);
	}
	.step.empty {
		background: none;
	}
</style>
