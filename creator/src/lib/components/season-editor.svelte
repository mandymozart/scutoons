<script lang="ts">
	import { coverLetteringOn, coverLines, entities, type Cover, type SeasonTrailer } from '@scutoons/content';
	import { CoverArt } from '@scutoons/reader';
	import { Icon } from '@scutoons/ui';
	import { asset } from '$lib/utils/cdn.ts';
	import { onMount } from 'svelte';
	import type { ToonLanguages } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { toonDoc } from '$lib/editor/languages.ts';
	import { seasonChapterNumbers, syncChapters } from '$lib/editor/visibility.ts';
	import type { VariantMedia } from '$lib/editor/media-variants.ts';
	import MetaPage from './meta-page.svelte';
	import LocaleText from './locale-text.svelte';
	import PropField from './prop-field.svelte';
	import SharedField from './shared-field.svelte';
	import MediaSlot from './media-slot.svelte';
	import MediaVariantsSlot from './media-variants-slot.svelte';
	import SupportersField from './supporters-field.svelte';
	import RelatedItemsField from './related-items-field.svelte';
	import { getCreatorNav, usePageNav } from '$lib/nav.svelte.ts';
	import ChapterBadges from './chapter-badges.svelte';
	import { fileLabel } from '$lib/labels.ts';
	import { listLanguage } from '$lib/editor/text-language.svelte.ts';

	/**
	 * Season meta (the entity season:<toon>:<N>), cleaned up like the chapter page (owner 2026-10-03): title and
	 * description (per language), release + authors, the media as slots (cover →
	 * the cover editor, the trailer with its quality levels, the poster), related items, supporters. Everything but title
	 * and description is the same in every language (written to every locale document; said once, in the hint).
	 * The season's number is its identity (the URL, the entity key) – not a field. The chapters and their visibility are in
	 * the sidebar only (owner 2026-10-03: no second list on the page).
	 */
	let { toon, number, languages }: { toon: string; number: number; languages: ToonLanguages } = $props();

	// the route re-creates this component per toon / season / languages ({#key})
	// svelte-ignore state_referenced_locally
	const season = toonDoc(entities.season(toon, number), languages);
	let chapters = $state<Record<number, ContentDoc>>({});

	const numbers = $derived(seasonChapterNumbers(season));
	const cover = $derived(season.get(season.primary, ['cover']) as Cover | undefined);
	const trailer = $derived(season.get(season.primary, ['trailer']) as Partial<SeasonTrailer> | undefined);
	/** the season's folder by the content rules – uploads from its fields land there (catpope/s1/) */
	// svelte-ignore state_referenced_locally
	const place = `${toon}/s${number}/`;
	/** titles in lists: one language – the i18n switch's, else the source; the others in the tooltip (owner 2026-10-02) */
	const titleLocale = $derived(listLanguage(season.locales, season.primary));
	/** documents whose `language` field is not their own language (a copy-paste slip): one warning, one fix */
	const wrongLanguage = $derived(season.present.filter((l) => season.get(l, ['language']) !== l));
	const chapterTitle = (n: number) => chapters[n]?.data?.[titleLocale]?.title || chapters[n]?.data?.[chapters[n].primary]?.title || fileLabel(entities.chapter(toon, n));

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
	});

	/**
	 * The trailer, the same in every locale document. `loop` is the default file of the quality variants
	 * (agents/ASSETS.md); without a loop and a poster the whole trailer goes (the hero then shows the cover).
	 */
	function writeTrailer(change: Partial<SeasonTrailer>, key: string) {
		season.structural((file) => {
			const next: Record<string, unknown> = { ...(file.trailer ?? {}), ...change };
			for (const k of Object.keys(next)) if (next[k] === undefined) delete next[k];
			if (!next.loop && !next.poster) delete file.trailer;
			else {
				const { loop, variants, poster, chapter, ...rest } = next;
				file.trailer = Object.fromEntries(Object.entries({ loop, variants, poster, chapter, ...rest }).filter(([, v]) => v !== undefined));
			}
		}, key);
	}
	const trailerMedia = $derived<VariantMedia>({ src: trailer?.loop, variants: trailer?.variants });
	function setTrailerMedia(next: VariantMedia) {
		// the old media-query encodings (`sources`) are replaced by the levels as soon as the trailer is chosen again
		writeTrailer({ loop: next.src, variants: next.variants, sources: undefined }, 'trailer-media');
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
					<span class="nav-title">{chapterTitle(n)}</span>
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
			<LocaleText doc={season} path={['description']} label="description" rows={6} />
			{#if wrongLanguage.length}
				<p class="warn" data-testid="season-language">
					{m['creator.season_language_mismatch']({ locales: wrongLanguage.join(', ') })}
					<button type="button" class="ghost" onclick={() => wrongLanguage.forEach((l) => season.setLocal(l, ['language'], l))}>{m['creator.season_language_fix']()}</button>
				</p>
			{/if}
			<div class="sc-fields" data-cols="2">
				<PropField doc={season} path={['release']} label="release" type="date" />
				<SharedField doc={season} path={['authors']} label="authors" kind="list" splittable={false} testid="season-authors" />
			</div>
			{#if season.present.length > 1}<p class="hint">{m['creator.season_neutral_hint']()}</p>{/if}
		</section>

		<!-- media as slots (owner 2026-10-03, like the chapter page): the preview is the button, paths only as tooltips -->
		<section class="block" data-testid="season-media">
			<h2 title={m['creator.trailer_hint']()}>{m['creator.season_media']()}</h2>
			<div class="media">
				<!-- the cover (owner 2026-10-02: covers are per season): art + lettering live in the cover editor -->
				<div class="cover-slot" data-testid="season-cover">
					<span class="sc-label">{m['creator.season_cover']()}</span>
					<a
						class="cover-link"
						class:empty={!cover?.square}
						href="/{toon}/season/{number}/cover"
						title="{cover?.square ? `${cover.square}\n` : ''}{m['creator.cover_open_hint']()}"
						aria-label="{m['creator.season_cover']()}: {m['creator.cover_editor_open']()}"
						data-testid="season-cover-editor"
					>
						{#if cover?.square}
							<CoverArt src={asset(`${toon}/${cover.square}`)} lettering={cover.lettering} title={nav.toonName(toon)} subtitle={String(season.get(season.primary, ['title']) ?? '')} lang={season.primary} />
						{:else}
							<Icon name="plus" />
						{/if}
					</a>
					<span class="sc-caption" data-testid="season-cover-lettering">
						{m['creator.cover_mode']()}: {!coverLetteringOn(cover?.lettering) ? m['creator.cover_mode_off']() : coverLines(cover?.lettering, 'square') ? m['creator.cover_mode_custom']() : m['creator.cover_mode_default']()}
					</span>
				</div>

				<!-- the toon page's desktop hero: a muted background loop + "Trailer ansehen" / "Ton an" → the trailer chapter -->
				<MediaVariantsSlot {toon} {place} label={m['creator.trailer_loop']()} value={trailerMedia} poster={trailer?.poster} onchange={setTrailerMedia} testid="trailer-video" />
				<MediaSlot {toon} {place} label={m['creator.trailer_poster']()} value={trailer?.poster} onchange={(v) => writeTrailer({ poster: v }, 'trailer-poster')} testid="trailer-poster" />
			</div>
			{#if trailer?.loop || trailer?.poster}
				<label class="trailer-chapter" title={m['creator.trailer_chapter_hint']()}>
					<span class="sc-label">{m['creator.trailer_chapter']()}</span>
					<select value={String(trailer?.chapter ?? 0)} onchange={(e) => writeTrailer({ chapter: Number(e.currentTarget.value) }, 'trailer-chapter')} data-testid="trailer-chapter">
						{#each numbers.includes(trailer?.chapter ?? 0) ? numbers : [trailer?.chapter ?? 0, ...numbers] as n (n)}
							<option value={String(n)}>{n} · {chapterTitle(n)}</option>
						{/each}
					</select>
				</label>
			{/if}
		</section>

		<!-- the season's related items (catalog ids, the same in every language): the toon page's "related" block -->
		<section class="block" data-testid="season-related">
			<h2>{m['creator.related_items']()}</h2>
			<RelatedItemsField doc={season} hint={m['creator.related_items_hint_season']()} legend={false} />
			<!-- the season's music = the album of a related record (owner 2026-10-02): no own download fields any more;
			     older seasons' download.music / download.image are still read by the reader (agents/ASSETS.md) -->
			<p class="hint" data-testid="season-music-hint">{m['creator.season_music_from_album']()}</p>
		</section>

		<!-- who supported the season: "Supported by" logos at the end of the reader's credits fine print -->
		<section class="block" data-testid="season-supporters">
			<h2>{m['creator.supporters']()}</h2>
			<SupportersField doc={season} {toon} place="{place}supporters/" />
		</section>
	{/if}
</MetaPage>

<style>
	.block {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		max-width: 56rem;
		margin-bottom: 2rem;
	}
	.block:last-child {
		margin-bottom: 0;
	}
	h2 {
		margin: 0;
	}
	.hint {
		margin: 0;
	}
	/* the field grid (tools.css .sc-fields): top-aligned, one column below 600px */
	.sc-fields {
		align-items: start;
	}
	@media (max-width: 600px) {
		.sc-fields[data-cols] {
			grid-template-columns: minmax(0, 1fr);
		}
	}
	.warn {
		margin: 0;
		color: var(--warn);
	}
	.media {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-start;
		gap: 0.75rem 1.5rem;
	}
	.cover-slot {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}
	/* the cover's preview is the link into the cover editor (like a media slot: the preview is the control) */
	.cover-link {
		display: flex;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		width: 10rem;
		aspect-ratio: 1;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
		overflow: hidden;
	}
	.cover-link.empty {
		border-style: dashed;
		color: var(--muted);
	}
	.cover-link:hover,
	.cover-link:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: 2px;
	}
	.cover-link :global(.cover-art) {
		width: 100%;
	}
	.trailer-chapter {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		max-width: 24rem;
	}
	.nav-head {
		margin: 0;
		padding: 1rem 1rem 0.25rem; /* the sidebar's 1rem line (owner 2026-10-03) */
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
		/* text on the sidebar's 1rem line (the 3px marker inside it) */
		padding: 0.35rem 1rem 0.35rem calc(1rem - 3px);
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
</style>
