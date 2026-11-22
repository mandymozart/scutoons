<script lang="ts">
	import { Floats, FloatingPanels } from '@scutoons/ui/editor';
	import { CloseButton, Icon } from '@scutoons/ui';
	import type { WorkspaceKind } from '$lib/shell/address.ts';
	import { availableTabs, panelSpecs, tabSpecs } from '$lib/shell/panels.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import SettingsPanel from '$lib/components/shell/settings-panel.svelte';
	import LibraryPanel from '$lib/components/library/library-panel.svelte';
	import LibraryBrowser from '$lib/components/library/library-browser.svelte';
	import { library } from '$lib/editor/library.svelte.ts';
	import ToonPanel from '$lib/components/shell/toon-panel.svelte';
	import Stage from '$lib/components/shell/stage.svelte';
	import LogPanel from '$lib/components/shell/log-panel.svelte';
	import CoreLogProvider from '$lib/components/shell/core-log-provider.svelte';
	import { log } from '$lib/log/log.svelte.ts';
	import { scopeOf } from '$lib/shell/address.ts';
	import { getCreatorNav } from '$lib/nav.svelte.ts';

	/**
	 * One workspace's panels over the stage (panels.html): the preset's panels with the tabs that have content now
	 * (lib/shell/panels.ts), their layout kept per workspace in this browser. Open or folded, a panel floats the same –
	 * the stage never gives way. A tab can take the stage (◱ main) or open over everything (⤢).
	 */
	let { kind, toon = null }: { kind: WorkspaceKind | 'home'; toon?: string | null } = $props();

	const specs = tabSpecs();
	const filled = $derived(Object.keys(shell.tabs).sort().join(' '));
	const available = $derived(availableTabs(filled ? filled.split(' ') : [], toon));
	const floats = $derived(new Floats(`creator-host-${kind}`, panelSpecs(kind, available, specs), specs));

	$effect(() => {
		shell.floats = floats;
		shell.kind = kind;
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

	// a panel asked for by its id, or a panel tab by its tab's id (the panel holding it opens on it)
	$effect(() => {
		const id = shell.wanted;
		if (!id) return;
		const panel = floats.spec(id) ? id : floats.specs.find((s) => floats.tabsOf(s.id).includes(id))?.id;
		if (!panel) return;
		floats.reopen(panel);
		floats.setCollapsed(panel, false);
		floats.front(panel);
		if (panel !== id) floats.select(panel, id);
		shell.wanted = null;
	});

	/** the tabs on the stage, only while they have content */
	const staged = $derived(shell.stage.filter((t) => available.has(t)));
	$effect(() => {
		floats.staged = staged;
	});
	const over = $derived(shell.over && available.has(shell.over) ? shell.over : null);
	const name = (tab: string) => floats.tab(tab)?.name ?? tab;
	let stage = $state<ReturnType<typeof Stage>>();
	/** ◱: the tab alone on the stage, or back */
	function onmain(tab: string) {
		if (!staged.includes(tab)) stage?.take(tab);
		shell.send(tab);
	}
	const typing = (e: KeyboardEvent) => {
		const t = e.target as HTMLElement | null;
		return !!t && (/^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName) || t.isContentEditable);
	};
	/** Esc: the tab over everything closes, else the last tab on the stage goes back; M: ◱ for the front panel's tab */
	function onkeydown(e: KeyboardEvent) {
		if (e.defaultPrevented || e.ctrlKey || e.metaKey || e.altKey || typing(e) || floats.heldTab || library.browser) return;
		if (e.key === 'Escape') {
			if (shell.over) shell.over = null;
			else if (staged.length && !document.querySelector('[aria-modal="true"]')) shell.back(staged.at(-1));
			else return;
			e.preventDefault();
		} else if (e.key === 'm' || e.key === 'M') {
			const front = [...floats.order].reverse().find((id) => floats.spec(id) && !floats.closed(id) && floats.activeOf(id));
			const tab = front && floats.activeOf(front);
			if (!tab) return;
			e.preventDefault();
			onmain(tab);
		}
	}

	function onover(tab: string) {
		if (tab === 'library') library.browser = true;
		else shell.over = tab;
	}
</script>

<svelte:window {onkeydown} />

{#snippet content(id: string)}
	<div class="ws-tab" data-testid="panel-{id}">
		{#if id === 'toon' && toon}
			<ToonPanel {toon} />
		{:else if id === 'library'}
			<LibraryPanel />
		{:else if id === 'log'}
			<LogPanel />
		{:else if id === 'settings'}
			<SettingsPanel onreset={() => shell.resetView()} />
		{:else if shell.tabs[id]}
			{@render shell.tabs[id]()}
		{/if}
	</div>
{/snippet}

<CoreLogProvider />
<Stage bind:this={stage} {floats} stage={staged} {content} {name} />

<div class="ws-overlay" data-workspace={kind} data-testid="workspace">
	{#key floats}
		<FloatingPanels
			{floats}
			{onmain}
			{onover}
			stageDrop={{ zone: (t, x, y) => stage?.zone(t, x, y) ?? null, drop: (t, x, y) => stage?.drop(t, x, y) ?? false }}
			labels={{ fold: m['creator.fold_panel'](), open: m['creator.unfold_panel'](), close: m['creator.close'](), main: m['shell.main'](), back: m['shell.main_back'](), over: m['shell.over']() }}
		>
			{#snippet tab(id)}
				{#if staged.includes(id) || id === over}
					<p class="away">{id === over ? m['shell.over']() : m['shell.on_stage']()}</p>
				{:else}
					{@render content(id)}
				{/if}
			{/snippet}
		</FloatingPanels>
	{/key}
</div>

{#if over}
	<div class="over" role="dialog" aria-modal="true" aria-label={name(over)} data-testid="over">
		<div class="box">
			<div class="head">
				<span class="name">{name(over)}</span>
				<button type="button" class="back" onclick={() => (shell.over = null)} aria-label={m['shell.main_back']()} title={m['shell.main_back']()}><Icon name="minimize-2" size="0.9rem" strokeWidth={2} /></button>
				<CloseButton label={m['creator.close']()} onclick={() => (shell.over = null)} />
			</div>
			<div class="body">{@render content(over)}</div>
		</div>
	</div>
{/if}
{#if library.browser}<LibraryBrowser />{/if}

<style>
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
	.away {
		margin: 0;
		padding: var(--editor-pad);
		color: var(--editor-muted);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
	.back {
		display: inline-grid;
		place-items: center;
		width: var(--control-height);
		min-height: var(--control-height);
		padding: 0;
		border: 0;
		border-radius: 50%;
		background: transparent;
		color: inherit;
	}
	.back:hover {
		background: var(--editor-hover);
		color: var(--editor-text);
	}
	/* over everything (as OSCT's asset manager): a dialog over the whole creator, the bar included */
	.over {
		position: fixed;
		z-index: 80;
		inset: 0;
		background: rgb(0 0 0 / 0.55);
	}
	.over .box {
		position: absolute;
		inset: 2rem 2.5rem 2.5rem;
		display: flex;
		flex-direction: column;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: var(--editor-radius);
		background: var(--editor-surface);
		color: var(--editor-text);
		box-shadow: 0 1rem 3rem rgb(0 0 0 / 0.5);
		overflow: hidden;
		font-family: var(--editor-sans);
	}
	.over .head {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.25rem 0.25rem 0.25rem 1rem;
		background: var(--editor-surface-head);
	}
	.over .name {
		flex: 1;
	}
	.over .body {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		overflow: auto;
	}
</style>
