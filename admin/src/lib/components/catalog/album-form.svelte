<script lang="ts">
	import { MediaSlot } from '@scutoons/media';
	import { Button, CloseIcon, Icon } from '@scutoons/ui';
	import { problemFor, text, type CatalogFormProps, type Data } from '$lib/catalog/kinds.ts';
	import { m } from '$lib/paraglide/messages.js';
	import CatalogField from './catalog-field.svelte';
	import FileSlot from './file-slot.svelte';
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
	function moveTrack(i: number, by: number) {
		const next = [...tracks];
		const [t] = next.splice(i, 1);
		next.splice(i + by, 0, t);
		data.tracks = next;
	}
</script>

<CatalogField label={m['catalog.toon']()} type="select" value={data.toon} options={toons} none={m['catalog.toon_none']()} error={err('toon')} onchange={(v) => (data.toon = v)} />
<LocaleText label={m['catalog.title_field']()} required value={data.title} error={err('title')} onchange={(v) => (data.title = v)} testid="album-title" />
<div class="media">
	<MediaSlot toon="" value={data.cover ?? undefined} label={m['catalog.cover']()} {place} onchange={(path) => (path ? (data.cover = path) : delete data.cover)} testid="album-cover" />
	<FileSlot kind="archive" value={data.download} label={m['catalog.album_download']()} {place} onchange={(path) => (path ? (data.download = path) : delete data.download)} testid="album-download" />
</div>
{#if err('cover', 'download')}<p class="error">{err('cover', 'download')}</p>{/if}

<div class="sc-fields" data-cols="fit" style:--fields-fit-min="10rem">
	<CatalogField label={m['catalog.access_download']()} type="select" value={data.access?.download} options={ACCESS.map((a) => ({ value: a }))} none={m['catalog.access_default']({ value: 'members' })} onchange={(v) => setAccess('download', v)} />
	<CatalogField label={m['catalog.access_listen']()} type="select" value={data.access?.listen} options={ACCESS.map((a) => ({ value: a }))} none={m['catalog.access_default']({ value: 'owners' })} onchange={(v) => setAccess('listen', v)} />
</div>

<fieldset class="tracks" data-testid="album-tracks">
	<legend>{m['catalog.album_tracks']()}</legend>
	{#if err('track', 'tracks', 'src', 'duration')}<p class="error">{err('track', 'tracks', 'src', 'duration')}</p>{/if}
	{#each tracks as t, i (i)}
		<div class="track" data-testid="album-track">
			<div class="head">
				<span class="sc-caption">{i + 1}</span>
				<span class="tools">
					<button type="button" class="sc-round" disabled={i === 0} onclick={() => moveTrack(i, -1)} title={m['catalog.up']()} aria-label={m['catalog.up']()}><Icon name="arrow-up" /></button>
					<button type="button" class="sc-round" disabled={i === tracks.length - 1} onclick={() => moveTrack(i, 1)} title={m['catalog.down']()} aria-label={m['catalog.down']()}><Icon name="arrow-down" /></button>
					<button type="button" class="sc-round" onclick={() => (data.tracks = tracks.filter((_, j) => j !== i))} title={m['catalog.remove']()} aria-label={m['catalog.remove']()}><CloseIcon /></button>
				</span>
			</div>
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
			</div>
			<CatalogField label={m['catalog.title_field']()} value={t.title} onchange={(v) => (t.title = v)} />
			<FileSlot kind="audio" value={t.src} label={m['catalog.audio']()} {place} onchange={(path) => (t.src = path ?? '')} testid="album-track-src" />
		</div>
	{/each}
	<span><Button size="sm" variant="secondary" onclick={() => (data.tracks = [...tracks, { id: '', title: '', src: '' }])} data-testid="album-add-track">{m['catalog.add_track']()}</Button></span>
</fieldset>

<style>
	.media {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1) var(--space-15);
		margin-bottom: var(--space-15);
	}
	.tracks {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.track {
		padding-bottom: var(--space-1);
		border-bottom: 1px solid var(--border-subtle, currentColor);
	}
	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: var(--space-05);
	}
	.tools {
		display: inline-flex;
		gap: var(--space-025);
	}
	.error {
		margin: 0 0 var(--space-1);
		color: var(--state-error);
	}
</style>
