<script lang="ts">
	import { coverLetteringOn, coverLines, entities, type Cover } from '@scutoons/content';
	import { CoverArt } from '@scutoons/reader';
	import { asset } from '$lib/utils/cdn.ts';
	import { onMount } from 'svelte';
	import type { ToonLanguages } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { toonDoc } from '$lib/editor/languages.ts';
	import { knownImages } from '$lib/editor/images.ts';
	import { seasonChapterNumbers, syncChapters } from '$lib/editor/visibility.ts';
	import MetaPage from './meta-page.svelte';
	import LocaleText from './locale-text.svelte';
	import SharedField from './shared-field.svelte';
	import SupportersField from './supporters-field.svelte';
	import RelatedItemsField from './related-items-field.svelte';
	import { MEDIA_QUALITIES } from '@scutoons/content';
	import { qualityVariants, rootPath, toonRelativePath } from '@scutoons/api';
	import AssetPickerSheet, { type PickedAsset } from './assets/asset-picker-sheet.svelte';
	import { getCreatorNav, usePageNav } from '$lib/nav.svelte.ts';
	import ChapterBadges from './chapter-badges.svelte';
	import { fileLabel } from '$lib/labels.ts';
	import { listLanguage, otherTexts } from '$lib/editor/text-language.svelte.ts';

	/**
	 * Season meta (the entity season:<toon>:<N>): title / plot per locale, language-neutral fields once (overrides per locale shown),
	 * covers – and the chapter sequence with the status per locale (the chapters' visibility), all one doc.
	 */
	let { toon, number, languages }: { toon: string; number: number; languages: ToonLanguages } = $props();

	// the route re-creates this component per toon / season / languages ({#key})
	// svelte-ignore state_referenced_locally
	const season = toonDoc(entities.season(toon, number), languages);
	let chapters = $state<Record<number, ContentDoc>>({});
	let images = $state<string[]>([]);

	const numbers = $derived(seasonChapterNumbers(season));
	const cover = $derived(season.get(season.primary, ['cover']) as Cover | undefined);
	/** titles in lists: one language – the i18n switch's, else the source; the others in the tooltip (owner 2026-10-02) */
	const titleLocale = $derived(listLanguage(season.locales, season.primary));

	onMount(async () => {
		await season.load();
		const docs: Record<number, ContentDoc> = {};
		await Promise.all(
			seasonChapterNumbers(season).map(async (n) => {
				const doc = toonDoc(entities.chapter(toon, n), languages);
				await doc.load();
				docs[n] = doc;
			})
		);
		chapters = docs;
		images = await knownImages(toon, season.locales, [$state.snapshot(season.data)]);
	});

	/** the trailer loop from a video asset: its 480p / 720p / 1080p files → variants, 720p the loop (agents/ASSETS.md) */
	let choosingTrailer = $state(false);
	function pickTrailer(picked: PickedAsset) {
		const levels = qualityVariants(picked.asset.renditions.filter((r) => r.mime?.startsWith('video/')));
		const variants = Object.fromEntries(MEDIA_QUALITIES.filter((l) => levels[l]).map((l) => [l, toonRelativePath(levels[l]!, toon)]));
		const loop = (variants.medium || variants.high || variants.low || toonRelativePath(picked.path, toon)) as string;
		season.setShared(['trailer', 'loop'], loop, 'trailer-asset');
		season.setShared(['trailer', 'variants'], Object.keys(variants).length ? variants : undefined, 'trailer-asset');
	}

	// the season's children in the sidebar: its chapters with their status per language
	usePageNav({ sidebar: chapterNav });
	const nav = getCreatorNav();
</script>

