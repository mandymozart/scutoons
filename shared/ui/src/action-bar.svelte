<script lang="ts">
	import type { Snippet } from 'svelte';
	import { clickOutside } from './click-outside.ts';
	import InfoPanel from './info-panel.svelte';
	import Icon from './icon.svelte';
	import { panelWidth } from './panel-width.svelte.ts';
	import { useDock } from './dock.svelte.ts';

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
	 * agents/DOCKING.md – next to the page's own panels, outermost; one remembered width `sc-panel-width:info`).
	 * Closed by default; open/closed is remembered in localStorage (`infoKey`), so it stays open across pages until the
	 * user closes it.
	 *
	 * With `phone` the bar is that snippet – the page's PhoneBar (grouped actions; on phones also the selection's actions,
	 * like CapCut): on phones alone and full width (< 768px, UI-CONCEPTS E.10, owner 2026-10-11), at every other width too
	 * with the Info toggle left of it (owner 2026-10-12: the phone grouping on the desktop); `start` / `secondary` / `end`
	 * are then not shown (the page puts what it needs into its PhoneBar). Without it, More (narrow screens) is the ⋯ icon.
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
		infoKey = 'sc-info-open',
		phone
	}: {
		/** phones (< 768px): the whole bar – the page's PhoneBar */
		phone?: Snippet;
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
	let phoneWide = $state(false);
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
	$effect(() => {
		const query = window.matchMedia('(max-width: 767px)');
		phoneWide = query.matches;
		const change = (event: MediaQueryListEvent) => (phoneWide = event.matches);
		query.addEventListener('change', change);
		return () => query.removeEventListener('change', change);
	});
	/** the grouped bar (a page with `phone` actions – every width since 2026-10-12) */
	const compact = $derived(!!phone);

	const hasMore = $derived(!!start || !!secondary);


	// the Info panel: closed by default, the user's choice is remembered (storage may be unavailable)
	$effect(() => {
		if (!info) return;
		try {
			// next to the page's own panels (10.57); the last page's Info held across the navigation is taken over
			// (agents/DOCKING.md "Panels across pages")
			infoOpen = localStorage.getItem(infoKey) === '1';
		} catch {
			/* private mode etc. */
		}
	});

	// docked in the shell's right dock while open, next to the page's own panels (10.57). The shell mounts it (`kind` + `content`): the next page's Info takes the same panel over – only its text changes
	const infoSize = panelWidth('sc-panel-width:info', 16, undefined, { foldable: true });
	const infoProps = () => ({
		id: `${uid}-info`,
		title: infoTitle,
		closeLabel: infoLabels.close,
		foldLabel: infoLabels.fold,
		maximizeLabel: infoLabels.maximize,
		restoreLabel: infoLabels.restore,
		href: infoHref,
		hrefLabel: infoLabels.guide,
		onclose: () => setInfo(false),
		children: info
	});
	useDock(() =>
		info && infoOpen ? { kind: 'info', content: { component: InfoPanel, props: infoProps }, size: infoSize, title: infoTitle, icon: 'info', close: () => (infoOpen = false) } : null
	);

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
		title={infoLabels.toggle ?? 'Info'}
		aria-expanded={infoOpen}
		aria-controls="{uid}-info"
		onclick={() => setInfo(!infoOpen)}
		data-testid="info-toggle"
	>
		<Icon name="info" /> <span class="info-word">{infoLabels.toggle ?? 'Info'}</span>
	</button>
{/snippet}

{#snippet moreToggle()}
	<button
		type="button"
		class="more-toggle"
		aria-expanded={moreOpen}
		aria-controls="{uid}-more"
		aria-label={moreLabel}
		title={moreLabel}
		onclick={() => (moreOpen = !moreOpen)}
		data-testid="more-toggle"
	>
		<Icon name="more-horizontal" />
	</button>
{/snippet}

<div class="sc-action-bar" class:compact role="region" aria-label={label}>
	{#if compact}
		{#if info && !phoneWide}{@render infoToggle()}{/if}
		{@render phone?.()}
	{:else if narrow}
		{#if info}{@render infoToggle()}{/if}
		{#if hasMore}
			<div class="more" {@attach clickOutside(() => (moreOpen = false))}>
				{@render moreToggle()}
				{#if moreOpen}
					<!-- phones: the page under the open panel is dimmed (a tab row peeking above it read as part of it – BACKLOG 10.50 c) -->
					<button type="button" class="more-scrim" tabindex="-1" data-control-exempt="scrim" aria-label={moreLabel} onclick={() => (moreOpen = false)}></button>
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
	/* the end may shrink: a flexible item in it (a note field) gives way before anything is pushed past the edge */
	.end {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: var(--space-05);
		flex: 0 1 auto;
		min-width: 0;
		margin-left: auto;
	}
	.end > :global(*) {
		flex-shrink: 0;
	}
	.end > :global(input) {
		flex-shrink: 1;
	}
	.more {
		position: relative;
	}
	.info-toggle {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-05);
		flex: none;
		box-sizing: border-box;
		min-height: var(--control-height);
		min-width: var(--control-height);
		padding: 0;
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
		display: inline-flex;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		min-height: var(--control-height);
		min-width: var(--control-height);
		padding: 0 var(--space-05);
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
	}
	/* the bar's icons carry no words, at every width (owner 2026-10-21: "so please remove everywhere"): the word is the
	   accessible name, the tooltip shows it */
	.info-word {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
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
	.more-scrim {
		display: none;
	}
	/* phones: the panel is a sheet on the bar – the full width, flush on its top rule, the page above dimmed */
	@media (max-width: 767px) {
		.more-scrim {
			display: block;
			position: fixed;
			inset: 0 0 var(--action-bar-height, var(--tool-bar-height)) 0;
			min-height: 0;
			padding: 0;
			border: 0;
			background: rgb(0 0 0 / 0.4);
			cursor: default;
		}
		.more-panel {
			position: fixed;
			left: 0;
			right: 0;
			bottom: calc(var(--action-bar-height, var(--tool-bar-height)) - var(--border-width-regular));
			width: auto;
			margin: 0;
			border-width: var(--border-width-regular) 0;
		}
	}
	/* phones with a PhoneBar: the bar is that row, edge to edge */
	@media (max-width: 767px) {
		.compact {
			padding: 0;
		}
	}
	.more-start,
	.more-secondary {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-075);
	}
</style>
