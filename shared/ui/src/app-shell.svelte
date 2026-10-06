<script lang="ts">
	import { dockRegion } from './dock-region.svelte.ts';
	import Divider from './divider.svelte';
	import { untrack, type Snippet } from 'svelte';
	import Icon from './icon.svelte';

	/**
	 * Page frame of the tools (admin, creator): top bar + optional sidebar (left) + content + optional dock (right),
	 * above the fixed bottom `ActionBar` (PLATFORM.md "Bottom action bar"). From 768px the regions tile and never
	 * overlap (agents/DOCKING.md); their borders always reach the ActionBar (or the viewport bottom without one), at any
	 * scroll position and content length:
	 *
	 * - `mode="page"` (default): the document scrolls. The top bar is sticky; sidebar and dock are sticky and exactly
	 *   as tall as the space between the top bar and the ActionBar, and scroll on their own when taller. A short page
	 *   does not scroll at all.
	 * - `mode="fill"`: the shell is exactly the viewport minus the ActionBar and the document never scrolls; sidebar,
	 *   content and dock scroll on their own (editors).
	 *
	 * The shell reserves the ActionBar's height itself (`--action-bar-height`, inherited from the app; default 3.25rem)
	 * instead of the `body` padding the ActionBar adds, so the page never scrolls by a wrong amount (rem-based padding
	 * on a fluid root size). The top bar's height is measured (`--sc-shell-top`), so a wrapping top bar is fine.
	 *
	 * `collapsible`: below 768px the sidebar becomes an off-canvas drawer (`sidebarOpen`, scrim → `onclose`).
	 * `resizeKey`: from 768px on, the sidebar's right edge (a `Divider`) drags it wider (owner 2026-10-02: creator) –
	 * from its normal width up to a third of the window; arrow keys on the handle, double-click resets; remembered per
	 * key in localStorage (`sc-sidebar-width:<key>`).
	 * `foldable` (with `resizeKey`, owner 2026-10-03 "version A"): from 768px on the sidebar folds to a 2.5rem rail –
	 * the fold button (Feather chevrons-left) at its top right, dragging the edge narrower than the normal width, or
	 * Ctrl + \; the rail shows chevrons-right and `railLabel` (the current place) written vertically. Remembered per key
	 * (`sc-sidebar-folded:<key>`). It folds by itself (not remembered) while `foldWhen` holds (who gives way first:
	 * agents/DOCKING.md "Running out of room").
	 * `dock` (+ `dockSize`, a `panelWidth`): a page's panel docked on the right (the creator's Problems / History / Flow)
	 * – a column of the frame, so the content makes room; its left edge is a `Divider`; dragged wider, it stops where the
	 * content would get narrower than `minContent` (default 20rem) next to the sidebar; below its normal width only when
	 * the content would get less than half of that. Below 768px it covers the page.
	 */
	interface DockSize {
		readonly css: string;
		/** CSS length: the normal width – the dock never gives way below it */
		readonly normal?: string;
		/** set by the shell: px a drag may widen it to (the content keeps `minContent`) */
		room?: (() => number) | null;
		readonly width: number | null;
		start(event: PointerEvent): void;
		key(event: KeyboardEvent): void;
		reset(): void;
	}
	let {
		topbar,
		sidebar,
		dock,
		children,
		mode = 'page',
		sidebarWidth = 'var(--tool-sidebar-width)',
		sidebarLabel,
		sidebarId,
		sidebarTag = 'nav',
		collapsible = false,
		sidebarOpen = false,
		onclose,
		closeLabel = 'Close menu',
		resizeKey,
		resizeLabel = 'Resize sidebar',
		foldable = false,
		railLabel = '',
		foldLabel = 'Fold the sidebar',
		unfoldLabel = 'Show the sidebar',
		foldWhen = false,
		minContent,
		dockSize,
		dockLabel,
		dockResizeLabel = 'Resize panel',
		class: className = ''
	}: {
		topbar?: Snippet;
		sidebar?: Snippet;
		/** the page's panel docked on the right */
		dock?: Snippet;
		children: Snippet;
		mode?: 'page' | 'fill';
		/** CSS length; may use `var(…)` so an app can change it per breakpoint */
		sidebarWidth?: string;
		/** accessible name of the sidebar landmark */
		sidebarLabel?: string;
		sidebarId?: string;
		sidebarTag?: 'nav' | 'aside' | 'div';
		collapsible?: boolean;
		sidebarOpen?: boolean;
		onclose?: () => void;
		closeLabel?: string;
		/** makes the sidebar resizable on desktop; the width is remembered under this key */
		resizeKey?: string;
		resizeLabel?: string;
		/** the sidebar can fold to a rail (desktop) */
		foldable?: boolean;
		/** shown vertically on the rail: where the user is */
		railLabel?: string;
		foldLabel?: string;
		unfoldLabel?: string;
		/** fold to the rail while true (the page needs the room, e.g. two panels docked on the right); not remembered –
		 * unfolding by hand wins until it turns false again */
		foldWhen?: boolean;
		/** rem: the content's minimum width – a dock dragged wider stops there */
		minContent?: number;
		/** the dock's width (`panelWidth`: its `css` length, the drag / keys / reset of its edge) */
		dockSize?: DockSize;
		/** accessible name of the dock */
		dockLabel?: string;
		dockResizeLabel?: string;
		class?: string;
	} = $props();

	let topHeight = $state(0);

	// ---------- the sidebar: resizable, folds to a rail (dock-region.svelte.ts) ----------
	let sidebarEl = $state<HTMLElement>();
	/** the normal width in px (the minimum) – measured before the first drag */
	let baseWidth = 0;
	function measureBase() {
		if (!baseWidth && sidebarEl && region.width === null && !region.folded) baseWidth = sidebarEl.offsetWidth;
		return baseWidth || 240;
	}
	const widthKey = $derived(resizeKey ? `sc-sidebar-width:${resizeKey}` : '');
	const foldKey = $derived(foldable && resizeKey ? `sc-sidebar-folded:${resizeKey}` : '');
	const region = dockRegion({
		key: () => widthKey,
		foldKey: () => foldKey,
		side: 'left',
		anchor: () => sidebarEl?.getBoundingClientRect().left ?? 0,
		normal: measureBase,
		max: () => window.innerWidth / 3,
		foldWhen: () => foldWhen
	});
	const folded = $derived(region.folded);

	$effect(() => {
		if (!foldKey) return;
		const key = (e: KeyboardEvent) => {
			if ((e.ctrlKey || e.metaKey) && e.key === '\\') {
				e.preventDefault();
				region.fold(!region.folded);
			}
		};
		window.addEventListener('keydown', key);
		return () => window.removeEventListener('keydown', key);
	});

	$effect(() => {
		if (!widthKey || !sidebarEl) return;
		// measured before a saved width applies, never the rail; untracked: folding must not re-run this restore
		if (!untrack(() => region.folded)) baseWidth = sidebarEl.offsetWidth;
		untrack(() => region.restore());
	});

	/** a remembered width wider than a third of a smaller window shrinks with it */
	$effect(() => {
		if (!resizeKey) return;
		const fit = () => region.fit();
		window.addEventListener('resize', fit);
		return () => window.removeEventListener('resize', fit);
	});

	let frameEl = $state<HTMLElement>();
	// a drag stops where the content would get narrower than its minimum (the CSS below keeps it so on a smaller window)
	$effect(() => {
		const size = dockSize;
		if (!size || !frameEl) return;
		const frame = frameEl;
		size.room = () => frame.clientWidth - (minContent ?? 20) * (parseFloat(getComputedStyle(frame).fontSize) || 16) - (sidebarEl?.offsetWidth ?? 0);
		return () => (size.room = null);
	});
	/** the dock gives way (agents/DOCKING.md "Running out of room"): dragged wider than leaves the content its minimum
	 * (next to the sidebar) it stops there; its normal width stays unless even that leaves the content less than half its
	 * minimum */
	const dockCss = $derived.by(() => {
		if (!dockSize) return '0rem';
		const min = minContent ?? 20;
		const room = (keep: number) => `calc(100% - ${keep}rem - ${sidebar ? 'var(--sc-shell-sidebar-width)' : '0rem'})`;
		return `min(${dockSize.css}, max(${room(min)}, min(${dockSize.normal ?? '0rem'}, ${room(min / 2)})))`;
	});
