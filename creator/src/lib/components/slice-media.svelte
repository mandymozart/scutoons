<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import {
		DEFAULT_SLICE, FRAME_PRESETS, FRAME_RATIO_MAX, FRAME_RATIO_MIN, MEDIA_QUALITIES, VIDEO_QUALITY_LABELS, VIEWPORT_POSITIONS,
		aspectRatioLabel, getAt, parseFrameRatio, viewportObjectFit, viewportObjectPosition,
		type MediaQuality, type Path, type SliceVideo, type ViewportFit
	} from '@scutoons/content';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { fillSliceSize } from '$lib/editor/images.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import ImagePicker from './image-picker.svelte';
	import PlacementPicker from './placement-picker.svelte';

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

	// the poster's own ratio (the previews of fit width / height, the "Original" warning)
	$effect(() => {
		const image = slice?.image;
		posterRatio = null;
		if (!image) return;
		const img = new Image();
		img.onload = () => (posterRatio = img.naturalWidth > 0 ? img.naturalWidth / img.naturalHeight : null);
		img.src = url(image);
	});
	$effect(() => {
		void video?.src;
		videoRatio = null;
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

	// ── placement: full viewport, or a frame in the column ───────────────────────────────────────────────────────────
	type Place = { position: (typeof VIEWPORT_POSITIONS)[number]; fit: ViewportFit };
	const frameRatio = $derived(video?.frame ? parseFrameRatio(video.frame.ratio) : null);
	/** what the picker edits: the viewport placement, else the frame's, else nothing (Original in the column) */
	const placement = $derived<Place | null>(
		video?.viewport
			? { position: video.viewport.position ?? 'center', fit: video.viewport.fit ?? 'contain' }
			: video?.frame && frameRatio
				? { position: video.frame.position ?? 'center', fit: video.frame.fit ?? 'contain' }
				: null
	);
	const presetOf = (ratio: unknown) => {
		const pair = parseFrameRatio(ratio);
		return pair ? (FRAME_PRESETS.find((p) => { const q = parseFrameRatio(p.ratio)!; return q[0] * pair[1] === q[1] * pair[0]; })?.id ?? 'custom') : 'original';
	};
	const frameChoice = $derived(video?.frame ? presetOf(video.frame.ratio) : 'original');
	let customW = $state(1);
	let customH = $state(2);
	$effect(() => {
		if (frameChoice === 'custom' && frameRatio) [customW, customH] = frameRatio;
	});

	/** full viewport on/off (the frame doesn't apply there: it goes, and the size follows the poster again) */
	function setViewportMode(on: boolean) {
		editVideo((v) => {
			if (on) {
				v.viewport = { position: 'center', fit: 'contain' };
				delete v.frame;
			} else delete v.viewport;
		}, 'slice-video-viewport');
		fillSliceSize(doc, slicePath, url);
	}
	function setPlace(next: Place) {
		editVideo((v) => {
			if (v.viewport) v.viewport = { position: next.position, fit: next.fit };
			else if (v.frame) v.frame = { ...v.frame, position: next.position, fit: next.fit };
		}, 'slice-video-place');
	}
	/** a frame ratio ("9:16" / [w, h], clamped to 4:1 … 1:4) or null = Original; the slice size follows */
	function setFrame(ratio: string | [number, number] | null) {
		editVideo((v) => {
			if (!ratio) delete v.frame;
			else v.frame = { ratio, fit: v.frame?.fit ?? 'contain', position: v.frame?.position ?? 'center' };
		}, 'slice-video-frame');
		fillSliceSize(doc, slicePath, url);
	}
	function chooseFrame(choice: string) {
		if (choice === 'original') return setFrame(null);
		if (choice === 'custom') return setCustom(customW, customH);
		const preset = FRAME_PRESETS.find((p) => p.id === choice);
		if (preset) setFrame(preset.ratio);
	}
	function setCustom(w: number, h: number) {
		if (!(w > 0) || !(h > 0)) return;
		let r = w / h;
		if (r > FRAME_RATIO_MAX) [w, h] = [FRAME_RATIO_MAX, 1];
		else if (r < FRAME_RATIO_MIN) [w, h] = [1, 1 / FRAME_RATIO_MIN];
		[customW, customH] = [w, h];
		setFrame([w, h]);
	}
	const DEVICES = [
		{ kind: 'phone', w: 390, h: 844 },
		{ kind: 'desktop', w: 1440, h: 900 }
	] as const;
	/** the poster's own ratio (for fit width / height in the previews) and the video's (the "Original" warning) */
	let posterRatio = $state<number | null>(null);
	let videoRatio = $state<number | null>(null);
	const mediaRatio = $derived(videoRatio ?? posterRatio ?? 16 / 9);
	const originalMismatch = $derived(
		!!video && !placement && !!posterRatio && !!videoRatio && Math.abs(posterRatio - videoRatio) / videoRatio > 0.02
	);
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
					onloadedmetadata={(e) => { videoState = 'ok'; const v = e.currentTarget; if (v.videoWidth > 0) videoRatio = v.videoWidth / v.videoHeight; }}
					onerror={() => (videoState = 'missing')}
				></video>
				<span class="meta">
					{#if videoState === 'missing'}<span class="bad">{m['creator.video_missing']()}</span>{:else if videoState === 'ok'}<Icon name="check" /> CDN{/if}
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
				<label><input type="radio" name="video-viewport" checked={!video.viewport} onchange={() => setViewportMode(false)} data-testid="video-in-column" /> {m['creator.video_in_column']()}</label>
				<label title={m['creator.video_full_viewport_hint']()}
					><input type="radio" name="video-viewport" checked={!!video.viewport} onchange={() => setViewportMode(true)} data-testid="video-full-viewport" />
					{m['creator.video_full_viewport']()}</label
				>
			</div>
			{#if !video.viewport}
				<!-- the box in the column: the media's own ratio, or a frame (the slice size follows it) -->
				<label class="field ratio">
					<span class="lbl" title={m['creator.video_frame_hint']()}>{m['creator.video_frame']()}</span>
					<select value={frameChoice} onchange={(e) => chooseFrame(e.currentTarget.value)} data-testid="video-frame">
						<option value="original">{m['creator.video_frame_original']()}</option>
						{#each FRAME_PRESETS as preset (preset.id)}
							<option value={preset.id}>{preset.id === 'keyvisual' ? m['creator.video_frame_keyvisual']() : preset.ratio}</option>
						{/each}
						<option value="custom">{m['creator.video_frame_custom']()}</option>
					</select>
				</label>
				{#if frameChoice === 'custom'}
					<div class="custom" data-testid="video-frame-custom">
						<input type="number" min="1" step="any" value={customW} aria-label={m['creator.video_frame_width']()} onchange={(e) => setCustom(Number(e.currentTarget.value), customH)} data-testid="video-frame-w" />
						<span>:</span>
						<input type="number" min="1" step="any" value={customH} aria-label={m['creator.video_frame_height']()} onchange={(e) => setCustom(customW, Number(e.currentTarget.value))} data-testid="video-frame-h" />
					</div>
				{/if}
				{#if originalMismatch}
					<p class="hint warn" data-testid="video-original-mismatch">{m['creator.video_original_mismatch']()}</p>
				{/if}
			{/if}
			{#if placement}
				<!-- each fit a tiny screen with the poster placed in it; position buttons only where they matter -->
				<PlacementPicker
					fit={placement.fit}
					position={placement.position}
					{mediaRatio}
					boxRatios={video.viewport ? DEVICES.map((d) => d.w / d.h) : frameRatio ? [frameRatio[0] / frameRatio[1]] : []}
					onchange={(fit, position) => setPlace({ fit, position })}
				/>
				{#if slice?.image}
					<!-- the poster placed like the reader does it: on a phone + a desktop (full viewport) or in the frame -->
					<div class="placement-preview" data-testid="placement-preview">
						{#if video.viewport}
							{#each DEVICES as { kind, w, h } (kind)}
								<figure>
									<div class="device {kind}" style:aspect-ratio={`${w} / ${h}`}>
										<img
											src={url(slice.image)}
											alt=""
											style:object-fit={viewportObjectFit(placement.fit, mediaRatio, w / h)}
											style:object-position={viewportObjectPosition(placement.position)}
										/>
									</div>
								</figure>
							{/each}
						{:else if frameRatio}
							<figure>
								<div class="device frame" style:aspect-ratio={`${frameRatio[0]} / ${frameRatio[1]}`}>
									<img
										src={url(slice.image)}
										alt=""
										style:object-fit={viewportObjectFit(placement.fit, mediaRatio, frameRatio[0] / frameRatio[1])}
										style:object-position={viewportObjectPosition(placement.position)}
									/>
								</div>
							</figure>
						{/if}
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
		{#if video?.viewport}
			{m['creator.slice_size_viewport']({ ratio: aspectRatioLabel(slice?.width ?? 0, slice?.height ?? 0, getLocale()) })}
		{:else if video?.frame && frameRatio}
			{m['creator.slice_size_frame']({ width: slice?.width ?? '–', height: slice?.height ?? '–' })}
		{:else}
			{m['creator.slice_size']({ width: slice?.width ?? '–', height: slice?.height ?? '–', standard: `${DEFAULT_SLICE.width} × ${DEFAULT_SLICE.height}` })}
		{/if}
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
		gap: var(--space-1);
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
		font-size: var(--type-caption-size);
		padding: 0 0.25rem;
	}
	.lbl {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
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
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.bad {
		color: var(--error);
	}
	.ratio select {
		max-width: 100%;
	}
	.custom {
		display: flex;
		align-items: center;
		gap: 0.3rem;
	}
	.custom input {
		width: 64px;
	}
	.warn {
		color: var(--error);
	}
	.device.frame {
		width: 48px;
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
	.placement-preview .device.phone {
		width: 40px;
	}
	.placement-preview .device.desktop {
		width: 96px;
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
</style>
