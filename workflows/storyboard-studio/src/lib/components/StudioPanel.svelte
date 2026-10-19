<script>
	// A studio dock panel with the creator's head (DOCKING "Several panels" 2, research/STUDIO-LAYOUT.md §1.3): its icon
	// and title (cut with … when narrow), DockFold (fold to the rail, maximize; the leftmost carries "Fold the page") and
	// the ✕; an optional row under the head (the Shot panel's tabs); the body scrolls on its own.
	import { CloseButton, DockFold, Icon } from '@scutoons/ui';

	/** @type {{ title: string, icon: string, onclose: () => void, tabs?: import('svelte').Snippet, children: import('svelte').Snippet, testid?: string }} */
	let { title, icon, onclose, tabs, children, testid } = $props();
</script>

<section class="studio-panel" aria-label={title} data-testid={testid}>
	<header>
		<h2><Icon name={icon} /> <span>{title}</span></h2>
		<DockFold />
		<CloseButton class="close" label={`Close ${title.split(' · ')[0]}`} onclick={onclose} />
	</header>
	{#if tabs}<div class="tabs">{@render tabs()}</div>{/if}
	<div class="body">{@render children()}</div>
</section>

<style>
	.studio-panel {
		display: flex;
		flex-direction: column;
		box-sizing: border-box;
		height: 100%;
		min-height: 0;
		background: var(--paper);
	}
	/* as the creator's panels (InfoPanel): one control row, the label style, a rule under it */
	header {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		flex: none;
		padding: var(--space-05) var(--space-05) var(--space-05) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	h2 {
		flex: 1;
		display: flex;
		align-items: center;
		gap: var(--space-05);
		min-width: 0;
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
	.tabs {
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
