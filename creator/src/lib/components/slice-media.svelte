<script lang="ts">
	import { DEFAULT_SLICE, MEDIA_QUALITIES, VIDEO_QUALITY_LABELS, getAt, type MediaQuality, type Path, type SliceVideo, type SliceViewport } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { fillSliceSize } from '$lib/editor/images.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import ImagePicker from './image-picker.svelte';

	/**
	 * The slice's background ("Hintergrund"): an image, or a video with the image as its poster. Width / height are read
	 * from the image on the CDN whenever its path is set (every slice carries its size; the standard is 390 × 625).
	 * A video has one file per quality level (480p / 720p / 1080p = `video.variants`, one of them the default `video.src`),
	 * the flags player / loop / autoplay / controls and its placement (in the column or full viewport, agents/DESIGN.md
	 * "Video placement").
	 */
	let { doc, slicePath, toon, known = [] }: { doc: ContentDoc; slicePath: Path; toon: string; known?: string[] } = $props();

	const slice = $derived<any>(doc.get(doc.primary, slicePath));
	const video = $derived<Partial<SliceVideo> | undefined>(slice?.video);
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

	/** edits `video` of the shown slice (shared by every language) */
	function editVideo(change: (v: any) => void, key: string) {
		doc.structural((file) => {
			const v = getAt(file, [...slicePath, 'video']);
			if (v) change(v);
		}, key);
	}

	/**
	 * player: a video to watch (sound, controls, no loop / autoplay); stored only when on. The slice then takes the
	 * poster's ratio at the slice width (the video's ratio, no 390 × 625 crop).
	 */
	function setPlayer(on: boolean) {
		editVideo((v) => {
			if (on) {
				v.player = true;
				delete v.loop;
				delete v.autoplay;
			} else delete v.player;
		}, 'slice-video-player');
		if (on) fillSliceSize(doc, slicePath, url);
	}

	/** loop / autoplay: stored only when they differ from the default (true) */
	function setFlag(key: 'loop' | 'autoplay', on: boolean) {
		editVideo((v) => {
			if (on) delete v[key];
			else v[key] = false;
		}, `slice-video-${key}`);
	}

	/** controls: default on for a player, off for a loop; stored only when it differs */
	const showsControls = $derived(video?.controls ?? video?.player === true);
	function setControls(on: boolean) {
		editVideo((v) => {
			if (on === (v.player === true)) delete v.controls;
			else v.controls = on;
		}, 'slice-video-controls');
	}

	// ── files per quality ────────────────────────────────────────────────────────────────────────────────────────────
	const variants = $derived<Partial<Record<MediaQuality, string>>>(video?.variants ?? {});
	/** the default file is one of the levels (else it is an extra row "default, no level": older single-file videos) */
	const defaultLevel = $derived(MEDIA_QUALITIES.find((level) => !!video?.src && variants[level] === video.src) ?? null);
	const legacySrc = $derived(!!video?.src && !defaultLevel);
	/** the default when the old one goes: medium, else high, else low */
	const preferred = (vs: Partial<Record<MediaQuality, string>>) => vs.medium || vs.high || vs.low || '';

	function setVariant(level: MediaQuality, path: string) {
		editVideo((v) => {
			const vs: Partial<Record<MediaQuality, string>> = { ...(v.variants ?? {}) };
			const old = vs[level];
			if (path) vs[level] = path;
			else delete vs[level];
			if (Object.keys(vs).length) v.variants = Object.fromEntries(MEDIA_QUALITIES.filter((l) => vs[l]).map((l) => [l, vs[l]]));
			else delete v.variants;
			// the first file becomes the default; the default row follows its file
			if (!v.src || (old && v.src === old)) v.src = path || preferred(vs);
		}, `slice-video-variant-${level}`);
	}
	function setDefault(level: MediaQuality) {
		const file = variants[level];
		if (file) doc.setShared([...slicePath, 'video', 'src'], file, 'slice-video-default');
	}

	// ── placement ────────────────────────────────────────────────────────────────────────────────────────────────────
	const POSITIONS = ['top', 'center', 'bottom'] as const;
	const FITS = ['contain', 'cover'] as const;
	const placement = $derived<Required<SliceViewport> | null>(
		video?.viewport ? { position: video.viewport.position ?? 'center', fit: video.viewport.fit ?? 'contain' } : null
	);
	function setViewport(next: Required<SliceViewport> | null) {
		editVideo((v) => {
			if (next) v.viewport = { position: next.position, fit: next.fit };
			else delete v.viewport;
		}, 'slice-video-viewport');
	}
	const positionLabel = (p: (typeof POSITIONS)[number]) => m[`creator.video_position_${p}`]();
	const fitLabel = (f: (typeof FITS)[number]) => m[`creator.video_fit_${f}`]();
	const DEVICES = [
		{ kind: 'phone', w: 390, h: 844 },
		{ kind: 'desktop', w: 1440, h: 900 }
	] as const;
	/** the preview's object-position, like the reader (`slice-video.svelte`) */
	const objectPosition = (p: string) => (p === 'top' ? '50% 0%' : p === 'bottom' ? '50% 100%' : '50% 50%');
