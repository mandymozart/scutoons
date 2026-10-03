<script lang="ts">
	import { untrack, type Snippet } from 'svelte';
	import Icon from './icon.svelte';

	/**
	 * Page frame of the tools (admin, creator): top bar + optional sidebar (left) + content + optional aside (right),
	 * above the fixed bottom `ActionBar` (PLATFORM.md "Bottom action bar"). The sidebar/aside borders always reach the
	 * ActionBar (or the viewport bottom without one), at any scroll position and content length:
	 *
	 * - `mode="page"` (default): the document scrolls. The top bar is sticky; sidebar and aside are sticky and exactly
	 *   as tall as the space between the top bar and the ActionBar, and scroll on their own when taller. A short page
	 *   does not scroll at all.
	 * - `mode="fill"`: the shell is exactly the viewport minus the ActionBar and the document never scrolls; sidebar,
	 *   content and aside scroll on their own (editors).
	 *
	 * The shell reserves the ActionBar's height itself (`--action-bar-height`, inherited from the app; default 3.25rem)
	 * instead of the `body` padding the ActionBar adds, so the page never scrolls by a wrong amount (rem-based padding
	 * on a fluid root size). The top bar's height is measured (`--sc-shell-top`), so a wrapping top bar is fine.
	 *
	 * `collapsible`: below 768px the sidebar becomes an off-canvas drawer (`sidebarOpen`, scrim → `onclose`).
	 * `resizeKey`: from 768px on, the sidebar's right edge can be dragged wider (owner 2026-10-02: creator) – from its
	 * normal width up to a third of the window; arrow keys on the handle, double-click resets; remembered per key in
	 * localStorage (`sc-sidebar-width:<key>`).
	 * `foldable` (with `resizeKey`, owner 2026-10-03 "version A"): from 768px on the sidebar folds to a 2.5rem rail –
	 * the fold button (Feather chevrons-left) at its top right, dragging the edge narrower than the normal width, or Ctrl + \; the rail shows chevrons-right and
	 * `railLabel` (the current place) written vertically. Remembered per key (`sc-sidebar-folded:<key>`).
	 */
	let {
		topbar,
		sidebar,
		aside,
		children,
		mode = 'page',
		sidebarWidth = 'var(--tool-sidebar-width)',
		asideWidth = '24rem',
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
		class: className = ''
	}: {
		topbar?: Snippet;
		sidebar?: Snippet;
		aside?: Snippet;
		children: Snippet;
		mode?: 'page' | 'fill';
		/** CSS length; may use `var(…)` so an app can change it per breakpoint */
		sidebarWidth?: string;
		asideWidth?: string;
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
		class?: string;
	} = $props();

	// ---------- foldable sidebar (a rail) ----------
	let folded = $state(false);
	const foldKey = $derived(foldable && resizeKey ? `sc-sidebar-folded:${resizeKey}` : '');
	$effect(() => {
		if (!foldKey) return;
		try {
			folded = localStorage.getItem(foldKey) === '1';
		} catch {
			// not remembered
		}
	});
	function setFolded(value: boolean) {
		folded = value;
		try {
			if (value) localStorage.setItem(foldKey, '1');
			else localStorage.removeItem(foldKey);
		} catch {
			// not remembered
		}
	}
	$effect(() => {
		if (!foldKey) return;
		const key = (e: KeyboardEvent) => {
			if ((e.ctrlKey || e.metaKey) && e.key === '\\') {
				e.preventDefault();
				setFolded(!folded);
			}
		};
		window.addEventListener('keydown', key);
		return () => window.removeEventListener('keydown', key);
	});

	let topHeight = $state(0);

	// ---------- resizable sidebar ----------
	let sidebarEl = $state<HTMLElement>();
	/** px set by dragging; null = the normal `sidebarWidth` */
	let customWidth = $state<number | null>(null);
	/** the normal width in px (the minimum) – measured before the first drag */
	let baseWidth = 0;
	const storageKey = $derived(resizeKey ? `sc-sidebar-width:${resizeKey}` : '');

	$effect(() => {
		if (!storageKey || !sidebarEl) return;
		// measured before a saved width applies, never the rail; untracked: folding must not re-run this restore
		if (!untrack(() => folded)) baseWidth = sidebarEl.offsetWidth;
		try {
			const saved = Number(localStorage.getItem(storageKey));
			if (saved > 0) customWidth = saved;
		} catch {
			// storage blocked: the sidebar keeps its normal width
		}
	});

	function measureBase() {
		if (!baseWidth && sidebarEl && customWidth === null && !folded) baseWidth = sidebarEl.offsetWidth;
		return baseWidth || 240;
	}

	function clamp(px: number) {
		return Math.round(Math.min(Math.max(px, measureBase()), window.innerWidth / 3));
	}

	function save() {
		try {
			if (customWidth === null) localStorage.removeItem(storageKey);
			else localStorage.setItem(storageKey, String(customWidth));
		} catch {
			// not remembered
		}
	}

	function startResize(event: PointerEvent) {
		if (!sidebarEl) return;
		event.preventDefault();
		measureBase();
		const left = sidebarEl.getBoundingClientRect().left;
		const handle = event.currentTarget as HTMLElement;
		handle.setPointerCapture(event.pointerId);
		// one direction per drag (owner 2026-10-03: unfolding and folding again in the same drag glitched): a drag that
		// starts on the rail only opens the sidebar, a drag that starts open only folds it
		const startedFolded = !!foldKey && folded;
		const threshold = () => measureBase() * 0.6;
		const move = (e: PointerEvent) => {
			const x = e.clientX - left;
			if (startedFolded) {
				if (folded && x < threshold()) return; // still the rail
				if (folded) setFolded(false);
			} else if (foldKey && x < threshold()) {
				setFolded(true);
				end();
				return;
			}
			customWidth = clamp(x);
		};
		const end = () => {
			handle.removeEventListener('pointermove', move);
			handle.removeEventListener('pointerup', end);
			handle.removeEventListener('pointercancel', end);
			document.documentElement.classList.remove('sc-resizing');
			save();
		};
		document.documentElement.classList.add('sc-resizing');
		handle.addEventListener('pointermove', move);
		handle.addEventListener('pointerup', end);
		handle.addEventListener('pointercancel', end);
	}

	function keyResize(event: KeyboardEvent) {
		const step = event.shiftKey ? 64 : 16;
		const current = customWidth ?? measureBase();
		if (event.key === 'ArrowRight') customWidth = clamp(current + step);
		else if (event.key === 'ArrowLeft') customWidth = clamp(current - step);
		else if (event.key === 'Home') customWidth = null;
		else return;
		event.preventDefault();
		save();
	}

	function resetWidth() {
		customWidth = null;
		save();
	}

	/** a remembered width wider than a third of a smaller window shrinks with it */
	$effect(() => {
		if (!resizeKey) return;
		const fit = () => customWidth !== null && (customWidth = Math.min(customWidth, Math.round(window.innerWidth / 3)));
		window.addEventListener('resize', fit);
		return () => window.removeEventListener('resize', fit);
	});
