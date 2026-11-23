<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Icon, type GroupBarItem } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { DocSession, useAutosave } from '$lib/editor/doc-session.svelte.ts';
	import { editorKeys } from '$lib/editor/editor-keys.ts';
	import { validateFile, validateLocales, validateNeutral, type Problem } from '@scutoons/content';
	import DocNotices from '$lib/components/page/doc-notices.svelte';
	import { isAccepted } from '$lib/editor/accepted.ts';
	import { useProblems } from '$lib/log/problems.svelte.ts';
	import { problemMessage, problemPlace } from '$lib/log/problem-words.ts';
	import { pageRedo, pageUndo } from '$lib/editor/history.svelte.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import type { HelpPage } from '$lib/components/page/page-help.svelte';
	import { usePageDoc } from '$lib/shell/shell.svelte.ts';

	/**
	 * The documents of a page (season, toon settings, characters, the map …): every edit saves on its own (useAutosave),
	 * it hands the shell what the bar's Publish asks of it and its History and its Problems (a panel tab), the local copy +
	 * recover, missing-locale bars (only the blocking ones stay in the flow – E.1) and Ctrl+S / Ctrl+Z (outside a field). `docs`
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
		more = [],
		selection = null,
		ongoto
	}: {
		/** a Problems row's "go to" (the page's own places – the storyboard's shots); without it a row only restores */
		ongoto?: (problem: Problem) => void;
		/** the page's own entries in its line (the map builder's Import) */
		more?: GroupBarItem[];
		/** the selected object's own actions in the page's line */
		selection?: { label: string; items: GroupBarItem[] } | null;
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
		/** page-level actions in the page's line (e.g. "New character") */
		actions?: Snippet;
		/** the page's help (Info) */
		help: HelpPage;
		children: Snippet;
		/** right end of the file status line (e.g. the script's line / column) – no row of its own */
		status?: Snippet;
	} = $props();

	const main = $derived(docs[0]);
	const creator = getCreatorSession();
	/** ↶ ↷: unsaved edits first, then the person's saved changes of this session (lib/editor/history.svelte.ts) */
	const undo = () => void pageUndo(docs, creator.user?.email ?? '');
	const redo = () => void pageRedo(docs);
	const loaded = $derived(docs.every((d) => !d.loading));
	const session = new DocSession({ docs: () => docs, companions: () => [...companions(), ...along()], beforeSave: () => beforeSave(), errorCount: () => errorCount });
	const anySaving = $derived(session.anySaving);
	useAutosave(session);

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
	const extras = $derived(!!selection || more.length > 0 || !!actions);
	/** a page entry or a selection's action as a button (a group gives its actions) */
	const flat = (items: GroupBarItem[]): GroupBarItem[] => items.flatMap((i) => (i.items ? flat(i.items) : [i]));

	// the bar: ↶ ↷, Publish (its dot; the page saved first) and History; the status bar: "saving…" and the counts; Problems is a panel tab
	usePageDoc(() => ({ saving: anySaving, errors: errorCount, warnings: warningCount, unpublished, ...session.publishHooks, docs, undo, redo }));
	useProblems(() => (loaded ? { problems, docs, home: main?.entity, place: (p) => problemPlace(p, main?.entity), message: problemMessage, ongoto } : null));

	$effect(() => {
		// the docs always (a clean one drops its copy); the files edited along only while dirty – their copy outlives a
		// clean load until its Recover bar is answered (the chapter editor's rule for the presets)
		const list = [...docs, ...along().filter((d) => d.dirty)];
		for (const d of list) void JSON.stringify(d.data);
		const timer = setTimeout(() => list.forEach((d) => d.autosave()), 1000);
		return () => clearTimeout(timer);
	});

	/** Ctrl+S saves at once (no wait for the pause); undo / redo outside a field (lib/editor/editor-keys.ts) */
	const onKeydown = (event: KeyboardEvent) => editorKeys(event, { save: () => void session.flush(), undo, redo });
</script>

<svelte:window onkeydown={onKeydown} />

<div class="meta-page" class:wide class:fill>
	{#if !loaded}
		<p class="pad">{m['creator.loading']()}</p>
	{:else}
		<!-- the page's own status line (the script's rules link + cursor) -->
		{#if status || extras}<div class="files" data-testid="page-status">{#if extras}{@render pageActions()}{/if}{#if status}<span class="status-end">{@render status()}</span>{/if}</div>{/if}
		<DocNotices {session} docs={[...docs, ...along()]} />
		<main class="content">
			{@render children()}
		</main>
	{/if}
</div>

<!-- the page's own actions (they lay in the bar's Publish menu until Publish became one dialog, RULES H18) -->
{#snippet pageActions()}
	<div class="page-actions" data-testid="page-actions">
		{#if selection}
			<p class="label">{selection.label}</p>
			<div class="row wrap">
				{#each flat(selection.items) as item (item.id)}
					<button type="button" class="ghost" class:active={item.active} disabled={item.disabled} onclick={() => item.run?.()} data-testid={item.testid}>{#if item.icon}<Icon name={item.icon} />{/if}{item.label}</button>
				{/each}
			</div>
		{/if}
		{#if more.length || actions}
			<div class="row wrap">
				{#each flat(more) as item (item.id)}
					<button type="button" class="ghost" disabled={item.disabled} onclick={() => item.run?.()} data-testid={item.testid}>{#if item.icon}<Icon name={item.icon} />{/if}{item.label}</button>
				{/each}
				{#if actions}{@render actions()}{/if}
			</div>
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
		border-bottom: 1px solid var(--tool-line);
	}
	/* the page's form in the stage's middle: the panels float beside it */
	.content {
		box-sizing: border-box;
		margin: 0 auto;
		padding: 1rem;
		max-width: 56rem;
	}
	.wide .content {
		max-width: none;
	}
	.meta-page.fill {
		height: 100%;
		display: flex;
		flex-direction: column;
	}
	/* a flex item with auto side margins shrinks to its content, and a canvas has none: the stage took 0 px */
	.fill .content {
		flex: 1 1 auto;
		min-height: 0;
		margin: 0;
		padding: 0;
		width: 100%;
		max-width: none;
	}
	.page-actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05);
	}
	.row {
		display: flex;
		gap: var(--space-05);
	}
	.row.wrap {
		flex-wrap: wrap;
	}
	.label {
		margin: 0;
		color: var(--tool-muted);
	}
	.pad {
		padding: 1rem;
	}
</style>
