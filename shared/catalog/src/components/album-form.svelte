<script lang="ts">
	import { FileSlot, MediaSlot } from '@scutoons/media';
	import { problemFor, text, type CatalogFormProps, type Data } from '../kinds.ts';
	import { m } from '../messages.ts';
	import CatalogField from './catalog-field.svelte';
	import CatalogRows from './catalog-rows.svelte';
	import LocaleText from './locale-text.svelte';

	/**
	 * An album (agents/ITEMS.md "Players, albums, related items"): the music media items carry – title, cover, the
	 * download ZIP, the tracks (audio from the asset manager), who may download / listen.
	 */
	let { data = $bindable(), lists, problems }: CatalogFormProps = $props();

	const ACCESS = ['public', 'members', 'owners'];
	const toons = $derived((lists.toons ?? []).map((e) => ({ value: e.id, label: text(e.data?.name) || e.id })));
	const tracks = $derived<Data[]>(Array.isArray(data.tracks) ? data.tracks : []);
	const place = $derived(`${data.toon || 'scutoons'}/music/`);
	const err = (...words: string[]) => problemFor(problems, ...words);

	function setAccess(key: 'download' | 'listen', value: string | null) {
		const access = { ...(data.access ?? {}) };
		if (value === null) delete access[key];
		else access[key] = value;
		if (Object.keys(access).length) data.access = access;
		else delete data.access;
	}
</script>

<CatalogField label={m['catalog.toon']()} type="select" value={data.toon} options={toons} none={m['catalog.toon_none']()} error={err('toon')} onchange={(v) => (data.toon = v)} />
<LocaleText label={m['catalog.title_field']()} required value={data.title} error={err('title')} onchange={(v) => (data.title = v)} testid="album-title" />
<div class="media">
	<MediaSlot toon="" value={data.cover ?? undefined} label={m['catalog.cover']()} {place} onchange={(path) => (path ? (data.cover = path) : delete data.cover)} testid="album-cover" />
	<FileSlot kind="archive" value={data.download} label={m['catalog.album_download']()} {place} onchange={(path) => (path ? (data.download = path) : delete data.download)} testid="album-download" />
</div>
{#if err('cover', 'download')}<p class="sc-error">{err('cover', 'download')}</p>{/if}

<div class="sc-fields" data-cols="fit" style:--fields-fit-min="10rem">
	<CatalogField label={m['catalog.access_download']()} type="select" value={data.access?.download} options={ACCESS.map((a) => ({ value: a }))} none={m['catalog.access_default']({ value: 'members' })} onchange={(v) => setAccess('download', v)} />
	<CatalogField label={m['catalog.access_listen']()} type="select" value={data.access?.listen} options={ACCESS.map((a) => ({ value: a }))} none={m['catalog.access_default']({ value: 'owners' })} onchange={(v) => setAccess('listen', v)} />
</div>

<CatalogRows
	legend={m['catalog.album_tracks']()}
	items={tracks}
	onchange={(next) => (data.tracks = next)}
	create={() => ({ id: '', title: '', src: '' })}
	addLabel={m['catalog.add_track']()}
	removeLabel={m['catalog.remove']()}
	reorder
	error={err('track', 'tracks', 'src', 'duration')}
	testid="album-tracks"
	rowTestid="album-track"
	addTestid="album-add-track"
>
	{#snippet row(t)}
		<div class="sc-fields" data-cols="fit" style:--fields-fit-min="10rem">
			<CatalogField label="id" value={t.id} onchange={(v) => (t.id = v)} />
			<CatalogField
				label={m['catalog.duration']()}
				type="number"
				value={t.duration}
				onchange={(v) => {
					if (v === null) delete t.duration;
					else t.duration = v;
				}}
			/>
			<!-- the record side (owner 2026-10-16): "Turn the record" plays the other side's songs; none = A -->
			<CatalogField
				label={m['catalog.track_side']()}
				type="select"
				value={t.side}
				options={[{ value: 'A' }, { value: 'B' }]}
				none={m['catalog.access_default']({ value: 'A' })}
				onchange={(v) => {
					if (v === null) delete t.side;
					else t.side = v;
				}}
			/>
		</div>
		<CatalogField label={m['catalog.title_field']()} value={t.title} onchange={(v) => (t.title = v)} />
		<FileSlot kind="audio" value={t.src} label={m['catalog.audio']()} {place} onchange={(path) => (t.src = path ?? '')} testid="album-track-src" />
		<!-- quality files (owner 2026-10-16): low 96k / medium 128k / high 320k, set through a proposal (vault files have no picker) -->
		{#if t.variants && Object.keys(t.variants).length}
			<p class="sc-caption muted" data-testid="album-track-variants">
				{m['catalog.track_variants']({ levels: ['low', 'medium', 'high'].filter((l) => t.variants[l]).map((l) => ({ low: '96k', medium: '128k', high: '320k' })[l]).join(' · ') })}
			</p>
		{/if}
	{/snippet}
</CatalogRows>

<style>
	.media {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1) var(--space-15);
		margin-bottom: var(--space-15);
	}
</style>
