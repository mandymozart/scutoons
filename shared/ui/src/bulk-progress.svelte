<script lang="ts">
	import type { Snippet } from 'svelte';
	import Button from './button.svelte';
	import type { BulkState } from './bulk-run.svelte.ts';

	/**
	 * A bulk run's progress (research/ASSET-LISTS.md): what runs, a bar, "done / total", Cancel while it runs (it stops
	 * before the next batch). After the run the page's report (`children`) takes its place. In progress is not a
	 * selection: the bar is ink on paper in a 2px ink frame (DESIGN.md "Selection" – In progress).
	 */
	let {
		run,
		label,
		count,
		cancelLabel,
		children
	}: {
		run: BulkState;
		/** e.g. "Deleting files …" */
		label: string;
		/** e.g. (done, total) => "1,200 of 7,252 files" */
		count: (done: number, total: number) => string;
		cancelLabel: string;
		/** the report, shown once the run is over */
		children?: Snippet;
	} = $props();
</script>

{#if run.running}
	<div class="sc-bulk-progress" role="status" data-testid="bulk-progress">
		<span>{label}</span>
		<span class="bar" role="progressbar" aria-label={label} aria-valuemin={0} aria-valuemax={run.total} aria-valuenow={run.done}><span style:width="{run.percent}%"></span></span>
		<span class="count">{count(run.done, run.total)}</span>
		<Button size="sm" variant="secondary" onclick={() => run.cancel()} data-testid="bulk-cancel">{cancelLabel}</Button>
	</div>
{:else if children}
	{@render children()}
{/if}

<style>
	.sc-bulk-progress {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05) var(--space-1);
		margin: 0 0 var(--space-1);
	}
	.bar {
		flex: 1 1 12rem;
		height: 1rem;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
	}
	.bar span {
		display: block;
		height: 100%;
		background: var(--ink);
	}
	.count {
		font-variant-numeric: tabular-nums;
	}
</style>
