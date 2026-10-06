<script lang="ts">
	import { dockRegion } from './dock-region.svelte.ts';
	import { shellDock } from './dock.svelte.ts';
	import Divider from './divider.svelte';
	import FoldButton from './fold-button.svelte';
	import Rail from './rail.svelte';
	import { untrack, type Snippet } from 'svelte';

	/**
	 * Page frame of the tools (admin, creator, embassy): top bar + optional sidebar (left) + content + the right dock +
	 * Ask (outermost right), above the fixed bottom `ActionBar` (PLATFORM.md "Bottom action bar"). From 768px the regions
	 * tile and never overlap (agents/DOCKING.md); their borders always reach the ActionBar (or the viewport bottom without
	 * one), at any scroll position and content length:
	 *
	 * - `mode="page"` (default): the document scrolls. The top bar is sticky; sidebar, dock and Ask are sticky and exactly
	 *   as tall as the space between the top bar and the ActionBar, and scroll on their own when taller. A short page
	 *   does not scroll at all.
	 * - `mode="fill"`: the shell is exactly the viewport minus the ActionBar and the document never scrolls; every region
	 *   scrolls on its own (editors).
	 *
	 * The shell reserves the ActionBar's height itself (`--action-bar-height`, inherited from the app; default 3.25rem)
	 * instead of the `body` padding the ActionBar adds, so the page never scrolls by a wrong amount (rem-based padding
	 * on a fluid root size). The top bar's height is measured (`--sc-shell-top`), so a wrapping top bar is fine.
	 *
	 * `collapsible`: below 768px the sidebar becomes an off-canvas drawer (`sidebarOpen`, scrim → `onclose`).
	 * `resizeKey`: from 768px on, the sidebar's right edge (a `Divider`) drags it wider (owner 2026-10-02: creator) –
	 * from its normal width up to a third of the window; arrow keys on the handle, double-click resets; remembered per
	 * key in localStorage (`sc-sidebar-width:<key>`).
	 * `foldable` (with `resizeKey`, owner 2026-10-03 "version A"): from 768px on the sidebar folds to a `Rail` –
	 * the `FoldButton` (chevrons-left) at its top right, dragging the edge narrower than the normal width, or Ctrl + \;
	 * the rail shows `railLabel` (the current place) written vertically. Remembered per key (`sc-sidebar-folded:<key>`).
	 * It folds by itself (not remembered) while an unfolded dock AND Ask are open, or while `foldWhen` holds (who gives way first:
	 * agents/DOCKING.md "Running out of room").
	 * The right dock: the ONE page panel opened with `useDock` (dock.svelte.ts – Problems / History / Flow, Info, a list's
	 * detail) – a column of the frame, so the content makes room; its left edge is a `Divider`; dragged wider, it stops
	 * where the content would get narrower than `minContent` (default 20rem) next to the sidebar; below its normal width
	 * only when the content would get less than half of that. It folds to a rail (owner 2026-10-06: the panel's
	 * `DockFold` button or dragging it narrow; the rail shows the panel's icon and title, a click unfolds it). Below
	 * 768px it covers the page.
	 * Maximize (owner 2026-10-06, `shellDock.maximized`, agents/DOCKING.md "Maximize"): the docked panel (`'dock'`) or a
	 * region of the main area (its id, e.g. the slice inspector – the page hides the rest of its main area) fills
	 * everything between the sidebar (folded to its rail) and Ask. The other regions are only hidden (`visibility`, laid out
	 * as before – nothing unmounts: their widths, folds, scroll positions and unsaved edits stay). The panel's
	 * `MaximizeButton`, Ctrl + Space (the focused region's, else the open dock's) and Esc restore. Not on phones.
	 * `ask`: the app-wide Ask panel (@scutoons/ask) – the outermost region on the right; it sizes itself.
	 */
	let {
		topbar,
		sidebar,
		ask,
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
		dockLabel,
		dockResizeLabel = 'Resize panel',
		dockUnfoldLabel = 'Show the panel',
		class: className = ''
	}: {
		topbar?: Snippet;
		sidebar?: Snippet;
		/** the Ask panel (outermost right) while it is open */
		ask?: Snippet;
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
		/** fold to the rail while true (the page needs the room); not remembered – unfolding by hand wins until it turns
		 * false again. A dock and Ask open together always fold it. */
		foldWhen?: boolean;
		/** rem: the content's minimum width – a dock dragged wider stops there */
		minContent?: number;
		/** accessible name of the dock */
		dockLabel?: string;
		dockResizeLabel?: string;
		/** the dock's rail button */
		dockUnfoldLabel?: string;
		class?: string;
	} = $props();

	let topHeight = $state(0);
	const dock = $derived(shellDock.current);
	/** the maximized region (the dock only while it shows a panel, unfolded) */
	const maximized = $derived(shellDock.maximized === 'dock' ? (dock && !dock.size.folded ? 'dock' : null) : shellDock.maximized);

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
		// who gives way first (agents/DOCKING.md "Running out of room"): a dock and Ask both open fold the sidebar (a
		// dock folded to its rail needs no room)
		foldWhen: () => foldWhen || (!!ask && !!dock && !dock.size.folded) || !!maximized
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

	// ---------- maximize: Esc and Ctrl + Space (agents/DOCKING.md "Maximize") ----------
	/** a dialog, popover, menu or an expanded combobox is open: its Escape is its own */
	const OVERLAYS = 'dialog[open], [aria-modal="true"], [role="menu"], [role="combobox"][aria-expanded="true"], [aria-haspopup]:not([aria-haspopup="false"])[aria-expanded="true"]';
	const overlayOpen = () => {
		let found: NodeListOf<HTMLElement>;
		try {
			found = document.querySelectorAll<HTMLElement>(`${OVERLAYS}, :popover-open`);
		} catch {
			found = document.querySelectorAll<HTMLElement>(OVERLAYS);
		}
		// only what shows (the command palette's combobox waits in its closed dialog)
		return [...found].some((el) => (el.checkVisibility ? el.checkVisibility() : el.offsetParent !== null));
	};
	const editable = (el: Element | null) => !!el?.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"])');
	/** focus before a keyboard maximize: it comes back on a keyboard restore (the hidden region can't keep it) */
	let focusBefore: HTMLElement | null = null;
	function restoreByKey() {
		shellDock.restore();
		const el = focusBefore;
		focusBefore = null;
		if (el?.isConnected) requestAnimationFrame(() => el.focus({ preventScroll: true }));
	}
	$effect(() => {
		if (!shellDock.maximized) focusBefore = null;
	});
	$effect(() => {
		/** before anything handles the key (an overlay closing on Escape would be gone in the bubble phase) */
		let blocked = false;
		const capture = (e: KeyboardEvent) => {
			blocked = e.key === 'Escape' && overlayOpen();
		};
		const key = (e: KeyboardEvent) => {
			if (e.defaultPrevented || e.isComposing || window.innerWidth < 768) return;
			if (e.key === 'Escape') {
				// a dialog's, a popover's, a field's Escape is its own
				if (!shellDock.maximized || blocked || editable(document.activeElement)) return;
				e.preventDefault();
				restoreByKey();
			} else if (e.code === 'Space' && (e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey) {
				if (shellDock.maximized) {
					e.preventDefault();
					restoreByKey();
					return;
				}
				if (overlayOpen()) return;
				const active = document.activeElement;
				if (active?.closest('.sc-shell-ask')) return;
				// the focused region; from anywhere else the open dock panel
				const target = active?.closest('[data-sc-maximize]')?.getAttribute('data-sc-maximize') || (dock && !dock.size.folded ? 'dock' : null);
				if (!target) return;
				e.preventDefault();
				shellDock.maximize(target);
				// focus outside the maximized region would be hidden: it goes to the region's button
				const region = document.querySelector(`[data-sc-maximize="${CSS.escape(target)}"]`);
				focusBefore = active instanceof HTMLElement && active !== document.body ? active : null;
				if (region && !region.contains(active)) requestAnimationFrame(() => region.querySelector<HTMLElement>('.sc-maximize-button')?.focus());
				else focusBefore = null;
			}
		};
		window.addEventListener('keydown', capture, true);
		window.addEventListener('keydown', key);
		return () => {
			window.removeEventListener('keydown', capture, true);
			window.removeEventListener('keydown', key);
		};
	});

	// ---------- the right dock + Ask ----------
	let frameEl = $state<HTMLElement>();
	/** px: Ask's width (it sizes itself) */
	let askWidth = $state(0);
	// a drag stops where the content would get narrower than its minimum (the CSS below keeps it so on a smaller window)
	$effect(() => {
		const size = dock?.size;
		if (!size || !frameEl) return;
		const frame = frameEl;
		size.room = () =>
			frame.clientWidth - (minContent ?? 20) * (parseFloat(getComputedStyle(frame).fontSize) || 16) - (sidebarEl?.offsetWidth ?? 0) - (ask ? askWidth : 0);
		return () => (size.room = null);
	});
	const dockFolded = $derived(!!dock?.size.folded);
	/** the dock gives way (agents/DOCKING.md "Running out of room"): dragged wider than leaves the content its minimum
	 * (next to the sidebar) it stops there; its normal width stays unless even that leaves the content less than half its
	 * minimum */
	const dockCss = $derived.by(() => {
		if (!dock) return '0rem';
		if (dockFolded) return 'var(--sc-shell-rail-width)';
		const min = minContent ?? 20;
		const room = (keep: number) => `calc(100% - ${keep}rem - ${sidebar ? 'var(--sc-shell-sidebar-width)' : '0rem'} - var(--sc-shell-ask-width))`;
		return `min(${dock.size.css}, max(${room(min)}, min(${dock.size.normal}, ${room(min / 2)})))`;
	});
	/** the frame's columns: sidebar | content | dock | Ask */
	const columns = $derived(
		[sidebar && 'var(--sc-shell-sidebar-width)', 'minmax(0, 1fr)', dock && 'var(--sc-shell-dock-width)', ask && 'auto'].filter(Boolean).join(' ')
	);
	/** phones: the sidebar is a drawer (collapsible), the dock and Ask cover the page */
	const phoneColumns = $derived([sidebar && !collapsible && 'var(--sc-shell-sidebar-width)', 'minmax(0, 1fr)'].filter(Boolean).join(' '));
</script>

<div
	class="sc-app-shell {mode} {className}"
	class:has-sidebar={!!sidebar}
	class:has-dock={!!dock}
	class:has-ask={!!ask}
	class:collapsible
	class:folded={folded && !!foldKey}
	class:dock-folded={dockFolded}
	class:max-dock={maximized === 'dock'}
	class:max-main={!!maximized && maximized !== 'dock'}
	style:--sc-shell-content-col={sidebar ? 2 : 1}
	style:--sc-shell-top="{topHeight}px"
	style:--sc-shell-sidebar-width={folded && foldKey ? 'var(--sc-shell-rail-width)' : region.width !== null ? `max(${region.width}px, ${sidebarWidth})` : sidebarWidth}
	style:--sc-shell-dock-width={dockCss}
	style:--sc-shell-dock-own={dock?.size.css}
	style:--sc-shell-ask-width="{ask ? askWidth : 0}px"
	style:--sc-shell-columns={columns}
	style:--sc-shell-columns-phone={phoneColumns}
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
					<Rail side="left" label={railLabel} {unfoldLabel} title="{unfoldLabel} (Ctrl + \)" controls={sidebarId} onunfold={() => region.fold(false)} testid="shell-rail" />
				{:else}
					{#if foldKey}
						<FoldButton side="left" label={foldLabel} title="{foldLabel} (Ctrl + \)" controls={sidebarId} onclick={() => region.fold(true)} class="sc-shell-fold" />
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
			<aside class="sc-shell-dock" id="sc-shell-dock" aria-label={dockFolded ? dock.title : dockLabel} data-testid="shell-dock" data-sc-maximize="dock">
				{#if dockFolded}
					<Rail side="right" label={dock.title} icon={dock.icon} unfoldLabel={dockUnfoldLabel} controls="sc-shell-dock" onunfold={() => dock.size.fold(false)} testid="dock-rail" />
				{:else}
					{@render dock.panel()}
				{/if}
			</aside>
			<Divider size={dock.size} edge="left" at="calc(var(--sc-shell-dock-width) + var(--sc-shell-ask-width))" label={dockResizeLabel} controls="sc-shell-dock" class="sc-shell-dock-resize" />
		{/if}
		{#if ask}
			<div class="sc-shell-ask" bind:offsetWidth={askWidth}>{@render ask()}</div>
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
		/* a folded region: its round unfold button and 0.5rem on either side, plus the border (owner 2026-10-06: a fixed
		   2.5rem clipped the round button – it is --control-height, 44px on touch screens) */
		--sc-shell-rail-width: calc(var(--control-height) + 1rem + var(--border-width-regular));
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
		grid-template-columns: var(--sc-shell-columns);
		min-height: 0;
	}
	.sc-shell-content {
		min-width: 0;
	}
	.sc-shell-sidebar,
	.sc-shell-dock,
	.sc-shell-ask {
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
	/* Ask: the panel sizes itself (its own Divider on its left edge) */
	.sc-shell-ask {
		overflow: visible;
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
	.fill .sc-shell-dock,
	.fill .sc-shell-ask {
		position: static;
		align-self: stretch;
		height: auto;
		min-height: 0;
	}
	/* Ask's divider is placed in its region */
	.fill .sc-shell-ask {
		position: relative;
		top: 0;
	}
	.fill .sc-shell-content {
		min-height: 0;
		overflow: auto;
	}

	/* the sidebar's fold button: its own row, 1rem from the top, at the right (owner 2026-10-03: like the script
	   outline's fold button) */
	.sc-shell-sidebar > :global(.sc-shell-fold) {
		display: flex;
		margin: 1rem 0.5rem 0 auto;
	}
	.folded .sc-shell-sidebar,
	.dock-folded .sc-shell-dock {
		overflow: hidden;
	}

	.sc-shell-scrim {
		display: none;
	}

	/* maximize (from 768px): the maximized region spans the content's and the dock's columns, above the hidden one –
	   the grid itself stays as it was, so nothing hidden changes its size (scroll positions, a canvas, an edit stay) */
	@media (min-width: 768px) {
		.max-dock .sc-shell-frame > *,
		.max-main.has-dock .sc-shell-frame > * {
			grid-row: 1;
		}
		/* the hidden region keeps its column */
		.max-dock .sc-shell-content {
			grid-column: var(--sc-shell-content-col) / span 1;
			visibility: hidden;
		}
		.max-main.has-dock .sc-shell-dock {
			grid-column: calc(var(--sc-shell-content-col) + 1) / span 1;
			visibility: hidden;
		}
		.max-dock .sc-shell-dock,
		.max-main.has-dock .sc-shell-content {
			grid-column: var(--sc-shell-content-col) / span 2;
			z-index: 16;
			background: var(--paper);
		}
		.max-dock .sc-shell-dock {
			border-left: 0;
		}
		.max-dock :global(.sc-shell-dock-resize),
		.max-main :global(.sc-shell-dock-resize) {
			display: none;
		}
	}
	@media (max-width: 767px) {
		.sc-shell-frame {
			grid-template-columns: var(--sc-shell-columns-phone);
		}
		/* phones: the dock and Ask cover the page (closed by their own buttons) */
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
		/* full screen: above an app's bottom navigation */
		.sc-shell-ask,
		.fill .sc-shell-ask {
			position: fixed;
			z-index: 40;
			top: var(--sc-shell-top);
			left: 0;
			right: 0;
			bottom: var(--sc-shell-bottom);
			height: auto;
			align-self: stretch;
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
