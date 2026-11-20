<script lang="ts">
	import { leaveGuard } from '$lib/editor/ask.ts';
	import type { Snippet } from 'svelte';
	import { Icon, useDock, type DockContent, type PhoneBarItem } from '@scutoons/ui';
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
	import ProblemsPanel from '$lib/components/page/problems-panel.svelte';
	import HistoryPanel from '$lib/components/page/history-panel.svelte';
	import type { HelpPage } from '$lib/components/page/page-help.svelte';
	import { fileLabel } from '$lib/labels.ts';
	import { flowWidth } from '$lib/nav.svelte.ts';
	import { usePageDoc } from '$lib/shell/shell.svelte.ts';

	/**
	 * The frame of a page's documents (season, toon settings, characters, the map …): it hands the shell its save state
	 * and its Publish · Problems · History as the Document panel's tabs (lib/shell), autosave + recover, missing-locale
	 * bars (only the blocking ones stay in the flow – E.1), the leave guard and Ctrl+S / Ctrl+Z (outside a field). `docs`
	 * are validated, saved and published; `companions()` are files that follow them (the chapter files whose status
	 * mirrors the season) – saved when dirty, published with the docs; `along()` are files edited here along with them (a
	 * score's custom collage presets) – companions that also keep a local copy while dirty and offer it back on the next
	 * load. Save / publish / conflict: lib/editor/doc-session.svelte.ts; the bars: DocNotices.
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
		tree,
		ongoto
	}: {
		/** a Problems row's "go to" (the page's own places – the storyboard's shots); without it a row only restores */
		ongoto?: (problem: Problem) => void;
		/** the page's level in the tree: Publish lists every file below it with a draft (BACKLOG 10.60, publish-tree.ts) */
		tree?: () => { scope: PublishScope; title: string } | null;
		/** the page's own entries in the Publish panel (the map builder's Import) */
		phoneMore?: PhoneBarItem[];
		/** the selected object's own actions in the Publish panel */
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
		/** page-level actions in the Publish panel (e.g. "New character") */
		actions?: Snippet;
		/** the page's help (Info) */
		help: HelpPage;
		children: Snippet;
		/** right end of the file status line (e.g. the script's line / column) – no row of its own */
		status?: Snippet;
		/**
		 * the page's Flow (the season's `FlowDock` + its props), docked in the page host's dock while `dockOpen`; mounted by
		 * the host (`kind: 'flow'`): the next page's Flow takes it over (UI-CONCEPTS E.3 b)
		 */
		dock?: DockContent;
		dockOpen?: boolean;
	} = $props();

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
	useDock(() => (dockOpen && dock ? { kind: 'flow', content: dock, size: dockSize, title: m['creator.flow'](), icon: 'git-merge', close: () => (dockOpen = false) } : null));

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
	const unpublished = $derived(docs.some((d) => d.unpublished.length));
	/** a page entry or a selection's action as a button (a group gives its actions) */
	const flat = (items: PhoneBarItem[]): PhoneBarItem[] => items.flatMap((i) => (i.items ? flat(i.items) : [i]));

	// the shell's bar shows the save state; the Document panel holds Publish · Problems · History
	usePageDoc(() => ({ dirty: anyDirty, saving: anySaving, errors: errorCount, warnings: warningCount, save: () => session.saveDraft(), publish: publishTab, problems: problemsTab, history: historyTab }));

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
	{#if !loaded}
		<p class="pad">{m['creator.loading']()}</p>
	{:else}
		<!-- the page's own status line (the script's rules link + cursor) -->
		{#if status}<div class="files" data-testid="page-status"><span class="status-end">{@render status()}</span></div>{/if}
		<DocNotices {session} docs={[...docs, ...along()]} />
		<main class="content">
			{@render children()}
		</main>
	{/if}
</div>

{#snippet publishTab()}
	<div class="publish" data-testid="publish-panel">
		<div class="row">
			<button type="button" class:secondary={!anyDirty} disabled={anySaving} onclick={() => session.saveDraft()} data-testid="save-draft">
				<Icon name="save" />{anySaving ? m['creator.saving']() : m['creator.save_short']()}{#if anyDirty}<span class="dot" aria-hidden="true"></span>{/if}
			</button>
			<button type="button" class="secondary" disabled={anySaving} onclick={() => session.publishAll()} data-testid="publish">
				<Icon name="upload-cloud" />{publishLabel(toon)}{#if unpublished}<span class="dot" aria-hidden="true"></span>{/if}
			</button>
		</div>
		<div class="row">
			<button type="button" class="ghost" disabled={!undoDoc?.canUndo} onclick={() => undoDoc?.undo()} data-testid="undo"><Icon name="rotate-ccw" />{m['creator.undo']()}</button>
			<button type="button" class="ghost" disabled={!redoDoc?.canRedo} onclick={() => redoDoc?.redo()} data-testid="redo"><Icon name="rotate-cw" />{m['creator.redo']()}</button>
		</div>
		{#if errorCount || warningCount}
			<p class="counts"><span class:bad={errorCount > 0}>{m['creator.problems_errors']({ count: errorCount })}</span> · {m['creator.problems_warnings']({ count: warningCount })}</p>
		{/if}
		{#if phoneSelection}
			<p class="label">{phoneSelection.label}</p>
			<div class="row wrap">
				{#each flat(phoneSelection.items) as item (item.id)}
					<button type="button" class="ghost" class:active={item.active} disabled={item.disabled} onclick={() => item.run?.()} data-testid={item.testid}>{#if item.icon}<Icon name={item.icon} />{/if}{item.label}</button>
				{/each}
			</div>
		{/if}
		{#if phoneMore.length || actions}
			<div class="row wrap">
				{#each flat(phoneMore) as item (item.id)}
					<button type="button" class="ghost" disabled={item.disabled} onclick={() => item.run?.()} data-testid={item.testid}>{#if item.icon}<Icon name={item.icon} />{/if}{item.label}</button>
				{/each}
				{#if actions}{@render actions()}{/if}
			</div>
		{/if}
	</div>
{/snippet}

{#snippet problemsTab()}
	<div class="side-panel"><ProblemsPanel {problems} {docs} home={main?.entity} {ongoto} bare /></div>
{/snippet}

{#snippet historyTab()}
	<div class="side-panel">
		{#if historyDoc}
			{#if docs.length > 1}
				<p class="history-files">
					{#each docs as d (d.entity)}
						<button class="ghost" class:active={d === historyDoc} onclick={() => (historyPath = d.entity)}>{fileLabel(d.entity)}</button>
					{/each}
				</p>
			{/if}
			{#key historyDoc}<HistoryPanel doc={historyDoc} bare />{/key}
		{/if}
	</div>
{/snippet}

<style>
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
	.side-panel {
		box-sizing: border-box;
		padding: 0 var(--space-1) var(--space-1);
	}
	.publish {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		padding: 0 var(--space-1) var(--space-1);
	}
	.row {
		display: flex;
		gap: var(--space-05);
	}
	.row.wrap {
		flex-wrap: wrap;
	}
	.dot {
		width: 0.45rem;
		height: 0.45rem;
		border-radius: 50%;
		background: var(--warn);
	}
	.counts,
	.label {
		margin: 0;
		color: var(--muted);
	}
	.bad {
		color: var(--error);
	}
	.pad {
		padding: 1rem;
	}
</style>
