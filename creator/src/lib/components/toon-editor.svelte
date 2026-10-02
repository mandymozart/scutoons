<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { entities, mediaRequests, NEUTRAL_LOCALE } from '@scutoons/content';
	import { CoverArt } from '@scutoons/reader';
	import { asset } from '$lib/utils/cdn.ts';
	import { requestsDoc, requestsOf } from '$lib/editor/characters.ts';
	import { onMount } from 'svelte';
	import type { ToonLanguages } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { toonDoc, toonIndexDoc } from '$lib/editor/languages.ts';
	import { knownImages } from '$lib/editor/images.ts';
	import { STATUSES } from '$lib/editor/visibility.ts';
	import MetaPage from './meta-page.svelte';
	import { fileLabel } from '$lib/labels.ts';
	import LocaleText from './locale-text.svelte';
	import SharedField from './shared-field.svelte';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { textLanguage } from '$lib/editor/text-language.svelte.ts';

	/**
	 * The toon's settings. Per language (entity toon-settings:<toon>, `ToonSettings`): title / description, plus release
	 * and status (shared with per-language overrides). The same in every language (`TOON_NEUTRAL_FIELDS`: authors,
	 * chapter count, thumbnail, the cover, the landing tile, announce, the map) live once in the toon's index entry
	 * `toon:<toon>`, saved and published with the settings through the same bottom bar. The cover's preview shows it per
	 * language as readers see it and with the rendered title (`CoverArt`, `cover.lettering`), plus the toon's open media
	 * requests (wanted images: e.g. a lettering-free cover).
	 */
	let { toon, languages }: { toon: string; languages: ToonLanguages } = $props();

	// svelte-ignore state_referenced_locally
	const file = toonDoc(entities.toonSettings(toon), languages);
	// svelte-ignore state_referenced_locally
	const index = toonIndexDoc(toon);
	// the season's title for the cover preview (season 1 = the toon page's season; read only, never saved here)
	// svelte-ignore state_referenced_locally
	const season = toonDoc(entities.season(toon, 1), languages);
	const requests = requestsDoc();
	let images = $state<string[]>([]);
	const nav = getCreatorNav();

	const cover = $derived(index.get(NEUTRAL_LOCALE, ['cover']) as { square?: string; portrait?: string; lettering?: boolean } | undefined);
	const wanted = $derived(mediaRequests(requestsOf(requests), { toon }));

	onMount(async () => {
		await Promise.all([file.load(), index.load(), season.load().catch(() => {}), requests.load().catch(() => {})]);
		images = await knownImages(toon, file.locales, [$state.snapshot(file.data), $state.snapshot(index.data)]);
	});
</script>

