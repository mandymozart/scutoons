<script lang="ts">
	import type { Snippet } from 'svelte';
	import { clickOutside } from './click-outside.ts';
	import InfoPanel from './info-panel.svelte';
	import Icon from './icon.svelte';
	import { panelWidth } from './panel-width.svelte.ts';
	import { shellDock, useDock } from './dock.svelte.ts';
	import { untrack } from 'svelte';

	/**
	 * Fixed bottom action bar (PLATFORM.md "Admin UI patterns" → "Bottom action bar", owner rule for admin + creator):
	 * mirrors the top bar – same height (`--action-bar-height`, default 3.25rem), paper, 2px ink rule on top, full width,
	 * always visible (no fade, no auto-hide). `start` = pagination / filters / search / status, `secondary` = secondary
	 * actions, `end` = the primary action(s), right-most. Narrower than `breakpoint`: `end` stays visible, `start` and
	 * `secondary` move into a "More" panel that opens upwards. While a bar is on the page, `body` gets a bottom padding
	 * of the bar's height, so nothing hides behind it (place the component anywhere in the page markup).
	 *
	 * `info`: the page's help. The bar then shows an "ⓘ Info" toggle (left-most, also on narrow screens) that opens an
	 * `InfoPanel` with this snippet (+ `infoHref`, a link to the full guide) in the AppShell's right dock (`useDock`,
	 * agents/DOCKING.md – it takes turns with the page's own panels; one remembered width `sc-panel-width:info`).
	 * Closed by default; open/closed is remembered in localStorage (`infoKey`), so it stays open across pages until the
	 * user closes it (or a page panel takes the dock).
	 */
	let {
		start,
		secondary,
		end,
		label = 'Page actions',
		moreLabel = 'More',
		breakpoint = 1024,
		info,
		infoOpen = $bindable(false),
		infoTitle = 'Info',
		infoHref,
		infoLabels = {},
		infoKey = 'sc-info-open'
	}: {
		start?: Snippet;
		secondary?: Snippet;
		end?: Snippet;
		/** accessible name of the bar */
		label?: string;
		moreLabel?: string;
		breakpoint?: number;
		/** the page's help, shown in the InfoPanel */
		info?: Snippet;
		/** open state of the InfoPanel (bindable, e.g. to close other side panels while it is open) */
		infoOpen?: boolean;
		/** heading of the InfoPanel (e.g. the page name) */
		infoTitle?: string;
		/** link to the full style guide at the end of the panel */
		infoHref?: string;
		/** translated labels: `toggle` (default "Info"), `close`, `guide` (the link text), `fold` (the dock's fold button) */
		infoLabels?: { toggle?: string; close?: string; guide?: string; fold?: string; maximize?: string; restore?: string };
		/** localStorage key of the remembered open state */
		infoKey?: string;
	} = $props();

	let narrow = $state(false);
	let moreOpen = $state(false);
	const uid = $props.id();

	$effect(() => {
		const query = window.matchMedia(`(max-width: ${breakpoint - 1}px)`);
		narrow = query.matches;
		const change = (event: MediaQueryListEvent) => {
			narrow = event.matches;
			if (!narrow) moreOpen = false;
		};
		query.addEventListener('change', change);
		return () => query.removeEventListener('change', change);
	});

	const hasMore = $derived(!!start || !!secondary);

	// the Info panel: closed by default, the user's choice is remembered (storage may be unavailable)
	$effect(() => {
		if (!info) return;
		try {
			// a panel the page opened itself (a deep link: `?flow=1`) keeps the dock
			infoOpen = localStorage.getItem(infoKey) === '1' && !untrack(() => shellDock.current);
		} catch {
			/* private mode etc. */
		}
	});

	// docked in the shell's right dock while open; a page panel taking the dock closes it for now (still remembered)
	const infoSize = panelWidth('sc-panel-width:info', 16, undefined, { foldable: true });
	useDock(() => (info && infoOpen ? { panel: infoPanel, size: infoSize, title: infoTitle, icon: 'info', close: () => (infoOpen = false) } : null));

	function setInfo(open: boolean) {
		infoOpen = open;
		try {
			if (open) localStorage.setItem(infoKey, '1');
			else localStorage.removeItem(infoKey);
		} catch {
			/* not remembered */
		}
	}
</script>

