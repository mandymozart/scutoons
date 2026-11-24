<script lang="ts">
	import { onMount, tick, untrack } from 'svelte';
	import { page } from '$app/state';
	import {
		kanonHeadings,
		kanonRole,
		kanonSubjectSegment,
		kanonTemplate,
		NEUTRAL_LOCALE,
		validateFile,
		validateLocales,
		withToonLanguages,
		type KanonSubject,
		type Language,
		type Problem,
		type ToonLanguages,
		type ToonScript
	} from '@scutoons/content';
	import { confirmDialog } from '@scutoons/ui';
	import { Button, ListRow } from '@scutoons/ui/editor';
	import { toonDoc, toonIndexDoc } from '$lib/editor/languages.ts';
	import { markedLanguage, textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { isEditKey } from '$lib/editor/script-edit.ts';
	import { wordCount } from '$lib/editor/writing.ts';
	import { usePageStatus, useTabs } from '$lib/shell/shell.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import MetaPage from '$lib/components/page/meta-page.svelte';
	import ProseEditor from '$lib/components/writing/prose-editor.svelte';
	import WritingCompare from '$lib/components/writing/writing-compare.svelte';
	import WritingPage from '$lib/components/writing/writing-page.svelte';
	import WritingStatus from '$lib/components/writing/writing-status.svelte';

	/**
	 * A Kanon text on the writing page (lap 2 C14; research/KANON.md §5): prose in the reading column, its `## ` sections the
	 * Outline (a panel, folded at rest), the Facts beside it with Versions; Compare (C14b) sets the source beside the edit
	 * language. Read-only until the member says they mean to edit, per visit; an empty text opens ready to type.
	 */
	let { entity, subject, toon, languages, title }: { entity: string; subject: KanonSubject; toon: string; languages: ToonLanguages; title: string } = $props();

	// svelte-ignore state_referenced_locally
	const doc = toonDoc(entity, languages);
	// svelte-ignore state_referenced_locally
	const role = kanonRole(subject);
	const requested = page.url.searchParams.get('lang');
	// svelte-ignore state_referenced_locally
	let locale = $state<string>(requested && doc.locales.includes(requested) ? requested : (textLanguage.chosen(doc.locales) ?? doc.primary));
	let followed = textLanguage.value;
	$effect(() => {
		if (textLanguage.value === followed) return;
		followed = textLanguage.value;
		const chosen = textLanguage.chosen(doc.locales) ?? doc.primary;
		untrack(() => switchLocale(chosen));
	});

	let editor = $state<ReturnType<typeof ProseEditor>>();
	let comparing = $state(false);
	let caret = $state(1);
	let progress = $state<{ done: number; total: number } | null>(null);
	const flush = () => editor?.flush();

	onMount(() => {
		doc.load();
	});

	// a text not written yet: an empty source, not dirty until something is typed (no "missing" noise)
	$effect(() => {
		if (doc.loading || doc.data[doc.primary] != null) return;
		untrack(() => {
			const empty: ToonScript = { text: '' };
			doc.data[doc.primary] = empty;
			doc.saved[doc.primary] = JSON.stringify(empty);
		});
	});

	const script = $derived<ToonScript | null>(doc.data[locale] ?? null);
	const text = $derived(typeof script?.text === 'string' ? script.text : '');
	const sourceText = $derived(typeof doc.data[doc.primary]?.text === 'string' ? (doc.data[doc.primary]!.text as string) : '');
	const outline = $derived(kanonHeadings(text, role));
	const current = $derived(outline.filter((h) => h.line <= caret).at(-1) ?? null);
	const words = $derived(wordCount(text));
	const translating = $derived(locale !== doc.primary);
	const showCompare = $derived(comparing && translating);

	function write(next: string) {
		// a translation written beside the source for the first time: its document starts with the first word
		if (!doc.data[locale]) {
			if (!next) return;
			doc.createLocale(locale, () => ({ text: '' }));
		}
		if (doc.data[locale].text !== next) doc.setLocal(locale, ['text'], next, `script-text:${locale}`);
	}

	let editing = $state(false);
	// an empty text has nothing to guard: it opens ready to type, the caret in the column
	let focused = false;
	$effect(() => {
		if (doc.loading || editing || text.trim() || (!script && !showCompare)) return;
		untrack(() => {
			editing = true;
			if (!focused) void tick().then(() => editor?.focus());
			focused = true;
		});
	});
	let asking = false;
	async function requestEdit() {
		if (editing || asking) return;
		asking = true;
		editing = await confirmDialog({ title: m['creator.kanon_edit_title'](), message: m['creator.kanon_edit_confirm'](), confirmLabel: m['creator.kanon_edit']() });
		asking = false;
		if (editing) editor?.focus();
	}
	function lockedKey(event: KeyboardEvent): boolean {
		if (!isEditKey(event)) return false;
		event.preventDefault();
		void requestEdit();
		return true;
	}

	/** an empty Kanon text gets its role's headings in the shown language (research/KANON.md §4) */
	function startWithHeadings() {
		if (text.trim()) return;
		write(kanonTemplate(role, locale));
		queueMicrotask(() => editor?.jump(1));
	}

	function switchLocale(next: string) {
		if (next === locale) return;
		flush();
		locale = next;
	}
	function startLocale(copy: boolean) {
		doc.createLocale(locale, copy ? undefined : () => ({ text: '' }));
	}

	/** a translation's text can make its language the toon's source – asked first, saved as a draft of the toon entry, the page reloads */
	let switching = $state(false);
	async function makeSource() {
		if (switching || locale === doc.primary || doc.dirty) return;
		const ok = await confirmDialog({ title: m['creator.make_source_title']({ locale }), message: m['creator.make_source_message']({ locale, source: doc.primary }), confirmLabel: m['creator.make_source']({ locale }) });
		if (!ok) return;
		switching = true;
		try {
			const entry = toonIndexDoc(toon);
			await entry.load();
			const next = withToonLanguages(entry.data[NEUTRAL_LOCALE] ?? {}, locale as Language, languages.languages);
			entry.setLocal(NEUTRAL_LOCALE, ['sourceLanguage'], next.sourceLanguage, 'languages');
			entry.setLocal(NEUTRAL_LOCALE, ['languages'], next.languages, 'languages');
			await entry.save(`source language ${locale}`);
			window.location.reload();
		} finally {
			switching = false;
		}
	}

	function validate(): Problem[] {
		const out: Problem[] = [];
		for (const l of doc.locales) if (doc.data[l] != null) out.push(...validateFile('kanon', doc.data[l], l, { file: doc.entity }));
		out.push(...validateLocales('kanon', doc.entity, doc.data, doc.primary));
		return out;
	}

	const kindWord = $derived(
		({ toon: m['creator.writing_kind_toon'], character: m['creator.writing_kind_character'], persona: m['creator.writing_kind_persona'], look: m['creator.writing_kind_look'], place: m['creator.writing_kind_place'] })[role]()
	);
	const sub = $derived(`${m['creator.kanon']()} · ${kindWord}`);
	const sourceSub = $derived(m['creator.writing_compare_source_head']({ locale: markedLanguage(doc.primary, doc.primary).toUpperCase() }));
	const targetSub = $derived(m['creator.writing_compare_target_head']({ locale: locale.toUpperCase() }));

	useTabs(() => ({ outline: outlinePanel, facts: factsPanel }));
	usePageStatus(() => (doc.loading ? null : status));
</script>

{#snippet outlinePanel()}
	<div class="panel-list" data-testid="writing-outline">
		{#each outline as h (h.line)}
			<ListRow dense label={h.heading} selected={h === current} class={h.id === null ? 'unknown' : ''} title={h.id === null ? `${h.heading} – ${m['creator.kanon_not_a_section']()}` : h.heading} onclick={() => editor?.jump(h.line)} data-testid="writing-outline-item" />
		{/each}
	</div>
{/snippet}

{#snippet factsPanel()}
	<dl class="facts" data-testid="writing-facts">
		<dt>{m['creator.writing_fact_subject']()}</dt><dd>{kanonSubjectSegment(subject)}</dd>
		<dt>{m['creator.writing_fact_kind']()}</dt><dd>{kindWord}</dd>
		<dt>{m['creator.writing_fact_languages']()}</dt><dd>{doc.locales.map((l) => markedLanguage(l, doc.primary).toUpperCase()).join(' · ')}</dd>
		<dt>{m['creator.writing_fact_words']()}</dt><dd>{words}</dd>
		<dt>{m['creator.writing_fact_sections']()}</dt><dd>{outline.length}</dd>
	</dl>
	{#if translating && script}
		<p class="facts-action"><Button variant="quiet" onclick={makeSource} disabled={switching || doc.dirty} title={doc.dirty ? m['creator.make_source_save_first']() : undefined} data-testid="script-make-source">{m['creator.make_source']({ locale })}</Button></p>
	{/if}
{/snippet}

{#snippet status()}
	<WritingStatus
		words={script ? words : null}
		languages={doc.locales}
		source={doc.primary}
		shown={locale}
		{comparing}
		oncompare={() => (comparing = !comparing)}
		saving={doc.saving}
		dirty={doc.dirty}
		savedAt={doc.lastSavedAt}
		lead={showCompare && progress ? m['creator.writing_paragraphs']({ done: progress.done, total: progress.total }) : undefined}
	>
		{#if !editing && script}<Button variant="quiet" icon="lock" title={m['creator.kanon_edit_confirm']()} onclick={requestEdit} data-testid="script-edit">{m['creator.kanon_edit']()}</Button>{/if}
	</WritingStatus>
{/snippet}

<MetaPage {toon} {title} docs={[doc]} {validate} wide help="kanon" beforeSave={flush}>
	{#if !script && !showCompare}
		<WritingPage title={title} {sub}>
			<p class="muted" data-testid="script-missing">{m['creator.kanon_missing_locale']({ locale, source: doc.primary })}</p>
			<p class="row">
				<Button onclick={() => startLocale(false)} data-testid="script-start-empty">{m['creator.script_start_empty']()}</Button>
				{#if sourceText}<Button variant="quiet" onclick={() => startLocale(true)} data-testid="script-copy-source">{m['creator.script_copy_source']({ source: doc.primary })}</Button>{/if}
			</p>
		</WritingPage>
	{:else if showCompare}
		<WritingCompare
			source={{ text: sourceText, locale: doc.primary, title, sub, lang: sourceSub }}
			target={{ text, locale, title, sub, lang: targetSub }}
			readonly={!editing}
			label={m['creator.kanon_editor_label']({ locale })}
			onchange={write}
			onlockedkey={lockedKey}
			onprogress={(done, total) => (progress = { done, total })}
		/>
	{:else}
		<WritingPage {title} {sub} testid="script-editor">
			{#if editing && !text.trim()}
				<p class="row"><Button variant="quiet" onclick={startWithHeadings} data-testid="kanon-start-headings">{m['creator.kanon_start_headings']()}</Button></p>
			{/if}
			{#key locale}
				<ProseEditor bind:this={editor} {text} readonly={!editing} label={m['creator.kanon_editor_label']({ locale })} lang={locale} placeholder={m['creator.kanon_placeholder']()} onchange={write} onlockedkey={lockedKey} oncaret={(l) => (caret = l)} />
			{/key}
		</WritingPage>
	{/if}
</MetaPage>

<style>
	.muted {
		color: var(--tool-muted);
	}
	.row {
		display: flex;
		gap: var(--editor-pad);
		margin: 0 0 1em;
		font-size: var(--editor-font);
	}
	.panel-list {
		display: flex;
		flex-direction: column;
	}
	.panel-list :global(.unknown) {
		color: var(--tool-muted);
	}
	.facts {
		display: grid;
		grid-template-columns: auto 1fr;
		gap: calc(var(--editor-pad) * 0.8) var(--editor-pad);
		margin: 0;
		padding: var(--editor-pad);
	}
	.facts dt {
		color: var(--tool-muted);
	}
	.facts dd {
		margin: 0;
		text-align: right;
	}
	.facts-action {
		margin: 0;
		padding: 0 var(--editor-pad) var(--editor-pad);
	}
</style>
