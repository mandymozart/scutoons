<script lang="ts">
	import type { Snippet } from 'svelte';

	/** Entity / page header (admin UI pattern): small uppercase type label, title, one meta line, actions. */
	let {
		type,
		title,
		meta,
		actions,
		children
	}: {
		type?: string;
		title: string;
		meta?: string;
		actions?: Snippet;
		children?: Snippet;
	} = $props();
</script>

<header class="page-header">
	<div class="text">
		{#if type}<p class="type">{type}</p>{/if}
		<h1>{title}</h1>
		{#if meta}<p class="meta">{meta}</p>{/if}
		{#if children}{@render children()}{/if}
	</div>
	{#if actions}<div class="actions">{@render actions()}</div>{/if}
</header>

<style>
	.page-header {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-end;
		justify-content: space-between;
		gap: var(--space-1);
		margin-bottom: var(--space-15);
	}
	.text {
		min-width: 0;
	}
	.type {
		margin: 0 0 var(--space-025);
		color: var(--text-muted);
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}
	h1 {
		margin: 0;
		font-size: 1.5em;
		line-height: 1.2;
		overflow-wrap: anywhere;
	}
	.meta {
		margin: var(--space-025) 0 0;
		color: var(--text-muted);
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
	}
</style>
