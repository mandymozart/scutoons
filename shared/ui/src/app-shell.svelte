<script lang="ts">
	import { dockRegion } from './dock-region.svelte.ts';
	import { shellDock, type DockEntry } from './dock.svelte.ts';
	import DockContentHost from './dock-content.svelte';
	import DockSlot from './dock-slot.svelte';
	import Divider from './divider.svelte';
	import FoldButton from './fold-button.svelte';
	import Rail from './rail.svelte';
	import { untrack, type Snippet } from 'svelte';

	/**
	 * Page frame of the tools (admin, creator, embassy): top bar + optional sidebar (left) + content + the right dock,
	 * above the fixed bottom `ActionBar` (PLATFORM.md "Bottom action bar"). From 768px the regions
	 * tile and never overlap (agents/DOCKING.md); their borders always reach the ActionBar (or the viewport bottom without
	 * one), at any scroll position and content length:
	 *
	 * - `mode="page"` (default): the document scrolls. The top bar is sticky; sidebar and dock are sticky and exactly
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
	 * It folds by itself (not remembered) while `foldWhen` holds or a region is maximized (who gives way first:
	 * agents/DOCKING.md "Running out of room").
	 * The right dock: the page panels opened with `useDock` (dock.svelte.ts – Problems / History / Flow / Reader, Info, a
	 * list's detail), several at once side by side in their fixed places (10.57, agents/DOCKING.md "Several panels") –
	 * each a column of the frame, so the content makes room; each one's left edge is a `Divider`; dragged wider, a panel
	 * stops where the content would get narrower than `minContent` (default 20rem) next to the sidebar and the other panels.
	 * When the room runs out the panels get narrower together (down to 12rem each), then the least recently
	 * used folds to its rail, then the next; alone, a panel gets narrower than 12rem only while the content keeps half
	 * its minimum. Each folds to a rail (owner 2026-10-06: the panel's `DockFold` button or dragging it narrow; the rail
	 * shows the panel's icon and title, a click unfolds it). Below 768px the most recently used covers the page.
	 * Maximize (owner 2026-10-06, `shellDock.maximized`, agents/DOCKING.md "Maximize"): a docked panel (`dock:<id>`) or a
	 * region of the main area (its id, e.g. the slice inspector – the page hides the rest of its main area) fills
	 * everything right of the sidebar (folded to its rail). The other regions are only hidden (`visibility`, laid out
	 * as before – nothing unmounts: their widths, folds, scroll positions and unsaved edits stay); the page's ActionBar
	 * stays (10.57). The panel's `MaximizeButton`, Ctrl + Space (the focused region's, else the most recently used open
	 * panel's) and Esc restore. Not on phones.
	 */
	let {
		topbar,
		sidebar,
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
		pageLabel,
		pageFoldLabel,
		pageUnfoldLabel,
		class: className = ''
	}: {
		topbar?: Snippet;
		sidebar?: Snippet;
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
		 * false again. */
		foldWhen?: boolean;
		/** rem: the content's minimum width – a dock dragged wider stops there */
		minContent?: number;
		/** accessible name of the dock */
		dockLabel?: string;
		dockResizeLabel?: string;
		/** the dock's rail button */
		dockUnfoldLabel?: string;
		/** the main area's rail (default `railLabel`, the place); its fold (the leftmost panel's button) and unfold labels */
		pageLabel?: string;
		pageFoldLabel?: string;
		pageUnfoldLabel?: string;
		class?: string;
	} = $props();

	let topHeight = $state(0);
	/** the docked panels in their places, main area → right edge */
	const panels = $derived(shellDock.panels);
	const maxPanel = $derived(shellDock.maximizedPanel);
	/** the maximized region: `'dock'` (a docked panel – `maxPanel`, unfolded) or a main region's id */
	const maximized = $derived(
		maxPanel ? (!maxPanel.panel.size.folded ? 'dock' : null) : shellDock.maximized?.startsWith('dock') ? null : shellDock.maximized
	);

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
		// who gives way first (agents/DOCKING.md "Running out of room"); maximized = fold everything else
		foldWhen: () => foldWhen || !!maximized
	});
	const folded = $derived(region.folded);
	/** phones (< 768px): a collapsible sidebar is the drawer – it never shows the rail (a fold remembered on a desktop, or
	 * one forced by `foldWhen`, would leave an empty rail as wide as the drawer); its fold button closes the drawer */
	let phone = $state(false);
	$effect(() => {
		const query = window.matchMedia('(max-width: 767px)');
		const change = () => (phone = query.matches);
		change();
		query.addEventListener('change', change);
		return () => query.removeEventListener('change', change);
	});
	const drawer = $derived(collapsible && phone);
	const railed = $derived(folded && !!foldKey && !drawer);

	$effect(() => {
		if (!foldKey) return;
		const key = (e: KeyboardEvent) => {
			if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key === '\\') {
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
				// the focused region (a panel: `dock:<id>`); from anywhere else the most recently used open panel
				const target = active?.closest('[data-sc-maximize]')?.getAttribute('data-sc-maximize') || (latestShown ? `dock:${latestShown.id}` : null);
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

	// ---------- the right dock (several panels, 10.57) ----------
	let frameEl = $state<HTMLElement>();
	/** px: the frame's width (measured) */
	let frameWidth = $state(0);
	/** px: the sidebar as it shows (its width or its rail) */
	let sidebarPx = $state(0);
	/** px: a rail's width (`--sc-shell-rail-width`, measured by a probe) and the root's rem */
	let railPx = $state(0);
	let remPx = $state(16);
	$effect(() => {
		const el = sidebarEl;
		if (!el) return;
		const observer = new ResizeObserver(() => (sidebarPx = el.offsetWidth));
		observer.observe(el);
		sidebarPx = el.offsetWidth;
		return () => observer.disconnect();
	});
	$effect(() => {
		void frameWidth;
		remPx = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
	});
	/** the main area folded to its rail – by the author (`shellDock.foldMain`) or because a panel is maximized */
	const mainFolded = $derived(shellDock.mainFolded && !phone);
	const mainRailed = $derived(mainFolded || maximized === 'dock');
	/** rem: the panels get narrower together down to this (or a panel's normal width, if smaller) */
	const FLOOR = 12;
	/**
	 * Who gives way when the room runs out (agents/DOCKING.md "Running out of room"): the content keeps `minContent` (a
	 * folded main: its rail); the panels share what is left – each its own width, all narrower by the same share down to
	 * their floor; when even the floors don't fit, the least recently used panel folds to its rail (`squeezed`, not
	 * remembered), then the next. The most recently used never folds for room: alone it gets narrower than its floor
	 * while the content keeps half its minimum. Folded panels (by hand or squeezed) take a rail's width.
	 */
	const layout = $derived.by(() => {
		const min = mainFolded ? railPx : (minContent ?? 20) * remPx;
		const avail = frameWidth > 0 ? frameWidth - sidebarPx - min : Infinity;
		const want = (e: DockEntry) => e.panel.size.width ?? e.panel.size.normalPx;
		const floor = (e: DockEntry) => Math.min(FLOOR * remPx, e.panel.size.normalPx);
		const kept = new Set<number>();
		let floors = 0;
		for (const e of panels.filter((p) => !p.panel.size.folded).sort((a, b) => b.used - a.used)) {
			const rails = panels.length - kept.size - 1;
			if (kept.size && floors + floor(e) + rails * railPx > avail) break;
			kept.add(e.id);
			floors += floor(e);
		}
		const room = avail - (panels.length - kept.size) * railPx;
		const shown = panels.filter((e) => kept.has(e.id));
		const wants = shown.map(want);
		const total = wants.reduce((a, w) => a + w, 0);
		const widths = new Map<number, number>();
		shown.forEach((e, i) => {
			let w = wants[i];
			if (total > room && floors <= room) w -= (w - floor(e)) * ((total - room) / Math.max(1, total - floors));
			else if (total > room) w = Math.min(w, Math.max(room, Math.min(floor(e), room + min / 2)));
			widths.set(e.id, Math.max(railPx, Math.round(w)));
		});
		return { widths, room, squeezed: new Set(panels.filter((e) => !e.panel.size.folded && !kept.has(e.id)).map((e) => e.id)) };
	});
	/** the most recently used panel that shows (Ctrl + Space; with the main folded it takes the rest of the room) */
	const latestShown = $derived(panels.filter((e) => layout.widths.has(e.id)).reduce<DockEntry | undefined>((a, e) => (!a || e.used > a.used ? e : a), undefined));
	$effect(() => {
		shellDock.shown = panels.filter((e) => layout.widths.has(e.id)).map((e) => e.id);
	});
	/** a panel shows as its rail: folded by hand, squeezed, or another region is maximized */
	const panelRailed = (e: DockEntry) => (maximized ? maxPanel?.id !== e.id || maximized !== 'dock' : !layout.widths.has(e.id));
	/** on phones the most recently used panel covers the page (the others stay open behind it) */
	const front = $derived(shellDock.latest);
	/** px: each panel's width as it shows (measured) – and as it last showed unfolded: a railed panel stays laid out at
	 * that width, hidden (nothing unmounts – the Flow's zoom, the Reader's frame, a scroll position stay) */
	let shownWidth = $state<Record<number, number>>({});
	let keptWidth = $state<Record<number, number>>({});
	$effect(() => {
		for (const e of panels) {
			const w = shownWidth[e.id];
			if (w && !panelRailed(e)) untrack(() => (keptWidth[e.id] = w));
		}
	});
	/** px: the main area's width as it last showed (it stays laid out at it while folded, hidden) */
	let contentWidth = $state(0);
	let keptMain = $state(0);
	$effect(() => {
		if (!mainRailed && contentWidth) keptMain = contentWidth;
	});
	// a drag stops where the content would get narrower than its minimum next to the other panels; the drag measures from
	// the panel's own right edge (another panel may sit right of it)
	$effect(() => {
		const list = panels;
		for (const e of list) {
			const size = e.panel.size;
			size.room = () => layout.room - list.reduce((a, o) => a + (o === e || !layout.widths.has(o.id) ? 0 : (layout.widths.get(o.id) ?? 0)), 0);
			size.edge = () => document.getElementById(`sc-shell-dock-${e.id}`)?.getBoundingClientRect().right ?? window.innerWidth;
		}
		return () => {
			for (const e of list) {
				e.panel.size.room = null;
				e.panel.size.edge = null;
			}
		};
	});
	/** where each panel's Divider sits: its own width + everything right of it (the other panels) – measured */
	const dividerAt = (i: number) => panels.slice(i).reduce((a, e) => a + (shownWidth[e.id] ?? railPx), 0);
	/** a rail while something is maximized restores; a folded panel's rail unfolds it; a squeezed one's makes it the
	 * most recently used (another one folds instead) */
	function unfold(e: DockEntry) {
		if (maximized) return shellDock.restore();
		e.panel.size.fold(false);
		shellDock.touch(e.id);
	}
	function unfoldMain() {
		if (maximized) shellDock.restore();
		shellDock.foldMain(false);
	}
	// Ctrl + Shift + \ folds / unfolds the main area while a panel is open (never by itself – owner 2026-10-07)
	$effect(() => {
		const key = (e: KeyboardEvent) => {
			if (!(e.ctrlKey || e.metaKey) || !e.shiftKey || e.altKey || phone) return;
			if (e.code !== 'Backslash' && e.key !== '\\' && e.key !== '|') return;
			if (!panels.length) return;
			e.preventDefault();
			if (shellDock.mainFolded) unfoldMain();
			else shellDock.foldMain(true);
		};
		window.addEventListener('keydown', key);
		return () => window.removeEventListener('keydown', key);
	});
	/** labels in the page's language (the shell has no messages of its own) */
	const german = typeof document !== 'undefined' && document.documentElement.lang.startsWith('de');
	const pageRailLabel = $derived(pageLabel ?? (railLabel || (german ? 'Seite' : 'Page')));
	const pageShow = $derived(pageUnfoldLabel ?? (german ? 'Seite zeigen' : 'Show the page'));
	$effect(() => {
		shellDock.pageFoldLabel = pageFoldLabel ?? (german ? 'Seite einklappen' : 'Fold the page');
	});
	const contentCol = $derived(sidebar ? 2 : 1);
	/** the frame's columns: sidebar | content | the panels – a railed region is a rail; maximized, the region takes
	 * the rest; with the main folded the most recently used panel does */
	const columns = $derived.by(() => {
		const rail = 'var(--sc-shell-rail-width)';
		const panelCol = (e: DockEntry) => {
			if (panelRailed(e)) return rail;
			if (maximized === 'dock') return 'minmax(0, 1fr)';
			const w = layout.widths.get(e.id) ?? 0;
			return mainFolded && latestShown?.id === e.id ? `minmax(${w}px, 1fr)` : `${w}px`;
		};
		return [sidebar && 'var(--sc-shell-sidebar-width)', mainRailed ? rail : 'minmax(0, 1fr)', ...panels.map(panelCol)]
			.filter(Boolean)
			.join(' ');
	});
	/** phones: the sidebar is a drawer (collapsible), the dock covers the page */
	const phoneColumns = $derived([sidebar && !collapsible && 'var(--sc-shell-sidebar-width)', 'minmax(0, 1fr)'].filter(Boolean).join(' '));
</script>

<div
	class="sc-app-shell {mode} {className}"
	class:has-sidebar={!!sidebar}
	class:has-dock={panels.length > 0}
class:collapsible
	class:folded={folded && !!foldKey}
	class:max-dock={maximized === 'dock'}
	class:max-main={!!maximized && maximized !== 'dock'}
	class:main-railed={mainRailed}
style:--sc-shell-content-col={contentCol}
style:--sc-shell-main-kept={mainRailed && keptMain ? `${keptMain}px` : null}
	style:--sc-shell-top="{topHeight}px"
	style:--sc-shell-sidebar-width={folded && foldKey ? 'var(--sc-shell-rail-width)' : region.width !== null ? `max(${region.width}px, ${sidebarWidth})` : sidebarWidth}
style:--sc-shell-columns={columns}
	style:--sc-shell-columns-phone={phoneColumns}
>
	{#if topbar}
		<div class="sc-shell-top" bind:offsetHeight={topHeight}>{@render topbar()}</div>
	{/if}
	<div class="sc-shell-frame" bind:this={frameEl} bind:clientWidth={frameWidth}>
		<div class="sc-shell-rail-probe" aria-hidden="true" bind:offsetWidth={railPx}></div>
		{#if sidebar}
			<svelte:element
				this={sidebarTag}
				id={sidebarId}
				class="sc-shell-sidebar"
				class:open={sidebarOpen}
				bind:this={sidebarEl}
				aria-label={sidebarLabel}
			>
				{#if railed}
					<Rail side="left" label={railLabel} {unfoldLabel} title="{unfoldLabel} (Ctrl + \)" controls={sidebarId} onunfold={() => (maximized ? shellDock.restore() : region.fold(false))} testid="shell-rail" />
				{:else}
					{#if drawer}
						<!-- the drawer's fold = close it (BACKLOG 10.50 a) -->
						<FoldButton side="left" label={closeLabel} title={closeLabel} controls={sidebarId} onclick={() => onclose?.()} class="sc-shell-fold" />
					{:else if foldKey}
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
		<div class="sc-shell-content" id="sc-shell-content" bind:offsetWidth={contentWidth}>{@render children()}</div>
		{#if mainRailed}
			<div class="sc-shell-main-rail">
				<Rail side="left" label={pageRailLabel} unfoldLabel={pageShow} title="{pageShow} (Ctrl + Shift + \)" controls="sc-shell-content" onunfold={unfoldMain} testid="main-rail" />
			</div>
		{/if}
		{#each panels as e, i (e.id)}
			{@const railed = panelRailed(e)}
			<aside
				class="sc-shell-dock"
				class:railed
				class:maxed={maxPanel?.id === e.id && maximized === 'dock'}
				class:front={front?.id === e.id}
				id="sc-shell-dock-{e.id}"
				aria-label={e.panel.title || dockLabel}
				data-testid="shell-dock"
				data-kind={e.panel.kind}
				data-sc-maximize="dock:{e.id}"
				style:--sc-dock-col={contentCol + 1 + i}
				style:--sc-shell-dock-own={e.panel.size.css}
				bind:offsetWidth={shownWidth[e.id]}
			>
				{#if railed}
					<Rail
						side="right"
						label={e.panel.title}
						icon={e.panel.icon}
						unfoldLabel={dockUnfoldLabel}
						controls="sc-shell-dock-{e.id}"
						onunfold={() => unfold(e)}
						testid={maximized ? 'dock-rail-maximized' : layout.squeezed.has(e.id) ? 'dock-rail-squeezed' : 'dock-rail'}
					/>
				{/if}
				<!-- the panel stays mounted while it is a rail: laid out at its last width, hidden -->
				<div class="sc-shell-panel" class:kept={railed} style:width={railed ? `${keptWidth[e.id] ?? e.panel.size.normalPx}px` : null}>
					<DockSlot id={e.id}>
						{#if e.panel.content}
							<!-- a panel that survives navigation (E.3 b): mounted here, the next page of the same kind hands it its props -->
							{#key e.panel.kind}<DockContentHost content={e.panel.content} />{/key}
						{:else if e.panel.panel}
							{@render e.panel.panel()}
						{/if}
					</DockSlot>
				</div>
			</aside>
			<Divider size={e.panel.size} edge="left" at="{dividerAt(i)}px" label={dockResizeLabel} controls="sc-shell-dock-{e.id}" class="sc-shell-dock-resize" />
		{/each}
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
	/* a rail's width, measured for the dock's arithmetic */
	.sc-shell-rail-probe {
		position: absolute;
		width: var(--sc-shell-rail-width);
		height: 0;
		visibility: hidden;
		pointer-events: none;
	}
	/* a dock panel: its content fills it and scrolls on its own (or pans, like the Flow) */
	.sc-shell-dock {
		display: flex;
		flex-direction: column;
		overflow: hidden;
		border-left: var(--border-width-regular) solid var(--ink);
	}
	/* the panel's own element fills the region (a railed one: laid out at its last width, hidden – below) */
	.sc-shell-panel {
		display: flex;
		flex-direction: column;
		flex: 1 1 auto;
		min-height: 0;
	}
	.sc-shell-panel > :global(*) {
		flex: 1 1 auto;
		min-height: 0;
	}
	/* side by side a panel can get narrow (10.57): its header's title gives way first (cut with …), the buttons stay */
	.sc-shell-panel :global(header > h2) {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.sc-shell-panel.kept {
		position: absolute;
		top: 0;
		bottom: 0;
		left: 0;
		visibility: hidden;
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
	/* a railed panel holds its hidden, laid-out content (absolute) – sticky (page) is positioned already */
	.fill .sc-shell-dock.railed {
		position: relative;
		top: 0;
	}

	/* the sidebar's fold button: its own row, 1rem from the top, at the right (owner 2026-10-03: like the script
	   outline's fold button) */
	.sc-shell-sidebar > :global(.sc-shell-fold) {
		display: flex;
		margin: 1rem 0.5rem 0 auto;
	}
	.folded .sc-shell-sidebar,
	.sc-shell-dock.railed {
		overflow: hidden;
	}

	.sc-shell-scrim {
		display: none;
	}

	@media (min-width: 768px) {
		/* every region in its own column of one row: sidebar | content | the panels */
		.sc-shell-frame > :global(.sc-shell-sidebar) {
			grid-area: 1 / 1;
		}
		.sc-shell-content {
			grid-area: 1 / var(--sc-shell-content-col);
		}
		.sc-shell-dock {
			grid-area: 1 / var(--sc-dock-col);
		}
		/* the main area folded (by hand, or a panel is maximized): its rail in its column; the page stays laid out at its
		   last width, hidden (nothing unmounts: scroll positions, a canvas, unsaved edits stay) */
		.main-railed .sc-shell-frame {
			overflow-x: clip;
		}
		.main-railed .sc-shell-content {
			width: var(--sc-shell-main-kept, auto);
			visibility: hidden;
		}
		.sc-shell-main-rail {
			grid-area: 1 / var(--sc-shell-content-col);
			position: sticky;
			z-index: 1;
			top: var(--sc-shell-top);
			align-self: start;
			box-sizing: border-box;
			height: var(--sc-shell-height);
			overflow: hidden;
			background: var(--paper);
		}
		.fill .sc-shell-main-rail {
			position: static;
			align-self: stretch;
			height: auto;
		}
		/* the page's ActionBar stays while the main is folded or a region maximized (10.57: Save / Publish / undo / the
		   slice nav still act on the main area the panels drive – like the top bar it is the page's chrome, not a region) */
		.main-railed .sc-shell-content :global(.sc-action-bar) {
			visibility: visible;
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
		.sc-shell-main-rail {
			display: none;
		}
		/* phones: the most recently used panel covers the page (closed by their own buttons); the other panels stay
		   open behind it */
		.sc-shell-dock:not(.front) {
			display: none;
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
