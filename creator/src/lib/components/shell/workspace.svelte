<script lang="ts">
	import { untrack } from 'svelte';
	import { fade } from 'svelte/transition';
	import { Button, Floats, FloatingPanels, StageZones, motionMs, type Cell, type StageZone } from '@scutoons/ui/editor';
	import { ITEM_VIEW, isCanvas } from '$lib/shell/stage.ts';
	import type { WorkspaceKind } from '$lib/shell/address.ts';
	import { PRESETS, availableTabs, panelSpecs, presetStart, tabSpecs } from '$lib/shell/panels.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import SettingsPanel from '$lib/components/shell/settings-panel.svelte';
	import LibraryPanel from '$lib/components/library/library-panel.svelte';
	import LibraryBrowser from '$lib/components/library/library-browser.svelte';
	import { library } from '$lib/editor/library.svelte.ts';
	import ToonPanel from '$lib/components/shell/toon-panel.svelte';
	import Stage from '$lib/components/shell/stage.svelte';
	import LogPanel from '$lib/components/shell/log-panel.svelte';
	import SporesPanel from '$lib/components/shell/spores-panel.svelte';
	import InfoPanel from '$lib/components/shell/info-panel.svelte';
	import CoreLogProvider from '$lib/components/shell/core-log-provider.svelte';
	import { log } from '$lib/log/log.svelte.ts';
	import { scopeOf } from '$lib/shell/address.ts';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import HistoryPanel from '$lib/components/page/history-panel.svelte';
	import { layouts } from '$lib/shell/layouts.svelte.ts';
	import { shownWorkspace } from '$lib/shell/layouts.ts';

	/**
	 * One workspace's panels over the stage (panels.html): the preset's panels with the tabs that have content now
	 * (lib/shell/panels.ts), their layout kept per workspace in this browser between saves; a new browser and Reset start
	 * from the workspace's saved layout (lib/shell/layouts.ts). An own workspace shows its kind's panels. Open or folded, a panel floats over the
	 * stage – the stage never gives way. A view takes the stage by a drag to a screen edge or corner, or through Views; it
	 * goes full screen through Views, Ctrl Space (the view under the pointer) or a hold past the top edge; Esc leaves.
	 */
	let { kind, toon = null }: { kind: WorkspaceKind | 'home'; toon?: string | null } = $props();

	const specs = tabSpecs();
	const filled = $derived(Object.keys(shell.tabs).sort().join(' '));
	const available = $derived(availableTabs(filled ? filled.split(' ') : [], toon));
	/** the workspace shown: the kind's own, or an own workspace made from it */
	const id = $derived(shownWorkspace(layouts.chosen, layouts.state.own, kind));
	/** where the workspace starts: its saved layout, else its preset's main view on the stage */
	const startOf = (id: string) => layouts.startOf(id) ?? presetStart(kind);
	const floats = $derived(new Floats(`creator-host-${id}`, panelSpecs(kind, available, specs), specs, untrack(() => startOf(id))));
	// a saved layout arriving (or saved anew) is the workspace's start: shown while this browser kept none of its own
	$effect(() => {
		const start = startOf(id);
		untrack(() => floats.setStart(start));
	});
	/** the opened item's main view, while the stage shows nothing of it: offered back onto the stage */
	const main = $derived(kind === 'home' ? undefined : PRESETS[kind]?.main);
	const mainAway = $derived(!!main && available.has(main) && floats.stage.slots.every((s) => !s || !available.has(s)));
	$effect(() => void layouts.load());

	$effect(() => {
		shell.floats = floats;
		shell.kind = kind;
		shell.available = available;
	});
	$effect(() => () => (shell.floats = null));

	// the Library follows the front item: its context is the workspace's pre-filter
	$effect(() => library.follow(kind === 'home' ? null : (shell.front?.address ?? null), toon));
	// the Log follows the front item too: its scope (a chapter under its season) and its name
	const nav = getCreatorNav();
	$effect(() => {
		const front = kind === 'home' ? null : shell.front;
		log.follow(scopeOf(front?.address, (t, c) => nav.chapterSeason(t, c)), front?.label ?? '');
	});
	$effect(() => {
		void log.load();
		return log.listen();
	});
	// a blank slot's pick opens the Library panel
	$effect(() => {
		if (library.pick) shell.open('library');
	});

	// a panel asked for by its id, or a panel tab by its tab's id (the panel holding it opens on it, a hidden tab shows again)
	$effect(() => {
		const id = shell.wanted;
		if (!id) return;
		if (floats.panelOf(id)) {
			floats.showTab(id);
			shell.wanted = null;
			return;
		}
		if (!floats.spec(id)) return;
		floats.reopen(id);
		floats.setCollapsed(id, false);
		floats.front(id);
		shell.wanted = null;
	});

	const name = (tab: string) => floats.tab(tab)?.name ?? tab;
	/** the view over everything, while it has content (the open item's own view is always there) */
	const full = $derived(floats.full && (floats.full === ITEM_VIEW || available.has(floats.full)) ? floats.full : null);
	/** the dashboard or the launcher opened from full screen lies over it; closed, full screen is back */
	const under = $derived(!!(shell.dashboard || shell.launcher));

	const CELL_NAMES: Record<Cell, () => string> = {
		whole: () => m['shell.cell_whole'](),
		left: () => m['shell.cell_left'](),
		right: () => m['shell.cell_right'](),
		'top-left': () => m['shell.cell_top_left'](),
		'top-right': () => m['shell.cell_top_right'](),
		'bottom-left': () => m['shell.cell_bottom_left'](),
		'bottom-right': () => m['shell.cell_bottom_right']()
	};
	const lands = (tab: string, zone: StageZone) =>
		zone.full ? m['shell.stage_full']({ tab: name(tab) }) : m['shell.stage_lands']({ tab: name(tab), where: CELL_NAMES[zone.cell]() });

	/** the view under the pointer: a cell of the stage, a panel's front tab, else the open item's own view */
	let pointer: [number, number] = [0, 0];
	function viewAt(x: number, y: number): string {
		const el = document.elementFromPoint(x, y);
		const cell = el?.closest<HTMLElement>('[data-stage-box]')?.dataset.stageBox;
		if (cell) return cell;
		const panel = el?.closest<HTMLElement>('[data-float]')?.dataset.float;
		return (panel && floats.activeOf(panel)) || ITEM_VIEW;
	}
	/** Ctrl Space: the view under the pointer full screen, and back; Esc leaves full screen */
	function onkeydown(e: KeyboardEvent) {
		if (e.defaultPrevented || floats.heldTab || library.browser) return;
		if (e.key === ' ' && e.ctrlKey && !e.metaKey && !e.altKey) {
			e.preventDefault();
			floats.toggleFull(full ?? viewAt(pointer[0], pointer[1]));
		} else if (e.key === 'Escape' && full && !document.querySelector('[aria-modal="true"]')) {
			e.preventDefault();
			floats.full = null;
		}
	}
