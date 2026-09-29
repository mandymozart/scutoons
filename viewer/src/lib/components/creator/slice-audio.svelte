<script lang="ts">
	import type { ContentDoc } from '$lib/creator/doc.svelte.ts';
	import { getAt, deleteAt, type Path } from '$lib/creator/paths.ts';
	import { Trigger, TriggerMode } from '$lib/models/enums.ts';
	import PropField from './prop-field.svelte';

	/** Per-slice audio: music volumes (per chapter track) and samples with triggers (rects drawn on the canvas). */
	let {
		doc,
		slicePath,
		chapterAudio,
		selectedSample = -1,
		onselectsample
	}: {
		doc: ContentDoc;
		slicePath: Path;
		/** the chapter's audio config (tracks, sampleLibrary) */
		chapterAudio: any;
		selectedSample?: number;
		onselectsample?: (index: number) => void;
	} = $props();

	const slice = $derived(doc.get(doc.primary, slicePath));
	const musicTracks = $derived<any[]>(chapterAudio?.music?.tracks ?? []);
	const library = $derived<string[]>((chapterAudio?.sampleLibrary ?? []).map((s: any) => s.id));
	const sliceMusic = $derived<any[]>(slice?.audio?.music?.tracks ?? []);
	const samples = $derived<any[]>(slice?.audio?.audio?.samples ?? []);
	const misplaced = $derived<any[]>(Array.isArray(slice?.audio?.samples) ? slice.audio.samples : []);

	const samplesPath = $derived([...slicePath, 'audio', 'audio', 'samples']);

	function volumeOf(id: string): number | undefined {
		return sliceMusic.find((t) => t.id === id)?.volume;
	}

	function setVolume(id: string, volume: number | undefined) {
		doc.structural((file) => {
			const s = getAt(file, slicePath);
			if (!s) return;
			if (volume === undefined) {
				const list = s.audio?.music?.tracks;
				if (Array.isArray(list)) {
					const i = list.findIndex((t: any) => t.id === id);
					if (i >= 0) list.splice(i, 1);
					if (!list.length) deleteAt(file, [...slicePath, 'audio', 'music', 'tracks'], slicePath.length);
				}
				return;
			}
			s.audio ??= {};
			s.audio.music ??= { tracks: [] };
			s.audio.music.tracks ??= [];
			const entry = s.audio.music.tracks.find((t: any) => t.id === id);
			if (entry) entry.volume = volume;
			else s.audio.music.tracks.push({ id, volume });
		}, `volume:${slicePath.join('.')}:${id}`);
	}

	function addSample(type: Trigger) {
		doc.structural((file) => {
			const s = getAt(file, slicePath);
			if (!s) return;
			s.audio ??= {};
			s.audio.audio ??= { samples: [] };
			s.audio.audio.samples ??= [];
			const trigger: any = { type, mode: type === Trigger.INTERSECT ? TriggerMode.MONO : TriggerMode.POLY };
			if (type !== Trigger.INTERSECT) trigger.rect = { x: 25, y: 25, width: 50, height: 50 };
			s.audio.audio.samples.push({ id: library[0] ?? '', trigger, volume: 0.6 });
		});
		onselectsample?.(samples.length);
	}

	function removeSample(index: number) {
		doc.structural((file) => {
			const list = getAt(file, samplesPath);
			if (!Array.isArray(list)) return;
			list.splice(index, 1);
			if (!list.length) deleteAt(file, samplesPath, slicePath.length);
		});
		onselectsample?.(-1);
	}

	function setTriggerType(index: number, type: string) {
		doc.structural((file) => {
			const trigger = getAt(file, [...samplesPath, index, 'trigger']);
			if (!trigger) return;
			trigger.type = type;
			if (type === Trigger.INTERSECT) delete trigger.rect;
			else trigger.rect ??= { x: 25, y: 25, width: 50, height: 50 };
		});
	}

	/** Known bug in older configs: samples at audio.samples are never played (reader reads audio.audio.samples). */
	function moveMisplaced() {
		doc.structural((file) => {
			const s = getAt(file, slicePath);
			if (!Array.isArray(s?.audio?.samples)) return;
			s.audio.audio ??= {};
			s.audio.audio.samples = [...(s.audio.audio.samples ?? []), ...s.audio.samples];
			delete s.audio.samples;
		});
	}
</script>

