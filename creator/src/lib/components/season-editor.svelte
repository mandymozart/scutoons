<script lang="ts">
	import { entities } from '@scutoons/content';
	import { onMount } from 'svelte';
	import type { ToonLanguages } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { toonDoc } from '$lib/editor/languages.ts';
	import { knownImages } from '$lib/editor/images.ts';
	import { seasonChapterNumbers, seasonStatusOf, syncChapters } from '$lib/editor/visibility.ts';
	import MetaPage from './meta-page.svelte';
	import LocaleText from './locale-text.svelte';
	import SharedField from './shared-field.svelte';
	import SupportersField from './supporters-field.svelte';
	import { getCreatorNav, usePageNav } from '$lib/nav.svelte.ts';
	import StatusBadges, { statusWord } from './status-badges.svelte';
	import { fileLabel } from '$lib/labels.ts';

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
					<span class="nav-title">{chapters[n]?.data?.[chapters[n].primary]?.title ?? fileLabel(entities.chapter(toon, n))}</span>
					<span class="nav-status">
						{#each season.locales as l (l)}
							{@const status = seasonStatusOf(season, l, n)}
							{#if status}<span class="st st-{status}" title="{l}: {statusWord(status)}">{l}</span>{/if}
						{/each}
					</span>
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
				{#each season.present as l}
					{@const value = season.get(l, ['language'])}
					<span class:warn={value !== undefined && value !== l}>{l}: {value ?? '–'}</span>
					{#if value !== l}<button type="button" class="ghost mini" onclick={() => season.setLocal(l, ['language'], l)}>= {l}</button>{/if}
				{/each}
			</p>
			<div class="grid g3">
				<SharedField doc={season} path={['number']} label="number" kind="number" />
				<SharedField doc={season} path={['release']} label="release" kind="date" />
				<SharedField doc={season} path={['authors']} label="authors" kind="list" testid="season-authors" />
			</div>
			<div class="grid g2">
				<SharedField doc={season} path={['cover', 'square']} label="cover.square" kind="image" {toon} known={images} />
				<SharedField doc={season} path={['cover', 'portrait']} label="cover.portrait" kind="image" {toon} known={images} />
			</div>
		</section>

		<!-- the season's downloads (the music album): the reader links them below the season's chapter list -->
		<section class="block" data-testid="season-download">
			<h2>{m['creator.downloads']()}</h2>
			<div class="grid g2">
				<SharedField doc={season} path={['download', 'music']} label="download.music" placeholder="audio/….zip" />
				<SharedField doc={season} path={['download', 'complete']} label="download.complete" kind="nullable" placeholder="null (none)" />
			</div>
		</section>

		<!-- who supported the season: "Supported by" logos at the end of the reader's credits fine print -->
		<section class="block" data-testid="season-supporters">
			<h2>{m['creator.supporters']()}</h2>
			<SupportersField doc={season} {toon} known={images} />
		</section>

		<section class="block" data-testid="season-sequence">
			<h2>{m['creator.chapters']()}</h2>
			<table class="chapters">
				<thead>
					<tr>
						<th>#</th>
						{#each season.locales as l}<th>{l}{l === season.primary ? ` (${m['creator.source']()})` : ''}</th>{/each}
						<th>{m['creator.status']()}</th>
					</tr>
				</thead>
				<tbody>
					{#each numbers as n (n)}
						{@const doc = chapters[n]}
						<!-- the whole row opens the chapter; the status is changed on the chapter page (meta) -->
						<tr class="chapter-row">
							<td>{n}</td>
							{#each season.locales as l}
								<td>
									{#if l === season.primary}
										<a class="row-link" href="/{toon}/chapter/{n}" title={m['creator.open_chapter_n']({ number: n })} data-testid="open-chapter-{n}"
											>{doc?.data?.[l]?.title || fileLabel(entities.chapter(toon, n))}</a
										>
									{:else}{doc?.data?.[l]?.title ?? '–'}{/if}
								</td>
							{/each}
							<td><StatusBadges {season} number={n} /></td>
						</tr>
					{/each}
				</tbody>
			</table>
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
		font-size: 14px;
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
	.mini {
		font-size: 10px;
		padding: 0 0.3em;
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
	.chapter-row {
		position: relative;
		cursor: pointer;
	}
	.chapter-row:hover td {
		background: var(--surface-hover);
	}
	.row-link {
		color: var(--ink);
		font-weight: 700;
		text-decoration: none;
	}
	.row-link::after {
		content: '';
		position: absolute;
		inset: 0;
	}
	.chapter-row:hover .row-link {
		text-decoration: underline;
	}
	.nav-head {
		margin: 0;
		padding: 0.75rem 0.75rem 0.25rem;
		font-size: 10px;
		text-transform: uppercase;
		color: var(--muted);
	}
	.nav-list {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.nav-list a {
		display: flex;
		justify-content: space-between;
		gap: 0.4rem;
		padding: 0.35rem 0.75rem;
		border-left: 3px solid transparent;
		color: var(--ink);
		text-decoration: none;
	}
	.nav-list a:hover {
		background: var(--surface-hover);
	}
	.nav-title {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.nav-status {
		display: inline-flex;
		gap: 0.2rem;
		flex: none;
	}
	.st {
		font-size: 10px;
		padding: 0 0.3em;
		border: 1px solid var(--border-muted);
	}
	.st-published {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
	.st-coming-soon {
		border-style: dashed;
		border-color: var(--ink);
	}
</style>
