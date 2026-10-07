<script lang="ts">
	import { leaveGuard } from '$lib/editor/ask.ts';
	import { untrack, type Snippet } from 'svelte';
	import { ActionBar, shellDock, useDock, type DockContent } from '@scutoons/ui';
	import { beforeNavigate } from '$app/navigation';
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { DocSession } from '$lib/editor/doc-session.svelte.ts';
	import { publishLabel } from '$lib/editor/reviews.svelte.ts';
	import { editorKeys } from '$lib/editor/editor-keys.ts';
	import { validateFile, validateLocales, validateNeutral, type Problem } from '@scutoons/content';
	import DocNotices from '$lib/components/page/doc-notices.svelte';
	import { isAccepted } from '$lib/editor/accepted.ts';
	import ProblemsDock from '$lib/components/page/problems-dock.svelte';
	import HistoryPanel from '$lib/components/page/history-panel.svelte';
	import LiveChip from '$lib/components/status/live-chip.svelte';
	import PageHelp, { type HelpPage } from '$lib/components/page/page-help.svelte';
	import { fileLabel } from '$lib/labels.ts';
	import { dockWidth, flowWidth } from '$lib/nav.svelte.ts';

	/**
	 * Editor shell for toon-level entities (season, toon settings): top bar with their versions, the bottom ActionBar
	 * (problems · undo/redo · history · note + Save draft / Publish), autosave + recover, missing-locale bars (only the blocking ones stay in the flow – E.1), the
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
		children,
		status,
		dock,
		dockOpen = $bindable(false)
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
		children: Snippet;
		/** right end of the file status line (e.g. the script's line / column) – no row of its own */
		status?: Snippet;
		/**
		 * the page's Flow (the season's `FlowDock` + its props), docked where Problems / History are: shown while `dockOpen`;
		 * it and Problems / History take turns – the page makes room for whichever is open. Mounted by the shell (`kind:
		 * 'flow'`): the next page's Flow takes it over (UI-CONCEPTS E.3 b)
		 */
		dock?: DockContent;
		dockOpen?: boolean;
	} = $props();

	const panelSize = dockWidth('meta', 24);
	const dockSize = flowWidth();

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
	// Problems the last page docked stay open here (taken over, not remounted – UI-CONCEPTS E.3 b)
	let panel = $state<'problems' | 'history' | null>(shellDock.has('problems') ? 'problems' : null);
	/** what is docked on the right: Problems / History, or the page's own dock */
	const shown = $derived(dockOpen && dock ? 'dock' : panel);
	const size = $derived(shown === 'dock' ? dockSize : panelSize);
	// the page's dock opens: Problems / History close
	$effect(() => {
		if (dockOpen) untrack(() => (panel = null));
	});
	// docked in the shell's right dock (agents/DOCKING.md): the page makes room; with Ask open too the sidebar folds
	const dockTitle = $derived(shown === 'dock' ? m['creator.flow']() : shown === 'history' ? m['creator.history']() : m['creator.problems']());
	const dockIcon = $derived(shown === 'dock' ? 'git-merge' : shown === 'history' ? 'clock' : 'alert-triangle');
	const problemsProps = () => ({ problems, docs, home: main?.entity, onclose: () => (panel = null) });
	useDock(() => {
		if (!shown) return null;
		const docked = { size, title: dockTitle, icon: dockIcon, close: () => ((panel = null), (dockOpen = false)) };
		// the Flow and Problems are mounted by the shell: the next page of the same kind takes them over
		if (shown === 'dock' && dock) return { ...docked, kind: 'flow', content: dock };
		if (shown === 'problems') return { ...docked, kind: 'problems', content: { component: ProblemsDock, props: problemsProps } };
		return { ...docked, panel: sidePanel };
	});
	function togglePanel(next: 'problems' | 'history') {
		dockOpen = false;
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
				// a season: its published positions are frozen (BACKLOG 10.41)
				const publishedSequence = doc.kind === 'season' ? doc.publishedSequences[doc.primary] : undefined;
				out.push(...validateNeutral(doc.kind, doc.data[doc.primary], doc.texts, { file: doc.entity, publishedSequence }));
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

<div class="meta-page" class:wide class:fill>
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
			<input class="message in-end" type="text" placeholder={m['creator.note_label']()} aria-label={m['creator.note_label']()} bind:value={session.message} />
			<button class:secondary={!anyDirty} onclick={() => session.saveDraft()} disabled={anySaving} title={anyDirty ? undefined : m['creator.nothing_to_save']()} data-testid="save-draft">
				{#if anySaving}{m['creator.saving']()}{:else}<span class="save-long">{m['creator.save_draft']()}</span><span class="save-short">{m['creator.save_short']()}</span>{/if}{anyDirty ? ' •' : ''}
			<input class="message in-start" type="text" placeholder={m['creator.note_label']()} aria-label={m['creator.note_label']()} bind:value={session.message} />
			</button>
			<LiveChip files={docs.map((d) => ({ doc: d, label: fileLabel(d.entity) }))} />
			<button class="secondary" onclick={() => session.publishAll()} disabled={anySaving} data-testid="publish">{publishLabel(toon)}</button>
		{/snippet}
	</ActionBar>

	{#if !loaded}
		<p class="pad">{m['creator.loading']()}</p>
	{:else}
		<!-- the page's own status line (the script's rules link + cursor); the files' status is the ActionBar's chip (E.1) -->
		{#if status}<div class="files" data-testid="page-status"><span class="status-end">{@render status()}</span></div>{/if}
		<DocNotices {session} {docs} />
		<main class="content">
			{@render children()}
		</main>
	{/if}

</div>


<!-- the History panel, shown in the shell's right dock (useDock) – the Flow and Problems are FlowDock / ProblemsDock -->
{#snippet sidePanel()}
	<div class="side-panel">
		{#if panel === 'history' && historyDoc}
			{#if docs.length > 1}
				<p class="history-files">
					{#each docs as d (d.entity)}
						<button class="ghost" class:active={d === historyDoc} onclick={() => (historyPath = d.entity)}>{fileLabel(d.entity)}</button>
					{/each}
				</p>
			{/if}
			{#key historyDoc}<HistoryPanel doc={historyDoc} onclose={() => (panel = null)} />{/key}
		{/if}
	</div>
{/snippet}
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
	.save-short {
		display: none;
	/* the bar fits at every width ≥ 375 (as the chapter editor): below 1280 "Save" and the note in "More" (the ActionBar's
	   breakpoint is 1024 here – between them the note sits in the bar's start); the chip drops its state word below 1600 */
	}
	@media (max-width: 1279px) {
		.save-long,
		.message.in-end {
			display: none !important;
		}
		.save-short {
			display: inline;
		}
	}
	.bad {
		color: var(--error);
	}
	.content {
		padding: 1rem;
		max-width: 70rem;
	@media (min-width: 1280px) {
		.message.in-start {
			display: none !important;
		}
	}
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
	/* the open page panel (in the shell's right dock: it sizes and places it) */
	.side-panel {
		box-sizing: border-box;
		overflow: auto;
		padding: var(--space-1) var(--space-1) var(--space-3);
	}
	.pad {
		padding: 1rem;
	}
</style>
