<script lang="ts">
	import type { Snippet } from 'svelte';

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
		<h2><span class="icon" aria-hidden="true">ⓘ</span> {title}</h2>
		{#if onclose}
			<button type="button" class="close" aria-label={closeLabel} title={closeLabel} onclick={onclose}>×</button>
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
		right: 0;
		bottom: var(--action-bar-height, 3.25rem);
		display: flex;
		flex-direction: column;
		box-sizing: border-box;
		width: min(var(--info-panel-width, 24rem), 100vw);
		background: var(--paper);
		color: var(--ink);
		border-left: var(--border-width-regular) solid var(--ink);
		font-family: var(--font-mono);
		font-size: 12px;
		line-height: 1.5;
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
		margin: 0;
		font-size: 13px;
		font-weight: var(--font-weight-bold);
	}
	.icon {
		font-weight: normal;
	}
	.close {
		margin: 0 0 0 auto;
		padding: 0 var(--space-025);
		background: transparent;
		border: 0;
		color: var(--ink);
		font: inherit;
		font-size: 1.5em;
		line-height: 1;
		cursor: pointer;
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
		margin: 1rem 0 0.25rem;
		font-size: 12px;
		font-weight: var(--font-weight-bold);
		text-transform: uppercase;
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
		font-weight: var(--font-weight-bold);
	}
	.body :global(dd) {
		margin: 0 0 0 0.75rem;
	}
	.body :global(code) {
		background: var(--surface-panel);
		padding: 0 0.2em;
		overflow-wrap: anywhere;
	}
	.body :global(kbd) {
		padding: 0 0.3em;
		border: 1px solid var(--border-muted);
		font: inherit;
	}
	.guide {
		margin-top: 1rem;
		padding-top: var(--space-075);
		border-top: var(--border-width-hairline) solid var(--border-subtle);
	}
	.guide a {
		color: var(--ink);
		font-weight: var(--font-weight-bold);
	}
</style>