<MetaPage {toon} title={fileLabel(entities.toonSettings(toon))} docs={[file, index]} help="toon-file">
	{#if file.present.length === 0}
		<p>{m['creator.toon_missing']()} <a href="/{toon}">{nav.toonName(toon)}</a></p>
	{:else}
		<section class="block" data-testid="toon-meta">
			<h2>{m['creator.meta']()}</h2>
			<LocaleText doc={file} path={['title']} label="title" multiline={false} />
			<LocaleText doc={file} path={['description']} label="description" rows={3} />
			<p class="hint">
				language:
				{#each file.present as l, i}
					{@const value = file.get(l, ['language'])}
					{#if i > 0}·{/if}
					<span class:warn={value !== l}>{l}: {value ?? '–'}</span>
					{#if value !== l}<button type="button" class="ghost mini" onclick={() => file.setLocal(l, ['language'], l)}>= {l}</button>{/if}
				{/each}
			</p>
			<div class="grid g3">
				<SharedField doc={file} path={['id']} label="id" required />
				<SharedField doc={file} path={['status']} label="status" kind="select" options={STATUSES} required />
				<SharedField doc={file} path={['release']} label="release" kind="date" required />
			</div>
		</section>

		<section class="block neutral" data-testid="toon-neutral">
			<h2>{m['creator.toon_neutral']()}</h2>
			<p class="hint">{m['creator.toon_neutral_hint']()}</p>
			{#if index.present.length === 0}
				<p class="warn">{m['creator.toon_entry_missing']()} <a href="/{toon}">{nav.toonName(toon)}</a></p>
			{:else}
				<SharedField doc={index} path={['authors']} label="authors" kind="list" testid="toon-authors" />
				<div class="grid g3">
					<SharedField doc={index} path={['chapters']} label="chapters (count)" kind="number" />
					<SharedField doc={index} path={['thumbnail']} label="thumbnail" kind="image" {toon} known={images} />
				</div>

				<div class="sub" data-testid="toon-cover">
					<h3>{m['creator.cover']()}</h3>
					<p class="hint">{m['creator.cover_hint']()}</p>
					<div class="grid g3">
						<SharedField doc={index} path={['cover', 'square']} label="cover.square" kind="image" {toon} known={images} />
						<SharedField doc={index} path={['cover', 'portrait']} label="cover.portrait" kind="image" {toon} known={images} />
					</div>
					<label class="check">
						<input
							type="checkbox"
							checked={cover?.lettering === true}
							onchange={(e) => index.setShared(['cover', 'lettering'], e.currentTarget.checked ? true : undefined, 'cover.lettering')}
							data-testid="toon-cover-lettering"
						/>
						<span>{m['creator.cover_lettering']()}</span>
					</label>
					<p class="hint">{m['creator.cover_lettering_hint']()}</p>
					{#if cover?.square || cover?.portrait}
						<!-- per language: what readers see, and the rendered title (the lettering model) to judge it -->
						<div class="cover-preview" data-testid="cover-preview">
							<h4>{m['creator.cover_preview']()}</h4>
							{#each textLanguage.visible(file.present, file.primary) as l (l)}
								{@const title = String(file.get(l, ['title']) ?? '')}
								{@const subtitle = String(season.get(l, ['title']) ?? '')}
								<div class="cover-grid" data-testid="cover-preview-{l}">
									{#each (['square', 'portrait'] as const).filter((f) => cover?.[f]) as format (format)}
										<div class="cover-cell {format}" data-testid="cover-shown-{l}-{format}">
											<CoverArt src={asset(`${toon}/${cover?.[format]}`)} {format} lettering={cover?.lettering === true} {title} {subtitle} lang={l} />
											<span class="caption">{m['creator.cover_preview_shown']({ locale: l })} · {format}</span>
										</div>
										{#if cover?.lettering !== true}
											<div class="cover-cell {format}" data-testid="cover-rendered-{l}-{format}">
												<CoverArt src={asset(`${toon}/${cover?.[format]}`)} {format} lettering {title} {subtitle} lang={l} veil />
												<span class="caption">{m['creator.cover_preview_rendered']({ locale: l })} · {format}</span>
											</div>
										{/if}
									{/each}
								</div>
							{/each}
							{#if cover?.lettering !== true}<p class="hint">{m['creator.cover_preview_veil']()}</p>{/if}
						</div>
					{/if}
					{#if wanted.length}
						<div class="wanted" data-testid="toon-media-requests">
							<h4>{m['creator.cover_requests']()}</h4>
							<ul>
								{#each wanted as r (r.id)}
									<li><code>{r.path}</code>{#if r.size} <span class="hint">{r.size}</span>{/if}{#if r.note} <span class="req"><Icon name="mail" /> {r.note}</span>{/if}</li>
								{/each}
							</ul>
						</div>
					{/if}
				</div>

				<div class="sub" data-testid="toon-landing">
					<h3>{m['creator.landing']()}</h3>
					<p class="hint">{m['creator.landing_hint']()}</p>
					<label class="check">
						<input
							type="checkbox"
							checked={index.get(NEUTRAL_LOCALE, ['announce']) === true}
							onchange={(e) => index.setShared(['announce'], e.currentTarget.checked ? true : undefined, 'announce')}
							data-testid="toon-announce"
						/>
						<span>{m['creator.announce']()}</span>
					</label>
					<p class="hint">{m['creator.announce_hint']()}</p>
					<div class="grid g3">
						<SharedField doc={index} path={['tile', 'image']} label="tile.image" kind="image" {toon} known={images} />
						<SharedField doc={index} path={['tile', 'color']} label="tile.color" placeholder={m['creator.tile_color_placeholder']()} testid="toon-tile-color" />
					</div>
				</div>

				<div class="sub" data-testid="toon-map">
					<h3>{m['creator.map_start_view']()}</h3>
					<p class="hint">{m['creator.map_start_view_hint']()} <a href="/map?toon={toon}">{m['creator.map_open_builder']()}</a></p>
					<div class="grid g4">
						<SharedField doc={index} path={['map', 'center', 'x']} label="map.center.x" kind="number" />
						<SharedField doc={index} path={['map', 'center', 'y']} label="map.center.y" kind="number" />
						<SharedField doc={index} path={['map', 'zoom']} label="map.zoom" kind="number" />
					</div>
				</div>
			{/if}
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
	.sub {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		margin-top: 0.75rem;
	}
	h2 {
		margin: 0;
	}
	h3 {
		margin: 0;
	}
	h4 {
		margin: 0 0 0.4rem;
	}
	.cover-grid {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
		align-items: flex-end;
		margin-bottom: 0.75rem;
	}
	.cover-cell {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	/* one height: a square 220 px wide, a 9:16 portrait 124 px (a language's four previews fit one row) */
	.cover-cell.square {
		width: 220px;
	}
	.cover-cell.portrait {
		width: 124px;
	}
	.caption {
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
	.wanted ul {
		margin: 0;
		padding: 0;
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}
	.req {
		background: var(--badge-info);
		color: var(--badge-info-text);
		padding: 0 0.25rem;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.grid {
		display: grid;
		gap: 0.75rem;
		align-items: start;
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
	.hint {
		margin: 0;
	}
	.check {
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}
	.mini {
		font-size: var(--type-caption-size);
		padding: 0 0.3em;
	}
</style>
