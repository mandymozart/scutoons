<script lang="ts" module>
	export interface BreadcrumbItem {
		href: string;
		label: string;
	}
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';
	import { clickOutside } from './click-outside.ts';

	/**
	 * The breadcrumb trail (UI-CONCEPTS E.2, DESIGN.md "Breadcrumbs"): every crumb a link with its FULL text (never a
	 * per-crumb ellipsis). When the trail doesn't fit, the crumbs from the left (after the root) collapse into ONE "…"
	 * button – a text control, square – that opens a small menu of them (Enter / Space / ↓ open it, ↑ ↓ Home End move,
	 * Esc / Tab close). The last `keep` crumbs never collapse; when even they don't fit, the trail wraps. Measured with a ResizeObserver (the nav's width and an
	 * invisible copy of the crumbs, so a label change measures again).
	 */
	let {
		items,
		label,
		moreLabel,
		root,
		keep = 2,
		plain = false,
		testid = 'crumb'
	}: {
		items: BreadcrumbItem[];
		/** the nav's accessible name ("Breadcrumb") */
		label: string;
		/** the "…" button's tooltip / name ("Show the levels above") */
		moreLabel: string;
		/** the first item is the root (e.g. the logo): rendered by this snippet inside its link, never collapses */
		root?: Snippet<[BreadcrumbItem]>;
		/** how many crumbs at the end never collapse */
		keep?: number;
		/** a path (the creator, CREATOR-NAV.md §2a): no underlines, the levels above muted, `/` between */
		plain?: boolean;
		/** data-testid prefix: `<testid>-<index>` per crumb, `<testid>-more` for the "…" button, `<testid>-menu` */
		testid?: string;
	} = $props();

	const uid = $props.id();
	let nav: HTMLElement | null = $state(null);
	let measure: HTMLElement | null = $state(null);
	let moreButton: HTMLButtonElement | null = $state(null);
	let popup: HTMLElement | null = $state(null);
	let open = $state(false);
	/** how many crumbs (after the root) are collapsed into "…" */
	let collapsed = $state(0);
	/** even with every crumb collapsed the trail is too wide (a long group name): it wraps – nothing is cut */
	let tight = $state(false);
	/** the popup's left edge under the "…" button (rem) */
	let popupLeft = $state(0);

	const first = $derived(root ? 1 : 0);
	/** the crumbs that may collapse: after the root, before the last `keep` */
	const collapsible = $derived(Math.max(0, items.length - first - keep));
	const hidden = $derived(items.slice(first, first + Math.min(collapsed, collapsible)));

	function fit() {
		if (!nav || !measure) return;
		const available = nav.clientWidth;
		const widths = [...measure.querySelectorAll<HTMLElement>('[data-measure="crumb"]')].map((el) => el.getBoundingClientRect().width);
		const sep = measure.querySelector<HTMLElement>('[data-measure="sep"]')?.getBoundingClientRect().width ?? 0;
		const more = measure.querySelector<HTMLElement>('[data-measure="more"]')?.getBoundingClientRect().width ?? 0;
		if (widths.length !== items.length) return;
		const width = (shown: number[], withMore: boolean) => {
			const parts = shown.length + (withMore ? 1 : 0);
			return shown.reduce((a, b) => a + b, 0) + (withMore ? more : 0) + Math.max(0, parts - 1) * sep;
		};
		// a pixel of slack: sub-pixel widths must not make the last crumb wrap
		if (width(widths, false) <= available + 0.5) {
			collapsed = 0;
			tight = false;
			return;
		}
		let c = 1;
		const rest = (n: number) => width([...widths.slice(0, first), ...widths.slice(first + n)], n > 0);
		while (c < collapsible && rest(c) > available + 0.5) c++;
		collapsed = Math.min(c, collapsible);
		tight = rest(collapsed) > available + 0.5;
	}

	$effect(() => {
		if (!nav || !measure) return;
		void items.length;
		void collapsible;
		const observer = new ResizeObserver(() => fit());
		observer.observe(nav);
		observer.observe(measure);
		fit();
		return () => observer.disconnect();
	});
	// the labels changed (same widths can't be assumed): measure after the copy rendered
	$effect(() => {
		void items.map((i) => i.label).join('\u0000');
		queueMicrotask(fit);
	});
	// nothing collapsed any more: the menu has nothing to show
	$effect(() => {
		if (!hidden.length) open = false;
	});

	const menuItems = () => [...(popup?.querySelectorAll<HTMLElement>('[role="menuitem"]') ?? [])];

	function openMenu(focus: 'first' | 'last' | null) {
		if (!hidden.length) return;
		if (moreButton && nav) {
			const rootSize = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
			popupLeft = (moreButton.getBoundingClientRect().left - nav.getBoundingClientRect().left) / rootSize;
		}
		open = true;
		if (focus) queueMicrotask(() => (focus === 'first' ? menuItems()[0] : menuItems().at(-1))?.focus());
	}

	function close(refocus: boolean) {
		if (!open) return;
		open = false;
		if (refocus) moreButton?.focus();
	}

	function onMoreKeydown(event: KeyboardEvent) {
		if (event.altKey || event.ctrlKey || event.metaKey) return;
		if (event.key === 'ArrowDown' || event.key === 'Enter' || event.key === ' ') {
			event.preventDefault();
			openMenu('first');
		} else if (event.key === 'ArrowUp') {
			event.preventDefault();
			openMenu('last');
		} else if (event.key === 'Escape' && open) {
			event.preventDefault();
			close(true);
		}
	}

	function onMenuKeydown(event: KeyboardEvent) {
		if (event.altKey || event.ctrlKey || event.metaKey) return;
		const list = menuItems();
		const at = list.indexOf(document.activeElement as HTMLElement);
		const go = (i: number) => {
			event.preventDefault();
			list[(i + list.length) % list.length]?.focus();
		};
		if (event.key === 'ArrowDown') go(at + 1);
		else if (event.key === 'ArrowUp') go(at - 1);
		else if (event.key === 'Home') go(0);
		else if (event.key === 'End') go(list.length - 1);
		else if (event.key === 'Escape') {
			event.preventDefault();
			event.stopPropagation();
			close(true);
		} else if (event.key === 'Tab') close(false);
	}
