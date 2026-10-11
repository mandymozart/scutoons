<script lang="ts">
	/**
	 * How far a chapter was read (BACKLOG 10.69): a hairline with the read part in ink – the reader's chapter list bar
	 * (viewer chapter-item.svelte) – plus the percentage.
	 */
	let { position }: { position: number } = $props();
	const percent = $derived(Math.round(Math.min(1, Math.max(0, position)) * 100));
</script>

<span class="reading-bar" data-testid="reading-bar" data-percent={percent}>
	<span class="rule" aria-hidden="true"><span style:width="{percent}%"></span></span>
	<span class="value">{percent} %</span>
</span>

<style>
	.reading-bar {
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
		white-space: nowrap;
	}
	.rule {
		position: relative;
		width: 6rem;
		height: var(--border-width-regular);
		background: var(--border-subtle);
	}
	.rule span {
		position: absolute;
		inset: 0 auto 0 0;
		background: var(--ink);
	}
	.value {
		font-variant-numeric: tabular-nums;
	}
</style>
