<script lang="ts">
	import { MediaSlot } from '@scutoons/media';
	import { problemFor, type CatalogFormProps } from '../kinds.ts';
	import { m } from '../messages.ts';
	import LocaleText from './locale-text.svelte';

	/**
	 * A home scene (agents/research/CAVE.md "Data model"): what a home looks like – its name, the still background in
	 * 1080p (everyone) and 4K (the reader's "high" media quality), and optionally a short muted video loop, again 1080p +
	 * 4K (owner 2026-10-14: "4K und 1080p ist … ausreichend", "können auch Videos sein"). Uploads go to scutoons/homes/.
	 */
	let { data = $bindable(), problems }: CatalogFormProps = $props();

	const place = 'scutoons/homes/';
	const err = (...words: string[]) => problemFor(problems, ...words);
	let bust = $state(0);

	/** one file of `image` / `video`: `src` (1080p) or `variants.high` (4K); an empty video goes */
	function setFile(key: 'image' | 'video', level: 'src' | 'high', path: string | null | undefined) {
		const media = { src: '', ...(data[key] ?? {}) };
		if (level === 'src') media.src = path ?? '';
		else if (path) media.variants = { ...(media.variants ?? {}), high: path };
		else delete media.variants;
		if (key === 'video' && !media.src && !media.variants) delete data.video;
		else data[key] = media;
		bust = Date.now();
	}
</script>

<LocaleText label={m['catalog.name']()} required value={data.name} error={err('name')} onchange={(v) => (data.name = v)} testid="home-scene-name" />

<fieldset class="group" data-testid="home-scene-image">
	<legend>{m['catalog.home_scene_image']()}</legend>
	<div class="slots">
		<MediaSlot toon="" value={data.image?.src || undefined} label="1080p" {place} {bust} onchange={(path) => setFile('image', 'src', path)} testid="home-scene-image-1080" />
		<MediaSlot toon="" value={data.image?.variants?.high || undefined} label="4K" {place} {bust} onchange={(path) => setFile('image', 'high', path)} testid="home-scene-image-4k" />
	</div>
	{#if err('image')}<p class="sc-error">{err('image')}</p>{/if}
</fieldset>

<fieldset class="group" data-testid="home-scene-video">
	<legend>{m['catalog.home_scene_video']()}</legend>
	<p class="sc-caption muted">{m['catalog.home_scene_video_hint']()}</p>
	<div class="slots">
		<MediaSlot toon="" kind="video" value={data.video?.src || undefined} label="1080p" {place} {bust} onchange={(path) => setFile('video', 'src', path)} testid="home-scene-video-1080" />
		<MediaSlot toon="" kind="video" value={data.video?.variants?.high || undefined} label="4K" {place} {bust} onchange={(path) => setFile('video', 'high', path)} testid="home-scene-video-4k" />
	</div>
	{#if err('video')}<p class="sc-error">{err('video')}</p>{/if}
</fieldset>

<style>
	.group {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		margin-bottom: var(--space-15);
	}
	.slots {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
	}
	.muted {
		margin: 0;
		color: var(--muted);
	}
</style>