</script>

<svelte:window {onkeydown} onpointermove={(e) => (pointer = [e.clientX, e.clientY])} />

{#snippet content(id: string)}
	<div class="ws-tab" data-testid="panel-{id}">
		{#if id === 'toon' && toon}
			<ToonPanel {toon} />
		{:else if id === 'library'}
			<LibraryPanel />
		{:else if id === 'log'}
			<LogPanel />
		{:else if id === 'weave'}
			<SporesPanel />
		{:else if id === 'info'}
			<InfoPanel />
		{:else if id === 'history'}
			{#if shell.doc?.docs?.length}<HistoryPanel docs={shell.doc.docs} />{:else}<p class="sc-empty">{m['shell.history_none']()}</p>{/if}
		{:else if id === 'settings'}
			<SettingsPanel onreset={() => shell.resetView()} />
		{:else if shell.tabs[id]}
			{@render shell.tabs[id]()}
		{/if}
	</div>
{/snippet}

<CoreLogProvider />
{#if !full}
	<Stage {floats} {content} {name} has={(t) => available.has(t)} />
{/if}

{#if !full && main && mainAway}
	<div class="main-away" data-testid="main-away">
		<p class="sc-empty">{m['shell.main_away']({ tab: name(main) })}</p>
		<Button icon="maximize-2" onclick={() => floats.setState(main, 'stage')} data-testid="main-to-stage">{m['shell.main_to_stage']({ tab: name(main) })}</Button>
	</div>
{/if}

<div class="ws-overlay" class:gone={!!full} data-workspace={kind} data-testid="workspace">
	{#key floats}
		<FloatingPanels {floats} labels={{ close: m['creator.close']() }}>
			{#snippet tab(id)}
				{@render content(id)}
			{/snippet}
		</FloatingPanels>
	{/key}
</div>
<div class="zones-layer"><StageZones {floats} label={lands} /></div>

{#if full && full !== ITEM_VIEW}
	<div class="full" class:under role="dialog" aria-label={name(full)} data-testid="full" out:fade={{ duration: motionMs('base') }}>
		{#if isCanvas(full)}{@render content(full)}{:else}<div class="full-page">{@render content(full)}</div>{/if}
	</div>
{/if}
{#if full && !under}
	<p class="full-hint" data-testid="full-hint">{m['shell.full_hint']()}</p>
{/if}
{#if library.browser}<LibraryBrowser />{/if}

<style>
	/* the stage without the item's main view: one quiet line and the way back, in the middle of the window */
	.main-away {
		position: fixed;
		z-index: 30;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		pointer-events: none;
	}
	.main-away > :global(*) {
		pointer-events: auto;
	}
	.main-away .sc-empty {
		margin: 0;
	}
	/* the panels' layer: under the bar's row, above the status bar; only the panels take the pointer */
	.ws-overlay {
		position: fixed;
		z-index: 40;
		top: var(--shell-top);
		bottom: var(--status-room);
		left: 0;
		right: 0;
		pointer-events: none;
	}
	.ws-overlay :global(.floats) {
		background: transparent;
	}
	.ws-overlay :global([data-float]) {
		pointer-events: auto;
	}
	.ws-tab {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
		box-sizing: border-box;
	}
	/* full screen: the panels step aside with the bar and the status bar */
	.ws-overlay.gone {
		display: none;
	}
	/* the drop zones reach the screen's edges: over the bar and the status bar */
	.zones-layer {
		position: fixed;
		z-index: 70;
		inset: 0;
		pointer-events: none;
	}
	/* a view over everything: the whole window, on the stage's ground */
	.full {
		position: fixed;
		z-index: 80;
		inset: 0;
		display: flex;
		flex-direction: column;
		overflow: auto;
		background: var(--tool-ground);
		color: var(--tool-text);
		font-family: var(--editor-sans);
		animation: full-in var(--motion-slow) var(--motion-ease);
	}
	/* under an overlay opened from full screen (the dashboard, the launcher) */
	.full.under {
		z-index: 44;
	}
	.full-page {
		box-sizing: border-box;
		width: 100%;
		max-width: 48rem;
		margin: 0 auto;
		padding: 2rem 1.5rem;
	}
	.full-hint {
		position: fixed;
		z-index: 81;
		right: 1rem;
		bottom: 0.75rem;
		margin: 0;
		color: var(--tool-muted);
		font-family: var(--editor-sans);
		font-size: var(--editor-small);
		pointer-events: none;
	}
	@keyframes full-in {
		from {
			opacity: 0;
			transform: scale(0.97);
		}
	}
</style>
