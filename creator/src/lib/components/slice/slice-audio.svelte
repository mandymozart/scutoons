<script lang="ts">
	import { untrack } from 'svelte';
	import { CloseIcon, Icon } from '@scutoons/ui';
	import { MediaVariantsSlot, type VariantMedia } from '@scutoons/media';
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { getAt, deleteAt, type Path } from '@scutoons/content';
	import { Trigger, TriggerMode } from '@scutoons/content';
	import PropField from '$lib/components/fields/prop-field.svelte';

	/**
	 * Per-slice audio: music volumes (per chapter track) and samples with triggers (rects drawn on the canvas). A sample is
	 * one line until opened, like the VN events (owner 2026-10-04); its sound is a slot with the asset picker (+ upload) –
	 * a file not in the chapter's sample library yet is added to it (`librarySample`).
	 */
	let {
		doc,
		slicePath,
		chapterAudio,
		librarySample,
		heldMusic = {},
		toon,
		place,
		selectedSample = -1,
		onselectsample
	}: {
		doc: ContentDoc;
		slicePath: Path;
		/** the chapter's audio config (tracks, sampleLibrary) */
		chapterAudio: any;
		librarySample: (media: VariantMedia) => string;
		/** per track: the level it holds from an earlier slice (none = silent) */
		heldMusic?: Record<string, { volume: number; label: string }>;
		toon: string;
		/** where uploads go (the chapter folder) */
		place: string;
		selectedSample?: number;
		onselectsample?: (index: number) => void;
	} = $props();

	const slice = $derived(doc.get(doc.primary, slicePath));
	const musicTracks = $derived<any[]>(chapterAudio?.music?.tracks ?? []);
	const libraryEntries = $derived<any[]>(chapterAudio?.sampleLibrary ?? []);
	const entryOf = (id: string): any => libraryEntries.find((s) => s?.id === id);
	const pathOf = (id: string): string | undefined => entryOf(id)?.path || undefined;
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
		// the new one's index before the push (read after it, the derived list already has it: one past – the old bug)
		const index = samples.length;
		doc.structural((file) => {
			const s = getAt(file, slicePath);
			if (!s) return;
			s.audio ??= {};
			s.audio.audio ??= { samples: [] };
			s.audio.audio.samples ??= [];
			const trigger: any = { type, mode: type === Trigger.INTERSECT ? TriggerMode.MONO : TriggerMode.POLY };
			if (type !== Trigger.INTERSECT) trigger.rect = { x: 25, y: 25, width: 50, height: 50 };
			s.audio.audio.samples.push({ id: '', trigger, volume: 0.6 });
		});
		open = [...open, index];
		onselectsample?.(index);
	}

	/** picked / uploaded files: their library id (added to the library when new, levels updated) becomes the sample's */
	function setSound(index: number, media: VariantMedia) {
		if (!media.src) return;
		const id = librarySample(media);
		doc.structural((file) => {
			const sample = getAt(file, [...samplesPath, index]);
			if (sample) sample.id = id;
		});
	}

	// ── one line until opened (like the VN events); the sample selected on the canvas opens ──
	let open = $state<number[]>([]);
	$effect(() => {
		if (selectedSample >= 0 && !untrack(() => open).includes(selectedSample)) open = [...untrack(() => open), selectedSample];
	});
	const isOpen = (i: number) => open.includes(i);
	function clickLine(i: number) {
		open = isOpen(i) ? open.filter((x) => x !== i) : [...open, i];
		onselectsample?.(i);
	}
	const TRIGGER_WORDS: Record<string, string> = { intersect: 'on scroll', click: 'on click', hover: 'on hover' };
	const triggerWord = (sample: any) => `${TRIGGER_WORDS[sample?.trigger?.type] ?? sample?.trigger?.type ?? ''}${sample?.trigger?.rect ? ' · area' : ''}`;

	function removeSample(index: number) {
		doc.structural((file) => {
			const list = getAt(file, samplesPath);
			if (!Array.isArray(list)) return;
			list.splice(index, 1);
			if (!list.length) deleteAt(file, samplesPath, slicePath.length);
		});
		open = open.filter((i) => i !== index).map((i) => (i > index ? i - 1 : i));
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
	<h4>Music</h4>
	{#if !musicTracks.length}
		<p class="hint">The chapter has no music tracks (Audio in the tree).</p>
	{/if}
	<!-- a mixer (owner 2026-10-04): one strip per chapter track. A track the slice doesn't set holds the level of an earlier
	     slice (shown faint, "from …"); moving its fader sets it here, × hands it back to the earlier level. -->
	<div class="mixer" data-testid="slice-mixer">
		{#each musicTracks as track (track.id)}
			{@const v = volumeOf(track.id)}
			{@const held = heldMusic[track.id]}
			{@const level = v ?? held?.volume ?? 0}
			<div class="strip" class:held={v === undefined} data-testid="mixer-{track.id}">
				<span class="val">{Math.round(level * 100)}</span>
				<input class="fader" type="range" min="0" max="1" step="0.05" value={level} aria-label="{track.id} volume" oninput={(e) => setVolume(track.id, Number(e.currentTarget.value))} />
				<span class="name" title={track.id}>{track.id}</span>
				{#if v === undefined}
					<span class="from" title={held ? `holds ${Math.round(held.volume * 100)} from ${held.label}` : 'silent – no earlier slice sets it'}>{held ? held.label : 'silent'}</span>
				{:else}
					<button type="button" class="sc-round" aria-label="{track.id}: hold the earlier level" title="Hold the earlier level ({held ? `${Math.round(held.volume * 100)}, ${held.label}` : 'silent'})" onclick={() => setVolume(track.id, undefined)}><CloseIcon /></button>
				{/if}
			</div>
		{/each}
	</div>

	<h4>Samples</h4>
	{#if misplaced.length}
		<p class="warn">
			{misplaced.length} sample(s) at <code>audio.samples</code> never play (the reader reads <code>audio.audio.samples</code>).
			<button type="button" class="secondary" onclick={moveMisplaced}>Move them</button>
		</p>
	{/if}
	{#each samples as sample, i (i)}
		{@const file = pathOf(sample?.id)}
		<article class="sample" class:selected={i === selectedSample} class:open={isOpen(i)} data-testid="sample-{i}">
			<header>
				<button type="button" class="ghost line" onclick={() => clickLine(i)} aria-expanded={isOpen(i)} data-testid="sample-line-{i}">
					<Icon name="chevron-right" />
					<span class="idx">{i + 1}</span>
					<span class="type">{sample?.id || '–'}</span>
					<span class="sum">{file?.split('/').pop() ?? ''}</span>
					<span class="trig">{triggerWord(sample)}</span>
				</button>
				<span class="tools">
					<button type="button" class="ghost" onclick={() => removeSample(i)} aria-label={m['creator.remove']()} title={m['creator.remove']()}><CloseIcon /></button>
				</span>
			</header>
			{#if isOpen(i)}
				<MediaVariantsSlot {toon} kind="audio" label="sound" value={{ src: file, variants: entryOf(sample?.id)?.variants }} {place} clearable={false} onchange={(media) => setSound(i, media)} testid="sample-sound-{i}" />
				{#if sample?.id && !file}
					<p class="warn">"{sample.id}" is not in the chapter's sample library – pick its file.</p>
				{/if}
				<div class="sc-fields" data-cols="2" data-density="dense">
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
					<p class="hint">Area: {sample.trigger.rect.x}/{sample.trigger.rect.y} {sample.trigger.rect.width}×{sample.trigger.rect.height}%</p>
				{/if}
			{/if}
		</article>
	{/each}
	<div class="row">
		<button type="button" class="secondary" onclick={() => addSample(Trigger.INTERSECT)}>+ on scroll</button>
		<button type="button" class="secondary" onclick={() => addSample(Trigger.CLICK)}>+ on click (area)</button>
	</div>
</section>

<style>
	.slice-audio {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	h4 {
		margin: var(--space-1) 0 0;
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.mixer {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
	}
	.strip {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.3rem;
		width: 4.5rem;
		padding: 0.4rem 0.25rem;
		border: var(--border-width-regular) solid var(--ink);
	}
	/* held from an earlier slice: faint, dashed – moving the fader sets it here */
	.strip.held {
		border-style: dashed;
		color: var(--text-muted);
	}
	.strip.held .fader {
		opacity: 0.45;
	}
	.val {
		font-variant-numeric: tabular-nums;
	}
	.fader {
		writing-mode: vertical-lr;
		direction: rtl;
		width: 1.5rem;
		height: 8rem;
		margin: 0;
	}
	.name,
	.from {
		max-width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.from {
		font-size: var(--type-caption-size);
		line-height: var(--control-height);
	}
	/* a sample = one line until opened, the VN events' look (vn-events.svelte) */
	.sample {
		border: 1px solid var(--border-muted);
		padding: 0.4rem;
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}
	.sample.selected {
		border: 2px solid var(--ink);
	}
	header {
		display: flex;
		gap: 0.4rem;
		align-items: center;
	}
	.line {
		flex: 1;
		min-width: 0;
		justify-content: flex-start;
		gap: 0.5rem;
		padding: 0 0.25rem;
		text-align: left;
	}
	.line,
	.line:hover {
		border-color: transparent;
		background: none;
	}
	.line :global(svg) {
		flex: none;
		transition: transform var(--duration-base) var(--easing);
	}
	.sample.open .line :global(svg) {
		transform: rotate(90deg);
	}
	.line .idx {
		flex: none;
		min-width: 1.5em;
	}
	.line .type {
		flex: none;
	}
	.line .sum,
	.line .trig {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--text-muted);
	}
	.line .sum {
		flex: 1 1 auto;
		min-width: 3em;
	}
	.line .trig {
		flex: 0 4 auto;
	}
	.tools {
		margin-left: auto;
		display: flex;
		flex: none;
	}
	.row {
		display: flex;
		gap: 0.4rem;
		justify-content: space-between;
		flex-wrap: wrap;
	}
	.field label {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
	}
	.warn {
		color: var(--warn);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
</style>