{#snippet infoToggle()}
	<button
		type="button"
		class="info-toggle"
		aria-expanded={infoOpen}
		aria-controls="{uid}-info"
		onclick={() => setInfo(!infoOpen)}
		data-testid="info-toggle"
	>
		<Icon name="info" /> <span class="info-word">{infoLabels.toggle ?? 'Info'}</span>
	</button>
{/snippet}

<div class="sc-action-bar" role="region" aria-label={label}>
	{#if narrow}
		{#if info}{@render infoToggle()}{/if}
		{#if hasMore}
			<div class="more" {@attach clickOutside(() => (moreOpen = false))}>
				<button
					type="button"
					class="more-toggle"
					aria-expanded={moreOpen}
					aria-controls="{uid}-more"
					onclick={() => (moreOpen = !moreOpen)}
				>
					{moreLabel}
				</button>
				{#if moreOpen}
					<div class="more-panel" id="{uid}-more">
						{#if start}<div class="more-start">{@render start()}</div>{/if}
						{#if secondary}<div class="more-secondary">{@render secondary()}</div>{/if}
					</div>
				{/if}
			</div>
		{/if}
		<div class="end">{#if end}{@render end()}{/if}</div>
	{:else}
		<div class="start">
			{#if info}{@render infoToggle()}{/if}
			{#if start}{@render start()}{/if}
		</div>
		<div class="end">
			{#if secondary}{@render secondary()}{/if}
			{#if end}{@render end()}{/if}
		</div>
	{/if}
</div>

{#snippet infoPanel()}
	<InfoPanel
		id="{uid}-info"
		title={infoTitle}
		closeLabel={infoLabels.close}
		foldLabel={infoLabels.fold}
		maximizeLabel={infoLabels.maximize}
		restoreLabel={infoLabels.restore}
		href={infoHref}
		hrefLabel={infoLabels.guide}
		onclose={() => setInfo(false)}
	>
		{#if info}{@render info()}{/if}
	</InfoPanel>
{/snippet}

<style>
	/* the page gets the bottom padding wherever the bar is placed in the markup */
	:global(body:has(.sc-action-bar)) {
		padding-bottom: calc(var(--action-bar-height, var(--tool-bar-height)) + var(--space-1));
	}
	.sc-action-bar {
		position: fixed;
		z-index: 40;
		left: 0;
		right: 0;
		bottom: 0;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-1);
		box-sizing: border-box;
		height: var(--action-bar-height, var(--tool-bar-height));
		padding: 0 var(--space-1);
		background: var(--paper);
		border-top: var(--border-width-regular) solid var(--ink);
		color: var(--ink);
	}
	.start {
		display: flex;
		align-items: center;
		gap: var(--space-075);
		min-width: 0;
		overflow-x: auto;
		scrollbar-width: thin;
	}
	.end {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		flex: none;
		margin-left: auto;
	}
	.more {
		position: relative;
	}
	.info-toggle {
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
		flex: none;
		box-sizing: border-box;
		min-height: var(--control-height);
		padding: 0 var(--space-1);
		border: var(--border-width-regular) solid var(--border-muted);
		line-height: var(--type-button-line);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
		white-space: nowrap;
		cursor: pointer;
	}
	.info-toggle:hover {
		border-color: var(--ink);
	}
	.info-toggle[aria-expanded='true'] {
		border-color: var(--ink);
		background: var(--ink);
		color: var(--paper);
	}
	.more-toggle {
		box-sizing: border-box;
		min-height: var(--control-height);
		padding: 0 var(--space-15);
		border: var(--border-width-regular) solid var(--border-muted);
		line-height: var(--type-button-line);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
		cursor: pointer;
	}
	.more-toggle:hover {
		border-color: var(--ink);
	}
	.more-toggle[aria-expanded='true'] {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
	.info-toggle:focus-visible,
	.more-toggle:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: var(--focus-offset);
	}
	/* phones: the bar keeps Info + More + the primary actions on one line (the label stays the accessible name) */
	@media (max-width: 479px) {
		.sc-action-bar :global(:is(button, a.sc-button)) {
			padding-inline: var(--space-1);
		}
		.info-toggle {
			padding-inline: var(--space-05);
		}
		.info-word {
			position: absolute;
			width: 1px;
			height: 1px;
			overflow: hidden;
			clip-path: inset(50%);
			white-space: nowrap;
		}
	}
	.more-panel {
		position: absolute;
		left: calc(-1 * var(--space-1));
		bottom: calc(100% + var(--space-05) + var(--border-width-regular));
		width: calc(100vw - var(--space-1) * 2);
		max-height: 70dvh;
		overflow: auto;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		box-sizing: border-box;
		margin-left: var(--space-1);
		padding: var(--space-1);
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
	}
	.more-start,
	.more-secondary {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-075);
	}
</style>