</script>

<div
	class="sc-app-shell {mode} {className}"
	class:has-sidebar={!!sidebar}
	class:has-aside={!!aside}
	class:collapsible
	style:--sc-shell-top="{topHeight}px"
	class:folded={folded && !!foldKey}
	style:--sc-shell-sidebar-width={folded && foldKey ? 'var(--sc-shell-rail-width)' : customWidth !== null ? `max(${customWidth}px, ${sidebarWidth})` : sidebarWidth}
	style:--sc-shell-aside-width={asideWidth}
>
	{#if topbar}
		<div class="sc-shell-top" bind:offsetHeight={topHeight}>{@render topbar()}</div>
	{/if}
	<div class="sc-shell-frame">
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
						<button type="button" class="sc-shell-fold sc-round" onclick={() => setFolded(false)} aria-label={unfoldLabel} aria-expanded="false" aria-controls={sidebarId} title="{unfoldLabel} (Ctrl + \)"><Icon name="chevrons-right" /></button>
						{#if railLabel}<span class="sc-shell-rail-label">{railLabel}</span>{/if}
					</div>
				{:else}
					{#if foldKey}
						<button type="button" class="sc-shell-fold sc-shell-fold-in sc-round" onclick={() => setFolded(true)} aria-label={foldLabel} aria-expanded="true" aria-controls={sidebarId} title="{foldLabel} (Ctrl + \)"><Icon name="chevrons-left" /></button>
					{/if}
					{@render sidebar()}
				{/if}
			</svelte:element>
			{#if resizeKey}
				<!-- a focusable separator is a widget (WAI-ARIA window splitter) -->
				<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
				<div
					class="sc-shell-resize"
					role="separator"
					aria-orientation="vertical"
					aria-label={resizeLabel}
					aria-controls={sidebarId}
					aria-valuenow={customWidth ?? undefined}
					tabindex="0"
					title={resizeLabel}
					onpointerdown={startResize}
					onkeydown={keyResize}
					ondblclick={resetWidth}
				></div>
			{/if}
			{#if collapsible && sidebarOpen}
				<button type="button" class="sc-shell-scrim" aria-label={closeLabel} onclick={() => onclose?.()}></button>
			{/if}
		{/if}
		<div class="sc-shell-content">{@render children()}</div>
		{#if aside}
			<aside class="sc-shell-aside">{@render aside()}</aside>
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
	.has-aside .sc-shell-frame {
		grid-template-columns: minmax(0, 1fr) var(--sc-shell-aside-width);
	}
	.has-sidebar.has-aside .sc-shell-frame {
		grid-template-columns: var(--sc-shell-sidebar-width) minmax(0, 1fr) var(--sc-shell-aside-width);
	}
	.sc-shell-content {
		min-width: 0;
	}
	.sc-shell-sidebar,
	.sc-shell-aside {
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
	.sc-shell-aside {
		border-left: var(--border-width-regular) solid var(--ink);
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
	.fill .sc-shell-aside {
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

	/* the drag handle sits on the sidebar's right border */
	.sc-shell-resize {
		position: absolute;
		z-index: 15;
		top: 0;
		bottom: 0;
		left: calc(var(--sc-shell-sidebar-width) - 0.375rem);
		width: 0.75rem;
		cursor: col-resize;
		touch-action: none;
	}
	.sc-shell-resize:hover,
	.sc-shell-resize:focus-visible {
		background: linear-gradient(var(--ink), var(--ink)) center / var(--border-width-focus) 100% no-repeat;
		outline: none;
	}
	:global(html.sc-resizing),
	:global(html.sc-resizing *) {
		cursor: col-resize !important;
		user-select: none !important;
	}
	.sc-shell-scrim {
		display: none;
	}
	@media (max-width: 767px) {
		.collapsible.has-sidebar .sc-shell-frame {
			grid-template-columns: minmax(0, 1fr);
		}
		.collapsible.has-sidebar.has-aside .sc-shell-frame {
			grid-template-columns: minmax(0, 1fr) var(--sc-shell-aside-width);
		}
		.sc-shell-resize,
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
