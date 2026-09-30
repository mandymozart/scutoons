<script lang="ts">
	import { DEFAULT_SLICE, getAt, type Path } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { fillSliceSize } from '$lib/editor/images.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import ImagePicker from './image-picker.svelte';

	/**
	 * The media of a slice: an image, or a video with the image as its poster. Width / height are read from the image
	 * on the CDN whenever its path is set (every slice carries its size; the standard is 390 × 625).
	 */
	let { doc, slicePath, toon, known = [] }: { doc: ContentDoc; slicePath: Path; toon: string; known?: string[] } = $props();

	const slice = $derived<any>(doc.get(doc.primary, slicePath));
	const video = $derived<{ src?: string; loop?: boolean; autoplay?: boolean } | undefined>(slice?.video);
	const url = (path: string) => asset(`${toon}/${path}`);

	let videoState = $state<'pending' | 'ok' | 'missing' | null>(null);

	// a slice without a size (older data) gets it as soon as it is opened
	$effect(() => {
		if (slice?.image && (!(slice.width > 0) || !(slice.height > 0))) fillSliceSize(doc, slicePath, url);
	});

	$effect(() => {
		videoState = video?.src ? 'pending' : null;
	});

	function setImage(path: string) {
		doc.setShared([...slicePath, 'image'], path);
		fillSliceSize(doc, slicePath, url);
	}

	function setMedia(kind: 'image' | 'video') {
		if (kind === 'video' && !video) doc.setShared([...slicePath, 'video'], { src: '' }, 'slice-media');
		if (kind === 'image' && video)
			doc.structural((file) => {
				const s = getAt(file, slicePath);
				if (s) delete s.video;
			}, 'slice-media');
	}

	/** loop / autoplay: stored only when they differ from the default (true) */
	function setFlag(key: 'loop' | 'autoplay', on: boolean) {
		doc.structural((file) => {
			const v = getAt(file, [...slicePath, 'video']);
			if (!v) return;
			if (on) delete v[key];
			else v[key] = false;
		}, `slice-video-${key}`);
	}
</script>

<div class="media" data-testid="slice-media">
	<div class="kind" role="radiogroup" aria-label={m['creator.media_type']()}>
		<label><input type="radio" name="slice-media" checked={!video} onchange={() => setMedia('image')} /> {m['creator.media_image']()}</label>
		<label><input type="radio" name="slice-media" checked={!!video} onchange={() => setMedia('video')} data-testid="media-video" /> {m['creator.media_video']()}</label>
	</div>

	{#if video}
		<label class="field">
			<span class="lbl">video.src</span>
			<input
				type="text"
				value={video.src ?? ''}
				placeholder="videos/s1c1-key-visual.900w.mp4"
				onchange={(e) => doc.setShared([...slicePath, 'video', 'src'], e.currentTarget.value.trim())}
				data-testid="video-path"
			/>
		</label>
		{#if video.src}
			<div class="preview">
				<!-- svelte-ignore a11y_media_has_caption -->
				<video
					src={url(video.src)}
					muted
					loop
					autoplay
					playsinline
					preload="metadata"
					onloadedmetadata={() => (videoState = 'ok')}
					onerror={() => (videoState = 'missing')}
				></video>
				<span class="meta">
					{#if videoState === 'missing'}<b class="bad">{m['creator.video_missing']()}</b>{:else if videoState === 'ok'}✓ CDN{/if}
				</span>
			</div>
		{/if}
		<div class="flags">
			<label><input type="checkbox" checked={video.loop !== false} onchange={(e) => setFlag('loop', e.currentTarget.checked)} /> loop</label>
			<label><input type="checkbox" checked={video.autoplay !== false} onchange={(e) => setFlag('autoplay', e.currentTarget.checked)} /> autoplay</label>
		</div>
		<p class="hint">{m['creator.video_hint']()}</p>
	{/if}

	<ImagePicker {toon} label={video ? m['creator.poster']() : 'image'} value={slice?.image} {known} showFormat onchange={setImage} />
	{#each doc.overrides([...slicePath, 'image']) as l}
		<p class="hint">
			{l}: {doc.get(l, [...slicePath, 'image'])} ({m['creator.override']()})
			<button class="ghost" onclick={() => doc.resetOverride(l, [...slicePath, 'image'])}>{m['creator.use_shared']()}</button>
		</p>
	{/each}
	<p class="hint" data-testid="slice-size">
		{m['creator.slice_size']({ width: slice?.width ?? '–', height: slice?.height ?? '–', standard: `${DEFAULT_SLICE.width} × ${DEFAULT_SLICE.height}` })}
	</p>
</div>

<style>
	.media {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.kind,
	.flags {
		display: flex;
		gap: 0.8rem;
		font-size: 12px;
	}
	.field {
		display: flex;
		flex-direction: column;
	}
	.lbl {
		font-size: 10px;
		color: var(--muted);
	}
	.preview {
		display: flex;
		gap: 0.5rem;
		align-items: flex-start;
	}
	.preview video {
		width: 64px;
		height: auto;
		border: 1px solid var(--border-subtle);
	}
	.meta {
		font-size: 11px;
	}
	.bad {
		color: var(--error);
	}
</style>