{#snippet chapterNav()}
	<p class="nav-head">{m['creator.chapters']()}</p>
	<ul class="nav-list" data-testid="season-chapters">
		{#each numbers as n (n)}
			<li>
				<a href="/{toon}/chapter/{n}" data-nav-child data-testid="nav-chapter-{n}">
					<span class="nav-title">{chapters[n]?.data?.[titleLocale]?.title || chapters[n]?.data?.[chapters[n].primary]?.title || fileLabel(entities.chapter(toon, n))}</span>
					<span class="nav-status"><ChapterBadges {season} number={n} doc={chapters[n]} /></span>
				</a>
			</li>
		{/each}
	</ul>
{/snippet}

<MetaPage
	{toon}
	help="season"
	title={m['creator.season']({ number })}
	docs={[season]}
	companions={() => Object.values(chapters)}
	beforeSave={() => syncChapters(chapters, season)}
>
	{#if season.present.length === 0}
		<p>{m['creator.season_missing']()} <a href="/{toon}">{nav.toonName(toon)}</a></p>
	{:else}
		<section class="block" data-testid="season-meta">
			<h2>{m['creator.meta']()}</h2>
			<LocaleText doc={season} path={['title']} label="title" multiline={false} />
			<LocaleText doc={season} path={['description']} label="description (plot)" rows={6} />
			<p class="hint">
				language:
				{#each season.present as l, i}
					{@const value = season.get(l, ['language'])}
					{#if i > 0}·{/if}
					<span class:warn={value !== undefined && value !== l}>{l}: {value ?? '–'}</span>
					{#if value !== l}<button type="button" class="ghost mini" onclick={() => season.setLocal(l, ['language'], l)}>= {l}</button>{/if}
				{/each}
			</p>
			<div class="grid g3">
				<SharedField doc={season} path={['number']} label="number" kind="number" />
				<SharedField doc={season} path={['release']} label="release" kind="date" />
				<SharedField doc={season} path={['authors']} label="authors" kind="list" testid="season-authors" />
			</div>
		</section>

		<!-- the season's cover (owner 2026-10-02: covers are per season): art + lettering in the cover editor -->
		<section class="block" data-testid="season-cover">
			<h2>{m['creator.season_cover']()}</h2>
			<p class="hint">{m['creator.cover_season_hint']()}</p>
			<div class="cover-row">
				{#if cover?.square}
					<div class="cover-thumb">
						<CoverArt src={asset(`${toon}/${cover.square}`)} lettering={cover.lettering} title={nav.toonName(toon)} subtitle={String(season.get(season.primary, ['title']) ?? '')} lang={season.primary} />
					</div>
				{/if}
				<p class="cover-state" data-testid="season-cover-lettering">
					<span class="sc-label">{m['creator.cover_mode']()}</span>
					<span>{!coverLetteringOn(cover?.lettering) ? m['creator.cover_mode_off']() : coverLines(cover?.lettering, 'square') ? m['creator.cover_mode_custom']() : m['creator.cover_mode_default']()}</span>
					<a class="btn secondary" href="/{toon}/season/{number}/cover" data-testid="season-cover-editor">{m['creator.cover_editor_open']()}</a>
				</p>
			</div>
		</section>

		<!-- the season's related items (catalog ids, the same in every language): the toon page's "related" block -->
		<section class="block" data-testid="season-related">
			<h2>{m['creator.related_items']()}</h2>
			<RelatedItemsField doc={season} hint={m['creator.related_items_hint_season']()} legend={false} />
			<!-- the season's music = the album of a related record (owner 2026-10-02): no own download fields any more;
			     older seasons' download.music / download.image are still read by the reader (agents/ASSETS.md) -->
			<p class="hint" data-testid="season-music-hint">{m['creator.season_music_from_album']()}</p>
		</section>

		<!-- the toon page's desktop hero: a muted background loop + "Trailer ansehen" / "Ton an" → the trailer chapter -->
		<section class="block" data-testid="season-trailer">
			<h2 title={m['creator.trailer_hint']()}>{m['creator.trailer']()}</h2>
			<button type="button" class="secondary choose" onclick={() => (choosingTrailer = true)} title={m['creator.assets_choose_video_hint']()} data-testid="trailer-choose">{m['creator.assets_choose_video']()}</button>
			<AssetPickerSheet
				bind:open={choosingTrailer}
				{toon}
				kind="video"
				value={season.get(season.primary, ['trailer', 'loop']) ? rootPath(season.get(season.primary, ['trailer', 'loop']), toon) : ''}
				title={m['creator.trailer']()}
				onpick={pickTrailer}
			/>
			<div class="grid g2">
				<SharedField doc={season} path={['trailer', 'loop']} label="trailer.loop" placeholder="s1/s1-trailer-loop.720p.mp4" testid="trailer-loop" />
				<SharedField doc={season} path={['trailer', 'poster']} label="trailer.poster" kind="image" {toon} known={images} testid="trailer-poster" />
				<!-- the bigger encoding as the "high" (1080p) quality variant – the reader's quality setting picks it, no media query -->
				<SharedField doc={season} path={['trailer', 'variants', 'low']} label="trailer · 480p" placeholder="s1/s1-trailer-loop.480p.mp4" testid="trailer-low" />
				<SharedField doc={season} path={['trailer', 'variants', 'medium']} label="trailer · 720p" placeholder="s1/s1-trailer-loop.720p.mp4" testid="trailer-medium" />
				<SharedField doc={season} path={['trailer', 'variants', 'high']} label="trailer · 1080p" placeholder="s1/s1-trailer-loop.1080p.mp4" testid="trailer-high" />
				<SharedField doc={season} path={['trailer', 'chapter']} label="trailer.chapter" kind="number" placeholder="0" />
			</div>
		</section>

		<!-- who supported the season: "Supported by" logos at the end of the reader's credits fine print -->
		<section class="block" data-testid="season-supporters">
			<h2>{m['creator.supporters']()}</h2>
			<SupportersField doc={season} {toon} known={images} />
		</section>

		<section class="block" data-testid="season-sequence">
			<h2>{m['creator.chapters']()}</h2>
			<ol class="chapter-list" data-testid="chapter-list">
				{#each numbers as n (n)}
					{@const doc = chapters[n]}
					{@const own = doc?.data?.[titleLocale]?.title}
					<li class="chapter-row" data-testid="chapter-row-{n}">
						<span class="num">{n}</span>
						<a
							class="row-link"
							class:fallback={!own}
							href="/{toon}/chapter/{n}"
							title="{m['creator.open_chapter_n']({ number: n })}{season.locales.length > 1 ? `\n${otherTexts(season.locales, titleLocale, (l) => doc?.data?.[l]?.title)}` : ''}"
							data-testid="open-chapter-{n}"
							>{own || doc?.data?.[season.primary]?.title || fileLabel(entities.chapter(toon, n))}</a
						>
						<ChapterBadges {season} number={n} {doc} />
					</li>
				{/each}
			</ol>
		</section>
	{/if}
</MetaPage>

<style>
	.block {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		margin-bottom: 2rem;
	}
	.block:last-child {
		margin-bottom: 0;
	}
	h2 {
		margin: 0;
	}
	.grid {
		display: grid;
		gap: 0.75rem;
		align-items: start;
	}
	.g2 {
		grid-template-columns: repeat(2, minmax(0, 1fr));
	}
	.g3 {
		grid-template-columns: repeat(3, minmax(0, 1fr));
	}
	@media (max-width: 900px) {
		.g3 {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}
	.warn {
		color: var(--warn);
	}
	.choose {
		align-self: flex-start;
	}
	.mini {
		font-size: var(--type-caption-size);
		padding: 0 0.3em;
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
	.chapter-row .num {
		flex: none;
		min-width: 2em;
		color: var(--text-muted);
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
	.nav-head {
		margin: 0;
		padding: 0.75rem 0.75rem 0.25rem;
		font-size: var(--type-caption-size);
		letter-spacing: var(--tracking-label);
		text-transform: uppercase;
		color: var(--muted);
	}
	.nav-list {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	/* narrow sidebar: the badges drop to their own line instead of squeezing the title to "Är…" (owner 2026-10-03) */
	.nav-list a {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: 0.25rem 0.4rem;
		padding: 0.35rem 0.75rem;
		border-left: 3px solid transparent;
		color: var(--ink);
		text-decoration: none;
	}
	.nav-list a:hover {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
	.nav-title {
		flex: 1 1 auto;
		min-width: min(8rem, 100%);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.nav-status {
		display: inline-flex;
		gap: 0.2rem;
		flex: none;
		margin-left: auto;
	}
	.cover-row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-1) var(--space-15);
	}
	.cover-thumb {
		width: 12rem;
	}
	.cover-state {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05) var(--space-1);
		margin: 0;
	}
</style>
