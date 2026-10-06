<script lang="ts">
	import { leaveGuard } from '$lib/editor/ask.ts';
	import type { Snippet } from 'svelte';
	import { ActionBar, panelWidth } from '@scutoons/ui';
	import { beforeNavigate } from '$app/navigation';
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { DocSession } from '$lib/editor/doc-session.svelte.ts';
	import { publishLabel } from '$lib/editor/reviews.svelte.ts';
	import { editorKeys } from '$lib/editor/editor-keys.ts';
	import { validateFile, validateLocales, validateNeutral, type Problem } from '@scutoons/content';
	import DocNotices from '$lib/components/page/doc-notices.svelte';
	import { pagePanel } from '$lib/ask.svelte.ts';
	import { isAccepted } from '$lib/editor/accepted.ts';
	import ProblemsPanel from '$lib/components/page/problems-panel.svelte';
	import HistoryPanel from '$lib/components/page/history-panel.svelte';
	import FileStatus, { hasStatus } from '$lib/components/status/file-status.svelte';
	import PageHelp, { type HelpPage } from '$lib/components/page/page-help.svelte';
	import { fileLabel } from '$lib/labels.ts';

	/**
	 * Editor shell for toon-level entities (season, toon settings): top bar with their versions, the bottom ActionBar
	 * (problems · undo/redo · history · note + Save draft / Publish), autosave + recover, missing-locale bars, the
	 * leave guard and Ctrl+S / Ctrl+Z (outside a field). `docs` are validated, saved and published; `companions()` are
	 * files that follow them (the chapter files whose status mirrors the season) – saved when dirty, published with the
	 * docs. Save / publish / conflict: lib/editor/doc-session.svelte.ts; the bars: DocNotices.
	 */
	let {
		toon,
		title,
		docs,
		companions = () => [],
		beforeSave = () => {},
		validate,
		wide = false,
		fill = false,
		actions,
		help,
		translationsOptional = false,
		children,
		status
	}: {
		/** the toon of the page ('' for platform-wide pages like Characters) */
		toon: string;
		title: string;
		docs: ContentDoc[];
		companions?: () => ContentDoc[];
		beforeSave?: () => void;
		/** replaces the per-file validation (e.g. credits, checked against the people + roles files) */
		validate?: () => Problem[];
		/** full-width content (editor + preview side by side) */
		wide?: boolean;
		/** the content fills the page's height without padding (a canvas: the map builder; with `usePageNav({ fill })`) */
		fill?: boolean;
		/** extra items at the start of the ActionBar (page-level actions, e.g. "New character") */
		actions?: Snippet;
		/** the page's help in the ActionBar's Info panel */
		help: HelpPage;
		/** a missing translation is no problem (the script: the page offers to start one itself) – no "create" bars */
		translationsOptional?: boolean;
		children: Snippet;
		/** right end of the file status line (e.g. the script's line / column) – no row of its own */
		status?: Snippet;
	} = $props();

	const panelSize = panelWidth('sc-panel-width:meta');

	const main = $derived(docs[0]);
	/** undo/redo act on the most recently edited doc */
	const undoDoc = $derived(docs.filter((d) => d.canUndo).sort((a, b) => b.lastEditAt - a.lastEditAt)[0] ?? main);
	const redoDoc = $derived(docs.filter((d) => d.canRedo).sort((a, b) => b.lastEditAt - a.lastEditAt)[0] ?? main);
	let historyPath = $state<string | null>(null);
	const historyDoc = $derived(docs.find((d) => d.entity === historyPath) ?? main);
	const loaded = $derived(docs.every((d) => !d.loading));
	const session = new DocSession({ docs: () => docs, companions: () => companions(), beforeSave: () => beforeSave(), errorCount: () => errorCount });
	const anyDirty = $derived(session.anyDirty);
	const anySaving = $derived(session.anySaving);
	let panel = $state<'problems' | 'history' | null>(null);
	// with the Ask panel open too, the sidebar folds to its rail (the page keeps room)
	$effect(() => {
		pagePanel.open = panel !== null;
		return () => (pagePanel.open = false);
	});
	function togglePanel(next: 'problems' | 'history') {
		panel = panel === next ? null : next;
	}
	const problems = $derived.by<Problem[]>(() => {
		const out: Problem[] = [];
		if (validate) {
			if (docs.some((d) => d.loading)) return out;
			const order = { error: 0, warning: 1, info: 2 };
			return validate().sort((a, b) => order[a.level] - order[b.level]);
		}
		for (const doc of docs) {
			if (doc.loading) continue;
			// a language-neutral season: once + its texts per language (agents/NEUTRAL.md)
			if (doc.texts) {
				out.push(...validateNeutral(doc.kind, doc.data[doc.primary], doc.texts, { file: doc.entity }));
				continue;
			}
			for (const l of doc.locales) {
				if (doc.data[l] == null) {
					out.push({ level: l === doc.primary ? 'error' : 'warning', file: doc.entity, locale: l, path: [], message: `${m['creator.missing']()}: ${l}` });
					continue;
				}
				out.push(...validateFile(doc.kind, doc.data[l], l, { file: doc.entity }));
			}
			out.push(...validateLocales(doc.kind, doc.entity, doc.data, doc.primary));
		}
		const order = { error: 0, warning: 1, info: 2 };
		return out.sort((a, b) => order[a.level] - order[b.level]);
	});
	const errorCount = $derived(problems.filter((p) => p.level === 'error').length);
	// accepted warnings ("Looks fine") don't count
	const warningCount = $derived(problems.filter((p) => p.level === 'warning' && !isAccepted(docs, p)).length);

	// unsaved changes: leaving asks in-page (lib/editor/ask.ts)
	beforeNavigate(leaveGuard(() => anyDirty));

	$effect(() => {
		const list = docs;
		for (const d of list) void JSON.stringify(d.data);
		const timer = setTimeout(() => list.forEach((d) => d.autosave()), 1000);
		return () => clearTimeout(timer);
	});

	/** Ctrl+S saves (with the note dialog); undo / redo outside a field (lib/editor/editor-keys.ts) */
	const onKeydown = (event: KeyboardEvent) => editorKeys(event, { save: () => session.saveDraft(), undo: () => undoDoc?.undo(), redo: () => redoDoc?.redo() });