</script>

<div
	class="sc-app-shell {mode} {className}"
	class:has-sidebar={!!sidebar}
	class:has-dock={!!dock}
	class:collapsible
	style:--sc-shell-top="{topHeight}px"
	class:folded={folded && !!foldKey}
	style:--sc-shell-sidebar-width={folded && foldKey ? 'var(--sc-shell-rail-width)' : region.width !== null ? `max(${region.width}px, ${sidebarWidth})` : sidebarWidth}
	style:--sc-shell-dock-width={dockCss}
	style:--sc-shell-dock-own={dockSize?.css}
>
	{#if topbar}
		<div class="sc-shell-top" bind:offsetHeight={topHeight}>{@render topbar()}</div>
	{/if}
	<div class="sc-shell-frame" bind:this={frameEl}>
		{#if sidebar}
			<svelte:element
				this={sidebarTag}
				id={sidebarId}
				class="sc-shell-sidebar"
				class:open={sidebarOpen}
				bind:this={sidebarEl}
				aria-label={sidebarLabel}
			>
				{#if foldKey && folded}
					<!-- the rail: unfold + where the user is -->
					<div class="sc-shell-rail">
						<button type="button" class="sc-shell-fold sc-round" onclick={() => region.fold(false)} aria-label={unfoldLabel} aria-expanded="false" aria-controls={sidebarId} title="{unfoldLabel} (Ctrl + \)"><Icon name="chevrons-right" /></button>
						{#if railLabel}<span class="sc-shell-rail-label">{railLabel}</span>{/if}
					</div>
				{:else}
					{#if foldKey}
						<button type="button" class="sc-shell-fold sc-shell-fold-in sc-round" onclick={() => region.fold(true)} aria-label={foldLabel} aria-expanded="true" aria-controls={sidebarId} title="{foldLabel} (Ctrl + \)"><Icon name="chevrons-left" /></button>
					{/if}
					{@render sidebar()}
				{/if}
			</svelte:element>
			{#if resizeKey}
				<Divider size={region} edge="right" at="var(--sc-shell-sidebar-width)" label={resizeLabel} controls={sidebarId} class="sc-shell-resize" />
			{/if}
			{#if collapsible && sidebarOpen}
				<button type="button" class="sc-shell-scrim" aria-label={closeLabel} onclick={() => onclose?.()}></button>
			{/if}
		{/if}
		<div class="sc-shell-content">{@render children()}</div>
		{#if dock}
			<aside class="sc-shell-dock" id="sc-shell-dock" aria-label={dockLabel} data-testid="shell-dock">{@render dock()}</aside>
			{#if dockSize}
				<Divider size={dockSize} edge="left" at="var(--sc-shell-dock-width)" label={dockResizeLabel} controls="sc-shell-dock" />
			{/if}
		{/if}
	</div>
</div>

<style>
	/* The shell reserves the ActionBar itself (see below), so the body keeps no padding of its own. More specific than
	   the ActionBar's `body:has(.sc-action-bar)`. */
	:global(html body:has(.sc-app-shell)) {
		padding-bottom: 0;
	}
	.sc-app-shell {
		--sc-shell-bottom: 0px;
		--sc-shell-height: calc(100dvh - var(--sc-shell-top) - var(--sc-shell-bottom));
		display: flex;
		flex-direction: column;
		box-sizing: border-box;
		min-height: 100dvh;
		padding-bottom: var(--sc-shell-bottom);
	}
	.sc-app-shell:has(:global(.sc-action-bar)) {
		--sc-shell-bottom: var(--action-bar-height, var(--tool-bar-height));
	}
	.sc-shell-top {
		position: sticky;
		top: 0;
		z-index: 20;
		flex: none;
		background: var(--paper);
	}
	.sc-shell-frame {
		position: relative;
		flex: 1 0 auto;
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		min-height: 0;
	}
	.has-sidebar .sc-shell-frame {
		grid-template-columns: var(--sc-shell-sidebar-width) minmax(0, 1fr);
	}
	.has-dock .sc-shell-frame {
		grid-template-columns: minmax(0, 1fr) var(--sc-shell-dock-width);
	}
	.has-sidebar.has-dock .sc-shell-frame {
		grid-template-columns: var(--sc-shell-sidebar-width) minmax(0, 1fr) var(--sc-shell-dock-width);
	}
	.sc-shell-content {
		min-width: 0;
	}
	.sc-shell-sidebar,
	.sc-shell-dock {
		position: sticky;
		top: var(--sc-shell-top);
		align-self: start;
		box-sizing: border-box;
		height: var(--sc-shell-height);
		overflow: auto;
		overscroll-behavior: contain;
		background: var(--paper);
	}
	.sc-shell-sidebar {
		border-right: var(--border-width-regular) solid var(--ink);
	}
	/* the dock: one page panel; its content fills it and scrolls on its own (or pans, like the Flow) */
	.sc-shell-dock {
		display: flex;
		flex-direction: column;
		overflow: hidden;
		border-left: var(--border-width-regular) solid var(--ink);
	}
	.sc-shell-dock > :global(*) {
		flex: 1 1 auto;
		min-height: 0;
	}

	/* fill: exactly the viewport minus the ActionBar; every column scrolls on its own */
	.sc-app-shell.fill {
		height: 100dvh;
		min-height: 0;
		overflow: hidden;
	}
	.fill .sc-shell-top {
		position: static;
	}
	.fill .sc-shell-frame {
		flex: 1 1 auto;
		grid-template-rows: minmax(0, 1fr);
	}
	.fill .sc-shell-sidebar,
	.fill .sc-shell-dock {
		position: static;
		align-self: stretch;
		height: auto;
		min-height: 0;
	}
	.fill .sc-shell-content {
		min-height: 0;
		overflow: auto;
	}

	/* the folded sidebar: a rail (foldable) */
	.sc-app-shell {
		--sc-shell-rail-width: 2.5rem;
	}
	.sc-shell-rail {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.75rem;
		padding-top: 1rem;
		height: 100%;
	}
	.sc-shell-rail-label {
		writing-mode: vertical-rl;
		transform: rotate(180deg);
		color: var(--muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-height: calc(100% - 4rem);
	}
	/* its own row, 1rem from the top, at the right (owner 2026-10-03: like the script outline's fold button) */
	.sc-shell-fold-in {
		display: flex;
		margin: 1rem 0.5rem 0 auto;
	}
	.folded .sc-shell-sidebar {
		overflow: hidden;
	}

	.sc-shell-scrim {
		display: none;
	}
	@media (max-width: 767px) {
		.collapsible.has-sidebar .sc-shell-frame {
			grid-template-columns: minmax(0, 1fr);
		}
		/* phones: the dock covers the page (closed by its own button) */
		.has-dock .sc-shell-frame,
		.has-sidebar.has-dock .sc-shell-frame {
			grid-template-columns: minmax(0, 1fr);
		}
		.collapsible.has-sidebar.has-dock .sc-shell-frame {
			grid-template-columns: minmax(0, 1fr);
		}
		.sc-shell-dock,
		.fill .sc-shell-dock {
			position: fixed;
			z-index: 20;
			top: var(--sc-shell-top);
			right: 0;
			bottom: var(--sc-shell-bottom);
			width: min(var(--sc-shell-dock-own, 100vw), 100vw);
			height: auto;
			/* `start` would shrink a fixed box with both insets to its content */
			align-self: stretch;
		}
		.sc-shell-fold {
			display: none;
		}
		.collapsible .sc-shell-sidebar {
			position: fixed;
			z-index: 30;
			top: var(--sc-shell-top);
			left: 0;
			bottom: var(--sc-shell-bottom);
			width: min(18rem, 85vw);
			height: auto;
			/* `start` would shrink a fixed box with both insets to its content */
			align-self: stretch;
			transform: translateX(-100%);
			transition: transform var(--duration-base) var(--easing);
			visibility: hidden;
		}
		.collapsible .sc-shell-sidebar.open {
			transform: none;
			visibility: visible;
		}
		.sc-shell-scrim {
			display: block;
			position: fixed;
			z-index: 25;
			inset: var(--sc-shell-top) 0 var(--sc-shell-bottom) 0;
			border: 0;
			background: rgb(0 0 0 / 0.4);
		}
	}
</style>