<section class="slice-audio" data-testid="slice-audio">
	<h4>Music volumes</h4>
	{#if !musicTracks.length}
		<p class="hint">The chapter has no music tracks (Audio in the tree).</p>
	{/if}
	{#each musicTracks as track (track.id)}
		{@const v = volumeOf(track.id)}
		<div class="volume">
			<label>
				<input type="checkbox" checked={v !== undefined} onchange={(e) => setVolume(track.id, e.currentTarget.checked ? 0.5 : undefined)} />
				{track.id}
			</label>
			<input type="range" min="0" max="1" step="0.05" value={v ?? 0} disabled={v === undefined} oninput={(e) => setVolume(track.id, Number(e.currentTarget.value))} />
			<span class="num">{v === undefined ? 'unchanged' : v.toFixed(2)}</span>
		</div>
	{/each}

	<h4>Samples</h4>
	{#if misplaced.length}
		<p class="warn">
			{misplaced.length} sample(s) at <code>audio.samples</code> never play (the reader reads <code>audio.audio.samples</code>).
			<button type="button" class="secondary" onclick={moveMisplaced}>Move them</button>
		</p>
	{/if}
	{#each samples as sample, i (i)}
		<div class="sample" class:selected={i === selectedSample}>
			<div class="row">
				<button type="button" class="ghost" onclick={() => onselectsample?.(i)}>♪ {i + 1}</button>
				<button type="button" class="ghost" onclick={() => removeSample(i)}>✕</button>
			</div>
			<div class="grid g3">
				<PropField {doc} path={[...samplesPath, i, 'id']} label="id" type="select" options={library} required />
				<PropField {doc} path={[...samplesPath, i, 'volume']} label="volume" type="number" step={0.05} min={0} max={1} placeholder="1" />
				<PropField {doc} path={[...samplesPath, i, 'delay']} label="delay ms" type="number" step={100} placeholder="0" />
				<div class="field">
					<label for="tt-{i}">trigger</label>
					<select id="tt-{i}" value={sample.trigger?.type ?? ''} onchange={(e) => setTriggerType(i, e.currentTarget.value)}>
						{#each Object.values(Trigger) as t}<option value={t}>{t}</option>{/each}
					</select>
				</div>
				<PropField {doc} path={[...samplesPath, i, 'trigger', 'mode']} label="mode" type="select" options={Object.values(TriggerMode)} placeholder="mono" />
				<PropField {doc} path={[...samplesPath, i, 'loop']} label="loop" type="bool" placeholder="false" />
				<PropField {doc} path={[...samplesPath, i, 'trigger', 'exclusive']} label="exclusive" type="bool" placeholder="false" />
				<PropField {doc} path={[...samplesPath, i, 'trigger', 'gate']} label="gate" type="bool" placeholder="false" />
			</div>
			{#if sample.trigger?.rect}
				<p class="hint">Area: {sample.trigger.rect.x}/{sample.trigger.rect.y} {sample.trigger.rect.width}×{sample.trigger.rect.height}% – drag it on the slice.</p>
			{/if}
		</div>
	{/each}
	<div class="row">
		<button type="button" class="secondary" onclick={() => addSample(Trigger.INTERSECT)} disabled={!library.length}>+ on scroll</button>
		<button type="button" class="secondary" onclick={() => addSample(Trigger.CLICK)} disabled={!library.length}>+ on click (area)</button>
	</div>
	<p class="hint">Or pick the “sample area” tool and draw on the slice.</p>
</section>

<style>
	.slice-audio {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	h4 {
		font-size: 11px;
		margin: 0.5rem 0 0;
		text-transform: uppercase;
	}
	.volume {
		display: grid;
		grid-template-columns: 8rem 1fr 5rem;
		align-items: center;
		gap: 0.4rem;
	}
	.volume label {
		display: flex;
		gap: 0.3rem;
		align-items: center;
	}
	.num {
		font-size: 11px;
		color: var(--muted);
	}
	.sample {
		border: 1px solid var(--border-subtle);
		padding: 0.3rem;
	}
	.sample.selected {
		border: 2px solid #b70aff;
	}
	.row {
		display: flex;
		gap: 0.4rem;
		justify-content: space-between;
		flex-wrap: wrap;
	}
	.grid {
		display: grid;
		gap: 0.3rem 0.4rem;
	}
	.g3 {
		grid-template-columns: repeat(3, minmax(0, 1fr));
	}
	.field label {
		font-size: 10px;
		color: var(--muted);
	}
	.warn {
		color: var(--warn);
		font-size: 11px;
	}
</style>
