<script lang="ts">
	import { onMount } from 'svelte';
	import { ActionBar, ToonAvatar } from '@scutoons/ui';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { newChapter, newSeason, newToon } from '$lib/editor/defaults.ts';
	import { contentIndex } from '$lib/api.ts';
	import { toonDoc, toonIndexDoc } from '$lib/editor/languages.ts';
	import { chapterSyncChanges, seasonStatusOf, setSeasonStatus, STATUSES, syncChapter } from '$lib/editor/visibility.ts';
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
	/** chapter numbers that have a file in the content store (any locale), in the season or not */
	let storedChapters = $state<number[]>([]);
	let busy = $state(false);
	let message = $state<string | null>(null);
	let error = $state<string | null>(null);
	let addLanguage = $state<string>('');
	/** chapter docs saved on this page because their status follows the season (published with the season) */
	let followersSaved = $state<ContentDoc[]>([]);

	/** the edited (maybe unsaved) languages */
	const edited = $derived<ToonLanguages | null>(index && !index.loading ? toonLanguages(index.data[NEUTRAL_LOCALE]) : null);
	const targets = $derived(edited ? edited.languages.filter((l) => l !== edited.source) : []);
	const addable = $derived(edited ? ALL_LANGUAGES.filter((l) => !edited.languages.includes(l)) : []);
	const languagesChanged = $derived(!!edited && !!langs && edited.languages.join(',') !== langs.languages.join(','));

	/** navigation avatar: the index entry's `avatar`, else the convention `<toon>/avatar.webp` */
	const avatarSrc = $derived(asset(`${toonId}/${index?.data?.[NEUTRAL_LOCALE]?.avatar || 'avatar.webp'}`));

	const sequence = $derived<SequenceItem[]>(season?.data?.[season.primary]?.sequence ?? []);
	const pageDocs = $derived([index, season, toonFile].filter((d): d is ContentDoc => !!d));
	/** no season1.json in any language (store or static): the page offers "Create season 1" */
	const seasonMissing = $derived(!!season && !season.loading && !season.error && season.present.length === 0);
	/** chapter numbers in the season of any language */
	const seasonNumbers = $derived(
		season ? [...new Set(season.present.flatMap((l) => (season!.data[l]?.sequence ?? []).map((i: SequenceItem) => Number(i.number))))] : []
	);
	/** chapters with a file in the store but in no season sequence ("Add to season") */
	const orphans = $derived(storedChapters.filter((n) => !seasonNumbers.includes(n)).sort((a, b) => a - b));
	/** chapter files whose (deprecated) status must follow the season on save, for older readers */
	const followers = $derived<ContentDoc[]>(
		season && !season.loading
			? sequence
					.filter((item) => chapters[item.number] && !chapters[item.number].loading && chapterSyncChanges(chapters[item.number], season, item.number).length)
					.map((item) => chapters[item.number])
			: []
	);
	const anyDirty = $derived(pageDocs.some((d) => d.dirty) || followers.length > 0);

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
		// chapters of the season (every language) + chapter files in the store that no season lists
		let stored: number[] = [];
		try {
			stored = [...new Set((await contentIndex(toonId)).files.map((f) => Number(f.path.match(/^chapter(\d+)\.json$/)?.[1])).filter((n) => n > 0))];
		} catch {
			/* index unavailable: the season's chapters only */
		}
		storedChapters = stored;
		const numbers = new Set<number>(stored);
		for (const l of nextSeason.present) for (const item of nextSeason.data[l]?.sequence ?? []) numbers.add(Number(item.number));
		const docs: Record<number, ContentDoc> = {};
		await Promise.all(
			[...numbers].map(async (number) => {
				const chapter = toonDoc(toonId, `chapter${number}.json`, languages);
				await chapter.load();
				docs[number] = chapter;
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
	/** authors of the toon's chapter files (for new season / toon files) */
	function chapterAuthors(): string[] {
		const names = Object.values(chapters).flatMap((d) => d.present.map((l) => d.data[l]?.author).filter((a): a is string => !!a));
		return [...new Set(names)];
	}

	/**
	 * Starts season1.json (and toon.json when it is missing) in every language of the toon, as unsaved docs. A chapter
	 * must always belong to a season: chapter files already in the store (e.g. created before the season) go into the
	 * new sequence as draft entries. Returns the numbers it added, or null when the files aren't loaded.
	 */
	function startSeason(): number[] | null {
		if (!season || !toonFile || !langs) return null;
		const entry = index?.data?.[NEUTRAL_LOCALE] ?? null;
		const authors = chapterAuthors();
		const adopted = [...orphans];
		const draft = (number: number): SequenceItem => ({ type: 'chapter', number, status: Status.DRAFT });
		season.create((locale) => ({ ...newSeason(locale, authors), sequence: adopted.map(draft) }));
		if (toonFile.present.length === 0) toonFile.create((locale) => newToon(toonId, locale, entry, authors));
		return adopted;
	}

	/** "Create season 1": the season (+ toon.json if missing) saved as drafts, existing chapter files adopted */
	async function createSeason() {
		message = null;
		error = null;
		const adopted = startSeason();
		if (!adopted || !season || !toonFile) {
			error = m['creator.not_loaded']();
			return;
		}
		busy = true;
		try {
			const files = [season, toonFile].filter((d) => d.dirty);
			for (const doc of files) await doc.save(`create ${doc.path}`);
			message = [
				m['creator.season_created']({ files: files.map((d) => d.path).join(', ') }),
				adopted.length ? m['creator.chapters_adopted']({ numbers: adopted.join(', ') }) : ''
			].join(' ');
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}

	/** A new chapter, always inside the season: chapter file + season entry (+ a new season / toon.json) in one save. */
	async function createChapter() {
		message = null;
		error = null;
		if (!season || !toonFile || !langs) {
			error = m['creator.not_loaded']();
			return;
		}
		// the next free number: after every chapter of the season and every chapter file in the store
		const highest = Math.max(0, ...seasonNumbers, ...storedChapters);
		const number = highest + 1;
		const last = chapters[highest];
		const author = last?.data?.[last.primary]?.author ?? chapterAuthors()[0] ?? 'Mandy Mozart';
		const adopted = seasonMissing ? startSeason() : [];
		if (adopted === null) {
			error = m['creator.not_loaded']();
			return;
		}
		busy = true;
		try {
			const chapter = toonDoc(toonId, `chapter${number}.json`, langs);
			chapter.create((locale) => newChapter(number, locale, author));
			season.structural((file) => {
				file.sequence ??= [];
				file.sequence.push({ type: 'chapter', number, status: Status.DRAFT });
			});
			await chapter.save(`new chapter ${number}`);
			await season.save(`add chapter ${number}`);
			if (toonFile.dirty) await toonFile.save(`create ${toonFile.path}`);
			message = m['creator.new_chapter_created']({ number });
			await goto(`/${toonId}/chapter/${number}`);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}

	/** the chapter's status (visibility) in one locale – the same control as in the chapter editor's meta panel */
	function setStatus(number: number, locale: string, status: string) {
		if (season) setSeasonStatus(season, locale, number, status as Status);
	}

	/** Mirrors the season status into the chapter files and saves them as drafts; returns the changed chapter docs. */
	async function saveFollowers(): Promise<ContentDoc[]> {
		const changed: ContentDoc[] = [];
		for (const item of sequence) {
			const doc = chapters[item.number];
			if (!doc || doc.loading || !season) continue;
			if (syncChapter(doc, season, item.number) && doc.dirty) {
				await doc.save('status from the season');
				changed.push(doc);
			}
		}
		return changed;
	}

	// ── Save / publish: the toon's index entry (languages), the season, toon.json ───────────────────────
	async function saveAll(): Promise<boolean> {
		busy = true;
		error = null;
		try {
			const reload = languagesChanged;
			for (const doc of pageDocs) if (doc.dirty) await doc.save();
			followersSaved = [...followersSaved, ...(await saveFollowers()).filter((d) => !followersSaved.includes(d))];
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
		// chapter files whose status follows the season (changed now or saved so on this page) are published with it
		const follow = [...new Set([...followers, ...followersSaved])];
		const docs = [...pageDocs.filter((d) => d.dirty || d.unpublished.length), ...follow];
		if (!docs.length) return;
		const locales = [...new Set(docs.flatMap((d) => [...d.dirtyLocales, ...d.unpublished]))].join(', ');
		if (!confirm(m['creator.publish_confirm']({ file: docs.map((d) => d.path).join(', '), locales }))) return;
		if (!(await saveAll())) return;
		busy = true;
		try {
			for (const doc of [...pageDocs, ...followersSaved]) if (doc.stored.length && doc.unpublished.length) await doc.publish();
			followersSaved = [];
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
		<a class="btn credits-link" href="/{toonId}/credits" title={m['creator.credits_open_hint']()} data-testid="edit-credits">{m['creator.credits_open']()} →</a>
		<a class="btn secondary" href="/{toonId}/cast" title={m['creator.cast_open_hint']()} data-testid="edit-cast">{m['creator.cast']()} →</a>
		<a class="btn secondary" href="/characters?toon={toonId}" data-testid="open-characters">{m['creator.characters']()} →</a>
		<span class="hint credits-hint">{m['creator.credits_open_hint']()}</span>
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
			{:else if seasonMissing}
				<section class="season-missing" data-testid="season-missing">
					<h2>{m['creator.season']({ number: 1 })}</h2>
					<p>{m['creator.season_missing']()}</p>
					<button disabled={busy || languagesChanged} onclick={createSeason} data-testid="create-season">+ {m['creator.create_season']()}</button>
				</section>
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
						<a class="btn" href="/{toonId}/season/1" data-testid="edit-season">{m['creator.edit_season']()}</a>
						{#if toonFile && !toonFile.loading && toonFile.present.length}<a class="btn" href="/{toonId}/toon" data-testid="edit-toon">{m['creator.edit_toon']()}</a>{/if}
					</div>
				</div>

				<h3>{m['creator.chapters']()}</h3>
				<table class="chapters">
					<thead>
						<tr>
							<th>#</th>
							{#each season.locales as l}<th>{l}{l === season.primary ? ` (${m['creator.source']()})` : ''}</th>{/each}
							<th>release</th>
							{#each season.locales as l}<th>{m['creator.status']()} {l}</th>{/each}
							<th>file</th><th></th>
						</tr>
					</thead>
					<tbody>
						{#each sequence as item (item.number)}
							{@const doc = chapters[item.number]}
							<tr>
								<td>{item.number}</td>
								{#each season.locales as l}
									<td>{doc?.data?.[l]?.title ?? '–'}</td>
								{/each}
								<td>{doc?.data?.[doc.primary]?.release ?? ''}</td>
								{#each season.locales as l}
									{@const status = seasonStatusOf(season, l, item.number)}
									<td>
										{#if status}
											<select value={status} onchange={(e) => setStatus(item.number, l, e.currentTarget.value)} data-testid="status-{item.number}-{l}">
												{#each STATUSES as s}<option value={s}>{s}</option>{/each}
											</select>
										{:else}
											<span class="hint">–</span>
										{/if}
									</td>
								{/each}
								<td>{#if doc}<FileStatus {doc} compact />{/if}</td>
								<td><a class="btn" href="/{toonId}/chapter/{item.number}" data-testid="open-chapter-{item.number}">{m['creator.open']()}</a></td>
							</tr>
						{/each}
					</tbody>
				</table>
				<p class="hint">{m['creator.status_hint']({ file: season.path })} {m['creator.status_follow_chapters']()}</p>
			{/if}
			{#if season && !season.loading && !seasonMissing && orphans.length}
				<p class="error" data-testid="orphans">{m['creator.orphans']({ numbers: orphans.join(', '), file: season.path })}</p>
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
			{#if error}<span class="error" data-testid="page-error">{error}</span>{:else if message}<span class="hint">{message}</span>{/if}
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
	.credits-link {
		margin-left: 1rem;
	}
	@media (max-width: 700px) {
		.credits-hint {
			display: none;
		}
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
	.season-missing {
		border: 2px dashed var(--border-muted);
		padding: 1rem;
		margin: 1rem 0;
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
