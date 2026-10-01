<script lang="ts">
	import { entities, NEUTRAL_LOCALE } from '@scutoons/content';
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

	/**
	 * The toon's settings. Per language (entity toon-settings:<toon>, `ToonSettings`): title / description, plus release,
	 * status and covers (shared with per-language overrides). The same in every language (`TOON_NEUTRAL_FIELDS`: authors,
	 * chapter count, thumbnail, the landing tile, announce, the map) live once in the toon's index entry `toon:<toon>`,
	 * saved and published with the settings through the same bottom bar.
	 */
	let { toon, languages }: { toon: string; languages: ToonLanguages } = $props();

	// svelte-ignore state_referenced_locally
	const file = toonDoc(entities.toonSettings(toon), languages);
	// svelte-ignore state_referenced_locally
	const index = toonIndexDoc(toon);
	let images = $state<string[]>([]);
	const nav = getCreatorNav();

	onMount(async () => {
		await Promise.all([file.load(), index.load()]);
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
				{#each file.present as l}
					{@const value = file.get(l, ['language'])}
					<span class:warn={value !== l}>{l}: {value ?? '–'}</span>
					{#if value !== l}<button type="button" class="ghost mini" onclick={() => file.setLocal(l, ['language'], l)}>= {l}</button>{/if}
				{/each}
			</p>
			<div class="grid g3">
				<SharedField doc={file} path={['id']} label="id" required />
				<SharedField doc={file} path={['status']} label="status" kind="select" options={STATUSES} required />
				<SharedField doc={file} path={['release']} label="release" kind="date" required />
			</div>
			<div class="grid g3">
				<SharedField doc={file} path={['cover', 'square']} label="cover.square" kind="image" {toon} known={images} />
				<SharedField doc={file} path={['cover', 'portrait']} label="cover.portrait" kind="image" {toon} known={images} />
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
		font-size: 14px;
		margin: 0;
	}
	h3 {
		font-size: 13px;
		margin: 0;
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
		font-size: 10px;
		padding: 0 0.3em;
	}
</style>
