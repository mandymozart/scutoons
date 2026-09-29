<script lang="ts">
	/**
	 * Loading placeholder in the shape of what is coming: text lines, a block (chart, panel) or a table. Static
	 * surface-hover bars (no shimmer: DESIGN.md motion stays short and utilitarian); announced once via `label`.
	 */
	let {
		variant = 'text',
		lines = 3,
		rows = 6,
		columns = 4,
		height = '10rem',
		label = 'Loading …'
	}: {
		variant?: 'text' | 'block' | 'table';
		lines?: number;
		rows?: number;
		columns?: number;
		/** height of a block */
		height?: string;
		label?: string;
	} = $props();

	// deterministic widths so the placeholder does not jump between renders
	const widths = [92, 78, 85, 64, 88, 71, 95, 60];
</script>

<div class="sc-skeleton {variant}" role="status" aria-label={label}>
	{#if variant === 'text'}
		{#each { length: lines }, i (i)}
			<span class="bar" style:width="{widths[i % widths.length]}%"></span>
		{/each}
	{:else if variant === 'block'}
		<span class="block" style:height></span>
	{:else}
		<div class="table" style:grid-template-columns="repeat({columns}, minmax(0, 1fr))">
			{#each { length: columns }, c (c)}
				<span class="bar head"></span>
			{/each}
			{#each { length: rows * columns }, i (i)}
				<span class="bar" style:width="{widths[i % widths.length]}%"></span>
			{/each}
		</div>
	{/if}
</div>

<style>
	.sc-skeleton {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
	.bar,
	.block {
		display: block;
		background: var(--surface-hover);
	}
	.bar {
		height: 0.9em;
	}
	.block {
		width: 100%;
	}
	.table {
		display: grid;
		gap: var(--space-075) var(--space-1);
		padding: var(--space-075);
		border: var(--border-width-regular) solid var(--border-subtle);
	}
	.head {
		background: var(--border-subtle);
	}
</style>
