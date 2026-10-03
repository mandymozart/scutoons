<script lang="ts">
	import { CloseIcon } from '@scutoons/ui';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { getAt } from '@scutoons/content';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PropField from '$lib/components/fields/prop-field.svelte';

	/** Chapter audio: music tracks (adaptive music player) and the sample library. Shared by all locales. */
	let { doc }: { doc: ContentDoc } = $props();

	const audio = $derived(doc.get(doc.primary, ['audio']));
	const tracks = $derived<any[]>(audio?.music?.tracks ?? []);
	const samples = $derived<any[]>(audio?.sampleLibrary ?? []);

	let playing: HTMLAudioElement | null = null;
	let playingPath = $state<string | null>(null);

	function play(path: string) {
		playing?.pause();
		if (playingPath === path) {
			playingPath = null;
			return;
		}
		playing = new Audio(asset(`${doc.toon}/${path}`));
		playing.volume = 0.6;
		playing.onended = () => (playingPath = null);
		playing.play().catch(() => (playingPath = null));
		playingPath = path;
	}

	function addTo(list: 'tracks' | 'sampleLibrary') {
		doc.structural((file) => {
			file.audio ??= {};
			if (list === 'tracks') {
				file.audio.music ??= { transitionDuration: 3000, tracks: [] };
				file.audio.music.tracks ??= [];
				file.audio.music.tracks.push({ id: `track${file.audio.music.tracks.length + 1}`, path: '' });
			} else {
				file.audio.sampleLibrary ??= [];
				file.audio.sampleLibrary.push({ id: `sample${file.audio.sampleLibrary.length + 1}`, path: '' });
			}
		});
	}

	function removeFrom(list: 'tracks' | 'sampleLibrary', index: number) {
		const path = list === 'tracks' ? ['audio', 'music', 'tracks'] : ['audio', 'sampleLibrary'];
		const item = getAt(doc.data[doc.primary], [...path, index]);
		if (!confirm(m['creator.remove_confirm']({ what: item?.id ?? '' }))) return;
		doc.structural((file) => getAt(file, path)?.splice(index, 1));
	}
</script>

<section class="audio-panel" data-testid="audio-panel">
	<h2>{m['creator.audio']()}</h2>

	<h3>Music</h3>
	<div class="sc-fields" data-cols="2">
		<PropField {doc} path={['audio', 'music', 'transitionDuration']} label="transitionDuration (ms)" type="number" step={100} placeholder="3000" />
		<PropField {doc} path={['audio', 'music', 'defaultTrack']} label="defaultTrack" type="select" options={tracks.map((t) => t.id)} placeholder="none" />
	</div>
	<table>
		<thead><tr><th>id</th><th>path</th><th></th></tr></thead>
		<tbody>
			{#each tracks as track, i (i)}
				<tr>
					<td><PropField {doc} path={['audio', 'music', 'tracks', i, 'id']} label="" required /></td>
					<td><PropField {doc} path={['audio', 'music', 'tracks', i, 'path']} label="" required /></td>
					<td class="row-tools">
						<button type="button" class="ghost" onclick={() => play(track.path)}>{playingPath === track.path ? '■' : '▶'}</button>
						<button type="button" class="ghost" onclick={() => removeFrom('tracks', i)} aria-label={m['creator.remove']()} title={m['creator.remove']()}><CloseIcon /></button>
					</td>
				</tr>
			{/each}
		</tbody>
	</table>
	<button type="button" class="secondary" onclick={() => addTo('tracks')}>+ track</button>

	<h3>Sample library</h3>
	<table>
		<thead><tr><th>id</th><th>path</th><th></th></tr></thead>
		<tbody>
			{#each samples as sample, i (i)}
				<tr>
					<td><PropField {doc} path={['audio', 'sampleLibrary', i, 'id']} label="" required /></td>
					<td><PropField {doc} path={['audio', 'sampleLibrary', i, 'path']} label="" required /></td>
					<td class="row-tools">
						<button type="button" class="ghost" onclick={() => play(sample.path)}>{playingPath === sample.path ? '■' : '▶'}</button>
						<button type="button" class="ghost" onclick={() => removeFrom('sampleLibrary', i)} aria-label={m['creator.remove']()} title={m['creator.remove']()}><CloseIcon /></button>
					</td>
				</tr>
			{/each}
		</tbody>
	</table>
	<button type="button" class="secondary" onclick={() => addTo('sampleLibrary')}>+ sample</button>
</section>

<style>
	.audio-panel {
		max-width: 52rem;
	}
	h3 {
		margin-top: 1rem;
	}
	.sc-fields {
		margin-bottom: 0.5rem;
	}
	table {
		width: 100%;
		border-collapse: collapse;
		margin-bottom: 0.4rem;
	}
	th {
		text-align: left;
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
		border-bottom: 2px solid var(--ink);
	}
	td {
		padding: 0.15rem 0.25rem 0.15rem 0;
		vertical-align: top;
	}
	td:nth-child(1) {
		width: 10rem;
	}
	.row-tools {
		white-space: nowrap;
		width: 4rem;
	}
</style>
