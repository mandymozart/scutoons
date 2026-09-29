<script lang="ts">
	import { onMount } from 'svelte';
	import type { ToonLanguages } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { toonDoc } from '$lib/editor/languages.ts';
	import { knownImages } from '$lib/editor/images.ts';
	import { seasonChapterNumbers, seasonStatusOf, setSeasonStatus, STATUSES, syncChapters } from '$lib/editor/visibility.ts';
	import MetaPage from './meta-page.svelte';
	import LocaleText from './locale-text.svelte';
	import SharedField from './shared-field.svelte';

	/**
	 * Season meta (season<N>.json): title / plot per locale, language-neutral fields once (overrides per locale shown),
	 * covers, credits – and the chapter sequence with the status per locale (the chapters' visibility), all one doc.
	 */
	let { toon, number, languages }: { toon: string; number: number; languages: ToonLanguages } = $props();

	// the route re-creates this component per toon / season / languages ({#key})
	// svelte-ignore state_referenced_locally
	const season = toonDoc(toon, `season${number}.json`, languages);
	let chapters = $state<Record<number, ContentDoc>>({});
	let images = $state<string[]>([]);

	const numbers = $derived(seasonChapterNumbers(season));

	onMount(async () => {
		await season.load();
		const docs: Record<number, ContentDoc> = {};
		await Promise.all(
			seasonChapterNumbers(season).map(async (n) => {
				const doc = toonDoc(toon, `chapter${n}.json`, languages);
				await doc.load();
				docs[n] = doc;
			})
		);
		chapters = docs;
		images = await knownImages(toon, season.locales, [$state.snapshot(season.data)]);
	});
</script>

<MetaPage
	{toon}
	title={m['creator.season']({ number })}
	docs={[season]}
	companions={() => Object.values(chapters)}
	beforeSave={() => syncChapters(chapters, season)}
>
	{#if season.present.length === 0}
		<p>{m['creator.season_missing']()} <a href="/{toon}">← {toon}</a></p>
	{:else}
		<section class="block" data-testid="season-meta">
			<h2>{m['creator.meta']()} · {season.path}</h2>
			<LocaleText doc={season} path={['title']} label="title" multiline={false} />
			<LocaleText doc={season} path={['description']} label="description (plot)" rows={6} />
			<p class="hint">
				language:
				{#each season.present as l}
					{@const value = season.get(l, ['language'])}
					<span class:warn={value !== undefined && value !== l}>{l} file → {value ?? '–'}</span>
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

		<section class="block" data-testid="season-credits">
			<h2>{m['creator.credits']()}</h2>
			<div class="grid g4">
				<SharedField doc={season} path={['credits', 'artist']} label="credits.artist" kind="list" testid="credits-artist" />
				<SharedField doc={season} path={['credits', 'writer']} label="credits.writer" testid="credits-writer" />
				<SharedField doc={season} path={['credits', 'music']} label="credits.music" kind="list" testid="credits-music" />
				<SharedField doc={season} path={['credits', 'sound']} label="credits.sound" kind="list" testid="credits-sound" />
			</div>
		</section>

		<section class="block" data-testid="season-sequence">
			<h2>{m['creator.chapters']()}</h2>
			<table class="chapters">
				<thead>
					<tr>
						<th>#</th>
						{#each season.locales as l}<th>{l}{l === season.primary ? ` (${m['creator.source']()})` : ''}</th>{/each}
						{#each season.locales as l}<th>{m['creator.status']()} {l}</th>{/each}
						<th></th>
					</tr>
				</thead>
				<tbody>
					{#each numbers as n (n)}
						{@const doc = chapters[n]}
						<tr>
							<td>{n}</td>
							{#each season.locales as l}<td>{doc?.data?.[l]?.title ?? '–'}</td>{/each}
							{#each season.locales as l}
								{@const status = seasonStatusOf(season, l, n)}
								<td>
									{#if status}
										<select value={status} onchange={(e) => setSeasonStatus(season, l, n, e.currentTarget.value as (typeof STATUSES)[number])} data-testid="status-{n}-{l}">
											{#each STATUSES as s}<option value={s}>{s}</option>{/each}
										</select>
									{:else}<span class="hint">–</span>{/if}
								</td>
							{/each}
							<td><a class="btn" href="/{toon}/chapter/{n}">{m['creator.open']()}</a></td>
						</tr>
					{/each}
				</tbody>
			</table>
			<p class="hint">{m['creator.status_hint']({ file: season.path })} {m['creator.status_follow_chapters']()}</p>
		</section>

		<section class="block">
			<h2>{m['creator.in_the_reader']()}</h2>
			<p class="hint">{m['creator.reader_uses_season']()}</p>
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
	.g4 {
		grid-template-columns: repeat(4, minmax(0, 1fr));
	}
	@media (max-width: 900px) {
		.g3,
		.g4 {
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
	.chapters select {
		width: auto;
	}
</style>
