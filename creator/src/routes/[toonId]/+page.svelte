<script lang="ts">
	import { onMount } from 'svelte';
	import { ActionBar, ToonAvatar } from '@scutoons/ui';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { newChapter } from '$lib/editor/defaults.ts';
	import { toonDoc, toonIndexDoc } from '$lib/editor/languages.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { Language, NEUTRAL_LOCALE, Status, toonLanguages, withToonLanguages } from '@scutoons/content';
	import type { SequenceItem, ToonLanguages } from '@scutoons/content';
	import StyleGuide from '$lib/components/style-guide.svelte';
	import FileStatus from '$lib/components/file-status.svelte';

	const toonId = $derived(page.params.toonId as string);

	/** Native names for the language picker (codes = the `Language` enum). */
	const LANGUAGE_NAMES: Record<string, string> = { de: 'Deutsch', en: 'English', es: 'Español', jp: '日本語', kr: '한국어' };
	const ALL_LANGUAGES = Object.values(Language) as Language[];
	const languageLabel = (l: string) => `${l} · ${LANGUAGE_NAMES[l] ?? l}`;

	/** the toon's index entry (`<toon>/_/index.json`): source language + languages */
	let index = $state<ContentDoc | null>(null);
	/** the languages the files below were loaded with (= the saved index entry) */
	let langs = $state<ToonLanguages | null>(null);
	let toonFile = $state<ContentDoc | null>(null);
	let season = $state<ContentDoc | null>(null);
	let chapters = $state<Record<number, ContentDoc>>({});
	let busy = $state(false);
	let message = $state<string | null>(null);
	let error = $state<string | null>(null);
	let addLanguage = $state<string>('');

	/** the edited (maybe unsaved) languages */
	const edited = $derived<ToonLanguages | null>(index && !index.loading ? toonLanguages(index.data[NEUTRAL_LOCALE]) : null);
	const targets = $derived(edited ? edited.languages.filter((l) => l !== edited.source) : []);
	const addable = $derived(edited ? ALL_LANGUAGES.filter((l) => !edited.languages.includes(l)) : []);
	const languagesChanged = $derived(!!edited && !!langs && edited.languages.join(',') !== langs.languages.join(','));

	/** navigation avatar: the index entry's `avatar`, else the convention `<toon>/avatar.webp` */
	const avatarSrc = $derived(asset(`${toonId}/${index?.data?.[NEUTRAL_LOCALE]?.avatar || 'avatar.webp'}`));

	const sequence = $derived<SequenceItem[]>(season?.data?.[season.primary]?.sequence ?? []);
	const pageDocs = $derived([index, season, toonFile].filter((d): d is ContentDoc => !!d));
	const anyDirty = $derived(pageDocs.some((d) => d.dirty));

	async function load() {
		const doc = toonIndexDoc(toonId);
		await doc.load();
		index = doc;
		await loadFiles(toonLanguages(doc.data[NEUTRAL_LOCALE]));
	}

	/** (Re)loads toon.json, the season and its chapters in the toon's languages. */
	async function loadFiles(languages: ToonLanguages) {
		langs = languages;
		const nextToon = toonDoc(toonId, 'toon.json', languages);
		const nextSeason = toonDoc(toonId, 'season1.json', languages);
		await Promise.all([nextToon.load(), nextSeason.load()]);
		toonFile = nextToon;
		season = nextSeason;
		const docs: Record<number, ContentDoc> = {};
		await Promise.all(
			(nextSeason.data[nextSeason.primary]?.sequence ?? []).map(async (item: SequenceItem) => {
				const chapter = toonDoc(toonId, `chapter${item.number}.json`, languages);
				await chapter.load();
				docs[item.number] = chapter;
			})
		);
		chapters = docs;
	}

	onMount(load);

	// ── Languages (toon level) ─────────────────────────────────────────────────────────────────────────
	function setLanguages(source: Language, languages: Language[]) {
		if (!index) return;
		const entry = index.data[NEUTRAL_LOCALE];
		if (entry == null) {
			// no index entry anywhere yet: start one (saved as the first version of <toon>/_/index.json)
			index.create(() => withToonLanguages({ id: toonId, uri: toonId, status: Status.DRAFT, i18n: {} }, source, languages));
			return;
		}
		const next = withToonLanguages(entry, source, languages);
		index.setLocal(NEUTRAL_LOCALE, ['sourceLanguage'], next.sourceLanguage, 'languages');
		index.setLocal(NEUTRAL_LOCALE, ['languages'], next.languages, 'languages');
	}

	function setSource(source: Language) {
		if (!edited) return;
		setLanguages(source, [source, ...edited.languages]);
	}

	function addTarget() {
		if (!edited || !addLanguage) return;
		setLanguages(edited.source, [...edited.languages, addLanguage as Language]);
		addLanguage = '';
	}

	function removeTarget(locale: Language) {
		if (!edited || locale === edited.source) return;
		setLanguages(edited.source, edited.languages.filter((l) => l !== locale));
	}

	// ── Missing locale files → "create from the source" (the editor's createLocale) ─────────────────────
	const fileDocs = $derived([toonFile, season, ...Object.values(chapters)].filter((d): d is ContentDoc => !!d && !d.loading));
	const missing = $derived.by(() => {
		const current = langs;
		if (!current) return [];
		// saved languages only, and not one that is being removed
		return current.languages
			.filter((l) => l !== current.source && (!edited || edited.languages.includes(l)))
			.map((locale) => ({ locale, docs: fileDocs.filter((d) => d.data[d.primary] != null && d.data[locale] == null) }))
			.filter((entry) => entry.docs.length > 0);
	});

	/** a copy of the source file for `locale` (files with a `language` field get the new one) */
	const localized = (locale: string) => (file: any) =>
		file && typeof file === 'object' && 'language' in file ? { ...file, language: locale } : file;

	async function createLocaleFiles(locale: string) {
		const current = langs;
		if (!current) return;
		busy = true;
		error = null;
		message = null;
		try {
			// the chapters' scene files (groups[].ref.src) too, so the chapter editor opens complete
			const scenes = new Set<string>();
			for (const doc of Object.values(chapters)) {
				for (const group of doc.data[doc.primary]?.groups ?? []) if (group?.ref?.src) scenes.add(group.ref.src);
			}
			const sceneDocs = await Promise.all(
				[...scenes].map(async (src) => {
					const doc = toonDoc(toonId, src, current);
					await doc.load();
					return doc;
				})
			);
			let count = 0;
			for (const doc of [...fileDocs, ...sceneDocs]) {
				if (doc.data[doc.primary] == null || doc.data[locale] != null) continue;
				doc.createLocale(locale, localized(locale));
				await doc.save(`create ${locale} from ${current.source}`);
				count++;
			}
			message = m['creator.locale_files_created']({ count, locale, source: current.source });
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}

	// ── Chapters ───────────────────────────────────────────────────────────────────────────────────────
	async function createChapter() {
		if (!season || !langs) return;
		const number = Math.max(0, ...sequence.map((s) => s.number)) + 1;
		const author = chapters[sequence[sequence.length - 1]?.number]?.data?.[season.primary]?.author ?? 'Mandy Mozart';
		busy = true;
		error = null;
		try {
			const chapter = toonDoc(toonId, `chapter${number}.json`, langs);
			chapter.create((locale) => newChapter(number, locale, author));
			await chapter.save(`new chapter ${number}`);
			season.structural((file) => {
				file.sequence ??= [];
				file.sequence.push({ type: 'chapter', number, status: Status.DRAFT });
			});
			await season.save(`add chapter ${number}`);
			message = m['creator.new_chapter_created']({ number });
			await goto(`/${toonId}/chapter/${number}`);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}

	function setStatus(index: number, status: string) {
		season?.setShared(['sequence', index, 'status'], status);
	}

	// ── Save / publish: the toon's index entry (languages), the season, toon.json ───────────────────────
	async function saveAll(): Promise<boolean> {
		busy = true;
		error = null;
		try {
			const reload = languagesChanged;
			for (const doc of pageDocs) if (doc.dirty) await doc.save();
			// changed languages: the files are edited in the new ones from now on
			if (reload && index) await loadFiles(toonLanguages(index.data[NEUTRAL_LOCALE]));
			return true;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
			return false;
		} finally {
			busy = false;
		}
	}

	async function publishAll() {
		const docs = pageDocs.filter((d) => d.dirty || d.unpublished.length);
		if (!docs.length) return;
		const locales = [...new Set(docs.flatMap((d) => [...d.dirtyLocales, ...d.unpublished]))].join(', ');
		if (!confirm(m['creator.publish_confirm']({ file: docs.map((d) => d.path).join(', '), locales }))) return;
		if (!(await saveAll())) return;
		busy = true;
		try {
			for (const doc of pageDocs) if (doc.stored.length && doc.unpublished.length) await doc.publish();
			message = m['creator.published']();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}
</script>

<div class="creator-page">
	<header class="creator-top">
		<ToonAvatar src={avatarSrc} name={toonId} />
		<nav class="crumbs"><a href="/">{m['creator.title']()}</a> / <strong>{toonId}</strong></nav>
	</header>
	<main class="browser">
		<section>
			{#if index && !index.loading && edited}
				<section class="languages" data-testid="languages">
					<div class="season-head">
						<h2>{m['creator.languages']()}</h2>
						<FileStatus doc={index} />
					</div>
					<p class="hint">{m['creator.languages_hint']()}</p>
					<div class="language-grid">
						<label class="field">
							<span>{m['creator.source_language']()}</span>
							<select value={edited.source} onchange={(e) => setSource(e.currentTarget.value as Language)} data-testid="source-language">
								{#each ALL_LANGUAGES as l}<option value={l}>{languageLabel(l)}</option>{/each}
							</select>
						</label>
						<div class="field">
							<span>{m['creator.target_languages']()}</span>
							<ul class="chips">
								{#each targets as l (l)}
									<li class="chip" data-testid="language-{l}">
										{languageLabel(l)}
										<button class="ghost" title={m['creator.make_source']({ locale: l })} onclick={() => setSource(l)}>★</button>
										<button class="ghost" title={m['creator.remove_language']({ locale: l })} onclick={() => removeTarget(l)} data-testid="remove-language-{l}">✕</button>
									</li>
								{:else}
									<li class="hint">{m['creator.no_target_languages']()}</li>
								{/each}
							</ul>
							{#if addable.length}
								<span class="add-language">
									<select bind:value={addLanguage} aria-label={m['creator.add_language_label']()} data-testid="add-language-select">
										<option value="">{m['creator.add_language_label']()} …</option>
										{#each addable as l}<option value={l}>{languageLabel(l)}</option>{/each}
									</select>
									<button class="secondary" disabled={!addLanguage} onclick={addTarget} data-testid="add-language">+ {m['creator.add_language']()}</button>
								</span>
							{/if}
						</div>
					</div>
					{#if languagesChanged}<p class="hint warn">{m['creator.languages_unsaved']()} {m['creator.languages_remove_note']()}</p>{/if}
					{#each missing as { locale, docs } (locale)}
						<p class="missing-bar" data-testid="missing-{locale}">
							{m['creator.missing_locale_files']({ locale, count: docs.length })}
							<span class="hint">{docs.map((d) => d.path).join(', ')}</span>
							<button class="secondary" disabled={busy} onclick={() => createLocaleFiles(locale)} data-testid="create-locale-{locale}">
								{m['creator.create_locale']({ locale, source: langs?.source ?? '' })}
							</button>
						</p>
					{/each}
				</section>
			{/if}
			{#if !season || season.loading}
				<p>{m['creator.loading']()}</p>
			{:else if season.error}
				<p class="error">{season.error}</p>
			{:else}
				<div class="season-head">
					<div>
						<h2>{m['creator.season']({ number: 1 })}: {season.data[season.primary]?.title}</h2>
						<p class="hint">
							{#each season.locales as l}{l}: {season.data[l]?.title ?? '–'} {/each}
						</p>
					</div>
					<div class="actions">
						<FileStatus doc={season} />
					</div>
				</div>

				<h3>{m['creator.chapters']()}</h3>
				<table class="chapters">
					<thead>
						<tr>
							<th>#</th>
							{#each season.locales as l}<th>{l}{l === season.primary ? ` (${m['creator.source']()})` : ''}</th>{/each}
							<th>release</th><th>status (season)</th><th>file</th><th></th>
						</tr>
					</thead>
					<tbody>
						{#each sequence as item, index (item.number)}
							{@const doc = chapters[item.number]}
							<tr>
								<td>{item.number}</td>
								{#each season.locales as l}
									<td>{doc?.data?.[l]?.title ?? '–'}</td>
								{/each}
								<td>{doc?.data?.[doc.primary]?.release ?? ''}</td>
								<td>
									<select value={item.status} onchange={(e) => setStatus(index, e.currentTarget.value)}>
										{#each Object.values(Status) as s}<option value={s}>{s}</option>{/each}
									</select>
								</td>
								<td>{#if doc}<FileStatus {doc} compact />{/if}</td>
								<td><a class="btn" href="/{toonId}/chapter/{item.number}" data-testid="open-chapter-{item.number}">{m['creator.open']()}</a></td>
							</tr>
						{/each}
					</tbody>
				</table>
			{/if}
			{#if message}<p class="hint">{message}</p>{/if}
			{#if error}<p class="error">{error}</p>{/if}
		</section>
		<aside>
			<StyleGuide />
		</aside>
	</main>
	<!-- Bottom action bar: back to the toons · new chapter · save / publish (languages, season, toon.json) -->
	<ActionBar label={m['creator.page_actions']()} moreLabel={m['creator.more']()}>
		{#snippet start()}
			<a class="bar-link" href="/">← {m['creator.toons']()}</a>
			{#if anyDirty}<span class="hint">{m['creator.unsaved']()}</span>{/if}
		{/snippet}
		{#snippet secondary()}
			<button onclick={createChapter} disabled={busy || !season || season.loading || languagesChanged} data-testid="new-chapter">+ {m['creator.new_chapter']()}</button>
		{/snippet}
		{#snippet end()}
			<button class="secondary" disabled={busy || !anyDirty} onclick={saveAll} data-testid="save-draft">{m['creator.save_draft']()}{anyDirty ? ' •' : ''}</button>
			<button class="secondary" disabled={busy || !pageDocs.some((d) => d.dirty || d.unpublished.length)} onclick={publishAll} data-testid="publish">{m['creator.publish']()}</button>
		{/snippet}
	</ActionBar>
</div>

<style>
	.creator-top {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		min-height: 64px;
		padding: 0 1rem 0 96px;
		border-bottom: 2px solid var(--ink);
	}
	.crumbs a,
	.bar-link {
		color: var(--ink);
	}
	.creator-page {
		--action-bar-height: 64px;
	}
	.browser {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 22rem;
		gap: 2rem;
		padding: 1.5rem 1rem;
		max-width: 1300px;
	}
	@media (max-width: 1000px) {
		.browser {
			grid-template-columns: 1fr;
		}
	}
	.season-head {
		display: flex;
		justify-content: space-between;
		gap: 1rem;
		align-items: flex-start;
	}
	.actions {
		display: flex;
		gap: 0.5rem;
		align-items: center;
		flex-wrap: wrap;
	}
	.chapters {
		width: 100%;
		border-collapse: collapse;
	}
	.chapters th,
	.chapters td {
		text-align: left;
		padding: 0.35rem 0.5rem;
		border-bottom: 1px solid var(--border-subtle);
	}
	.chapters th {
		border-bottom: 2px solid var(--ink);
		font-size: 11px;
	}
	.chapters tr:hover td {
		background: var(--surface-hover);
	}
	.chapters select {
		width: auto;
	}
	.error {
		color: var(--error);
	}
	.languages {
		margin-bottom: 2rem;
		padding-bottom: 1.5rem;
		border-bottom: 2px solid var(--ink);
	}
	.language-grid {
		display: flex;
		flex-wrap: wrap;
		gap: 1rem 2rem;
		align-items: flex-start;
	}
	.field {
		display: flex;
		flex: 1 1 12rem;
		min-width: 0;
		flex-direction: column;
		gap: 0.35rem;
	}
	.field > span {
		font-size: 11px;
		font-weight: 700;
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.chip {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.1rem 0.2rem 0.1rem 0.5rem;
		border: 2px solid var(--ink);
		white-space: nowrap;
	}
	.add-language {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		align-items: center;
	}
	.add-language select {
		width: auto;
	}
	.warn {
		color: var(--ink);
		border-left: 2px solid var(--primary-cyan);
		padding-left: 0.5rem;
	}
	.missing-bar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem;
		border: 2px dashed var(--ink);
	}
</style>
