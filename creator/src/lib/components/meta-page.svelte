<script lang="ts">
	import { panelWidth } from '$lib/panel-width.svelte.ts';
	import { askDialog, confirmDialog } from '@scutoons/ui';
	import type { Snippet } from 'svelte';
	import { ActionBar, CloseIcon } from '@scutoons/ui';
	import { beforeNavigate } from '$app/navigation';
	import { m } from '$lib/paraglide/messages.js';
	import { ContentDoc, SaveConflict } from '$lib/editor/doc.svelte.ts';
	import { NEUTRAL_LOCALE, validateFile, validateLocales, type Problem } from '@scutoons/content';
	import ProblemsPanel from './problems-panel.svelte';
	import HistoryPanel from './history-panel.svelte';
	import FileStatus, { hasStatus } from './file-status.svelte';
	import PageHelp, { type HelpPage } from './page-help.svelte';
	import { fileLabel, fileLabels } from '$lib/labels.ts';

	/**
	 * Editor shell for toon-level entities (season, toon settings): top bar with their versions, the bottom ActionBar
	 * (problems · undo/redo · history · note + Save draft / Publish), autosave + recover, missing-locale bars, the
	 * leave guard and Ctrl+S / Ctrl+Z. `docs` are validated, saved and published; `companions()` are files that follow
	 * them (the chapter files whose status mirrors the season) – saved when dirty, published with the docs.
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
	const anyDirty = $derived(docs.some((d) => d.dirty) || companions().some((d) => d.dirty));
	const anySaving = $derived(docs.some((d) => d.saving));
	let panel = $state<'problems' | 'history' | null>(null);
	function togglePanel(next: 'problems' | 'history') {
		panel = panel === next ? null : next;
	}
	let notice = $state<string | null>(null);
	let conflict = $state<{ doc: ContentDoc; error: SaveConflict } | null>(null);
	let saveMessage = $state('');
	/** companions saved on this page (published together with the docs) */
	let savedCompanions: ContentDoc[] = [];

	const problems = $derived.by<Problem[]>(() => {
		const out: Problem[] = [];
		if (validate) {
			if (docs.some((d) => d.loading)) return out;
			const order = { error: 0, warning: 1, info: 2 };
			return validate().sort((a, b) => order[a.level] - order[b.level]);
		}
		for (const doc of docs) {
			if (doc.loading) continue;
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
	const warningCount = $derived(problems.filter((p) => p.level === 'warning').length);

	/** the button: never a silent no-op (owner 2026-09-30: "Save draft / Publish don't work" on a page without changes) */
	async function saveClicked() {
		beforeSave();
		if (!anyDirty) {
			notice = m['creator.nothing_to_save']();
			return;
		}
		// without a note, ask for one – it can be skipped (owner 2026-10-03)
		if (!saveMessage.trim()) {
			const note = await askDialog({
				title: m['creator.note_title'](),
				message: m['creator.note_message'](),
				label: m['creator.note_label'](),
				placeholder: m['creator.note_placeholder'](),
				confirmLabel: m['creator.note_save'](),
				skipLabel: m['creator.note_skip']()
			});
			if (note === null) return;
			saveMessage = note;
		}
		await saveAll();
	}

	async function saveAll(force = false) {
		notice = null;
		conflict = null;
		beforeSave();
		for (const doc of [...docs, ...companions()]) {
			if (!doc.dirty) continue;
			try {
				await doc.save(saveMessage.trim() || undefined, force);
				if (!docs.includes(doc) && !savedCompanions.includes(doc)) savedCompanions.push(doc);
			} catch (e) {
				if (e instanceof SaveConflict) conflict = { doc, error: e };
				else notice = `${fileLabel(doc.entity)}: ${e instanceof Error ? e.message : String(e)}`;
				return false;
			}
		}
		saveMessage = '';
		notice = m['creator.saved']();
		return true;
	}

	async function publishAll() {
		beforeSave();
		const follow = [...new Set([...companions().filter((d) => d.dirty), ...savedCompanions])];
		const files = [...docs.filter((d) => d.dirty || d.unpublished.length), ...follow];
		if (!files.length) {
			notice = m['creator.nothing_to_publish']();
			return;
		}
		if (errorCount && !(await confirmDialog({ title: m['creator.publish_title'](), message: m['creator.publish_errors']({ count: errorCount }), confirmLabel: m['creator.publish']() }))) return;
		// language-neutral entities (cast, characters …) have no locale to name
		const locales = [...new Set(files.flatMap((d) => [...d.dirtyLocales, ...d.unpublished]))].filter((l) => l !== NEUTRAL_LOCALE).join(', ');
		const file = fileLabels(files.map((d) => d.entity));
		if (!(await confirmDialog({ title: m['creator.publish_title'](), message: locales ? m['creator.publish_confirm']({ file, locales }) : m['creator.publish_confirm_neutral']({ file }), confirmLabel: m['creator.publish']() }))) return;
		if (!(await saveAll())) return;
		try {
			for (const doc of [...docs, ...savedCompanions]) if (doc.stored.length && doc.unpublished.length) await doc.publish();
			savedCompanions = [];
			notice = m['creator.published']();
		} catch (e) {
			notice = e instanceof Error ? e.message : String(e);
		}
	}

	async function resolveConflict(action: 'reload' | 'overwrite') {
		if (!conflict) return;
		const { doc } = conflict;
		conflict = null;
		if (action === 'reload') await doc.load();
		else await saveAll(true);
	}

	beforeNavigate((navigation) => {
		if (!anyDirty) return;
		if (navigation.type === 'leave') {
			navigation.cancel();
			return;
		}
		if (!confirm(m['creator.leave_confirm']())) navigation.cancel();
	});

	$effect(() => {
		const list = docs;
		for (const d of list) void JSON.stringify(d.data);
		const timer = setTimeout(() => list.forEach((d) => d.autosave()), 1000);
		return () => clearTimeout(timer);
	});

	function onKeydown(event: KeyboardEvent) {
		if (!(event.ctrlKey || event.metaKey)) return;
		const key = event.key.toLowerCase();
		if (key === 's') {
			event.preventDefault();
			saveClicked();
		} else if (key === 'z' && !isField(event.target)) {
			event.preventDefault();
			if (event.shiftKey) redoDoc?.redo();
			else undoDoc?.undo();
		} else if (key === 'y' && !isField(event.target)) {
			event.preventDefault();
			redoDoc?.redo();
		}
	}
	function isField(target: EventTarget | null) {
		const el = target as HTMLElement | null;
		return !!el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT');
	}

	const recoverables = $derived(docs.filter((d) => d.recoverable));
	const missingLocales = $derived(
		translationsOptional ? [] : docs.flatMap((d) => (d.loading || d.data[d.primary] == null ? [] : d.locales.filter((l) => l !== d.primary && d.data[l] == null).map((l) => ({ doc: d, locale: l }))))
	);
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
			<input class="message" type="text" placeholder="note (optional)" bind:value={saveMessage} />
			<button class:secondary={!anyDirty} onclick={saveClicked} disabled={anySaving} title={anyDirty ? undefined : m['creator.nothing_to_save']()} data-testid="save-draft">
				{anySaving ? m['creator.saving']() : m['creator.save_draft']()}{anyDirty ? ' •' : ''}
			</button>
			<button class="secondary" onclick={publishAll} disabled={anySaving} data-testid="publish">{m['creator.publish']()}</button>
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
		{#if anyDirty}<div class="bar dirty-bar">{m['creator.unsaved']()}</div>{/if}
		{#each recoverables as d (d.entity)}
			<div class="bar warn-bar">
				{fileLabel(d.entity)}: {m['creator.recover_title']({ time: new Date(d.recoverable!.at).toLocaleString() })}
				{#if d.recoverableIsStale}<strong class="sc-emphasis">{m['creator.recover_stale']()}</strong>{/if}
				<button class="secondary" onclick={() => d.recover()}>{m['creator.recover']()}</button>
				<button class="ghost" onclick={() => d.clearAutosave()}>{m['creator.discard']()}</button>
			</div>
		{/each}
		{#each missingLocales as { doc, locale } (doc.entity + locale)}
			<div class="bar warn-bar">
				{fileLabel(doc.entity)}: {locale} {m['creator.missing']()}
				<button class="secondary" onclick={() => doc.createLocale(locale, (f) => (f && typeof f === 'object' && 'language' in f ? { ...f, language: locale } : f))}>
					{m['creator.create_locale']({ locale, source: doc.primary })}
				</button>
			</div>
		{/each}
		{#if conflict}
			<div class="bar error-bar">
				{m['creator.conflict']({ locale: conflict.error.locale, file: fileLabel(conflict.doc.entity) })}
				<button class="secondary" onclick={() => resolveConflict('reload')}>{m['creator.reload']()}</button>
				<button class="secondary" onclick={() => resolveConflict('overwrite')}>{m['creator.overwrite']()}</button>
			</div>
		{/if}
		{#if notice}<div class="bar note-bar">{notice} <button class="ghost" onclick={() => (notice = null)} aria-label={m['creator.remove']()} title={m['creator.remove']()}><CloseIcon /></button></div>{/if}
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
				<ProblemsPanel {problems} onclose={() => (panel = null)} />
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
	.bar {
		padding: 0.3rem 0.75rem;
		display: flex;
		gap: 0.5rem;
		align-items: center;
		border-bottom: 1px solid var(--border-subtle);
	}
	.dirty-bar {
		background: var(--ink);
		color: var(--paper);
	}
	.warn-bar {
		border-left: 0.5rem solid var(--state-warning);
	}
	.error-bar {
		border-left: 0.5rem solid var(--state-error);
		border-bottom-color: var(--error);
	}
	.note-bar {
		border-left: 0.5rem solid var(--primary-cyan);
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
		right: 0;
		bottom: var(--action-bar-height);
		width: var(--panel-width);
		background: var(--paper);
		border-left: 2px solid var(--ink);
		overflow: auto;
		padding: 0.5rem 0.75rem 2rem;
		z-index: 20;
		box-sizing: border-box;
	}
	.pad {
		padding: 1rem;
	}
	/* the side panel's left edge drags it wider (lib/panel-width.svelte.ts) */
	.panel-resize {
		position: fixed;
		z-index: 21;
		top: var(--top);
		bottom: var(--action-bar-height);
		right: calc(var(--panel-width) - 0.375rem);
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