</script>

{#snippet more()}
	<li class="more">
		<button
			bind:this={moreButton}
			type="button"
			class="more-button"
			aria-haspopup="menu"
			aria-expanded={open}
			aria-controls="{uid}-menu"
			aria-label={moreLabel}
			title={moreLabel}
			data-testid="{testid}-more"
			onclick={() => (open ? close(false) : openMenu(null))}
			onkeydown={onMoreKeydown}>…</button
		>
	</li>
{/snippet}

<nav class="sc-breadcrumbs" class:plain aria-label={label} bind:this={nav} {@attach clickOutside(() => close(false), { escape: false })}>
	<ol class:tight>
		{#each items as item, i (i)}
			{#if i === first && hidden.length}{@render more()}{/if}
			{#if i < first || i >= first + hidden.length}
				<li class:root={root && i === 0}>
					<a href={item.href} aria-current={i === items.length - 1 ? 'page' : undefined} data-testid="{testid}-{i}">
						{#if root && i === 0}{@render root(item)}{:else}{item.label}{/if}
					</a>
				</li>
			{/if}
		{/each}
	</ol>
	{#if open && hidden.length}
		<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
		<ul class="menu" role="menu" id="{uid}-menu" aria-label={moreLabel} bind:this={popup} style:left="{popupLeft}rem" data-testid="{testid}-menu" onkeydown={onMenuKeydown} onclick={() => close(false)}>
			{#each hidden as item, i (i)}
				<li role="none"><a role="menuitem" href={item.href} data-testid="{testid}-{first + i}">{item.label}</a></li>
			{/each}
		</ul>
	{/if}
	<!-- the measuring copy: every crumb at its full width, a separator and the "…" button (invisible, not read out) -->
	<div class="measure-clip" aria-hidden="true">
	<div class="measure" bind:this={measure}>
		{#each items as item, i (i)}
			<span data-measure="crumb">{#if root && i === 0}{@render root(item)}{:else}{item.label}{/if}</span>
		{/each}
		<span class="sep" data-measure="sep"></span>
		<span class="more-button" data-measure="more">…</span>
	</div>
	</div>
</nav>

<style>
	.sc-breadcrumbs {
		position: relative;
		flex: 1 1 auto;
		min-width: 0;
	}
	ol {
		display: flex;
		align-items: center;
		margin: 0;
		padding: 0;
		list-style: none;
		white-space: nowrap;
	}
	/* the kept crumbs alone are too wide: the trail wraps (every crumb stays whole and readable) */
	ol.tight {
		flex-wrap: wrap;
		row-gap: var(--space-025);
	}
	li {
		display: flex;
		flex: none;
		align-items: center;
	}
	/* the separator between two crumbs (and in the measuring copy) */
	li + li::before,
	.sep::before {
		content: '›';
		padding: 0 var(--space-025);
		color: var(--text-muted);
	}
	a {
		display: flex;
		align-items: center;
		color: var(--ink);
	}
	a[aria-current='page'] {
		text-decoration: none;
	}
	/* touch screens (BACKLOG 10.50 d): every crumb a full-height target (they were 20px high on a phone) */
	@media (pointer: coarse) {
		a {
			min-height: var(--control-height);
		}
	}
	.root a {
		text-decoration: none;
	}
	/* a path: links all the same, without underlines; the current place ink, the levels above muted */
	.plain a {
		color: var(--text-muted);
		text-decoration: none;
	}
	.plain a:hover,
	.plain a[aria-current='page'] {
		color: var(--ink);
	}
	.plain .more-button {
		min-width: 0;
		border-color: transparent;
		padding: 0 var(--space-025);
	}
	.plain li + li::before,
	.plain .sep::before {
		content: '/';
		padding: 0 var(--space-05);
		color: var(--border-muted);
	}
	/* "…": a text control – square (DESIGN.md "Round vs square") */
	.more-button {
		box-sizing: border-box;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: var(--control-height);
		min-height: var(--control-height);
		margin: 0;
		padding: 0 var(--space-05);
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--border-muted);
		border-radius: var(--radius-none);
		font: inherit;
		line-height: 1;
		cursor: pointer;
	}
	button.more-button:hover,
	button.more-button[aria-expanded='true'] {
		border-color: var(--ink);
	}
	button.more-button:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: var(--focus-offset);
	}
	.menu {
		position: absolute;
		top: calc(100% + var(--space-025));
		z-index: 100;
		display: flex;
		flex-direction: column;
		min-width: 12rem;
		max-width: min(24rem, calc(100vw - 2rem));
		margin: 0;
		padding: var(--space-025) 0;
		list-style: none;
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
		border-radius: var(--radius-none);
	}
	.menu li {
		display: block;
	}
	.menu li + li::before {
		content: none;
	}
	.menu a {
		box-sizing: border-box;
		width: 100%;
		min-height: var(--control-height);
		padding: 0 var(--space-1);
		text-decoration: none;
		white-space: nowrap;
	}
	/* hover frames, focus = the 3px ring (DESIGN.md "Selection") – inside the popup's frame */
	.menu a:hover {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
	.menu a:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-focus));
	}
	/* the copy is clipped to nothing, so it never widens the page */
	.measure-clip {
		position: absolute;
		top: 0;
		left: 0;
		width: 0;
		height: 0;
		overflow: hidden;
		visibility: hidden;
		pointer-events: none;
	}
	.measure {
		display: flex;
		align-items: center;
		width: max-content;
		white-space: nowrap;
	}
	.measure > span {
		display: inline-flex;
		flex: none;
		align-items: center;
	}
</style>
