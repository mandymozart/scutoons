<script lang="ts">
	// A storyboard studio tab's body: its icon and title (the shot it shows), an optional row under it (the Shot inspector's
	// tabs); the body scrolls on its own. The panel's head folds, closes and moves it.
	import type { Snippet } from 'svelte';
	import { Icon } from '@scutoons/ui';

	let { title, icon, tabs, children, testid }: { title: string; icon: string; tabs?: Snippet; children: Snippet; testid?: string } = $props();
</script>

<section class="storyboard-panel" aria-label={title} data-testid={testid}>
	<header>
		<h2><Icon name={icon} /> <span>{title}</span></h2>
	</header>
	{#if tabs}<div class="head-row">{@render tabs()}</div>{/if}
	<div class="body">{@render children()}</div>
</section>

<style>
	.storyboard-panel {
		display: flex;
		flex-direction: column;
		box-sizing: border-box;
		height: 100%;
		min-height: 0;
	}
	/* as the creator's panels: one control row, the label style, a rule under it */
	header {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		flex: none;
		padding: var(--space-05) var(--space-05) var(--space-05) var(--space-1);
		border-bottom: var(--editor-edge) solid var(--editor-line);
	}
	h2 {
		flex: 1;
		display: flex;
		align-items: center;
		gap: var(--space-05);
		min-width: 3rem;
		margin: 0;
		font-size: var(--type-label-size);
		line-height: var(--type-label-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	h2 span {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	h2 :global(svg) {
		flex: none;
	}
	.head-row {
		flex: none;
		padding: var(--space-05) var(--space-1) 0;
	}
	.body {
		flex: 1 1 auto;
		min-height: 0;
		overflow-x: hidden;
		overflow-y: auto;
		overscroll-behavior: contain;
	}
</style>