</script>

<div class="media" data-testid="slice-media">
	<div class="kind" role="radiogroup" aria-label={m['creator.media_type']()}>
		<label><input type="radio" name="slice-media" checked={!video} onchange={() => setMedia('image')} /> {m['creator.media_image']()}</label>
		<label><input type="radio" name="slice-media" checked={!!video} onchange={() => setMedia('video')} data-testid="media-video" /> {m['creator.media_video']()}</label>
	</div>

	{#if video}
		<fieldset class="group" data-testid="video-files">
			<legend title={m['creator.video_files_hint']()}>{m['creator.video_files']()}</legend>
			<div class="files">
				<span></span><span></span><span class="lbl" title={m['creator.video_default_hint']()}>{m['creator.video_default']()}</span>
				{#each MEDIA_QUALITIES as level (level)}
					<label class="lbl level" for={`video-${level}`}>{VIDEO_QUALITY_LABELS[level]} <small>{m[`creator.video_quality_${level}`]()}</small></label>
					<input
						id={`video-${level}`}
						type="text"
						value={variants[level] ?? ''}
						placeholder={`videos/clip.${VIDEO_QUALITY_LABELS[level]}.mp4`}
						onchange={(e) => setVariant(level, e.currentTarget.value.trim())}
						data-testid={`video-variant-${level}`}
					/>
					<input
						type="radio"
						name="video-default"
						checked={defaultLevel === level}
						disabled={!variants[level]}
						onchange={() => setDefault(level)}
						aria-label={`${m['creator.video_default']()} ${VIDEO_QUALITY_LABELS[level]}`}
						title={m['creator.video_default_hint']()}
						data-testid={`video-default-${level}`}
					/>
				{/each}
				{#if legacySrc}
					<label class="lbl level" for="video-src">{m['creator.video_default']()} <small>{m['creator.video_default_legacy']()}</small></label>
					<input
						id="video-src"
						type="text"
						value={video.src ?? ''}
						onchange={(e) => doc.setShared([...slicePath, 'video', 'src'], e.currentTarget.value.trim())}
						data-testid="video-path"
					/>
					<input type="radio" name="video-default" checked disabled aria-label={m['creator.video_default']()} />
				{/if}
			</div>
		</fieldset>
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
			<label title={m['creator.video_player_hint']()}
				><input type="checkbox" checked={video.player === true} onchange={(e) => setPlayer(e.currentTarget.checked)} data-testid="video-player" /> player</label
			>
			{#if video.player !== true}
				<label><input type="checkbox" checked={video.loop !== false} onchange={(e) => setFlag('loop', e.currentTarget.checked)} /> loop</label>
				<label><input type="checkbox" checked={video.autoplay !== false} onchange={(e) => setFlag('autoplay', e.currentTarget.checked)} /> autoplay</label>
			{/if}
			<label title={m['creator.video_controls_hint']()}
				><input type="checkbox" checked={showsControls} onchange={(e) => setControls(e.currentTarget.checked)} data-testid="video-controls" /> controls</label
			>
		</div>

		<fieldset class="group" data-testid="video-placement">
			<legend>{m['creator.video_placement']()}</legend>
			<div class="kind">
				<label><input type="radio" name="video-viewport" checked={!placement} onchange={() => setViewport(null)} data-testid="video-in-column" /> {m['creator.video_in_column']()}</label>
				<label title={m['creator.video_full_viewport_hint']()}
					><input type="radio" name="video-viewport" checked={!!placement} onchange={() => setViewport({ position: 'center', fit: 'contain' })} data-testid="video-full-viewport" />
					{m['creator.video_full_viewport']()}</label
				>
			</div>
			{#if placement}
				<!-- three positions × two fits; each cell a tiny phone with the video placed like that -->
				<div class="picker" role="radiogroup" aria-label={m['creator.video_placement']()}>
					<span></span>
					{#each POSITIONS as p (p)}<span class="lbl col">{positionLabel(p)}</span>{/each}
					{#each FITS as f (f)}
						<span class="lbl row" title={m[`creator.video_fit_${f}_hint`]()}>{fitLabel(f)}</span>
						{#each POSITIONS as p (p)}
							<button
								type="button"
								class="cell"
								class:active={placement.position === p && placement.fit === f}
								role="radio"
								aria-checked={placement.position === p && placement.fit === f}
								aria-label={`${fitLabel(f)}, ${positionLabel(p)}`}
								title={`${fitLabel(f)}, ${positionLabel(p)}: ${m[`creator.video_fit_${f}_hint`]()}`}
								onclick={() => setViewport({ position: p, fit: f })}
								data-testid={`placement-${f}-${p}`}
							>
								<span class="screen"><span class="clip {f} {p}"></span></span>
							</button>
						{/each}
					{/each}
				</div>
				<p class="hint">{m[`creator.video_fit_${placement.fit}_hint`]()}</p>
				{#if slice?.image}
					<!-- the poster placed like the reader does it, on a phone (390 × 844) and a desktop (1440 × 900) -->
					<div class="placement-preview" data-testid="placement-preview">
						{#each DEVICES as { kind, w, h } (kind)}
							<figure>
								<div class="device" style:aspect-ratio={`${w} / ${h}`}>
									<img
										src={url(slice.image)}
										alt=""
										style:object-fit={placement.fit}
										style:object-position={objectPosition(placement.position)}
									/>
								</div>
								<figcaption>{m[`creator.video_preview_${kind}`]()}</figcaption>
							</figure>
						{/each}
					</div>
				{/if}
			{/if}
		</fieldset>
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
		flex-wrap: wrap;
		gap: 0.8rem;
		font-size: 12px;
	}
	.group {
		border: 1px solid var(--border-subtle);
		margin: 0;
		padding: 0.4rem 0.5rem 0.5rem;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		min-width: 0;
	}
	legend {
		font-size: 11px;
		padding: 0 0.25rem;
	}
	.lbl {
		font-size: 10px;
		color: var(--muted);
	}
	.files {
		display: grid;
		grid-template-columns: auto 1fr auto;
		gap: 0.3rem 0.4rem;
		align-items: center;
	}
	.level small {
		display: block;
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
	.picker {
		display: grid;
		grid-template-columns: auto repeat(3, 44px);
		gap: 0.25rem;
		align-items: center;
		justify-items: center;
	}
	.picker .row {
		justify-self: start;
		max-width: 112px;
	}
	.cell {
		padding: 0.25rem;
		border: 2px solid var(--border-muted);
		background: var(--paper);
		color: var(--ink);
		cursor: pointer;
		line-height: 0;
	}
	.cell:hover {
		border-color: var(--ink);
	}
	.cell.active {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
	.cell.active .screen {
		background: var(--ink);
	}
	/* a tiny phone (portrait) with a landscape video placed in it */
	.screen {
		position: relative;
		display: block;
		width: 20px;
		height: 36px;
		border: 1px solid currentColor;
		background: var(--paper);
		overflow: hidden;
	}
	.clip {
		position: absolute;
		left: 0;
		right: 0;
		background: currentColor;
	}
	.clip.contain {
		height: 32%;
	}
	.clip.contain.top {
		top: 0;
	}
	.clip.contain.center {
		top: 34%;
	}
	.clip.contain.bottom {
		bottom: 0;
	}
	/* cover fills it; a lighter band marks the part that stays when the top/bottom are cropped */
	.clip.cover {
		inset: 0;
		background: color-mix(in srgb, currentColor 30%, transparent);
	}
	.clip.cover::after {
		content: '';
		position: absolute;
		left: 0;
		right: 0;
		height: 40%;
		background: currentColor;
	}
	.clip.cover.top::after {
		top: 0;
	}
	.clip.cover.center::after {
		top: 30%;
	}
	.clip.cover.bottom::after {
		bottom: 0;
	}
	.placement-preview {
		display: flex;
		gap: 0.75rem;
		align-items: flex-end;
	}
	.placement-preview figure {
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}
	.placement-preview figure:first-child .device {
		width: 56px;
	}
	.placement-preview figure:last-child .device {
		width: 144px;
	}
	.device {
		background: #000;
		border: 1px solid var(--border-subtle);
		overflow: hidden;
	}
	.device img {
		display: block;
		width: 100%;
		height: 100%;
	}
	figcaption {
		font-size: 10px;
		color: var(--muted);
	}
</style>
