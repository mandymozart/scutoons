<script lang="ts">
	import type { Snippet } from 'svelte';
	import { ActionBar } from '@scutoons/ui';
	import { beforeNavigate } from '$app/navigation';
	import { m } from '$lib/paraglide/messages.js';
	import { ContentDoc, SaveConflict } from '$lib/editor/doc.svelte.ts';
	import { validateFile, validateLocales, type Problem } from '@scutoons/content';
	import ProblemsPanel from './problems-panel.svelte';
	import HistoryPanel from './history-panel.svelte';
	import FileStatus from './file-status.svelte';

	/**
	 * Editor shell for toon-level files (season, toon.json): top bar with the files' versions, the bottom ActionBar
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
		children
	}: {
		toon: string;
		title: string;
		docs: ContentDoc[];
		companions?: () => ContentDoc[];
		beforeSave?: () => void;
		children: Snippet;
	} = $props();

	const main = $derived(docs[0]);
	const loaded = $derived(docs.every((d) => !d.loading));
	const anyDirty = $derived(docs.some((d) => d.dirty) || companions().some((d) => d.dirty));
	const anySaving = $derived(docs.some((d) => d.saving));
	let panel = $state<'problems' | 'history' | null>(null);
	let notice = $state<string | null>(null);
	let conflict = $state<{ doc: ContentDoc; error: SaveConflict } | null>(null);
	let saveMessage = $state('');
	/** companions saved on this page (published together with the docs) */
	let savedCompanions: ContentDoc[] = [];

	const problems = $derived.by<Problem[]>(() => {
		const out: Problem[] = [];
		for (const doc of docs) {
			if (doc.loading) continue;
			for (const l of doc.locales) {
				if (doc.data[l] == null) {
					out.push({ level: l === doc.primary ? 'error' : 'warning', file: doc.path, locale: l, path: [], message: `no ${l} file` });
					continue;
				}
				out.push(...validateFile(doc.kind, doc.data[l], l, { file: doc.path }));
			}
			out.push(...validateLocales(doc.kind, doc.path, doc.data, doc.primary));
		}
		const order = { error: 0, warning: 1, info: 2 };
		return out.sort((a, b) => order[a.level] - order[b.level]);
	});
	const errorCount = $derived(problems.filter((p) => p.level === 'error').length);
	const warningCount = $derived(problems.filter((p) => p.level === 'warning').length);

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
				else notice = `${doc.path}: ${e instanceof Error ? e.message : String(e)}`;
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
		if (!files.length) return;
		if (errorCount && !confirm(`${errorCount} errors in the problem list. Publish anyway?`)) return;
		const locales = [...new Set(files.flatMap((d) => [...d.dirtyLocales, ...d.unpublished]))].join(', ');
		if (!confirm(m['creator.publish_confirm']({ file: files.map((d) => d.path).join(', '), locales }))) return;
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
			saveAll();
		} else if (key === 'z' && !isField(event.target)) {
			event.preventDefault();
			if (event.shiftKey) main?.redo();
			else main?.undo();
		} else if (key === 'y' && !isField(event.target)) {
			event.preventDefault();
			main?.redo();
		}
	}
	function isField(target: EventTarget | null) {
		const el = target as HTMLElement | null;
		return !!el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT');
	}

	const recoverables = $derived(docs.filter((d) => d.recoverable));
	const missingLocales = $derived(
		docs.flatMap((d) => (d.loading || d.data[d.primary] == null ? [] : d.locales.filter((l) => l !== d.primary && d.data[l] == null).map((l) => ({ doc: d, locale: l }))))
	);
</script>

<svelte:window onkeydown={onKeydown} />

