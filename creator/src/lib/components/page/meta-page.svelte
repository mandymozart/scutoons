<script lang="ts">
	import { leaveGuard } from '$lib/editor/ask.ts';
	import type { Snippet } from 'svelte';
	import { ActionBar, Icon, PhoneBar, shellDock, useDock, type DockContent, type PhoneBarItem } from '@scutoons/ui';
	import { beforeNavigate } from '$app/navigation';
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { DocSession } from '$lib/editor/doc-session.svelte.ts';
	import { publishLabel } from '$lib/editor/reviews.svelte.ts';
	import type { PublishScope } from '$lib/editor/publish-tree.ts';
	import { editorKeys } from '$lib/editor/editor-keys.ts';
	import { validateFile, validateLocales, validateNeutral, type Problem } from '@scutoons/content';
	import DocNotices from '$lib/components/page/doc-notices.svelte';
	import { isAccepted } from '$lib/editor/accepted.ts';
	import ProblemsDock from '$lib/components/page/problems-dock.svelte';
	import HistoryPanel from '$lib/components/page/history-panel.svelte';
	import PageHelp, { type HelpPage } from '$lib/components/page/page-help.svelte';
	import { fileLabel } from '$lib/labels.ts';
	import { dockWidth, flowWidth } from '$lib/nav.svelte.ts';
	import { MediaQuery } from 'svelte/reactivity';

	/**
	 * Editor shell for toon-level entities (season, toon settings): top bar with their versions, the bottom ActionBar
	 * (problems · undo/redo · history · note + Save draft / Publish), autosave + recover, missing-locale bars (only the blocking ones stay in the flow – E.1), the
	 * leave guard and Ctrl+S / Ctrl+Z (outside a field). `docs` are validated, saved and published; `companions()` are
	 * files that follow them (the chapter files whose status mirrors the season) – saved when dirty, published with the
	 * docs; `along()` are files edited here along with them (a score's custom collage presets) – companions that also keep
	 * a local copy while dirty and offer it back on the next load. Save / publish / conflict:
	 * lib/editor/doc-session.svelte.ts; the bars: DocNotices.
	 */
	let {
		toon,
		title,
		docs,
		companions = () => [],
		along = () => [],
		beforeSave = () => {},
		validate,
		wide = false,
		fill = false,
		actions,
		help,
		children,
		status,
		dock,
		dockOpen = $bindable(false),
		phoneMore = [],
		phoneSelection = null,
		tree
	}: {
		/** the page's level in the tree: Publish lists every file below it with a draft (BACKLOG 10.60, publish-tree.ts) */
		tree?: () => { scope: PublishScope; title: string } | null;
		/** phones: the page's own entries in the bar's "More" group (the map builder's Import) */
		phoneMore?: PhoneBarItem[];
		/** phones: the selected object's own row in the bar (UI-CONCEPTS E.10, like CapCut's clip toolbar) */
		phoneSelection?: { label: string; items: PhoneBarItem[]; onback: () => void } | null;
		/** the toon of the page ('' for platform-wide pages like Characters) */
		toon: string;
		title: string;
		docs: ContentDoc[];
		companions?: () => ContentDoc[];
		/** companions edited here (research/COLLAGE.md §12.4): autosaved only while dirty, offered back at load */
		along?: () => ContentDoc[];
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
		 * next to Problems / History (10.57) – the page makes room for every open one. Mounted by the shell (`kind:
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
	const session = new DocSession({ docs: () => docs, companions: () => [...companions(), ...along()], beforeSave: () => beforeSave(), errorCount: () => errorCount, tree: () => tree?.() ?? null });
	const anyDirty = $derived(session.anyDirty);
	const anySaving = $derived(session.anySaving);
	// Problems the last page docked stay open here (taken over, not remounted – UI-CONCEPTS E.3 b); Problems, History
	// and the page's Flow can be open together (10.57), each its own panel side by side
	const open = $state({ problems: shellDock.has('problems'), history: false });
	// docked in the shell's right dock (agents/DOCKING.md): the page makes room; with Ask open too the sidebar folds. The
	// Flow and Problems are mounted by the shell: the next page of the same kind takes them over
	const problemsProps = () => ({ problems, docs, home: main?.entity, onclose: () => (open.problems = false) });
	useDock(() => (dockOpen && dock ? { kind: 'flow', content: dock, size: dockSize, title: m['creator.flow'](), icon: 'git-merge', close: () => (dockOpen = false) } : null));
	useDock(() =>
		open.problems
			? { kind: 'problems', content: { component: ProblemsDock, props: problemsProps }, size: panelSize, title: m['creator.problems'](), icon: 'alert-triangle', close: () => (open.problems = false) }
			: null
	);
	useDock(() => (open.history ? { kind: 'history', panel: sidePanel, size: panelSize, title: m['creator.history'](), icon: 'clock', close: () => (open.history = false) } : null));
	function togglePanel(next: 'problems' | 'history') {
		open[next] = !open[next];
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

	/**
	 * The bar (UI-CONCEPTS E.10, owner 2026-10-11, like CapCut; every width since 2026-10-12 – "phone grouping"): Save ·
	 * History (undo, redo, versions) · Publish · More (problems, the page's own entries, the note, the page's actions);
	 * on a desktop Problems leads the row (its count as the badge) and the More group drops it
	 */
	const phoneWidth = new MediaQuery('max-width: 767px', false);
	const problemsItem = $derived<PhoneBarItem>({
		id: 'problems',
		icon: 'alert-triangle',
		label: m['creator.problems'](),
		badge: errorCount + warningCount || undefined,
		active: open.problems,
		run: () => togglePanel('problems'),
		testid: 'problems-toggle'
	});
	const phoneItems = $derived<PhoneBarItem[]>([
		...(phoneWidth.current ? [] : [problemsItem]),
		{ id: 'save', icon: 'save', label: anySaving ? m['creator.saving']() : m['creator.save_short'](), primary: anyDirty, badge: anyDirty ? '•' : undefined, disabled: anySaving, run: () => session.saveDraft(), testid: 'save-draft' },
		{
			id: 'history',
			icon: 'clock',
			label: m['creator.history'](),
			items: [
				{ id: 'undo', icon: 'rotate-ccw', label: m['creator.undo'](), disabled: !undoDoc?.canUndo, run: () => undoDoc?.undo(), stay: true, testid: 'undo' },
				{ id: 'redo', icon: 'rotate-cw', label: m['creator.redo'](), disabled: !redoDoc?.canRedo, run: () => redoDoc?.redo(), stay: true, testid: 'redo' },
				{ id: 'versions', icon: 'list', label: m['creator.bar_versions'](), active: open.history, run: () => togglePanel('history'), testid: 'history-versions' }
			]
		},
		// a dot while saved drafts are not live yet (owner 2026-10-11: the state on Save / Publish, no status chip in More)
		{ id: 'publish', icon: 'upload-cloud', label: publishLabel(toon), disabled: anySaving, badge: docs.some((d) => d.unpublished.length) ? '•' : undefined, run: () => session.publishAll(), testid: 'publish' },
		{
			id: 'more',
			icon: 'more-horizontal',
			label: m['creator.more'](),
			badge: errorCount || undefined,
			items: [
				...(phoneWidth.current ? [problemsItem] : []),
				...phoneMore,
				...(actions ? [{ id: 'actions', label: m['creator.page_actions'](), content: actions }] : [])
			]
		}
	]);

	// unsaved changes: leaving asks in-page (lib/editor/ask.ts)
	beforeNavigate(leaveGuard(() => anyDirty));

	$effect(() => {
		// the docs always (a clean one drops its copy); the files edited along only while dirty – their copy outlives a
		// clean load until its Recover bar is answered (the chapter editor's rule for the presets)
		const list = [...docs, ...along().filter((d) => d.dirty)];
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
		{#snippet phone()}
			<PhoneBar items={phoneItems} selection={phoneSelection} label={m['creator.bar_actions']()} backLabel={m['creator.bar_back']()} />
		{/snippet}
	</ActionBar>

	{#if !loaded}
		<p class="pad">{m['creator.loading']()}</p>
	{:else}
		<!-- the page's own status line (the script's rules link + cursor); the files' status is the ActionBar's chip (E.1) -->
		{#if status}<div class="files" data-testid="page-status"><span class="status-end">{@render status()}</span></div>{/if}
		<DocNotices {session} docs={[...docs, ...along()]} />
		<main class="content">
			{@render children()}
		</main>
	{/if}

</div>



<!-- the History panel, shown in the shell's right dock (useDock) – the Flow and Problems are FlowDock / ProblemsDock -->
{#snippet sidePanel()}
	<div class="side-panel">
		{#if open.history && historyDoc}
			{#if docs.length > 1}
				<p class="history-files">
					{#each docs as d (d.entity)}
						<button class="ghost" class:active={d === historyDoc} onclick={() => (historyPath = d.entity)}>{fileLabel(d.entity)}</button>
					{/each}
				</p>
			{/if}
			{#key historyDoc}<HistoryPanel doc={historyDoc} onclose={() => (open.history = false)} />{/key}
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