</script>

<svelte:window onkeydown={onKeydown} />

<div class="meta-page" class:with-panel={panel !== null} class:wide class:fill style:--panel-width={panelSize.css}>
	<ActionBar label={m['creator.page_actions']()} moreLabel={m['creator.more']()}>
		{#snippet start()}
			<button class="ghost" class:active={panel === 'problems'} onclick={() => togglePanel('problems')} data-testid="problems-toggle">
				{m['creator.problems']()} <span class:bad={errorCount > 0}>{errorCount}</span>/<span>{warningCount}</span>
			</button>
			{@render actions?.()}
		{/snippet}
		{#snippet secondary()}
			<button class="ghost" title="{m['creator.undo']()} (Ctrl+Z)" aria-label={m['creator.undo']()} disabled={!undoDoc?.canUndo} onclick={() => undoDoc?.undo()} data-testid="undo">↶</button>
			<button class="ghost" title="{m['creator.redo']()} (Ctrl+Shift+Z)" aria-label={m['creator.redo']()} disabled={!redoDoc?.canRedo} onclick={() => redoDoc?.redo()} data-testid="redo">↷</button>
			<button class="ghost" class:active={panel === 'history'} onclick={() => togglePanel('history')}>{m['creator.history']()}</button>
		{/snippet}
		{#snippet end()}
			<input class="message" type="text" placeholder="note (optional)" bind:value={session.message} />
			<button class:secondary={!anyDirty} onclick={() => session.saveDraft()} disabled={anySaving} title={anyDirty ? undefined : m['creator.nothing_to_save']()} data-testid="save-draft">
				{anySaving ? m['creator.saving']() : m['creator.save_draft']()}{anyDirty ? ' •' : ''}
			</button>
			<button class="secondary" onclick={() => session.publishAll()} disabled={anySaving} data-testid="publish">{publishLabel(toon)}</button>
		{/snippet}
	</ActionBar>

	{#if !loaded}
		<p class="pad">{m['creator.loading']()}</p>
	{:else}
		{#if docs.some(hasStatus) || status}
		<div class="files" data-testid="file-status">
			{#each docs as d (d.entity)}
				<FileStatus doc={d} label={fileLabel(d.entity)} />
			{/each}
			{#if status}<span class="status-end">{@render status()}</span>{/if}
		</div>
		{/if}
		<DocNotices {session} {docs} dirty={anyDirty} {translationsOptional} />
		<main class="content">
			{@render children()}
		</main>
	{/if}

	{#if panel}
			<!-- a focusable separator is a widget (WAI-ARIA window splitter): drag the panel wider (owner 2026-10-03) -->
			<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
			<div class="panel-resize" role="separator" aria-orientation="vertical" aria-label={m['creator.resize_panel']()} title={m['creator.resize_panel']()} tabindex="0" onpointerdown={panelSize.start} onkeydown={panelSize.key} ondblclick={panelSize.reset}></div>
		<aside class="side-panel">
			{#if panel === 'problems'}
				<ProblemsPanel {problems} {docs} home={main?.entity} onclose={() => (panel = null)} />
			{:else if panel === 'history' && historyDoc}
				{#if docs.length > 1}
					<p class="history-files">
						{#each docs as d (d.entity)}
							<button class="ghost" class:active={d === historyDoc} onclick={() => (historyPath = d.entity)}>{fileLabel(d.entity)}</button>
						{/each}
					</p>
				{/if}
				{#key historyDoc}<HistoryPanel doc={historyDoc} onclose={() => (panel = null)} />{/key}
			{/if}
		</aside>
	{/if}
</div>

<style>
	/* the ActionBar space is reserved by the app's AppShell (@scutoons/ui), not here;
	   --top (the top bar's height) comes from the layout */
	.status-end {
		margin-left: auto;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.75rem;
	}
	.files {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
		align-items: center;
		padding: 0.35rem 0.75rem;
		border-bottom: 1px solid var(--border-subtle);
	}
	.message {
		width: 11rem !important;
	}
	.bad {
		color: var(--error);
	}
	.content {
		padding: 1rem;
		max-width: 70rem;
	}
	.wide .content {
		max-width: none;
	}
	.meta-page.fill {
		height: 100%;
		display: flex;
		flex-direction: column;
	}
	.fill .content {
		flex: 1 1 auto;
		min-height: 0;
		padding: 0;
		max-width: none;
	}
	.history-files {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
		margin: 0 0 0.5rem;
	}
	.with-panel .content {
		margin-right: var(--panel-width);
	}
	.side-panel {
		position: fixed;
		top: var(--top);
		/* left of the Ask panel: Ask belongs to the shell, the page's panels to the page */
		right: var(--ask-inset, 0rem);
		bottom: var(--action-bar-height);
		width: var(--panel-width);
		background: var(--paper);
		border-left: var(--border-width-regular) solid var(--ink);
		overflow: auto;
		padding: var(--space-1) var(--space-1) var(--space-3);
		z-index: 20;
		box-sizing: border-box;
	}
	.pad {
		padding: 1rem;
	}
	/* the side panel's left edge drags it wider (@scutoons/ui panelWidth) */
	.panel-resize {
		position: fixed;
		z-index: 21;
		top: var(--top);
		bottom: var(--action-bar-height);
		right: calc(var(--ask-inset, 0rem) + var(--panel-width) - 0.375rem);
		width: 0.75rem;
		cursor: col-resize;
		touch-action: none;
	}
	.panel-resize:hover,
	.panel-resize:focus-visible {
		background: linear-gradient(var(--ink), var(--ink)) center / var(--border-width-focus) 100% no-repeat;
		outline: none;
	}
	@media (max-width: 767px) {
		.panel-resize {
			display: none;
		}
	}
</style>
