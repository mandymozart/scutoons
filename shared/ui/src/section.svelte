<script lang="ts">
	import type { Snippet } from 'svelte';

	/** Entity-detail section (admin UI pattern): header with title + one-line hint, content, no frame. */
	let {
		title,
		hint,
		actions,
		children
	}: {
		title: string;
		hint?: string;
		actions?: Snippet;
		children: Snippet;
	} = $props();
</script>

<section class="section">
	<header>
		<div>
			<h2>{title}</h2>
			{#if hint}<p class="hint">{hint}</p>{/if}
		</div>
		{#if actions}<div class="actions">{@render actions()}</div>{/if}
	</header>
	{@render children()}
</section>

<style>
	.section {
		padding: var(--space-15) 0;
		border-top: var(--border-width-hairline) solid var(--border-subtle);
	}
	header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--space-1);
		margin-bottom: var(--space-1);
	}
	h2 {
		margin: 0;
		font-size: 1em;
		font-weight: var(--font-weight-bold);
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