<div class="meta-page" class:with-panel={panel !== null}>
	<header class="topbar">
		<nav class="crumbs">
			<a href="/">{m['creator.title']()}</a> / <a href="/{toon}">{toon}</a> / <strong>{title}</strong>
		</nav>
		<div class="files">
			{#each docs as d (d.path)}
				{#if !d.loading}<span class="hint">{d.path}</span><FileStatus doc={d} />{/if}
			{/each}
		</div>
	</header>

	<ActionBar label={m['creator.page_actions']()} moreLabel={m['creator.more']()}>
		{#snippet start()}
			<a class="bar-link" href="/{toon}" data-testid="back-to-toon">← {toon}</a>
			<button class="ghost" class:active={panel === 'problems'} onclick={() => (panel = panel === 'problems' ? null : 'problems')} data-testid="problems-toggle">
				{m['creator.problems']()} <b class:bad={errorCount > 0}>{errorCount}</b>/<span>{warningCount}</span>
			</button>
		{/snippet}
		{#snippet secondary()}
			<button class="ghost" title="{m['creator.undo']()} (Ctrl+Z)" disabled={!main?.canUndo} onclick={() => main?.undo()} data-testid="undo">↶</button>
			<button class="ghost" title="{m['creator.redo']()} (Ctrl+Shift+Z)" disabled={!main?.canRedo} onclick={() => main?.redo()} data-testid="redo">↷</button>
			<button class="ghost" class:active={panel === 'history'} onclick={() => (panel = panel === 'history' ? null : 'history')}>{m['creator.history']()}</button>
		{/snippet}
		{#snippet end()}
			<input class="message" type="text" placeholder="note (optional)" bind:value={saveMessage} />
			<button onclick={() => saveAll()} disabled={!anyDirty || anySaving} data-testid="save-draft">
				{anySaving ? m['creator.saving']() : m['creator.save_draft']()}{anyDirty ? ' •' : ''}
			</button>
			<button class="secondary" onclick={publishAll} disabled={anySaving} data-testid="publish">{m['creator.publish']()}</button>
		{/snippet}
	</ActionBar>

	{#if !loaded}
		<p class="pad">{m['creator.loading']()}</p>
	{:else}
		{#if anyDirty}<div class="bar dirty-bar">{m['creator.unsaved']()}</div>{/if}
		{#each recoverables as d (d.path)}
			<div class="bar warn-bar">
				{d.path}: {m['creator.recover_title']({ time: new Date(d.recoverable!.at).toLocaleString() })}
				{#if d.recoverableIsStale}<b>{m['creator.recover_stale']()}</b>{/if}
				<button class="secondary" onclick={() => d.recover()}>{m['creator.recover']()}</button>
				<button class="ghost" onclick={() => d.clearAutosave()}>{m['creator.discard']()}</button>
			</div>
		{/each}
		{#each missingLocales as { doc, locale } (doc.path + locale)}
			<div class="bar warn-bar">
				{doc.path}: {locale} {m['creator.missing']()}
				<button class="secondary" onclick={() => doc.createLocale(locale, (f) => (f && typeof f === 'object' && 'language' in f ? { ...f, language: locale } : f))}>
					{m['creator.create_locale']({ locale, source: doc.primary })}
				</button>
			</div>
		{/each}
		{#if conflict}
			<div class="bar error-bar">
				{m['creator.conflict']({ locale: conflict.error.locale, file: conflict.doc.path })}
				<button class="secondary" onclick={() => resolveConflict('reload')}>{m['creator.reload']()}</button>
				<button class="secondary" onclick={() => resolveConflict('overwrite')}>{m['creator.overwrite']()}</button>
			</div>
		{/if}
		{#if notice}<div class="bar note-bar">{notice} <button class="ghost" onclick={() => (notice = null)}>✕</button></div>{/if}
		<main class="content">
			{@render children()}
		</main>
	{/if}

	{#if panel}
		<aside class="side-panel">
			{#if panel === 'problems'}
				<ProblemsPanel {problems} onclose={() => (panel = null)} />
			{:else if panel === 'history' && main}
				<HistoryPanel doc={main} onclose={() => (panel = null)} />
			{/if}
		</aside>
	{/if}
</div>

<style>
	.meta-page {
		--top: 64px;
		--action-bar-height: var(--top);
		padding-bottom: var(--action-bar-height);
	}
	.topbar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.4rem 1rem;
		min-height: var(--top);
		padding: 0.4rem 0.75rem 0.4rem 96px;
		border-bottom: 2px solid var(--ink);
		box-sizing: border-box;
	}
	.crumbs a,
	.bar-link {
		color: var(--ink);
		white-space: nowrap;
	}
	.files {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
		align-items: center;
	}
	.message {
		width: 11rem !important;
	}
	.bad {
		color: var(--error);
	}
	.bar {
		padding: 0.3rem 0.75rem;
		font-size: 12px;
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
		background: #fff8e6;
	}
	.error-bar {
		background: #fff0ee;
		border-bottom-color: var(--error);
	}
	.note-bar {
		background: #eafaff;
	}
	.content {
		padding: 1rem 1rem 3rem;
		max-width: 70rem;
	}
	.with-panel .content {
		margin-right: min(24rem, 92vw);
	}
	.side-panel {
		position: fixed;
		top: var(--top);
		right: 0;
		bottom: var(--action-bar-height);
		width: min(24rem, 92vw);
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
</style>
