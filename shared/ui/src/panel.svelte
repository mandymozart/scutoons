<script lang="ts">
	import type { Snippet } from 'svelte';

	/** A framed box (2px ink border, paper, square) with an optional header (title, hint, actions). */
	let {
		title,
		hint,
		actions,
		quiet = false,
		children
	}: {
		title?: string;
		hint?: string;
		actions?: Snippet;
		/** surface-panel background and no border (quiet info panels) */
		quiet?: boolean;
		children: Snippet;
	} = $props();
</script>

<section class="panel" class:quiet>
	{#if title || actions}
		<header>
			<div>
				{#if title}<h2>{title}</h2>{/if}
				{#if hint}<p class="hint">{hint}</p>{/if}
			</div>
			{#if actions}<div class="actions">{@render actions()}</div>{/if}
		</header>
	{/if}
	{@render children()}
</section>

<style>
	.panel {
		box-sizing: border-box;
		padding: var(--space-1);
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
		border-radius: var(--radius-none);
	}
	/* quiet = the inactive frame (no grey fill) */
	.quiet {
		border-color: var(--border-muted);
	}
	header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--space-1);
		margin-bottom: var(--space-075);
	}
	h2 {
		margin: 0;
		font-size: var(--type-label-size);
		line-height: var(--type-label-line);
		font-weight: var(--font-weight-bold);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label-bold);
	}
	.hint {
		margin: var(--space-025) 0 0;
		color: var(--text-muted);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.actions {
		display: flex;
		gap: var(--space-05);
		flex: none;
	}
</style>
