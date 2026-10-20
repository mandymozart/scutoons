<script lang="ts">
	// A Storyboard dock panel with the creator's head (DOCKING "Several panels" 2, DESIGN "Steps and the timeline"): its
	// icon and title (cut with … when narrow), DockFold (fold to the rail, maximize; the leftmost carries "Fold the page")
	// and the ✕; an optional row under the head (the Shot inspector's tabs); the body scrolls on its own. A narrow head
	// keeps room for its title: the icon goes first, then the buttons that have a key (a container query).
	import type { Snippet } from 'svelte';
	import { CloseButton, DockFold, Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';

	let { title, icon, onclose, tabs, children, testid }: { title: string; icon: string; onclose: () => void; tabs?: Snippet; children: Snippet; testid?: string } = $props();
</script>

<section class="storyboard-panel" aria-label={title} data-testid={testid}>
	<header>
		<h2><Icon name={icon} /> <span>{title}</span></h2>
		<DockFold label={m['creator.fold_panel']()} maximizeLabel={m['creator.maximize_panel']()} restoreLabel={m['creator.restore_panel']()} />
		<CloseButton class="close" label={m['storyboard.close_panel']({ panel: title })} onclick={onclose} />
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
		background: var(--paper);
	}
	/* as the creator's panels: one control row, the label style, a rule under it */
	header {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		flex: none;
		padding: var(--space-05) var(--space-05) var(--space-05) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
		container-type: inline-size;
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
	header :global(.close) {
		margin: 0;
		flex: none;
	}
	/* narrow heads: the icon goes first (the rail shows it), then "Fold the page", then maximize – the fold to the rail
	   and the ✕ always stay */
	@container (max-width: 18rem) {
		header:has(:global(.sc-page-fold)) h2 :global(svg.icon) {
			display: none;
		}
	}
	@container (max-width: 16rem) {
		header :global(.sc-page-fold) {
			display: none;
		}
	}
	@container (max-width: 15.5rem) {
		header h2 :global(svg.icon) {
			display: none;
		}
	}
	@container (max-width: 13rem) {
		header :global(.sc-maximize-button) {
			display: none;
		}
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
