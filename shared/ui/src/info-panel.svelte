<script lang="ts">
	import type { Snippet } from 'svelte';
	import Icon from './icon.svelte';
	import CloseButton from './close-button.svelte';

	/**
	 * The one place for a page's help (owner 2026-09-30): explanations, conventions, glossaries and the style guide live
	 * here instead of long hint texts above the fields. A non-modal panel on the right, from the top bar
	 * (`--info-panel-top`, else the AppShell's measured top bar, else 0) down to the ActionBar (`--action-bar-height`);
	 * from 1100px on the page content makes room for it, narrower it lies above the content. Usually opened by the
	 * ActionBar's "ⓘ Info" toggle (`ActionBar` `info` snippet), which also remembers the open state.
	 * `href` + `hrefLabel`: a link to the full guide at the end of the panel. Escape inside the panel closes it.
	 */
	let {
		title = 'Info',
		closeLabel = 'Close',
		href,
		hrefLabel = 'Full style guide',
		id,
		onclose,
		children
	}: {
		title?: string;
		closeLabel?: string;
		/** link to the full guide (opens in a new tab) */
		href?: string;
		hrefLabel?: string;
		id?: string;
		onclose?: () => void;
		children: Snippet;
	} = $props();

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && onclose) {
			event.stopPropagation();
			onclose();
		}
	}
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<aside class="sc-info-panel" {id} aria-label={title} {onkeydown} data-testid="info-panel">
	<header>
		<h2><Icon name="info" /> {title}</h2>
		{#if onclose}
			<CloseButton class="close" label={closeLabel} onclick={onclose} />
		{/if}
	</header>
	<div class="body">
		{@render children()}
		{#if href}
			<p class="guide"><a {href} target="_blank" rel="noopener">{hrefLabel}</a></p>
		{/if}
	</div>
</aside>

<style>
	/* wide screens: the page makes room for the panel instead of hiding under it */
	@media (min-width: 1100px) {
		:global(body:has(.sc-info-panel) .sc-shell-content) {
			margin-right: var(--info-panel-width, 24rem);
		}
	}
	.sc-info-panel {
		position: fixed;
		z-index: 35;
		top: var(--info-panel-top, var(--sc-shell-top, 0px));
		/* left of a shell-level panel docked on the right (the creator's Ask panel sets --ask-inset) */
		right: var(--ask-inset, 0rem);
		bottom: var(--action-bar-height, 3.25rem);
		display: flex;
		flex-direction: column;
		box-sizing: border-box;
		width: min(var(--info-panel-width, 24rem), 100vw);
		background: var(--paper);
		color: var(--ink);
		border-left: var(--border-width-regular) solid var(--ink);
		font-family: var(--font-mono);
		font-size: var(--type-body-size);
		line-height: var(--type-body-line);
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-05);
		flex: none;
		padding: var(--space-05) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	h2 {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		margin: 0;
		font-size: var(--type-label-size);
		line-height: var(--type-label-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	/* the close cross (CloseButton): right, level with the title */
	header :global(.close) {
		margin: 0 0 0 auto;
	}
	.body {
		flex: 1;
		min-height: 0;
		overflow-x: hidden;
		overflow-y: auto;
		overscroll-behavior: contain;
		padding: var(--space-075) var(--space-1) var(--space-15);
	}
	/* help content: plain HTML from the app, styled here once */
	.body :global(h3) {
		margin: var(--space-15) 0 var(--space-05);
		font-size: var(--type-label-size);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.body :global(h3:first-child) {
		margin-top: 0;
	}
	.body :global(p) {
		margin: 0 0 0.5rem;
	}
	.body :global(ul) {
		margin: 0 0 0.5rem;
		padding-left: 1.1rem;
	}
	.body :global(li) {
		margin-bottom: 0.25rem;
	}
	.body :global(dl) {
		margin: 0 0 0.5rem;
	}
	.body :global(dt) {
		margin-top: 0.4rem;
		font-weight: var(--font-weight-regular);
	}
	.body :global(dd) {
		margin: 0 0 0 0.75rem;
	}
	.body :global(code) {
		font-weight: var(--font-weight-regular);
		overflow-wrap: anywhere;
	}
	.body :global(kbd) {
		padding: 0 0.3em;
		border: var(--border-width-regular) solid var(--ink);
		font: inherit;
	}
	.guide {
		margin-top: var(--space-15);
		padding-top: var(--space-075);
		border-top: var(--border-width-regular) solid var(--ink);
	}
	.guide a {
		color: var(--ink);
		font-weight: var(--font-weight-regular);
	}
</style>
