<script lang="ts">
	import { confirmPublish } from '$lib/editor/doc-session.svelte.ts';
	import { confirmSubmit, publishLabel, reviews, submitForReview } from '$lib/editor/reviews.svelte.ts';
	import { onMount } from 'svelte';
	import { ActionBar, CloseIcon, Icon, ToonAvatar } from '@scutoons/ui';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { newChapterDoc, newSeasonDoc, sequenceStatus } from '$lib/editor/defaults.ts';
	import { contentIndex } from '$lib/api.ts';
	import { toonDoc, toonIndexDoc } from '$lib/editor/languages.ts';
	import { chapterSyncChanges, syncChapter } from '$lib/editor/visibility.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { entities, hasToonTexts, Language, NEUTRAL_LOCALE, parseEntityKey, previewText, Status, toonLanguages, withToonLanguages } from '@scutoons/content';
	import type { SequenceItem, ToonLanguages } from '@scutoons/content';
	import { fileLabel, fileLabels } from '$lib/labels.ts';
	import FileStatus from '$lib/components/status/file-status.svelte';
	import ChapterBadges from '$lib/components/status/chapter-badges.svelte';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { listLanguage, otherTexts } from '$lib/editor/text-language.svelte.ts';

	const toonId = $derived(page.params.toonId as string);
	const nav = getCreatorNav();
	/** the toon's display name in the UI language (never the id as a title) */
	const toonName = $derived(nav.toonName(toonId));

	/** Native names for the language picker (codes = the `Language` enum). */
	const LANGUAGE_NAMES: Record<string, string> = { de: 'Deutsch', en: 'English', es: 'Español', jp: '日本語', ko: '한국어' };
	const ALL_LANGUAGES = Object.values(Language) as Language[];
	const languageLabel = (l: string) => `${l} · ${LANGUAGE_NAMES[l] ?? l}`;

	/** the toon's index entry (`toon:<toon>`): source language + languages */
	let index = $state<ContentDoc | null>(null);
	/** the languages the files below were loaded with (= the saved index entry) */
	let langs = $state<ToonLanguages | null>(null);
	/** the toon still has per-language toon settings (`toon-settings:<toon>:<l>`, legacy – agents/NEUTRAL.md step 4) */
	let toonSettingsStored = $state(false);
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

	/** titles in lists: one language – the i18n switch's, else the source; the others in the tooltip (owner 2026-10-02) */
	const titleLocale = $derived(season ? listLanguage(season.textLanguages, season.textSource) : '');
	/** a title in the list's language (else the source's, else any) */
	const titleOf = (doc: ContentDoc | null | undefined) => (doc ? previewText(doc.get(doc.primary, ['title']), [titleLocale, doc.textSource]) : '');

	const sequence = $derived<SequenceItem[]>(season?.data?.[season.primary]?.sequence ?? []);
	// a season converted in memory (agents/NEUTRAL.md) is not saved from here: the Convert page / its editor stores it
	const pageDocs = $derived([index, season].filter((d): d is ContentDoc => !!d && !d.unconverted));
	/** no season 1 yet: the page offers "Create season 1" */
	const seasonMissing = $derived(!!season && !season.loading && !season.error && season.present.length === 0);
	/** chapter numbers in the season of any language */
	const seasonNumbers = $derived(
		season ? [...new Set(season.present.flatMap((l) => (season!.data[l]?.sequence ?? []).map((i: SequenceItem) => Number(i.number))))] : []
	);
	/** chapters with a file in the store but in no season sequence ("Add to season") */
	const orphans = $derived(storedChapters.filter((n) => !seasonNumbers.includes(n)).sort((a, b) => a - b));
	/** chapter documents with fields a language-neutral chapter no longer stores (status mirror, published flag): saved without them */
	const followers = $derived<ContentDoc[]>(
		season && !season.loading
			? sequence.filter((item) => chapters[item.number] && !chapters[item.number].loading && chapterSyncChanges(chapters[item.number]).length).map((item) => chapters[item.number])
			: []
	);
	/**
	 * documents still stored per language (converted in memory on load) or toon settings not in the toon entry yet: the
	 * Convert page converts the toon
	 */
	const unconverted = $derived(
		[season, ...Object.values(chapters)].some((d) => !!d?.unconverted) ||
			(toonSettingsStored && !!index && !index.loading && index.data[NEUTRAL_LOCALE] != null && !hasToonTexts(index.data[NEUTRAL_LOCALE]))
	);
	const anyDirty = $derived(pageDocs.some((d) => d.dirty) || followers.length > 0);

	async function load() {
		const doc = toonIndexDoc(toonId);
		await doc.load();
		index = doc;
		await loadFiles(toonLanguages(doc.data[NEUTRAL_LOCALE]));
	}

	/** (Re)loads the season and its chapters in the toon's languages. */
	async function loadFiles(languages: ToonLanguages) {
		langs = languages;
		const nextSeason = toonDoc(entities.season(toonId, 1), languages);
		await nextSeason.load();
		season = nextSeason;
		// chapters of the season (every language) + chapter files in the store that no season lists
		let stored: number[] = [];
		try {
			const refs = (await contentIndex(toonId)).entities.map((e) => parseEntityKey(e.key));
			stored = [...new Set(refs.filter((r) => r?.kind === 'chapter').map((r) => r!.number!))];
			toonSettingsStored = refs.some((r) => r?.kind === 'toon-settings' && r.toon === toonId);
		} catch {
			/* index unavailable: the season's chapters only */
		}
		storedChapters = stored;
		const numbers = new Set<number>(stored);
		for (const l of nextSeason.present) for (const item of nextSeason.data[l]?.sequence ?? []) numbers.add(Number(item.number));
		const docs: Record<number, ContentDoc> = {};
		await Promise.all(
			[...numbers].map(async (number) => {
				const chapter = toonDoc(entities.chapter(toonId, number), languages);
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
			// no index entry anywhere yet: start one (saved as the first version of `toon:<toon>`)
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

	// A new language: chapters, scenes, seasons and the toon entry are language-neutral (agents/NEUTRAL.md) – their
	// texts in it are simply missing (warnings) until translated; nothing is created per language.

	// ── Chapters ───────────────────────────────────────────────────────────────────────────────────────
	/** authors of the toon's chapter files (for new season / toon files) */
	function chapterAuthors(): string[] {
		const names = Object.values(chapters).flatMap((d) => d.present.map((l) => d.data[l]?.author).filter((a): a is string => !!a));
		return [...new Set(names)];
	}

	/**
	 * Starts season 1 (one language-neutral document) as an unsaved doc. A chapter must always belong to a season: chapter
	 * files already in the store (e.g. created before the season) go into the new sequence as draft entries. Returns the
	 * numbers it added, or null when the season isn't loaded.
	 */
	function startSeason(): number[] | null {
		if (!season || !langs) return null;
		const authors = chapterAuthors();
		const adopted = [...orphans];
		const draft = (number: number) => ({ type: 'chapter' as const, number, status: sequenceStatus(langs!.languages) });
		season.create(() => ({ ...newSeasonDoc(langs!.languages, authors), sequence: adopted.map(draft) }));
		return adopted;
	}

	/** "Create season 1": the season saved as a draft, existing chapter files adopted */
	async function createSeason() {
		message = null;
		error = null;
		const adopted = startSeason();
		if (!adopted || !season) {
			error = m['creator.not_loaded']();
			return;
		}
		busy = true;
		try {
			const files = [season].filter((d) => d.dirty);
			for (const doc of files) await doc.save(`create ${doc.entity}`);
			message = [
				m['creator.season_created']({ files: fileLabels(files.map((d) => d.entity)) }),
				adopted.length ? m['creator.chapters_adopted']({ numbers: adopted.join(', ') }) : ''
			].join(' ');
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}

	/** A new chapter, always inside the season: chapter file + season entry (+ a new season) in one save. */
	async function createChapter() {
		message = null;
		error = null;
		if (!season || !langs) {
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
			const chapter = toonDoc(entities.chapter(toonId, number), langs);
			chapter.create(() => newChapterDoc(number, langs!.languages, author));
			season.edit((file) => {
				file.sequence ??= [];
				file.sequence.push({ type: 'chapter', number, status: sequenceStatus(langs!.languages) });
			});
			await chapter.save(`new chapter ${number}`);
			await season.save(`add chapter ${number}`);
			message = m['creator.new_chapter_created']({ number });
			await goto(`/${toonId}/chapter/${number}`);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}

	/** Drops what a language-neutral chapter no longer stores and saves those chapters as drafts; returns them. */
	async function saveFollowers(): Promise<ContentDoc[]> {
		const changed: ContentDoc[] = [];
		for (const item of sequence) {
			const doc = chapters[item.number];
			if (!doc || doc.loading || !season) continue;
			if (syncChapter(doc) && doc.dirty) {
				await doc.save('status from the season');
				changed.push(doc);
			}
		}
		return changed;
	}

	// ── Save / publish: the toon's index entry (languages), the season ──────────────────────────────────
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
		if (reviews.needsReview(toonId)) {
			// needs a review in this toon: the same files go to the reviewers (lib/editor/reviews.svelte.ts)
			const note = await confirmSubmit(docs);
			if (note === null || !(await saveAll())) return;
			busy = true;
			try {
				message = await submitForReview([...pageDocs, ...followersSaved].filter((d) => d.stored.length && d.unpublished.length), note);
				followersSaved = [];
			} catch (e) {
				error = e instanceof Error ? e.message : String(e);
			} finally {
				busy = false;
			}
			return;
		}
		// the one publish dialog (lib/editor/doc-session.svelte.ts): the files and their languages
		if (!(await confirmPublish(docs))) return;
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
	<main class="browser">
		<section>
			<h1 class="toon-title">
				<a href="/{toonId}/toon" title={m['creator.open_toon_settings']({ name: toonName })} data-testid="toon-title"><ToonAvatar src={avatarSrc} name={toonName} /> <span>{toonName}</span></a>
			</h1>
			{#if index && !index.loading && edited}
				<section class="languages" data-testid="languages">
					<div class="season-head">
						<h2>{m['creator.languages']()}</h2>
						<FileStatus doc={index} />
					</div>
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
									<li class="sc-chip-control" data-testid="language-{l}">
										{languageLabel(l)}
										<button class="ghost" title={m['creator.make_source']({ locale: l })} onclick={() => setSource(l)}>★</button>
										<button class="ghost" title={m['creator.remove_language']({ locale: l })} onclick={() => removeTarget(l)} data-testid="remove-language-{l}" aria-label={m['creator.remove_language']({ locale: l })}><CloseIcon /></button>
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
									<button class="secondary" disabled={!addLanguage} onclick={addTarget} data-testid="add-language"><Icon name="plus" /> {m['creator.add_language']()}</button>
								</span>
							{/if}
						</div>
					</div>
					{#if languagesChanged}<p class="hint warn">{m['creator.languages_unsaved']()}</p>{/if}
				</section>
			{/if}
			{#if unconverted}
				<!-- only while something is still stored per language (agents/NEUTRAL.md): the owner's Convert page -->
				<p class="sc-bar is-note" data-testid="convert-bar">{m['creator.convert_bar']()} <a href="/{toonId}/convert" data-testid="open-convert">{m['creator.convert_open']()}</a></p>
			{/if}
			{#if !season || season.loading}
				<p>{m['creator.loading']()}</p>
			{:else if season.error}
				<p class="error">{season.error}</p>
			{:else if seasonMissing}
				<section class="season-missing" data-testid="season-missing">
					<h2>{m['creator.season']({ number: 1 })}</h2>
					<p>{m['creator.season_missing']()}</p>
					<button disabled={busy || languagesChanged} onclick={createSeason} data-testid="create-season"><Icon name="plus" /> {m['creator.create_season']()}</button>
				</section>
			{:else}
				<div class="season-head">
					<div>
						<h2 class="season-title">
							<a
								href="/{toonId}/season/1"
								title="{m['creator.open_season_editor']({ season: m['creator.season']({ number: 1 }) })}{season.textLanguages.length > 1 ? `\n${otherTexts(season.textLanguages, titleLocale, (l) => season?.text(['title'], l))}` : ''}"
								data-testid="season-title"
								>{m['creator.season']({ number: 1 })}: {titleOf(season)}</a
							>
						</h2>
					</div>
					<FileStatus doc={season} />
				</div>

				<h3>{m['creator.chapters']()}</h3>
				<ol class="chapter-list" data-testid="chapter-list">
					{#each sequence as item (item.number)}
						{@const doc = chapters[item.number]}
						{@const own = doc?.hasText(['title'], titleLocale) ? doc.text(['title'], titleLocale) : ''}
						<li class="chapter-row" data-testid="chapter-row-{item.number}">
							<span class="num" title={m['creator.chapter_number_hint']({ path: `/${toonId}/chapter/${item.number}` })}>#{item.number}</span>
							<a
								class="row-link"
								class:fallback={!own}
								href="/{toonId}/chapter/{item.number}"
								title="{m['creator.open_chapter_n']({ number: item.number })}{season.textLanguages.length > 1 ? `\n${otherTexts(season.textLanguages, titleLocale, (l) => doc?.text(['title'], l))}` : ''}"
								data-testid="open-chapter-{item.number}"
								>{own || titleOf(doc) || fileLabel(entities.chapter(toonId, item.number))}</a
							>
							<ChapterBadges {season} number={item.number} {doc} />
						</li>
					{/each}
				</ol>
			{/if}
			{#if season && !season.loading && !seasonMissing && orphans.length}
				<p class="error" data-testid="orphans">{m['creator.orphans']({ numbers: orphans.join(', '), file: fileLabel(season.entity) })}</p>
			{/if}
			{#if message}<p class="hint">{message}</p>{/if}
			{#if error}<p class="error">{error}</p>{/if}
		</section>
	</main>
	<!-- Bottom action bar: back to the toons · new chapter · save / publish (languages, season) -->
	<ActionBar label={m['creator.page_actions']()} moreLabel={m['creator.more']()}>
		{#snippet start()}
			{#if anyDirty}<span class="hint">{m['creator.unsaved']()}</span>{/if}
			{#if error}<span class="error" data-testid="page-error">{error}</span>{:else if message}<span class="hint">{message}</span>{/if}
		{/snippet}
		{#snippet secondary()}
			<button onclick={createChapter} disabled={busy || !season || season.loading || languagesChanged} data-testid="new-chapter"><Icon name="plus" /> {m['creator.new_chapter']()}</button>
		{/snippet}
		{#snippet end()}
			<button class="secondary" disabled={busy || !anyDirty} onclick={saveAll} data-testid="save-draft">{m['creator.save_draft']()}{anyDirty ? ' •' : ''}</button>
			<button class="secondary" disabled={busy || !pageDocs.some((d) => d.dirty || d.unpublished.length)} onclick={publishAll} data-testid="publish">{publishLabel(toonId)}</button>
		{/snippet}
	</ActionBar>
</div>

<style>
	.toon-title {
		font-size: var(--type-title-size);
		line-height: var(--type-title-line);
	}
	.toon-title a,
	.season-title a {
		color: var(--ink);
		text-decoration: none;
	}
	.toon-title a {
		display: inline-flex;
		align-items: center;
		gap: 0.6rem;
	}
	.toon-title a:hover span,
	.season-title a:hover {
		text-decoration: underline;
	}
	.browser {
		padding: 1.5rem 1rem;
		max-width: 1300px;
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
	/* the chapters: a plain list (owner 2026-10-02: no spreadsheet) – number, title (the whole row is the link),
	   badges only when something is up; rows split by 1px ink */
	.chapter-list {
		list-style: none;
		margin: 0;
		padding: 0;
		border-top: var(--border-width-regular) solid var(--ink);
	}
	.chapter-row {
		position: relative;
		display: flex;
		align-items: center;
		gap: var(--space-1);
		min-height: var(--control-height);
		padding: var(--space-05) 0;
		border-bottom: var(--border-width-hairline) solid var(--ink);
	}
	/* the chapter's number = its URL and release order (owner 2026-10-03): in ink, not muted */
	.chapter-row .num {
		flex: none;
		min-width: 2.5em;
		color: var(--ink);
		font-variant-numeric: tabular-nums;
	}
	.row-link {
		flex: 1;
		min-width: 0;
		color: var(--ink);
		text-decoration: none;
		overflow-wrap: anywhere;
	}
	/* the title link covers the row: a click anywhere in it opens the chapter */
	.row-link::after {
		content: '';
		position: absolute;
		inset: 0;
	}
	/* no title in the chosen language: the source's, muted */
	.row-link.fallback {
		color: var(--text-muted);
	}
	.chapter-row:hover .row-link {
		text-decoration: underline;
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
		font-size: var(--type-caption-size);
		font-weight: var(--font-weight-regular);
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
		margin: 0;
		padding: 0;
		list-style: none;
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
</style>
